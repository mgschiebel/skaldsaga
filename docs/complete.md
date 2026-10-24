# Completeness check

`skaldsaga complete` answers one question: how close is the whole story to done? It reads the story as it stands, changes nothing about it, and reports **codes and IDs only**. Wording for the writer is the agents' job, never the program's.

## Commands

| Command | Output | Notes |
|---|---|---|
| `complete [--project DIR]` | JSON report | Refuses (exit 1) without `.harness/`. Deletes acknowledgments whose finding is resolved (see below). |
| `decisions acknowledge --code CODE --words-file FILE [--ids ID,ID] [--detail TEXT] [--project DIR]` | JSON | Records the writer saying "that one is fine". The words come from a file (16 KiB cap, symlinks refused), never from the command line. Acknowledging the same finding again renews the same file. Exit 64 if the story has no such finding. |
| `decisions withdraw --decision ID [--project DIR]` | JSON | Deletes one acknowledgment. `not_found` if it is already gone. |

`acknowledge` returns `add_paths` and `message`; `withdraw` returns `remove_paths` and `message`; `complete` returns `remove_paths` and, when that list is not empty, `message`. As with `commit`, the skill makes the git commit from exactly those paths and the program never runs git.

## Gates

Three questions, each with the blockers that keep it from a yes. The lists are cumulative: outline includes weave's blockers, complete includes outline's.

| Gate | Asks | Blocker codes |
|---|---|---|
| `weave` | Is the material ready to be woven into an outline? | `no_main_story`, `spec_problems`, `hard_findings`, `dramatic_question_open`, `thematic_question_open`, `decision_open`, `fog_subplot_open`, `open_findings` |
| `outline` | Is the outline ready? | the above, plus `landmark_open`, `landmarks_out_of_order` |
| `complete` | Is the story complete? | the above, plus `grid_cell_empty`, `unresolved_fact` |

A blocker has a `code` and, where it applies, `arc`, `ids`, `detail` and `count`. `open_findings` counts soft findings nobody has answered (open or resurfaced). Challenger and relationship threads whose decisions are all `not_explored` are exempt from the coverage grid.

Four landmarks block the outline gate: `mc_commits`, `midpoint`, `low_point` and `climax`. A provisional placement counts. The other five landmarks only produce the soft finding `landmark_missing`.

## Soft checks

Each check has a versioned code. A check only starts to count once the story can support it (it is "dormant" before that, and an acknowledgment of one of its findings is never cleaned up just because the check is quiet), so a young story is not buried in findings.

| Code | Version | Flags | Starts counting when |
|---|---|---|---|
| `unearned_twist` | 1 | A reveal with no earlier hint. | always |
| `dangling_setup` | 1 | A hint never revealed or used. | the main plot has a Resolution scene |
| `early_pivot` | 1 | The pivot belief of a "changes" decision ends well before the climax. | the climax is placed |
| `subplot_after_climax` | 1 | A subplot that resolves after the main climax. | the main climax is placed |
| `scene_count_pacing` | 1 | A movement with far too few or too many scenes (each expects a quarter for now). | the arc's three movement landmarks are placed in order |
| `threads_out_of_alignment` | 1 | A thread stage whose scenes mostly sit in a different movement. | same as pacing |
| `similar_attribute_keys` | 1 | Two attribute keys that look like a slip of each other (the starter vocabulary is never flagged against itself). | always |
| `landmark_missing` | 1 | One of the five soft landmarks is not placed in the main arc. Never blocks. | always |

### Thresholds (starting values, all in `.harness/settings.json`)

| Setting | Default | Used by |
|---|---|---|
| `pacing.under_share` | 0.5 | Fewer scenes than this share of the expected quarter per movement. |
| `pacing.over_share` | 2.0 | More scenes than this multiple of the expected quarter. |
| `checks.early_pivot_share` | 0.25 | Gap between pivot end and climax, as a share of the reading order. |
| `checks.misaligned_share` | 0.5 | Share of a thread stage's scenes in another movement. |
| `checks.similar_key_max_distance` | 2 | Largest edit distance between two keys (underscores ignored) to count as similar. |

Values outside their allowed range fall back to the default.

## Report

```json
{
  "outcome": "reported",
  "spec_problems": 0,
  "hard_findings": [{"code": "…", "ids": ["…"]}],
  "gates": {
    "weave": {"ready": false, "blockers": [{"code": "open_findings", "count": 1}]},
    "outline": {"ready": false, "blockers": []},
    "complete": {"ready": false, "blockers": []}
  },
  "findings": [
    {"code": "threads_out_of_alignment", "version": 1, "severity": "soft", "ids": ["a-…", "t-…"],
     "detail": "crisis", "data": {}, "gates": true, "status": "open",
     "fan_out": 3, "touches_confirmed": false}
  ],
  "assumptions": ["pr-…"],
  "acknowledgments": {"active": 0, "removed": [], "cleanup_skipped": false},
  "decision_problems": [{"path": "…", "message": "…"}],
  "remove_paths": []
}
```

With no main story, every gate is blocked by `no_main_story` and nothing else is reported.

Findings are ranked: those resting on a confirmed shape decision first, then by how many other nodes read what they rest on (`fan_out`). `gates` says whether the finding counts toward `open_findings`. `assumptions` are "not sure yet" answers (proposal IDs) still open.

## Acknowledgments

When the writer says a finding is fine, one file records it at `.harness/decisions/finding_ack/<id>.json` (ID prefix `wd-`).

```json
{"format": 1, "kind": "finding_ack", "id": "wd-…", "words": "…", "rests_on": {"<node id>": <revision>},
 "status": "active", "date": "2026-10-24",
 "finding": {"code": "…", "version": 1, "scope": "canon", "ids": ["…"], "detail": "…"}}
```

- A finding's identity is its code, check version, scope (`canon`), IDs and detail. Findings that depend on counts (pacing, alignment) put the movement or direction in `detail`, so a changed situation is a different finding and asks again.
- **Status** is `open`, `acknowledged` or `resurfaced`. An acknowledgment **lapses** when a node it rests on changes revision: the finding comes back as `resurfaced`, with the changed IDs in `changed`, and counts as open again. It never stays silent.
- A finding that no longer exists gets its acknowledgment file deleted by `complete` (`removed`, `remove_paths`). Cleanup is skipped when the spec has loading problems (`cleanup_skipped`), since a finding may be missing only because a file didn't load. Dormant checks and codes this version doesn't know are left alone.
- Files are size-capped at 64 KiB, symlinks are refused, and a bad file is reported in `decision_problems` and skipped. Decision IDs are checked with `ids::is_kind` before they become paths.
- Decision files are data. Nothing in them is ever run or followed as an instruction.
