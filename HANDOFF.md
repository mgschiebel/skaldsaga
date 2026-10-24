# Skaldsaga: Planning Handoff (v9)

*Planning session summary, October 18, 2026 (late night; last updated October 19, 08:50, after the 0.3.0 phone test review and a planning session). Supersedes v8 (October 18, night). Background, research on the original story-theory inspiration, and attribution live in `docs/background.md`, which build sessions should not load. Status: phase 1 design complete; build steps 1 to 8 done (step 5, the spec model and first-run setup, is release 0.2.0; steps 6 and 7, the generated index and the dependency checker, are internal; step 8, the committer and the demo, is release 0.3.0), and the installed plugin is tested from a phone at each testable milestone (see Current stage). Companion file: `ARCHITECTURE.md` (one-page sketch, reviewed in the v6 session). The project was called Story Harness until the v6 session; the GitHub repo has been renamed to Skaldsaga.*

## How to use this document

Drop it in the project root (for example as `HANDOFF.md`) and reference it at the start of a Claude Code session, or paste or upload it into a chat session. Suggested opening prompt for the first build session: "Read HANDOFF.md and ARCHITECTURE.md (not docs/background.md). We're starting phase 1 building with the phone-friendly pipeline. Run the To verify checks under Build, release and delivery, then build step 1: the Rust workspace (skaldsaga-core plus the command-line front end), Linux x64 CI on GitHub Actions, and the session meter pilot. Ask me before any decision the handoff doesn't cover, one short question at a time."

For a planning session instead: "Read HANDOFF.md. Continue from the open items, and grill me one question at a time before we build anything."

Labels used below:

- **Decided**: confirmed by the author.
- **Proposed**: suggested in discussion, not yet explicitly confirmed.
- **Research**: from web searches or documentation; verify before relying on it. Items marked *unverified* have not been checked at all.

**Build steps and GitHub numbers (Decided, October 18, 2026):** the build plan is counted in **steps** (Step 1, Step 2 …), not pull requests. Each step cites its GitHub pull request number and branch, for example "Step 6 (#11, branch `build/index`)", so the number matches what the GitHub app shows. Small fix and doc pull requests don't get a step number and are cited by GitHub number only when they matter. Reason: GitHub counts every pull request, so the two numbers drift apart; a step number can also be named before its pull request exists.

## Current stage: 0.3.0 phone test done; reviewing fixes before step 9 (October 19, 2026)

**The 0.3.0 phone test passed on everything step 8 delivers.** Full report: `docs/tests/2026-10-19-0.3.0-phone-test.md` (two runs on October 19, in the empty repository `mgschiebel/skaldsaga-test`). Summary:

- **Passed:** version 0.3.0 reported (the manual Sync took); `init` in the empty repository (`created`, meter `wired`) and its snapshot; `demo` filed both sample answers; `pr-demstart` passed, committed and saved to git; `pr-demmixup` failed with `learns_before_true` and was held, then deleted; the push to GitHub worked. `spec/_index/` stayed out of git as designed. An empty repository (no commits) caused no trouble for Skaldsaga.
- **Found 1: setup leaves `.claude/settings.json` untracked.** `init` writes the meter's status line there but doesn't return it in `commit_paths`, so the folder isn't clean after setup (the cloud session's stop hook asked about it). The file holds an absolute plugin path tied to one machine or session, so committing it (done in the test, commit `ca500b5`) breaks the meter elsewhere. This affects writers on local Claude Code too, not just testing.
- **Found 2: the session-start hook stays silent when the story folder is one level below the session folder,** which is always the case in cloud sessions (session in `/home/claude`, repository in `/home/claude/<repo>`). Run by hand with the repository as `cwd`, it works. Correction to the report: the hook reads `CLAUDE_PROJECT_DIR` first and falls back to the payload's `cwd`; cloud sessions leave `CLAUDE_PROJECT_DIR` empty. It never looks down a level (skill step 0 does) or up from a subfolder. The skill's summary also wrongly promises the hook "will run next session."
- **Found 3: the meter still has no reading.** Likely cause: Claude Code reads project settings from the session folder, so the repository's `.claude/settings.json` (and its status line) is never loaded in cloud sessions. Meter testing stays paused, as decided earlier.
- **Environment notes:** the clone helper mentions a `register_repo_root` tool that cloud sessions didn't have (October 19: also missing in the build session), so a repository's `CLAUDE.md`, skills and plugins aren't loaded automatically. GitHub now reports the test repository as `Skaldsaga-test`; pushes still work.

**Next session (updated October 19, 08:50, end of the planning session):** the fix questions below are all settled and built in #17. In order:

*Update, October 19, 13:05 (planning session):* #16 and #17 are merged and **release 0.3.1 is published** (started 10:15 Mountain). Step 9's remaining design questions are settled (see "Step 9 planning"), so **step 9 is ready to build**. The author's story-map idea is parked as open item 20, the next planning topic.

*Update, October 19, 14:10 (build session): 0.3.1 tested, fixes in 0.3.2.* Reports: `docs/tests/2026-10-19-0.3.1-phone-test.md` (mobile app: everything 0.3.1 ships worked) and `docs/tests/2026-10-19-0.3.1-web-retest.md` (claude.ai/code on the web). **New finding: the two surfaces start sessions in different places.** The mobile app starts in `/home/claude` with the repository one level down; claude.ai/code on the web starts *inside* the attached repository (`/home/user/<repo>`). The hook handles both. Hook stderr is not shown to Claude at session start, so the debug switch was invisible. Fix pull request (branch `fix/phone-test-0.3.1`, version 0.3.2):

- **Old shared meter entry moved:** `init` and `meter wire` now take a meter status line that 0.3.0 left in the tracked `.claude/settings.json` out of that file (removing the file if nothing else is in it; the writer's own status line and other settings are never touched) and list it under `meter.cleaned`. The skill stages that file only if git already tracks it, and still makes a snapshot when setup was otherwise already done. This was the "migrate or at least not double-wire" note under decision 1, missed in #17.
- **`locate` ignores loose files** when looking for a lone repository to suggest; another visible folder still blocks the guess (the skill always asks before using it).
- **Commit messages are shortened at a word boundary.**
- **Debug reasons reach Claude:** with `SKALDSAGA_HOOK_DEBUG=1`, the hook also puts its reason into the context (appended to the found line, or as its own line when it stays silent). The silent-case line never contains "Skaldsaga project", the phrase the skill looks for; the "several" reason now says "several story folders".
- **`ci/cloud-setup.sh` no longer creates `/home/claude/.harness`** (it made a stray, empty project) or wires a meter there; it only installs the cargo wrapper. **Author: paste the new script into the Project's environment settings** (or clear the old one).
- *Not changed:* a `.harness/` without `settings.json` still counts as a project. Requiring it was suggested in the re-test; with the setup script fixed it isn't needed, and it would change the "once `.harness/` exists, setup proceeds" rule.

**Next phone test (0.3.2, released October 19 at 14:09 Mountain; allow about two hours before syncing):** after Sync, and with the new setup script pasted in, run the re-test's prepared orphan branches in `mgschiebel/skaldsaga-test` on the web (`test/empty-orphan`: expect only a debug line; `test/orphan-one-level-down`: expect the one-level-down line plus its debug reason), and on `main` run `/skaldsaga` once to confirm the old `.claude/settings.json` is removed and committed. Then step 9.

1. **Author:** sync the marketplace, then a fresh phone session checks the hook line one folder down and the debug switch (set `SKALDSAGA_HOOK_DEBUG=1` in the Project's environment settings). *(Merges and the 0.3.1 release are done.)*
2. **Build step 9 (`complete`)** in a fresh context, from "Step 9 planning" below. Nothing left to confirm first.
3. **Planning: open item 20** (story map in the session-start recap). First question already posed: map plus one recommendation (Claude's lean), map then open choice, or map only on request or at milestones.
4. **Triage the remaining older Proposed bullets** (Architecture, Governance, Spec and storage, Story model, Weaver, Future phases): mark those settled by later decisions as superseded with a pointer, and list the few that are genuinely open. Audience principles and the completeness bar were settled on October 19.
5. **Security (October 20 review):** author actions SR-50 (activate the "Main protect" ruleset), SR-02 (GitHub email privacy) and the other GitHub settings in `docs/security/github-settings-checklist.md`. Then schedule the P1 security requirements into the build plan, SR-10 first (see "Security requirements").

1. *Decided (October 19, 2026):* **the meter's status line goes in `.claude/settings.local.json`,** with the absolute plugin path the hook keeps current. Reasons: a status line is a shell command Claude Code runs automatically, so a committed one would run for anyone who clones the story (a supply-chain path), and the absolute path leaks the username and machine layout; a portable run-time lookup was set aside because it runs whatever matches a pattern. No true relative path exists (the plugin lives outside the story folder). Each machine wires its own meter, which the hook already does. **`init` also writes a `.gitignore`** (appending missing lines to an existing one, never replacing it) and returns it in `commit_paths`:

    ```gitignore
    # Skaldsaga: personal or secret files, never shared
    .claude/settings.local.json
    .claude/*.local.*
    CLAUDE.local.md
    .env
    .env.*
    *.pem
    *.key
    # System clutter
    .DS_Store
    Thumbs.db
    ```

    Reason: `settings.local.json` can hold secrets (an `env` block with API keys, `apiKeyHelper`, permissions), and whether Claude Code ignores it automatically when another tool creates it is unverified. The real guard stays the skill committing only `commit_paths`; the `.gitignore` is the backstop. The Skaldsaga source repo gets the same personal and secret lines. Build notes: the hook's re-pointing and the status-line precedence logic in `statusline.rs` move to the local file; a writer's existing personal status line is still wrapped, not replaced; projects already wired in `.claude/settings.json` (the test repo) should be migrated or at least not double-wired.
1a. *Decided (October 19, 2026; confirmed by the author at the end of the session):* **keep the meter design as built.** The writer never needs to read the status line or know what a context window is; it runs out of sight as the sensor (Claude Code pipes usage into it, and only it writes `.harness/meter.json`). The interviewer runs `meter check` after each saved answer (cheap: the binary runs in milliseconds) and raises it in plain words, without "AI" or "context window" (for example: "We've covered a lot. I start losing track of early details in long conversations, so this is a good spot to save and pick up fresh"), saying "start a new session" where `/clear` doesn't exist. `unknown` falls back to the manual boundary. Note: no live reading has been seen yet; both phone runs said `unknown`, and the earlier Linux check used a sample input. Writers use local Claude Code, where the status line is documented, so the gap is likely cloud-only. **Gate before any rethink:** one meter test in local Claude Code on a real machine. Backups considered and set aside unless that fails (or the desktop app turns out not to run status lines): reading token usage from the session transcript (`transcript_path`), or suggesting a fresh session after a number of saved answers. *Live check in the October 19 build session:* a scratch meter was wired at the session folder (`/home/claude/.harness` plus `statusLine` in `/home/claude/.claude/settings.local.json`) to see whether a cloud session runs the status line at all; **Result (stopped at 08:05 by the author, plan already set):** no reading in about 40 minutes of checks (07:24 to 08:00), including after the session went idle and **resumed**, with the status line already wired at the session folder before the resume. More evidence that cloud sessions don't run status lines; the local-machine test stays the gate. Side finding: on that resume the hook line did appear ("This folder is a Skaldsaga project … reason: resume"), the first time in a real cloud session, because `.harness/` was in the session folder. The scratch wiring was removed afterward.
2. *Decided (October 19, 2026):* **the hook looks one level down, narrowly.** If the session folder has no `.harness/`, the hook checks its direct child folders and acts only when exactly one has `.harness/` (zero or several: silent, so it never guesses between stories). When the story is found one level down, the hook adds the context line only, naming where it is (for example "Skaldsaga project found in `skaldsaga-test/`"), and writes nothing: no meter reset and no status-line re-pointing, since project settings don't load from a child folder anyway. No upward search (local sessions set `CLAUDE_PROJECT_DIR`, which covers a subfolder). The lookup lives once in `skaldsaga-core` and serves both the hook and skill step 0. Reason: writers on local Claude Code are unaffected, and phone tests get a visible sign that the hook ran.
3. *Decided (October 19, 2026):* **a debug switch for the hook.** With `SKALDSAGA_HOOK_DEBUG=1` set, the hook prints one plain line to stderr saying why it stayed silent or what it found (for example "no `.harness/` here or one level down" or "two story folders one level down, not guessing"). No files are written, and writers never see it unless they turn it on. For phone tests, set it in the testing Project's environment settings. Tested in the next session, after the fix release (a hook change needs a fresh session anyway).
4. *Decided (October 19, 2026):* **the skill's summary wording** becomes: "(It only runs when a session starts. Right after first-time setup it's normal for it to be missing; it runs in the next session that starts in the story folder or the folder just above it.)" The skill looks for any context line containing "Skaldsaga project", since the one-level-down line reads differently.

**Built (October 19, 2026):** decisions 1 to 4 are in fix pull request #17 (branch `fix/phone-test-0.3.0`, version 0.3.1): the meter wires into `.claude/settings.local.json`; `init` writes or appends the `.gitignore` and returns it in `commit_paths`; a new `skaldsaga locate` command holds the shared folder lookup (the session folder, exactly one child project, or a lone git repository as a setup candidate), used by the hook and skill step 0; the hook adds a context-only line for a story one level down; `SKALDSAGA_HOOK_DEBUG=1` explains a silent hook; `ci/cloud-setup.sh` now writes the personal settings file. After merging: release, Sync, then a fresh phone session to check the hook line and the debug switch.

**Scope note (author, October 19, 2026):** the hook and cloud-session items above are low risk and low effort; don't over-engineer them for the cloud test bench. Build them as decided, then leave this area until the author picks it up in local Claude Code on the desktop (which is also where the meter gate test happens).

**Read this if you are a session started from the "skaldsaga design project" Claude Code Project.** Sessions opened there may be **test sessions**: the author types `/skaldsaga` or `/skaldsaga demo` to check that the installed pieces work. Do not start building unless asked. Earlier test notes follow.

What a test session checks, and what to expect:

- **Program:** `/skaldsaga` reports the version (0.1.1, 0.2.0 and 0.3.0 worked).
- **Session-start hook:** the context should contain a line starting "This folder is a Skaldsaga project". If it's missing, check whether `/home/claude/.harness` existed *before* the session started (`stat -c %w`): the hook is guarded and stays silent without it.
- **Meter:** `meter check` reports a state. `unknown` with "no meter reading yet" means the status line hasn't run; whether it runs in cloud sessions at all is still unconfirmed.

**Finding from the first test session (October 18, evening):** every cloud session starts on a fresh machine, so nothing from an earlier session survives, including `.harness/` and the meter wiring. The Project instructions said to run `mkdir -p /home/claude/.harness`, but Claude runs those only after the session has started, so the hook had already run and stayed silent. Running the hook by hand afterward worked. **Fix:** `ci/cloud-setup.sh`, pasted into the Project's environment setup script, which runs before the session starts. It creates `.harness/`, wires the meter's status line (finding the plugin at run time; the hook then re-points it at the exact path), and installs the cargo wrapper from `CLAUDE.md`. Next test: start a fresh session with that script in place and confirm the hook line appears and whether the meter gets a reading.

**Releasing for phone tests (Decided, October 18, 2026):** whenever a PR changes something testable from the phone (a new command, a skill change, a hook change), it bumps the version, and once the author merges it the build session starts the Release workflow (steps in `CLAUDE.md`). PRs with nothing visible to try skip the release. A dev channel that tracks `main` without version bumps was considered and set aside unless releasing becomes a chore (Claude Code may only notice plugin updates when the version changes; unverified).

**Finding (October 18, 2026, release 0.2.0): the Project doesn't pick up a new release right away.** After the release, new sessions still reported 0.1.1 even though the marketplace was set to sync automatically. Opening the Project's plugin settings, choosing *Manage Marketplaces* and clicking *Sync* brought in 0.2.0. **For now, every phone-test release ends with a manual Sync** before starting the test session. *Open check:* how often automatic sync runs (possibly daily); if it's ever confirmed to be quick, the manual step can go. *Update (October 19, 2026, author's timing):* a new release takes **about two hours** after it's published on GitHub before the marketplace can deliver it, so plan phone tests for a later session rather than right after a release.

**Phone test of 0.2.0 (done October 18, late night; superseded by the 0.3.0 test below):** setup refused, as predicted below, because the Skaldsaga repo was cloned into the home folder; the version, the meter (`unknown`) and the refusal all behaved as designed. Original plan: in a fresh session, type `/skaldsaga`. Expect it to set up the folder (four folders: `spec`, `manuscript`, `reviews`, `style-guide`, plus `.harness/settings.json`), save a first git snapshot of only those paths, and report the meter. Typing `/skaldsaga` again should say everything is already in place. If the session has no `.harness` (setup script not run) and the repo was cloned into the home folder, setup correctly refuses because the folder holds other files; that is the existing-project path, not built yet.

**Second test session (October 18, late night): meter testing paused.** The setup script's effects were all missing (no `.harness/`, no status line in `.claude/settings.json`, no cargo wrapper), so either it wasn't saved in the Project's environment settings or setup scripts didn't run for that session; the hook stayed silent as designed. Checked by hand in a scratch project with `.harness/`, everything worked on Linux: the hook printed its context line, the setup script's status-line command found the installed plugin (under `/root/.claude/plugins/synced/…/skaldsaga`) and wrote a reading, the hook re-pointed the status line at the exact plugin path, and a new session ID cleared the stale reading. **Decided:** a passing Linux check is enough for now. Whether the status line runs in phone or cloud sessions stays unconfirmed, and phone testing resumes later in the build, once there are more commands to test. Test sessions should not repeat the meter check until then. (Note: `/skaldsaga:meter` does not exist; the plugin ships only the `/skaldsaga` skill.)

**Next phone test (0.3.0, released October 18; Decided October 18, 2026):** test in a **new, empty GitHub repository** added to the testing Project, so test files never mix with the Skaldsaga repo. Create it with nothing ticked (no README, .gitignore or license): any visible file makes setup treat the folder as an existing project. After the release, sync the marketplace manually, then start a fresh session and type `/skaldsaga demo`. Cloud sessions start in the home folder with the repository cloned one level down (`CLAUDE_PROJECT_DIR` is empty), so expect the skill to ask whether to use the repository's folder; say yes. Expect: setup (if not done) and its snapshot, two sample answers filed, the first checked, committed and saved to git, the second refused for having Mara know about Jonah's arrest before it happens and then deleted, and an offer to push the snapshot to GitHub. Note: the session-start hook looks at the session's working folder, so it stays silent for a story folder one level down.

### Step 9 planning (`complete`), October 19, 2026

- *Decided:* **`complete` is its own command, run by the skill right after each commit** (not built into `commit`), and on its own when the writer asks how close the story is. It reports the whole story: where it stands against the completeness bar (unanswered shape decisions, empty grid cells, the plot-hole checks) and the script soft findings for the queue. `commit` stays focused on checking and saving one answer.
- *Decided:* **writer decisions are stored one file per decision, in a folder per kind:** `.harness/decisions/<kind>/<id>.json`, committed to git (like `.harness/committed/`). Reasons: adding one never rewrites another, so diffs show exactly one decision and branches or sessions don't conflict; old ones can be deleted cleanly with git as the archive. A single flat file or one file per kind was set aside (whole-file rewrites and conflicts; size itself wasn't the issue).
- *Decided:* **one shared envelope for every kind:** `format` (for later migrations), `kind`, `id`, the writer's own words, what it rests on (node IDs with their revisions), status, and date. Kinds: finding acknowledgments now; ruled-out ideas and later edition and review choices. "Keep it simple" opt-outs stay in the spec (they are story meaning), and "keep anyway" stays with the committed proposal.
- *Decided:* **script findings are recomputed every run; model findings (housekeeper, rubric) are stored,** since they can't be recomputed for free. Only acknowledgments are stored for script findings.
- *Decided:* **stale checks are never silent.** An acknowledgment lapses when any node it rests on changes revision (the version check `commit` already uses), and the finding resurfaces as a follow-up ("Earlier you said this was fine. The story has changed since; still fine?"). A script finding that no longer appears marks its acknowledgment resolved, and the file is deleted. A stored model finding whose nodes changed becomes "needs recheck" and is never raised until the housekeeper's next pass confirms or drops it. The writer can withdraw any decision.
- *Decided:* **future-proofing:** check codes carry a version, so a changed check doesn't match old acknowledgments; edition scope is part of a finding's key (reader checks run per edition later); only the writer creates decisions (a sub-agent proposal that writes one is a hard failure); findings rest on as few nodes as possible, and resurfaced ones go through the normal flag budget so small edits don't cause a flood. Exact fields are step 9 build work.
- *Decided (October 19, 2026):* **step 9 builds all seven script soft checks** listed in "Checks: who catches what," with scene-count pacing using the default four-part framework until framework choice exists. Reason: they are cheap, deterministic and share one finding format, so splitting them would mean a second pass over the same code; pacing simply stays quiet until a skeleton exists.
- *Decided (October 19, 2026):* **two reveals of the same fact stay a hard failure** (`two_reveals`), so each fact has exactly one reveal and step 9's unearned-twist and dangling-setup checks count from it. No writer override and no new "strong" verdict level: hard keeps meaning hard. **Hard findings are raised as guidance with the fix offered in one step, never as a pass-or-fail verdict** (for example: "This shows the reader the same thing twice. Want the earlier moment to be a hint instead?"). Reason: every deliberate case found is really a hint followed by a reveal, and the author's concern was tone (a test the writer fails), which the offered fix answers. The interviewer's wording for every hard check follows this pattern.
- *Decided (October 19, 2026):* **dangling setup stays quiet until the story has a Resolution:** at least one scene covering the main plot's Resolution stage. Before that, an unpaid hint is an open promise, not a finding. Reason: `complete` runs after every saved answer, and early on almost every hint is unpaid, so the queue that must be cleared before weaving would fill with noise. Unearned twist needs no wait: a reveal with no earlier hint is a problem as soon as it's written.
- *Decided (October 19, 2026):* **each script soft check starts counting only once the story can support it,** so a half-built story doesn't produce findings that only mean "not done yet." "The skeleton exists" means **all three movement landmarks (MC commits, midpoint, low point) are placed in order**, the signal the index already uses, so no new stored state.

  | Check | Starts counting when |
  |---|---|
  | Unearned twist | Always |
  | Similar attribute keys | Always |
  | Dangling setup | A main-plot Resolution scene exists |
  | Early pivot | The climax landmark is placed |
  | Subplot resolving after the main climax | The main climax landmark is placed |
  | Scene-count pacing | The skeleton exists |
  | Threads out of alignment | The skeleton exists |
  | Missing landmarks | Split in two; see the next bullet |

- *Decided (October 19, 2026):* **landmarks split into two tiers.** **The three movement landmarks plus the climax block the outline gate** (`complete` reports them as blockers: deepening can't start until all four are placed). They can be satisfied **provisionally** ("not sure yet, probably this scene" passes), the same softener the dramatic and thematic questions use, and they're raised as an inferred suggestion with the fix offered ("It sounds like Mara's arrest is the point of no return. Shall we mark it?"), never as a failed test. **The other five landmarks** (normal world, inciting incident, rising stakes, things fall apart, new normal) stay soft suggestions, raised **when the outliner deepens their movement**. Reasons: the four aren't a craft opinion (every story with four filled stages has points where each movement ends and a scene where the question is answered; the tag only marks them, and the craft teaching stays soft on top); the outliner deepens one movement at a time and pacing, alignment, early pivot and subplot timing all measure from these points, so running on the scene-stage fallback would quietly weaken those checks. **This is a deliberate exception** to "landmarks are never completeness blockers" (Coverage grid and movements), which still holds for the other five. Not a hard check on saving: the weaver builds the skeleton one proposal at a time, so blocking saves would fail every early step.
- *Decided (October 19, 2026):* **thresholds are build work,** as in step 7: how far before the climax counts as an early pivot, how far off a thread must be to count as out of alignment, and how similar two attribute keys must be. The build session picks starting values, makes each a setting, and lists them in the pull request for the author to adjust. **With this, step 9 is ready to build.**
- *Decided:* **`complete` reports to the agents only, as codes and IDs, never prose.** Each gate (internally `weave`, `outline`, `complete`) gets its readiness and a list of blockers such as `{code: "thematic_question_open", arc: "a-…"}` or `{code: "open_findings", count: 2}`, plus the soft findings. All writer-facing wording lives in the orchestrator, never in the Rust code. The orchestrator never names a gate or uses internal words like "weaving": it says what comes next and why in plain words with the story's own names ("Before we put Mara's threads together into one storyline, there's one question left: what is your story really asking?"), then teaches the craft term by the terminology ladder (Story-shape layer). Open item 19 covers tracking what the writer has learned.

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

## Build progress

**Step 1 (#1, branch `build/meter-pilot`, October 18, 2026):** Rust workspace (`skaldsaga-core` library plus the `skaldsaga` command-line front end), the session meter pilot (`meter statusline`, `check`, `reset`, `wire`), the launcher `scripts/skaldsaga`, Linux x64 CI, and a tag-triggered release workflow that publishes a GitHub Release and rebuilds the `dist` branch. Interface notes in `docs/meter.md`. `init`, `check`, `commit`, `index` and `complete` are reserved subcommands that say "not built yet."

**Step 2 (#2, branch `build/serde`):** `serde_json` replaces the stand-in JSON reader; `CLAUDE.md` holds build-session notes (cargo wrapper for cloud sessions, pipeline rules).

**Step 3 (#3, branch `build/plugin`):** (fixes followed in #4 `build/release-button`, #5 `build/fix-dist` and #6 `build/root-marketplace`) a minimal installable plugin in `plugin/`: manifest, marketplace file, a test-build `skaldsaga` skill, and a guarded SessionStart hook (`skaldsaga hook session-start`: silent outside a project; in one, clears a stale meter reading on a fresh session, keeps the status line pointing at the current plugin folder, and adds a line of context). CI assembles, validates (`claude plugin validate --strict`) and smoke-tests the same folder a release ships. Install from the `dist` branch: `/plugin marketplace add mgschiebel/skaldsaga#dist`, then `/plugin install skaldsaga@skaldsaga`.

**Step 4 (#7, branch `test/cloud-setup`):** `ci/cloud-setup.sh` for the testing Project, and the "Current stage" section above.

**Step 5 (#9, branch `build/spec-init`, release 0.2.0):** the typed spec model and `skaldsaga init`.

- **IDs** (`ids`): random, type-prefixed, 8 characters from a 32-character alphabet without look-alikes. **Decided:** group `g-`, item `i-`, and the viewpoint and style node `st-` (the two-letter prefix follows `pr-` for proposals). Other prefixes as already used: `p-` person, `l-` place, `s-` scene, `d-` decision, `t-` thread, `a-` arc, `f-` fact, `e-` edition, `pr-` proposal.
- **Model** (`model`, `node_file`): every node type and the fact entry as typed Rust structures, read and written as Markdown with YAML front matter (via `serde_norway`, the maintained fork of `serde_yaml`). Unknown keys are rejected, so a hand-edit typo is reported instead of silently dropped on rewrite. Written files use a fixed key order and leave out empty optional fields. Threads must have the four stages in order.
- **Loader** (`spec`): one folder per node type under `spec/` (`people`, `places`, `groups`, `items`, `scenes`, `decisions`, `threads`, `style`, `arcs`), file named after the node ID; `sequence.md` lists scene IDs as `- s-…` lines under a short explanation. Loading collects every problem (wrong folder, file name not matching the ID, duplicate node or fact IDs across files, stray files, bad sequence lines) instead of stopping at the first. Fixtures in `crates/skaldsaga-core/tests/fixtures/specs/`.
- **`init`** (fresh-project path): creates `spec/`, `manuscript/`, `reviews/` and `style-guide/` with one-line READMEs, `spec/sequence.md`, and `.harness/` with `inbox/`, `sessions/`, the full default settings (with `format: 1` for later migrations), and a `.gitignore` for `meter.json`. Optionally wires the meter (`--meter-command`). Safe to run again: it only adds what's missing, never replaces a file, and fills in missing settings without touching existing values. **Decided:** the folder is named `style-guide` (no space, safe in shell commands). **Decided:** a folder counts as fresh when it has no visible entries (hidden files such as `.git`, `.claude` and system clutter don't count); otherwise setup reports what it found and touches nothing, until the existing-project path is built. Once `.harness/` exists, setup always proceeds, since the writer has already opted in.
- **Decided: the skill runs git, not the program,** so problems (git missing, no name configured) are explained in plain words. The skill commits only the paths `init` returns in `commit_paths`, never anything else in the folder.
- **Prerequisites** added to the README: Claude Code, a Claude account that includes it, and git. A GitHub account is not needed to install.

**Step 6 (#11, branch `build/index`, no release: nothing visible to try from the phone yet):** `skaldsaga index` rebuilds `spec/_index/` (interface notes in `docs/index.md`). All Decided in the step 6 session:

- **The story's own attribute keys live in `spec/attributes.md`**, Markdown with YAML front matter listing each key's relation (`property`, `symmetric`, `inverse`, `one_way`, `related`), inverse name for pairs, cardinality (one value, or a map by node type such as `located_in`: single for a person, many for a place) and a plain summary. Reason: custom keys are story vocabulary, so they belong in canon next to the facts that use them and go back in history with them; settings are tunable behavior and changing them shouldn't change what the story means. The starter vocabulary is built into the program. Only proposals change the file, once the committer exists; a missing file means no custom keys. Bad entries are reported as loading problems and skipped.
- **`_index/` stays out of git.** The folder writes its own `.gitignore`, so history holds only what the writer decided and diffs stay clean; it's rebuilt on demand.
- **Backlinks sit only under whatever is referenced.** A fact naming a scene (reader moments, who learns it where, `true_from`/`ends` in a scene) is listed under the scene; the fact already lists its own scenes, so a fact-side entry would only repeat it. Generated views can't drift (they're rebuilt whole from the single stored reference), but one direction keeps the rule simple. The Backlinks index example below is corrected to match.
- **Landmark keys** (chosen by Claude, recorded in `model::LANDMARKS`): `normal_world`, `inciting_incident`, `mc_commits`, `rising_stakes`, `midpoint`, `things_fall_apart`, `low_point`, `climax`, `new_normal`. Movement boundaries use `mc_commits`, `midpoint` and `low_point`.
- **Build details (Claude's choices, open to change):** the index is built from whatever loaded, with loading problems and references to missing IDs listed in `status.json`; output is deterministic (no timestamps); the main story's movements span the whole reading order and a subplot's span runs from its first scene to its last; until all three boundary landmarks are placed in order, scenes fall back on their `coverage` stage; shelved subplots appear in the grid but aren't counted as gaps; `index` refuses to run in a folder without `.harness/`. A new fixture story (`tests/fixtures/specs/harbor`) exercises every view.

**Step 7 (#13, branch `build/check`, no release: nothing visible to try from the phone yet):** the dependency checker, `skaldsaga check` (interface notes in `docs/check.md`), and the operation engine it runs on. Decided in the step 7 session:

- **Step 7 builds the operation engine too.** `check` applies a proposal's operations to an in-memory copy of the spec and checks the result; step 8's `commit` reuses the same engine and writes the result, so check and commit can't disagree. Step 7 shipped as one pull request (splitting it was considered and set aside).
- **Lanes:** the interviewer starts optimistic; the harvester, researcher, weaver and outliner start strict. A proposal escalates to strict when 10 or more other nodes read what it changes, or when it touches a confirmed shape decision or that decision's pivot belief. The threshold is a new setting, `routing.strict_fan_out` (default 10).
- **Build details (Claude's choices, open to change):**
  - Verdicts are `passed`, `flagged` (soft findings only; the writer decides) and `failed` (hard findings; revise only), written back into the proposal file with the revisions checked (`sequence.md` and `attributes.md`, which have no revision, are recorded by a content fingerprint).
  - Only findings a proposal introduces count; problems already in the spec don't block it.
  - Ops beyond the handoff's list: `add_attribute` (records a story key in `spec/attributes.md`). The reading order is edited with `add_item` and `remove_item` on the target `sequence`, with optional `after` or `before`. `move_fact` is `{fact, to}`.
  - `supersede` moves references from the old fact to its replacement and flags the scenes that read it; `reclassify` flags them too.
  - In-world order comes only from anchors: `after` is strictly later; `in` and `at` are the same moment. Reading order never implies in-world order.
  - "The reader cannot learn a fact before it is revealed" is read as: at most one reveal per fact in the reading order (alternative scenes may also reveal, since an edition can swap one in), and a hint placed after the reveal is a soft finding. *Confirmed October 19, 2026: stays hard* (see Step 9 planning).
  - Uniqueness looks at both sides of symmetric relations and normalizes inverse pairs to their registered direction. Overlaps the timeline can't decide are soft; two values that never end are hard. Beliefs and unresolved facts are skipped.
  - The harvest drift check on delete (comparing drafts) is left for the harvester, since it needs a model.

**Step 8 (#14, branch `build/commit`, release 0.3.0):** the single committer, `skaldsaga commit`, and `skaldsaga demo` (interface notes in `docs/commit.md`), plus the skill's `/skaldsaga demo` flow. Decided in the step 8 session:

- **Committed proposals move to `.harness/committed/`** with how they were decided (lane, the writer's decision, whether a "not sure yet" assumption is open), so recaps can quote the writer and open assumptions stay findable.
- **Phone testing uses an empty GitHub repository** plus the demo, so test files never mix with real ones (see Current stage).
- **Build details (Claude's choices, open to change):**
  - Commit always re-checks first. A failed verdict never commits; a flagged one needs `--decision keep_anyway` or `not_sure`; the strict lane needs `--cleared` and a fresh clearance (nothing in `revisions.checked` changed: the version check). A held proposal keeps the fresh verdict in the inbox.
  - Labels on what a proposal writes: `unreviewed` when optimistic, `cleared` when strict and cleared; "not sure yet" turns confirmed into provisional.
  - The program writes files and the skill makes the git commit (as with setup, step 5), using `add_paths` and `remove_paths`; removals go through `git rm --cached --ignore-unmatch`, since the inbox file may never have been in git. The commit message is "Skaldsaga: " plus the writer's answer.
  - Writes are not atomic across files; git is the safety net if a commit is interrupted.
  - The skill now finds the story folder when it is one level down (a repository cloned into the home folder), asking first and passing `--project`.

**Step 9 (#22, branch `build/complete`, version 0.3.3, release after the author's go-ahead):** `skaldsaga complete`, `decisions acknowledge` and `decisions withdraw` (interface notes in `docs/complete.md`), built from "Step 9 planning" as written. Build details (Claude's choices, open to change):

- **Starting thresholds, all settings:** `pacing.under_share` 0.5 and `pacing.over_share` 2.0 (each movement expects a quarter of the scenes until framework choice exists), `checks.early_pivot_share` 0.25, `checks.misaligned_share` 0.5, `checks.similar_key_max_distance` 2.
- **Soft checks start counting only when the story supports them:** dangling setup waits for a main-plot Resolution scene; early pivot and subplot-after-climax wait for the climax; pacing and alignment wait for the arc's three movement landmarks in order. A dormant check's acknowledgments are never cleaned up.
- **Acknowledgment identity** is code, check version, scope (`canon`), IDs and detail. Count-dependent findings put the movement or direction in `detail`, so a changed situation is a new finding.
- **Cleanup** of resolved acknowledgments is skipped (`cleanup_skipped`) when the spec has loading problems. Unknown codes are left alone.
- **Challenger and relationship threads** whose decisions are all `not_explored` are exempt from the coverage grid.
- **Similar attribute keys** compares keys with underscores removed, skips pairs of starter keys and inverse pairs, and needs at least 4 characters.
- **The skill's demo** gets a last step that runs `complete`; on the demo story every gate is blocked by `no_main_story`, which is the expected answer.
- **Not built:** the orchestrator wording for findings, and the skill flow that asks the writer about open findings (both belong with the agents).

**SR-10 fix (branch `build/wrapped-command`, version 0.3.4):** the writer's own status line command moved out of committed `.harness/settings.json` into `.harness/local.json`, which `.harness/.gitignore` keeps out of git. The meter reads it only from there (symlink or over 64 KiB: ignored) and never runs a command from a committed file; `init` and `meter wire` migrate an old entry and list `.harness/settings.json` and `.harness/.gitignore` under `meter.cleaned`. Choices: a gitignored file inside `.harness/` rather than a key in Claude Code's `settings.local.json` (unknown keys there may trigger Claude Code warnings); a story that force-adds `local.json` to git could still carry a command, which SR-12 (pre-release) should address; history keeps commands committed earlier.

**SR-11 (branch `build/story-folder-scan`, version 0.3.5):** links, executable files and special files in the story folder are reported and never followed (interface notes in `docs/safety.md`). `fsio::list_dir` now says what each entry is without following links; `safety::scan` walks `spec/`, `manuscript/` and `.harness/` (depth 8, 20,000 entries) and `init`, `check`, `index` and `complete` print `unsafe_files`; the spec loader, `check` and `commit` refuse to read links and special files and cap story file reads at 1 MiB. Choices: the executable bit is reported, not blocking (drives that mark every file executable would otherwise break a story); the scan never gates `complete`; the skill tells the writer in plain words and changes nothing itself. Still open from SR-11: an existing-project import (not built) must apply the same rule, and later readers (manuscript, style guide) must use the capped, link-refusing reads.

*Housekeeping (October 24):* the merged branches `build/complete` and `build/wrapped-command` still exist on GitHub; cloud sessions can't delete branches (403), so the author deletes them from a computer, or turns on "Automatically delete head branches".

**SR-20, SR-22, SR-24 (branch `build/path-safety`, version 0.3.6):** `commit --proposal` checks the ID with `ids::is_kind` before it becomes a path (exit 64; the committer also answers `not_found`), so `../` can't reach a file outside the inbox; `fsio::write_atomic` creates its temporary file exclusively and tries the next name if one is taken, so a planted link is never written through; the meter command quotes the plugin path with single quotes, tested with a hostile folder name. An old double-quoted command is replaced the next time the hook refreshes it. An earlier demo-era test used an invalid proposal ID and now uses a valid one.

**SR-40, SR-41, SR-42, SR-45 (branch `build/ci-pinning`, no version bump: nothing visible to try):** every third-party Action in `ci.yml` and `release.yml` is pinned to a commit SHA with its version in a comment (`privacy.yml` already was); the Rust toolchain is pinned to 1.97.0 in both workflows and `claude-code` to 2.1.310 in the plugin validation step. `release.yml` now has `permissions: {}` at the top and two jobs: `build` (read-only; tests, builds, assembles, uploads the tarball) and `publish` (the only job with `contents: write`; unpacks the tarball, creates the release and pushes `dist`). The `v*` tag trigger is gone, so only running the workflow on `main` releases (SR-45). A `dry_run` input lets the workflow be tested from any branch: it runs both jobs but publishes nothing. *Deviation from the review (SR-42):* no `rust-toolchain.toml`, because rustup would try to download that exact toolchain in cloud sessions, where `static.rust-lang.org` isn't allowlisted; the pin lives in the workflows instead. Dependabot will keep the SHAs current once SR-43 lands.

**Decided (October 18, 2026): phones and cloud sessions are the author's build and test setup only.** Writers install a released plugin into their own local Claude Code; they are not expected to use Skaldsaga from cloud sessions. This supersedes the v9 note that phone writers reach Skaldsaga through cloud sessions, and drops the idea of copying the skills into each story folder. For testing from a phone, the author uses a Claude Code Project (public beta, Pro and Max), which loads plugins added in *Project settings > Plugins* into every cloud session and has an environment setup script.

**Findings from the To verify checks (Research, October 18, 2026; verify again before relying):**

- **Executable permission:** plugin installs from GitHub marketplaces used to drop the executable bit; a fix in Claude Code v2.1.86 preserves it on `.sh` files, and it is unclear whether other files (our binary) are covered. Handled either way: hooks and the status line call the launcher through `sh`, and the launcher restores the binary's bit before running it.
- **Cloud sessions do not install plugins declared in a repo's `.claude/settings.json`;** Claude Code Projects do load plugins added in their settings. Settled by the decision above: cloud use is the author's testing only.
- **`/clear` is not available in cloud sessions** ("start a new session from the sidebar instead"). The session-boundary wording must say "start a new session" there; the SessionStart hook's `startup` matcher still covers it.
- **Status line in cloud sessions:** not documented either way (the docs describe a terminal status bar). Likely unavailable; the meter already degrades to `unknown` and the manual boundary. Test from a phone session once a project is wired.
- **crates.io from cloud sessions:** blocked at first; fixed by allowlisting `index.crates.io`, `static.crates.io` and `crates.io` in the environment settings, plus a small cargo wrapper each session (see `CLAUDE.md`). Step 2 replaced the stand-in JSON reader with `serde_json`.

## What changed since v8

A grilling session on the coverage-grid stages and the weaver and outliner (open items 4 and 5). All of the following are **Decided**; details are in "Coverage grid and movements" and "Weaver, outliner and frameworks."

- **Fixed four-stage grid for every thread** (closes open item 4): **Setup, Complications, Crisis, Resolution.** The grid stays framework-agnostic; frameworks layer named beats on top, so switching frameworks never invalidates answers or changes what "no empty cells" means.
- **Stages align across an arc's threads by default**, giving the book four shared movements. Misalignment is a soft "possible pacing issue," explained once and respected.
- **Subplots float independently** of the main arc but keep four aligned stages internally. A subplot's minimum is its own dramatic question, stakes, and at least one thread with all four stages; other threads accept "keep it simple."
- **Two tiers of beats, all soft:** a small set of universal **landmarks** owned by the grid (inciting incident, midpoint, low point, climax and others), and **framework beats** supplied by the chosen framework. Inferred and confirmed, never asked cold; missing ones become suggestions, never completeness blockers.
- **Movement boundaries are derived from landmark scenes**, never stored, so `sequence.md` stays pure.
- **The outline is layered:** a skeleton pass across the whole book, then deepening one movement at a time, starting with Setup. Layers set the work order, not a depth limit: detail the writer raises early is captured at whatever depth it arrives and becomes a pin the skeleton builds around.
- **Framework choice:** inferred any time and confirmed when obvious; if the writer is unsure, it waits for the weaver's late recommendation. Four-part is the default, switching is free, and "no framework" is valid.
- **The skeleton is sized by story need, not word counts.**
- **Removed from phase 1: the suggested first edition and the length primitive** (supersedes the v7 decision). Phase 1 has no prose, so word counts would be guesses; word budgets belong to editions and move to a later phase, where the edition step teaches length during revision. Phase 1 keeps only relative pacing notes in scene counts.
- **Fog subplots at weave time:** each is mapped, shelved, or folded into an existing thread, chosen by the writer.
- **Promote to subplot** stays in phase 1: the reverse of fold-in, for any thread content, writer-started, using existing operations.
- **Stage readings written for all four threads** (main plot and challenger added).
- **Landmarks and framework beats are stored on the arc**, one entry per landmark, not as scene tags.
- **The weaver produces the skeleton directly**; no separate woven plan. The outliner only deepens.
- **Scenes gain a "turn"** (what the POV character wants, what gets in the way, how it ends differently), which catches flat scenes and feeds cause-and-effect checks.
- **Scene-count pacing heuristic** set (expected share from the framework; soft note when a movement is under half or over double it). Open item 5 is closed.
- **Skaldsaga stands on its own.** The shape decisions and completeness criteria are our own, in our own words. Attribution and background moved to `docs/background.md`.
- **The theme layer collapses to the thematic question.** "Where the conflict lives" and "what everyone cares about" are dropped; the root problem moves into the optional "what's really wrong" story point.
- **The 12 shape decisions are final, worded in plain language** using the story's own names, with the craft term taught afterward (see the table in Story-shape layer). Open item 3 is closed.
- **Optional story points need no new node types:** stakes sit with the dramatic question; strength and flaw are person facts; "what's really wrong" is a belief fact plus the truth it contradicts; pressure along the way is facts with a `role` tag; backstory is facts true before the first scene. Open item 17's node fields are settled.
- **Soft checks split between script and model.** Countable checks run free and instant in the `complete` script; the housekeeper keeps only checks that need judgment.
- **Flags are budgeted:** the housekeeper reports everything, the interviewer raises at most one or two per natural pause by impact, and the rest wait in a queue that must be cleared before weaving and outlining.
- **The completeness rubric makes four judgments:** cells meet their stage, decisions have support, the ending answers both questions, and the threads interlock (the interlock check is now Decided). Open item 6 is closed at design level.
- **Starting values for all settings chosen**, and an existing personal status line is wrapped rather than replaced (see Default settings). Open item 10 is closed apart from two unverified research points.
- **Starter relation vocabulary set, and two small node types added: `group` and `item`** (see Relationship vocabulary under Node format). Open item 12 is closed.
- **Evals plan set:** ordinary Rust tests with fixture spec folders for everything deterministic, and seeded-defect test stories with clean controls for the housekeeper and completeness rubric, which also decide whether a cheaper model can be the default (see Testing and evals). Open item 7 is closed at plan level.
- **Rust track settled** (closes open items 13 and 18; see Build, release and delivery): one binary with subcommands on a `skaldsaga-core` library; binaries kept out of `main`; the session meter is the pilot; binaries reach the writer through a CI-built distribution branch; launch targets, cargo-dist, shared versioning, and staged signing chosen.
- **Phones:** no native iOS or Android binaries in phase 1. Writers on phones use cloud Claude Code sessions, which the Linux binary covers (verify plugin, hook and status line support there). `skaldsaga-core` stays portable so a native mobile app can come with the standalone app later.
- **Research items 9 and 11 done:** the wayfinder SKILL.md has been read, with several lessons proposed for confirmation; licensing for the original story-theory inspiration was checked, and our design is on the safe side (see `docs/background.md` and Open items 9 and 11).
- **Internal "storyform" renamed to "arc"** (node type `arc`, ID prefix `a-`), and all outside story-theory terms removed from the build documents so they cannot leak into prompts, schemas or code. History, research and attribution moved to `docs/background.md`; the README keeps a plain acknowledgment.
- **Lessons from wayfinder confirmed** (see Open items 9): keep questions short with the why in one sentence and more on request; a ruled-out ideas list no agent re-suggests; agents cannot loosen their own rules (settings, gates and opt-outs change only through the writer); the writer chooses among variants; the housekeeper suggests revisiting a confirmed decision when evidence against it builds; a question becomes a frontier question once it can be stated precisely.
- **New sub-agent: researcher.** Looks up real-world facts a story depends on, only when the writer asks or accepts an offer, and files them as provisional proposals with sources. Mid-tier model. The writer can always bend history for the story.
- **Phase 1 is not a public release, and builds only for Linux x64** (the author's development environment and the cloud sessions used from a phone). Signing, macOS and Windows move to the public-release phase; the launch-target and signing decisions stay as that phase's plan.
- **Phase 1 uses the phone-friendly build pipeline:** code written and tested in cloud sessions, GitHub Actions for Linux CI, pull requests reviewed in the GitHub mobile app (see Build, release and delivery). Later phases move to the author's desktop Claude Code environment, reached remotely (likely Claude Code's remote-control feature; *verify the exact name and setup*). `skaldsaga-core` stays portable, but macOS and Windows runners wait until the CI/CD pipeline is built on the desktop.

## What changed in v8

A short side session revisited the script language, which had been Node since v6. Details are in Architecture, Future phases, Research notes and open items 13 and 18.

- **Decided: scripts move from Node to Rust.** The Rust source lives in the repo, and what the writer installs carries compiled binaries for each supported operating system. Reasons:
  - **Nothing for the writer to install.** A fresh Claude Code install guarantees neither Node nor Python (see Research notes). A compiled binary just runs, which matters for writers who have never used a terminal.
  - **Fast startup.** The status line script runs constantly, and a compiled binary starts in milliseconds.
  - **Types for the spec schema.** Facts, the two clocks, confidence and review labels, and scene reads and writes become typed structures, so the checker, committer and index rebuild are harder to get subtly wrong. The compiler also catches mistakes that coding agents would otherwise ship.
  - **A path to the long-term product:** eventually a single Rust binary with everything inside, including a friendly interface that uses Claude through the API. The local web viewer that motivated Node can be served from Rust too.
- **Proposed: one binary with subcommands** (`init`, `check`, `commit`, `index`, `meter`, `complete`) instead of one executable per script. Build it as a core library (working name `skaldsaga-core`) plus a thin command-line front end, so the future standalone app wraps the same core instead of rewriting it.
- **Proposed: compiled binaries stay out of the main git history.** CI builds every target and publishes them (GitHub Releases, and possibly a built distribution branch that the plugin installs from). How the right binary reaches the writer is open item 13.
- **Folder name caution.** The idea discussed was a `bin` directory, but a top-level `bin/` folder blocks plugin installation through claude.ai and Cowork (see Research notes), so the binaries need another home, for example `scripts/bin/` or inside a skill folder.
- **Proposed: the session meter is the pilot.** It is small but exercises the whole delivery chain: status line wiring, cross-platform builds, first-run delivery, and signing.
- **Known costs, accepted for now:** a build and release pipeline for macOS (Apple Silicon and Intel), Linux and Windows; unsigned binaries trigger macOS Gatekeeper and Windows SmartScreen warnings, which would scare novice writers, so signing likely needs an Apple developer account and Windows signing work; a higher bar for open-source contributors than scripts; and slower iteration while the schema is still changing.
- **Future front end note.** The Claude Agent SDK is aimed at TypeScript and Python (verify), so a Rust app would either run its own agent loop over the Claude API or drive Claude Code as a subprocess. Calling the API directly also raises the question of API keys and per-token billing versus the writer's Claude subscription. Both are recorded under Future phases.

## What changed in v7

This session worked through editions (open item 14) from the writer's day-to-day point of view. All of the following are **Decided**; details are in "Editions" under Spec and storage.

- **Editions are separate files; `sequence.md` stays pure.** Chapter, act and update breaks never go into the sequence. Reason: the sequence is canon, so breaks inside it would stale reading-order checks whenever a break moved, and CD would need to write to a canon file.
- **One true canon.** Every edition uses the same facts. An edition may leave scenes out or use alternative scenes, but it can never change what is true.
- **Reader checks run per edition.** Facts, timing and character knowledge are checked once, in canon. What the reader sees (hints, reveals, setups and payoffs, cause and effect between visible scenes) is checked per edition, because each edition has its own scene list.
- **Alternative scenes are ordinary scene nodes** that are not in the main sequence. They don't have to cover the same ground as the scene they replace; the edition just has to pass every check.
- **A retelling that changes canon is a new project,** forked by copying the facts with a note of where they came from (project and version), never synced back.
- **Edition lifecycle:** not published → publishing in progress (serials and anything released in parts) → published. Published editions are frozen snapshots; canon may move on and republishing means a new edition. While in progress, a canon change that touches released groupings marks them "needs republishing" and Skaldsaga strongly recommends bringing them up to current canon.
- **Review snapshots** (beta readers, editors, publishers) belong to not-published editions. Each is pinned to a canon version so feedback can be shown against the exact text the reviewer read. They never trigger republishing.
- *Superseded in v9 (removed from phase 1):* **Phase 1 ends with a suggested first edition** (closes the main question in open item 14): the outliner's last step proposes a not-published edition chosen from the writer's goal, clearly labeled as a suggestion, using a minimal edition file (breaks anchored by scene ID).
- **IDs are random instead of counting up** (see Spec and storage), for collision safety if facts later move between projects, and so shuffled scenes never look out of order.
- **Story-shape naming started (open item 3):** a terminology ladder (craft term, then abbreviation, then names); "main character"/MC; "challenger"; MC and POV character taught as different ideas; threads named main plot, inner journey, challenger, relationship.
- **Dramatic and thematic questions** are tracked on every arc, inferred rather than asked cold, and block weaving and outlining until answered at least provisionally.
- **"Keep it simple" is a valid answer** for the thematic question, three of the MC's four decisions, story driver and limit, the challenger and relationship decisions, and the challenger thread itself. Always required: the dramatic question, the stakes, whether the MC changes or holds firm, outcome and judgment.
- **Story-theory research done** (now in `docs/background.md`): the theme approach (our own words) and the full list of candidate story points were reviewed for novice value.
- **Stakes join the required core**, which is now: the dramatic question, the stakes, whether the MC changes or holds firm, and the ending type (outcome and judgment). Strength and flaw, pressure along the way (costs, small wins, warning signs, required steps), want versus need, and backstory are adopted as optional depth (open item 17).
- **Book series noted for a later phase** (see Future phases): books are story-level units, not editions; the series shape is undecided.

## What changed in v6

The v6 session reviewed `ARCHITECTURE.md` (open item 1, now closed) and settled open item 2. All of the following are **Decided** unless marked; details are in the sections named.

- **Name: Skaldsaga** (see "Name"). It is the project, plugin and command name (`/skaldsaga`). Internal role names such as "orchestrator" stay internal and are never shown to the writer.
- **Starting Skaldsaga** (see Architecture): the writer types `/skaldsaga` the first time, which runs setup. Afterwards a SessionStart hook shipped in the plugin opens each session with the recap (also after `/clear`). The hook is guarded: it does nothing unless `.harness/` exists, so it never starts questions in other folders or in an existing project before setup.
- **Existing projects** (see Architecture): setup looks before touching anything and asks the writer how to bring files in, recommending: move originals untouched into an archive folder, copy prose drafts into `manuscript/`. Non-prose notes stay in the archive and are harvested into the spec from there. Folders that clash with Skaldsaga's names go into the archive too. If the folder already uses git, setup asks whether to commit into the writer's repo (recommended, with a labeled "before Skaldsaga" commit) or keep a separate history.
- **Setup pattern:** where writers reasonably differ, setup asks in plain language, recommends a default, and stores the answer in `.harness/settings.json`.
- **Hand-back pattern** (closes open item 2): a sub-agent that needs the writer returns plain-language questions; the interviewer asks them and saves proposals; the orchestrator re-dispatches the sub-agent.
- **Incremental session-open pass:** the housekeeper pass at session start covers only changes since the last cleared pass. Full passes run before weave and outline and on request. *Proposed:* a full pass every N sessions.
- **Models per sub-agent are tunable settings.** The housekeeper (it does judgment work; the scripts are the deterministic part), the completeness rubric, weaver and outliner default to the strong model; the harvester to mid-tier. A cheaper housekeeper default must first pass the evals.
- **Scripts are written for Node** *(superseded in v8: Rust)*, for room to grow (for example a local web viewer for the manuscript). How they reach the writer (require Node, or ship single executables) is deferred to a future phase, alongside research into embedding the agent harness in a Node app with a web front end.
- **No chapters in the story model.** The story is one ordered sequence of scenes, stored in `spec/sequence.md`. Chapters, acts and serial updates belong to **editions**, groupings laid over the sequence, and a project can hold several. Flashbacks need nothing special (the two clocks already cover them).
- **CI/CD as the long-term shape** (see Future phases): CI makes the story and produces a pinned version; CD runs one pipeline per edition with format-specific checks, can block and send proposals back, and never edits canon.
- **Research recorded** (see Research notes): a fresh Claude Code install guarantees no Node or Python; plugins cannot register the main status line, so setup writes it into the project's `.claude/settings.json`; a skill's `model` field lasts one turn, so the main-thread skills run on the writer's model.

## The project

A set of Claude Code skills (a "harness") that helps a novice aspiring author turn an incomplete story idea into a well-crafted, detailed novel outline with no plot holes. The skills ask questions, build the spec in the background, and teach craft along the way. The goal is a good story, not a theory-correct one. Where the ideas came from is recorded in `docs/background.md`. The harness will be distributed for use by others.

**Decided**

- Runs in Claude Code. Building begins in future sessions.
- Targets novel-length fiction in this phase, but the story model stays medium-agnostic (one scene sequence, with chapters, acts or serial updates as editions), so plays, serials and other forms can follow.
- This phase delivers only a complete, detailed outline (spec, plot structure, and the scene sequence with beats). Editions, word budgets and the length primitive are not part of phase 1 (v9).
- Modeled on Matt Pocock's wayfinder skill concept: planning across many sessions where each decision reveals what to plan next.
- **Long-term goal (later phases, out of scope now):** a fully automated novel-writing factory, with either assisted writing or automated scene writing for human review, producing publishable scenes and documents. Later phases include writer and review agent pairs and editor agents (pacing, flow, grammar, voice, and so on). The spec layout must be future-proofed for this (see Spec and storage, and Node format).

## Name

**Decided**

- **The project is called Skaldsaga.** It is the open-source project name, the plugin name, and the command the writer types (`/skaldsaga`). The GitHub repo has been renamed from Story-harness to Skaldsaga.
- **Internal role names stay internal.** "Orchestrator," "interviewer," "housekeeper" and the rest remain the names used in these docs and in code, but the writer never sees them. The orchestrator is the `skaldsaga` skill.
- **Why this name:**
  - A *skald* is a Norse poet-storyteller, which gives us the mentor persona (an expert advisor who guides the writer). Like a muse, it also covers later phases (pair and automated writing) without a rename.
  - *Saga* is Old Norse for "story" or "what is told." It keeps the name about story rather than any medium (novels, serials, comics), and everyone can spell it.
  - In modern Icelandic, *skáldsaga* (poet + story) is the everyday word for "novel."
  - Short, one word, no hyphens or apostrophes, so the command matches the name. "Harness" was dropped because it is a technical word.
- **Names rejected and why:** any "muse" name (an existing story-coaching app already has an AI coach called Muse in the same niche; Plotmuse is an existing company); bare "Skald" (existing AI writing apps Skald Writer and Skald Scribe); Gaelic pairings such as *scéal* (hard to spell, and a cross-language blend).

**Research (web search only, October 18, 2026; GitHub search was not available)**

- No product found using "Skaldsaga." The term appears only in scholarship, where "skaldsagas" are the medieval Icelandic sagas about poets. Recheck GitHub, package registries and trademarks before publishing.
- Known risk: "skald" sounds like "scald," so some people will misspell the command. Consider whether the plugin can also answer to a misspelling.

## Audience and interaction principles

**Decided**

- Users have never written a story and know neither story theory nor writing jargon. Every prompt explains in plain English what is being asked and why it matters to the story.
- Socratic style: open-ended questions that guide the writer and explain the impact on the story. Questions are teachable moments that lean into the why.
- Ambiguous answers: record the possibilities as unresolved and let later answers collapse them. Once a question becomes frontier (the next thing worth resolving), ask a plain-language disambiguating follow-up.
- Stuck users: if the question is not frontier, move on, track it, and re-ask when a later answer helps. If it is frontier, rephrase concretely first, then offer contrasting "what if" options.
- Provisional answers matter more than provenance. Placeholders supplied by the user are tracked as provisional too.
- Writers may draft scenes before structure exists. This phase harvests story facts from drafts into the spec. Writing feedback on drafts is parked for a later phase.
- Role: expert advisor that guides the writer and teaches craft.
- **Infer-and-confirm** is the default for the character and plot shape decisions: ask about concrete scenes and people, infer the decision, confirm it back ("it sounds like your hero ends up changing, is that right?"). Inferred answers stay provisional until confirmed. A direct plain-language question is the fallback when the inference is ambiguous and the decision is frontier.
- **No fixed question order.** The orchestrator and the writer's momentum drive the flow. When momentum slows, the orchestrator offers another area to look at.
- **Raise issues while the topic is live.** Flags and conflicts are raised as close as possible to the answer that caused them. Topical questions are easier for the writer to remember, and deferring them to wrap-up forces a context switch that can confuse. This principle drives the live handling of confirmed-decision changes and the rolling housekeeper passes (see Governance).

- **Ask about instances before principles, for everything** (*Decided October 19, 2026*, was Proposed): scenes, people and places first, then infer structural choices and confirm them back. The general form of infer-and-confirm, and matches the craft-teaching lessons (open item 19).

- **Option sparks** (*Decided October 19, 2026*, was Proposed): when the writer is stuck and the interviewer offers "what if" options, they are built from the writer's own earlier answers, not generic tropes. The program shuffles them (open item 19).
- **Brainstorming as an offering** (*Decided October 19, 2026*): the writer can ask to just throw ideas around; by default the interviewer keeps its infer-and-confirm style. It is also **offered** in a lull or when the writer hits a snag, alongside the backlog of open questions ("Want to brainstorm for a bit?"); the writer chooses.
  - **Nothing becomes canon by brainstorming.** Ideas wait in a holding area until the writer picks one, then go through the normal check and commit. Unpicked ideas are not rejections: they go on the ruled-out list only if the writer explicitly rejects them.
  - **What-if plot impact uses the real engine:** `check` already applies a proposal to a copy of the story without saving, so brainstorming can show what a twist would ripple into ("if Jonah is the informant, these three scenes no longer fit") before anything changes.
  - **Scope:** plot, scenes, characters and what-ifs in phase 1 (outline level). Voice, POV voice and dialogue are prose-level, for a later phase with the style guide.
- **Taste anchors** (*Decided October 19, 2026*, was Proposed): ask early, lightly and once, about the genre and feel the writer wants for this story and their favorite stories; never steer toward copying a favorite. The genre for this story is story meaning (spec); favorite stories are writer state. **The writer's words come first:** when they name a story, ask what they love about it. **Don't assume we know it:** when the interviewer will refer to specifics, the researcher looks up public sources (summaries, reviews, author interviews), and genre conventions are researched the same way. For copyrighted works, sources about the story, never its text; full text only for public-domain works. Pairs with the light starting check (open item 19).

- **At most one ripple question per answer, and no deadlock** (*Decided October 19, 2026*, was Proposed): when an answer affects other parts of the story, the interviewer asks at most one follow-up right away; anything more joins the flag queue. If the writer is still stuck after the what-if options and the brainstorming offer, the interviewer records a flagged provisional answer and moves on, so a session never stalls on one question. Fits the flag budget and the provisional-answer rule.

- **Hide internal terms** (*Decided October 19, 2026*, was Proposed): the writer never sees any internal word from our docs or code (for example map, tickets, fog, proposal, gate, weaving, or the agent role names); they see plain phrases such as "what we've decided" and "things we'll get to later." The only exception is craft terms, which are taught through the terminology ladder.

- **When momentum slows, offer a choice** (*Decided October 19, 2026*, was Proposed): the backlog of open questions is ranked by unblocking value (confirming which provisional answer frees the most downstream work, using the fan-out count `check` already computes), and the orchestrator offers a choice, never an order: the most useful next thing, brainstorming, or something the writer picks.

## Story-shape layer

The coverage grid checks that every thread has content at every stage. It does not check what the story is arguing. The story-shape layer is the set of decisions that define the story's argument and shape, twelve decisions in all.

**Decided**

- Every shape decision must be answered at least provisionally before the outline step ("complete" requires it).
- Use our own plain-English names throughout, since the checker is our own.
- **Terminology ladder (decided in v7):** introduce the common craft term a writer might meet in books or online, explained plainly (teaching the craft), but avoid terms that are too technical (for example "protagonist"). Once taught, shorten to the common abbreviation, then switch to the story's own names as soon as they exist. Applied to the central character: "main character" with an explanation, then "MC," then the character's name ("Does Mara change by the end?").
- **The challenger keeps that name (decided in v7)** for the character who presses on the MC's way of seeing things. A teaching aside mentions the related terms a writer may meet: foil (a character who contrasts with the MC; ours actively pushes back) and antagonist (whoever opposes the MC's goal, who may or may not be the challenger).
- **Main character and POV character are taught as different ideas (decided in v7).** The POV character is whose eyes a scene is seen through (one or many); the MC is whose inner struggle the story argues (usually one per arc). Other POV characters' arcs may become subplots. Ladder: "point of view character," then "POV character," then names. Timing follows infer-and-confirm: each described scene notes whose eyes it is through, and a second viewpoint triggers the teaching moment and a check on which one is the MC.
- **Thread names (decided in v7):** the **main plot** (the external problem and stakes; "external plot" taught as the related term), the MC's **inner journey** ("internal arc" taught as the related term), the **challenger**, and the **relationship**. Main plot and inner journey give the writer the classic outer-versus-inner split used in most craft advice. Internal kind values in the thread format may stay as they are.
- **Two big questions, tracked on every arc (decided in v7):**
  - **The dramatic question:** will the MC achieve the goal? (A common craft term.) Answered by the ending's outcome.
  - **The thematic question:** what the story examines, for example "Is loyalty worth lying for?" Answered by how the ending feels (better or worse off, and at what cost). It is the writer's own words for the theme layer's central tension.
  - **The threads examine the thematic question from different sides:** the MC lives one answer, the challenger embodies another, the relationship is where they collide, and the main plot shows what each costs at scale. The coherence checker tests that every thread bears on the thematic question and that the ending answers both questions.
  - **Asked late, inferred first.** Ask directly only if the writer raises it; otherwise infer once a few scenes and choices exist and confirm ("Your story keeps circling whether Mara should trust anyone. Is that the big question?"). Never ask "what's your theme?" cold.
  - **Blocker:** both questions must be answered (at least provisionally, like other shape decisions) before weaving or outlining can start. The orchestrator steers toward them as that stage approaches. For the thematic question, "keep it simple" is a valid answer (see below).
- **"Keep it simple" is a valid answer (decided in v7).** A writer may say they are writing a simple, fun story and do not want to deal with the deeper layers. The goal is a good story, not a theory-correct one.
  - **What can be opted out of:** the thematic question, three of the MC's four decisions (start or stop, approach, problem-solving style) and two of the four plot decisions (story driver, story limit), all still inferred and offered when scenes make them obvious, the challenger and relationship decisions, and the challenger thread itself (some simple stories have none; without a challenger there is no relationship thread either). **Always required:** the dramatic question (even a simple story needs to know what the MC wants and whether they get it) the stakes (what happens if the MC fails), whether the MC changes or holds firm (the core of the inner journey), and the ending type: outcome (is the problem solved) and judgment (is the MC better or worse off), which together give the four classic endings.
  - **How it works:** the decision gets an explicit value (for example `not_explored`) recorded as the writer's choice, so it counts as answered and does not block weaving or completeness. Coverage-grid cells for an opted-out thread are not counted as empty.
  - **Explain once, then respect it:** Skaldsaga says briefly what the writer would gain, then stops raising it. Checks that depend on an opted-out decision are skipped, not failed. The writer can revisit it any time; if later scenes strongly suggest a theme anyway, Skaldsaga may mention it once, as an offer.
- **Challenger and relationship decisions (draft, inferred and confirmed, never asked cold):** does the challenger change or hold firm by the end (usually the opposite of the MC); does the bond between the MC and the challenger deepen, break, or stay unresolved. Both accept "keep it simple." Final wording is decisions 11 and 12 in the table below (v9).
- **Theme layer (collapsed in v9):** the thematic question is the theme layer's only decision, answered in the writer's own words, not from a fixed menu, with follow-ups guiding toward a non-conflicting answer. Dropped: "where the conflict lives" and "what everyone cares about" (their useful content appears in the main plot's setup and stakes). The root problem moved into the optional "what's really wrong versus what the MC thinks" story point.
- **Skaldsaga's criteria stand on their own (v9).** The decisions below and the completeness bar are our own and need no outside story theory (background in `docs/background.md`).
- **Challenger and relationship threads:** add our own shape decisions, one or two per thread (for example: does the challenger change or hold firm; does the bond deepen, break, or stay unresolved), answered in the writer's own words with guiding follow-ups.
- **No hard combination rules.** The checker raises soft "possible issue" warnings when story content seems to clash with a decision (for example, "your hero holds firm, but scene 12 shows them changing their core belief; which did you mean?").
- **Evidence and pivot (new):** each decision records the scenes and facts its inference came from, and change-type decisions can name a pivot belief (see Node format). Character change is tied to a character learning the truth.

**The 12 shape decisions (Decided in v9; wording final)**

Questions use the story's own names as soon as they exist (shown here with Mara as the main character and Jonah as the challenger). Craft terms are taught only after the plain question. Most decisions are inferred and confirmed rather than asked ("It sounds like Mara is a different person by the end. Is that right?"); the plain question is the fallback when an inference is ambiguous and the decision is frontier.

| # | Plain question | Taught afterward as | Required? |
|---|---|---|---|
| 1 | By the end, has Mara become a different person, or is she still the same at heart? | The character arc; changes or holds firm | Required |
| 2 | Is Mara's trouble something she keeps doing that she needs to quit, or something she won't do that she needs to start? | Want versus need | Optional |
| 3 | When things go wrong, does Mara try to fix the situation, or does she try to fix herself? | Active versus reflective character | Optional |
| 4 | Does Mara figure things out one step at a time, or does she go with her gut? | Problem-solving style | Optional |
| 5 | What is Mara trying to do, and what's the worst that happens if she fails? | The dramatic question and the stakes | Required |
| 6 | In your story, do things happen to Mara that force her to choose, or do her choices make things happen? | Plot-driven versus character-driven | Optional |
| 7 | Does the story end because time runs out, or because there's nowhere left to turn? | The ticking clock | Optional |
| 8 | Does Mara get what she was after? | The outcome | Required |
| 9 | Is Mara happier or better off when it's over? | The ending's judgment; with 8, the four classic endings (happy, bittersweet, tragic, hollow win) | Required |
| 10 | Underneath it all, what is your story wondering about? | The theme (the thematic question) | Optional |
| 11 | Does Jonah end up seeing things Mara's way, or does he stick to his own? | The foil | Optional |
| 12 | By the end, are Mara and Jonah closer, broken apart, or still unsettled? | The relationship arc | Optional |

- **Required** matches the core: changes or holds firm, dramatic question with stakes, outcome, judgment. **Optional** accepts "keep it simple" and is still inferred and offered when scenes make it obvious.
- 11 and 12 disappear when the story has no challenger.
- How these map to the original inspiration is recorded in `docs/background.md` for maintainers; those names are never shown to writers or used in code.

## Completeness bar

**Decided**

- A "complete" outline requires: all shape decisions answered at least provisionally (including the dramatic and thematic questions, which also gate the start of weaving), no empty cells in the coverage grid, and the five plot-hole checks run clean.
- A rough outline is allowed early, with gaps marked (so story theory is never forced on the writer).
- A complete outline may contain provisional answers. Each provisional answer gates the tasks that depend on it. Anything unresolved blocks "complete."

- **A complete outline lists its provisional items as "assumptions to confirm,"** each with what would settle it (*Decided October 19, 2026*, was Proposed). `complete` reports them as codes and IDs like its other blockers ("not sure yet" answers already carry an open assumption in `.harness/committed/`), and the orchestrator words each plainly ("We're still assuming Jonah never learns the truth; one scene showing his reaction would settle it").

## Architecture

**Decided**

- Hub and workers, not a linear pipeline. An orchestrator reads story state, recaps the last session in plain language, and guides the human, including suggestions for where to pick up.
- A living story skeleton that changes and grows as questions are answered. A rough outline is allowed early, with gaps marked.
- A housekeeper task that rechecks old answers for conflicts, forgotten details, idea drift, and plot changes. Full passes run as a gate before the weave and outline steps, and whenever the writer asks. Scoped passes run in the background during a topic (see Governance).
- **Session-open pass is incremental (decided this session).** The pass at the start of each session checks only what changed since the last cleared pass (found through the review labels), not the whole spec. The housekeeper already runs as a sub-agent, so main-thread context was never at risk; the saving is token cost and time, which for a full pass grow with the size of the spec every session. *Proposed:* a full pass every N sessions (a setting) catches slow drift that incremental passes can miss, such as the premise wandering across many small answers.
- A primitive shared by interviewer agents for consistency, worked into the question flow ("that affects X, how do you see that changing?").
- A weaving step that combines arcs into a plot structure. Weaving may reveal a relationship that needs its own arc, creating a feedback loop back to the map.
- The gap assessor is **not** a separate worker. It is a shared **completeness-check primitive** called directly by the orchestrator, housekeeper, and weaver. The orchestrator turns its findings into frontier questions.
- The interviewer is a separate worker from the orchestrator. The orchestrator owns the session frame (recap, choosing the topic, wrap-up). The interviewer runs the question loop for a chosen topic.
- **Confirmed split:**
  - **Main thread:** the orchestrator (index-only reads, recap) and the interviewer skill (the live conversation).
  - **Sub-agents (non-interactive, short summaries back):** housekeeper, harvester for drafts, weaver, outliner, researcher (v9).
  - **Scripts:** deterministic checks (dependency checker, links, status, timelines, setup-and-payoff ledger), the committer, and the index rebuild.
  - **Completeness check:** a primitive with a script half and a rubric half; the rubric half runs as a sub-agent for deep passes.
- **Session boundaries:** each topic ends at a deliberate boundary. The writer clears the chat and the next session opens with the orchestrator's recap. State is saved to disk after every confirmed answer, so nothing lives only in the chat.
- **Session meter:** a small script, not a sub-agent (a sub-agent cannot see the main context). The status line script writes context usage to a project file, and the interviewer checks it after each saved answer. At a first threshold the harness recommends wrapping up at the next natural pause (the writer may decline once). At a second, higher threshold it wraps up regardless. Thresholds are settings. Wrap-up means: finish the current question, save state, write the recap, tell the writer to clear.
- **Models per sub-agent are settings (decided this session).** Each sub-agent's model is a default in `.harness/settings.json` that the writer can tune, so a writer on a budget can turn things down and newer models can be adopted without a code change. Starting defaults:
  - **Scripts:** no model, no tokens.
  - **Housekeeper (all passes) and completeness rubric:** the strong model. The housekeeper is not the deterministic part (the scripts are); it does judgment work such as unearned twists, uncorrected beliefs and premise drift. A cheap pass that misses something still marks facts cleared, and strict-lane agents trust cleared state.
  - **Harvester and researcher:** mid-tier (the researcher does careful lookup, not creative work). It is mostly careful extraction, and adopting an existing project can mean reading a whole manuscript, so this is where tiering clearly saves.
  - **Weaver and outliner:** strong (creative structural work).
  - **Orchestrator and interviewer:** run in the main conversation on whatever model the writer has selected. A skill's `model` field only lasts one turn, so it cannot pin these (see Research notes).
  - **Lowering a default needs evidence.** The seeded-plot-hole evals (open item 7) test whether a cheaper model catches enough in scoped housekeeper passes; only then is that default lowered.
- **Researcher (v9).** Looks up real-world facts the story depends on (how a 1920s blockade worked, what a courier could carry, travel time between two cities). Started only when the writer asks, or when Skaldsaga offers ("Want me to look up how blockades actually worked back then?") and the writer accepts; never automatic. Findings go to the inbox as ordinary proposals, marked provisional, with sources recorded on each fact, and pass the normal checks; canon is never changed silently. If the writer wants history bent for the story, that is their call and the fact records it as their choice. Reason: settings and plausibility are a common novice weak spot, and a wrong real-world detail is its own kind of plot hole. Inspired by wayfinder's research tickets.
- **Hand-back pattern for sub-agents that need the writer.** Sub-agents cannot talk to the writer, but the weaver (framework choice) and the outliner may need answers. Such a sub-agent stops and returns a short list of plain-language questions. The interviewer asks them in the main conversation and saves the answers as normal proposals. The orchestrator then re-dispatches the sub-agent, which finds the answers in the spec. Rejected alternative: running the weaver as a main-thread skill, which would fill the chat's context with weaving work and shorten sessions.
- **Scripts are written in Rust (decided in v8, replacing Node from v6).** Source in the repo; the writer gets compiled binaries for their operating system, so no runtime has to be installed (a fresh Claude Code install includes neither Node nor Python; see Research notes). Rust also gives fast status line startup, typed spec structures, and a path to a single standalone binary in later phases. A local web viewer for the manuscript, the reason Node was picked, can be served from Rust as well. One binary with subcommands (`init`, `check`, `commit`, `index`, `meter`, `complete`), built as a core library plus a thin command-line front end (confirmed in v9). Delivery and the release pipeline are in Build, release and delivery (v9).
- Scripts can ship inside skill folders (a skill is a folder with a markdown file plus scripts). For compiled binaries this means a per-platform executable inside the plugin, never in a top-level `bin/` folder (see Research notes).
- **First-run setup** (all four top-level folders, session meter wiring, default thresholds) is part of the orchestrator's first-run path, not a separate skill. The orchestrator handles it in the background so a non-programmer never has to invoke a setup skill.
- **Starting Skaldsaga:** the writer types `/skaldsaga` the first time in a folder; that runs first-run setup and creates `.harness/`. Two first-run paths:
  - **Fresh project** (empty folder): create the folders and start the conversation.
  - **Existing project** (folder already holds the writer's files): look before touching anything, summarize what is there in plain language, then ask the writer how to bring the files in, with a recommended default. Writers will have their own preferences, so this is always asked, never assumed.
- **Recommended default for an existing project:** move the writer's existing files untouched into an archive folder, copy the drafts into `manuscript/`, then set up as normal (the harvester reads the copies). Other options to offer: leave files where they are and record where the drafts live, or move them straight into `manuscript/`.
- **Git in an existing project is a setup-time choice.** If the folder already has git history, setup asks whether Skaldsaga should commit into the writer's existing repo (recommended) or keep a separate history of its own. The answer is stored in `.harness/settings.json`. With the existing repo, Skaldsaga makes one clearly labeled commit before moving anything ("Before Skaldsaga: your files as they were"), giving the writer an undo point. If the folder has no git at all, setup creates it.
- **General pattern (from the two choices above):** where writers will reasonably differ, setup asks in plain language, recommends a sane default, and stores the answer in `.harness/settings.json` so it can be changed later.
- **Only prose goes to `manuscript/`.** Non-prose material in an existing project (character sheets, old outlines, world-building notes, research) stays in the archive only. The harvester reads it from there and proposes spec updates through the normal approval flow, since that information belongs in the spec. `manuscript/` holds only the writer's story text.
- **Later sessions start on their own (guarded hook):** the plugin ships a SessionStart hook (matchers `startup`, `resume`, `clear`) that does nothing unless `.harness/` exists, so installing the plugin never starts questions in a non-Skaldsaga folder or in an existing project before the writer types `/skaldsaga`. Setup does not write any hook into the project. In a Skaldsaga project the hook adds context telling Claude to open with the orchestrator's recap. Firing on `clear` matches the session boundary (clear, then recap). *Research:* plugins can ship hooks, which run wherever the plugin is enabled; SessionStart can add hidden context before Claude's first reply. *Unverified:* whether the hook's `initialUserMessage` field can start the conversation without the writer typing; if not, the writer types anything and gets the recap.

**Proposed**

- The orchestrator recommends one default next step plus "something else."
- Shared primitives: plain-language translation reference, impact lookup (related items via relationship facts, typed fields, the backlinks index, and rules), spec read/write with status, a harvest primitive for drafted material, and the coherence checker. (The length primitive moved to a later phase in v9.)
- The housekeeper never edits on its own. It raises plain-language heads-ups. Keep the original premise and taste anchors as a versioned baseline, and supersede decisions with dated notes rather than overwriting.
- Even a single arc needs weaving, to interleave its four threads across the book.
- When weaving surfaces a possible new arc, add it to the map as a fog item automatically and raise it with the user only when it is ready to explore.
- The orchestrator should explain the settings change plainly ("I'm turning on a small progress meter so we know when to take a break") and let the Claude Code permission prompt act as the confirmation. If the writer declines the meter, the manual session boundary still works.

## Governance: how changes reach the spec

Two separate concerns, kept distinct:

- **The pending inbox** keeps unchecked data out of canon, so nothing ever reads or builds on a change that a checker later rejects. Quarantine reinforces this: strict-lane agents, including the housekeeper, read only cleared state.
- **The proposal format** decides how precisely the checkers and the committer can do their jobs: what a proposal touches, when its clearance is stale, and how the interviewer overlays pending changes on the spec.

**Decided**

- **Nothing in the spec is updated until all governance checks (dependency checker and housekeeper) have completed.** This avoids a race where the spec is updated and then the housekeeper rejects the change.
- **Pending inbox:** changes go to a pending inbox on disk. The dependency checker runs on each immediately. The interviewer reads the spec plus the pending inbox so it does not contradict uncommitted answers.
- **Single committer (new, was Proposed):** one committer applies proposals one at a time, bumps node and fact revisions, and rebuilds the generated index folder after each commit. The version check, uniqueness rule, merge handling, and index rebuild all depend on this.
- **Proposal format:** one JSON file per confirmed answer, made of field-level operations. Each proposal records which agent made it (which determines its lane), the writer's answer in their own words (for recaps), and the revisions it was checked against. Full schema and op list in Node format.
- **Dependency checker:** a deterministic script run instantly on every proposal, with three jobs:
  1. **Validity:** referenced IDs exist, references resolve, required fields are present, operations are well-formed. Delete is blocked when anything references the target (new).
  2. **Routing:** counts fan-out (how many nodes read what changed) and detects whether a confirmed decision is touched. This sets the lane and triggers live confirmation.
  3. **Hard ordering and uniqueness checks:** a character cannot learn a fact before it becomes true; the reader cannot learn a fact before it is revealed; an event cannot precede its cause in in-world time; no scene reads a fact after it has ended (new); at most one live fact per entity and attribute at any in-world moment (new). It enforces only relations that are actually pinned down. Loose in-world time it cannot resolve goes to the housekeeper as a possible issue.
- **Housekeeper scoped passes:**
  - **Scope:** the touched nodes plus their readers (anything that references or declares a read of them, found through the backlinks index). Above a cap of N readers (a setting), it recommends a full pass at the next session start instead.
  - **Timing:** rolling background passes. After every few saved answers (a setting), a scoped pass runs as a background sub-agent over the answers since the last pass. Only one pass is in flight at a time. The interviewer raises any flags at the next natural pause while the topic is still live. The end-of-topic step covers only the final few answers.
- **Flagged proposals: the writer decides.** A flagged proposal stays held in the inbox. The interviewer or orchestrator explains the flag in plain language, and the writer chooses:
  - **Revise:** the answer is updated and the proposal is re-checked.
  - **Keep anyway:** the proposal is committed and the flag is recorded as acknowledged.
  - **Not sure yet:** the proposal is committed as provisional and the flag stays open as an assumption to confirm.
  - **Hard script failures** (broken references, ordering or uniqueness violations, blocked deletes) are held and explained the same way, but "keep anyway" is not offered, because they cannot be true.
- **Changes to confirmed decisions are handled live.** When an answer would change a confirmed decision, the interviewer's consistency check catches it and confirms in the moment ("You'd confirmed Mara changes by the end; this sounds like she holds firm. Is that a change of mind?"). If the writer says yes, a housekeeper scoped pass starts immediately in the background. The interviewer steers to topics that do not depend on the changed decision until the pass returns, then raises any flags.
- **Removing and changing things (new):**
  - **Never referenced** (no relationship facts, typed-field references, scene reads, anchors, or evidence in the backlinks index): plain **delete**. Git underneath keeps history. Delete also runs the harvest drift check against drafts, because the index cannot see prose.
  - **Referenced:** delete is blocked and the interviewer asks what changed:
    - **In the story** ("Mara stops trusting Jonah"): the fact gets `ends` and `ended_by`; the change becomes a story moment.
    - **The writer's mind** ("she never trusted him"): **supersede with a required replacement**; scenes that read the old fact are flagged.
  - **Cutting something referenced** (for example, Jonah's sister): hard failures on the scenes that read it force revision first; once nothing references it, it can be deleted. No separate retire state exists.
  - **Series shelf:** unused facts are simply left in place for later books. The writer deletes only what they truly do not want.
  - **Merge** (two entities become one): redirect all references to the survivor, then run the attribute uniqueness check on the combined facts.
  - **Split** (one entity becomes two): no automatic redirect; the interviewer asks about each reference ("In scene 12, is this Jonah or his brother?").
  - **Truth becomes belief** (the reader was misled all along): reclassify the fact's kind in place and flag the scenes that relied on it.
  - **Deleting a scene:** facts anchored to it count as references; re-anchor them first.
  - **Moving a fact to a different home node:** keep its ID and its fact revision; bump both nodes' revisions. Never done as delete plus create.
- **Per-agent lanes:** cheap agents such as the interviewer accept backout costs and commit optimistically. Expensive or isolated parallel agents are held to stricter rigor (checks before commit). Any change to a confirmed decision, or with high fan-out, escalates to the strict lane regardless of which agent made it.
- **Quarantine:** optimistic commits are tagged unreviewed. Strict-lane agents read only cleared state, so a housekeeper rejection can only undo cheap work.
- **Two independent labels** on each fact: writer confidence (confirmed, provisional, unresolved) and review state (unreviewed, cleared).
- **Version check (settled for phase 1, may change in later phases; updated in v5):** each node carries a revision for its node-level fields, and each fact carries its own revision. A proposal records the revisions of (1) the nodes and facts it writes, (2) the nodes and facts its author read to form the answer, and (3) the nodes and facts each checker examined when clearing it. At commit, if any of those revisions has changed, the clearance is stale and the proposal is re-checked. Otherwise it commits, even if unrelated parts of the spec changed. In short: a clearance is only as fresh as everything its checks looked at.
- **Task readiness (future phases):** a task is ready only when every dependency is confirmed. In assisted mode the writer can opt in to "draft anyway"; the draft is tagged with its assumptions, and the housekeeper marks it stale if an assumption changes.
- **Scenes declare reads at the fact level; writes are derived (updated).** A scene lists the fact IDs it relies on. What a scene makes true, hints, or reveals is generated from the facts that point at it. Dependencies and parallel-conflict handling derive from this data.

**Proposed**

- The readiness gate means both checkers have run since the last relevant change (not stale) and there are no unacknowledged blocking findings. Findings the writer has acknowledged stop blocking.
- After drafting, the harvest step compares what a scene was expected to establish with what the draft actually established, and flags undeclared writes (including new attributes such as "a chipped front tooth") as possible spec drift.

## Spec and storage

**Decided**

- Plain files in the project folder, as a folder hierarchy where higher-level index files link to node Markdown files, to minimize context bloat. Not a flat file, and no issue tracker.
- **Organized by story element:** people, places, scenes, shape decisions, threads, each holding one small node per entity. The coverage grid and map are generated index views linking to those nodes. Reason: structure changes a lot during planning and entities mostly do not, so the layout should not be keyed to the most volatile thing.
- **Four top-level folders created at first run:** spec, manuscript (prose), reviews (findings per editor type), and style guide (folder `style-guide`, decided in step 5). Empty folders get a one-line README explaining what goes there later. Drafts a writer brings in before any structure exists go in the manuscript folder, where the harvest step reads them.
- **Stable IDs are opaque and type-prefixed:** for example `people/p-0007.md`, `scenes/s-0042.md`, facts `f-0031`. Names live in front matter and index lines, so renaming a character touches one field and breaks no links. IDs never change.
- **IDs are random, not counting up (decided in v7).** Reasons: collision safety if facts later move between projects (series, forks); no implied order, so shuffled scenes never look "out of order"; and agents can create IDs without a shared counter, so a proposal can name a new node before it is committed. The committer rejects duplicates. *Proposed format:* type prefix plus 8 lowercase characters from an alphabet without look-alikes (no `0/o`, `1/l`), for example `s-7kq2xmhd`. The sequential IDs used as examples elsewhere in this document are illustrative only.
- **Generated index folder (new):** `spec/_index/` holds `backlinks.json`, `attributes.json`, `grid.json`, `scenes.json`, and `status.json`. One script rebuilds all of them after every commit. Nothing in it is ever hand-edited; deleting the folder and re-running the script always restores it. Agents load only the file they need. Nodes store only their outgoing references (relationship facts and typed fields); backlinks are generated. *Built in step 6* (shapes in `docs/index.md`); the folder ignores itself in git.
- **The story's own attribute keys (step 6):** `spec/attributes.md`, canon, changed only through proposals (see Build progress, step 6).
- **Facts are ID'd entries inside entity nodes:** each entity node holds a list of its facts (schema in Node format). A fact involving two entities is stored once on one home node; the generated index shows it from the other side. Scene reads, the setup-and-payoff ledger, and the ordering checks reference fact IDs. Each fact has its own revision (updated in v5), so editing one fact does not stale readers of other facts on the same node.
- **Labels per fact, node status derived:** writer confidence and review state sit on each fact (and on each shape decision's value). A node's status in the index is computed, never stored (for example "2 provisional, 1 unreviewed"). A quarantined fact does not quarantine its whole entity, and readiness can gate on exactly the facts a scene reads.
- **The spec is the source of truth. Flow is one-way (spec to draft).** If a draft introduces a new fact, the harvest step proposes a spec update for the writer's approval, never changing canon silently.
- **Two clocks tracked from the start:** in-world time (relative unless a date is given) and reading order (the order of scene IDs in the sequence file, never stored as position numbers). This lets an automated scene-writing agent see only the spec up to that point in the timeline.
- **The scene is the atomic unit.**
- **The story is one sequence of scenes; chapters belong to editions (decided this session).** Chapters are a written-novel idea, and the harness supports other forms (plays, web serials, comics). So the story itself is a single ordered series of scenes from start to end, and every grouping is presentation:
  - **The sequence** is stored in one file, `spec/sequence.md`: the scene IDs in reading order. Moving a scene is a one-line edit. The single committer serializes edits, so the shared file causes no conflicts, and any reorder correctly stales whatever checked reading order (reveals, knowledge). It is also the one file a writer might open to see the story's shape. Rejected: a sortable position key on each scene (no single place shows the whole story).
  - **Editions** are groupings laid over the same sequence: chapters (optionally inside parts) for a novel, acts for a play, updates with release days for a web serial. A project can hold several editions side by side, and publishing uses one.
  - **Flashbacks need no special structure:** a flashback is a scene whose in-world time is earlier than its place in the sequence (the two clocks above).
  - **Word budgets** apply to each edition's groupings (for example, serial updates of about 2,500 words), in the later phase that builds editions (v9). Phase 1 paces by scene counts per movement only.
  - *Proposed:* each edition file lives in `spec/editions/` and lists its breaks by scene ID plus labels. Groupings break only at scene boundaries.

### Editions (decided in v7)

- **Separate files, pure sequence.** Each edition is its own file; `sequence.md` never holds breaks. Breaks inside the sequence would bump its revision and stale every reading-order check whenever a break moved, and would make CD write to a canon file. *Proposed:* a generated, read-only view in `spec/_index/` shows the sequence with one edition's breaks, so the writer still gets a single-file picture.
- **One true canon, shared by every edition.** An edition may leave scenes out or use alternative scenes, but facts are the same in every edition.
- **Alternative scenes** are real scene nodes in the spec that are not in the main sequence. They may read and reveal any canon fact. If one would make something new true, that fact enters canon through the normal approval flow and must not contradict anything. An alternative scene does not have to match what the scene it replaces revealed; the edition just has to be a complete story.
- **What is checked where:**
  - **Once, in canon:** what is true, when it is true, and what characters know.
  - **Per edition:** reader hints and reveals, setups and payoffs, and cause and effect between scenes the reader actually sees. Cutting the scene that plants a setup leaves an unpaid payoff in that edition only.
- **Retellings are forks.** If a new version needs canon to change (a genuinely different story), it becomes a new project that imports a copy of the facts with a note of where they came from (project and version). It never ties back to the original.
- **Lifecycle (three stages):**
  1. **Not published.** Planned or in draft. Breaks and groupings change freely; it always builds from current canon.
  2. **Publishing in progress.** For serials and anything released in parts. Each released grouping is pinned to the canon version it was built from. When a canon change touches something already released, those groupings are marked **needs republishing** and Skaldsaga strongly recommends bringing them up to current canon; a republished grouping is re-pinned to the new version. This keeps a live edition consistent for readers who are following along.
  3. **Published.** Finished and frozen: a historical snapshot. It is never re-checked; canon may move on, and Skaldsaga may note "differs from current canon" as information only. Republishing means starting a new edition.
- **The writer marks an edition published deliberately** (for example when a serial ends). A published edition never reopens. A novel usually goes straight from not published to published in a single release.
- **Review snapshots.** Copies handed out for comment (beta readers, an editor, a publisher, advance copies) belong to a not-published edition. Sharing a copy creates a snapshot pinned to its canon version (for example "Beta round 1, sent to 4 readers"). Feedback is filed in `reviews/` tied to that snapshot, so Skaldsaga can show what the reviewer actually read and translate the comment into a question about the current text. Snapshots never trigger republishing. *Proposed:* a soft, information-only heads-up when a change touches text in a late advance copy. Serial writers can use the same mechanism to beta-read an update before release.
- *Superseded in v9:* ~~Phase 1 proposes a first edition.~~ Editions are not built in phase 1. The rules and minimal file format below stay decided for the phase that first needs editions (likely drafting or CD), where an edition step also runs the word-budget check and teaches length during revision.
- **Reorders and breaks.** Canon cares only about scene order and the ripple effects of a move. Breaks are anchored by scene ID, with no rules about following or staying put: published editions and released groupings are pinned and unaffected (a move touching released groupings falls under "needs republishing"), and a not-published edition is marked as needing a fresh look, after which the edition flow checks it and proposes new breaks if needed.
- **Phase 1 edition file (minimal):**

  ```yaml
  id: e-4hq8nzwt
  name: "Novel, first draft"
  kind: novel            # novel | serial | play ...
  status: not_published  # not_published | in_progress | published
  groupings:
    - {label: "Chapter 1", starts_at: s-7kq2xmhd}
    - {label: "Chapter 2", starts_at: s-p3rwe9ka}
  ```

  Cuts, alternative scenes, per-grouping release status and pinned versions, and review snapshots are added in the phases that need them; the format leaves room for them.
- **Harness machinery (new in v5)** lives in a hidden `.harness/` folder at the project root: `inbox/` (pending proposals), `settings.json`, `meter.json` (written by the status line), and `sessions/` (one plain-language recap per session). First run creates it alongside the four visible folders. Recaps could later be surfaced to the writer as a journal.
- Phase 1 captures a **viewpoint and style node**, asked in plain language and held as provisional (fields in Node format).
- **What-if scene prototypes** are visible variant files inside the project folder, promoted by an explicit step (promotion runs the harvest step). Git stays hidden underneath for history and the agents' parallel workspaces.

**Proposed**

- Index entries with a one-line summary, derived status, and links. A changelog of superseded decisions that is not loaded by default. Nodes about one entity or decision cluster, under about a page. A validation script for broken links and orphaned nodes.
- Scene packets: a compact bundle for a writer agent containing the scene's beats, the relevant people and places, the attribute facts the scene reads, the style rules resolved from its POV, what the reader and POV character know so far, and planting instructions for setups ("establish that the lock is old, don't explain why") rather than the payoff itself.

## Node format (new; all Decided unless marked)

Every node is a Markdown file. Structured data lives in YAML front matter; the body is free prose notes for the writer that no checker relies on.

### Common core (every node)

```yaml
id: p-0007
type: person        # person | place | group | item | scene | decision | thread | style | arc (group, item added in v9)
name: "Mara Vance"
aliases: ["Mara", "the courier"]
summary: "Courier who smuggles letters past the blockade"  # index line
revision: 12        # bumped by the committer when node-level fields change
facts: [...]        # fact entries, schema below
```

- **No node-level links list (v5).** Relationships in the story world are facts (see below). Spec bookkeeping uses typed fields per node type (scene `pov`, `location`, `participants`, `coverage`; thread `characters`; decision `evidence`, `pivot`; arc `threads`, `decisions`).

### Fact entry

```yaml
- id: f-0031
  kind: truth              # truth | belief
  revision: 3              # bumped by the committer when this fact changes
  statement: "The cellar lock is old and brittle"
  attribute: null          # optional property key, e.g. eye_color
  value: null              # optional property value, e.g. "grey-green", or an entity ID for relationships
  holder: null             # belief only: a person ID or reader
  contradicts: null        # belief only: the truth's fact ID
  question: null           # unresolved only: what's undecided, plain words
  options: []              # unresolved only, optional; empty = TBD
  true_from: {after: f-0012, offset: "3 days", precision: approx}
                           # or {in: s-0042} or {date: ...} or unknown
  ends: {at: f-0040}       # optional, same anchor format
  ended_by: f-0040         # optional, the fact that replaced it
  reader:                  # list of moments; at most one reveal
    - {scene: s-0004, level: hinted}
    - {scene: s-0031, level: revealed}
  characters:
    - {who: p-0007, learns_in: s-0012, how: told}  # saw | told | worked_out
  links: [p-0002]          # other entities this fact involves
  confidence: provisional  # confirmed | provisional | unresolved
  review: unreviewed       # unreviewed | cleared
  source: interview        # interview | harvest:<file> | research:<url or citation> | placeholder
  role: null               # v9, optional: cost | small_win | warning | required_step (pressure along the way)
```

Rules:

- **In-world time** is relative by default (anchored to a fact or a scene, with optional offset and precision). An absolute date is used only when the writer gives one.
- **Facts end explicitly.** In-world change is a new fact plus `ends` and `ended_by` on the old one. This is distinct from the writer changing their mind (supersede).
- **Beliefs** use the same time fields; a belief usually ends when its holder learns the truth. The **reader** can be a holder, which covers red herrings.
- **Reader hints and reveals:** a reveal with no earlier hint gets a soft "possible unearned twist"; a hint never revealed or used gets a soft "dangling setup."
- **Unresolved facts** keep one ID so scenes can still declare a read and readiness blocks on them. When settled, a set operation picks the winner and losing options go to history.
- **Properties are facts with an `attribute` key.** The attribute registry (`attributes.json`) lists keys in use per node type with a **cardinality**: single (one live value per entity at any in-world moment, e.g. `eye_color`), many (a list, e.g. `sibling_of`), or per-target (one live value per other entity, e.g. `bond`). The uniqueness check (hard) follows the cardinality. A new key that looks like an existing one is soft-flagged. Attribute writes are declared like any other write.
- **Relationships are facts (v5):** `attribute` names the relation and `value` is the other entity's ID (`attribute: sibling_of, value: p-0002`). Stored once on a home node (the node being discussed when the answer came up). The registry marks each relation symmetric (`sibling_of`, `spouse_of`), an inverse pair (`parent_of`/`child_of`, `located_in`/`contains`; only one direction stored), or one-way (`bond`; each direction is its own fact). The `related` fallback is a fact whose `statement` describes the connection. Knowledge of a relationship is tracked on the fact's `characters` and `reader` lists like any other fact.
- **Detail nodes were dropped in v5.** Per-fact revisions keep a scene that reads only Mara's eye color from going stale when a plot fact about her changes.

### Person, place, group and item

No extra fields beyond the common core. Facts (including relationship facts) cover them. Roles such as hero and challenger come from thread `characters` fields. **Group** (a guild, a family business, a rebel cell) and **item** (the letters, a key, a ship) were added in v9 so membership and ownership relations have something to point at.

### Relationship vocabulary (starter registry, v9)

| Relation | Type | Cardinality |
|---|---|---|
| `parent_of` / `child_of` | Inverse pair (one direction stored) | Many |
| `sibling_of` | Symmetric | Many |
| `spouse_of` | Symmetric | Single at any in-world moment (can end) |
| `partner_of` | Symmetric (romantic, unmarried) | Many |
| `friend_of` | Symmetric | Many |
| `rival_of` | Symmetric | Many |
| `works_for` / `employs` | Inverse pair | Many |
| `member_of` / `has_member` | Inverse pair (person to group) | Many |
| `bond` | One-way (how one feels about another: trusts, fears, loves, resents) | Per target |
| `located_in` / `contains` | Inverse pair (person or place to place) | Single for a person at a moment; many for places |
| `owns` / `owned_by` | Inverse pair (person or group to item or place) | Many |
| `related` | Fallback; `statement` describes the connection | Many |

Non-relation attributes registered so far: `strength`, `flaw` (single). New relations are added to the registry as stories need them, in `spec/attributes.md` (step 6); a new key resembling an existing one is soft-flagged.

### Scene

```yaml
pov: p-0007
location: l-0003
participants: [p-0007, p-0002]
coverage: {thread: t-0001, stage: crisis}
purpose: "Mara discovers Jonah lied"   # works for open slots too
when: {after: s-0011, offset: "next morning"}
beats: ["...", "..."]
reads: [f-0031, f-0050]     # declared: facts this scene relies on
turn:                       # v9: the scene's change; inferred and confirmed
  wants: "Get the letters past the checkpoint"    # POV character's goal in this scene
  against: "The guard recognizes her"             # what gets in the way
  ends: "She gets through, but Jonah is arrested" # how it ends differently than it began
status: outlined            # slot | outlined | drafted | reviewed | approved (slot added in v9)
```

Writes, hints, and reveals are not stored on the scene. They are generated into `scenes.json` from facts that point at it (`true_from: {in: ...}`, reader moments, character learns). Landmark and framework-beat roles are also generated into `scenes.json`, from the arc's `landmarks` and `beats` maps (v9).

- **Slots (v9):** the weaver's skeleton creates scenes with `status: slot`, holding at least `purpose` and `coverage`, and optionally a partial `turn`. Deepening fills the rest and moves them to `outlined`.
- **The turn (v9):** goal, conflict and outcome (taught with plain names; "scene and sequel" and "value shift" are related craft terms). A scene whose end matches its beginning gets a soft "possible flat scene: what changes here?" One scene's `ends` should lead into a later scene's `wants`, which supports the cause-and-effect check.

### Shape decision

```yaml
question: "Does Mara change or hold firm by the end?"
area: hero          # hero | plot | theme | challenger | relationship
choices: [changes, holds_firm]   # empty = writer's own words (theme)
value: changes
options: []         # same unresolved pattern as facts
evidence: [s-0012, f-0044]       # scenes and facts the inference came from
pivot: f-0050       # optional: the belief whose ending marks the change
confidence: provisional
review: unreviewed
```

- **Evidence** shows the writer why, tells the housekeeper what to recheck if a scene is cut or rewritten, and seeds the soft clash warnings. Evidence can point at beliefs as well as truths.
- **Pivot checks:** for "changes," the pivot belief must end, ideally near the climax (an early end gets a soft "change happens too early?"). For "holds firm," the pivot belief is challenged but survives the climax. The scene where the pivot ends is a candidate for the personal thread's turning-point cell.

### Thread

```yaml
kind: personal          # personal | challenger | relationship | big_picture
arc: a-0001             # the arc this thread belongs to
characters: [{who: p-0007, role: lead}]   # roles: lead, opposite, etc.
decisions: [d-0001, d-0002]
stages:               # always these four keys (v9)
  - {stage: setup, summary: "Mara trusts no one", confidence: provisional}
  - {stage: complications, summary: null}
  - {stage: crisis, summary: null}
  - {stage: resolution, summary: null}
```

Coverage grid cell states (generated into `grid.json`): **empty** (no summary, no scene; becomes a frontier question), **planned** (summary, no scene), **covered** (scenes linked via their `coverage` field). The housekeeper can soft-flag scenes that drift from their cell's summary. Stage keys are settled in v9 (see Coverage grid and movements). A thread opted out with "keep it simple" has no counted cells.

### Viewpoint and style

```yaml
defaults:
  person: third         # first | third | second
  tense: past
  distance: close       # close | distant | omniscient
  tone: "Tense, wry, hopeful underneath"
taste_anchors: ["...", "..."]
viewpoints:
  - {who: p-0007, overrides: {person: first}}
  - {who: p-0002}
voice_refs: []          # links into the style guide folder, later phases
```

Each scene's `pov` resolves its style rules. Voice samples, the flag-and-confirm typo list, and voice features live in the style guide folder, not the spec.

### Arc

An **arc** (internal name; called "storyform" before v9) is one complete argument: a dramatic question, its threads and shape decisions. The main story is one arc and each subplot is another. Writers never see the word as a node type; they hear "the main story" or "the subplot about...". Not to be confused with the craft terms "character arc" and "relationship arc," which are taught to writers and describe how a thread changes.

```yaml
status: main            # main | fog | active | shelved (shelved Proposed in v9: fog kept out of this outline)
threads: [t-0001, t-0002, t-0003, t-0004]   # one per kind; empty while fog
decisions: [d-0001, d-0002]                  # empty while fog
premise:                # writer's own words, versioned for drift checks
  current: "A courier smuggles letters past a blockade..."
  history: [{date: 2026-10-18, text: "..."}]
dramatic_question: "Will Mara get the letters through?"  # plot: answered by the outcome
thematic_question: "Is loyalty worth lying for?"         # theme: answered by how the ending feels
framework: {value: hero_journey, confidence: provisional} # v9; value may be undecided or none
landmarks:              # v9: tier 1, one entry each; null = not found yet (becomes a suggestion)
  inciting_incident: {scene: s-7kq2xmhd, confidence: provisional}
  mc_commits:        {scene: s-p3rwe9ka, confidence: confirmed}
  midpoint:          null
beats:                  # v9: tier 2, from the chosen framework; replaced when the framework changes
  theme_stated:      {scene: s-4hq8nzwt, confidence: provisional}
characters: [p-0007, p-0002]   # fog only: who the subplot involves
```

- **Fog subplots are real arc nodes (v5)** with `status: fog`, holding only a one-line premise and the characters involved. They get stable IDs that scenes, facts, and the weaver can reference before they are mapped. Mapping one flips it to active and fills its threads and decisions.
- **Landmarks live on the arc (v9),** not on scenes: one entry per landmark makes "one inciting incident per arc" hold by shape, avoids ambiguity when a scene is a landmark for both the main story and a subplot, lets movement boundaries be derived from one small map, and makes missing landmarks visible as empty entries.
- `dramatic_question` and `thematic_question` (v7; replace `central_question`) feed the argument-coherence check; `premise.history` is the versioned baseline the housekeeper uses for drift.

### Proposal

```json
{
  "id": "pr-0123",
  "agent": "interviewer",
  "lane": "optimistic",
  "created": "2026-10-18T12:58",
  "answer": "Mara learns the truth from Jonah's sister",
  "ops": [
    {"op": "add_fact", "node": "p-0007", "fact": {"...": "..."}},
    {"op": "set", "target": "p-0007#f-0050.ends", "value": {"in": "s-0031"}},
    {"op": "add_item", "target": "s-0031.participants", "value": "p-0004"}
  ],
  "revisions": {"writes": {}, "read": {}, "checked": {}}   // node and fact revisions
}
```

Facts are addressed by ID (`node#fact.field`), never by list position, so parallel edits cannot shift each other.

**Op list:** `create`, `set`, `add_fact`, `add_item` and `remove_item` (for list-valued typed fields such as `participants`; confirmed in v9, replacing `add_link`/`remove_link` now that node-level links are gone; also the reading order, target `sequence`, from step 7), `supersede` (replacement required), `delete` (zero references plus draft check), `merge` (redirect references, then attribute uniqueness check), `move_fact` (keeps ID), `reclassify` (truth to belief or back), and `add_attribute` (a story key into `spec/attributes.md`, step 7). Split is handled by the interviewer as per-reference changes. Exact shapes in `docs/check.md`.

### Backlinks index

```json
{
  "f-0031": [
    {"from": "s-0012", "via": "reads"},
    {"from": "d-0002", "via": "evidence"}
  ],
  "s-0004": [
    {"from": "p-0007", "via": "fact:f-0031.reader.hinted"}
  ],
  "p-0007": [
    {"from": "s-0031", "via": "pov"},
    {"from": "p-0002", "via": "fact:f-0060.sibling_of"}
  ]
}
```

Keyed by target ID: whatever is referenced (corrected in step 6; an earlier version listed the reader hint under the fact). Each entry records how the target is referenced, which the delete check, housekeeper scope, and impact lookup all need. A reference from inside a fact is recorded from the fact's home node, with the fact ID in `via`.

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

## Checks: who catches what (new in v9; all Decided)

**Script soft checks** (in `complete`, run on every commit; no model, no tokens):

- Missing landmarks (empty entries in an arc's landmark map)
- Scene-count pacing per movement
- Unearned twist (a reveal with no earlier hint) and dangling setup (a hint never revealed or used)
- Early pivot (the pivot belief ends well before the climax)
- A subplot resolving after the main climax
- Threads out of alignment across movements
- Similar attribute keys (name matching)

**Housekeeper judgment checks** (scoped, rolling, full and session-open passes):

- Premise and idea drift against the versioned baseline
- Motivation: does an action make sense given what the character knows and wants?
- Cause and effect: does one scene's ending actually lead into the next?
- Flat scenes (the turn's start and end differ in words but not in substance)
- A scene drifting from its grid cell's summary
- A thread that seems to carry two separate stories (the promote-to-subplot note)
- Soft clashes between story content and shape decisions
- False beliefs never corrected or paid off, and facts that end with no cause

Script findings use the same plain-language flag flow, so the writer never sees the difference. Keeping countable checks out of the housekeeper makes it cheaper and focuses the cheaper-model evals (item 7) on judgment alone.

**Completeness rubric** (sub-agent; the deep half of `complete`; run at the weave and outline gates and on request):

1. **Cells meet their stage.** A filled cell must do what its stage requires (a Crisis cell where nothing reverses is "filled but not met").
2. **Decisions have support.** Each answered decision is backed by real scenes or facts in its evidence.
3. **The ending answers both questions.** Resolution answers the dramatic question and reflects the thematic question, consistent with decisions 8 and 9 (plot-hole check 4).
4. **The threads interlock.** The climax resolves more than one thread, and every thread bears on the thematic question (adopted from Proposed).

Checks tied to anything opted out with "keep it simple" are skipped. At the gates, any "not met" must be resolved or acknowledged before weaving or outlining continues.

**Flag budget** (applies to housekeeper, rubric and script soft findings):

- Each finding carries a confidence level, the IDs it rests on, and a suggested plain-language question.
- Low-confidence findings are logged, not raised; later passes can see them, so a strengthening pattern can surface later.
- The interviewer raises at most one or two per natural pause, ranked by impact: anything touching a confirmed decision first, then highest fan-out, then the rest.
- The rest wait in a queue of open findings. The orchestrator offers them when momentum slows, and all must be resolved or acknowledged before weaving and outlining (the existing gates).
- Hard script failures are never budgeted: they are raised immediately because the proposal cannot commit until they are fixed.

## Story model and coherence checking

**Decided**

- Build our own validation as a skill primitive rather than depend on any licensed story-theory tooling.
- Top priority: a "complete" story that addresses all four threads per arc.
- Cover all five plot-hole categories: (1) continuity and timeline, (2) knowledge and motivation, (3) setups without payoffs and payoffs without setups, (4) argument coherence (the ending answers the story's central question), (5) cause-and-effect gaps between scenes.
- One arc per map for now. Potential subplots are recorded in the spec as fog items for future mapping sessions.
- Hard timeline, knowledge ordering, and attribute uniqueness violations are caught per proposal by the dependency checker (see Governance).
- **Reader-facing checks run per edition (v7).** Reveals, hints, setups and payoffs, and cause and effect between visible scenes depend on each edition's scene list; truth, timing and character knowledge are checked once in canon.
- **Knowledge checks use truth and belief facts (new)**, for example "Mara acts on knowing Jonah lied in scene 9, but only learns it in scene 14." The housekeeper soft-flags false beliefs that are never corrected or paid off, and facts that end with no cause (possible cause-and-effect gap).

**Proposed**

- *Superseded in v7:* plain labels for the four threads are now main plot, inner journey, challenger and relationship (see Story-shape layer).
- *The coverage grid itself is Decided in v9* (see Coverage grid and movements). Still proposed: an interlock check that the climax resolves more than one thread and every thread bears on the thematic question.
- Deterministic scripts for links, status, timelines, and a setup-and-payoff ledger. Rubric-guided model checks reported as "possible issues," not verdicts. A small argument-coherence rule set written in our own vocabulary, which tests the ending against the story-shape decisions.
- Evals using stories with seeded plot holes to test the checker (the skill-creator skill supports evals).
- Never reproduce any outside theory's proprietary terminology, definitions or matrices; see the practices in `docs/background.md`. This is not legal advice.

## Coverage grid and movements (new in v9; all Decided unless marked)

- **Four fixed stages for every thread**, whatever framework is used. Reasons: completeness means the same thing in every story, so the checker, evals and housekeeper never change with the framework; four stages is one idea a novice can hold.

  | Stage (key) | Plain requirement, any thread |
  |---|---|
  | **Setup** (`setup`) | How things stand, and what disturbs them. |
  | **Complications** (`complications`) | Pressure builds; first attempts fail or cost something. |
  | **Crisis** (`crisis`) | A reversal or discovery changes the direction; ends at the lowest point. |
  | **Resolution** (`resolution`) | The climax and its aftermath; the thread's question gets answered. |

  "Crisis" was chosen over "turning point," which craft writing uses loosely (sometimes for the midpoint).
- **Per-thread readings** of the four stages (stage 4 of each is tested against that thread's shape decisions):

  | Thread | Setup | Complications | Crisis | Resolution |
  |---|---|---|---|---|
  | **Main plot** | The world as it is, the problem arrives, and what is at stake if it isn't solved. | Efforts toward the goal meet growing obstacles; costs pile up. | A major reversal; the goal looks out of reach and the consequence is close. | The final confrontation; the dramatic question is answered (outcome). |
  | **Inner journey** | The MC's belief and its cost. | The belief is strained. | It cracks, or is doubled down on. | The MC changes or holds firm (the pivot belief ends or survives). |
  | **Challenger** | Who they are, and how their view differs from the MC's. | Their pressure on the MC grows, through example, argument or conflict. | Their challenge peaks; the MC can't ignore it anymore. | They change or hold firm, usually the opposite of the MC. |
  | **Relationship** | How the bond stands. | It is tested. | It reaches a breaking point. | It deepens, breaks, or stays unresolved. |
- **Alignment within an arc:** each thread's stage N falls in roughly the same part of the book, giving four shared **movements**, each advancing every thread. Soft guidance only: a thread running early or late gets one "possible pacing issue," then the writer's choice is respected. Lined-up resolutions support the proposed interlock check.
- **Subplots float independently** of the main arc (they may span the whole book or only part of it), but their own threads align with each other and they get their own four movements.
- **Subplot minimum:** its own dramatic question, stakes, and at least one thread with all four stages. Other threads accept "keep it simple"; infer-and-confirm suggests which threads the subplot really has. Every thread that exists must have all four stages filled. A subplot can grow into a full arc by adding threads.
- **Cross-arc soft checks:** a subplot resolving after the main climax gets a possible pacing issue; every subplot should touch the main arc's thematic question.
- **Two tiers of beats, all soft (never completeness blockers):** *(Exception decided October 19, 2026: the three movement landmarks and the climax block the outline gate, provisionally satisfiable; see Step 9 planning.)*
  - **Tier 1, landmarks (owned by the grid, every story):** Setup: show the normal world, the inciting incident, the MC commits (end of setup). Complications: rising stakes, the midpoint shift (end of complications). Crisis: things fall apart, the low point (end of crisis). Resolution: the climax, the new normal.
  - **Tier 2, framework beats:** supplied only by the chosen framework (for example "theme stated," "refusal of the call") and slotted into the same four stages.
  - **Behavior:** inferred from described scenes and confirmed ("That sounds like the moment that kicks off the story; is that right?"), which tags the scene and teaches the craft term. Missing ones become a suggestion, explained once, then respected. Attached to the main plot by default, except where a landmark plainly belongs to another thread. Subplots get landmarks too, more lightly. Beats tied to theme (such as "theme stated") are suggested only once the thematic question exists.
- **Movement boundaries are derived, never stored:** end of Setup = the scene tagged "MC commits"; end of Complications = the midpoint scene; end of Crisis = the low-point scene; Resolution = everything after. Moving a landmark scene moves the boundary; `sequence.md` stays pure. Before landmarks exist, scenes fall back on their `coverage` stage and the skeleton's provisional placements. A generated index view shows the sequence split into movements. Subplots use the same rule with their own landmarks.
- **Pacing in phase 1 uses scene counts only,** in plain words ("your middle may feel rushed"), as soft notes from the housekeeper or completeness rubric. Relative balance across movements is canon in any edition; word counts are not (see below).
  - **Expected share** per movement comes from the framework's proportions (four-part: about a quarter each; other frameworks supply their own).
  - **A soft note when a movement is under half or over double its expected share** (both thresholds are settings), phrased as a story effect with one sentence of why ("your Crisis has 3 of 40 scenes; the low point may arrive before readers feel the pressure build").
  - **Only once the skeleton exists**; explained once, then respected. Subplots get the same check against their own span, more loosely.

## Weaver, outliner and frameworks

**Decided**

- **Framework choice** (refined in v9): the weaver chooses a framework (four-part, hero's journey, Save the Cat, and so on) with the writer and teaches the tradeoffs.
  - **Inferred any time.** If the writer's scenes clearly follow a framework, Skaldsaga confirms ("this looks like the hero's journey; is that what you're going for?"). A confident yes is recorded as provisional; confusion or "not sure" parks it until weave time.
  - **Recommended late,** at weave time, from taste anchors, genre and the writer's goal.
  - **Four-part is the default** if nothing points elsewhere (it maps one-to-one onto the four stages). **Switchable any time** (only tier-2 beats re-map). **"No framework" is valid** (landmarks only).
  - Stored on the arc node (`framework`).
- **Layered outline (v9):** a **skeleton pass** across the whole book creates scene slots described by purpose, each linked to a grid cell and any landmarks; then a **deepening pass, one movement at a time, starting with Setup,** adds beats, POV, location, reads and the scene turn. One movement per session fits the clear-and-recap rhythm, keeps sub-agent work bounded, and keeps hand-back questions topical. Later movements may send revisions back to earlier ones; the housekeeper catches the ripple.
- **The weaver produces the skeleton; no separate woven plan (v9).** The weaver chooses the framework (hand-back), resolves fog subplots, then interleaves every thread's cells, landmarks and beats into ordered slot scenes in `sequence.md`, built around pins. The outliner only deepens, one movement per dispatch. Everything weaving decides already lives in slots, their `coverage`, their order, and the arc's landmark and beat maps, so no intermediate artifact can go stale. Split of judgment: the weaver does whole-book structure, the outliner does scene craft. Full housekeeper passes gate both the weaver and the start of deepening.
- **Layers set the work order, not a depth limit (v9).** Anything the writer raises early (a detailed climax in session two, say) is captured at whatever depth it arrives, provisional like any answer. It becomes a **pin**: the skeleton pass builds around existing detailed scenes rather than over them, and conflicts are flagged as usual. This is the outline-level version of allowing drafts before structure.
- **The skeleton is sized by story need (v9):** at least one slot per grid cell, plus landmarks and framework beats, plus pinned scenes. No word targets.
- **Fog subplots at weave time (v9):** before weaving, the writer chooses for each fog subplot: **map it now** (interview until it meets the subplot minimum, then weave it), **shelve it** (left out of this outline and kept for later), or **fold it in** (it is really part of an existing thread; its facts move there with `move_fact` and the fog node is deleted). The orchestrator raises these as weave time approaches, in plain words. Shelved subplots are not gaps.
- **Promote to subplot (v9, phase 1):** the reverse of fold-in, for any thread content, including content folded in earlier. A new arc node is created (or a shelved one revived), facts move with `move_fact` (IDs kept, so reads and links survive), and the interviewer asks about each reference as in a split. Emptied cells in the original thread are flagged by the grid; the new subplot must meet the subplot minimum before weaving. Writer-started; the only automatic part in phase 1 is a soft housekeeper note when a thread seems to carry two separate stories. Automatic promotion suggestions are deferred.
- Authors may aim at traditional publishing, self-publishing, or just finishing a story. The skills ask the writer and adapt.
- **Length moves out of phase 1 (v9; supersedes the v1 serial length profile in phase 1).** Word budgets belong to editions (a novel's total, a serial's per-update target), and phase 1 has no prose, so counts would be guesses. A later phase's edition step compares real length with the edition's target and drives revisions that teach why length matters (too fast here, too slow there, over or under budget). Full serial support (a parent map plus arc maps) stays deferred.

**Proposed**

- Name frameworks but write original descriptions, since some beat sheets are trademarked or book-specific.
- For the later length phase: a length data file with ranges by genre and publishing path, with sources and dates, producing a target length, budgets per edition grouping, subplot-fit checks and per-scene estimates. All advice with reasons, never hard rules.

## Future phases (recorded ideas, not in scope now)

**Decided as ideas**

- **Automated multi-agent harness** with the outline split into discrete tasks with dependency blockers. A task moves from blocked to ready as its dependencies are settled. This also allows what-if prototyping and parallel agent activity.
- **Voice development:** the writer writes sample text for each voice (narration per viewpoint character, dialogue per character, and so on) so agents have a style to work from and any automated writing stays in the writer's own voice.
- Raw voice-sample files are kept as raw files. A basic spelling and grammar check runs on them before the style guide is built, **flag-and-confirm**: the raw file is never edited, suspected errors are listed, the writer marks each as a typo or a deliberate choice, typos are excluded from style analysis, and deliberate choices are recorded as voice features (for example, "Mara drops her g's and speaks in fragments").
- Writer and review agent pairs, plus editor agents for pacing, flow, grammar, and voice.
- The phase 1 version-check rule may need revisiting once parallel scene agents exist.
- **Parallel attribute safety (new):** if two writer agents propose the same entity attribute, the single committer applies one and the other fails the uniqueness check and is re-checked. No silent drift.
- **Series shelf (new):** unused facts carry over as material for later books.
- **Book series (raised in v7, not designed).** A book is a story-level unit with its own arc, central question, ending and shape decisions, so book splits do not belong in edition files (an omnibus could later be an edition spanning books). Phase 1 keeps the doors open by using random IDs and keeping arcs as separate nodes, so one arc could later cover a range of the sequence. Questions for a later phase:
  - One project with books as divisions of a single sequence, or one project per book sharing a fact library?
  - If separate projects, how are facts shared or migrated between them (unlike a fork, a series needs them kept in step)?
  - How a published book constrains later books (likely the same pattern as a serial in progress constraining canon changes).
- **CI/CD framing for the whole pipeline (decided this session as the long-term shape):**
  - **CI = making the story.** Proposals are like pull requests, the dependency checker and housekeeper are the tests, and the single committer is the merge. Outline (this phase), drafting and editing are CI stages. "Green" means a complete, approved story.
  - **The build artifact** is a pinned, git-tagged version of the approved sequence and manuscript. Every CD run builds from one pinned version, so all editions come from the same text. *Refined in v7:* for editions released in parts, each released grouping records its own pinned version (see Editions under Spec and storage).
  - **CD = one pipeline per edition** (the editions above are its deployment targets). Each holds format-specific agents and checks: Kindle (EPUB validity, table of contents), print (trim size, margins, page count, spine width), web serial (update lengths, release schedule, platform formatting), and so on.
  - **CD can block and send findings back to CI.** Some format needs are really story needs (a serial update should end on a hook, which is pacing). CD fails like a broken deploy and files proposals in the inbox; it never patches canon itself (the same one-way rule as the harvester).
  - **Deferred to the CD phase:** whether an edition may carry its own extra text (serial recaps, chapter titles, front matter) or every word must come from CI. The author leans toward allowing CD-specific rewrites. *Settled in v7:* editions may cut scenes and use alternative scenes (which are spec scene nodes), but never change canon facts.
- **Web front end (research; retargeted to Rust in v8):** a friendlier home than a terminal or VS Code for non-programmers. The long-term aim is a single Rust binary that embeds the skill and agent prompts, serves a web interface to the agents and the manuscript, and uses Claude through the API. Leads to check:
  - **Agent loop.** The Claude Agent SDK runs Claude Code's agent loop from TypeScript (and Python), not Rust (verify). Options: write our own loop over the Claude API (tool use, prompt caching), drive Claude Code as a subprocess in headless mode, or a community Rust client.
  - **Authentication and billing.** Calling the API means an API key and per-token billing, which a novice may not expect; whether a writer's Claude subscription can be used instead must be checked against Anthropic's current terms.
  - **Licensing and usage terms** for distributing such an app.
  - The phase 1 binary and the later app share one core library (`skaldsaga-core`).
  - **Native iOS and Android (raised in v9):** a phone app wrapping `skaldsaga-core` with a thin mobile interface, distributed through the App Store and Play Store. Writers often capture ideas on the go. Phase 1 keeps the core portable so this stays possible.

**Proposed**

- An agent-drafted scene never becomes a voice sample unless the writer approves it, so the style does not drift toward the machine's habits.
- A writer sub-agent can be held to its scene packet: limit its tools, block reads outside the packet folder with a hook, and omit project instruction files (`omitClaudeMd`).
- Run parallel scene tasks in separate workspaces (sub-agents can run in a temporary git worktree). Scenes that write the same node are serialized. A scene that follows another in reading order waits for its outcome.

- **Craft cheat sheet (noted October 19, 2026):** a short reference of the craft terms the writer has earned, kept in the story folder. Parked: phase 1 teaching is invisible, through the interviewer only.
- **Writers contributing craft concepts (noted October 19, 2026):** since Skaldsaga is open source, a later phase could ask writers, opt-in, whether to contribute the craft concepts researched in their project (story content stripped) to the built-in list. Phase 1 only builds the export capability (open item 19).
- **Gamified craft learning (noted October 19, 2026):** if the writer's craft knowledge is tracked against a rubric (open item 19), it could become visible progress for the writer, such as craft skills unlocked as they are learned and applied. Idea only.

## Research notes (verify before relying)

- **Story-theory background and licensing** (the original inspiration, its terms, what we did not adopt, and licensing findings) live in `docs/background.md`, which build sessions do not load.
- **Wayfinder (SKILL.md read, v9; mattpocock/skills, MIT licensed, commit of October 9, 2026).** A user-invoked planning skill for efforts bigger than one session. Structure: a **destination** named first (it fixes scope); a **map** that is an index, not a store (one line per decision, the detail lives once in its ticket); **decision tickets** typed grilling (HITL, default), prototype (HITL, a cheap artifact to react to), research (AFK, run by sub-agents in parallel) and task (manual work that unblocks a decision); **fog of war** ("Not yet specified"), whose test is whether a question can be *stated* precisely now, not whether it can be answered; an **out of scope** list that never graduates; the **frontier** (open, unblocked, unclaimed tickets) with claims made by assignment; refer to things by name, never bare IDs; at most one non-research ticket per session; plan, don't build. Field reports in its docs: over-charting up front goes stale (27 tickets, wrong by the 13th); agents writing their own exemptions into the map's Notes and then obeying them; a prototype agent choosing among its own variants; long, verbose questions causing decision fatigue and hiding why each question is asked; agents designing around a bad closed decision instead of challenging it. Lessons for Skaldsaga are in "Lessons from wayfinder" under Open items 9.
- **Serial length (anecdotal).** Chapters run about 1,500 to 4,000 words, commonly 2,000 to 3,000. Royal Road rewards frequent updates, and many authors post weekly or twice weekly. Traditional and self-publishing length norms have not been researched yet.
- **Claude Code subagents (official docs, checked October 2026).**
  - `AskUserQuestion` is removed from every subagent. A subagent starts with a fresh context window, works independently, and returns only a summary. The docs recommend the main conversation for tasks needing frequent back-and-forth. This is why the interviewer cannot be a sub-agent.
  - Skills load into the main conversation context (they cost context while active), whereas subagents run in an isolated window.
  - A custom agent can run as the whole session (`claude --agent <name>`), and the `tools` field can restrict which subagent types it may spawn (`Agent(type, ...)`).
  - A subagent's frontmatter supports `tools` and `disallowedTools`, `hooks` (such as `PreToolUse` to block certain actions), `omitClaudeMd`, `skills` preloading, `isolation: worktree`, `memory`, and `maxTurns`.
  - Nested subagents are possible but limited by depth, and there is a concurrent subagent limit. Rolling housekeeper passes and live confirmed-decision passes run as background sub-agents alongside the main-thread interview.
- **Claude Code status line (official docs).** The status line script receives JSON on stdin including `context_window.used_percentage` (input tokens only), `context_window_size`, and `session_id`. It runs locally and does not use API tokens. A script can write that value to a file. *No documented way was found for a hook or skill to clear a session automatically.* *Unverified:* whether a plugin can ship or register the status line; whether hooks receive the context percentage.
- **Claude Code runtime and plugin facts (official docs, checked October 18, 2026):**
  - **No script runtime is guaranteed.** The recommended native installer ships Claude Code as a standalone binary; it brings neither Node nor Python (only the npm install path needs Node 22+). On Windows, Git for Windows is optional; without it Claude Code runs commands through PowerShell. Python on macOS is not guaranteed either (general knowledge, not from the docs: `python3` there is often a stub that prompts to install developer tools).
  - **Plugins cannot register the main status line.** A plugin's `settings` only honor `agent` and `subagentStatusLine`. The status line can be set in project settings (`.claude/settings.json`), so first-run setup must write it there, behind the permission prompt. On Windows the status line runs through Git Bash or PowerShell, and file paths need forward slashes. `context_window.used_percentage` counts input tokens only and may be null early in a session.
  - **A skill's `model` field lasts one turn only.** It applies for the rest of the current turn; the session model resumes on the next prompt. So the orchestrator and interviewer effectively run on the writer's chosen model. Skills also support `context: fork` with `agent`, which runs the skill in a sub-agent instead.
  - **Plugin facts:** components are namespaced under the plugin name; skills can bundle scripts in any language and reference them with `${CLAUDE_SKILL_DIR}`; hook commands get `${CLAUDE_PLUGIN_ROOT}` and `${CLAUDE_PROJECT_DIR}`; a top-level `bin/` folder blocks installation through claude.ai and Cowork, so bundled executables belong elsewhere (for example `scripts/`). A plugin's `settings.agent` can make an agent the main-thread agent, but it would apply in every project where the plugin is enabled, so it does not suit Skaldsaga's guarded start.
- **Rust delivery (general knowledge, v8; verify before relying).** cargo-dist can automate cross-platform builds, GitHub Releases and installers. macOS quarantines downloaded unsigned binaries (Gatekeeper); signing and notarization need an Apple developer account. Windows SmartScreen warns on unsigned executables. Downloading a binary on first run needs a download tool: curl ships with macOS and recent Windows, and PowerShell can download too. *Unverified:* whether a plugin installed through Claude Code keeps the executable permission on bundled files, and whether macOS quarantines files a plugin install writes.
- **Anthropic harness guidance** ("Effective harnesses for long-running agents"): agents work in discrete sessions with no memory between them, so use a progress file plus git history to orient each fresh session, track features in JSON with a pass/fail flag, work one feature per session, and use separate checkers because agents judge their own work poorly.

## Open items

1. *Closed:* `ARCHITECTURE.md` reviewed this session (see What changed since v5).
2. *Closed:* sub-agents that need the writer use the hand-back pattern (see Architecture).
3. *Closed in v9:* the 12 shape decisions are final with plain-language wording (see Story-shape layer); the theme layer collapsed to the thematic question; node fields for the adopted story points are settled (see item 17).
4. *Closed in v9:* four fixed stages (Setup, Complications, Crisis, Resolution), aligned within an arc, with landmarks and framework beats on top (see Coverage grid and movements).
5. *Closed in v9* (see Weaver, outliner and frameworks, and Coverage grid and movements): layered outline, weaver produces the skeleton, outliner deepens, pins, framework timing, fog subplot outcomes, promote to subplot, stage readings for all threads, arc-level landmarks, the scene turn, and the scene-count pacing heuristic.
6. *Closed at design level in v9* (see Checks: who catches what): script versus housekeeper split, the four completeness-rubric judgments, and the flag budget. Exact prompt text, confidence scales and message wording are build work.
7. *Closed at plan level in v9* (see Testing and evals). Remaining is build work: writing and reviewing the test stories and fixtures.
8. *Deferred in v9 (length left phase 1):* research traditional and self-publishing length norms for the length data file, when the edition phase begins.
9. *Closed in v9 (research):* wayfinder SKILL.md read (see Research notes). **Lessons from wayfinder** (confirmed by the author, so all Decided except the last bullet):
    - **Keep questions short.** Wayfinder's sharpest user complaint is long, verbose questions causing decision fatigue and hiding why each is asked. Skaldsaga's teaching asides make this a bigger risk for us: one short question, one sentence of why, and the longer teaching only on request ("want to know more about why this matters?").
    - **Ruled-out ideas list.** Wayfinder's out-of-scope list never graduates. Skaldsaga equivalent: ideas the writer has explicitly rejected are recorded so no agent suggests them again (different from shelved subplots, which may return).
    - **Agents cannot loosen their own rules.** Wayfinder agents wrote exemptions into the map's Notes and then obeyed them. In Skaldsaga, only the writer (through the orchestrator) changes settings, gates or opt-outs; a sub-agent proposal that touches settings or governance is a hard failure.
    - **The writer chooses.** A wayfinder prototype agent picked its own variant. Skaldsaga's what-if scene variants and any multiple-option proposals must be chosen by the writer, never by the agent that made them.
    - **Challenge, don't design around.** Agents tend to build around a closed decision that has started to look wrong. The housekeeper's soft clash check should explicitly suggest revisiting a confirmed decision when evidence against it accumulates.
    - **Fog versus question test.** Something becomes a frontier question when it can be *stated* precisely, even if it can't be answered yet; until then it stays a loose "things we'll get to later" note. This matches our unresolved facts versus fog subplots.
    - **Already aligned:** index-not-store (our generated index), refer by name (names in front matter, IDs hidden), one topic per session, no pre-slicing the fog, the human speaks for themselves (the interviewer never answers its own questions).
    - **Research sub-agent (Decided in v9):** see Architecture.
10. *Closed in v9 except research:* plugins cannot ship the main status line, so setup writes it into the project's `.claude/settings.json`; an existing personal status line is wrapped, not replaced; starting values for all settings are in Default settings. Still unverified: the SessionStart `initialUserMessage` field, and hook access to context usage (no longer needed).
11. *Closed in v9 (research, not legal advice):* licensing findings, attribution and practices are in `docs/background.md`; our design uses none of the licensed material. The internal rename (storyform to arc) and the terminology scrub are done. Remaining: have the trademark and patent points checked before public release.
12. *Closed in v9:* starter relation vocabulary and the `group` and `item` node types (see Node format); the `add_item` and `remove_item` ops are confirmed.
13. *Closed in v9:* binaries reach the writer through a CI-built distribution branch, in `scripts/bin/<platform>/` (see Build, release and delivery). Remaining checks are listed there under To verify.
14. *Revised in v9:* phase 1 no longer proposes a first edition. The minimal edition file format and edition rules stay decided for the later phase that builds editions, which also extends the format with cuts, alternative scenes, per-grouping release status and pins, and review snapshots.
15. How reviewer and reader feedback is stored in `reviews/` and turned into questions about the current text (mostly later phases; the snapshot link is decided).
16. How forking a project for a retelling works in practice (later phase; the rule is decided).
17. *Closed in v7:* extra story points adopted, in our own words, inferred and confirmed like everything else. **Stakes are required** (part of the required core); the rest are optional depth that accepts "keep it simple." Node fields settled in v9 (no new node types; see the notes after this list):
    - **Stakes (required):** the goal plus what happens if it fails.
    - **Pressure and payoff along the way:** costs, small wins, warning signs that the consequence is near, and the steps required to reach the goal. Useful to the outliner for pacing.
    - **Strength and flaw** for the MC and challenger; "fatal flaw" is a familiar craft term.
    - **What's really wrong versus what the MC thinks is wrong**, and what they try versus what would fix it. Ties to the pivot belief and to common craft ideas such as "want versus need."
    - **Backstory** for the MC, the challenger and their relationship, feeding facts.
    - Points considered and not adopted are listed in `docs/background.md`.
    - **Storage (v9):** stakes sit with the dramatic question on the arc (decision 5); strength and flaw are person facts with registered attributes `strength` and `flaw` (single cardinality, so they can change over time with `ends`); "what's really wrong versus what the MC thinks" is a belief fact (the mistaken idea, often the pivot belief) contradicting a truth fact (the real problem), building on decision 2; pressure along the way is facts with a `role` tag (`cost`, `small_win`, `warning`, `required_step`) linked to the main plot thread, which the outliner spreads across movements; backstory is ordinary facts whose `true_from` is before the first scene. All of them get time tracking, knowledge tracking and plot-hole checks for free.
18. *Closed in v9:* launch targets, cargo-dist, shared versioning, staged signing, and the crate layout (see Build, release and delivery). Signing costs and services still need verifying before purchase.
19. *Phase 1 scope (Decided October 19, 2026):* **track the writer's craft knowledge and teach through primitives, not a separate agent.** Built alongside the interviewer and orchestrator work, after step 9: the built-in concept list, the writer's craft-knowledge and teaching-preference record, and researched concepts with their export command, all used by the interviewer and orchestrator. Reason: a sub-agent can't see the main conversation, and teaching happens inside the interviewer's questions, so a separate teacher would add hand-offs and break the one-short-question rule. Original note: At minimum, record which terms the writer has been taught, so the terminology ladder knows when to shorten "main character" to "MC" or switch to the story's names. Beyond that, consider a rubric of craft knowledge to measure the writer against (for example: knows the term, can apply it to their own story, uses it unprompted), tied to the tracked terms and concepts, which could also shape how much teaching each question carries. Storage shape is build work (for example a small writer-profile file in `.harness/` with a `format` field, so it can grow). Writer state, not story meaning, so it stays out of the spec. *Future note:* the rubric could later be gamified for the writer (progress through craft skills); see Future phases.
    - **Lessons from Matt Pocock's `teach` skill (Research, read October 19, 2026 from `mattpocock/skills`, MIT; the repo credits Matt Pocock only). Decided by the author:**
      - **Teaching is invisible.** It happens only through the interviewer, as background guidance inside the normal conversation. No separate teaching mode, lessons or formal output (`teach`'s HTML lessons are dropped entirely).
      - **Two levels at minimum: taught and demonstrated.** Coverage isn't learning: a concept is *demonstrated* only when the writer uses it correctly in their own story. What the writer says they already know ("I know what a theme is") counts only as *taught*, which lets the interviewer skip the long explanation; it never reaches demonstrated on the writer's word alone.
      - **Corrected misconceptions are recorded** and are the most valuable records, since they predict later stumbles.
      - **Each concept has one chosen term, with aliases to avoid,** and a term is shortened (for example "main character" to "MC") only once it's earned. This is the terminology ladder; the avoid list matches the existing scrub (for example "protagonist").
      - **Teach only what the current question needs,** at the frontier, anchored in the writer's own story and names. Explaining is the light part; applying it to their story is the practice.
      - **Records are superseded, not deleted,** when understanding changes (same as the writer-decision envelope).
      - **A light starting check** of the writer's experience (have they written before, read craft books?), since `teach`'s most common complaint is assuming knowledge with no assessment. **Never use a term before it's taught.**
      - **Stop teaching a concept once the writer uses its term unprompted.**
      - **The program shuffles writer-choice options** (what-if variants and any multiple-option question). In `teach`, correct quiz answers kept landing in the first slot and prompt wording didn't fix it; only shuffling in code did. Position must not sway the writer's choice.
      - **Practice by applying, never quizzing.** After a concept is taught, the interviewer later asks the writer to place it in their own story ("Which scene feels like Mara's lowest point?"), and later still uses the term without explaining it to see whether it stuck; each counts as evidence toward demonstrated. Guard: only when the question is useful to the story anyway, never a check for its own sake.
      - **Two sources of craft teaching (hybrid), so we never assume we know what a writer needs:**
        - **Built-in concept list:** the concepts Skaldsaga's own process requires (the 12 shape decisions, the four stages, landmarks, main character versus POV character, and so on), since our questions depend on them. Written once in our own words, reviewed, updated per release. Each entry: plain explanation, chosen craft term, shorthand, aliases to avoid, and the concepts it builds on. The interviewer adapts the wording to the writer's story but doesn't invent the definition.
        - **Researched on demand:** concepts the writer's own story or curiosity calls for (for example an unreliable narrator, mystery clue placement, romance pacing). The researcher sub-agent extends from real-world facts to craft: triggered by real need (the writer asks, shows confusion, or reaches for an off-list technique), only on request or an accepted offer ("Want me to look into how other writers handle that?"). It finds high-trust primary sources and writes an entry in our own words with citations (never copying the source's text), translates into our vocabulary and checks the avoid list so outside story-theory terms don't leak in, and stores the entry per project, marked provisional. Teaching from it follows the same invisible rules.
        - **Capturing well-trod needs (capability required in phase 1):** researched entries use the same format as the built-in list, with the story-specific examples kept separate from the definition, so promoting one is a copy and a review. A command exports a project's researched concepts (story-free) so the author, from their own testing and use, can collect frequent ones and add them to the built-in list in a later release. Writers' story folders are private, so nothing is collected automatically. *Future phase:* ask the writer whether they want to contribute their researched concepts to the open-source project (opt-in, story content stripped); see Future phases.
      - **Teaching preferences are writer state,** kept next to the craft-knowledge record, not in `.harness/settings.json` (settings are tunable behavior; this is about the person and changes as they grow). A preference ("less explaining, please", "I like knowing the why") shapes how much teaching rides along with a question, but never removes the one sentence of why. The writer changes it any time just by saying so.
      - Skipped for phase 1: timed spaced repetition, and pointing writers to communities (fits the later reviews phase).

20. *Parked (raised by the author October 19, 2026; discuss after the step 9 planning questions):* **a story map in the session-start recap.** Open each session with an overview of where the project stands: in the outlining phase, the outline's shape and its gaps; otherwise overall project status. Worked into the conversation, ending with the writer choosing ("Here's what's still open. What do you want to work on today?"). Claude's note: `complete` (step 9) already produces the data (gate readiness, blockers, empty grid cells, queued findings) as codes and IDs, so this is mostly orchestrator wording plus possibly a compact visual. Questions to settle: how much to show on a phone screen, whether it's text or a picture, how it fits the one-short-question rule, and how it relates to the existing recap.

## Suggested next steps

Build order for the deterministic half of phase 1: `index` (step 6, done), `check` with the proposal format and the operation engine (step 7, done), `commit` (step 8, done: the single committer with the version check, reusing the engine, plus the demo), then a fix pull request for the 0.3.0 phone-test findings (see Current stage), then step 9: `complete` with the script soft checks. Each step that adds something testable from the phone gets a release (see Current stage). After that come the interviewer and orchestrator prompts and the sub-agents, with the craft-teaching primitives (open item 19, added to phase 1 on October 19). Remaining open items are later-phase work (15, 16) and a pre-release legal check (11).
