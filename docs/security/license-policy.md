# License policy

*The one place that spells out which licenses Skaldsaga accepts. Every other file refers here and does not repeat the list, so changing the list means editing this file and `deny.toml` and nothing else. Decided October 25, 2026; the reasoning and research are in [`appsec-tooling-research.md`](appsec-tooling-research.md) (Decided items 4, 8 and 11). Not legal advice.*

## The rule

Everything that goes into the repository or a release is either:

1. licensed **AGPL-3.0-or-later** (the project's own license; see `LICENSE` and `NOTICE`), or
2. under a license on the **allowlist** below.

This covers dependencies, copied or adapted code, text, data and other assets. Anything else is a **one-off decision**: research whether the license is compatible with AGPL-3.0-or-later, then either add it to the allowlist (with the reason and the author's approval) or use something else. Nobody adds an unlisted license quietly.

Dependencies in the normal graph and build-dependencies count as shipped. Dev-dependencies and pipeline tools (cargo-deny, cargo-audit, zizmor, CodeQL, gitleaks) are not shipped and are outside the rule.

## The allowlist

SPDX ids, as `cargo deny` reads them. For a dual-licensed crate (`A OR B`), one side only needs to be on the list, and the license we choose is recorded in [`license-elections.toml`](license-elections.toml).

- MIT
- X11
- BSD-2-Clause
- BSD-3-Clause
- ISC
- Zlib
- BSL-1.0
- NCSA
- Unlicense
- CC0-1.0
- Unicode-3.0
- Apache-2.0

**Where the machine-checked copy lives:** `deny.toml` (`[licenses] allow`) is the list `cargo deny` enforces. `ci/check-license-sync.sh` fails CI when this section and `deny.toml` are not the same set, so edit both together. `ci/check-license-elections.py` checks that every shipped dependency has an entry in `license-elections.toml` whose elected license is allowed here.

## Why these, and what is left out

- **Permissive licenses the FSF lists as GPL-compatible** (everything above except Apache-2.0): the FSF lists no permissive license that works with GPLv2 but not GPLv3, so they all stay valid under the AGPL (the AGPL is GPLv3 plus one network clause).
- **Apache-2.0** was added when the project moved from `GPL-2.0-only` to AGPL-3.0-or-later: the FSF and the Apache Software Foundation say it is compatible with GPLv3 and not GPLv2. *Research (verify):* neither says so for the AGPLv3 explicitly; that step is an inference.
- **Left to one-off review (permissive):** ECL-2.0, FreeType and XFree86-1.1 (GPLv3-only per the FSF; rare in Rust; the last two carry attribution or acknowledgment requirements), and `Apache-2.0 WITH LLVM-exception` (it only adds permission, so it should be at least as compatible; not verified).
- **Left off on purpose (copyleft):** MPL-2.0 (indirectly compatible with the AGPL through its section 3.3), LGPL-2.1, LGPL-3.0, Artistic-2.0 and the rest of the compatible copyleft licenses, because they carry extra obligations. Also not listed: 0BSD and WTFPL (not covered or not recommended by the FSF).
- **Unicode-3.0** is on the list because `unicode-ident` needs it. The FSF entry is named "Unicode"; this is the entry to recheck.

## Changing the list

Edit the allowlist section above, add a line to the history below, and make the same change in `deny.toml`. The author approves every change.

## History

- October 25, 2026: first list, eleven permissive licenses, while the project was `GPL-2.0-only` (Apache-2.0 excluded).
- October 25, 2026: project license changed to AGPL-3.0-or-later; Apache-2.0 added.
- October 25, 2026: `deny.toml` and the elections file built (tooling pull request); sync and elections checks added to CI.
