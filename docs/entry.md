# `route`, `session` and `propose`

The program side of the interviewer's opening (HANDOFF, Slice 1). All three only ever read a small file the skill wrote, never writer text from the command line (SR-24).

## `skaldsaga route --text yes|no [--project DIR]`

Finds the story folder like `locate` and names how the session opens. `--text` is only whether the writer typed anything with the command; the text itself never reaches the program, so story text can't steer the routing. It only looks: it never runs `init`, never creates a folder, never writes.

Prints `found`, `path`, `name` (and `several`) as `locate` does, plus:

| Field | Meaning |
|---|---|
| `condition` | One of the seven below, or `null` when several story folders were found and the writer must choose first |
| `built` | Whether the conversation for that condition exists yet (`false` for `story_mid_thread` and `story_stopping_point`) |
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

## `skaldsaga session answer --words-file FILE --confirmed yes|no` and `session stop`

Record how the session stands, in `.harness/writer/last-session.json` (format 1; writer state, not story meaning; committed like the craft record so it follows the story between sessions). Either `mid_thread` with the writer's last answer and whether they confirmed the reword (at most 2,000 characters), or `stopping_point`. A file that can't be used is an error and is never overwritten. Saying the same thing again changes nothing. Returns `add_paths` and `message_file` when something changed. The command never prints the writer's words back.

## `skaldsaga propose --file FILE [--project DIR]`

Files a drafted answer as a proposal in `.harness/inbox/` and prints `{"outcome": "filed", "id": ...}`; `check` and `commit` follow as usual. A draft holds only `answer` (the writer's confirmed words, up to 2,000 characters) and `ops`. It can't carry its own `id`, `agent`, `lane`, `created`, `revisions` or `check`: the program chooses the ID, files it as the interviewer's, and stamps the time, so a draft can't ask for a looser lane or arrive already checked.

New things are named with placeholders: `@` and a kind letter (`p` person, `l` place, `g` group, `i` item, `s` scene, `t` thread, `a` arc, `f` fact, `d` decision, `st` style), a colon and a short lowercase name, for example `@p:mara` or `@a:main.threads`. Each distinct placeholder in `ops` becomes one fresh ID of that kind everywhere it appears (the `answer` is left alone), so the skill never makes up an ID. `@pr:`, `@e:` and `@wd:` are not placeholders.

## `scene_untied`

A scene with no thread and stage is a **soft finding** in `check` (raised only when the answer introduces it, like every finding) and a **blocker** for the `complete` gate only. Weaving and outlining don't wait on it, so scenes can still be drafted, or harvested from a pasted draft, before structure exists. The `basics` scenario's sample scenes are untied, so its first answer comes back `flagged` and is kept with `--decision keep_anyway`.

## Tests

`crates/skaldsaga-core/src/entry.rs`, `marker.rs` and `propose.rs` (unit tests, one fixture per condition); `crates/skaldsaga/tests/entry_cli.rs` (the real binary, including the opener's exact drafts through `propose`, `check` and `commit`); `crates/skaldsaga/tests/skill_prompt.rs` (every writer-facing line in `SKILL.md` free of internal words, the skill's tool permissions, no writer words in commands); `plugin/skills/skaldsaga/evals/` (model-run interview evals, left out of the release).
