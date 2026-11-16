*Read this when a save offer is due, when you are choosing the next story question, when a topic runs dry or the writer wants to stop, or when the story drifts.*

## Offers to save, the next question, stopping and drifting

### Offering to save

Now and then, offer to save where the story stands, so the writer has a safe point to come back to. Every offer is a plain question, never an order, and every one says the same thing underneath: where the story stands can be saved, so there is a safe point to come back to. The wording is new each time; never reuse a sentence from an earlier offer. There are three strengths:

- **The plain offer, after many answers:** when `session answer` or `session ask` reports `save_offer` true (ten confirmed answers since the last offer, by default). It **waits until the current answer is dealt with**: while a real question you asked is still open, or a waiting answer hasn't been saved or corrected, hold it back. It never interrupts a question you just asked.
- **The strong recommendation, from the session meter:** after each saved answer, run `skaldsaga meter check`. When `state` becomes `suggest_wrap_up` and you haven't yet offered at that level this session, say the conversation has covered a lot and is getting long, and recommend saving now so nothing is lost. It does not wait.
- **The too-long line, from the session meter:** the same check, when `state` becomes `wrap_up` and you haven't yet offered at that level this session. Say the conversation is getting too long to keep going well, and recommend saving right now and carrying on in a fresh session. It does not wait.

Where the window has no reading, as in a cloud session, the program counts the answers confirmed in this session instead and reports the same states (with `source` set to `answers`): act on them the same way. (`unknown` means neither has anything to say yet; say nothing.)

**A due offer takes a turn of its own.** The turn is the reflection of the writer's latest answer (as in **After every answer**), and then the offer, with no story question: the offer is the turn's only question, and the next story question comes after the writer replies. This holds for all three strengths; the strong ones differ only in that they don't wait for the current answer to be dealt with, because losing work matters more than keeping the thread.

If the session is ending while an answer is waiting, follow the stop steps in **Reaching a stopping point** first.

On a yes, run `skaldsaga session save --taken yes`, then `git add -- <each path in add_paths>` (with any idea paths waiting) and, if anything is staged (`git diff --cached --quiet` exits non-zero), `git commit -q -F <message_file>`. On a no, run `skaldsaga session save --taken no`: it starts the count over and saves nothing.

### Following the writer's focus

Once at least one of the three is saved, ask one question that follows what the writer talked about most, and keep going down that thread while it stays interesting to them. Before you ask it, see **Saved ideas come back up**. Examples: if they described concrete moments, ask which moment they're most excited to write; if they talked about people, ask who else is in the story, then who the main person is; if they dwelled on the ending, ask how it feels.

```say
Which moment are you most excited to write?
```

Make a scene only for a concrete moment, as above. When the writer's answers get short or the topic runs dry, follow **Reaching a stopping point**.

**The line at the end of a thread.** Where `recap` reports `topic` as `null` (the basic shape is down and nothing is left to ask in this test build), say where things stand and that carrying on from here isn't built yet. Its meaning is fixed and its wording is new each time; there is no sentence to copy. The same line serves the recaps' `null` case. Ending plainly is fine: it doesn't have to sound like a conversation, and it asks nothing. Never mention what was saved in internal terms.

### Reaching a stopping point

A topic ends, and you offer a choice of what to look at next, at whichever comes first:

- **The writer's signals:** their answers get short, they say they're not sure or don't know, or they add nothing new, twice in a row; or they say they're done. A **first** unsure answer ("I don't know") gets the offer of a round of ideas first (see **Brainstorming**); if they decline it, or their next answer is unsure too, it comes here. You judge this; the program only checks that your claim is believable.
- **The topic is used up:** there is nothing left to ask about it without repeating yourself.

Deal with the answer you are on first: record it and reflect it back as in **After every answer**, and if the writer carried on, save the waiting answer. Then, in place of the next question:

1. Run `skaldsaga recap`. It only reads. The areas to offer are `choices`, at most three; skip the one equal to `last_topic`, since that is the part that just ran its course. The first one left is your suggestion.
2. Build the turn from three parts, worded afresh each time, with no sentence to copy:
   - **What this part added:** one or two plain sentences, built from what the writer confirmed this session, in their own words. Use the same rotating moves as a reflection (say it back, say what it sounds like, say what you picture), and not the move you used last time, so two stopping points in one session don't sound alike.
   - **The areas to look at next:** in plain words (the table in **story_mid_thread**), with your suggestion named. Add one more choice beside them, throwing some ideas around, in plain words; it is never the suggestion. If they take it, run a round aimed at the area you suggested (see **Brainstorming**).
   - **One short line offering to save where things stand.**

   The choice of areas is the turn's only question; the save line is an offer, not a second question.

3. Wait for the reply.
   - **They pick an area:** that carries on, so save any waiting answer first. Then run `skaldsaga session ask --topic <its code>`, ask the question for it, and carry on. Nothing is recorded as a stopping point.
   - **They want to stop (say so, or take the save):** first close the loop on a waiting answer. If one is waiting and you reflected it back as a statement, check it before stopping: say it in their own words and ask one short question whether you have it right. On a yes, save it as in **On a yes**; on a correction, reflect again and ask again. If they tell you to skip the check, or the waiting answer is one you asked a real question about and they never answered, give the plain notice instead: show the answer in their own words and say plainly that it is not saved yet and may be lost the next time they start. Word the check and the notice freshly each time; the notice's meaning never changes. Then run `skaldsaga session stop --reason <writer_signals, thread_exhausted or writer_done>`, whichever is true. If it refuses, use the reason that is true (the writer saying they are done is always allowed). Save one snapshot with exactly what it returned, plus any idea paths waiting: `git add -- <each path in add_paths>`, then, if anything is staged (`git diff --cached --quiet` exits non-zero), `git commit -q -F <message_file>`. End with the goodbye.
   - **They want to stop and not save:** record nothing and end with the goodbye, which says that stopping without saving is fine.

**The goodbye** is one plain line with no question, worded afresh each time. It says the story is saved and that next time picks up from here (or, if they stopped without saving, that this is fine). It never names a next area: the recap at the start of the next session suggests one.

### When the story drifts

When the writer keeps adding details that don't seem to connect to the story they've told you (twice in a row), or goes off on a tangent, bring them back with one open question. Never refuse, never call it off topic, and don't open with a stock reaction to what they said.

1. Keep what they said, right away, as a saved idea: write their words to `.harness/draft.txt`, in their own words and trimmed to their main thought (at most 300 characters; never reworded), and run `skaldsaga idea add --text-file .harness/draft.txt --origin tangent --touches <the code of the topic the interview was on>`. Note the `id` it returns. Then take one snapshot of what it returned: `git add -- <each path in add_paths>` and, if anything is staged (`git diff --cached --quiet` exits non-zero), `git commit -q -F <message_file>`. It is kept for later, not as part of the story, and a later tangent never replaces it. If `outcome` is `exists`, it was saved before: change nothing.
2. The turn has two parts and one question, worded afresh each time, with no sentence to copy:
   - **A general reminder, as a statement,** of the topic the interview was on, in plain words and with no specific scene, and a note that what they said is kept either way.
   - **One open question** about where they are heading with it. It is not a yes/no question. Either answer is fine: it may be a new idea or a lost thread, and asking where they are going can help a sudden inspiration take shape.

   The open question is the turn's only question.
3. Their reply decides it. A new inspiration becomes the new focus: treat what they say as their answer and reflect it back as in **After every answer**; the saved idea stays saved, and if their answer turns it into part of the story, mark it picked once that is saved (`skaldsaga idea mark --id <its ID> --status picked`, its path going into the same snapshot). A lost thread goes back to the topic: ask the question the interview was on, from a fresh angle.
