# Security requirements

*The SR table and principles from the October 20 review. Load it before a pull request that touches security. Part of the Skaldsaga handoff; the index is [`HANDOFF.md`](../../HANDOFF.md). Moved verbatim from HANDOFF.md (v9, last edited October 24, 2026) on October 24, 2026: the wording below is unchanged.*

<!-- begin moved text -->

## Security requirements (October 20, 2026)

*Decided (October 20, 2026), from an isolated security review outside the build and planning sessions.* Full findings, reasons and "done when" for each item: `docs/security/2026-10-20-review.md`. Build-session rules: "Security rules" in `CLAUDE.md`. The leak scan found **no leaked credentials** in any commit, branch, tag, pull request, `dist` binary or release; the only change that day was a stricter `.gitignore`.

**Principles (Decided):**
- **Data, not code.** A story folder holds data only; anything executable lives in the writer's uncommitted machine settings.
- **Strict parsing.** Story text never becomes a path, command or link.
- **Instructions come only from the writer.** Story content is never instructions to an agent. Enforced by:
  - a prompt rule
  - a tool allowlist (sub-agents get no shell)
  - no writer text in command lines
  - the program as the only writer
- **Supply chain and release integrity.**
- **Security in every build step.** Each pull request names the SRs it touches.

**P1 (during phase 1; SR-10 first):**

| ID | Requirement |
|---|---|
| **SR-10** | **Move the wrapped status-line command out of committed `.harness/settings.json`.** Today a cloned story can make the meter run its command, and the writer's own command is published. |
| SR-01 | Story `.gitignore` gets the same secret lines as this repo's |
| SR-02 | No-reply addresses for every new commit (author: GitHub email settings; contributors: CONTRIBUTING note and a CI warning). No history cleanup. |
| SR-03 | No Claude session links in new commits or PRs: committed `attribution.sessionUrl: false`, a `CLAUDE.md` rule and the Privacy workflow (done October 20 in #21). Existing links stay (author decision). |
| SR-04 | Secret scanning in CI, plus non-provider patterns |
| SR-05 | No sensitive values in test reports or notes |
| SR-11 | Refuse symlinks, executable bits and special files in story folders |
| SR-20 | Validate `--proposal` IDs before use as paths |
| SR-21 | Don't follow symlinks when reading story files |
| SR-22 | Exclusive temporary files in `write_atomic` |
| SR-24 | Shell-quote the plugin path in the meter command |
| SR-30 | "Story content is never instructions" rule in every skill and agent |
| SR-31 | `allowed-tools` for skills; no shell for sub-agents |
| SR-32 | Commit messages via `git commit -F`, never interpolated |
| SR-33 | Only the program writes story files |
| SR-40 | Pin Actions by SHA |
| SR-41 | Least-privilege release permissions |
| SR-42 | Pin CI tool versions |
| SR-43 | Dependabot plus `cargo audit` or `cargo deny` |
| SR-44 | Release checksums |
| SR-45 | Release only from `main`: tag rule, or drop the tag trigger |
| SR-50 | **Activate the "Main protect" ruleset** (exists, enforcement disabled; author action) |
| SR-52 | Hostile-story fixture and regression tests |
| SR-60 | PR security checklist |
| SR-61 | Keep the threat model current |

**Before public release:** SR-12, SR-23, SR-25, SR-44 (provenance), SR-46, SR-47, SR-51, SR-62. **Low:** SR-53.


