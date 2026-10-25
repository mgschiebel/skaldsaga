# Design: architecture, spec and checks

*Architecture, governance, spec and storage, node format and the checks. Load it for anything that reads or writes the spec. Part of the Skaldsaga handoff; the index is [`HANDOFF.md`](../../HANDOFF.md). Moved verbatim from HANDOFF.md (v9, last edited October 24, 2026) on October 24, 2026: the wording below is unchanged.*

<!-- begin moved text -->

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


