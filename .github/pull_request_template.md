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
- [ ] If this adds a new input, automatic runner or output channel, `docs/security/status.md` is updated (SR-61)

## Handoff

- [ ] `HANDOFF.md` Status and Next/Ready-to-build blocks are updated, and this change is recorded in the right file in `docs/handoff/` (or "nothing to record")

## Checked before pushing

`cargo fmt --all`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`
