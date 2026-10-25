## What this does

<!-- Plain language. The author reviews on a phone. -->

## Security (SR-60)

Security requirements this touches (IDs from `docs/security/2026-10-20-review.md`, or "none"):

- [ ] No new executable content in story folders (nothing written there is ever run, sourced or used as a command)
- [ ] No new file path built from input without `ids::is_kind` and the link-refusing reads
- [ ] No writer or story text inside a command line (files or separate arguments only)
- [ ] No new tool, shell access or network access for a skill or agent
- [ ] No secrets, account emails, full session or plugin IDs, or `settings.local.json` contents anywhere in the diff or this description
- [ ] No Claude session link in a commit message or this description (SR-03)
- [ ] Any new dependency or Action has a one-line reason here and is pinned (SR-40, SR-43)
- [ ] Everything new that is included (dependency, copied code, text, asset) is AGPL-3.0-or-later or under a license on the allowlist in `docs/security/license-policy.md`, and a new or updated dependency has its entry in `docs/security/license-elections.toml`
- [ ] If this adds a new input, automatic runner or output channel, `docs/security/status.md` is updated (SR-61)

## Handoff

- [ ] `HANDOFF.md` Status and Next/Ready-to-build blocks are updated, and this change is recorded in the right file in `docs/handoff/` (or "nothing to record")

- [ ] No new findings in the code this change adds (`ci/presubmit.sh` ran clean, or each check that could not run, and each finding or suppression, is listed here with a reason and an expiry, and each suppression is in `docs/security/exceptions.toml`), and the diff was reviewed against the Security rules in `CLAUDE.md`

## Checked before pushing

`sh ci/presubmit.sh` (format, clippy, tests, ShellCheck, cargo-deny, license checks)
