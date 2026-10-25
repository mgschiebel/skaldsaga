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

## Status (last updated October 24, 2026, evening)

- **Phase 1, deterministic half: built.** Steps 1 to 10 are done (meter pilot, serde, plugin, cloud setup, spec model and `init`, `index`, `check` and the operation engine, `commit` and the demo, `complete`, and the craft record). Crate version on `main`: 0.3.7.
- **Security work built:** SR-10, SR-11, SR-20, SR-22, SR-24, SR-30 to SR-33, SR-40 to SR-45 (except provenance and signing), SR-04, SR-43, SR-52, SR-60, SR-61. Live status per requirement: `docs/security/status.md`.
- **Not built yet:** the interviewer, the orchestrator and the sub-agent prompts, and the entries for stages and landmarks in the craft record. These turn on choices only the author can make, so planning comes first (see Next).
- **Author actions still open** (details in `docs/handoff/build-log.md` and `docs/security/github-settings-checklist.md`): activate the "Main protect" ruleset (SR-50), GitHub email privacy (SR-02), the other GitHub settings in the checklist, delete merged branches from a computer (or turn on automatic deletion of head branches), and the meter gate test in local Claude Code on a real machine.
- **Planning session 1 (interviewer) started October 24.** The first question was posed and is not yet answered: who the interviewer sounds like when it asks a story question (a warm writing coach, a curious first reader, or a quiet neutral guide). Claude's lean: the curious reader, with craft terms arriving only when a question needs them.

## Ready to build

Nothing. Build steps resume once the planning answers are enough to build without asking further questions (see Working mode).

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
