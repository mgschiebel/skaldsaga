# Application security tooling for the Rust code: research and strategy

*Research session of October 25, 2026. Label for everything below: **Research** (from web pages read this session; verify before relying on it) unless marked **Proposed** (my suggestion, not yet confirmed by the author). Nothing here is **Decided**. This is meant to become its own isolated pull request, separate from the handoff work.*

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

## Strategy (**Proposed**)

**1. Two lanes, so a new advisory never blocks unrelated work.**

- *PR gate* (required checks, run on every pull request): `cargo deny check` for all four checks, CodeQL, zizmor, Clippy with the tightened lints, tests, a short fuzz pass, gitleaks.
- *Scheduled sweep* (weekly, plus on `main`): the same dependency and CodeQL scans against the whole tree, because new advisories appear for code that has not changed. A failure opens an issue and is fixed in its own pull request. The Dependabot update that fixes it must be able to merge, so the gate must pass on that PR.

The open question for the author is whether a brand-new advisory should block every pull request (strictest reading of the goal) or only the one that fixes it (what the cargo-deny docs recommend). My recommendation: block only pull requests that change dependencies; the weekly sweep catches the rest.

**2. "Any severity" has to be configured in each tool, because the defaults do not do it.**

- cargo-deny: `deny.toml` with `unmaintained = "all"`, `yanked = "deny"`, vulnerabilities denied, `unknown-registry = "deny"`, `unknown-git = "deny"`, `wildcards = "deny"`.
- CodeQL: code-scanning results do not fail a check unless the ruleset's "Code scanning results" rule is set to block on alerts of every severity (security and non-security).
- zizmor: `--format=github`, or require the job; do not rely on SARIF alone.
- Clippy: already `-D warnings`.

**3. Exceptions have a paper trail.** Every `ignore` in `deny.toml`, and every dismissed CodeQL alert, needs a reason and an expiry date, and is listed in `docs/security/status.md`. A small CI check fails when an exception is past its date.

**4. Enforcement lives in the ruleset, not in the workflows.** Once the new checks have run clean for a while, add them as required status checks in the "Main protect" ruleset (the status page already says to consider this for SR-50). A check that is not required does not block anything.

**5. Release hardening.** Build releases with `cargo auditable build --release --locked`, run `cargo audit bin` on the tarball's binary before publishing, and keep the existing checksum step. Provenance (`actions/attest-build-provenance`) stays on its current SR-44 plan.

**6. Licence policy needs an author decision.** The workspace is `GPL-2.0-only`. As far as I know, Apache-2.0 is not compatible with GPLv2-only (it is with GPLv3). Most Rust crates are `MIT OR Apache-2.0`, which is fine because MIT can be chosen, but an Apache-2.0-only crate would not be. The `deny.toml` license allowlist should reflect that; please confirm the rule you want. Verify the compatibility point before relying on it.

## Suggested pull-request plan (**Proposed**)

1. **deny.toml and the cargo-deny job.** Replaces the advisories part of `audit.yml` (same data) and adds bans, licenses and sources. Keep the weekly schedule. Pin the Action by commit and the tool by version (SR-40, SR-42).
2. **CodeQL and zizmor workflows.** Per-job minimal permissions; pin everything.
3. **Tightened Clippy lints**, fixing or justifying what they flag.
4. **cargo-auditable in `release.yml`**, with the stripped-binary test.
5. **Fuzz targets** for the loaders and a CI smoke run.
6. **Ruleset update** making the new jobs required (an author action in GitHub settings), and the exception-expiry check.
7. Each step updates `docs/security/status.md`, the SR-xx rows it touches, and the handoff's Status and Next blocks (the repository's own rule).

Steps 1 and 2 give the most protection for the least effort and could be one pull request.

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
