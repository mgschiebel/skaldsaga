//! End-to-end tests for `skaldsaga verdict add|list` and `skaldsaga queue
//! add|list|mark` (the agent records). The unit tests cover the rules; these
//! check the real binary: that a write names its file for git and returns a
//! message file only when something changed, that `list` only reads, that
//! content comes from a file and never from a flag, and that unusable input is
//! refused with nothing written.
#![expect(
    clippy::unwrap_used,
    reason = "a test that cannot go on should fail loudly, and a test failure is a panic"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

const MARA: &str = "p-maraharb";
const ANSWER: &str = "pr-abcdefgh";

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("skaldsaga-records-test-{}-{n}", std::process::id()));
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

fn stderr_of(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).to_string()
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// A set-up project holding the harbor test story and one saved answer.
fn project() -> TempDir {
    let dir = TempDir::new();
    assert!(skaldsaga(dir.path(), &["init"]).status.success());
    std::fs::remove_dir_all(dir.path().join("spec")).unwrap();
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../skaldsaga-core/tests/fixtures/specs/harbor");
    copy_dir(&fixture, &dir.path().join("spec"));
    let committed = dir.path().join(".harness/committed");
    std::fs::create_dir_all(&committed).unwrap();
    std::fs::write(committed.join(format!("{ANSWER}.json")), "{}\n").unwrap();
    dir
}

fn text_file(dir: &Path, name: &str, text: &str) -> String {
    let path = dir.join(name);
    std::fs::write(&path, text).unwrap();
    path.display().to_string()
}

/// Every file under `dir` with its bytes, to prove a command changed nothing.
/// The test's own input files (`*.txt`, `*.json` beside the story) are left out.
fn snapshot(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, out);
            } else if path.parent() == Some(root)
                && path.extension().is_some_and(|e| e == "txt" || e == "json")
            {
                // Input files the test wrote beside the story.
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

fn verdict_json(result: &str, cited: &[(&str, u32)], why: &str) -> String {
    let checked: Vec<_> = cited
        .iter()
        .map(|(id, revision)| serde_json::json!({"id": id, "revision": revision}))
        .collect();
    serde_json::json!({
        "answer": ANSWER, "result": result, "checked": checked, "justification": why
    })
    .to_string()
}

fn add_verdict(dir: &Path, name: &str, body: &str) -> Output {
    let file = text_file(dir, name, body);
    skaldsaga(
        dir,
        &["verdict", "add", "--agent", "continuity", "--file", &file],
    )
}

fn add_question(dir: &Path, body: &str) -> Output {
    let file = text_file(dir, "question.json", body);
    skaldsaga(
        dir,
        &["queue", "add", "--agent", "continuity", "--file", &file],
    )
}

fn failed_verdict(dir: &Path) -> String {
    let body = verdict_json("contradiction_reported", &[(MARA, 2)], "Eyes differ.");
    let v = json_of(&add_verdict(dir, "verdict.json", &body));
    v.get("id").unwrap().as_str().unwrap().to_string()
}

#[test]
fn a_verdict_is_recorded_and_names_its_file_and_a_message_for_git() {
    let dir = project();
    let p = dir.path();
    let v = json_of(&add_verdict(
        p,
        "verdict.json",
        &verdict_json("pass", &[(MARA, 2)], "Nothing conflicts."),
    ));
    assert_eq!(v["outcome"], "recorded");
    let id = v["id"].as_str().unwrap();
    assert!(id.starts_with("verdict-"), "{id}");
    assert_eq!(
        v["add_paths"],
        serde_json::json!([format!("spec/verdicts/continuity/{id}.md")])
    );
    assert!(
        p.join(format!("spec/verdicts/continuity/{id}.md"))
            .is_file()
    );
    let message = v["message_file"].as_str().unwrap();
    let text = std::fs::read_to_string(p.join(message))
        .unwrap_or_else(|_| std::fs::read_to_string(message).unwrap());
    assert!(
        text.contains("Skaldsaga: record a check of an answer"),
        "{text}"
    );
    // Nothing else appeared in the story's folders.
    assert!(!p.join("spec/queue").exists());
}

#[test]
fn the_same_verdict_again_is_reported_and_changes_nothing() {
    let dir = project();
    let p = dir.path();
    let body = verdict_json("pass", &[(MARA, 2)], "Nothing conflicts.");
    let first = json_of(&add_verdict(p, "verdict.json", &body));
    let before = snapshot(p);
    let again = json_of(&add_verdict(p, "verdict.json", &body));
    assert_eq!(again["outcome"], "exists");
    assert_eq!(again["id"], first["id"]);
    assert_eq!(again["add_paths"], serde_json::json!([]));
    assert!(again.get("message_file").is_none());
    assert_eq!(snapshot(p), before);
}

#[test]
fn an_unusable_verdict_is_refused_and_nothing_is_written() {
    let dir = project();
    let p = dir.path();
    let before = snapshot(p);
    let cases: Vec<(String, &str)> = vec![
        (
            verdict_json("made_up", &[(MARA, 2)], "x"),
            "isn't a result continuity may give",
        ),
        (
            verdict_json("pass", &[(MARA, 9)], "x"),
            "can't have been seen",
        ),
        (
            verdict_json("pass", &[("p-zzzzzzzz", 1)], "x"),
            "isn't in this story",
        ),
        (verdict_json("pass", &[], "x"), "at least one saved item"),
        ("not json".to_string(), "isn't usable"),
    ];
    for (body, expect) in cases {
        let out = add_verdict(p, "bad.json", &body);
        assert!(!out.status.success(), "{body}");
        assert!(
            stderr_of(&out).contains(expect),
            "{body}: {}",
            stderr_of(&out)
        );
        assert_eq!(snapshot(p), before, "{body}");
    }
    // An agent that is not registered, and one that only reads, cannot record.
    for agent in ["interviewer", "nobody"] {
        let file = text_file(p, "ok.json", &verdict_json("pass", &[(MARA, 2)], "x"));
        let out = skaldsaga(p, &["verdict", "add", "--agent", agent, "--file", &file]);
        assert!(!out.status.success(), "{agent}");
        assert_eq!(snapshot(p), before);
    }
}

#[test]
fn content_comes_only_from_a_file_never_from_a_flag() {
    let dir = project();
    let p = dir.path();
    let before = snapshot(p);
    for args in [
        vec![
            "verdict",
            "add",
            "--agent",
            "continuity",
            "--justification",
            "Eyes differ.",
        ],
        vec![
            "verdict",
            "add",
            "--agent",
            "continuity",
            "--file",
            "x.json",
            "--result",
            "pass",
        ],
        vec!["verdict", "add", "--file", "x.json"],
        vec!["verdict", "add", "--agent", "continuity"],
        vec![
            "verdict",
            "add",
            "--agent",
            "continuity",
            "--file",
            "missing.json",
        ],
        vec!["queue", "add", "--agent", "continuity", "--reason", "Why."],
        vec![
            "queue",
            "add",
            "--agent",
            "continuity",
            "--wording",
            "Is she red-haired?",
        ],
        vec![
            "queue",
            "mark",
            "--question",
            "question-abcdefgh",
            "--wording",
            "x",
        ],
        vec!["verdict", "frobnicate"],
        vec!["queue", "frobnicate"],
    ] {
        let out = skaldsaga(p, &args);
        assert!(!out.status.success(), "{args:?} should be refused");
        assert_eq!(snapshot(p), before, "{args:?}");
    }
}

#[test]
fn verdict_list_only_reads_and_marks_a_verdict_stale_when_its_cited_item_changes() {
    let dir = project();
    let p = dir.path();
    let id = failed_verdict(p);
    let before = snapshot(p);
    let v = json_of(&skaldsaga(p, &["verdict", "list"]));
    assert_eq!(v["outcome"], "listed");
    assert_eq!(snapshot(p), before, "list wrote something");
    let first = &v["verdicts"][0];
    assert_eq!(first["id"], id.as_str());
    assert_eq!(first["agent"], "continuity");
    assert_eq!(first["result"], "contradiction_reported");
    assert_eq!(first["stale"], false);
    assert_eq!(first["checked"][0]["id"], MARA);
    assert_eq!(first["checked"][0]["revision"], 2);
    assert_eq!(first["checked"][0]["current"], 2);

    let person = p.join("spec/people/p-maraharb.md");
    let text = std::fs::read_to_string(&person).unwrap();
    std::fs::write(&person, text.replacen("revision: 2\n", "revision: 3\n", 1)).unwrap();
    let after = json_of(&skaldsaga(p, &["verdict", "list", "--answer", ANSWER]));
    assert_eq!(after["verdicts"][0]["stale"], true);
    assert_eq!(after["verdicts"][0]["checked"][0]["current"], 3);

    // Filters that can't match, or aren't IDs, are refused rather than ignored.
    let none = json_of(&skaldsaga(
        p,
        &["verdict", "list", "--answer", "pr-zzzzzzzz"],
    ));
    assert_eq!(none["verdicts"], serde_json::json!([]));
    assert!(
        !skaldsaga(p, &["verdict", "list", "--agent", "nobody"])
            .status
            .success()
    );
    assert!(
        !skaldsaga(p, &["verdict", "list", "--answer", "../x"])
            .status
            .success()
    );
}

#[test]
fn a_question_is_queued_listed_and_marked_by_the_right_owners() {
    let dir = project();
    let p = dir.path();
    let verdict = failed_verdict(p);
    let body = serde_json::json!({
        "concerns": [MARA, "crisis"], "reason": "The eye colour clashes.", "verdict": verdict
    })
    .to_string();
    let q = json_of(&add_question(p, &body));
    assert_eq!(q["outcome"], "queued");
    let qid = q["id"].as_str().unwrap().to_string();
    assert!(qid.starts_with("question-"), "{qid}");
    assert_eq!(
        q["add_paths"],
        serde_json::json!([format!("spec/queue/continuity/{qid}.md")])
    );
    assert!(q["message_file"].is_string());
    // Queued again: reported, nothing written.
    let before = snapshot(p);
    let again = json_of(&add_question(p, &body));
    assert_eq!(again["outcome"], "exists");
    assert_eq!(again["id"], qid.as_str());
    assert_eq!(again["add_paths"], serde_json::json!([]));
    assert_eq!(snapshot(p), before);

    let listed = json_of(&skaldsaga(p, &["queue", "list", "--near", "crisis"]));
    assert_eq!(snapshot(p), before, "list wrote something");
    let item = &listed["questions"][0];
    assert_eq!(item["id"], qid.as_str());
    assert_eq!(item["state"], "open");
    assert_eq!(item["near"], true);
    assert_eq!(item["verdict"], verdict.as_str());
    assert_eq!(item["concerns"], serde_json::json!([MARA, "crisis"]));
    assert!(item["stale"] == false);

    // The Interviewer's mark writes only its own folder.
    let m = json_of(&skaldsaga(
        p,
        &["queue", "mark", "--question", &qid, "--state", "asked"],
    ));
    assert_eq!(m["outcome"], "marked");
    assert_eq!(m["state"], "asked");
    let path = m["add_paths"][0].as_str().unwrap();
    assert!(path.starts_with("spec/queue/interviewer/asked-"), "{path}");
    assert!(m["message_file"].is_string());
    let listed = json_of(&skaldsaga(p, &["queue", "list"]));
    assert_eq!(listed["questions"][0]["state"], "asked");
    let continuity: Vec<String> = std::fs::read_dir(p.join("spec/queue/continuity"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        continuity,
        [format!("{qid}.md")],
        "mark touched Continuity's folder"
    );

    // Marking the same state again changes nothing; a new state replaces it.
    let before = snapshot(p);
    let same = json_of(&skaldsaga(
        p,
        &["queue", "mark", "--question", &qid, "--state", "asked"],
    ));
    assert_eq!(same["outcome"], "unchanged");
    assert_eq!(same["add_paths"], serde_json::json!([]));
    assert_eq!(snapshot(p), before);
    let done = json_of(&skaldsaga(
        p,
        &["queue", "mark", "--question", &qid, "--state", "answered"],
    ));
    assert_eq!(done["outcome"], "marked");
    // An answered question leaves the list, and its state is final.
    let listed = json_of(&skaldsaga(p, &["queue", "list"]));
    assert_eq!(listed["questions"], serde_json::json!([]));
    let out = skaldsaga(
        p,
        &["queue", "mark", "--question", &qid, "--state", "set_aside"],
    );
    assert!(!out.status.success());
    assert!(
        stderr_of(&out).contains("final state"),
        "{}",
        stderr_of(&out)
    );
}

#[test]
fn an_unusable_question_is_refused_and_nothing_is_written() {
    let dir = project();
    let p = dir.path();
    let failed = failed_verdict(p);
    let passed = json_of(&add_verdict(
        p,
        "pass.json",
        &verdict_json("pass", &[(MARA, 2)], "Fine."),
    ))["id"]
        .as_str()
        .unwrap()
        .to_string();
    let before = snapshot(p);
    let q = |concerns: serde_json::Value, reason: &str, verdict: &str| {
        serde_json::json!({"concerns": concerns, "reason": reason, "verdict": verdict}).to_string()
    };
    let cases = [
        (
            q(serde_json::json!([MARA]), "r", &passed),
            "raises no question",
        ),
        (
            q(serde_json::json!([MARA]), "r", "verdict-zzzzzzzz"),
            "there's no verdict",
        ),
        (
            q(serde_json::json!([]), "r", &failed),
            "at least one story part",
        ),
        (
            q(serde_json::json!(["p-zzzzzzzz"]), "r", &failed),
            "isn't a part of this story",
        ),
        (
            q(serde_json::json!(["../x"]), "r", &failed),
            "isn't a story part",
        ),
        (
            q(serde_json::json!([MARA]), "   ", &failed),
            "needs a reason",
        ),
    ];
    for (body, expect) in cases {
        let out = add_question(p, &body);
        assert!(!out.status.success(), "{body}");
        assert!(
            stderr_of(&out).contains(expect),
            "{body}: {}",
            stderr_of(&out)
        );
        assert_eq!(snapshot(p), before, "{body}");
    }
    for (args, why) in [
        (
            vec![
                "queue",
                "mark",
                "--question",
                "question-zzzzzzzz",
                "--state",
                "asked",
            ],
            "an unknown question",
        ),
        (
            vec![
                "queue",
                "mark",
                "--question",
                "../etc/passwd",
                "--state",
                "asked",
            ],
            "a path instead of an ID",
        ),
        (
            vec![
                "queue",
                "mark",
                "--question",
                "question-abcdefgh",
                "--state",
                "open",
            ],
            "a state that isn't a mark",
        ),
        (
            vec!["queue", "list", "--agent", "interviewer"],
            "an agent that raises nothing",
        ),
        (
            vec!["queue", "list", "--near", "../x"],
            "a nearness that isn't a story part",
        ),
    ] {
        let out = skaldsaga(p, &args);
        assert!(!out.status.success(), "{why}");
        assert_eq!(snapshot(p), before, "{why}");
    }
}

#[test]
fn the_record_commands_refuse_outside_a_project() {
    let dir = TempDir::new();
    for args in [
        vec!["verdict", "list"],
        vec!["queue", "list"],
        vec![
            "queue",
            "mark",
            "--question",
            "question-abcdefgh",
            "--state",
            "asked",
        ],
    ] {
        let out = skaldsaga(dir.path(), &args);
        assert!(!out.status.success(), "{args:?}");
    }
}

#[test]
fn a_story_with_records_still_checks_and_indexes_the_same() {
    let dir = project();
    let p = dir.path();
    let check_before = json_of(&skaldsaga(p, &["check"]));
    let index_before = json_of(&skaldsaga(p, &["index"]));
    let verdict = failed_verdict(p);
    let body = serde_json::json!({
        "concerns": [MARA], "reason": "Why.", "verdict": verdict
    })
    .to_string();
    json_of(&add_question(p, &body));
    // Records are not story parts: they change no check and no index.
    assert_eq!(json_of(&skaldsaga(p, &["check"])), check_before);
    assert_eq!(json_of(&skaldsaga(p, &["index"])), index_before);
}
