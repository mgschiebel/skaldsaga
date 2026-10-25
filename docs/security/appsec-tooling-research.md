# Application security tooling for the Rust code: research and strategy

*Research session of October 25, 2026. Label for everything below: **Research** (from web pages read this session; verify before relying on it), **Proposed** (suggested, not yet confirmed by the author) or **Decided** (confirmed by the author on October 25, 2026). The tooling itself is built in its own isolated pull request, separate from the handoff work; this file is the plan for it.*

## Decided (October 25, 2026)

1. **Blocking rule for advisories.** Only pull requests that change dependencies (`Cargo.toml` or `Cargo.lock`) are blocked by `cargo deny check`. On every other pull request the same advisory check runs but does not block; it shows as a warning.
2. **No weekly sweep.** Not waiting on a schedule: exploitation can follow disclosure within hours. Instead: Dependabot alerts and security updates turned on (they open the fix pull request as soon as GitHub's advisory database has the advisory), Dependabot version updates daily instead of weekly, and a **daily non-blocking advisory check** that opens an issue when it finds something.
3. **Remediation comes first.** While the project is in active development, every session starts by looking for open advisory issues, Dependabot pull requests and code-scanning alerts, and spins off a focused remediation pull request for each before any new feature build or planning work. The rule is in `HANDOFF.md` ("How to use").
4. **License policy (Decided; updated October 25, 2026 when the project license changed to `AGPL-3.0-or-later`, see `delivery.md`, "License and attribution").** The dependency license allowlist, the one-off review rule for anything not on it, and the reasons for each inclusion and exclusion live in one file, [`license-policy.md`](license-policy.md). Nothing else spells the list out, so changing it means editing that file (and `deny.toml` once it exists, where the same list is enforced). The rule is that everything shipped is either AGPL-3.0-or-later or on the list; contributors are told so in `CONTRIBUTING.md`. The `deny.toml` settings: `exclude-dev = true` (dev-dependencies are not shipped), build-dependencies checked as if shipped, and an explicit allowance for the workspace's own crates (`AGPL-3.0-or-later`). Not legal advice.
5. **Tool choices** as listed under "Strategy" and "Suggested pull-request plan": adopt cargo-deny, cargo-auditable (releases), CodeQL for Rust, zizmor, tightened Clippy lints and fuzzing of the loaders; defer cargo-vet; skip OSV-Scanner for now; Sighthound does not apply (no Rust support).
6. **No new defects in newly written code.** A build session finds and fixes every finding in the code it wrote, of any severity, before it opens the pull request. A finding is fixed, not silenced: a suppression (`#[allow(...)]`, a `deny.toml` ignore, a dismissed code-scanning alert) needs a written reason and an expiry, is listed in the pull request description, and needs the author's approval. Before opening the pull request the session also reviews its own diff against the SR checklist; an independent reviewer (a separate agent that did not write the code) is preferred over self-review. One script, `ci/presubmit.sh`, holds every check; CI runs the same script, so the local run and the gate cannot drift apart. The in-session rule is an instruction, not a mechanism: CI is the enforcement. Anything a cloud session cannot run (a tool that cannot be installed or reach its data there) is CI-only, and the session says so instead of skipping it silently. "No new defects" means no new findings from the tools plus the checklist; it cannot prove the absence of flaws.
7. **Baselining comes before feature work.** A baselining session, as its own pull request (or a few, split by tool if large), runs every new check over the whole existing codebase, reviews the existing code against the SR checklist, and fixes every finding or records a formal exception. Findings and their disposition go in `docs/security/baseline.md`. The new checks are made required only after the baseline is clean, so that "any finding is new" holds and feature work then proceeds under decision 6. Sequence: author actions, then the tooling pull request (checks report but are not yet required), then baselining, then the ruleset makes the checks required, then feature work resumes.
8. **Elected licenses are recorded (Decided October 25, 2026).** Many crates are dual-licensed (`ryu` is `Apache-2.0 OR BSL-1.0`; `unicode-ident` adds `AND Unicode-3.0`). The project keeps a reviewed file, `docs/security/license-elections.toml`, with one entry per shipped dependency: its declared license expression, the license we elect from it, and the reason where there is a choice. A check, built in the cargo-deny pull request, fails when a shipped crate in `Cargo.lock` has no entry, when the elected license is not part of the declared expression, when it is not on the allowlist, or when an entry names a crate no longer in the lock file.
9. **Software bill of materials (Decided October 25, 2026).** Each release carries an SBOM in SPDX JSON, attached to the GitHub Release next to the checksums. It lists the shipped dependencies, direct and transitive, with the NTIA minimum fields (supplier, component name, version, other unique identifiers such as package URLs, dependency relationships, author of the SBOM data, timestamp). Each package gets its *declared* license from the crate and its *concluded* license from the elections file. SPDX was chosen because every package has both license fields. The generator is chosen when it is built, by what the toolchain supports most easily; a small script may set the concluded license if the generator cannot. *Research (verify):* `cargo-cyclonedx` emits CycloneDX (the page read showed spec versions 1.3 and 1.4 only and a May 2024 release); `cargo-sbom` is unconfirmed; Cargo itself has only a proposal for a precursor file. Built with the cargo-auditable release pull request (plan step 4).
10. **One pull request (Decided October 25, 2026, author): the cargo-deny work and the CodeQL and zizmor work together,** with permission to stop early and continue in another pull request if it grew too long. It did not need to be split: see `build-log.md`, Build progress.
11. **Third-party notices ship with each release (Decided October 25, 2026).** MIT, BSD, Apache and the other permissive licenses generally require reproducing their copyright and license notices when a compiled binary is distributed, and the SBOM does not do that. Each release (the plugin folder and the GitHub Release) includes a generated third-party notices file listing every shipped dependency's license text and notices; Apache-2.0 crates' NOTICE files are included. The generator (for example `cargo-about`, *Research: not yet verified*) is chosen when it is built, with the SBOM and cargo-auditable release pull request (plan step 4). Not legal advice.

## Goal

No pull request merges while any security finding of any severity is open: dependency advisories, banned or badly licensed crates, source-code weaknesses, and weaknesses in the CI workflows themselves. Exceptions are possible only as a written, reasoned, expiring entry that the author approves.

## What the repo already has

| Control | Where | Covers |
|---|---|---|
| `cargo audit --deny warnings` (pinned 0.22.2), on dependency changes and weekly | `audit.yml` | Known advisories, unmaintained and yanked crates (SR-43) |
| Dependabot for cargo and Actions, weekly | `dependabot.yml` | Keeping dependencies and pinned Actions current (SR-40, SR-43) |
| gitleaks over new commits | `secret-scan.yml` | Secrets (SR-04) |
| `unsafe_code = "forbid"` for the workspace | `Cargo.toml` | No `unsafe` in our own code |
| `clippy -D warnings`, `fmt --check`, `--locked`, shellcheck | `ci.yml` | Lints, formatting, lockfile drift, shell scripts |
| Actions pinned by commit, per-job permissions, pinned toolchain | all workflows | SR-40, SR-41, SR-42 |
| Hostile-story tests | `tests/hostile_story_cli.rs` | Behaviour on hostile input (SR-52) |

The gaps are: no license, ban or source policy for dependencies; no static analysis of the Rust source beyond Clippy; no analysis of the workflow files; no dependency data inside the released binary; no fuzzing of the parsers; and "any finding blocks the merge" is not yet enforced as a required check.

## The tools you named

**cargo-deny** (EmbarkStudios). Runs four checks: *advisories* (RustSec: vulnerabilities, unmaintained, yanked), *bans* (named crates and duplicate versions), *licenses* and *sources* (which registries and git repositories crates may come from). Configured in `deny.toml` (`cargo deny init` writes a starter). It has a GitHub Action, `EmbarkStudios/cargo-deny-action`. Its docs warn that advisory checks can turn CI red when a new advisory appears, and suggest running advisories separately from the other three. **Fit: strong.** It covers three things we have nothing for, and its advisories check uses the same RustSec data as `cargo audit`.

**cargo-auditable** (rust-secure-code). Embeds the exact dependency tree, compressed, into the built binary (`cargo auditable build --release`). Afterwards `cargo audit bin <file>` can check the shipped binary, and Trivy, Grype, Syft and osv-scanner can read it too. It is not a scanner on its own. **Fit: good for releases.** It answers "is the binary we shipped affected by an advisory published later?" One thing to check before adopting: `release.yml` builds with `strip = true`, `lto` and `panic = "abort"`; confirm in a test that `cargo audit bin` still reads the data from the stripped binary.

**Sighthound** (Corgea). A tree-sitter static scanner with pattern and taint rules, MIT licensed, SARIF output. It is *written in* Rust but its rule sets cover Python, JavaScript, TypeScript, Java, Go, C#, Ruby, PHP and HTML. **Rust is not a scan target** (new languages need a grammar and rules written by us). **Fit: none today.** Revisit only if the repo gains code in one of those languages.

## Other tools worth considering

| Tool | What it does | Fit for us (**Proposed**) |
|---|---|---|
| **CodeQL for Rust** | GitHub's code-scanning engine. Rust went generally available October 14, 2025. Covers the OWASP Top 10 except vulnerable components (that is Dependabot's job). Free on public repositories. | **Adopt.** The main source-level scanner. Results appear under Security > Code scanning. |
| **Clippy, tightened** | Already run. Adds opt-in security-relevant lints (for example `unwrap_used`, `expect_used`, `panic`, `indexing_slicing`, `arithmetic_side_effects`, `cast_possible_truncation`) in `[workspace.lints.clippy]`. | **Adopt**, tuned. Free and in the pipeline already. Expect some noise; start with a short list. |
| **zizmor** | Static analysis of GitHub Actions workflows (injection, excessive permissions, unpinned or impostor actions, credential persistence). Runs offline or online; SARIF or annotations. | **Adopt.** The workflows are part of our attack surface (SR-40 to SR-45). Note: in SARIF mode it exits 0 even with findings, so use `--format=github` or a ruleset to block. |
| **OSV-Scanner** | Google's scanner over OSV data, with reusable PR and scheduled workflows. Its Action page does not mention Cargo.lock, so Rust support needs confirming on its supported-files page. | **Skip for now.** It would overlap with RustSec via cargo-deny; its value is a second database. |
| **cargo-vet** (Mozilla) | Requires a recorded human audit, or an import from a trusted organisation's audits, for every crate version. Used by Firefox, Chromium, Wasmtime and others. A 2026 survey found a median of 131 exemptions per project and about 8.7k changed lines of review per week. | **Defer.** Strongest supply-chain control, but high friction for a small team. Our dependency tree is tiny (`serde`, `serde_json`, `indexmap`, `getrandom` and their dependencies), so it may become cheap later. |
| **cargo-fuzz / property tests** | Feeds random input to parsers. | **Adopt for the loaders** (story files, markers, `--words-file`, status-line JSON). The security reviews list parsing as the main risk (SR-20 to SR-23), and `panic = "abort"` makes a crash a denial of service. Run a short fuzz pass in CI and a long one weekly. |
| **cargo-geiger, Miri** | Count `unsafe` in the dependency tree; run tests under an interpreter that detects undefined behaviour. | **Low value now.** Our own code forbids `unsafe`. Geiger could report on dependencies but I could not confirm its current maintenance; Miri matters only if we write `unsafe`. |
| **OpenSSF Scorecard** | Scores repository practices (branch protection, pinned dependencies, token permissions). | **Optional**, reporting only. Useful before the public release. |
| **Trivy or Grype** | Scan the released binary or artifacts using cargo-auditable data. | **Optional**, after cargo-auditable lands. `cargo audit bin` does the same job with a tool we already use. |

Things I could not confirm this session: cargo-geiger's current status, the present state of Semgrep's Rust rules (the only page I could read was from 2021, so I left Semgrep out), and Trivy and Grype behaviour on stripped binaries.

## Strategy

**1. Two lanes, so a new advisory never blocks unrelated work (Decided).**

- *Blocking gate* (required checks): `cargo deny check` for all four checks on pull requests that change dependencies; CodeQL, zizmor, Clippy with the tightened lints, tests, a short fuzz pass and gitleaks on every pull request.
- *Non-blocking advisory lane:* on pull requests that do not change dependencies, the advisory check still runs and reports as a warning. A **daily** run (and a run on pushes to `main`) does the same for the whole tree and opens or updates one issue when it finds something. The fix arrives as a separate dependency-update pull request, which Dependabot security updates can open within hours of the advisory reaching GitHub's database; that pull request changes dependencies, so it is gated and must pass.
- *The limit:* any scan is only as fast as the advisory databases (RustSec, GitHub). A schedule cannot find what has not been published yet. Updating quickly also cuts both ways, since a malicious release reaches us as an update pull request; that is the case cargo-vet and build-script review address, and why cargo-vet is deferred rather than dropped.

**2. "Any severity" has to be configured in each tool, because the defaults do not do it.**

- cargo-deny: `deny.toml` with `unmaintained = "all"`, `yanked = "deny"`, vulnerabilities denied, `unknown-registry = "deny"`, `unknown-git = "deny"`, `wildcards = "deny"`.
- CodeQL: code-scanning results do not fail a check unless the ruleset's "Code scanning results" rule is set to block on alerts of every severity (security and non-security).
- zizmor: `--format=github`, or require the job; do not rely on SARIF alone.
- Clippy: already `-D warnings`.

**3. Exceptions have a paper trail.** Every `ignore` in `deny.toml`, and every dismissed CodeQL alert, needs a reason and an expiry date, and is listed in `docs/security/status.md`. A small CI check fails when an exception is past its date.

**4. Enforcement lives in the ruleset, not in the workflows.** Once the new checks have run clean for a while, add them as required status checks in the "Main protect" ruleset (the status page already says to consider this for SR-50). A check that is not required does not block anything.

**5. Release hardening.** Build releases with `cargo auditable build --release --locked`, run `cargo audit bin` on the tarball's binary before publishing, and keep the existing checksum step. Provenance (`actions/attest-build-provenance`) stays on its current SR-44 plan.

**6. License policy (Decided).** See Decided items 4, 8 and 9, and [`license-policy.md`](license-policy.md) for the list and the research behind it. If the project's own license changes again, revisit that file.

## Suggested pull-request plan (**Proposed**; the policy above is Decided)

0. **Author actions in GitHub settings** (no pull request): turn on Dependabot alerts and security updates (`github-settings-checklist.md`, section 3); later, make the new jobs required in the "Main protect" ruleset.
1. **deny.toml and the cargo-deny jobs.** Replaces `cargo audit` in `audit.yml` (same data) and adds bans, licenses and sources. Blocking job for dependency-changing pull requests; non-blocking warning job for the rest; a daily scheduled run that opens or updates a labelled issue (`security-advisory`). Change Dependabot's cargo and Actions schedules to daily. Pin the Action by commit and the tool by version (SR-40, SR-42). Keep `cargo audit bin` for the release binary (step 4). Also adds the license allowlist (Decided item 4), `docs/security/license-elections.toml` and its check (Decided item 8); the workspace crates get an explicit allowance.
2. **CodeQL and zizmor workflows.** Per-job minimal permissions; pin everything.
3. **Tightened Clippy lints**, fixing or justifying what they flag.
4. **cargo-auditable in `release.yml`**, with the stripped-binary test (**built October 25, 2026**, `build/release-supply-chain`; the data survives `strip = true`, LTO and `panic = "abort"`: tested with `rust-audit-info` and `cargo audit bin`; the SBOM generator is our own script, `ci/release-files.py`, because it needs the elected license per package, which no off-the-shelf generator has). The same pull request adds the SPDX SBOM per release (Decided item 9) and the third-party notices file (Decided item 11).
5. **Fuzz targets** for the loaders and a CI smoke run (**built October 25, 2026**, `build/fuzz-targets`; the loaders are fuzzed as text, not through files, and a long weekly run is left for later).
6. **`ci/presubmit.sh`** running every check, called by CI and by build sessions (Decided item 6); the exception-expiry check and the flag on new suppressions (**built October 25, 2026**, `build/exception-expiry`: the register is `docs/security/exceptions.toml`). Add the presubmit line and a diff-review checkbox to `CLAUDE.md` and the pull request template. Test which tools a cloud session can run and note the rest as CI-only.
7. **Baselining pull request** (Decided item 7): run everything over the existing code, fix or formally except each finding, write `docs/security/baseline.md`. Until this is done the new jobs report but do not block, because the existing code would fail them. **Built October 25, 2026** (`build/baselining`): the record is `docs/security/baseline.md`; zizmor now blocks and is part of `ci/presubmit.sh`.
8. **Ruleset update** making the new jobs required (an author action in GitHub settings), after the baseline is clean.
7. Each step updates `docs/security/status.md`, the SR-xx rows it touches, and the handoff's Status and Next blocks (the repository's own rule).

Steps 1 and 2 give the most protection for the least effort and could be one pull request. They were built as one pull request (Decided item 10).

## Sources

- [cargo-deny documentation](https://embarkstudios.github.io/cargo-deny/) and [its checks page](https://embarkstudios.github.io/cargo-deny/checks/index.html)
- [cargo-deny-action](https://github.com/EmbarkStudios/cargo-deny-action)
- [cargo-auditable](https://github.com/rust-secure-code/cargo-auditable)
- [Sighthound repository](https://github.com/Corgea/Sighthound) and [Corgea's Sighthound page](https://corgea.com/sighthound)
- [CodeQL Rust general availability (GitHub changelog)](https://github.blog/changelog/2025-10-14-codeql-scanning-rust-and-c-c-without-builds-is-now-generally-available)
- [OSV-Scanner GitHub Action](https://google.github.io/osv-scanner/github-action/)
- [zizmor integrations](https://docs.zizmor.sh/integrations/)
- [A look at cargo-vet in 2026](https://www.lightsquares.dev/blog/cargo-vet-in-2026)
- [Rust and Cargo supply chain security](https://www.systemshardening.com/articles/cicd/rust-cargo-supply-chain-security/)
