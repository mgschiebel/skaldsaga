#!/usr/bin/env python3
"""Checks docs/security/license-elections.toml against the locked dependencies.

One entry per shipped dependency: the license expression the crate declares,
the license we elect from it, and the reason where there was a choice
(docs/security/appsec-tooling-research.md, Decided item 8). It fails when:

  - a shipped crate has no entry;
  - an entry names a crate that is no longer shipped;
  - an entry's `declared` text no longer matches what the crate declares
    (a license change must be looked at again, not carried over);
  - the elected license is not part of, or does not satisfy, the declared
    expression;
  - an elected license is not on the allowlist in deny.toml;
  - the crate offered a choice (OR) and the entry gives no reason.

"Shipped" means the normal and build dependencies of the workspace crates,
for every target; dev-dependencies and the workspace's own crates are left out
(docs/security/license-policy.md). Standard library only. Run it from anywhere:

    python3 -I ci/check-license-elections.py
"""

import json
import os
import re
import subprocess
import sys
import tomllib

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ELECTIONS = os.path.join(ROOT, "docs", "security", "license-elections.toml")
DENY = os.path.join(ROOT, "deny.toml")

TOKEN = re.compile(r"\(|\)|[^\s()]+")


def tokens(expression):
    # Old crates write "MIT/Apache-2.0" for "either".
    return TOKEN.findall(expression.replace("/", " OR "))


class Parser:
    """SPDX expressions: OR binds loosest, then AND, then WITH, then parentheses."""

    def __init__(self, expression):
        self.toks = tokens(expression)
        self.pos = 0

    def peek(self):
        return self.toks[self.pos] if self.pos < len(self.toks) else None

    def take(self):
        tok = self.peek()
        self.pos += 1
        return tok

    def parse(self):
        tree = self.expr()
        if self.peek() is not None:
            raise ValueError(f"unexpected '{self.peek()}'")
        return tree

    def expr(self):
        parts = [self.term()]
        while self.peek() == "OR":
            self.take()
            parts.append(self.term())
        return parts[0] if len(parts) == 1 else ("or", parts)

    def term(self):
        parts = [self.factor()]
        while self.peek() == "AND":
            self.take()
            parts.append(self.factor())
        return parts[0] if len(parts) == 1 else ("and", parts)

    def factor(self):
        tok = self.take()
        if tok is None:
            raise ValueError("expression ends too early")
        if tok == "(":
            tree = self.expr()
            if self.take() != ")":
                raise ValueError("missing ')'")
            return tree
        if tok in (")", "AND", "OR", "WITH"):
            raise ValueError(f"unexpected '{tok}'")
        if self.peek() == "WITH":
            self.take()
            exception = self.take()
            if exception is None:
                raise ValueError("'WITH' needs an exception id")
            return ("id", f"{tok} WITH {exception}")
        return ("id", tok)


def ids_in(tree):
    if tree[0] == "id":
        return {tree[1]}
    found = set()
    for part in tree[1]:
        found |= ids_in(part)
    return found


def satisfied(tree, chosen):
    if tree[0] == "id":
        return tree[1] in chosen
    if tree[0] == "and":
        return all(satisfied(p, chosen) for p in tree[1])
    return any(satisfied(p, chosen) for p in tree[1])


def has_choice(tree):
    if tree[0] == "id":
        return False
    return tree[0] == "or" or any(has_choice(p) for p in tree[1])


def shipped_crates():
    out = subprocess.run(
        ["cargo", "metadata", "--locked", "--format-version", "1"],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    meta = json.loads(out)
    packages = {p["id"]: p for p in meta["packages"]}
    nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
    members = set(meta["workspace_members"])
    seen = set()
    stack = list(members)
    while stack:
        for dep in nodes[stack.pop()]["deps"]:
            kinds = {k["kind"] for k in dep["dep_kinds"]}
            if kinds <= {"dev"} or dep["pkg"] in seen or dep["pkg"] in members:
                continue
            seen.add(dep["pkg"])
            stack.append(dep["pkg"])
    return {packages[i]["name"]: packages[i] for i in seen}


def main():
    errors = []
    shipped = shipped_crates()

    with open(DENY, "rb") as handle:
        allowed = set(tomllib.load(handle)["licenses"]["allow"])
    with open(ELECTIONS, "rb") as handle:
        entries = tomllib.load(handle).get("crates", {})

    for name in sorted(shipped):
        if name not in entries:
            declared = shipped[name].get("license") or "(none declared)"
            errors.append(f"{name}: no entry in license-elections.toml (declares: {declared})")
    for name in sorted(entries):
        if name not in shipped:
            errors.append(f"{name}: entry for a crate that is no longer shipped; remove it")

    for name in sorted(set(entries) & set(shipped)):
        entry = entries[name]
        declared = shipped[name].get("license")
        if not declared:
            errors.append(f"{name}: declares no SPDX expression; it needs a one-off decision (license-policy.md)")
            continue
        if entry.get("declared") != declared:
            errors.append(
                f"{name}: the crate now declares '{declared}' but the entry says "
                f"'{entry.get('declared')}'; review the change, then update the entry"
            )
            continue
        elected = entry.get("elected", "")
        try:
            tree = Parser(declared).parse()
            chosen_tree = Parser(elected).parse()
        except ValueError as problem:
            errors.append(f"{name}: cannot read a license expression ({problem})")
            continue
        if has_choice(chosen_tree):
            errors.append(f"{name}: 'elected' must be one license, or licenses joined with AND, not '{elected}'")
            continue
        chosen = ids_in(chosen_tree)
        outside = chosen - ids_in(tree)
        if outside:
            errors.append(f"{name}: elected {sorted(outside)} is not in the declared '{declared}'")
        elif not satisfied(tree, chosen):
            errors.append(f"{name}: elected '{elected}' does not satisfy the declared '{declared}'")
        off_list = sorted(chosen - allowed)
        if off_list:
            errors.append(f"{name}: elected {off_list} is not on the allowlist in deny.toml")
        if has_choice(tree) and not str(entry.get("reason", "")).strip():
            errors.append(f"{name}: the crate offers a choice ('{declared}'), so the entry needs a reason")

    if errors:
        print("License elections do not match the locked dependencies:", file=sys.stderr)
        for line in errors:
            print(f"  - {line}", file=sys.stderr)
        return 1
    print(f"License elections cover all {len(shipped)} shipped dependencies.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
