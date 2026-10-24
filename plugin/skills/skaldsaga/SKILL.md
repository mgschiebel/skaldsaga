---
name: skaldsaga
description: Skaldsaga story mentor (test build). Use when the person types /skaldsaga or asks to start or test Skaldsaga.
---

# Skaldsaga (test build)

This is an early test build. The story conversation isn't built yet; this skill sets up a story folder, checks that the installed pieces work, and can run a short demo of how answers are checked and saved. Keep every reply short and in plain language, with no file paths or command names unless the person asks.

If the person typed `/skaldsaga demo` (or asks for the demo), follow **Demo** below instead of steps 2 to 5.

Below, `skaldsaga` means `sh "${CLAUDE_PLUGIN_ROOT}/scripts/skaldsaga"`, and `METER` means the text `sh "${CLAUDE_PLUGIN_ROOT}/scripts/skaldsaga" meter statusline` (passed as one quoted argument).

## 0. Find the story folder

Run `skaldsaga locate` and read `found` (in cloud sessions, a repository is cloned one level below the current folder):

- `here` or `nothing`: use the current folder.
- `below`: the story folder is `name`, one level down. Use it, and say so in one short line.
- `candidate`: no story yet, but the current folder holds only one git repository, `name`. Ask one question: "Should I use the `<name>` folder for your story?" On a no, use the current folder.
- `several`: more than one story folder is one level down (listed in `several`). Ask which one to use.

When you use a folder other than the current one, add `--project <its path>` (the `path` field) to every `skaldsaga` command below, and run every `git` command inside that folder.

## 1. Check the program

Run `skaldsaga --version`.

- If it prints a version, note it for the summary.
- If it fails, quote the message it printed and stop.

## 2. Set up this folder

Look for a `.harness` folder at the top of the current project.

- **It doesn't exist:** ask one question: "Want me to set this folder up for your story? I'll add four folders (your story notes, your writing, feedback, and style notes) and a small session meter that tells us when it's time for a break." Do nothing else until they answer. On a no, stop.
- **It exists**, or they said yes: run setup. It only adds what's missing and never replaces a file, so it's safe in a folder that's already set up.

  ```sh
  skaldsaga init --meter-command "METER"
  ```

Read the JSON it prints:

- `existing_files`: the folder already holds the person's own files (`existing` lists some of them). Say that bringing in writing they already have isn't built yet, and suggest opening an empty folder for now. Nothing was changed. Stop here.
- `already_set_up`: everything was in place. Skip to step 4, unless `meter.cleaned` lists a file; then do step 3 for that file only.
- `created`: setup added the folders. Continue with step 3.

The `meter` part says what happened to the session meter:

- `wired`: the meter is on. If `wrapped` isn't null, mention that their own status line still shows.
- `already_wired` or `updated`: the meter was already on.
- `cleaned` lists files an earlier version changed by mistake: it had put the meter's setting in the shared settings file, which goes into git. Setup has taken it out. Mention it in one short line.

The meter's setting is kept in a personal file that never goes into git; setup's `.gitignore` keeps it (and other personal or secret files) out.

## 3. Save a first snapshot with git

Skaldsaga keeps its history in git, so the person can always go back.

1. If the folder has no `.git` folder, run `git init -q`.
2. Add only the paths listed in `commit_paths`, never anything else in the folder:

   ```sh
   git add -- <each path in commit_paths>
   ```

   For each file in `meter.cleaned`, stage the change only if git already tracks that file (it may have been edited or removed):

   ```sh
   git ls-files --error-unmatch -- <path> >/dev/null 2>&1 && git add -A -- <path>
   ```

3. If anything is staged (`git diff --cached --quiet` exits non-zero), commit it. Use "Skaldsaga: set up the story folders" when setup added folders, or "Skaldsaga: move the session meter to personal settings" when only `meter.cleaned` changed:

   ```sh
   git commit -q -m "<message>"
   ```

If git is missing or the commit fails (for example, git doesn't know the person's name yet), explain the problem in one plain sentence and say the folders are set up anyway; the snapshot can be saved later. Don't change git settings without asking.

## 4. Meter check

Run `skaldsaga meter check`.

Report the `state` in plain words: `ok` (plenty of room), `suggest_wrap_up` (time to think about a break), `wrap_up` (time to stop and start fresh), or `unknown` (no reading yet; normal at the start of a session, or where the status line doesn't run). Include `note` if there is one.

## 5. Summary

In three or four short lines, say:

- the program version;
- what setup did (folders added, or already in place) and whether the snapshot was saved;
- the meter state;
- whether the session-start hook ran: it adds a line to your context containing "Skaldsaga project". Say yes or no; don't quote it. (It only runs when a session starts. Right after first-time setup it's normal for it to be missing; it runs in the next session that starts in the story folder or the folder just above it.)

## Demo

The demo files two sample answers, the way the interviewer will once it exists, and shows how Skaldsaga checks an answer before it changes the story, then saves it. Explain each step in a sentence or two, in plain words; the person is testing, so it's fine to say what each step proves.

1. **Program and folder:** do steps 0 and 1. If there's no `.harness` folder, do steps 2 and 3 first (setup and its snapshot), then come back here.
2. **File the sample answers:** run `skaldsaga demo`.
   - `written`: continue.
   - `spec_not_empty`: the story already has content; say the demo only runs in an empty story folder, so it never mixes sample content into a real story. Stop.
   - `already_written`: say the demo has already run here. Stop.
3. **Check the first answer:** run `skaldsaga check --proposal pr-demstart`. Explain: the answer creates a tiny story (Mara, Jonah, the docks, a checkpoint scene and an arrest scene), and the check applied it to a copy of the story first. Say whether it `passed`.
4. **Save it:** run `skaldsaga commit --proposal pr-demstart`. If the `outcome` is `committed`, save a git snapshot with exactly the paths it lists:

   ```sh
   git rm --cached --ignore-unmatch -q -- <each path in remove_paths>
   git add -- <each path in add_paths>
   git commit -q -m "<message>"
   ```

   Say the story files now hold the answer and the answer moved to the archive of saved answers.
5. **Check the second answer:** run `skaldsaga check --proposal pr-demmixup`. It has Mara already knowing about Jonah's arrest at the checkpoint, which happens before the arrest. Expect `failed` with the finding `learns_before_true`; explain it in your own plain words (a character can't know something before it happens).
6. **Try to save it anyway:** run `skaldsaga commit --proposal pr-demmixup`. Expect `held` with reason `failed`: nothing in the story changed. Then delete that sample answer from the inbox (the file `.harness/inbox/pr-demmixup.json`), so the folder holds no leftovers, and say so.
7. **Ask what's left:** run `skaldsaga complete`. It reports codes and IDs only, so describe the result in your own plain words. On this tiny story every one of the three gates (`weave`, `outline`, `complete`) is not ready, starting with `no_main_story`: there is no main plot yet, so nothing can be called finished. Say that this is the expected answer, and that once a story has a main plot this check will list what still stands in the way. If the result has `remove_paths` (it only does when a lapsed acknowledgment was cleaned up), save a snapshot the same way as step 4 using its `message`; otherwise change nothing.
8. **Wrap up** in three or four short lines: what was saved, what was refused and why, what the final check said, and that the snapshot is in git. If the folder has a git remote (`git remote` prints something), offer to push the snapshot so it can be seen on GitHub; push only on a yes.

If git is missing or a commit fails, explain in one plain sentence; the story files are saved either way. Don't change git settings without asking.
