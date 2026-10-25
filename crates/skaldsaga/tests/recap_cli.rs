//! End-to-end tests for `skaldsaga recap`, `session ask`, `session save` and
//! the `resume` scenario (slice 2, the mid-thread recap). The ranking rule has
//! its own unit tests; these check the real binary: that the recap only reads,
//! that the marker never makes a snapshot of its own, and that the save offer
//! comes due at the setting.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("skaldsaga-recap-test-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        TempDir(path)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn skaldsaga(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_skaldsaga"))
        .env_remove("CLAUDE_PROJECT_DIR")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}

fn json_of(o: &Output) -> serde_json::Value {
    assert!(
        o.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    serde_json::from_slice(&o.stdout).unwrap()
}

fn words_file(dir: &Path, text: &str) -> String {
    let path = dir.join("words.txt");
    std::fs::write(&path, text).unwrap();
    path.display().to_string()
}

/// Every file under `dir` with its bytes, to prove a command changed nothing.
fn snapshot(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let rel = path.strip_prefix(root).unwrap().display().to_string();
                out.insert(rel, std::fs::read(&path).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

/// A set-up folder with the `resume` scenario in place.
fn resumed() -> TempDir {
    let dir = TempDir::new();
    let p = dir.path();
    assert!(skaldsaga(p, &["init"]).status.success());
    let v = json_of(&skaldsaga(p, &["scenario", "resume"]));
    assert_eq!(v["outcome"], "written");
    dir
}

#[test]
fn the_resume_scenario_is_a_story_left_mid_thread() {
    let dir = resumed();
    let p = dir.path();
    let again = json_of(&skaldsaga(p, &["scenario", "resume"]));
    assert_eq!(again["outcome"], "already_written");

    let route = json_of(&skaldsaga(p, &["route", "--text", "no"]));
    assert_eq!(route["condition"], "story_mid_thread");
    assert_eq!(route["built"], true);
}

#[test]
fn the_resume_scenario_hands_back_paths_and_a_message_for_git() {
    let dir = TempDir::new();
    let p = dir.path();
    assert!(skaldsaga(p, &["init"]).status.success());
    let v = json_of(&skaldsaga(p, &["scenario", "resume"]));
    assert!(
        v["next_command"]
            .as_str()
            .unwrap()
            .starts_with("/skaldsaga")
    );
    let paths: Vec<&str> = v["add_paths"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
    assert!(paths.contains(&".harness/writer/last-session.json"));
    assert!(paths.iter().any(|x| x.starts_with("spec/")));
    for path in &paths {
        assert!(p.join(path).is_file(), "{path} should exist");
    }
    assert!(p.join(v["message_file"].as_str().unwrap()).is_file());
}

#[test]
fn a_scenario_only_runs_in_an_empty_story() {
    let dir = resumed();
    let p = dir.path();
    // The inbox and marker are gone; the spec is not empty.
    std::fs::remove_file(p.join(".harness/writer/last-session.json")).unwrap();
    let v = json_of(&skaldsaga(p, &["scenario", "resume"]));
    assert_eq!(v["outcome"], "spec_not_empty");
}

#[test]
fn the_recap_names_the_story_and_the_people_and_asks_how_it_begins() {
    let dir = resumed();
    let p = dir.path();
    let v = json_of(&skaldsaga(p, &["recap"]));
    assert_eq!(v["outcome"], "reported");
    assert_eq!(
        v["premise"],
        "A modern Jack and Jill, set in a city at night"
    );
    assert_eq!(v["people"], serde_json::json!(["Jack", "Jill"]));
    assert_eq!(v["loose_idea"], "maybe they enter a dark alley");
    assert_eq!(v["last_topic"], "people");
    assert_eq!(v["topic"], "setup");
    assert_eq!(
        v["stages_without_summary"],
        serde_json::json!(["setup", "complications", "crisis", "resolution"])
    );
    assert_eq!(v["save_offer"], false);
    // Facts and a topic code only: the program never prints a question.
    assert!(v.get("question").is_none());
}

#[test]
fn the_recap_changes_nothing_on_disk() {
    let dir = resumed();
    let p = dir.path();
    let before = snapshot(p);
    for _ in 0..2 {
        assert!(skaldsaga(p, &["recap"]).status.success());
        assert!(skaldsaga(p, &["route", "--text", "no"]).status.success());
    }
    assert_eq!(snapshot(p), before);
}

#[test]
fn asking_records_the_topic_and_an_unanswered_one_is_asked_again() {
    let dir = resumed();
    let p = dir.path();
    let v = json_of(&skaldsaga(
        p,
        &["session", "ask", "--topic", "complications"],
    ));
    assert_eq!(v["outcome"], "recorded");
    assert_eq!(
        v["add_paths"],
        serde_json::json!([".harness/writer/last-session.json"])
    );
    assert!(
        v.get("message_file").is_none(),
        "a question never gets a snapshot of its own"
    );
    let r = json_of(&skaldsaga(p, &["recap"]));
    assert_eq!(r["last_topic"], "complications");
    assert_eq!(r["topic"], "complications");
    // The loose idea is still there beside the new question.
    assert_eq!(r["loose_idea"], "maybe they enter a dark alley");

    // Asking the same thing again changes nothing.
    let again = json_of(&skaldsaga(
        p,
        &["session", "ask", "--topic", "complications"],
    ));
    assert_eq!(again["outcome"], "unchanged");
}

#[test]
fn bad_topics_and_asking_at_a_stopping_point_are_usage_errors() {
    let dir = resumed();
    let p = dir.path();
    let bad = skaldsaga(p, &["session", "ask", "--topic", "themes"]);
    assert_eq!(bad.status.code(), Some(64));
    assert_eq!(skaldsaga(p, &["session", "ask"]).status.code(), Some(64));
    assert!(skaldsaga(p, &["session", "stop"]).status.success());
    let before = snapshot(p);
    let late = skaldsaga(p, &["session", "ask", "--topic", "setup"]);
    assert_eq!(late.status.code(), Some(64));
    assert_eq!(snapshot(p), before);
}

#[test]
fn a_stop_clears_the_topic_and_returns_a_snapshot_message() {
    let dir = resumed();
    let p = dir.path();
    let v = json_of(&skaldsaga(p, &["session", "stop"]));
    assert_eq!(v["outcome"], "recorded");
    assert!(p.join(v["message_file"].as_str().unwrap()).is_file());
    let r = json_of(&skaldsaga(p, &["recap"]));
    assert_eq!(r["last_topic"], serde_json::Value::Null);
    assert_eq!(r["loose_idea"], serde_json::Value::Null);
}

#[test]
fn the_save_offer_comes_due_after_ten_confirmed_answers_and_can_start_over() {
    let dir = resumed();
    let p = dir.path();
    for n in 1..=10 {
        let words = words_file(p, &format!("answer number {n}"));
        for confirmed in ["no", "yes"] {
            let v = json_of(&skaldsaga(
                p,
                &[
                    "session",
                    "answer",
                    "--words-file",
                    &words,
                    "--confirmed",
                    confirmed,
                ],
            ));
            assert!(v.get("message_file").is_none());
        }
        let r = json_of(&skaldsaga(p, &["recap"]));
        assert_eq!(r["answers_since_offer"], n);
        assert_eq!(r["save_offer"], n >= 10, "after {n} answers");
    }

    // Declining starts the count over and saves nothing.
    let declined = json_of(&skaldsaga(p, &["session", "save", "--taken", "no"]));
    assert_eq!(declined["outcome"], "recorded");
    assert!(declined.get("message_file").is_none());
    let r = json_of(&skaldsaga(p, &["recap"]));
    assert_eq!(r["answers_since_offer"], 0);
    assert_eq!(r["save_offer"], false);
}

#[test]
fn taking_the_offer_hands_back_the_marker_and_a_message_for_a_safe_point() {
    let dir = resumed();
    let p = dir.path();
    let v = json_of(&skaldsaga(p, &["session", "save", "--taken", "yes"]));
    assert_eq!(
        v["add_paths"],
        serde_json::json!([".harness/writer/last-session.json"])
    );
    assert!(p.join(v["message_file"].as_str().unwrap()).is_file());
    assert_eq!(
        skaldsaga(p, &["session", "save"]).status.code(),
        Some(64),
        "--taken is required"
    );
}

#[test]
fn the_save_offer_count_is_a_setting() {
    let dir = resumed();
    let p = dir.path();
    let settings = p.join(".harness/settings.json");
    let text = std::fs::read_to_string(&settings).unwrap();
    assert!(text.contains("\"save_offer_after_answers\": 10"), "{text}");
    std::fs::write(
        &settings,
        text.replace(
            "\"save_offer_after_answers\": 10",
            "\"save_offer_after_answers\": 2",
        ),
    )
    .unwrap();
    for n in 1..=2 {
        let words = words_file(p, &format!("answer {n}"));
        for confirmed in ["no", "yes"] {
            assert!(
                skaldsaga(
                    p,
                    &[
                        "session",
                        "answer",
                        "--words-file",
                        &words,
                        "--confirmed",
                        confirmed,
                    ],
                )
                .status
                .success()
            );
        }
    }
    let r = json_of(&skaldsaga(p, &["recap"]));
    assert_eq!(r["save_offer_after"], 2);
    assert_eq!(r["save_offer"], true);
}

#[test]
fn a_story_with_content_and_no_marker_still_has_a_recap() {
    let dir = resumed();
    let p = dir.path();
    std::fs::remove_file(p.join(".harness/writer/last-session.json")).unwrap();
    let r = json_of(&skaldsaga(p, &["recap"]));
    assert_eq!(r["loose_idea"], serde_json::Value::Null);
    assert_eq!(r["last_topic"], serde_json::Value::Null);
    assert_eq!(r["topic"], "setup");
    let route = json_of(&skaldsaga(p, &["route", "--text", "no"]));
    assert_eq!(route["condition"], "story_mid_thread");
}

#[test]
fn a_marker_we_cant_use_stops_the_recap_and_is_left_alone() {
    let dir = resumed();
    let p = dir.path();
    let marker = p.join(".harness/writer/last-session.json");
    std::fs::write(&marker, "not json").unwrap();
    let o = skaldsaga(p, &["recap"]);
    assert!(!o.status.success());
    assert_eq!(std::fs::read_to_string(&marker).unwrap(), "not json");
}
