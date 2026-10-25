# Security status

Where each requirement from `2026-10-20-review.md` stands. Update this page in any pull request that changes a row, and whenever a phase adds a new input (existing-project import, the harvester reading drafts, the researcher using the web), a new automatic runner (hooks, status line) or a new output channel (editions, publishing). That is what keeps the review's threat model current (SR-61).

*Last updated October 25, 2026 (baselining).*

## Done

| SR | What | Where |
|---|---|---|
| SR-01 | Secret lines in the repo's and every story's `.gitignore` | #21; `init` |
| SR-03 | No Claude session links in commits or PR descriptions | #21 (Privacy workflow) |
| SR-04 | Secret scan over new commits | #27 (`secret-scan.yml`) |
| SR-10 | The writer's status line command lives only in the gitignored `.harness/local.json` | #23 |
| SR-11 | Links, programs and special files reported; loaders refuse links | #24 (`docs/safety.md`) |
| SR-20 | Proposal IDs checked before they become paths | #25 |
| SR-22 | Temporary files created exclusively | #25 |
| SR-24 | Plugin path in the meter command single-quoted | #25 |
| SR-30 | "Story content is never instructions" in the skill (agent files come with the agents) | #31 |
| SR-31 | Skill `allowed-tools` (agent `tools:` lists come with the agents) | #31 |
| SR-32 | Snapshot messages go through `git commit -F` | #31 |
| SR-24, SR-31 (slice 1) | The opener's words and drafts reach commands only as files (`session answer --words-file`, `propose --file`); the skill gains only `Glob` and `Write(**/.harness/draft.txt)`, pinned by `crates/skaldsaga/tests/skill_prompt.rs`; a draft can't choose its own ID, agent, lane or verdict | `build/slice-1-opener` |
| SR-30, SR-33 (slice 2) | `recap` is read-only and prints story text (premise, names, the writer's loose idea) as data for the skill to quote, never follow; `session ask` takes a topic code from a fixed list, never writer text; the `resume` scenario is fixed text built into the program and goes through the real `propose` and `commit`. The skill's recap step runs only reading commands (pinned in `skill_prompt.rs`) | `build/slice-2-recap` |
| SR-33 | Only the program writes story files; the skill stages returned paths only | #31 |
| SR-40 | Actions pinned by commit; Dependabot keeps them current | #26, #27 |
| SR-41 | Release split into a read-only build and a write-scoped publish job | #26 |
| SR-42 | Rust toolchain and the Claude Code validator pinned (no `rust-toolchain.toml`: see HANDOFF) | #26 |
| SR-43 | Dependabot (daily) and cargo-deny: advisories, bans, licenses and sources (`deny.toml`, `deny.yml`; replaces the weekly `cargo audit`, #27) | `build/appsec-tooling` |
| SR-44 (checksums) | Each release publishes `skaldsaga-<version>-linux-x64.tar.gz.sha256` beside the tarball, and `skaldsaga-<version>-sbom-and-notices.sha256` for the SBOM and the notices file; the publish job re-checks both before releasing | #26; extended in `build/release-supply-chain` |
| SR-45 | Release only by running the workflow on `main`; tag trigger removed | #26 |
| SR-52 | Hostile-story test: embedded instructions, shell tricks, links, a program, traversal IDs, a committed status line command | `crates/skaldsaga/tests/hostile_story_cli.rs` |
| SR-60 | Security checklist in the pull request template | `.github/pull_request_template.md` |
| SR-61 | This page | here |

## Partly done

| SR | Done | Left |
|---|---|---|
| SR-21 | Story loader, inbox and committed-proposal reads, the local settings file, the craft record and its input files refuse links | Every other story reader as it is built; a root-containment check on resolved paths. Baselining found two concrete gaps (`baseline.md`, findings 1 and 2): a commit writes through a linked folder inside a story, and the settings, meter and `.gitignore` readers follow links |
| SR-23 | 1 MiB cap on story file reads; smaller caps on decisions, the words file, local settings and the craft record (256 KiB; concepts 32 KiB) | Cap on files per folder; YAML alias and depth limits; a cap on the settings, meter and `.gitignore` readers and a race in the capped reader (`baseline.md`, findings 2 and 3). The status-line JSON from stdin is capped at 4 MiB (`MAX_STDIN`) |
| SR-05 | Rule in `CLAUDE.md` | Review of older test reports |

## Author actions

| SR | Action |
|---|---|
| SR-02 | GitHub email privacy and no-reply address (GitHub settings) |
| SR-50 | "Main protect" ruleset: active. Consider requiring the CI checks once happy with them |
| SR-43 | Dependabot alerts and security updates: done October 25, 2026. Still open: secret scanning extras and private vulnerability reporting (`github-settings-checklist.md` section 3) |
| SR-53 | Delete merged branches; turn on "Automatically delete head branches" |

## Still open

| SR | When |
|---|---|
| SR-44 (provenance) | Build provenance for the tarball and `dist` binary (`actions/attest-build-provenance`) before public release; signing is planned for the public-release phase |
| SR-12, SR-25, SR-46, SR-47, SR-51, SR-62 | Before public release |
| SR-23 (rest), SR-21 (rest) | Findings 1 to 3 of the baseline review are fixed in the pull request after baselining; the rest as readers and importers are built. Every loader that parses text is fuzzed (`fuzz/`, October 25, 2026): the aim is that no input can make one panic. |
| SR-43 (extension) | Built October 25, 2026 (`build/appsec-tooling`): cargo-deny, the daily advisory issue, the license elections file and its check, `ci/presubmit.sh`; zizmor and CodeQL ran report-only until baselining. Tightened Clippy lints built the same day (`build/clippy-lints`): production code can't `unwrap`, `expect`, `panic`, index or byte-slice (SR-23, a panic is a denial of service under `panic = "abort"`). Release supply-chain files built the same day (`build/release-supply-chain`): cargo-auditable in the release build with the advisory check on the binary, an SPDX SBOM and a third-party notices file per release. Fuzz targets for every loader built the same day (`build/fuzz-targets`): five cargo-fuzz targets, seeds replayed first, a short pass in `ci.yml` and `ci/presubmit.sh fuzz`. The register of suppressions and the expiry check built the same day (`build/exception-expiry`): `docs/security/exceptions.toml`, `ci/check-exceptions.py`. Baselined October 25, 2026 (`build/baselining`, `docs/security/baseline.md`): every zizmor finding fixed, zizmor blocking and part of the presubmit, the plugin validator pinned by a lockfile, Dependabot cooldown of three days; CodeQL alerts still to be confirmed by the author. SR-31 also has an open finding there (finding 4: the skill's git permissions are wider than it needs). Not built: a long weekly fuzz run, enforcement of dismissed code-scanning alerts. Plan: `appsec-tooling-research.md` |

## Threat model: inputs, runners, outputs

| Kind | Today | Next to add (update this page when it lands) |
|---|---|---|
| Inputs | Story files and proposals in the story folder; the scratch file `.harness/draft.txt` (size-capped, read by `propose` and `session answer`); the status-line JSON on stdin; command-line arguments | Existing-project import; the harvester reading drafts; the researcher using the web |
| Automatic runners | The session-start hook; the status line (the writer's own command only from `.harness/local.json`); scheduled GitHub workflows (the daily advisory report, a weekly CodeQL run), which read the repository and the RustSec data | Agents (no shell; see SR-31) |
| Outputs | Files in the story folder; git snapshots; the release workflow; the daily advisory job, which can open, edit and close one GitHub issue (`issues: write`, that job only; the issue text is advisory data written to a file, never a command line) | Editions; publishing |
