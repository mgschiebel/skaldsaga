# Design: the story side

*Project, name, audience, story-shape layer, completeness bar, story model, coverage grid, weaver and outliner. Load it for interviewer, outliner or weaver planning. Part of the Skaldsaga handoff; the index is [`HANDOFF.md`](../../HANDOFF.md). Moved verbatim from HANDOFF.md (v9, last edited October 24, 2026) on October 24, 2026: the wording below is unchanged.*

<!-- begin moved text -->

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

- **Interviewer voice: the curious first reader** (*Decided October 25, 2026*, planning session 1): the interviewer asks like someone who wants to know what happens next in the writer's story, not like a coach grading it or a neutral form. Craft terms arrive only when a question needs them, through the terminology ladder above. Rejected: the warm writing coach (risks sounding like a lesson) and the quiet neutral guide (risks feeling cold, teaches less).

- **Opening funnel: three ways in, one shared flow** (*Decided October 25, 2026*, planning session 1; author's idea, Claude's refinements): the opening depends on what the writer typed with the command. (1) **Bare command, new story:** one question that covers both paths 1 and 3: "What's the story, even a scrap? Or if you've got nothing yet, say so and we'll find one together." (2) **Command plus an idea (or a pasted draft):** start from what they gave; reflect it back as a curious reader, confirm two or three facts, commit them. (3) **Brainstorm:** help the writer find a story (taste and genre questions are natural here); nothing becomes canon until they pick, and the pick moves them to path 2. **The paths converge:** the opening ends once the writer owns an initial idea. **The experience check is not asked up front;** it is inferred from how the writer writes and what they say, and refines open item 19's "light starting check" (ask lightly) to infer first. A bare command in a folder that already has a story is a recap, not an opener (orchestrator planning, open item 20).

- **What the opening collects: about, begins, ends** (*Decided October 25, 2026*, planning session 1; the three-part wording is the author's, the "any one is enough" rule is *Proposed*): the opening asks in loose, plain words what the story is about, how it begins and where the writer sees it ending, not for a character who wants something with something in the way (too abstract for a novice). Those are concrete instances; the want-and-obstacle shape, the dramatic question and the ending's outcome and judgment (shape decisions 5, 8 and 9) are inferred from the answers and confirmed back, then fleshed out in a craft-like next step. *Proposed:* any one of the three is enough to leave the opening, a scrap counts, and each missing piece gets one gentle follow-up before we move on.

- **After the opening, follow the writer's focus** (*Decided October 25, 2026*, planning session 1; the author's idea; the concrete rule for the gap fallback is *Proposed*): the first craft-like question is inferred from what the writer focused on most, and the interviewer keeps going down that thread while it is topical, until a stopping point, before returning to other areas. Examples: concrete scenes lead to "which moment are you most excited to write?"; characters lead to who else is in the story and then the main character; the ending leads to how it feels. This is the concrete form of "no fixed question order" and "when momentum slows, offer a choice." *Proposed:* when there is nothing to infer, the interviewer takes the gap with the highest unblocking value (the same ranking and fan-out count as the momentum offer).

- **Stopping point and drift guard** (*Decided October 25, 2026*, planning session 1; the author's idea; the parking detail is *Proposed*): a thread ends, and the interviewer offers a choice of what to look at next, at whichever comes first: the **writer's signals** (answers get short, "not sure," or no new details, twice in a row) or **thread exhaustion** (nothing left to ask on the topic without repeating). **Drift guard:** when the writer keeps adding details that don't seem to connect to the story, or goes off on tangents, the interviewer asks a gentle back-to-the-story question ("Is this part of the story you told me about? I thought we were heading toward the ending you described"), never a refusal. *Proposed:* the tangent is parked as a provisional item, not dropped, and the writer chooses whether to follow it or return.

- **Confirm back after every answer, in craft-like prose** (*Decided October 25, 2026*, planning session 1; the author's lean, Claude agreed; the details after the colon are *Proposed*): after each answer the interviewer rewords it in more craft-like prose and asks whether that is what the writer meant. This is a soft teaching moment and validates the spec as it grows. Only confirmed facts are committed; each confirmed reword is one small proposal through `check` and `commit`. **Session-start recap:** it ends with the reword of the writer's last answer, for validation. *Proposed:* the reword is the turn's only question (the next question waits for the writer's yes or correction); a long answer or pasted draft gets one reword covering the two or three facts that matter most; a craft term is introduced once in the reword, then follows the terminology ladder.

- **The session-start recap depends on how the last session ended** (*Decided October 25, 2026*, planning session 1; the author's idea; the build consequences are *Proposed*): (1) **Left mid-thread:** the recap ends with the reword of the writer's last answer, to pick the thread back up. (2) **Left at a stopping point** (writer signals or an exhausted thread): the recap ends with one recommendation, a topical shift into a new area (chosen by unblocking value; the writer still chooses). (3) **Bare command and no story found:** the interviewer does not assume a new start. It says plainly that it isn't finding a story and asks whether that is right or whether the writer expected one (wrong folder, a story that should be there); if it is new, the opening funnel above takes over. *Proposed:* the program records how the last session ended (mid-thread with the last answer and whether it was confirmed, or at a stopping point), and `init` runs only after the writer confirms a new story. The fuller at-a-glance picture of the whole story (open item 20) is not part of every recap; when it appears is still open.

- **A story exists and the writer types the command with text** (*Decided October 25, 2026*, planning session 1): the interviewer asks one short question, "Is this a new story, or more about the one we're working on?", and does not guess. A new story starts the opening funnel (and `init`, after the writer confirms); more about the current story is treated as an answer to the open thread (reworded and confirmed). Rejected: treating it as an answer without asking (a new idea gets misread) and treating it as new material for the current story (fails when the writer has a second story).

- **A story folder exists but nothing is committed yet** (*Decided October 25, 2026*, planning session 1): the interviewer goes straight to the opener with a light acknowledgement ("We set things up last time but didn't get anything down yet. What's the story, even a scrap?"), without asking "I'm not finding a story" again, because the writer already confirmed a story when `init` ran. This condition means no committed facts and no unconfirmed answer in the last-session marker; a marker holding an unconfirmed answer is the mid-thread condition instead.

- **What the opening commits** (*Decided October 25, 2026*, planning session 1): only what the writer confirmed in the reword. "What it's about" goes into the main arc's `premise` in the writer's own words. "How it begins" is the main plot thread's Setup summary and "where it ends" is its Resolution summary, both provisional; these are planned grid cells with no scene, so no half-formed scene slot is created and the empty Complications and Crisis cells become the natural next questions ("how do you see the story getting from here to the ending you described?"). People and places the writer names become person and place nodes with the facts they stated, provisional until confirmed. Shape decisions are not committed at the opening; they are inferred later with scenes as evidence and confirmed then. **A scene is created only when the writer describes a concrete moment** (usually in the follow-the-focus step), and the weaver later turns planned cells into slots as already decided. Rejected: scene slots for every begin and end (a vague scrap pollutes the sequence) and facts only on the premise (nothing visibly takes shape, and the grid cannot guide the next question).

- **Slice 1 build choices** (*Decided October 25, 2026* where marked with the author; the rest *Proposed* for review): the author approved the slice 1 plan and chose a **`propose` command** for filing each confirmed answer (the program picks the ID, agent and time, so the skill never writes into the inbox). *Proposed:* text typed into a story folder with nothing committed goes straight to the opener as the first answer, since there is nothing to be "more about"; committed content with no record of how the last session ended reads as a thread still open; the opener's skill may write only one scratch file, `.harness/draft.txt`; the last-session marker is `.harness/writer/last-session.json`, committed with the craft record; a damaged folder (an unreadable story file or marker) is said plainly and nothing is changed. Details: `docs/entry.md`.

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
- **Every scene is tied to a thread and stage** (*Decided October 25, 2026*, planning session 1; the author's concern; checked against the code: a scene's `coverage` is optional today and nothing flags a scene without one): (1) the interviewer **infers the thread and stage** from the moment the writer describes and says it plainly inside the reword ("that sounds like part of how your story begins, is that right?"); the writer's confirmation is the cross-check. Tying a scene to a thread ties it to the arc. (2) A deterministic check flags a scene with no thread and stage as a **soft flag at proposal time** (raised while the topic is live) and as a **blocker for "complete."** It is not a hard rule, so writers can still draft scenes before structure exists and harvest scenes from a pasted draft. (3) Whether a scene fits the stage it claims is a judgment for the housekeeper (not built); until then the writer's confirmation covers it. *Built in slice 1 (0.4.0):* `scene_untied` is a soft finding in `check` and a blocker for the `complete` gate only (`docs/entry.md`).
- **Resuming with no input: pick the next question fresh, and remember the last topic** (*Decided October 25, 2026*, planning session 2; from the 0.4.0 test run, where `/skaldsaga` with no text in a saved story stopped at `story_mid_thread` with no summary and no question). (1) The session-start recap for `story_mid_thread` is a short plain-language summary of what the story has so far, built from the premise, the people named and the thread stages on disk, followed by **one** question. (2) The question is **chosen fresh from what is on disk**, not re-asked from a stored copy: the marker does not hold question text, so nothing generated sits in a tracked file and a resume never repeats a stale question. (3) The marker gains **one small field, the topic last asked about** (a short code, not text and not writer words), so the choice can avoid repeating itself and can tell "asked but unanswered" from "never asked". It is written when the question is asked, which means one more marker write per question; how to keep that from adding snapshots (the test run already saw one per answer) is open, below. (4) Nothing on disk changes until the writer answers. **Slice 2 covers the mid-thread recap only** (*Decided October 25, 2026*); the stopping-point recap waits for slice 3, with stopping-point detection and the offer. *Not built yet; first item under Ready to build.* Open: how to avoid a snapshot per marker write, how the fresh choice ranks topics (for example setup and ending before the alley detail), the exact topic codes, and whether the tentative "maybe a dark alley" belongs in the premise or on a stage (raised by the test run).

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


