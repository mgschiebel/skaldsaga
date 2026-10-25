# Interview evals for the opener and the mid-thread recap

Each folder is one case in the layout `claude plugin eval` reads (see the skill-creator notes): a prompt, an expected outcome and one-sentence graders. They score **properties, not exact words**: one question per turn, no internal words in anything the writer sees, a reword before anything is saved, no setup before the writer confirms a new story, and only confirmed facts saved.

Three of the cases (`mid-thread-*` and `save-offer-*`) need the `skaldsaga` program: put it on PATH or set `SKALDSAGA_BIN` to it; their setup scripts use the `resume` scenario.

Entry routing itself is deterministic and covered by script tests (`crates/skaldsaga/tests/entry_cli.rs`), not here. A script test also scans every writer-facing line in `SKILL.md` for internal words (`crates/skaldsaga/tests/skill_prompt.rs`).

Run from this skill's folder: `claude plugin eval . --allow-tools Write,Edit,Bash --scaffold`. These are real, billed model runs. This folder is left out of the installable plugin.
