*Read this when the person typed `/skaldsaga check` or `/skaldsaga scenario <name>` (developer-only; never offer it to a writer).*

## Install check

Used when the person asks to check the install. Do steps 1 and 0 (find the folder), then step 2 and 3 only if the folder isn't set up yet and the person agrees, then:

1. Run `skaldsaga meter check` and report the `state` in plain words: `ok` (plenty of room), `suggest_wrap_up` (time to think about a break), `wrap_up` (time to stop and start fresh), or `unknown` (no reading yet; normal at the start of a session, or where the status line doesn't run). Where there is no reading, `source` `answers` means the count of confirmed answers in this session decided the state. Include `note` if there is one.
2. In three or four short lines, say: the program version; what setup did (folders added, or already in place) and whether the snapshot was saved; the meter state; and whether the session-start hook ran (it adds a line to your context containing "Skaldsaga project"; say yes or no without quoting it. It only runs when a session starts, so right after first-time setup it's normal for it to be missing).

## Scenarios

`/skaldsaga scenario <name>` is a developer-only test setup, for the person testing Skaldsaga (it is not part of the writer's experience, and you never offer it to a writer). The name after `scenario` picks the setup; the program knows the names. If there is no name, or `skaldsaga scenario <name>` says the name is unknown, tell them the names it lists (exit code 64, usage) and stop. The names are `basics` (follow **Scenario: basics** below) and `resume` (follow **Scenario: resume** below).

### Scenario: basics

The `basics` scenario files two sample answers, the way the interviewer will once the rest exists, and shows how Skaldsaga checks an answer before it changes the story, then saves it. Explain each step in a sentence or two, in plain words; the person is testing, so it's fine to say what each step proves, and fine to use the program's own words here.

1. **Program and folder:** do steps 0 and 1. If there's no `.harness` folder, do steps 2 and 3 first (setup and its snapshot), then come back here.
2. **File the sample answers:** run `skaldsaga scenario basics`. The result also names the next command and what to expect; the steps below already cover them.
   - `written`: continue.
   - `spec_not_empty`: the story already has content; say a scenario only runs in an empty story folder, so it never mixes sample content into a real story. Stop.
   - `already_written`: say this scenario has already run here. Stop.
3. **Check the first answer:** run `skaldsaga check --proposal pr-demstart`. Explain: the answer creates a tiny story (Mara, Jonah, the docks, a checkpoint scene and an arrest scene), and the check applied it to a copy of the story first. Expect `flagged` with `scene_untied` twice: the two sample scenes aren't tied to a storyline and a part of it, which is a gentle heads-up the writer decides on, not a refusal. Say so.
4. **Save it anyway:** run `skaldsaga commit --proposal pr-demstart`. Expect `held` (reason `needs_decision`): nothing changed, because the writer decides on a flag. For this scenario, keep it: run `skaldsaga commit --proposal pr-demstart --decision keep_anyway`. If the `outcome` is `committed`, save a git snapshot with exactly the paths it lists:

   ```sh
   git rm --cached --ignore-unmatch -q -- <each path in remove_paths>
   git add -- <each path in add_paths>
   git commit -q -F <message_file>
   ```

   Say the story files now hold the answer and the answer moved to the archive of saved answers.
5. **Check the second answer:** run `skaldsaga check --proposal pr-demmixup`. It has Mara already knowing about Jonah's arrest at the checkpoint, which happens before the arrest. Expect `failed` with the finding `learns_before_true`; explain it in your own plain words (a character can't know something before it happens).
6. **Try to save it anyway:** run `skaldsaga commit --proposal pr-demmixup`. Expect `held` with reason `failed`: nothing in the story changed. Then delete that sample answer from the inbox (the file `.harness/inbox/pr-demmixup.json`), so the folder holds no leftovers, and say so.
7. **Ask what's left:** run `skaldsaga complete`. It reports codes and IDs only, so describe the result in your own plain words. On this tiny story every one of the three gates (`weave`, `outline`, `complete`) is not ready, starting with `no_main_story`: there is no main plot yet, so nothing can be called finished. The last gate also lists the two sample scenes as `scene_untied`. Say that this is the expected answer, and that once a story has a main plot this check will list what still stands in the way. If the result has `remove_paths` (it only does when a lapsed acknowledgment was cleaned up), save a snapshot the same way as step 4 using its `message_file`; otherwise change nothing.
8. **Wrap up** in three or four short lines: what was saved, what was refused and why, what the final check said, and that the snapshot is in git. If the folder has a git remote (`git remote` prints something), offer to push the snapshot so it can be seen on GitHub; push only on a yes.

If git is missing or a commit fails, explain in one plain sentence; the story files are saved either way. Don't change git settings without asking.

### Scenario: resume

The `resume` scenario sets up the Jack and Jill story, left in the middle of something, so the mid-thread recap can be tried. It is a test setup, so it's fine to use the program's own words.

1. **Program and folder:** do steps 0 and 1. If there's no `.harness` folder, do steps 2 and 3 first, then come back here.
2. **Set it up:** run `skaldsaga scenario resume`.
   - `spec_not_empty`: say a scenario only runs in an empty story folder. Stop.
   - `already_written`: say this scenario has already run here. Stop.
   - `written`: save a snapshot so the folder is clean: `git add -- <each path in add_paths>`, then `git commit -q -F <message_file>` if anything is staged.
3. **Say what to try next, then stop.** Don't run the recap yourself; the test is a fresh session. Tell them to start a new session in this folder and type `/skaldsaga` with nothing after it, and what to expect (`expect` in the result): a short summary that names the story and Jack and Jill, the dark alley mentioned as something floated, one question about how the story begins, and nothing in the folder changed.
