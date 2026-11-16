//! SR-30, SR-33 and the one-writer rule: each agent record folder has exactly
//! one command that writes in it. The owner is the command, not the process, so
//! this is checked in the source: only `verdict.rs` may build a path in
//! `spec/verdicts/`, only `question.rs` (a Continuity-style agent's questions)
//! and `asked.rs` (the Interviewer's record of what happened to them) may build
//! one in `spec/queue/`, and `asked.rs` can only reach the Interviewer's own
//! folder. Like `guarded_writes.rs`, it reads the production source (the code
//! above each file's `#[cfg(test)]` line), so a later change that lets another
//! module write into an agent's folder fails here.
#![expect(
    clippy::unwrap_used,
    reason = "a test that cannot go on should fail loudly, and a test failure is a panic"
)]

use std::collections::BTreeMap;
use std::path::PathBuf;

/// Production source of every file in the core crate, by file name.
fn sources() -> BTreeMap<String, String> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../skaldsaga-core/src");
    let mut out = BTreeMap::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "rs") {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let text = std::fs::read_to_string(&path).unwrap();
            let mut production = String::new();
            for line in text.lines() {
                if line.trim() == "#[cfg(test)]" {
                    break;
                }
                if !line.trim_start().starts_with("//") {
                    production.push_str(line);
                    production.push('\n');
                }
            }
            out.insert(name, production);
        }
    }
    out
}

/// The files whose production code mentions `needle`.
fn files_using(sources: &BTreeMap<String, String>, needle: &str) -> Vec<String> {
    sources
        .iter()
        .filter(|(_, text)| text.contains(needle))
        .map(|(name, _)| name.clone())
        .collect()
}

fn count(sources: &BTreeMap<String, String>, file: &str, needle: &str) -> usize {
    sources.get(file).unwrap().matches(needle).count()
}

#[test]
fn only_the_verdict_command_builds_a_path_in_the_verdicts_folder() {
    let src = sources();
    assert_eq!(
        files_using(&src, "VERDICTS_FOLDER"),
        ["records.rs", "verdict.rs"]
    );
}

#[test]
fn only_the_question_commands_build_a_path_in_the_queue_folder() {
    let src = sources();
    assert_eq!(
        files_using(&src, "QUEUE_FOLDER"),
        ["asked.rs", "question.rs", "records.rs"]
    );
}

#[test]
fn the_folder_names_are_written_out_in_one_place_only() {
    let src = sources();
    for literal in ["\"verdicts\"", "\"queue\""] {
        assert_eq!(files_using(&src, literal), ["records.rs"], "{literal}");
    }
    // The front end never names an agent's folder; it passes IDs and a file.
    let main =
        std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/main.rs"))
            .unwrap();
    for needle in [
        "spec/verdicts",
        "spec/queue",
        "VERDICTS_FOLDER",
        "QUEUE_FOLDER",
    ] {
        assert!(!main.contains(needle), "main.rs names {needle}");
    }
}

#[test]
fn each_record_command_writes_one_kind_of_file_through_the_guarded_writer() {
    let src = sources();
    for file in ["verdict.rs", "question.rs", "asked.rs"] {
        assert_eq!(count(&src, file, ".write_atomic("), 1, "{file}");
        assert_eq!(count(&src, file, ".ensure_dir("), 1, "{file}");
    }
    // Nothing else in the record layer writes: the shared module only reads
    // and renders, and the agent list is data.
    for file in ["records.rs", "agents.rs"] {
        for needle in [".write_atomic(", ".ensure_dir(", "fs::"] {
            assert_eq!(count(&src, file, needle), 0, "{file}: {needle}");
        }
    }
}

#[test]
fn the_interviewer_can_only_reach_its_own_queue_folder() {
    let src = sources();
    let asked = src.get("asked.rs").unwrap();
    assert!(asked.contains("INTERVIEWER"));
    // It never looks an agent up or loops over the registry, so it cannot be
    // pointed at another agent's folder, and it takes no agent name.
    for needle in ["agents::find", "AGENTS", "agent_name", "names()"] {
        assert!(!asked.contains(needle), "asked.rs uses {needle}");
    }
    // The agents that raise questions and verdicts never name the Interviewer's folder.
    for file in ["question.rs", "verdict.rs"] {
        assert!(!src.get(file).unwrap().contains("INTERVIEWER"), "{file}");
    }
}

#[test]
fn no_other_module_writes_a_record() {
    let src = sources();
    for (name, text) in &src {
        if ["verdict.rs", "question.rs", "asked.rs", "records.rs"].contains(&name.as_str()) {
            continue;
        }
        for needle in [
            "verdict::add",
            "question::add",
            "asked::mark",
            "records::render",
        ] {
            assert!(!text.contains(needle), "{name} calls {needle}");
        }
    }
}
