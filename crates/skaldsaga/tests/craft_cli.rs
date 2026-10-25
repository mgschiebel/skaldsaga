//! End-to-end tests for `skaldsaga craft ...`.
#![expect(
    clippy::unwrap_used,
    reason = "a test that cannot go on should fail loudly, and a test failure is a panic"
)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("skaldsaga-craft-test-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(path.join("novel/.harness")).unwrap();
        TempDir(path)
    }
    fn project(&self) -> PathBuf {
        self.0.join("novel")
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn craft(dir: &TempDir, args: &[&str]) -> Output {
    let project = dir.project();
    Command::new(env!("CARGO_BIN_EXE_skaldsaga"))
        .arg("craft")
        .args(args)
        .args(["--project", project.to_str().unwrap()])
        .env_remove("CLAUDE_PROJECT_DIR")
        .output()
        .unwrap()
}

fn shuffle(count: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_skaldsaga"))
        .args(["craft", "shuffle", "--count", count])
        .output()
        .unwrap()
}

fn json(o: &Output) -> serde_json::Value {
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    serde_json::from_slice(&o.stdout).unwrap()
}

fn code(o: &Output) -> Option<i32> {
    o.status.code()
}

fn concept(id: &str) -> String {
    format!(
        r#"{{"id":"{id}","term":"unreliable narrator","explanation":"A narrator the reader learns not to trust.","builds_on":["pov_character"],"sources":["A craft article"],"examples":["Mara hides the letter."]}}"#
    )
}

#[test]
fn teaching_and_demonstrating_report_paths_for_git() {
    let dir = TempDir::new();
    let o = json(&craft(&dir, &["teach", "--concept", "main_character"]));
    assert_eq!(o["outcome"], "taught");
    assert_eq!(o["add_paths"][0], ".harness/writer/craft.json");
    let msg = o["message_file"].as_str().unwrap();
    assert!(msg.ends_with("commit-message.txt"), "{msg}");

    // Teaching again changes nothing and asks for no commit.
    let again = json(&craft(&dir, &["teach", "--concept", "main_character"]));
    assert_eq!(again["outcome"], "unchanged");
    assert_eq!(again["add_paths"].as_array().unwrap().len(), 0);

    let d = json(&craft(
        &dir,
        &[
            "demonstrate",
            "--concept",
            "main_character",
            "--evidence",
            "p-aaaaaaaa",
        ],
    ));
    assert_eq!(d["outcome"], "demonstrated");

    let status = json(&craft(&dir, &["status"]));
    let mc = status["concepts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "main_character")
        .unwrap();
    assert_eq!(mc["level"], "demonstrated");
    assert_eq!(mc["term_use"], "short");
    let pov = status["concepts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "pov_character")
        .unwrap();
    assert_eq!(pov["term_use"], "explain");
}

#[test]
fn the_writers_word_and_bad_input_are_refused() {
    let dir = TempDir::new();
    let p = dir.project();
    // No evidence, bad evidence, and a path-like concept ID are usage errors.
    for args in [
        vec!["demonstrate", "--concept", "foil", "--evidence", ""],
        vec!["demonstrate", "--concept", "foil", "--evidence", "../x"],
        vec!["teach", "--concept", "../../etc/passwd"],
        vec!["teach", "--concept", "nonesuch"],
        vec!["teach", "--concept", "foil", "--via", "magic"],
        vec!["teaching", "--level", "lots"],
        vec![
            "experience",
            "--written-before",
            "maybe",
            "--read-craft-books",
            "no",
        ],
        vec!["bogus"],
    ] {
        assert_eq!(code(&craft(&dir, &args)), Some(64), "{args:?}");
    }
    assert!(!p.join(".harness/writer").exists());
}

#[test]
fn misconceptions_come_from_a_file_and_are_kept() {
    let dir = TempDir::new();
    let words = dir.project().join("words.txt");
    std::fs::write(&words, "thought a foil is any villain; $(touch /tmp/x)").unwrap();
    let w = words.to_str().unwrap();
    let o = json(&craft(
        &dir,
        &[
            "misconception",
            "--concept",
            "foil",
            "--words-file",
            w,
            "--corrected",
            "yes",
        ],
    ));
    assert_eq!(o["outcome"], "misconception_noted");
    let saved = std::fs::read_to_string(dir.project().join(".harness/writer/craft.json")).unwrap();
    assert!(saved.contains("any villain"));

    let missing = dir.project().join("nope.txt");
    assert_eq!(
        code(&craft(
            &dir,
            &[
                "misconception",
                "--concept",
                "foil",
                "--words-file",
                missing.to_str().unwrap()
            ]
        )),
        Some(64)
    );
}

#[cfg(unix)]
#[test]
fn a_linked_words_file_is_refused() {
    let dir = TempDir::new();
    let real = dir.project().join("real.txt");
    std::fs::write(&real, "secret").unwrap();
    let link = dir.project().join("link.txt");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    assert_eq!(
        code(&craft(
            &dir,
            &[
                "misconception",
                "--concept",
                "foil",
                "--words-file",
                link.to_str().unwrap()
            ]
        )),
        Some(64)
    );
}

#[test]
fn preferences_and_experience() {
    let dir = TempDir::new();
    assert_eq!(
        json(&craft(&dir, &["teaching", "--level", "less"]))["outcome"],
        "teaching_set"
    );
    assert_eq!(
        json(&craft(
            &dir,
            &[
                "experience",
                "--written-before",
                "yes",
                "--read-craft-books",
                "no"
            ]
        ))["outcome"],
        "experience_noted"
    );
    let status = json(&craft(&dir, &["status"]));
    assert_eq!(status["record"]["teaching"], "less");
    assert_eq!(status["record"]["experience"]["written_before"], true);
}

#[test]
fn researched_concepts_are_stored_and_exported_without_examples() {
    let dir = TempDir::new();
    let file = dir.project().join("concept.json");
    std::fs::write(&file, concept("unreliable_narrator")).unwrap();
    let f = file.to_str().unwrap();
    let added = json(&craft(&dir, &["research-add", "--file", f]));
    assert_eq!(added["outcome"], "concept_added");
    assert_eq!(
        added["add_paths"][0],
        ".harness/writer/concepts/unreliable_narrator.json"
    );
    // The same ID twice is refused.
    assert_eq!(code(&craft(&dir, &["research-add", "--file", f])), Some(64));

    let exported = craft(&dir, &["export"]);
    let text = String::from_utf8_lossy(&exported.stdout).into_owned();
    assert!(text.contains("unreliable_narrator"));
    assert!(text.contains("\"provisional\": true"));
    assert!(!text.contains("Mara"), "{text}");

    let status = json(&craft(&dir, &["status"]));
    assert!(
        status["concepts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == "unreliable_narrator" && c["origin"] == "researched")
    );

    // A file with an unknown field or a hostile ID is refused.
    std::fs::write(&file, concept("../evil")).unwrap();
    assert_eq!(code(&craft(&dir, &["research-add", "--file", f])), Some(64));
    std::fs::write(
        &file,
        r#"{"id":"ok_one","term":"t","explanation":"e","run":"x"}"#,
    )
    .unwrap();
    assert_eq!(code(&craft(&dir, &["research-add", "--file", f])), Some(64));
}

#[test]
fn shuffle_gives_a_permutation_within_bounds() {
    let o = json(&shuffle("4"));
    let mut order: Vec<u64> = o["order"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap())
        .collect();
    order.sort_unstable();
    assert_eq!(order, vec![0, 1, 2, 3]);
    for bad in ["1", "13", "x", "-2"] {
        assert_eq!(code(&shuffle(bad)), Some(64), "{bad}");
    }
}

#[test]
fn a_broken_record_is_a_clear_failure_and_is_left_alone() {
    let dir = TempDir::new();
    let path = dir.project().join(".harness/writer/craft.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "{ not json").unwrap();
    assert_eq!(code(&craft(&dir, &["status"])), Some(64));
    assert_eq!(
        code(&craft(&dir, &["teach", "--concept", "foil"])),
        Some(64)
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "{ not json");
    let _: &Path = &path;
}
