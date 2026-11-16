# Design: agents (architecture review)

*Planning session 10, November 16, 2026 (branch `plan/agent-architecture`; doc only). Part of the Skaldsaga handoff; the index is [`HANDOFF.md`](../../HANDOFF.md). Load it for any work that splits `SKILL.md`, adds an agent, or changes how agents check each other. The October 18 sketch in [`ARCHITECTURE.md`](../../ARCHITECTURE.md) stays as the record of the first plan; where the two disagree, this file is newer, but only its **Decided** choices replace the sketch, and its **Proposed** ones do not yet. The grid of who checks whom is in [`agent-grid.md`](agent-grid.md).*

*Nothing in this file is built. What exists is one skill (`plugin/skills/skaldsaga/SKILL.md`, about 480 lines, doing the orchestrator's and the interviewer's jobs) plus the Rust program. Labels as in `HANDOFF.md`: **Decided** means the author confirmed it in this session; **Proposed** means Claude suggested it and the author has not yet confirmed it. Agent names here (Argument, Cast and so on) are internal and are never shown to the writer.*

## Why this review

The builds so far taught four things: the single skill does two jobs; it grows with every feature and every feature adds string-pinned tests; every new thing that writes needed a small program change; and the craft record (Step 10) was built but the skill never calls it. The author named three gaps (*Decided*):

1. The learning ladder is not worked into the overall flow.
2. The interrelationships between the agents are not tied together, and one large skill should be decomposed into individual agents.
3. Primitives and agents are muddled and need to be decomposed and realigned.

## Layers (Decided)

1. **System.** A set of agents, each with one concern, working in an adversarial pattern: every agent's output is checked or challenged by another, and none grades its own work.
2. **Agent.** A skill plus a primitive. The skill makes the decisions that are not deterministic and need intelligence. The primitive supplies the deterministic flow and the guardrails that keep the agent inside its own concern and out of other agents' concerns.
3. **Primitive.** A subcommand group of the one Rust binary (the single binary was decided in v9), not a separate binary per agent.

**Terms.** "Agent" means the skill-and-primitive pair. "Sub-agent" is no longer its own term; it is an agent running in the background (*Proposed*; the author left the decomposition of sub-agents open and this was the suggestion). An agent runs in one of two modes: **foreground** (human in the loop: talks to the writer) or **background** (AFK: a narrow brief, a fresh context, no writer contact, a short summary or flags back).

**Only the Interviewer is foreground (Decided).** It is the one agent focused on natural communication, questions and answers. Every other agent runs in the background and returns work to it, which gives one voice, one place for the one-short-question and confirmation rules, and one agent that ever sees the writer's text. A fresh context is also what makes an adversarial check real: a checker that has not seen the producer's reasoning cannot rubber-stamp it.

## Roster (Proposed: 23 candidates, merge pass still to do)

The old orchestrator is decomposed into the Planner, the Gatekeeper and the Session keeper, with the Interviewer speaking for all three. The old housekeeper is Continuity (plus the program's `check`). The old completeness rubric is Completeness.

| Agent | Mode | Single concern |
|---|---|---|
| Interviewer | foreground | Converse with the writer: voice, one question at a time, confirm back, ask what is queued |
| **Watchers** (check one area and return gaps as questions) | | |
| Argument | background | What the story is about: the two big questions and every side of them |
| Character | background | Individuals: inner life, how they change, relationships |
| Cast | background | Story roles: which are needed, filled, redundant or shifting (name *Decided*) |
| Plot | background | Threads across the four stages, stakes, setup and payoff |
| Genre | background | The promises made to the reader, and the limits they put on plot and roles |
| Sequence | background | Order and rhythm: reveals, pacing, when each change lands |
| Setting | background | Where and when, and the rules of that world |
| Scene | background | Each scene earns its place and has a turn |
| Viewpoint | background | The method of telling (person, distance) and which viewpoint character owns each scene |
| Voice | background | A recognizable voice profile per viewpoint character (profiles only; see below) |
| Learning | background | The writer's craft knowledge, and nudges to the Interviewer |
| Continuity | background | Contradictions, who-knows-what, time order |
| **Checkers** | | |
| Completeness | background | Is the story done? Adversary to the Gatekeeper (split from the Auditor, *Decided*) |
| Auditor | background | Did each agent stay inside its concern and can it back its claims (*Decided*) |
| **Producers** | | |
| Harvester | background | Facts from the writer's drafts |
| Researcher | background | Real-world facts, verified |
| Brainstormer | background | Ideas for the writer to react to |
| Weaver | background | Build the skeleton: the order of everything |
| Outliner | background | Deepen the skeleton one movement at a time |
| **Coordinators** | | |
| Planner | background | Rank the question queue by gates and unblocking value |
| Gatekeeper | background | Decide when the weave and outline gates open |
| Session keeper | background | Compose the recap, save offers and stopping point for the Interviewer to say |

Notes:

- The Setting and Scene agents and the Voice and Viewpoint split came out of discussion and are *Proposed*; the author wanted Setting, Scenes and similar craft points considered as agents of their own.
- **Voice produces profiles only (Decided).** Phase 1 ends at an outline with no prose, so Voice records how each viewpoint character thinks and talks (rhythm, word choice, what they notice) and writes no sample text. Voice samples and the style guide stay in a later phase (`future-phases.md`). The existing viewpoint and style node (`design-system.md`, "Viewpoint and style") already holds person, tense, distance and tone with per-character overrides.
- Genre is the adversary to Plot because genres follow different structures; Genre also pushes on Argument (some genres answer the thematic question at a typical cost), Setting and Voice.
- Sequence is the order the reader meets things in. A deadline can act as a character (a ticking clock) and a place can act as one (see Embodiments).

**The merge test (Proposed).** An agent earns its own place if it has all four of: a single concern, an adversary, obligations to or from other agents, and data of its own. An agent that fails two or more is folded into a neighbor or becomes primitive-only. The Planner, the Gatekeeper and the Session keeper are the likeliest to become primitive-only, since the program already decides the recap's topic, the counts and the meter. The merge pass has not been done.

## Phases and gates (Proposed, from the story design files)

From idea to an outline ready to be written: (1) opening, (2) gathering the story, (3) shape, the argument's decisions, (4) coverage of every thread at every stage, (5) gap check, (6) weaving, (7) deepening, (8) complete.

- **Hard gates:** both big questions answered before weaving; a full consistency pass before weaving and again before deepening; the skeleton before any deepening; movements deepened in order, starting with setup.
- **Soft:** phases 2 to 4 are one continuous loop of drawing the story out, not a sequence. Layers set the work order, not a depth limit: a detailed climax raised early becomes a pinned scene and the skeleton is built around it (`design-story.md`, "Weaver, outliner and frameworks").

So the flow is a set of gates, not a pipeline.

## How the agents are wired

- **One question queue (Proposed; the author agreed to the flow, ownership of the ranking not explicitly confirmed).** Every background agent returns questions for the writer into a shared queue, like the existing inbox. The Planner ranks it, and the Interviewer takes the top one and asks it in its own words. This replaces special hand-backs, such as the Weaver's framework choice, with one mechanism.
- **Timing.** Background agents cannot answer mid-conversation, so they run after each save and at session start, and the Interviewer reads results that are already waiting.
- **Obligations (Proposed).** Each agent publishes obligations: things it needs another agent's area to satisfy. The primitive tracks them deterministically, much as the coverage grid does today; the skill judges quality (does a character really embody a side, or only mention it?).
- **Embodiments (Decided).** A side of the argument may be embodied by something other than a person: a character, a constraint such as a deadline (Sequence acting as a character) or a place (Setting acting as a character). Whether each side becomes its own saved record that characters and events link to is open; Claude leans yes, so the program can see which sides are unembodied or untested. The current design has two sides only (the main character and the challenger); the author's picture has several faces of the argument, such as foils who make the opposite choice. This is a design gap to close when Argument is built.
- **Roles change over time (Decided).** A challenger can become an ally. Cast and Sequence interact: Cast owns what the roles are and whether a shift is coherent, Sequence owns when it lands and whether it is earned, Character owns whether the person's change justifies it, Argument re-checks who embodies a side, and Continuity checks the time order. The data model looks able to carry a role as a time-bounded fact, the way `strength` and `flaw` change with `ends`; to be confirmed when we reach the data model.

## The learning ladder (changes part of open item 19)

- **No teaching sessions, ever (Decided).** Teaching is by doing, inside the Interviewer's normal conversation. There is no class mode and no teaching-style session. The Learning agent only sends nudges about what to focus on.
- **Option C (the author's lean; Proposed).** Domain agents tag the concepts a question depends on (a code from the concept list) and do not teach. The craft record primitive holds the ladder rules (a term is never used before it is taught; a term shortens only once earned; teaching stops once the writer uses the term unprompted) and flags a question that uses an untaught term before it reaches the writer. A thin Learning agent does the judgment a program cannot: how much teaching a question carries, whether a concept is demonstrated in the writer's own story, whether a misconception needs recording. The other options were Learning as the only teacher with no tagging by others, and each agent carrying its own teaching; both were set aside because what the writer has been taught is shared state.
- **Topic list (gap).** The craft record tracks untaught, taught and demonstrated per concept, with story evidence, but there is no defined list of topics the writer is measured against. The list is grouped by the same areas as the agents. Each topic has a writer-facing name, a plain explanation and its prerequisites ("main character" before "MC").
- **The path is a forecast, not a syllabus.** It shows what the story is about to need and is one input to the Planner's ranking. The story drives the teaching; the teaching never drives the story.
- **A writer command shows the topics and where the writer stands, on demand only (Decided).** The name is not chosen. When it is built, the README adds it, since the README lists every command a writer types. Light gamification only (*Proposed*): celebrate what the writer demonstrated in their own story; no points, streaks or quizzes; a topic the writer opted out of under "keep it simple" is never a gap.
- **Amendment to the October 19 decision (open item 19):** teaching stays invisible by default, so "no formal output" now has one exception, the progress list shown when the writer asks. "Teach through primitives, not a separate agent" becomes the split above: the primitive holds the rules and a thin Learning agent annotates questions.

## The adversarial contract (Decided as a direction; details Proposed)

Agents are goal-seeking and may try to reach a goal by going around a guardrail. So each agent has explicit goals, and an honest failure is a finished job.

Each agent has three things:

1. **Pass:** the condition for success, with evidence the Auditor can check.
2. **Sanctioned fails:** a closed list of named reason codes, each a legitimate end state, such as "gap open and reported" or "the writer acknowledged and declined".
3. **Forbidden paths:** the shortcuts to a pass that are out of bounds.

The primitive enforces the closed list: an agent records a fail only with a code from its own list, and a pass only with evidence references the program can verify. The sanctioned fails map onto states the data model already has (a provisional answer, an open assumption, `keep_anyway`, `not_explored`, a shelved arc, a held or set-aside idea). The agent's goal is not "close every gap" but "bring every item in my concern to a sanctioned end state".

**Example, Researcher.** Pass: every requested fact has a source the Auditor can verify. Sanctioned fails: no reliable source found (the fact stays unverified); sources conflict (both listed); the question is really a story choice (handed to the Interviewer); the writer declined research. Forbidden: citing a source it did not read, marking a fact verified without a source, choosing between conflicting sources on its own.

### Opposing goals in a pair (Proposed; the author's idea)

In each two-way relationship the agents' goals oppose, so that one agent's pass is the other's sanctioned fail, and a pair resolves with one passing and the other failing in an accepted way. A pair is written in one template: the contested item; each side's pass; each side's sanctioned fails; the outcomes (A: one side wins, B: the other wins, C: the two are compatible and both pass, and any other resolution); the forbidden paths; the tiebreaker type; and the evidence the loser cites (usually the recorded decision that overruled it).

Rules around the pairs:

- **Deferring is not free.** An unresolved item stays open and keeps the gates closed, so a stalemate has a cost.
- **Both-pass fudging is checked.** The program confirms the two agents did not record contradictory outcomes for one contested item.
- **Who decides.** A story-choice conflict goes to the writer through the Interviewer. A factual conflict follows a precedence order: confirmed decisions, then Continuity's hard checks, then the watchers' judgment calls. A process or scope dispute is settled by the program's rules, and if they cannot settle it the Gatekeeper keeps the gate closed.
- **Unmapped pairs.** Until a pair is mapped, neither agent may settle a dispute on its own; the result is a sanctioned open gap in the question queue.

### The Auditor and Completeness

- **Completeness** checks the story ("is it done?") and is the Gatekeeper's adversary. It was dropped by mistake during the session and restored.
- **Auditor** checks the other agents: their evidence and whether they stayed inside their concern. Scope is mostly mechanical: each primitive limits what kinds of output its agent may produce, and the program enforces that. The Auditor judges only content that drifts into another agent's area while staying inside its own output format. It does not audit everything every time: it runs at gates, on high-risk outputs (Researcher, Gatekeeper, Planner) and every N answers, and it works in a fresh context.
- **Who checks the Auditor (Decided in direction):** the agents it audits. An audited agent may accept, rebut once with evidence (the program verifies that the referenced nodes exist), or raise the point to the Interviewer as a story question. After one rebuttal the dispute stops: a story question goes to the writer, a process dispute follows the program's rules (*Proposed*). The program also counts outcomes per agent: if an agent's rebuttals are always upheld the audits are miscalibrated, and if they are never upheld the agent is rubber-stamping or the Auditor is too lenient. Seeded-fault evals (as `delivery.md` plans for plot holes) check the Auditor's own accuracy.

## Pilot pairs (Proposed; the template is the point)

**1. Argument and Plot.** Contested item: every side of the argument tested by events with its cost shown, versus every thread earning its place without padding. Argument passes when each side is tested and its cost shown; Plot passes when the stages are filled and nothing is padded. Outcome A (Argument wins): Plot adds or revises an event and fails correctly with `thread_extended_for_side`. Outcome B (Plot wins): the side stays untested and the writer knows; Argument fails correctly with `side_untested_acknowledged`. Outcome C: an existing event already tests the side, so both pass. Other resolution: a thread that bears on no side is folded in, shelved, or the argument is revised. Forbidden: Plot tagging an event as testing a side when no such link exists; Argument inventing events. Tiebreaker: the writer.

**2. Genre and Plot.** Contested item: whether the threads deliver the genre's promise. Outcome A (Plot wins): the writer subverts the genre on purpose; Genre fails with `promise_subverted_by_writer`. Outcome B (Genre wins): threads are revised; Plot fails with `revised_to_convention`. Outcome C: both pass. Forbidden: Plot relabeling the story's genre to dodge the check; Genre rewriting threads. Tiebreaker: the writer.

**3. Weaver and Outliner.** Contested item: whether the skeleton can carry the deepening, or the skeleton changes. Weaver passes when every cell, landmark and pinned scene has an ordered slot; Outliner passes when every slot gets beats, viewpoint, location and a turn. Outcome A (Weaver wins): the slot stands and Outliner fails with `slot_unworkable` and the reason. Outcome B (Outliner wins): the slot is revised and Weaver fails with `slot_revised_on_deepening`; Continuity rechecks the ripple. Forbidden: Outliner reordering slots (only Weaver changes order); Weaver deleting a slot to avoid a failed deepen; marking a slot deepened with empty beats. Tiebreaker: a process dispute, so the program's rules decide; if the revision touches a pinned scene or a confirmed decision it goes to the writer.

What the pilot showed about the template: some pairs have a third resolution (restructure); Weaver and Outliner run in sequence, not in parallel; and Genre and Plot carry a gaming risk (relabeling to dodge a check).

## Mapping and build order

- **Mapped gradually (Decided).** Each build works on one agent and its interface with the agents that already exist, and the map fills in as builds proceed. Long-term every pair is mapped. Pairs between agents that do not exist yet are not specified ahead of time, so we never write contracts for speculative neighbors.
- **Proposed first build.** Foundations plus a first pair, not a lone agent, since the adversarial value only appears with two: the agent contract enforced by the binary (pass, sanctioned fails, forbidden paths), the shared question queue, and splitting the Interviewer out of `SKILL.md` into the skill and its shared references. First pair: the Interviewer with Learning (the craft record already exists and it wires up the ladder), or the Interviewer with Continuity (`check` already exists). It needs its own plan and the author's approval before it is built.
- **The grid** (`agent-grid.md`) holds the whole map, with each pair marked mapped or to map. Earlier contracts may be revised when a later pair needs it; the pair's entry records why.

## Gaps the grid showed (Proposed to act on)

From the first drawing of the grid (21 agents, before Completeness and the Auditor were added): nobody challenged Continuity or the Gatekeeper; Completeness was missing; nobody checked the Researcher's sources; the Planner was challenged only by the Interviewer, though it decides what the writer is asked; tone has no owner; the Weaver and Outliner are challenged by more than ten agents; and Voice, Researcher and Session keeper are barely connected. Completeness and the Auditor close the first four. The rest are open questions below. A fix for the over-review is to have the primitive check the obligations deterministically and escalate only the ambiguous ones to a model.

## Open questions

1. **Sequence and Weaver:** does Sequence stay a watcher that challenges the Weaver's order, or merge with it? The author said Sequence ripples across everything but did not choose.
2. **Tone** sits between Genre, Viewpoint and Voice with no owner.
3. **Setting and Scene** as agents: suggested, not confirmed.
4. **Argument sides as saved records** and several faces of the argument (see Embodiments).
5. **How a background agent's primitive is called.** Sub-agents get no shell (SR-30 to SR-33) while a primitive is a command of the binary. The parent may have to run it on the agent's behalf, or an exact allowed form may be pinned. This needs a design before any agent is built.
6. **The question queue:** where it is stored, what the Planner ranks on, and who may add to it.
7. **The merge pass** with the four-part test, and which coordinators become primitive-only.
8. **The learning topic list:** the topics, their writer-facing names and their prerequisites.
9. **Reader's view:** what the reader knows versus what characters know is folded into Sequence for now; it may need to be its own concern.
10. **The digest of Proposed choices** (Project docs, `proposed-and-open-choices-digest.md`) does not yet include this session's choices; planning agenda item 5 should add them.
