# Agent grid (Proposed)

*Planning session 10, November 16, 2026. Part of the Skaldsaga handoff; the index is [`HANDOFF.md`](../../HANDOFF.md). The roster and the reasons are in [`design-agents.md`](design-agents.md). Nothing here is built, and every relationship is Proposed: Claude's reading of how the agents would check each other, for the author to confirm or change. Internal names are never shown to the writer.*

**The pair list is the source.** The grid is derived from it. Edit the list, then redraw the grid by hand until a later build adds a generator. A pair is **mapped** once its contract (the contested item, each side's pass and sanctioned fails, forbidden paths and tiebreaker, as in `design-agents.md`) is written; the rest are **to map**, and they fill in as builds touch them. Until a pair is mapped, neither agent may settle a dispute on its own: the result is an open gap in the question queue.

Counts: 169 of 253 pairs have a relationship; 3 are mapped (the three pilots) and 1 has a first-build contract (Interviewer and Continuity); 165 are to map; 84 have no direct relationship drawn so far (to confirm as agents are built).

## How to read a cell

The cell is the **row** agent's relationship to the **column** agent. `X` they challenge each other; `>` the row agent challenges the column agent; `<` the column agent challenges the row agent; `f` the row agent feeds the column agent without challenging it; `g` the column agent feeds the row agent; blank means no direct relationship drawn.

## Agents (codes)

`INT` Interviewer, `ARG` Argument, `CHR` Character, `CST` Cast, `PLT` Plot, `GEN` Genre, `SEQ` Sequence, `SET` Setting, `SCN` Scene, `VPT` Viewpoint, `VOI` Voice, `LRN` Learning, `CON` Continuity, `CMP` Completeness, `AUD` Auditor, `HAR` Harvester, `RES` Researcher, `BRA` Brainstormer, `WEA` Weaver, `OUT` Outliner, `PLN` Planner, `GAT` Gatekeeper, `SES` Session keeper

## Grid

| |INT|ARG|CHR|CST|PLT|GEN|SEQ|SET|SCN|VPT|VOI|LRN|CON|CMP|AUD|HAR|RES|BRA|WEA|OUT|PLN|GAT|SES|
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| **INT** |-|<|<|<|<|<|<|<|<|<|<|X|<|<|X|g|g|g|g|g|X||g|
| **ARG** |>|-|X|X|X|X|X|X|X|X||<|<|X|X|>||>|X|X|f|<||
| **CHR** |>|X|-|X|X|X|X|X|X|X|X|<|<|X|X|>||>|X|X|f|<||
| **CST** |>|X|X|-|X|X|X|X|X|||<|<|X|X|>||>|X||f|<||
| **PLT** |>|X|X|X|-|X|X|X|X|X||<|<|X|X|>|g|>|X|X|f|<||
| **GEN** |>|X|X|X|X|-|X|X|X|X|X|<||X|X|>||>|X|||||
| **SEQ** |>|X|X|X|X|X|-|X|X|X||<|<|X|X|>||>|X|X||||
| **SET** |>|X|X|X|X|X|X|-|X||||<|X|X|>|g|>||X||||
| **SCN** |>|X|X|X|X|X|X|X|-|X||<|<|X|X|>||>|X|X||<||
| **VPT** |>|X|X||X|X|X||X|-|X|<|<|X|X|>|||X|X||||
| **VOI** |>||X|||X||||X|-|||X|X|>||||||||
| **LRN** |X|>|>|>|>|>|>||>|>||-|||X||g|>|>|>|f||>|
| **CON** |>|>|>|>|>||>|>|>|>|||-||X|>|>|>|>|>|f|f||
| **CMP** |>|X|X|X|X|X|X|X|X|X|X|||-|X||||||f|>||
| **AUD** |X|X|X|X|X|X|X|X|X|X|X|X|X|X|-|X|X|X|X|X|X|X|X|
| **HAR** |f|<|<|<|<|<|<|<|<|<|<||<||X|-||||||||
| **RES** |f||||f|||f||||f|<||X||-|||||||
| **BRA** |f|<|<|<|<|<|<|<|<|||<|<||X|||-||||||
| **WEA** |f|X|X|X|X|X|X||X|X||<|<||X||||-|X||<||
| **OUT** |f|X|X||X||X|X|X|X||<|<||X||||X|-||<||
| **PLN** |X|g|g|g|g|||||||g|g|g|X||||||-|g|f|
| **GAT** ||>|>|>|>||||>||||g|<|X||||>|>|f|-||
| **SES** |f|||||||||||<|||X||||||g||-|

## Pair list

Each line is `Agent and Agent (kind) [status]: the contested item or reason`. Pilot pairs are mapped in `design-agents.md`, "Pilot pairs". The Auditor's pairs are one rule, listed once at the end.

### Interviewer
- **Interviewer and Argument** (Argument challenges Interviewer) [to map]: Finds sides of the question the interview hasn't drawn out; queues questions
- **Interviewer and Character** (Character challenges Interviewer) [to map]: Finds people left thin or stances unexplored; queues questions
- **Interviewer and Cast** (Cast challenges Interviewer) [to map]: Finds roles unfilled or redundant; queues questions
- **Interviewer and Plot** (Plot challenges Interviewer) [to map]: Finds threads empty at a stage; queues questions
- **Interviewer and Genre** (Genre challenges Interviewer) [to map]: Finds genre promises the story hasn't kept or knowingly broken
- **Interviewer and Sequence** (Sequence challenges Interviewer) [to map]: Finds pacing and reveal-order problems in what's saved
- **Interviewer and Setting** (Setting challenges Interviewer) [to map]: Finds places and time left vague
- **Interviewer and Scene** (Scene challenges Interviewer) [to map]: Finds scenes with no turn or no thread
- **Interviewer and Viewpoint** (Viewpoint challenges Interviewer) [to map]: Finds scenes with no viewpoint owner, or knowledge leaks
- **Interviewer and Voice** (Voice challenges Interviewer) [to map]: Finds viewpoint characters who would sound alike
- **Interviewer and Learning** (challenge each other) [to map]: Learning flags unearned terms in questions; the interviewer's logs show what the writer has demonstrated
- **Interviewer and Continuity** (Continuity challenges Interviewer) [mapped (first build contract, planning session 12; the adversary pairing itself stays Proposed)]: Flags contradictions in what the interviewer saved; the verdict contract is in `design-agents.md`, "The first build's plan"
- **Interviewer and Completeness** (Completeness challenges Interviewer) [to map]: Finds what the story still lacks as a whole; queues questions
- **Interviewer and Harvester** (Harvester feeds Interviewer) [to map]: Harvested facts reach the writer only through the interviewer
- **Interviewer and Researcher** (Researcher feeds Interviewer) [to map]: Sourced facts are offered to the writer by the interviewer
- **Interviewer and Brainstormer** (Brainstormer feeds Interviewer) [to map]: Ideas are offered one round at a time by the interviewer
- **Interviewer and Weaver** (Weaver feeds Interviewer) [to map]: Hand-back questions (framework, side plots) are asked by the interviewer
- **Interviewer and Outliner** (Outliner feeds Interviewer) [to map]: Questions raised while deepening are asked by the interviewer
- **Interviewer and Planner** (challenge each other) [to map]: Planner ranks what to ask; the writer's own direction can override the ranking
- **Interviewer and Session keeper** (Session keeper feeds Interviewer) [to map]: Recap, save offer and stopping point are worded by the interviewer

### Argument
- **Argument and Character** (challenge each other) [to map]: Every side needs someone embodying it / a character's stance matches no side
- **Argument and Cast** (challenge each other) [to map]: Every side needs a role filled / a role shift changes who embodies a side
- **Argument and Plot** (challenge each other) [mapped (pilot)]: Every side must be tested and its cost shown / a thread bears on no side
- **Argument and Genre** (challenge each other) [to map]: Genre expects a typical answer / the argument's answer breaks the promise
- **Argument and Sequence** (challenge each other) [to map]: Sides tested in an order that builds to the answer / a deadline can itself embody a side
- **Argument and Setting** (challenge each other) [to map]: A place can embody a side / the place's role matches no side
- **Argument and Scene** (challenge each other) [to map]: Each scene should bear on a side / a side no scene dramatizes
- **Argument and Viewpoint** (challenge each other) [to map]: Each side seen from some viewpoint, or deliberately unseen / viewpoints show only one side
- **Argument and Learning** (Learning challenges Argument) [to map]: Unearned terms in shape questions (thematic question, challenger)
- **Argument and Continuity** (Continuity challenges Argument) [to map]: The argument or its answer contradicts saved facts or the ending
- **Argument and Completeness** (challenge each other) [to map]: Completeness checks across areas for what no single watcher sees / the watcher reports its own area
- **Argument and Harvester** (Argument challenges Harvester) [to map]: Draft passages that argue something other than the story's argument
- **Argument and Brainstormer** (Argument challenges Brainstormer) [to map]: Ideas that serve no side of the question
- **Argument and Weaver** (challenge each other) [to map]: Skeleton must dramatize every side / the framework forces an answer shape
- **Argument and Outliner** (challenge each other) [to map]: Deepened beats keep every side visible / beats reveal a side never touched
- **Argument and Planner** (Argument feeds Planner) [to map]: Argument gaps decide which question is worth asking next
- **Argument and Gatekeeper** (Gatekeeper challenges Argument) [to map]: Both big questions must be answered before weaving

### Character
- **Character and Cast** (challenge each other) [to map]: A person changes and breaks their role / a role fits no person
- **Character and Plot** (challenge each other) [to map]: Events must press on the people / the person wouldn't act that way to move the plot
- **Character and Genre** (challenge each other) [to map]: Genre expects character types / the person is a cliche or doesn't fit
- **Character and Sequence** (challenge each other) [to map]: Change must be earned in order / the change needs pressure scenes first
- **Character and Setting** (challenge each other) [to map]: The place shapes the person / the person's history must fit the world
- **Character and Scene** (challenge each other) [to map]: Who is in the scene and what it costs them / the turn must land on someone
- **Character and Viewpoint** (challenge each other) [to map]: Would this person see the scene this way / the viewpoint shapes who we know
- **Character and Voice** (challenge each other) [to map]: Voice must fit who they are / similar characters sound alike
- **Character and Learning** (Learning challenges Character) [to map]: Craft terms (main character, challenger) used before taught
- **Character and Continuity** (Continuity challenges Character) [to map]: Contradictions in traits, history or what they know
- **Character and Completeness** (challenge each other) [to map]: Completeness checks across areas for what no single watcher sees / the watcher reports its own area
- **Character and Harvester** (Character challenges Harvester) [to map]: Draft shows someone acting out of character
- **Character and Brainstormer** (Character challenges Brainstormer) [to map]: Ideas that don't fit the people
- **Character and Weaver** (challenge each other) [to map]: Skeleton must carry each change / a change that can't be placed
- **Character and Outliner** (challenge each other) [to map]: Beats show the change in the person's choices / beats ask for out-of-character action
- **Character and Planner** (Character feeds Planner) [to map]: Character gaps feed the question ranking
- **Character and Gatekeeper** (Gatekeeper challenges Character) [to map]: Main character, challenger and relationship decisions answered before weaving

### Cast
- **Cast and Plot** (challenge each other) [to map]: Threads need roles filled / a role attached to no thread
- **Cast and Genre** (challenge each other) [to map]: Genre's expected roles missing or redundant / a role breaks the genre
- **Cast and Sequence** (challenge each other) [to map]: When each role shift lands / a shift not earned in order
- **Cast and Setting** (challenge each other) [to map]: A place or constraint can fill a role / a place role with no function
- **Cast and Scene** (challenge each other) [to map]: Roles shown in action / a role that never appears in any scene
- **Cast and Learning** (Learning challenges Cast) [to map]: Role terms (challenger, foil, antagonist) before taught
- **Cast and Continuity** (Continuity challenges Cast) [to map]: A role shift contradicts earlier scenes
- **Cast and Completeness** (challenge each other) [to map]: Completeness checks across areas for what no single watcher sees / the watcher reports its own area
- **Cast and Harvester** (Cast challenges Harvester) [to map]: Draft shifts a role without intent
- **Cast and Brainstormer** (Cast challenges Brainstormer) [to map]: Ideas that duplicate a role
- **Cast and Weaver** (challenge each other) [to map]: Skeleton must place role shifts / a shift that can't be woven
- **Cast and Planner** (Cast feeds Planner) [to map]: Cast gaps feed the question ranking
- **Cast and Gatekeeper** (Gatekeeper challenges Cast) [to map]: Roles the shape decisions need are filled before weaving

### Plot
- **Plot and Genre** (challenge each other) [mapped (pilot)]: Genre's expected structure and beats / threads that don't deliver them
- **Plot and Sequence** (challenge each other) [to map]: Stage events must build to the crisis / an order that undercuts the build
- **Plot and Setting** (challenge each other) [to map]: Events must be possible in this world / plot needs places that don't exist
- **Plot and Scene** (challenge each other) [to map]: Each scene serves a thread / a stage cell with no scene
- **Plot and Viewpoint** (challenge each other) [to map]: Can a viewpoint character witness this event / key events no viewpoint can see
- **Plot and Learning** (Learning challenges Plot) [to map]: Plot terms (crisis, dramatic question) before taught
- **Plot and Continuity** (Continuity challenges Plot) [to map]: Plot-hole checks: timeline and who-knows-what
- **Plot and Completeness** (challenge each other) [to map]: Completeness checks across areas for what no single watcher sees / the watcher reports its own area
- **Plot and Harvester** (Plot challenges Harvester) [to map]: Draft events that fit no thread
- **Plot and Researcher** (Researcher feeds Plot) [to map]: Real-world facts that events depend on
- **Plot and Brainstormer** (Plot challenges Brainstormer) [to map]: Ideas that break a thread (the ripple preview)
- **Plot and Weaver** (challenge each other) [to map]: Every cell needs a slot / a thread that can't be woven
- **Plot and Outliner** (challenge each other) [to map]: Beats cover the stage / beats that drift from the thread
- **Plot and Planner** (Plot feeds Planner) [to map]: Plot gaps feed the question ranking
- **Plot and Gatekeeper** (Gatekeeper challenges Plot) [to map]: Coverage grid full before complete

### Genre
- **Genre and Sequence** (challenge each other) [to map]: Beats in expected places / an order that serves the argument instead
- **Genre and Setting** (challenge each other) [to map]: Genre expects certain worlds and conventions / the world doesn't fit
- **Genre and Scene** (challenge each other) [to map]: Obligatory scenes the genre expects / scenes that serve nothing
- **Genre and Viewpoint** (challenge each other) [to map]: Genre norms for person and distance / the chosen method breaks them
- **Genre and Voice** (challenge each other) [to map]: The genre's tone register / a voice that clashes
- **Genre and Learning** (Learning challenges Genre) [to map]: Genre terms before taught
- **Genre and Completeness** (challenge each other) [to map]: Completeness checks across areas for what no single watcher sees / the watcher reports its own area
- **Genre and Harvester** (Genre challenges Harvester) [to map]: Draft drifts from the genre
- **Genre and Brainstormer** (Genre challenges Brainstormer) [to map]: Ideas that break the genre's promise
- **Genre and Weaver** (challenge each other) [to map]: Framework choice: the genre's structure versus the writer's threads

### Sequence
- **Sequence and Setting** (challenge each other) [to map]: Calendar, distance and seasons must fit the order / a deadline can be a force
- **Sequence and Scene** (challenge each other) [to map]: Rhythm of turns: too many alike, or a scene that makes the story sag
- **Sequence and Viewpoint** (challenge each other) [to map]: Rotation of viewpoint owners / long gaps with one voice
- **Sequence and Learning** (Learning challenges Sequence) [to map]: Pacing terms (midpoint, crisis) before taught
- **Sequence and Continuity** (Continuity challenges Sequence) [to map]: Story-time versus telling-order contradictions
- **Sequence and Completeness** (challenge each other) [to map]: Completeness checks across areas for what no single watcher sees / the watcher reports its own area
- **Sequence and Harvester** (Sequence challenges Harvester) [to map]: Draft scene order against the declared order
- **Sequence and Brainstormer** (Sequence challenges Brainstormer) [to map]: Ideas that shift the order
- **Sequence and Weaver** (challenge each other) [to map]: Weaver builds the order / Sequence challenges pacing and reveal order (merge candidate)
- **Sequence and Outliner** (challenge each other) [to map]: Beats and rhythm / beats that flatten the rhythm

### Setting
- **Setting and Scene** (challenge each other) [to map]: Each scene's location / places used inconsistently
- **Setting and Continuity** (Continuity challenges Setting) [to map]: Place and time contradictions
- **Setting and Completeness** (challenge each other) [to map]: Completeness checks across areas for what no single watcher sees / the watcher reports its own area
- **Setting and Harvester** (Setting challenges Harvester) [to map]: Draft's places differ from the saved world
- **Setting and Researcher** (Researcher feeds Setting) [to map]: Real-world places and period facts
- **Setting and Brainstormer** (Setting challenges Brainstormer) [to map]: Ideas the world cannot support
- **Setting and Outliner** (challenge each other) [to map]: Location per beat / beats that need a place that doesn't exist

### Scene
- **Scene and Viewpoint** (challenge each other) [to map]: Viewpoint owner and what the scene may reveal / the turn needs knowledge the owner lacks
- **Scene and Learning** (Learning challenges Scene) [to map]: Scene terms (turn, goal) before taught
- **Scene and Continuity** (Continuity challenges Scene) [to map]: A scene contradicts saved facts
- **Scene and Completeness** (challenge each other) [to map]: Completeness checks across areas for what no single watcher sees / the watcher reports its own area
- **Scene and Harvester** (Scene challenges Harvester) [to map]: Draft scenes that match no slot
- **Scene and Brainstormer** (Scene challenges Brainstormer) [to map]: Ideas that give a scene no purpose
- **Scene and Weaver** (challenge each other) [to map]: Weaver makes slots / a slot with no purpose or turn
- **Scene and Outliner** (challenge each other) [to map]: Beats deepen the scene / a scene with no turn after deepening
- **Scene and Gatekeeper** (Gatekeeper challenges Scene) [to map]: Every slot has a purpose before deepening

### Viewpoint
- **Viewpoint and Voice** (challenge each other) [to map]: Style must follow the viewpoint mapping / a voice profile that points to the wrong owner
- **Viewpoint and Learning** (Learning challenges Viewpoint) [to map]: Terms (point of view, limited) before taught
- **Viewpoint and Continuity** (Continuity challenges Viewpoint) [to map]: Knowledge leaks in a limited viewpoint
- **Viewpoint and Completeness** (challenge each other) [to map]: Completeness checks across areas for what no single watcher sees / the watcher reports its own area
- **Viewpoint and Harvester** (Viewpoint challenges Harvester) [to map]: Draft slips viewpoint
- **Viewpoint and Weaver** (challenge each other) [to map]: An owner per slot / slots with no possible owner
- **Viewpoint and Outliner** (challenge each other) [to map]: An owner per scene / beats the owner can't know

### Voice
- **Voice and Completeness** (challenge each other) [to map]: Completeness checks across areas for what no single watcher sees / the watcher reports its own area
- **Voice and Harvester** (Voice challenges Harvester) [to map]: Harvested prose against the voice profile (later phase)

### Learning
- **Learning and Researcher** (Researcher feeds Learning) [to map]: Researched craft concepts feed the topic list
- **Learning and Brainstormer** (Learning challenges Brainstormer) [to map]: Ideas offered with unearned terms
- **Learning and Weaver** (Learning challenges Weaver) [to map]: Framework names before taught
- **Learning and Outliner** (Learning challenges Outliner) [to map]: Outline terms before taught
- **Learning and Planner** (Learning feeds Planner) [to map]: The learning path is one input to the ranking
- **Learning and Session keeper** (Learning challenges Session keeper) [to map]: Recap wording that uses unearned terms

### Continuity
- **Continuity and Harvester** (Continuity challenges Harvester) [to map]: Harvested facts that contradict the story
- **Continuity and Researcher** (Continuity challenges Researcher) [to map]: A researched fact contradicts the story's world
- **Continuity and Brainstormer** (Continuity challenges Brainstormer) [to map]: Ideas that break saved facts
- **Continuity and Weaver** (Continuity challenges Weaver) [to map]: Skeleton contradicts timeline or knowledge order
- **Continuity and Outliner** (Continuity challenges Outliner) [to map]: Deepened beats contradict earlier ones
- **Continuity and Planner** (Continuity feeds Planner) [to map]: Flags become candidate questions
- **Continuity and Gatekeeper** (Continuity feeds Gatekeeper) [to map]: Gates need a clean full pass

### Completeness
- **Completeness and Planner** (Completeness feeds Planner) [to map]: Completeness gaps feed the question ranking
- **Completeness and Gatekeeper** (Completeness challenges Gatekeeper) [to map]: Challenges the Gatekeeper's call that the story is done

### Harvester

### Researcher

### Brainstormer

### Weaver
- **Weaver and Outliner** (challenge each other) [mapped (pilot)]: Outliner finds slots that can't carry beats / Weaver finds deepening that departs from the skeleton
- **Weaver and Gatekeeper** (Gatekeeper challenges Weaver) [to map]: Full consistency pass clean before weaving

### Outliner
- **Outliner and Gatekeeper** (Gatekeeper challenges Outliner) [to map]: Full pass and skeleton before deepening

### Planner
- **Planner and Gatekeeper** (Gatekeeper feeds Planner) [to map]: Gate state decides which questions can be asked
- **Planner and Session keeper** (Planner feeds Session keeper) [to map]: Planner's top question becomes the recap's one suggestion

### Gatekeeper

### Session keeper

### Auditor (every agent)

- **The Auditor and each other agent** (challenge each other) [to map, one rule]: the Auditor checks the agent's evidence and that it stayed within its concern; the audited agent may respond once, with evidence the program can verify, or raise the point as a story question to the interviewer. Scope is enforced mechanically by each agent's primitive where it can be; the Auditor judges only content that drifts into another agent's area.
