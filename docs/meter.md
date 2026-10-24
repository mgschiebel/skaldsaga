# Session meter

The meter tells the interviewer when the conversation's context window is getting full, so a session can end at a natural pause before the model loses sharpness. It is the first Rust subcommand and the pilot for the whole delivery chain.

## How it fits together

1. **First-run setup** wires the status line once:
   `skaldsaga meter wire --command "sh <plugin>/scripts/skaldsaga meter statusline"`
   This writes `statusLine` into the project's personal settings, `.claude/settings.local.json` (a plugin can't register it). That file stays out of git (setup's `.gitignore` lists it): the entry holds a path tied to this computer, and a status line is a command Claude Code runs on its own, so a shared one would run for anyone who clones the story. The shared `.claude/settings.json` is never edited. If the writer already has a status line, in either project file or in `~/.claude/settings.json` (looked for in that order), it is recorded in `.harness/settings.json` under `meter.wrapped_statusline` and keeps running: the writer still sees their own line. A meter entry left in the shared file by version 0.3.0 or earlier is not wrapped; the personal file overrides it.
2. **Claude Code** runs the status line after each turn. `meter statusline` records `.harness/meter.json` and prints a line.
3. **The interviewer** runs `skaldsaga meter check` after each saved answer and acts on `state`.
4. **The SessionStart hook** (`skaldsaga hook session-start`) clears the last reading when a fresh session starts, so a full reading from the last session can't end a new one early. It also re-points the personal status line at the current plugin folder after a plugin update. When the story is one level below the session folder (cloud sessions), it adds its context line only and touches nothing, since that folder's settings don't load. `SKALDSAGA_HOOK_DEBUG=1` makes it say what it found or why it stayed silent, on stderr and in Claude's context (Claude doesn't see hook stderr at session start). Setup (`init` and `meter wire`) also takes out a meter status line that 0.3.0 wrote into the shared `.claude/settings.json`, and lists that file under `meter.cleaned`.

Everything is guarded by `.harness/`: outside a Skaldsaga project, `statusline` prints an empty line and `reset` does nothing.

## Commands

| Command | Output | Notes |
|---|---|---|
| `meter statusline` | One status line | Reads Claude Code's JSON on stdin. Never fails visibly: always exits 0. |
| `meter check [--session ID] [--project DIR]` | JSON (below) | `--session` ignores a reading left by another session. |
| `meter reset [--project DIR]` | Nothing | Deletes `.harness/meter.json`. |
| `meter wire --command CMD [--project DIR] [--user-settings FILE]` | JSON `outcome` | `wired`, `updated`, or `already_wired`. |

`DIR` defaults to `$CLAUDE_PROJECT_DIR`, then the current folder. Exit codes: 0 success, 1 failure, 64 usage mistake, 69 subcommand not built yet.

### `meter check` output

```json
{
  "state": "suggest_wrap_up",
  "used_percentage": 55.2,
  "suggest_wrap_up_percent": 50,
  "wrap_up_percent": 70,
  "session_id": "…",
  "note": null
}
```

| `state` | Interviewer does |
|---|---|
| `ok` | Carries on. |
| `suggest_wrap_up` | Recommends wrapping up at the next natural pause; the writer may decline once. |
| `wrap_up` | Finishes the current question, saves, writes the recap, and tells the writer to start fresh. |
| `unknown` | Carries on; `note` says why (no reading yet, usage not reported yet, or a different session). The manual session boundary still works. |

## Settings

Thresholds live in `.harness/settings.json`, which is committed and shared:

```json
{
  "meter": {
    "suggest_wrap_up_percent": 50,
    "wrap_up_percent": 70
  }
}
```

The writer's own status line lives only in `.harness/local.json`, which stays out of git (setup adds it to `.harness/.gitignore`):

```json
{ "wrapped_statusline": { "type": "command", "command": "~/.claude/statusline.sh" } }
```

The meter never runs a command read from a committed file (SR-10). A `meter.wrapped_statusline` left in `settings.json` by version 0.3.2 or earlier is ignored; `init` and `meter wire` move it to `local.json` (unless that file already has one), take it out of `settings.json`, and list `.harness/settings.json` and `.harness/.gitignore` under `cleaned`. A `local.json` that is a symlink or over 64 KiB is ignored. Git history keeps any command that was committed before.

Values outside 1 to 100 fall back to the defaults, and a suggestion threshold above the hard one is lowered to match, so a hand-edited typo never stops the meter. Other keys in the file are preserved.

## The status line command

Setup and the session-start hook write the command as `sh '<plugin folder>/scripts/skaldsaga' meter statusline`, with the path in single quotes (any single quote in the path is escaped), so a plugin folder name with spaces, quotes, `$` or backticks stays plain text and nothing in it runs (SR-24). An older command with double quotes is replaced the next time the hook refreshes it.

## The launcher

`plugin/scripts/skaldsaga` (installed as `scripts/skaldsaga` in the plugin) picks `scripts/bin/<platform>/skaldsaga`, restores a missing executable bit, and (except for the status line, which must stay fast) checks the binary's version against `scripts/VERSION`. Always invoke it through `sh`, so it works even if a plugin install drops its executable bit. `SKALDSAGA_BIN` points it at another binary during development.
