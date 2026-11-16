*Read this when `route` reports `story_mid_thread` or `story_stopping_point` (step 0 in `SKILL.md`).*

## Resuming a story

### story_mid_thread

The last session ended in the middle of something, or the story has content and no record of how the last session ended. Pick it up with a short recap and one question. **Nothing on disk changes until the writer answers:** the recap runs only commands that read, writes no file and saves no snapshot.

1. Run `skaldsaga recap` (add `--project` when needed). It only reads. Read `premise`, `people`, `loose_idea` (the writer's own words, kept exactly), `last_topic` and `topic`. `stages_without_summary` is for you; never list it to the writer. If the command fails, say plainly that something in the story's files can't be read, change nothing and stop. Then run `skaldsaga idea list --touching <topic>` (it only reads; skip it when `topic` is `null`) to see whether a saved idea belongs to that topic.
2. Say two or three plain sentences of what is decided: what the story is (from `premise`, in the writer's own words) and who is in it (`people`). If `loose_idea` isn't null, mention it in the writer's own words as something they floated, not something decided. Never list what's missing; a gap shows only through the question. Then ask **one** question, the one for `topic`:

   | `topic` | The question, in plain words |
   |---|---|
   | `premise` | What's the story about? |
   | `people` | Who's in it? |
   | `setup` | How does it begin? What happens first? |
   | `complications` | What gets in the way as it goes on? |
   | `crisis` | What's the worst moment, where it all falls apart? |
   | `resolution` | Where do you see it ending? |
   | `null` | No question: the basic shape is down. Say so, and that carrying on from here isn't built yet. Stop. |

   If `topic` is the same as `last_topic`, it was asked before and didn't get settled: ask it again from a fresh angle, in different words. Never repeat the earlier sentence.

   A loose idea is never the question, and neither is a saved idea. If `idea list` shows ideas for this topic, mention at most **one**, the best fit, in the writer's own words, as a way to answer ("or we could try <the idea>"); that uses its one mention for the session (see **Saved ideas come back up**). Where there is no saved idea for the topic, a loose idea may take that place: "or we can come back to <their idea>". That is still the turn's only question.

```say
You're writing a modern Jack and Jill, set in a city at night, and Jack and Jill are in it. You also floated a dark alley. Let's start with how it begins: what happens first?
```

```say
You're writing a modern Jack and Jill, set in a city at night, and Jack and Jill are in it. Let's start with how it begins: what happens first? Or we could try that night-bus idea from last time.
```

```say
Last time we were circling how it begins. Picture the very first scene: where are we, and who's there?
```

3. Wait for the answer. When the writer answers the question you asked, first run `skaldsaga session ask --topic <topic>` (the code you asked about; skip it when they took the loose idea instead). Then handle their answer exactly as in **After every answer** and **On a yes** in the opener: record it, reflect it back, and save it as in **On a yes** (a yes to a real question, or the writer carrying on). An answer about how the story begins becomes the main plot's first summary; one about the middle, the worst moment or the ending fills that part of the main plot, using the small-draft rule in **On a yes**. Then follow **Following the writer's focus**.

### story_stopping_point

The last session ended at a stopping point: the writer was done, or a part of the story had run its course. Pick the story back up with a short recap that ends in one suggestion for what to look at next. **Nothing on disk changes until the writer answers:** the recap runs only commands that read, writes no file and saves no snapshot.

1. Run `skaldsaga recap` (add `--project` when needed). It only reads. Read `premise`, `people`, `topic` (the suggestion) and `choices`. `stages_without_summary` is for you; never list it to the writer. If the command fails, say plainly that something in the story's files can't be read, change nothing and stop. Then run `skaldsaga idea list --touching <topic>` (it only reads; skip it when `topic` is `null`).
2. Say two or three plain sentences of what is decided: what the story is (from `premise`, in the writer's own words) and who is in it (`people`). Never list what's missing. Then make **one** suggestion, the question for `topic` from the table in **story_mid_thread**, and say they can pick something else. If `idea list` shows ideas for `topic`, you may mention at most **one**, in the writer's own words, as a way to take the suggestion (see **Saved ideas come back up**). If `topic` is `null`, the basic shape is down: say so, and that carrying on from here isn't built yet. Stop.

```say
You're writing a modern Jack and Jill, set in a city at night, and Jack and Jill are in it. A good place to pick it up is how it begins. Want to start there, or somewhere else?
```

3. Wait for the answer. If they take the suggestion or name another part of the story, run `skaldsaga session ask --topic <the code for that part>` (the codes in the table), ask the question for it, and handle their answer exactly as in **After every answer** and **On a yes**. Then follow **Following the writer's focus**.
