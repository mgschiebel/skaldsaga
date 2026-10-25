# Craft record

`skaldsaga craft ...` keeps what the interviewer needs to teach craft inside its questions (HANDOFF open item 19). Teaching is invisible to the writer; these commands only hold the facts. All output is JSON, codes and IDs only.

## What is stored

| What | Where | In git? |
|---|---|---|
| The writer's record: taught, demonstrated, corrected misconceptions, teaching preference, starting experience check | `.harness/writer/craft.json` | Yes |
| Concepts researched for this story, one file each, always provisional | `.harness/writer/concepts/<concept>.json` | Yes |
| The built-in concept list | inside the program (`crates/skaldsaga-core/data/concepts.json`) | n/a |

It is writer state, not story meaning, so none of it is in the spec, and it is not in `settings.json` (settings are tunable behavior; this is about the person).

## Rules

- **Never use a term before it is taught.** `status` gives each concept a `term_use`: `explain` (not taught: explain in plain words, don't use the term), `term` (taught: use the full term), `short` (earned: use the shorthand, or the story's own names once they exist).
- **Taught is not demonstrated.** The writer saying they already know something records `taught` only. `demonstrate` needs evidence: IDs from the story where the writer used the concept correctly (exit 64 without them), and is only ever recorded by the interviewer's observation.
- **Nothing is deleted.** A newer entry marks the older one `superseded` and keeps it. A misconception sets a demonstrated concept back to taught and stays as history, corrected or not.
- **Concept IDs** are lowercase letters, digits and underscores (40 at most) and are checked before they become a path. A concept must exist to be recorded against.
- **Unusable files are never overwritten.** A record that is broken, too big (256 KiB), the wrong format or a link is reported and left alone. Researched concept files over 32 KiB or badly named are listed under `problems` and skipped.
- **Option order is shuffled in code** (`shuffle`), because asking a model to vary it doesn't hold up. Position must not sway the writer.

## Commands

| Command | Notes |
|---|---|
| `craft status [--project DIR]` | The record, every concept with `level`, `term_use`, `open_misconception`, `teach_first` (concepts it builds on that aren't taught), and any `problems`. |
| `craft teach --concept ID [--via interviewer\|writer_said]` | Records `taught`. No change if already taught or better. |
| `craft demonstrate --concept ID --evidence ID,ID` | Records `demonstrated` (20 IDs at most, each a well-formed story ID). |
| `craft misconception --concept ID --words-file FILE [--corrected yes\|no]` | The writer's belief, from a file (16 KiB cap, links refused), shortened to 500 characters. |
| `craft teaching --level normal\|less\|more_why` | How much explaining rides along with a question. Never removes the one sentence of why. |
| `craft experience --written-before yes\|no --read-craft-books yes\|no` | The light starting check. |
| `craft research-add --file FILE` | Stores a researched concept, marked provisional. Refused if the ID exists, it builds on something that isn't a concept, or it uses a term another concept says to avoid. Unknown fields are refused. |
| `craft export` | The researched concepts with the story's own examples removed, ready to review for the built-in list. Nothing is collected automatically; story folders are private. |
| `craft shuffle --count N` | A random order for 2 to 12 options. |

Writes return `outcome`, `add_paths` and `message_file` (see `docs/commit.md`), or `outcome: unchanged` with no paths. The skill commits only the paths returned.

## Concept entries

`id`, `term` (the one chosen term), `short` (shorthand, may be empty), `avoid` (terms we never use), `builds_on`, `explanation` (our own words), and for researched concepts `provisional`, `sources` (links and titles, never copied text) and `examples` (story-specific, kept apart from the definition so promoting a concept is a copy and a review). The built-in list covers the concepts the shape decisions need; stages and landmarks join when the interviewer is written.
