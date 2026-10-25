#!/usr/bin/env python3
"""The supply-chain files that ship with a release.

    python3 -I ci/release-files.py sbom    --output FILE   # SPDX 2.3 JSON
    python3 -I ci/release-files.py notices --output FILE   # third-party notices (Markdown)
    python3 -I ci/release-files.py check                   # build both in memory and check them
    python3 -I ci/release-files.py matches --audit-info FILE
        # the SBOM lists exactly the packages `rust-audit-info BINARY` reports

What they list is what the release binary is built from: the normal and build
dependencies of the workspace crates, for the release target (Linux x64 in
phase 1), without dev-dependencies (docs/security/license-policy.md). The
license elections file covers every target, so it is a superset. A crate that
is only built for another target (for example `r-efi`, UEFI only) is not in the
binary and so not in these files. Their licenses come from
two places: the license each crate *declares* (from `cargo metadata`) and the
license we *elect* from it (docs/security/license-elections.toml, Decided items
8, 9 and 11 in docs/security/appsec-tooling-research.md). Run the elections
check first; this script assumes it passes. Standard library only. Not legal
advice.

The SBOM carries the NTIA minimum fields: supplier (crates.io, where the crate
was downloaded from), component name and version, other unique identifiers (a
package URL and the SHA-256 from Cargo.lock), dependency relationships, the
author of the SBOM data (this script) and a timestamp. Each package has both a
declared and a concluded license. The notices file reproduces each shipped
crate's license text for the license we elected, and its NOTICE file if it has
one, because most permissive licenses require that when a compiled binary is
redistributed.
"""

import argparse
import datetime
import json
import os
import re
import subprocess
import sys
import tomllib
import uuid

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ELECTIONS = os.path.join(ROOT, "docs", "security", "license-elections.toml")
REPOSITORY = "https://github.com/mgschiebel/skaldsaga"
OWN_LICENSE = "AGPL-3.0-or-later"
TOOL = "skaldsaga ci/release-files.py"

# Which file names carry the text of which license, for the notices file.
LICENSE_FILE_TOKENS = {
    "MIT": ["MIT"],
    "Apache-2.0": ["APACHE"],
    "BSL-1.0": ["BOOST", "BSL"],
    "Unlicense": ["UNLICENSE"],
    "Zlib": ["ZLIB"],
    "Unicode-3.0": ["UNICODE"],
    "BSD-2-Clause": ["BSD"],
    "BSD-3-Clause": ["BSD"],
    "ISC": ["ISC"],
    "CC0-1.0": ["CC0"],
    "X11": ["X11"],
    "NCSA": ["NCSA"],
}
LICENSE_FILE = re.compile(r"^(LICEN[CS]E|COPYING|UNLICENSE|NOTICE|COPYRIGHT)([-._].*)?$", re.IGNORECASE)
GENERIC_NAMES = ("LICENSE", "LICENCE", "COPYING")


TARGET = "x86_64-unknown-linux-gnu"


def cargo_metadata():
    subprocess.run(["cargo", "fetch", "--locked"], cwd=ROOT, check=True, capture_output=True)
    out = subprocess.run(
        ["cargo", "metadata", "--locked", "--format-version", "1", "--filter-platform", TARGET],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    return json.loads(out)


def lock_checksums():
    with open(os.path.join(ROOT, "Cargo.lock"), "rb") as handle:
        lock = tomllib.load(handle)
    return {(p["name"], p["version"]): p.get("checksum") for p in lock.get("package", [])}


def shipped(meta):
    """The workspace crates, and the packages they ship, with the edges between them."""
    packages = {p["id"]: p for p in meta["packages"]}
    nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
    members = set(meta["workspace_members"])
    seen = set(members)
    edges = {}
    stack = list(members)
    while stack:
        current = stack.pop()
        targets = []
        for dep in nodes[current]["deps"]:
            kinds = {k["kind"] for k in dep["dep_kinds"]}
            if kinds <= {"dev"}:
                continue
            targets.append(dep["pkg"])
            if dep["pkg"] not in seen:
                seen.add(dep["pkg"])
                stack.append(dep["pkg"])
        edges[current] = sorted(set(targets))
    return packages, members, seen, edges


def load_elections():
    with open(ELECTIONS, "rb") as handle:
        return tomllib.load(handle).get("crates", {})


def normalise(expression):
    """SPDX writes "either" as OR; some crates still write a slash."""
    return re.sub(r"\s*/\s*", " OR ", expression.strip())


def spdx_id(package):
    safe = re.sub(r"[^A-Za-z0-9.-]", "-", f"{package['name']}-{package['version']}")
    return f"SPDXRef-Package-{safe}"


def strip_email(author):
    return re.sub(r"\s*<[^>]*>", "", author).strip()


def build_sbom(meta, created, namespace_suffix):
    packages, members, seen, edges = shipped(meta)
    elections = load_elections()
    checksums = lock_checksums()
    root = next(i for i in members if packages[i]["name"] == "skaldsaga")
    document_packages = []
    ids = {}
    for pid in sorted(seen, key=lambda i: (packages[i]["name"], packages[i]["version"])):
        pkg = packages[pid]
        sid = spdx_id(pkg)
        if sid in ids.values():
            raise SystemExit(f"two packages would share the SPDX id {sid}")
        ids[pid] = sid
        own = pid in members
        declared = normalise(pkg["license"] or "") if pkg.get("license") else "NOASSERTION"
        if own:
            concluded = OWN_LICENSE
        else:
            entry = elections.get(pkg["name"])
            if not entry:
                raise SystemExit(f"{pkg['name']} has no entry in license-elections.toml")
            concluded = entry["elected"]
        entry_out = {
            "SPDXID": sid,
            "name": pkg["name"],
            "versionInfo": pkg["version"],
            "supplier": "Organization: Skaldsaga" if own else "Organization: crates.io",
            "downloadLocation": REPOSITORY
            if own
            else f"https://crates.io/api/v1/crates/{pkg['name']}/{pkg['version']}/download",
            "filesAnalyzed": False,
            "licenseDeclared": declared,
            "licenseConcluded": concluded,
            "copyrightText": "NOASSERTION",
            "primaryPackagePurpose": "APPLICATION" if pid == root else "LIBRARY",
            "externalRefs": [
                {
                    "referenceCategory": "PACKAGE-MANAGER",
                    "referenceType": "purl",
                    "referenceLocator": f"pkg:cargo/{pkg['name']}@{pkg['version']}",
                }
            ],
        }
        authors = [strip_email(a) for a in pkg.get("authors") or [] if strip_email(a)]
        if authors and not own:
            entry_out["originator"] = f"Person: {authors[0]}"
        checksum = checksums.get((pkg["name"], pkg["version"]))
        if checksum and not own:
            entry_out["checksums"] = [{"algorithm": "SHA256", "checksumValue": checksum}]
        document_packages.append(entry_out)

    relationships = [
        {
            "spdxElementId": "SPDXRef-DOCUMENT",
            "relationshipType": "DESCRIBES",
            "relatedSpdxElement": ids[root],
        }
    ]
    for pid in sorted(edges, key=lambda i: ids[i]):
        for target in edges[pid]:
            relationships.append(
                {
                    "spdxElementId": ids[pid],
                    "relationshipType": "DEPENDS_ON",
                    "relatedSpdxElement": ids[target],
                }
            )
    version = packages[root]["version"]
    return {
        "spdxVersion": "SPDX-2.3",
        "dataLicense": "CC0-1.0",
        "SPDXID": "SPDXRef-DOCUMENT",
        "name": f"skaldsaga-{version}",
        "documentNamespace": f"{REPOSITORY}/sbom/{version}/{namespace_suffix}",
        "creationInfo": {"created": created, "creators": [f"Tool: {TOOL}"]},
        "packages": document_packages,
        "relationships": relationships,
    }


def check_sbom(sbom):
    """Facts every release SBOM must satisfy; returns a list of problems."""
    problems = []
    ids = [p["SPDXID"] for p in sbom["packages"]]
    if len(ids) != len(set(ids)):
        problems.append("duplicate SPDX ids")
    known = set(ids) | {"SPDXRef-DOCUMENT"}
    for pkg in sbom["packages"]:
        for field in ("name", "versionInfo", "supplier", "licenseDeclared", "licenseConcluded", "externalRefs"):
            if not pkg.get(field):
                problems.append(f"{pkg.get('name')}: missing {field}")
        if pkg.get("licenseConcluded") in (None, "NOASSERTION"):
            problems.append(f"{pkg.get('name')}: no concluded license")
    for rel in sbom["relationships"]:
        for end in ("spdxElementId", "relatedSpdxElement"):
            if rel[end] not in known:
                problems.append(f"relationship names unknown {rel[end]}")
    if not any(r["relationshipType"] == "DESCRIBES" for r in sbom["relationships"]):
        problems.append("nothing is described")
    if not sbom["creationInfo"]["created"] or not sbom["creationInfo"]["creators"]:
        problems.append("missing creation info")
    return problems


def license_files(directory, elected):
    """The files holding the text of the elected licenses, plus any NOTICE."""
    try:
        names = sorted(os.listdir(directory))
    except OSError as error:
        raise SystemExit(f"cannot read {directory}: {error}")
    candidates = [n for n in names if LICENSE_FILE.match(n) and os.path.isfile(os.path.join(directory, n))]
    chosen = []
    missing = []
    for license_id in [x.strip() for x in elected.split(" AND ")]:
        tokens = LICENSE_FILE_TOKENS.get(license_id, [])
        matched = [n for n in candidates if any(t in n.upper() for t in tokens)]
        if not matched:
            matched = [n for n in candidates if n.upper().split(".")[0] in GENERIC_NAMES]
        if not matched:
            missing.append(license_id)
        for name in matched:
            if name not in chosen:
                chosen.append(name)
    chosen += [n for n in candidates if n.upper().startswith("NOTICE") and n not in chosen]
    return chosen, missing


def build_notices(meta):
    packages, members, seen, _ = shipped(meta)
    elections = load_elections()
    lines = [
        "# Third-party notices",
        "",
        "Skaldsaga is licensed under AGPL-3.0-or-later (see `LICENSE` and `NOTICE`).",
        "The program is built from the Rust crates below. Each is listed with the",
        "license under which we use it (the one we elect where a crate offers a",
        "choice) and that license's text and notices, as its authors published them.",
        "",
    ]
    problems = []
    count = 0
    for pid in sorted(seen - members, key=lambda i: (packages[i]["name"], packages[i]["version"])):
        pkg = packages[pid]
        entry = elections.get(pkg["name"])
        if not entry:
            problems.append(f"{pkg['name']}: no election")
            continue
        directory = os.path.dirname(pkg["manifest_path"])
        files, missing = license_files(directory, entry["elected"])
        if missing:
            problems.append(f"{pkg['name']}: no license file found for {missing}")
        count += 1
        lines += [
            f"## {pkg['name']} {pkg['version']}",
            "",
            f"License used: {entry['elected']} (declared by the crate: {pkg.get('license')}).",
            f"Source: https://crates.io/crates/{pkg['name']}/{pkg['version']}",
            "",
        ]
        for name in files:
            with open(os.path.join(directory, name), encoding="utf-8", errors="replace") as handle:
                text = handle.read().strip("\n")
            lines += [f"### {name}", "", "~~~~", text, "~~~~", ""]
    return "\n".join(lines) + "\n", problems, count


def now():
    epoch = os.environ.get("SOURCE_DATE_EPOCH")
    moment = (
        datetime.datetime.fromtimestamp(int(epoch), datetime.timezone.utc)
        if epoch
        else datetime.datetime.now(datetime.timezone.utc)
    )
    return moment.strftime("%Y-%m-%dT%H:%M:%SZ")


def namespace_suffix():
    return os.environ.get("GITHUB_SHA") or str(uuid.uuid4())


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    for name in ("sbom", "notices"):
        p = sub.add_parser(name)
        p.add_argument("--output", required=True)
    sub.add_parser("check")
    p = sub.add_parser("matches")
    p.add_argument("--audit-info", required=True, help="JSON from `rust-audit-info BINARY`")
    args = parser.parse_args()

    meta = cargo_metadata()
    if args.command == "sbom":
        sbom = build_sbom(meta, now(), namespace_suffix())
        problems = check_sbom(sbom)
        if problems:
            print("The SBOM is incomplete:\n  - " + "\n  - ".join(problems), file=sys.stderr)
            return 1
        with open(args.output, "w", encoding="utf-8") as handle:
            json.dump(sbom, handle, indent=2)
            handle.write("\n")
        print(f"Wrote {args.output}: {len(sbom['packages'])} packages.")
        return 0
    if args.command == "notices":
        text, problems, count = build_notices(meta)
        if problems:
            print("The notices file is incomplete:\n  - " + "\n  - ".join(problems), file=sys.stderr)
            return 1
        with open(args.output, "w", encoding="utf-8") as handle:
            handle.write(text)
        print(f"Wrote {args.output}: {count} third-party crates.")
        return 0

    if args.command == "matches":
        with open(args.audit_info, encoding="utf-8") as handle:
            embedded = {(p["name"], p["version"]) for p in json.load(handle)["packages"]}
        listed = {(p["name"], p["versionInfo"]) for p in build_sbom(meta, now(), "check")["packages"]}
        if embedded != listed:
            print(
                "The SBOM and the data inside the binary list different packages.\n"
                f"  only in the binary: {sorted(embedded - listed)}\n"
                f"  only in the SBOM:   {sorted(listed - embedded)}",
                file=sys.stderr,
            )
            return 1
        print(f"The SBOM lists exactly the {len(listed)} packages embedded in the binary.")
        return 0

    sbom = build_sbom(meta, now(), "check")
    text, notice_problems, count = build_notices(meta)
    problems = check_sbom(sbom) + notice_problems
    if problems:
        print("Release files are not ready:\n  - " + "\n  - ".join(problems), file=sys.stderr)
        return 1
    print(f"SBOM ({len(sbom['packages'])} packages) and notices ({count} crates) build and pass their checks.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
