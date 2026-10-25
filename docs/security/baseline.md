# Baseline: every check run over the existing code

*October 25, 2026 (pull request: branch `build/baselining`). Plan step 7 in [`appsec-tooling-research.md`](appsec-tooling-research.md); the decision is in `docs/handoff/security.md`, "Application security tooling". This is the record of where the project stood when the new checks first ran over all of it, and what was done about each finding.*

Labels: **Decided** is confirmed by the author, **Proposed** is not yet.

## Result

The tooling findings are all fixed and no suppression was added. The review of the code against the security requirements (SR) found five points: four are open and will be fixed together in the next pull request (the author chose one pull request for all fixes after baselining), and one was a stale line in `status.md`, fixed here.

| Check | How it was run | Result |
|---|---|---|
| Format, Clippy (`-D warnings`, with the tightened lints), tests | `sh ci/presubmit.sh` | Clean |
| ShellCheck | presubmit | Clean |
| cargo-deny (advisories, bans, licenses, sources) | presubmit | Clean |
| License checks, release files (SBOM, notices), suppression register | presubmit | Clean (17 suppressions, all on the register) |
| Fuzzing | five targets, 30 seconds each on the pinned nightly (the presubmit default is 10) | No crash, about 7.3 million inputs in all |
| zizmor (workflows and the Dependabot file) | `zizmor --offline --persona auditor .github`; CI ran the same audit online | 12 findings, all fixed (next section); now 0 |
| gitleaks | CI only | Green on `main`; not run again here |
| CodeQL | CI only | The job is green on `main`. **Open:** the open alerts could not be read from the cloud session (the code-scanning API answers HTTP 403). The author confirms the Security tab shows none, or lists them here. No dismissed alert is known. |

## zizmor: 12 findings, all fixed

| # | Rule | Where | What was done |
|---|---|---|---|
| 1 to 3 | `dependabot-cooldown` | `dependabot.yml`, three ecosystems | **Decided:** Dependabot waits three days after a release before it offers a version update. Security updates are exempt (GitHub's documentation and news in July 2026; **Research**). Written into every ecosystem. zizmor's own threshold is seven days, so `.github/zizmor.yml` sets it to three. That is a threshold, not a suppression, so it is not on the register. |
| 4 to 6 | `concurrency-limits` | `ci.yml`, `privacy.yml`, `secret-scan.yml` | One group per workflow and branch; a newer run cancels an older one only on pull requests, so every push to `main` is still checked. |
| 7, 8 | `superfluous-actions` | `ci.yml`, `release.yml` | The third-party toolchain Action is gone; `rustup` installs the pinned 1.97.0, as the fuzz job already did. One fewer third-party Action to trust (SR-40). |
| 9 | `adhoc-packages` | `ci.yml` | The Claude Code plugin validator was installed from npm with no lockfile. It is now `ci/claude-code/` (`package.json` pinned to 2.1.310, `package-lock.json` with checksums) installed by `npm ci`, and Dependabot watches it. The install script has to run, because that is how the native program is put in place; with scripts off the program does not start. Checked here: `npm ci` gives 2.1.310 and `claude plugin validate --strict` passes on the assembled plugin. It is a test tool and never shipped, so the license rule for releases does not apply (`security.md`: pipeline tools are outside it). |
| 10 | `artipacked` | `privacy.yml` | `persist-credentials: false`. |
| 11 | `artipacked` | `release.yml`, publish job | `persist-credentials: false`, and the one step that pushes the `dist` branch passes the token through environment variables (`GIT_CONFIG_*`) for that step only, so nothing is written to `.git/config`. The derived value is masked, because GitHub masks only the token itself. This is the one change that touches how a release publishes: see "Release dry run" below. |
| 12 | `undocumented-permissions` | `release.yml` | A comment says why the publish job may write. |

`zizmor.yml` no longer passes `--no-exit-codes`, so the job fails on any finding, and `sh ci/presubmit.sh zizmor` runs the same audit offline (zizmor is now part of the presubmit).

### Release dry run

Started from this branch with `dry_run` on, which builds everything and publishes nothing; `git push --dry-run` still has to authenticate with the new token handling. Result: RESULT_PLACEHOLDER

## Review of the code against the security requirements

**What was reviewed.** Not a line-by-line read of the roughly 15,000 lines of production code. The checks cover panics (the Clippy lints, 5 fuzz targets) and dependencies. The review looked at the places the requirements are about: every process launch, file-system call, path built from a variable, environment read, network use and `unsafe`, then read the code at each one; plus the skill, the hook, the launcher and the workflows. One suspicion was tried against the built program rather than argued (finding 1).

**Reviewed, no finding:**

- No `unsafe` code and no network code anywhere.
- One process launch in production code: the writer's own status line command (`wrapped.rs`). It comes from the uncommitted local settings, runs with a 5-second limit and a 64 KiB output cap, and gets no story text in its command line (SR-10, SR-24). The other two launches are in tests.
- Every ID that becomes a file name is checked first: proposal IDs (`commit.rs`), node and fact IDs (`node_file::parse_node`, which every create and every load goes through), writer decision IDs, and craft concept IDs (`is_slug`, with a test for hostile ones). Names that come from a folder listing never hold a separator (SR-20).
- All file access in production code goes through `fsio`, apart from the stdin reads (capped at 4 MiB) and one link check in `propose.rs`. Temporary files are created exclusively and renamed into place (SR-22).
- The skill may write only `.harness/draft.txt`; the hook runs through `sh` with the plugin path as a separate argument and does nothing without `.harness/` (SR-24, SR-31).

**Findings.** Each stays open until the next pull request, which fixes them together (**Decided**: one pull request for everything found at baselining). None is an exception: nothing was silenced.

1. **A linked folder inside a story is written through (SR-21; medium).** Reproduced: with `spec/people` and the other node folders replaced by links to a folder outside the story, `commit` created the node files in the outside folder. `check` already reports the links, but a commit goes ahead anyway. Files are only created or removed under the program's own ID-shaped names (`<id>.md`), which limits the damage, but a shared or cloned story should not be able to point a write outside itself. Fix: before any write, refuse a path that goes through a link inside the story folder, and make the commit stop with a plain message.
2. **Several reads have no size cap and follow links (SR-21, SR-23; low to medium).** `fsio::read_optional` and `read_json_object` read the whole file and follow links. They read `.harness/settings.json` (`settings.rs`, `init.rs`), `.harness/meter.json` (`meter.rs`), `.claude/settings.json` in the story (`statusline.rs`) and `.gitignore` (`init.rs`); a shared story can plant a huge file or a link to a device there. Fix: one cap (1 MiB) for all of them, and refuse links for the files that live in the story folder. The writer's own settings file in their home folder stays link-friendly, since many people keep it as a link to a dotfiles folder.
3. **The capped reader checks before it reads (SR-23; low).** `read_regular_capped` looks at the size and the link once and reads afterwards, so a file swapped or grown in between is read in full. Fix: open once, check what was opened, and read at most the cap plus one byte.
4. **The skill's git permissions are wider than it needs (SR-31; medium).** `allowed-tools` lists whole commands (`git diff`, `git remote`, `git commit`, `git add`) with any arguments. In git, `git diff --output=<file>` writes any file, `git remote add` or `set-url` changes where the folder would push, and `git commit -m` takes any text. The story text is never supposed to reach them (SR-30), but the list is the second line of defence. The skill only uses these forms: `git init -q`; `git add -- <paths>`; `git rm --cached --ignore-unmatch -q -- <paths>`; `git ls-files --error-unmatch -- <path>`; `git diff --cached --quiet`; `git commit -q -m "<fixed text>"`; `git commit -q -F <file>`; and `git remote` with no arguments. Fix: narrow the list to those forms, update the test that pins it (`skill_prompt.rs`), and try it from the phone, because a wrong pattern shows up as a permission prompt in the middle of a save. This one changes the skill, so the pull request carries a version bump and a release.
5. **`status.md` was out of date (fixed in this pull request).** It listed a cap on the status line's input as still to do; it exists (`MAX_STDIN`, 4 MiB). The rows for SR-21 and SR-23 now list findings 1 to 3 instead.

**Not covered here:** the requirements marked "before public release" (SR-12, SR-25, SR-46, SR-47, SR-51, SR-62) and build provenance (SR-44); the CodeQL alerts (above); a line-by-line read of the code.

## Exceptions

None added. `docs/security/exceptions.toml` is unchanged: its two entries (the integration-test `#![expect(...)]` lines and the four `clippy.toml` test settings) still carry the **Proposed** expiry of April 25, 2027; the author sets the real dates.

## What the author does next

1. Review and merge this pull request.
2. Check the CodeQL alerts (above).
3. In the "Main protect" ruleset, make these checks required: Format, lint, test; Bans, licenses, sources (and Advisories); Analyze (Rust); Audit workflows; Fuzz smoke; Secret scan; Privacy checks. Steps are in `github-settings-checklist.md`, sections 1 and 3. Merge first: a check that is required before it has run on `main` blocks every pull request.
4. Then the next pull request fixes findings 1 to 4.
