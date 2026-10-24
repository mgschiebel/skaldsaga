# Committer and demo

`skaldsaga commit` is the single committer: it applies one checked proposal to the spec files, archives the proposal in `.harness/committed/`, and rebuilds the index. It reuses the operation engine `check` runs on, so the two can't disagree about what a proposal does. `skaldsaga demo` files two sample answers for trying both before the interviewer exists.

## Commands

| Command | Output | Notes |
|---|---|---|
| `commit --proposal ID [--decision keep_anyway\|not_sure] [--cleared] [--project DIR]` | JSON | Re-checks, then commits or holds. Refuses (exit 1) without `.harness/`; usage mistakes exit 64. |
| `demo [--project DIR]` | JSON `outcome` | `written`, `spec_not_empty` (the story already has content) or `already_written`. |

## What commit needs

Commit always re-checks against the spec as it is now, then:

| Fresh verdict and lane | Needs | Otherwise held with reason |
|---|---|---|
| failed | can't commit | `failed` |
| flagged | `--decision keep_anyway` or `not_sure` | `needs_decision` |
| strict lane | `--cleared` (the housekeeper cleared it), and nothing in `revisions.checked` changed since | `waiting_for_housekeeper`, `clearance_stale` |
| passed, optimistic | nothing | — |

A held proposal stays in the inbox with the fresh verdict and revisions written into it, as `check` would.

**Labels:** facts and shape decisions the proposal writes are tagged `unreviewed` (optimistic) or `cleared` (strict, after the housekeeper's clearance). With `not_sure`, any of them marked confirmed become provisional, and the archive records the flags as an open assumption to confirm.

## Output

```json
{
  "outcome": "committed",
  "id": "pr-demstart",
  "status": "passed", "lane": "optimistic", "fan_out": 0, "touches_confirmed": false, "findings": [],
  "decision": null,
  "files": ["spec/people/p-demmara2.md", "…", "spec/_index/status.json"],
  "add_paths": ["spec/people/p-demmara2.md", "…", ".harness/committed/pr-demstart.json"],
  "remove_paths": [".harness/inbox/pr-demstart.json"],
  "message": "Skaldsaga: Mara is a courier at the east docks. …"
}
```

Held: `{"outcome": "held", "id", "reason", "status", "lane", "fan_out", "touches_confirmed", "findings"}`. Also `not_found` and `unreadable` (with `message`).

The program writes files; the skill makes the git commit:

```sh
git rm --cached --ignore-unmatch -q -- <remove_paths>
git add -- <add_paths>
git commit -q -m "<message>"
```

`remove_paths` holds deleted node files and the inbox file, which git may never have seen (hence `--ignore-unmatch`). The index is ignored by git and never listed.

## Archive

`.harness/committed/<id>.json` is the proposal as committed, with its final `check` and `revisions.checked`, plus:

```json
"committed": {"lane": "optimistic", "decision": null, "open_assumption": false, "cleared": false, "committed_by": "skaldsaga 0.3.0"}
```

## Demo

Runs only in a set-up project whose story is empty. `pr-demstart` creates Mara, Jonah, the east docks, a checkpoint scene and an arrest scene that happens after it, with Jonah's arrest becoming true in the arrest scene; it passes. `pr-demmixup` has Mara learn about the arrest at the checkpoint, before it happens; it fails with `learns_before_true`. The skill's `/skaldsaga demo` walks through checking and committing both, then deletes the refused sample from the inbox.

## Proposal IDs

`--proposal` must be a proposal ID (`pr-` and eight characters); anything else, such as a path, exits 64 before any file is looked at. The same check sits inside the committer, which answers `not_found` for a malformed ID (SR-20).
