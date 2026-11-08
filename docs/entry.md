# `route`, `session` and `propose`

The program side of the interviewer's opening (HANDOFF, Slice 1). All three only ever read a small file the skill wrote, never writer text from the command line (SR-24).

## `skaldsaga route --text yes|no [--project DIR]`

Finds the story folder like `locate` and names how the session opens. `--text` is only whether the writer typed anything with the command; the text itself never reaches the program, so story text can't steer the routing. It only looks: it never runs `init`, never creates a folder, never writes.

Prints `found`, `path`, `name` (and `several`) as `locate` does, plus:

| Field | Meaning |
|---|---|
| `condition` | One of the seven below, or `null` when several story folders were found and the writer must choose first |
| `built` | Whether the conversation for that condition exists yet (every condition is built since version 0.7.0; the mid-thread recap came in slice 2 and the stopping-point path in slice 3) |
| `damage` | For `story_damaged` only: codes (`spec_unreadable`, `spec_problems`, `marker_unusable`), never paths or story text |

| Condition | When |
|---|---|
| `no_story_bare` | No story found, nothing typed |
| `no_story_with_text` | No story found, text typed (a lone candidate repository still counts as no story; the skill asks about it) |
| `story_empty` | A story folder with nothing committed and no answer recorded. Text typed here is the idea for the opener (*Proposed*) |
| `story_with_text` | Something is under way (committed content, or an answer recorded) and text was typed: the skill asks "new story or more about this one?" |
| `story_mid_thread` | Last session ended mid-thread (an answer recorded, confirmed or not), or this computer has a thread open (an unconfirmed answer or a question asked), even over a committed stopping point. Committed content with no record at all reads the same way (*Proposed*) |
| `story_stopping_point` | Last session ended at a stopping point |
| `story_damaged` | A story file or the record below can't be read. Checked first, whatever was typed |

## `skaldsaga session answer --words-file FILE --confirmed yes|no`, `session ask`, `session save` and `session stop --reason CODE`

Record how the session stands. The note is split in two (*Decided October 25, 2026*, planning session 3; built in slice 3, version 0.6.0), because the cloud session's own stop check flags any uncommitted change:

- **Committed:** `.harness/writer/last-session.json` (format 1; writer state, not story meaning). Either `mid_thread` with the last *confirmed* answer, the topic and the save-offer count, or `stopping_point`. It changes only inside a real save: a confirmed answer, a save the writer accepts, or a stop. A file that can't be used is an error and is never overwritten. A note written by an earlier version may still hold an unconfirmed answer; it is still read.
- **This computer only:** `.harness/session-local.json` (ignored by git, in `.harness/.gitignore`; never printed back). It holds the unconfirmed answer (at most 2,000 characters), the topic last asked about, a save-offer count the writer declined, and `answers_this_session`. A file that can't be used reads as empty; it never gets in the writer's way. The session-start hook starts `answers_this_session` over for a fresh session (`startup`, `clear`).

**Cost, accepted by the author:** an unconfirmed answer or a question just asked is lost if the writer carries on from another device or a fresh copy; the recap then rewords the last confirmed answer.

| Command | What it does |
|---|---|
| `session answer ...` | Unconfirmed: kept on this computer, nothing for git. Confirmed: writes the committed note (with the topic and the count taken over from this computer) and adds one to the count and to `answers_this_session` (confirming the same words again does not). |
| `session ask --topic CODE` | Records the topic being asked about, on this computer. Allowed at a stopping point too: the writer has picked a new area, so the story reads as a thread in progress again. A topic that isn't one of the six codes is a usage error. |
| `session save --taken yes\|no` | Records that a save offer was made. `no`: the count starts over on this computer, nothing for git. `yes`: writes the committed note (topic folded in, count zero) and returns its path and a message file for a snapshot. |
| `session stop --reason CODE` | Records a stopping point and clears this computer's thread state. `--reason` is required: `writer_signals` (answers got short or unsure, twice in a row; needs at least 2 confirmed answers this session), `thread_exhausted` (nothing left to ask; at least 1) or `writer_done` (the writer said so; always allowed). The skill judges the reason; the program only checks the claim is plausible. Returns `unconfirmed_pending` (true when an unconfirmed answer was pending, which the stop drops), the note's path and a message file. |

**Nothing but a real save writes the committed note** (*Decided October 25, 2026*, planning session 3). `session answer` and `session ask` return `outcome`, `add_paths` (empty for a change kept on this computer; the note's path and `.harness/.gitignore` after a confirmed answer, so the ignore line reaches folders set up by an earlier version) and `save_offer` (true when the count has reached `session.save_offer_after_answers` in `.harness/settings.json`, default 10), and no message file. Only `session save --taken yes` and `session stop` return a message file.

**The answer count stands in for the meter** (*Decided November 1, 2026*, planning session 4; the numbers are *Proposed*). `skaldsaga meter check` uses the context window when it has a reading. When it has none (`unknown`, as in the cloud), it uses `answers_this_session`: `suggest_wrap_up` at `session.suggest_wrap_up_after_answers` (default 25) and `wrap_up` at `session.wrap_up_after_answers` (default 35). The result then carries `source` `answers` and `answers_this_session`; a real reading is never overruled, and below the first count the state stays `unknown`. If the two settings are out of order or unusable, both fall back to the defaults.

## `skaldsaga recap [--project DIR]`

Read-only (*name Proposed*). What a story left mid-thread has so far, and the one topic to ask about next; the skill words the summary and the question from it. Prints `premise`, `people` (alphabetical, at most 12), `stages_without_summary` (codes, for the skill only), `loose_idea` (the writer's unconfirmed last answer, in their own words), `last_topic`, `topic` (`null` when nothing is left to ask in this slice), `answers_since_offer`, `save_offer_after`, `save_offer`, `state` (`mid_thread`, or `stopping_point` when the last session ended at one and this computer has no thread open since) and `choices` (up to three topic codes to offer at a stopping point, the suggestion first, then the next missing parts in ranking order). It never prints a question or writes anything. An unusable marker is an error and is left alone.

**Ranking rule** (*Decided October 25, 2026*): if the topic last asked about still has nothing in the story (a premise, a person, or a summary for that part of the main plot), ask it again; the skill finds a fresh angle. Otherwise ask for the premise if there is none, then the earliest part of the main plot with no summary, starting with how it begins. Unit tests cover the rule in `crates/skaldsaga-core/src/recap.rs`.

**The topic is recorded when the writer answers, not when the question is asked** (*Proposed*, an implementation choice): the design says both "nothing on disk changes until the writer answers" and "the topic is written when the question is asked". The first is the acceptance check, so the skill runs `session ask` as the first step of handling the answer, alongside the answer itself. The recap stays read-only, and a topic still shows as asked-but-unsettled when the writer answered but never confirmed.

## `skaldsaga propose --file FILE [--project DIR]`

Files a drafted answer as a proposal in `.harness/inbox/` and prints `{"outcome": "filed", "id": ...}`; `check` and `commit` follow as usual. A draft holds only `answer` (the writer's confirmed words, up to 2,000 characters) and `ops`. It can't carry its own `id`, `agent`, `lane`, `created`, `revisions` or `check`: the program chooses the ID, files it as the interviewer's, and stamps the time, so a draft can't ask for a looser lane or arrive already checked.

New things are named with placeholders: `@` and a kind letter (`p` person, `l` place, `g` group, `i` item, `s` scene, `t` thread, `a` arc, `f` fact, `d` decision, `st` style), a colon and a short lowercase name, for example `@p:mara` or `@a:main.threads`. Each distinct placeholder in `ops` becomes one fresh ID of that kind everywhere it appears (the `answer` is left alone), so the skill never makes up an ID. `@pr:`, `@e:` and `@wd:` are not placeholders.

## `scene_untied`

A scene with no thread and stage is a **soft finding** in `check` (raised only when the answer introduces it, like every finding) and a **blocker** for the `complete` gate only. Weaving and outlining don't wait on it, so scenes can still be drafted, or harvested from a pasted draft, before structure exists. The `basics` scenario's sample scenes are untied, so its first answer comes back `flagged` and is kept with `--decision keep_anyway`.

## Tests

`crates/skaldsaga-core/src/entry.rs`, `marker.rs`, `session.rs` and `propose.rs` (unit tests, one fixture per condition); `crates/skaldsaga/tests/entry_cli.rs` (the real binary, including the opener's exact drafts through `propose`, `check` and `commit`); `crates/skaldsaga/tests/session_cli.rs` (the split note, the answer-count stand-in, stop reasons); `crates/skaldsaga/tests/skill_prompt.rs` (every writer-facing line in `SKILL.md` free of internal words, the skill's tool permissions, no writer words in commands); `plugin/skills/skaldsaga/evals/` (model-run interview evals, left out of the release).

## The skill's stopping-point path (slice 3, part 2; version 0.7.0)

Three sections of `SKILL.md`, all judged by the skill and recorded by the commands above:

- **`story_stopping_point`** (resuming): `recap` (read only), two or three sentences of what is decided, one suggestion for `topic`, and the writer may pick another area. Nothing is written until they answer; then `session ask --topic` as in the mid-thread recap.
- **Reaching a stopping point:** on the writer's signals (short, unsure or empty answers twice in a row, or they say they're done) or an exhausted topic, the skill runs `recap`, skips the choice equal to `last_topic`, says what the part added, offers up to three areas with its suggestion first and one line offering to save. A chosen area is a `session ask`; a stop is `session stop --reason ...` plus one snapshot from the paths and message file it returns; stopping without saving records nothing. If an answer is waiting when the writer stops, the skill checks it with one short question first; only if the writer skips the check, or never answered a real question, does it show the answer and say plainly that it is not saved yet and may be lost the next time the writer starts.
- **When the story drifts:** after two details in a row that don't connect, or a tangent, the skill asks one gentle question (is it part of the story you told me about?) and keeps the tangent with `session answer --confirmed no`, so it comes back as the loose idea. Only the latest tangent is kept; a fuller holding area waits for the brainstorm path (*Proposed*).

Pinned by `skill_prompt.rs`; the model-run evals are `resume-from-a-stopping-point-*`, `short-answers-*`, `skipping-the-check-*`, `stop-after-*` and `tangent-*`.
