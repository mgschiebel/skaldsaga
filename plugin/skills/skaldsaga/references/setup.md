*Read this when a new story has been agreed and the folder isn't set up yet (steps 1 to 3), before running any of them.*

## 1. Check the program

Run `skaldsaga --version`.

- If it prints a version, note it for the summary.
- If it fails, quote the message it printed and stop.

## 2. Set up this folder

Only after the writer has agreed to a new story (see the entries above), run setup. It only adds what's missing and never replaces a file, so it's safe in a folder that's already set up.

```sh
skaldsaga init --meter-command "METER"
```

Read the JSON it prints:

- `existing_files`: the folder already holds the person's own files (`existing` lists some of them). Nothing was changed. Stop here.

```say
This folder already has files of your own in it, and bringing in writing you already have isn't built yet. For now, an empty folder works best.
```

- `already_set_up`: everything was in place. Skip to step 3 only if `meter.cleaned` lists a file (do step 3 for that file only); otherwise go on to the opener.
- `created`: setup added the folders. Continue with step 3.

The `meter` part says what happened to the session meter:

- `wired`: the meter is on. If `wrapped` isn't null, mention that their own status line still shows.
- `already_wired` or `updated`: the meter was already on.
- `cleaned` lists files an earlier version changed by mistake: it had put a meter setting where git shares it. Setup has moved it to a file that stays on this computer. Mention it in one short line.

The meter's setting is kept in a personal file that never goes into git; setup's `.gitignore` keeps it (and other personal or secret files) out.

Setup, `check`, `index` and `complete` also print `unsafe_files`. If its `flagged` list is not empty, tell the person in plain words that some files in the story folder are links, programs or special files, which Skaldsaga never opens or runs, and name up to three of the `path` values. Say what to do: replace a link with the real file, or remove the program permission (`why` is `executable`, which can also come from a drive that marks every file that way). Don't change or delete those files yourself. If `truncated` is true, say the scan stopped early because the folder is very large. If `unsafe_files` is null or the list is empty, say nothing.

## 3. Save a first snapshot with git

Skaldsaga keeps its history in git, so the person can always go back.

1. If the folder has no `.git` folder, run `git init -q`.
2. Add only the paths listed in `commit_paths`, never anything else in the folder:

   ```sh
   git add -- <each path in commit_paths>
   ```

   For each file in `meter.cleaned`, stage the change only if git already tracks that file (it may have been edited or removed):

   ```sh
   git ls-files --error-unmatch -- <path> >/dev/null 2>&1 && git add -- <path>
   ```

3. If anything is staged (`git diff --cached --quiet` exits non-zero), commit it. Use exactly "Skaldsaga: set up the story folders" when setup added folders, or exactly "Skaldsaga: move the session meter to personal settings" when only `meter.cleaned` changed (these two fixed messages are the only ones you type yourself):

   ```sh
   git commit -q -m "Skaldsaga: set up the story folders"
   git commit -q -m "Skaldsaga: move the session meter to personal settings"
   ```

   (Run only the one that applies.)

If git is missing or the commit fails (for example, git doesn't know the person's name yet), explain the problem in one plain sentence and say the folders are set up anyway; the snapshot can be saved later. Don't change git settings without asking.

After step 3 the writer has a story folder, so go on to **the opener**. The writer doesn't need a summary of setup; one short line that the folder is ready is enough.
