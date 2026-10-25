# Future phases

*Recorded ideas that are not in scope now. Part of the Skaldsaga handoff; the index is [`HANDOFF.md`](../../HANDOFF.md). Moved verbatim from HANDOFF.md (v9, last edited October 24, 2026) on October 24, 2026: the wording below is unchanged.*

<!-- begin moved text -->

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


