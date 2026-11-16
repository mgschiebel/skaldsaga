# Skaldsaga: Architecture Sketch (reviewed)

*October 18, 2026. Companion to `HANDOFF.md` (v8), which holds the reasoning and full schemas. Everything here reflects Decided items unless marked* **Proposed***. The project name Skaldsaga and the entry-point skill name `skaldsaga` are Decided; other names of skills, scripts and files are working names. Internal role names (orchestrator, interviewer, housekeeper and so on) are never shown to the writer.*

> **Update, November 16, 2026 (planning session 10).** An architecture review changed parts of this sketch: only the Interviewer talks to the writer; every other agent runs in the background; an agent is a skill plus a primitive, where the primitive is a subcommand group of the one binary; the old orchestrator is split up; and agents check each other in designed pairs. The review is in [`docs/handoff/design-agents.md`](docs/handoff/design-agents.md), with the grid of who checks whom in [`docs/handoff/agent-grid.md`](docs/handoff/agent-grid.md). Its Decided choices replace what is written below; its Proposed ones do not yet. Nothing described here or there is built beyond one skill plus the program.

## Components

### Main thread (talks to the writer)

| Component | Kind | Job |
|---|---|---|
| **orchestrator** (skill name: `skaldsaga`) | skill (entry point; the writer types `/skaldsaga`) | First-run setup; session open (recap from the last session file, start an incremental housekeeper pass, suggest one next step plus "something else"); chooses the topic; dispatches sub-agents; session wrap-up. Reads only `spec/_index/` and `.harness/`, never whole nodes. |
| **interviewer** | skill | The live question loop for one topic. Socratic, infer-and-confirm, plain language. Reads spec nodes plus the pending inbox. Writes one proposal per confirmed answer. Raises flags at natural pauses. Checks the meter after each saved answer. |

### Sub-agents (no writer contact; short summary back)

Each sub-agent's model is a tunable default in `.harness/settings.json`: housekeeper, completeness rubric, weaver and outliner start on the strong model; the harvester and researcher on mid-tier.

| Sub-agent | Job |
|---|---|
| **housekeeper** | Incremental pass at session start (changes since the last cleared pass); full pass (gate before weave and outline, on request) or scoped pass (touched nodes and facts plus their readers; rolling every N answers; immediately after a confirmed-decision change). Reads cleared state only. Never edits; returns flags. |
| **harvester** | Reads drafts in `manuscript/`, proposes spec updates for new facts, flags drift against declared scene reads. |
| **weaver** | Chooses a framework with the writer (via hand-back: returns questions, the interviewer asks them, the orchestrator re-dispatches), resolves fog subplots (map, shelve or fold in), then interleaves every thread's stages, landmarks and beats into the skeleton: ordered slot scenes in the sequence across the four movements (Setup, Complications, Crisis, Resolution), built around pinned scenes. No separate woven plan. May surface a new fog arc. |
| **outliner** | Deepens the skeleton one movement per dispatch: beats, POV, location, reads and the scene turn. Chapters and other groupings belong to editions, which are not built in phase 1 (HANDOFF open item 14). |
| **researcher** | Looks up real-world facts the story depends on, when the writer asks or accepts an offer; files provisional proposals with sources. Mid-tier model. |
| **completeness-rubric** | The deep, model-judged half of the completeness check. |

### Scripts (deterministic; written in Rust, compiled binaries ship with the plugin)

**Decided (v9):** one binary with these as subcommands, built on a core library that the later standalone app will reuse (HANDOFF open items 13 and 18).

| Script | Job |
|---|---|
| `init` | First-run scaffold: four visible folders with one-line READMEs, `.harness/`, default settings, status line wiring. |
| `check` | Dependency checker: validity, routing (fan-out, confirmed-decision touch, lane), hard ordering and uniqueness checks (cardinality-aware). |
| `commit` | The single committer: applies one proposal, version check, bumps node and fact revisions, then calls `index`. |
| `index` | Rebuilds everything in `spec/_index/`. |
| `meter` | Status line script writes context usage to `.harness/meter.json`; a helper reads it against the thresholds. |
| `complete` | Script half of the completeness check: shape decisions answered, grid cells, setup-and-payoff ledger, timeline and knowledge ordering. |

### Shared primitives (reference files and helpers loaded by skills and sub-agents)

Plain-language glossary (our names for every internal concept), consistency and impact lookup (via the backlinks index), spec read with pending-inbox overlay, proposal writer, harvest rules. (The length guide moved to a later phase, with editions.)

## Folder layout

```
my-novel/
  spec/
    people/       p-0001.md ...
    places/       l-0001.md ...
    scenes/       s-0001.md ...
    decisions/    d-0001.md ...
    threads/      t-0001.md ...
    arcs/         a-0001.md ... (main story, active and fog subplots)
    ideas/        idea-xxxxxxxx.md ... (held ideas: brainstormed ideas and parked tangents)
    sequence.md   scene IDs in reading order (the story itself)
    attributes.md the story's own attribute keys (the starter list is built in)
    editions/     groupings over the sequence: chapters, acts, serial updates (Proposed)
    style.md      viewpoint and style node
    _index/       backlinks, attributes, grid, scenes, status (generated; ignored by git)
  manuscript/     prose drafts and what-if variants
  reviews/        later phases
  style-guide/    later phases
  .harness/
    inbox/        pending proposals, one JSON each
    settings.json thresholds, rolling-pass interval, reader cap
    meter.json    written by the status line
    sessions/     one recap per session
```

Distributed as a Claude Code plugin named `skaldsaga`, bundling the skills, agent definitions and scripts. **Proposed.** A plugin cannot register the main status line, so `init` writes the `statusLine` entry into the project's `.claude/settings.json` (one permission prompt). No script runtime is guaranteed on the writer's machine, so the scripts are written in Rust and ship as compiled binaries for each operating system (Decided in HANDOFF v8). Binaries cannot sit in a top-level `bin/` folder, which blocks installation through claude.ai and Cowork. How they reach the writer is HANDOFF open item 13.

## Call flows

### Session

1. First time: the writer types `/skaldsaga`, and `init` runs (fresh or existing-project path; see HANDOFF Architecture), with a plain explanation before the status line permission prompt. Later sessions: the plugin's SessionStart hook sees `.harness/` and has Claude open with the orchestrator's recap (also after `/clear`).
2. Orchestrator reads the latest `.harness/sessions/` recap and `_index/status.json`, starts an incremental housekeeper pass in the background, and gives the recap plus one suggested topic.
3. Writer picks a topic. The interviewer runs the loop (next flow).
4. Wrap-up when the topic ends or the meter says so: finish the question, save, run the end-of-topic scoped pass over the last few answers, write the session recap, tell the writer to clear the chat.

### One answer

```
writer answers -> interviewer confirms back
  -> proposal JSON into .harness/inbox/
  -> check (instant)
       hard failure      -> held; explain; revise only
       optimistic lane   -> commit as unreviewed -> index
       strict lane       -> held until a housekeeper scoped pass clears it -> commit -> index
  -> every N answers: background scoped housekeeper pass over unreviewed facts
       clean   -> facts marked cleared
       flagged -> interviewer raises at next pause: revise / keep anyway / not sure yet
  -> meter check: first threshold suggests wrapping up, second wraps up
```

A change to a confirmed decision is caught by the interviewer's consistency check, confirmed live, escalated to the strict lane, and starts a scoped pass immediately. The interviewer steers to unrelated topics until it returns.

### Drafts

Writer drops prose in `manuscript/` -> orchestrator dispatches the harvester -> proposals go into the inbox like any answer -> the interviewer walks the writer through approving them.

### Toward the outline

1. Orchestrator runs `complete` and, for depth, the completeness rubric. Gaps become frontier questions for the interviewer.
2. Full housekeeper pass as a gate.
3. Weaver (with the writer's framework choice) resolves fog subplots and plans the whole book; any new arc goes in as fog.
4. Full housekeeper pass as a gate, then the weaver writes the skeleton and the outliner deepens it one movement at a time, all as proposals.
5. A rough outline with gaps marked is allowed at any point; "complete" requires every shape decision answered at least provisionally, no empty grid cells, and clean plot-hole checks.

## Long-term shape (later phases)

CI makes the story: proposals, checks and the single committer, through outline, drafting and editing. It produces a pinned, git-tagged version. CD runs one pipeline per edition (Kindle, print, web serial) with format-specific agents and checks, building from that pinned version. CD can block and file proposals back into the inbox, but never edits canon. See HANDOFF Future phases.

## Open in this sketch

- Editions are out of phase 1 (v9); the format is recorded for later (HANDOFF open item 14).
- Plugin packaging and status line registration (open item 10).
- Binary delivery and the release pipeline are settled in HANDOFF (Build, release and delivery); a few install behaviors still need verifying.
