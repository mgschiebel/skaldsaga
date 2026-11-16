*Read this when `route` reports `no_story_bare`, `no_story_with_text`, `story_empty`, `story_with_text` or `story_damaged` (step 0 in `SKILL.md`). The entries for `story_mid_thread` and `story_stopping_point` are in `resuming.md`.*

## Route entries

### no_story_bare

Nothing in this folder looks like a story. Don't assume a new start. Ask one question and wait.

```say
I'm not finding a story in this folder. Is that right, or did you expect one to be here?
```

- They expected one: help them find the right folder (ask where it is, or suggest opening the folder their story is in). Set nothing up. Stop.
- It's new: do steps 1 to 3, then **the opener**, beginning with the opening question.

### no_story_with_text

The writer typed an idea and there is no story here. Ask one question and wait.

```say
I'm not finding a story in this folder yet. Want me to start a new one for this?
```

On a yes, do steps 1 to 3, then **the opener**, treating what they typed as their first answer (go straight to reflecting it back). On a no, stop.

### story_empty

A story folder was set up but nothing is saved in it yet; the writer already agreed to a story when it was set up, so don't ask again whether they want one. Skip steps 1 to 3 and go to **the opener**. If the writer typed text, treat it as their first answer; otherwise open with this, and nothing before it:

```say
We set things up last time but didn't get anything down yet. What's the story, even a scrap?
```

### story_with_text

A story is already under way and the writer typed something. Don't guess what it is. Ask one question and wait.

```say
Is this a new story, or more about the one we're working on?
```

Neither answer is built yet. If it's a new story, say so plainly, and suggest opening an empty folder for it. If it's more about this one, say that continuing a story isn't built yet. Change nothing either way.

```say
Starting a second story, or carrying on with this one, isn't built yet. For a new story, open an empty folder and try again there.
```

### story_damaged

Something in the story folder can't be read safely (`damage` lists codes only). Say so plainly, change nothing, and stop. Don't try to fix it, and don't set anything up.

```say
Something in this story's files can't be read, so I'm not going to change anything until that's sorted out.
```
