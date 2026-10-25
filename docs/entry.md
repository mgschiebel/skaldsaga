# `route`, `session` and `propose`

The program side of the interviewer's opening (HANDOFF, Slice 1). All three only ever read a small file the skill wrote, never writer text from the command line (SR-24).

## `skaldsaga route --text yes|no [--project DIR]`

Finds the story folder like `locate` and names how the session opens. `--text` is only whether the writer typed anything with the command; the text itself never reaches the program, so story text can't steer the routing. It only looks: it never runs `init`, never creates a folder, never writes.

Prints `found`, `path`, `name` (and `several`) as `locate` does, plus:

| Field | Meaning |
|---|---|
| `condition` | One of the seven below, or `null` when several story folders were found and the writer must choose first |
| `built` | Whether the conversation for that condition exists yet (`false` only for `story_stopping_point`; the mid-thread recap is built in slice 2) |
| `damage` | For `story_damaged` only: codes (`spec_unreadable`, `spec_problems`, `marker_unusable`), never paths or story text |

| Condition | When |
|---|---|
| `no_story_bare` | No story found, nothing typed |
| `no_story_with_text` | No story found, text typed (a lone candidate repository still counts as no story; the skill asks about it) |
| `story_empty` | A story folder with nothing committed and no answer recorded. Text typed here is the idea for the opener (*Proposed*) |
| `story_with_text` | Something is under way (committed content, or an answer recorded) and text was typed: the skill asks "new story or more about this one?" |
| `story_mid_thread` | Last session ended mid-thread (an answer recorded, confirmed or not). Committed content with no record at all reads the same way (*Proposed*) |
| `story_stopping_point` | Last session ended at a stopping point |
| `story_damaged` | A story file or the record below can't be read. Checked first, whatever was typed |

## `skaldsaga session answer --words-file FILE --confirmed yes|no`, `session ask`, `session save` and `session stop`

Record how the session stands, in `.harness/writer/last-session.json` (format 1; writer state, not story meaning; committed like the craft record so it follows the story between sessions). Either `mid_thread` with the writer's last answer and whether they confirmed the reword (at most 2,000 characters), or `stopping_point`. A file that can't be used is an error and is never overwritten. Saying the same thing again changes nothing. The command never prints the writer's words back.

Slice 2 adds two small fields, both only for `mid_thread`: `topic` (the topic last asked about, one of `premise`, `people`, `setup`, `complications`, `crisis`, `resolution`, never text) and `answers_since_offer` (confirmed answers since the last save offer, left out when zero). A `mid_thread` marker needs a last answer or a topic. A stopping point has neither.

| Command | What it does |
|---|---|
| `session answer ...` | Records the answer. Keeps the topic. A newly confirmed answer adds one to the count (confirming the same words again does not). |
| `session ask --topic CODE` | Records the topic being asked about. Keeps the last answer and the count. Starts a mid-thread marker if there is none. Refused at a stopping point. A topic that isn't one of the six codes is a usage error. |
| `session save --taken yes\|no` | Records that a save offer was made: the count starts over. With `yes` it returns the marker's path and a message file for a snapshot; with `no` it saves nothing. |
| `session stop` | Records a stopping point (clears the topic and the count). |

**A marker write never makes a snapshot of its own** (*Decided October 25, 2026*). `session answer` and `session ask` return `outcome`, `add_paths` (the marker's path, to stage with the next real save) and `save_offer` (true when the count has reached `session.save_offer_after_answers` in `.harness/settings.json`, default 10), and no message file. Only `session save --taken yes` and `session stop` return a message file. If a session ends abruptly the newest marker state may be uncommitted and a resume can be one step behind; accepted.

## `skaldsaga recap [--project DIR]`

Read-only (*name Proposed*). What a story left mid-thread has so far, and the one topic to ask about next; the skill words the summary and the question from it. Prints `premise`, `people` (alphabetical, at most 12), `stages_without_summary` (codes, for the skill only), `loose_idea` (the writer's unconfirmed last answer, in their own words), `last_topic`, `topic` (`null` when nothing is left to ask in this slice), `answers_since_offer`, `save_offer_after` and `save_offer`. It never prints a question or writes anything. An unusable marker is an error and is left alone.

**Ranking rule** (*Decided October 25, 2026*): if the topic last asked about still has nothing in the story (a premise, a person, or a summary for that part of the main plot), ask it again; the skill finds a fresh angle. Otherwise ask for the premise if there is none, then the earliest part of the main plot with no summary, starting with how it begins. Unit tests cover the rule in `crates/skaldsaga-core/src/recap.rs`.

**The topic is recorded when the writer answers, not when the question is asked** (*Proposed*, an implementation choice): the design says both "nothing on disk changes until the writer answers" and "the topic is written when the question is asked". The first is the acceptance check, so the skill runs `session ask` as the first step of handling the answer, alongside the answer itself. The recap stays read-only, and a topic still shows as asked-but-unsettled when the writer answered but never confirmed.

## `skaldsaga propose --file FILE [--project DIR]`

Files a drafted answer as a proposal in `.harness/inbox/` and prints `{"outcome": "filed", "id": ...}`; `check` and `commit` follow as usual. A draft holds only `answer` (the writer's confirmed words, up to 2,000 characters) and `ops`. It can't carry its own `id`, `agent`, `lane`, `created`, `revisions` or `check`: the program chooses the ID, files it as the interviewer's, and stamps the time, so a draft can't ask for a looser lane or arrive already checked.

New things are named with placeholders: `@` and a kind letter (`p` person, `l` place, `g` group, `i` item, `s` scene, `t` thread, `a` arc, `f` fact, `d` decision, `st` style), a colon and a short lowercase name, for example `@p:mara` or `@a:main.threads`. Each distinct placeholder in `ops` becomes one fresh ID of that kind everywhere it appears (the `answer` is left alone), so the skill never makes up an ID. `@pr:`, `@e:` and `@wd:` are not placeholders.

## `scene_untied`

A scene with no thread and stage is a **soft finding** in `check` (raised only when the answer introduces it, like every finding) and a **blocker** for the `complete` gate only. Weaving and outlining don't wait on it, so scenes can still be drafted, or harvested from a pasted draft, before structure exists. The `basics` scenario's sample scenes are untied, so its first answer comes back `flagged` and is kept with `--decision keep_anyway`.

## Tests

`crates/skaldsaga-core/src/entry.rs`, `marker.rs` and `propose.rs` (unit tests, one fixture per condition); `crates/skaldsaga/tests/entry_cli.rs` (the real binary, including the opener's exact drafts through `propose`, `check` and `commit`); `crates/skaldsaga/tests/skill_prompt.rs` (every writer-facing line in `SKILL.md` free of internal words, the skill's tool permissions, no writer words in commands); `plugin/skills/skaldsaga/evals/` (model-run interview evals, left out of the release).
