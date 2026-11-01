//! End-to-end tests for the session note's split between what is committed and
//! what stays on this computer, the answer count that stands in for the meter,
//! and the reasons a stop must give (slice 3, planning session 4).
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
            std::env::temp_dir().join(format!("skaldsaga-session-test-{}-{n}", std::process::id()));
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

fn empty_story() -> TempDir {
    let dir = TempDir::new();
    assert!(skaldsaga(dir.path(), &["init"]).status.success());
    dir
}

fn answer(p: &Path, text: &str, confirmed: &str) -> serde_json::Value {
    let path = p.join("words.txt");
    std::fs::write(&path, text).unwrap();
    json_of(&skaldsaga(
        p,
        &[
            "session",
            "answer",
            "--words-file",
            &path.display().to_string(),
            "--confirmed",
            confirmed,
        ],
    ))
}

fn meter_state(p: &Path) -> serde_json::Value {
    json_of(&skaldsaga(p, &["meter", "check"]))
}

fn marker_path(p: &Path) -> PathBuf {
    p.join(".harness/writer/last-session.json")
}

#[test]
fn an_unconfirmed_answer_and_a_question_leave_the_committed_note_alone() {
    let dir = empty_story();
    let p = dir.path();
    let a = answer(p, "A courier crosses the blockade", "no");
    assert_eq!(a["add_paths"], serde_json::json!([]));
    let q = json_of(&skaldsaga(p, &["session", "ask", "--topic", "setup"]));
    assert_eq!(q["add_paths"], serde_json::json!([]));
    assert!(!marker_path(p).exists());
    // This computer still knows a thread is open.
    let r = json_of(&skaldsaga(p, &["route", "--text", "no"]));
    assert_eq!(r["condition"], "story_mid_thread");
    let recap = json_of(&skaldsaga(p, &["recap"]));
    assert_eq!(recap["loose_idea"], "A courier crosses the blockade");
    assert_eq!(recap["last_topic"], "setup");
    // The file that holds it is ignored by git.
    let ignore = std::fs::read_to_string(p.join(".harness/.gitignore")).unwrap();
    assert!(ignore.lines().any(|l| l == "session-local.json"));
}

#[test]
fn a_confirmed_answer_folds_the_topic_into_the_committed_note() {
    let dir = empty_story();
    let p = dir.path();
    answer(p, "A courier crosses the blockade", "no");
    json_of(&skaldsaga(p, &["session", "ask", "--topic", "setup"]));
    let c = answer(p, "A courier crosses the blockade", "yes");
    assert_eq!(c["outcome"], "recorded");
    assert_eq!(c["add_paths"][0], ".harness/writer/last-session.json");
    let note = std::fs::read_to_string(marker_path(p)).unwrap();
    assert!(note.contains("\"topic\": \"setup\""), "{note}");
    assert!(note.contains("\"confirmed\": true"), "{note}");
    let recap = json_of(&skaldsaga(p, &["recap"]));
    assert_eq!(recap["loose_idea"], serde_json::Value::Null);
    assert_eq!(recap["answers_since_offer"], 1);
}

#[test]
fn the_answer_count_stands_in_for_a_meter_with_no_reading() {
    let dir = empty_story();
    let p = dir.path();
    assert_eq!(meter_state(p)["state"], "unknown");
    for n in 1..=35 {
        answer(p, &format!("answer number {n}"), "yes");
        let m = meter_state(p);
        let want = match n {
            ..=24 => "unknown",
            25..=34 => "suggest_wrap_up",
            _ => "wrap_up",
        };
        assert_eq!(m["state"], want, "after {n} answers");
        if n >= 25 {
            assert_eq!(m["source"], "answers");
            assert_eq!(m["answers_this_session"], n);
        }
    }
}

#[test]
fn the_wrap_up_counts_are_settings() {
    let dir = empty_story();
    let p = dir.path();
    let settings = p.join(".harness/settings.json");
    let text = std::fs::read_to_string(&settings).unwrap();
    assert!(
        text.contains("\"suggest_wrap_up_after_answers\": 25"),
        "{text}"
    );
    assert!(text.contains("\"wrap_up_after_answers\": 35"), "{text}");
    std::fs::write(
        &settings,
        text.replace(
            "\"suggest_wrap_up_after_answers\": 25",
            "\"suggest_wrap_up_after_answers\": 2",
        )
        .replace(
            "\"wrap_up_after_answers\": 35",
            "\"wrap_up_after_answers\": 3",
        ),
    )
    .unwrap();
    answer(p, "one", "yes");
    assert_eq!(meter_state(p)["state"], "unknown");
    answer(p, "two", "yes");
    assert_eq!(meter_state(p)["state"], "suggest_wrap_up");
    answer(p, "three", "yes");
    assert_eq!(meter_state(p)["state"], "wrap_up");
}

#[test]
fn a_real_meter_reading_is_never_overruled_by_the_count() {
    let dir = empty_story();
    let p = dir.path();
    std::fs::write(
        p.join(".harness/meter.json"),
        r#"{"session_id":"s","used_percentage":10.0,"updated_at":1}"#,
    )
    .unwrap();
    for n in 1..=30 {
        answer(p, &format!("answer number {n}"), "yes");
    }
    let m = meter_state(p);
    assert_eq!(m["state"], "ok");
    assert_eq!(m["source"], "context");
}

#[test]
fn the_session_start_hook_starts_the_count_over() {
    let dir = empty_story();
    let p = dir.path();
    for n in 1..=25 {
        answer(p, &format!("answer number {n}"), "yes");
    }
    assert_eq!(meter_state(p)["state"], "suggest_wrap_up");
    let hook = Command::new(env!("CARGO_BIN_EXE_skaldsaga"))
        .env("CLAUDE_PROJECT_DIR", p)
        .args(["hook", "session-start"])
        .current_dir(p)
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    assert!(hook.status.success());
    assert_eq!(meter_state(p)["state"], "unknown");
}

#[test]
fn a_stop_must_give_a_reason_that_is_believable() {
    let dir = empty_story();
    let p = dir.path();
    // A reason is required, and must be one of the three.
    assert_eq!(skaldsaga(p, &["session", "stop"]).status.code(), Some(64));
    let bad = skaldsaga(p, &["session", "stop", "--reason", "tired"]);
    assert_eq!(bad.status.code(), Some(64));
    // No stopping point on the first answer, however it is put...
    answer(p, "one", "yes");
    let early = skaldsaga(p, &["session", "stop", "--reason", "writer_signals"]);
    assert_eq!(early.status.code(), Some(64));
    // ...but an exhausted thread is believable after one, and the writer's own
    // "I'm done" always is.
    let ok = json_of(&skaldsaga(
        p,
        &["session", "stop", "--reason", "thread_exhausted"],
    ));
    assert_eq!(ok["outcome"], "recorded");
    assert_eq!(ok["unconfirmed_pending"], false);
    assert!(
        skaldsaga(p, &["session", "stop", "--reason", "writer_done"])
            .status
            .success()
    );
}

#[test]
fn a_stop_with_an_unconfirmed_answer_says_so_and_drops_it() {
    let dir = empty_story();
    let p = dir.path();
    answer(p, "one", "yes");
    answer(p, "two", "yes");
    answer(p, "not sure, maybe a bridge", "no");
    let stop = json_of(&skaldsaga(
        p,
        &["session", "stop", "--reason", "writer_signals"],
    ));
    assert_eq!(stop["unconfirmed_pending"], true);
    assert!(p.join(stop["message_file"].as_str().unwrap()).is_file());
    let recap = json_of(&skaldsaga(p, &["recap"]));
    assert_eq!(recap["state"], "stopping_point");
    assert_eq!(recap["loose_idea"], serde_json::Value::Null);
    assert_eq!(
        json_of(&skaldsaga(p, &["route", "--text", "no"]))["condition"],
        "story_stopping_point"
    );
}

#[test]
fn the_recap_at_a_stopping_point_offers_up_to_three_areas_with_the_suggestion_first() {
    let dir = empty_story();
    let p = dir.path();
    json_of(&skaldsaga(
        p,
        &["session", "stop", "--reason", "writer_done"],
    ));
    let recap = json_of(&skaldsaga(p, &["recap"]));
    assert_eq!(recap["state"], "stopping_point");
    assert_eq!(recap["topic"], "premise");
    assert_eq!(
        recap["choices"],
        serde_json::json!(["premise", "people", "setup"])
    );
}
