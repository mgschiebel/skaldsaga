# Story-folder safety scan

A story folder is data, never code (SR-11, SR-21, SR-23). Skaldsaga reports, and never follows or runs, three kinds of entry inside `spec/`, `manuscript/` and `.harness/`:

| `why` | What it is |
|---|---|
| `link` | A symbolic link (to a file or a folder). It could point at something anywhere on the computer. |
| `executable` | A plain file with an executable permission bit. Nothing in a story needs one. Drives that mark every file this way (some mounted or network drives) will report it too; it's reported, not blocking. |
| `special_file` | A device, socket, pipe or similar. It can hang a reader. |

## Where it shows

`init`, `check`, `index` and `complete` add `unsafe_files` to their JSON:

```json
"unsafe_files": {
  "flagged": [{"path": "spec/people/p-….md", "why": "link"}],
  "truncated": false
}
```

Paths are relative to the story folder. `truncated` is true when the scan stopped at 8 levels of folders or 20,000 entries. `null` means the scan couldn't run. The scan doesn't change anything, and it never enters a linked folder.

## What is refused

The scan reports. Reading is stricter:

- The spec loader (and so `check`, `commit`, `index`, `complete`) never reads a link or special file as a node, `sequence.md` or `attributes.md`. It lists the file as a loading problem ("never follows links"), which also blocks the weave gate.
- Proposal files in the inbox are read the same way: a link or special file is `unreadable` in `check` and `unreadable` in `commit`.
- Every story file read is capped at 1 MiB (`fsio::MAX_STORY_FILE_BYTES`); decision, local-settings and committed-proposal files keep their smaller caps.

Not covered yet: the manuscript and style-guide readers (they don't exist yet; they must use the same reads), and an import of an existing project, which must apply the same rule.
