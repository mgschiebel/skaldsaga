# Build, release, settings and testing

*Build and release rules, default settings, testing and evals. Load it for pipeline or settings work. Part of the Skaldsaga handoff; the index is [`HANDOFF.md`](../../HANDOFF.md). Moved verbatim from HANDOFF.md (v9, last edited October 24, 2026) on October 24, 2026: the wording below is unchanged.*

<!-- begin moved text -->

## Build, release and delivery (new in v9; all Decided unless marked)

**Phase 1 scope (Decided):** phase 1 is **not a public release**. It builds for **Linux x64 only**, which covers the author's Linux development environment and cloud Claude Code sessions (also Linux). No signing in phase 1. The macOS and Windows install checks move to the public-release phase. The targets, cargo-dist and signing items below are the **public-release plan**. `skaldsaga-core` stays portable (no desktop-only dependencies; file and process access behind a small interface), but no macOS or Windows runners until the CI/CD pipeline is built on the author's desktop in a later phase.

**Phone-friendly pipeline (Decided for phase 1):** the author may work from the Claude mobile app without a local Claude Code install for a while.

1. **Write and test in a cloud session:** Claude writes the Rust code in the cloud workspace (which has the Rust toolchain and push access) and runs `cargo build`, `cargo test` and `cargo clippy` before pushing. One branch per task (for example `build/meter-pilot`), never straight to `main`.
2. **GitHub Actions:** format, lint and test on every push and pull request; on a version tag, build the Linux x64 binary, publish a GitHub Release, and assemble the distribution branch.
3. **Review on the phone:** Claude opens a pull request with a plain-language summary; the author reviews and merges in the GitHub mobile app. Claude checks CI results and fixes failures in-session (the GitHub CLI login in the cloud workspace was failing on October 18 and may need fixing).
4. **Later phases:** work moves to the author's desktop Claude Code environment, reached remotely from the phone (likely Claude Code's remote-control feature, *verify*), where the full CI/CD pipeline will be built.
5. **Needs a real machine later:** a hands-on plugin install test on the author's Linux machine (executable permission, status line meter). Cloud-session support can be partly tested from a cloud session itself.


- **One binary with subcommands** (`init`, `check`, `commit`, `index`, `meter`, `complete`), built as a core library (`skaldsaga-core`) plus a thin command-line front end. One file to deliver and sign; the later standalone app reuses the core. (Confirms the v8 proposal.)
- **Compiled binaries stay out of `main`'s history**; `main` holds source only. (Confirms v8.)
- **The session meter is the pilot**: small, but it exercises status line wiring, cross-platform builds, delivery and signing before the harder scripts are written. (Confirms v8.)
- **Delivery: a CI-built distribution branch** (`dist`, or a separate marketplace repo if that proves simpler) holding the plugin files plus every platform's binary, built together from one tagged source version. The writer's plugin installs from there. No network or download tool is needed at first run, and plugin files and binaries cannot mismatch. Cost: the installed plugin carries all platforms' binaries (likely a few tens of megabytes). Rejected: binaries on `main` (bloats every clone) and first-run download (a failure path at the worst moment for a novice).
- **Binary location:** `scripts/bin/<platform>/`, never a top-level `bin/`; a tiny launcher picks the right one and checks the version.
- **Launch targets (public release):** macOS universal (Apple Silicon and Intel), Windows x64, Linux x64, Linux ARM. Windows on ARM uses the x64 build under emulation until there is demand.
- **CI tooling:** cargo-dist for cross-platform builds and GitHub Releases; our CI assembles the distribution branch from its output.
- **Versioning:** one version number shared by plugin and binary; the launcher compares the binary's version with the plugin manifest and explains plainly on a mismatch.
- **Signing, staged (public release; none in phase 1):** unsigned during early testing (with a plain note about the warning); before public release, macOS signing and notarization (Apple Developer account, about $99 a year, *verify*) and a low-cost Windows signing service such as Microsoft's Trusted Signing (*verify* availability and price). Keys live as CI secrets on the author's GitHub account, never in the repo.
- **Phones (no phase 1 targets):** Claude Code does not run locally on phones and iOS has no shell, so native mobile binaries would have nothing to run in. Phone users reach Skaldsaga through cloud Claude Code sessions from the Claude mobile app, which run on Linux and use the Linux binary. `skaldsaga-core` stays portable from day one (no desktop-only dependencies; file and process access behind a small interface) so a native mobile app remains possible (see Future phases).
- **To verify before building:** whether a plugin install keeps the executable permission on bundled files (if not, setup fixes permissions); whether cloud sessions support the plugin, its hooks, and the status line the meter depends on. *Public-release phase:* whether macOS quarantines files a plugin install writes.

## Default settings (new in v9; all Decided, all tunable in `.harness/settings.json`)

| Setting | Default | Reasoning |
|---|---|---|
| Meter: recommend wrapping up | 50% of context used | Models get less sharp as context fills, so the "smart window" ends well before 100%. |
| Meter: wrap up regardless | 70% | Leaves room to finish the question, save, and write the recap. |
| Rolling housekeeper pass | Every 5 saved answers | Raises issues while the topic is live without a pass running constantly. |
| Reader cap for a scoped pass | 25 readers | Above this, a full pass at the next session start is better. |
| Full pass every N sessions | Every 10 sessions | Catches slow drift that incremental passes miss (adopted from Proposed). |
| Flags per natural pause | 2 | Flag budget. |
| Strict lane at fan-out | 10 other nodes reading what changed (`routing.strict_fan_out`, step 7) | High fan-out escalates a proposal to the strict lane. |
| Pacing note thresholds | Under 0.5x or over 2x a movement's expected share | Pacing heuristic. |
| Models per sub-agent | As in Architecture | Unchanged. |

**Existing personal status line:** setup detects it and explains in one plain sentence; the meter wraps it (the writer still sees their own line, and usage is recorded). If wrapping fails, setup asks before replacing it.

## Testing and evals (new in v9; all Decided)

**1. Script tests (ordinary Rust tests, run in CI on every change, must always pass).** Fixture spec folders with expected results for everything deterministic: gating lanes, quarantine, version-checked commits, the held-proposal flow, the uniqueness rule for all three cardinalities, delete blocking, merge, index rebuild, and every script soft check.

**2. Judgment evals (housekeeper and completeness rubric).**

- **A small library of short test stories,** each a real spec folder of roughly 15 to 25 scenes.
- **Seeded defects,** one or two per story, each labeled with the expected finding, together covering every housekeeper judgment check and every rubric judgment.
- **Clean controls** with no defects, to measure false alarms; for a novice, false alarms harm as much as misses.
- **Scoring:** seeded defects caught, and false flags raised.
- **Model comparison:** a cheaper model becomes a default only if it catches at least about 90% of what the strong model catches, with no more false alarms. Rolling scoped passes are tested this way too.
- **Authoring:** Claude may draft test stories, but a human reviews every seeded defect, because an accidental second plot hole skews the scores. (The skill-creator skill supports running evals.)

**3. Interview evals and the slice test plan (*Proposed October 25, 2026*, planning session 1; the author's idea of test-first slices).**

- **Every vertical slice gets a short test plan, written before building,** naming the one thing the slice must prove. Tests come first where they can (TDD).
- **Entry routing is deterministic and tested with script tests.** The program, not the model, decides which entry condition applies, from the folder state and whether the command had text, so story text never steers the routing. Fixture folders cover every entry condition, including ones whose conversation is not built yet (marked pending until their slice).
- **Interview evals score properties, not exact words,** using scripted writer inputs for each entry condition: one question per turn; no internal terms in anything the writer sees; the right closing move for the condition (reword of the last answer, one recommendation, or "I'm not finding a story"); no `init` before the writer confirms a new story. A deterministic scan for internal terms in writer-facing prompt text can run as an ordinary script test.


