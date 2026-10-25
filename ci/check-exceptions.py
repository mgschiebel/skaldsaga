#!/usr/bin/env python3
"""Checks that every security suppression is on the register and has not expired.

A suppression is anything that tells a tool to look away: a Rust `allow` or
`expect` attribute, a lint level of "allow", a `clippy.toml` relaxation, a
cargo-deny ignore, skip or extra license exception, a ShellCheck disable, a
zizmor ignore, a CodeQL filter, a gitleaks allowlist, `continue-on-error`.
The rule (docs/security/appsec-tooling-research.md, Decided item 6 and Strategy
item 3): a suppression needs a written reason, an expiry and the author's
approval. The register is docs/security/exceptions.toml. This check fails when:

  - a suppression in the repository is not covered by a register entry (this
    is the flag on new suppressions);
  - a register entry covers nothing any more (remove it);
  - an entry is incomplete: no reason, no approval date, no PR or note saying
    where the author approved it;
  - an entry has expired, or its expiry is more than a year away.

It cannot see dismissed code-scanning alerts (they live on GitHub, not in the
repository) and cannot tell who approved an entry; the author merges every
pull request, so the register file is where the approval is read.

Standard library only. Run it from anywhere:

    python3 -I ci/check-exceptions.py
"""

import datetime
import fnmatch
import os
import re
import sys
import tomllib

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
REGISTER = "docs/security/exceptions.toml"
SKIP_DIRS = {".git", "target", "corpus", "artifacts", "dist", "node_modules", "__pycache__"}
KINDS = {
    "rust-attribute",
    "cargo-lint-level",
    "clippy-config",
    "deny-toml",
    "workflow",
    "codeql-config",
    "shellcheck",
    "python",
    "gitleaks",
}
MAX_DAYS = 366
WARN_DAYS = 30
NAMES = re.compile(r"[A-Za-z_][\w:-]*")
CRATES_IO = {
    "https://github.com/rust-lang/crates.io-index",
    "sparse+https://index.crates.io/",
}
# Built in two pieces so this file does not match its own scan.
GITLEAKS_ALLOW = re.compile("gitleaks" + ":allow")


def walk(root):
    for base, dirs, files in os.walk(root):
        dirs[:] = sorted(d for d in dirs if d not in SKIP_DIRS)
        for name in sorted(files):
            full = os.path.join(base, name)
            yield os.path.relpath(full, root).replace(os.sep, "/")


def read(root, rel):
    try:
        with open(os.path.join(root, rel), encoding="utf-8") as f:
            return f.read()
    except (OSError, UnicodeDecodeError):
        return ""


def load_toml(root, rel):
    try:
        return tomllib.loads(read(root, rel))
    except tomllib.TOMLDecodeError:
        return {}


def found(kind, path, lints, detail=""):
    return {"kind": kind, "path": path, "lints": sorted(set(lints)), "detail": detail}


# ---- Rust attributes --------------------------------------------------------


def attribute_texts(text):
    """The text inside every `#[...]` and `#![...]`, skipping comment lines."""
    code = "\n".join("" if line.lstrip().startswith("//") else line for line in text.split("\n"))
    for m in re.finditer(r"#!?\[", code):
        depth, i, in_str = 1, m.end(), False
        while i < len(code) and depth:
            c = code[i]
            if in_str:
                if c == "\\":
                    i += 1
                elif c == '"':
                    in_str = False
            elif c == '"':
                in_str = True
            elif c == "[":
                depth += 1
            elif c == "]":
                depth -= 1
            i += 1
        if depth == 0:
            yield code[m.end() : i - 1]


def paren_body(text, start):
    """The text between the parenthesis at `start` and its match."""
    depth, i, in_str = 0, start, False
    while i < len(text):
        c = text[i]
        if in_str:
            if c == "\\":
                i += 1
            elif c == '"':
                in_str = False
        elif c == '"':
            in_str = True
        elif c == "(":
            depth += 1
        elif c == ")":
            depth -= 1
            if depth == 0:
                return text[start + 1 : i]
        i += 1
    return text[start + 1 :]


def lint_names(body):
    body = re.sub(r'"(?:[^"\\]|\\.)*"', '""', body)  # drop strings (the reason)
    return [n for n in NAMES.findall(body) if n != "reason"]


def rust_suppressions(rel, text):
    for attr in attribute_texts(text):
        head = re.match(r"\s*(allow|expect)\s*\(", attr)
        if head:
            yield found("rust-attribute", rel, lint_names(paren_body(attr, head.end() - 1)))
        elif re.match(r"\s*cfg_attr\s*\(", attr):
            for m in re.finditer(r"\b(allow|expect)\s*\(", attr):
                yield found("rust-attribute", rel, lint_names(paren_body(attr, m.end() - 1)))
    if re.search(r"codeql\[", text):
        yield found("codeql-config", rel, ["inline-comment"])


# ---- Config files -----------------------------------------------------------


def workspace_crates(root):
    names = set()
    doc = load_toml(root, "Cargo.toml")
    for member in doc.get("workspace", {}).get("members", []):
        name = load_toml(root, member + "/Cargo.toml").get("package", {}).get("name")
        if name:
            names.add(name)
    fuzz = load_toml(root, "fuzz/Cargo.toml").get("package", {}).get("name")
    if fuzz:
        names.add(fuzz)
    return names


def entry_name(entry):
    if isinstance(entry, str):
        return entry
    if isinstance(entry, dict):
        return str(entry.get("id") or entry.get("crate") or entry.get("name") or entry)
    return str(entry)


def deny_suppressions(root):
    doc = load_toml(root, "deny.toml")
    own = workspace_crates(root)
    for entry in doc.get("advisories", {}).get("ignore", []):
        yield found("deny-toml", "deny.toml", ["advisories.ignore:" + entry_name(entry)])
    for key in ("skip", "skip-tree"):
        for entry in doc.get("bans", {}).get(key, []):
            yield found("deny-toml", "deny.toml", [f"bans.{key}:" + entry_name(entry)])
    for entry in doc.get("licenses", {}).get("exceptions", []):
        if entry_name(entry) not in own:  # the project's own crates are not a finding
            yield found("deny-toml", "deny.toml", ["licenses.exceptions:" + entry_name(entry)])
    for entry in doc.get("licenses", {}).get("clarify", []):
        yield found("deny-toml", "deny.toml", ["licenses.clarify:" + entry_name(entry)])
    for key in ("allow-git", "allow-registry"):
        for entry in doc.get("sources", {}).get(key, []):
            if entry_name(entry) not in CRATES_IO:  # the normal registry is the default, not a finding
                yield found("deny-toml", "deny.toml", [f"sources.{key}:" + entry_name(entry)])


def cargo_lint_levels(rel, doc):
    tables = [doc.get("lints", {}), doc.get("workspace", {}).get("lints", {})]
    for tools in tables:
        for tool, lints in tools.items():
            if not isinstance(lints, dict):
                continue
            for lint, value in lints.items():
                level = value.get("level") if isinstance(value, dict) else value
                if level == "allow":
                    yield found("cargo-lint-level", rel, [f"{tool}::{lint}"])


def text_suppressions(rel, text):
    if rel.startswith(".github/workflows/") and rel.endswith((".yml", ".yaml")):
        if re.search(r"^\s*continue-on-error:\s*true\b", text, re.M):
            yield found("workflow", rel, ["continue-on-error"])
        for m in re.finditer(r"zizmor:\s*ignore\[([^\]]*)\]", text):
            yield found("workflow", rel, ["zizmor:" + r.strip() for r in m.group(1).split(",")])
    if rel.startswith(".github/") and re.search(r"^\s*(paths-ignore|query-filters):", text, re.M):
        yield found("codeql-config", rel, ["paths-ignore-or-query-filters"])
    if rel.endswith(".sh") or rel == "plugin/scripts/skaldsaga":
        for m in re.finditer(r"#\s*shellcheck\s+disable=([\w,]+)", text):
            yield found("shellcheck", rel, m.group(1).split(","))
    if rel.endswith(".py"):
        for m in re.finditer(r"#\s*(nosec|noqa)\b", text):
            yield found("python", rel, [m.group(1)])
    if rel.endswith(".py") or rel.endswith(".rs") or rel.endswith(".sh") or rel.endswith(".yml"):
        if GITLEAKS_ALLOW.search(text):
            yield found("gitleaks", rel, ["inline-allow-comment"])


def suppressions(root):
    for rel in walk(root):
        text = read(root, rel)
        if rel.endswith(".rs"):
            yield from rust_suppressions(rel, text)
        elif rel.endswith("Cargo.toml"):
            try:
                yield from cargo_lint_levels(rel, tomllib.loads(text))
            except tomllib.TOMLDecodeError:
                pass
        elif rel == "clippy.toml":
            for key, value in load_toml(root, rel).items():
                if key.startswith("allow-") and value:
                    yield found("clippy-config", rel, [key])
        elif rel == ".gitleaks.toml":
            doc = load_toml(root, rel)
            for key in ("allowlist", "allowlists"):
                if doc.get(key):
                    yield found("gitleaks", rel, [key])
        elif rel == ".gitleaksignore":
            if any(line.strip() and not line.startswith("#") for line in text.split("\n")):
                yield found("gitleaks", rel, [".gitleaksignore"])
        elif rel == ".github/zizmor.yml" or rel == "zizmor.yml":
            if re.search(r"^\s*ignore:", text, re.M):
                yield found("workflow", rel, ["zizmor-config-ignore"])
        yield from text_suppressions(rel, text)
    yield from deny_suppressions(root)


# ---- The register -----------------------------------------------------------


def parse_date(value):
    try:
        return datetime.date.fromisoformat(str(value))
    except ValueError:
        return None


def check_entry(entry, today):
    """Problems with one register entry, in plain words."""
    name = entry.get("id", "(no id)")
    problems = []
    for field in ("id", "kind", "where", "reason", "approved", "approved_in", "expires"):
        if not str(entry.get(field, "")).strip():
            problems.append(f"{name}: '{field}' is missing")
    lints = entry.get("lints")
    if not isinstance(lints, list) or not lints or not all(isinstance(x, str) for x in lints):
        problems.append(f"{name}: 'lints' must be a list of the suppressed names")
    if entry.get("kind") and entry["kind"] not in KINDS:
        problems.append(f"{name}: kind '{entry['kind']}' isn't one of {sorted(KINDS)}")
    approved, expires = parse_date(entry.get("approved", "")), parse_date(entry.get("expires", ""))
    if entry.get("approved") and (approved is None or approved > today):
        problems.append(f"{name}: 'approved' must be a past or present date (YYYY-MM-DD)")
    if entry.get("expires") and expires is None:
        problems.append(f"{name}: 'expires' must be a date (YYYY-MM-DD)")
    elif expires is not None and expires <= today:
        problems.append(
            f"{name}: expired on {expires}. Fix the finding it covers, or have the author "
            "renew it with a new reason and a new date"
        )
    elif expires is not None and (expires - today).days > MAX_DAYS:
        problems.append(f"{name}: expires {expires}, more than a year away; pick an earlier date")
    return problems


def covers(entry, s):
    return (
        entry.get("kind") == s["kind"]
        and fnmatch.fnmatch(s["path"], str(entry.get("where", "")))
        and set(s["lints"]) <= set(entry.get("lints") or [])
    )


def check(root, today):
    """Returns (problems, notes, count of suppressions found)."""
    problems, notes = [], []
    entries = load_toml(root, REGISTER).get("exception", [])
    ids = [e.get("id") for e in entries]
    for dup in sorted({i for i in ids if ids.count(i) > 1}):
        problems.append(f"{dup}: appears more than once in {REGISTER}")
    for entry in entries:
        problems.extend(check_entry(entry, today))
        expires = parse_date(entry.get("expires", ""))
        if expires and 0 < (expires - today).days <= WARN_DAYS:
            notes.append(f"{entry.get('id')}: expires {expires}, within {WARN_DAYS} days")

    used = set()
    total = 0
    for s in suppressions(root):
        total += 1
        hits = [i for i, e in enumerate(entries) if covers(e, s)]
        used.update(hits)
        if not hits:
            problems.append(
                f"{s['path']}: {s['kind']} {', '.join(s['lints'])} is not in {REGISTER}. "
                "Fix the finding instead; if it has to stay, add an entry with a reason, "
                "an expiry and the author's approval"
            )
    for i, entry in enumerate(entries):
        if i not in used:
            problems.append(f"{entry.get('id')}: covers nothing any more; remove it from {REGISTER}")
    return problems, notes, total


def main():
    override = os.environ.get("EXCEPTIONS_TODAY")
    today = parse_date(override) if override else datetime.datetime.now(datetime.timezone.utc).date()
    if today is None:
        print("EXCEPTIONS_TODAY must be YYYY-MM-DD", file=sys.stderr)
        return 2
    problems, notes, total = check(ROOT, today)
    for note in notes:
        print(f"note: {note}")
    if problems:
        print("Suppressions that are not in order:", file=sys.stderr)
        for p in problems:
            print(f"  - {p}", file=sys.stderr)
        return 1
    entries = load_toml(ROOT, REGISTER).get("exception", [])
    print(f"{total} suppressions found, all on the register ({len(entries)} entries, none expired).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
