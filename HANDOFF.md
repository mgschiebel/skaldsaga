# Skaldsaga: Handoff index

*Index and reference format since October 24, 2026. This file is the only one a session reads at the start. The detail lives in `docs/handoff/` and is loaded only when the work needs it. Earlier versions of this document (v9 and before) are preserved word for word in those files; see the lookup table at the bottom.*

## How to use

1. Read this file. Don't load anything else yet.
2. Read `CLAUDE.md` if the session will build.
3. Load only the reference file the topic needs (File map below). Never load `docs/background.md` (it keeps outside story-theory terms out of prompts and code) or `docs/handoff/build-log.md` unless you are looking for the history behind something.
4. Source-code comments say things like "see HANDOFF, Node format". Use the lookup table at the bottom to find the file.

Suggested opening prompts:

- **Planning session:** "Read HANDOFF.md. Continue from Next, one short question at a time with a sentence of why, and write each decision into the matching reference file as we make it."
- **Build session:** "Read HANDOFF.md and CLAUDE.md. Build only what is listed under Ready to build, and ask me one short question before any decision the handoff doesn't cover."

Labels used in the reference files:

- **Decided**: confirmed by the author.
- **Proposed**: suggested in discussion, not yet explicitly confirmed.
- **Research**: from web searches or documentation; verify before relying on it.

Build steps are counted as **steps** (Step 1, Step 2 …), not pull requests; each step cites its GitHub pull request and branch, for example "Step 6 (#11, branch `build/index`)". Small fix and doc pull requests get no step number.

## Keeping this handoff current (Decided October 24, 2026)

**Every pull request leaves the handoff ready for a fresh session to pick up once it is merged.** Each PR, including doc-only ones:

- updates the **Status** and **Next** blocks below (version, what just merged, what is pending, what is ready to build);
- adds or updates its entry in the right reference file (a build step or fix goes under Build progress in `build-log.md`; a design decision goes into the design file it belongs to, with the date and the label Decided);
- updates the File map or the lookup table if it adds, moves or renames a section.

Planning sessions do the same: decisions go into the matching reference file as they are made, with a one-line pointer here only when the Status or Next block changes. The reason: a session that starts after the merge, possibly on another device, has only the repository, so anything not in the handoff is lost.

## Status (last updated October 25, 2026)

- **Phase 1, deterministic half: built.** Steps 1 to 10 are done (meter pilot, serde, plugin, cloud setup, spec model and `init`, `index`, `check` and the operation engine, `commit` and the demo, `complete`, and the craft record). Crate version on `main`: 0.3.7.
- **Slice 1 (the new-story path) is built and merged** (#38, branch `build/slice-1-opener`; version 0.4.0). The author approved the plan on October 25. It adds entry routing (`route`), the last-session marker (`session answer`, `session stop`), the `scene_untied` check, the `propose` command, the opener path in the skill, five interview evals and a script test that scans writer-facing text for internal words. Interface notes: `docs/entry.md`; what was built and which choices are still *Proposed*: `build-log.md`, Build progress, and `design-story.md`.
- **Planning session 2 (October 25, 2026, PR #39):** settled slice 2, the mid-thread recap, and the `demo` to `scenario` rename. Both are now built: see the next bullets. The decisions are in `design-story.md` ("Resuming with no input") and `delivery.md` (Testing and evals).
- **Scenario rename built, in review** (#40, branch `build/scenario-rename`; version 0.4.1 after merge; merge it first, slice 2 is built on top of it): `skaldsaga scenario <name>` replaces `demo` (the old name is gone, no alias); `basics` is the old demo. It is developer-only; to try it from the phone, run `/skaldsaga scenario basics` in an empty set-up folder. Notes: `docs/commit.md`; log: `build-log.md`, Build progress.
- **Slice 2 (the mid-thread recap) is built, in review** (#41, branch `build/slice-2-recap`; version 0.5.0 after merge). `/skaldsaga` with no text in a story left mid-thread gives a short summary and one question and changes nothing until the writer answers; the marker gains the topic last asked and a count for the save offer; marker writes never get a snapshot of their own; new `recap`, `session ask` and `session save`; the `resume` scenario. Interface notes: `docs/entry.md`; what was built and the *Proposed* choices to confirm: `build-log.md`, Build progress, and `design-story.md` (end of "Resuming with no input").
- **Security work built:** SR-10, SR-11, SR-20, SR-22, SR-24, SR-30 to SR-33, SR-40 to SR-45 (except provenance and signing), SR-04, SR-43, SR-52, SR-60, SR-61. Live status per requirement: `docs/security/status.md`.
- **Not built yet:** the stopping-point recap, stopping-point detection and the natural-pause offer, the drift guard and parking, the brainstorm path, the orchestrator and the sub-agent prompts, and the entries for stages and landmarks in the craft record.
- **Author actions still open** (details in `docs/handoff/build-log.md` and `docs/security/github-settings-checklist.md`): review and merge #40, then the slice 2 pull request; then start the Release workflow, sync the marketplace in the testing Project (Project plugin settings > Manage Marketplaces > Sync), start a fresh session, run `/skaldsaga scenario resume` in an empty set-up folder, then start another fresh session and try `/skaldsaga` with nothing after it. Also still open: activate the "Main protect" ruleset (SR-50), GitHub email privacy (SR-02), the other GitHub settings in the checklist, delete merged branches from a computer (or turn on automatic deletion of head branches), and the meter gate test in local Claude Code on a real machine.
- **Open from planning session 1:** when the fuller at-a-glance picture of the story appears (item 20), and the *Proposed* slice 1 choices listed in `design-story.md` (opener for text in an empty story, content with no marker reads as mid-thread, the scratch file `.harness/draft.txt`, the marker committed, the demo now `flagged` and kept with `keep_anyway`, the damaged-folder default).

## Ready to build

Nothing is ready to build. The scenario rename (A) and slice 2 (B) are built and in review (see Status). The author's open question for B is the list of *Proposed* choices at the end of "Resuming with no input" in `design-story.md`; confirm or change them, then plan the next slice.

**Later candidates** (to be planned with the author first):

1. **Stopping point:** the stopping-point recap, detection, the offer, the natural-pause save offer, and the drift guard with parking (slice 3).
2. **Brainstorm path** and its holding area. Reassess how loose ideas (like "maybe a dark alley") are kept: slice 2 keeps them only in the marker and recap (the recap offers one as a way to answer; the loose idea is replaced when the writer's next answer is recorded).
3. **At-a-glance story picture** (open item 20).
4. **Orchestrator and sub-agents** (planning agenda items 2 and 4 below).

## Working mode from here (Decided October 24, 2026)

**Planning sessions come before the next build steps.** Steps 1 to 10 (the deterministic half of phase 1, the security work and the craft record) are built. What remains is the interviewer, the orchestrator and the sub-agent prompts, which turn on choices only the author can make. So the next sessions are **wayfinder-style planning sessions** (the method in Research notes and open item 9): the destination is named first, one short question at a time with a sentence of why, each answer shows what to ask next, and decisions are written into the reference files as they are made. Build steps resume once the answers are enough to build without asking further questions; a build session should not start on this work before then.

Agenda for the planning sessions, in a suggested order:

1. **Interviewer:** voice and tone, how a teaching aside sounds, how the one-sentence why reads, how the starting experience check is asked, when to offer research (uses the `craft` commands from Step 10).
2. **Orchestrator:** what it does at session start and between questions, how it hands work to sub-agents and takes it back, how the writer chooses what to work on.
3. **Open item 20:** the story map in the session-start recap (first question already posed: map plus one recommendation, map then open choice, or map only on request or at milestones).
4. **Sub-agents:** which are phase 1, what each may read and write, and the agent-side security rules (SR-30, SR-31, SR-52).
5. **Triage the older Proposed bullets** (listed under "Next session" in the old Current stage; it is in the build log).

Each planning session ends with the decisions written into the reference files and a short list of what is now ready to build.

## File map

All files are in `docs/handoff/`.

| File | What is in it | Load it when |
|---|---|---|
| `design-story.md` | The project, name, audience and interaction principles, story-shape layer (the 12 shape decisions), completeness bar, story model, coverage grid and movements, weaver, outliner and frameworks | Planning the interviewer, outliner or weaver |
| `design-system.md` | Architecture, governance, spec and storage (including editions), node format, and who catches what (checks) | Anything that reads or writes the spec |
| `delivery.md` | Build, release and delivery, default settings, testing and evals | Pipeline, settings or test work |
| `security.md` | Principles and the SR table from the October 20 review | A pull request that touches security |
| `open-items.md` | Open and closed items 1 to 20, including craft teaching (19) and the story map (20) | Checking what is still undecided |
| `future-phases.md` | Recorded ideas that are not in scope | Only when someone asks about later phases |
| `research-notes.md` | Research findings to verify before relying on them | Before relying on an outside fact |
| `../entry.md` (in `docs/`) | Interface notes for `route`, `session`, `propose` and the `scene_untied` check | Touching entry routing, the marker or the opener |
| `build-log.md` | Old Current stage, Step 9 planning, build progress for every step and fix, and the "What changed" summaries for v6 to v9 | Looking for the history behind a decision or a step |

## Lookup: section to file

| Section (as named in source comments and older notes) | File |
|---|---|
| The project; Name; Audience and interaction principles; Story-shape layer; Completeness bar | `design-story.md` |
| Story model and coherence checking; Coverage grid and movements; Weaver, outliner and frameworks | `design-story.md` |
| Architecture; Governance: how changes reach the spec; Spec and storage; Editions | `design-system.md` |
| Node format (every node type, Fact entry, Relationship vocabulary, Scene, Shape decision, Thread, Viewpoint and style, Arc, Proposal, Backlinks index) | `design-system.md` |
| Checks: who catches what | `design-system.md` |
| Build, release and delivery; Default settings; Testing and evals | `delivery.md` |
| Security requirements | `security.md` |
| Open items (1 to 20) | `open-items.md` |
| Future phases; Research notes | `future-phases.md`; `research-notes.md` |
| Current stage; Step 9 planning; Build progress (Steps 1 to 10 and every fix); What changed since v8, in v8, v7 and v6; Suggested next steps (old) | `build-log.md` |
