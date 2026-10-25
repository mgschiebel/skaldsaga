//! End-to-end tests for `skaldsaga check`, running the real binary.
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
            std::env::temp_dir().join(format!("skaldsaga-check-test-{}-{n}", std::process::id()));
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

/// A set-up project holding the harbor test story.
fn project() -> TempDir {
    let dir = TempDir::new();
    assert!(skaldsaga(dir.path(), &["init"]).status.success());
    std::fs::remove_dir_all(dir.path().join("spec")).unwrap();
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../skaldsaga-core/tests/fixtures/specs/harbor");
    copy_dir(&fixture, &dir.path().join("spec"));
    dir
}

fn write_proposal(dir: &Path, id: &str, ops: &str) {
    let text =
        format!(r#"{{"id": "{id}", "agent": "interviewer", "answer": "test", "ops": {ops}}}"#);
    std::fs::write(dir.join(".harness/inbox").join(format!("{id}.json")), text).unwrap();
}

fn json_of(o: &Output) -> serde_json::Value {
    assert!(
        o.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    serde_json::from_slice(&o.stdout).unwrap()
}

#[test]
fn checks_the_inbox_and_writes_each_verdict_into_its_file() {
    let dir = project();
    let p = dir.path();
    write_proposal(
        p,
        "pr-gdgdgdgd",
        r#"[{"op": "set", "target": "p-dadsharb.summary", "value": "Ada's father"}]"#,
    );
    write_proposal(
        p,
        "pr-bdbdbdbd",
        r#"[{"op": "delete", "target": "p-dadsharb"}]"#,
    );
    std::fs::write(p.join(".harness/inbox/pr-brkbrkbr.json"), "{not json").unwrap();
    let mara_before = std::fs::read_to_string(p.join("spec/people/p-dadsharb.md")).unwrap();

    let v = json_of(&skaldsaga(p, &["check"]));
    assert_eq!(v["outcome"], "checked");
    assert_eq!(v["spec_problems"], 1);
    let list = v["proposals"].as_array().unwrap();
    assert_eq!(list.len(), 3);
    let by = |key: &str, val: &str| {
        list.iter()
            .find(|e| e[key] == val)
            .unwrap_or_else(|| panic!("{val} in {list:?}"))
    };
    let bad = by("id", "pr-bdbdbdbd");
    assert_eq!(bad["status"], "failed");
    assert_eq!(bad["findings"][0]["code"], "delete_blocked");
    assert!(
        bad["findings"][0]["message"]
            .as_str()
            .unwrap()
            .contains("still referenced")
    );
    let good = by("id", "pr-gdgdgdgd");
    assert_eq!(good["status"], "passed");
    assert_eq!(good["lane"], "optimistic");
    assert_eq!(by("file", "pr-brkbrkbr.json")["status"], "unreadable");

    // The verdict and checked revisions are in the file; the spec is untouched.
    let saved: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(p.join(".harness/inbox/pr-gdgdgdgd.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(saved["check"]["status"], "passed");
    assert_eq!(saved["revisions"]["checked"]["p-dadsharb"], 1);
    assert!(saved["revisions"]["checked"]["sequence.md"].is_string());
    assert_eq!(saved["answer"], "test");
    assert_eq!(
        std::fs::read_to_string(p.join("spec/people/p-dadsharb.md")).unwrap(),
        mara_before
    );
    assert_eq!(
        std::fs::read_to_string(p.join(".harness/inbox/pr-brkbrkbr.json")).unwrap(),
        "{not json"
    );
}

#[test]
fn checks_only_the_named_proposals() {
    let dir = project();
    let p = dir.path();
    write_proposal(p, "pr-gdgdgdgd", "[]");
    write_proposal(p, "pr-bdbdbdbd", "[]");
    let v = json_of(&skaldsaga(
        p,
        &[
            "check",
            "--proposal",
            "pr-gdgdgdgd",
            "--proposal",
            "pr-nnnnnnnn",
        ],
    ));
    let list = v["proposals"].as_array().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["id"], "pr-gdgdgdgd");
    assert_eq!(v["not_found"][0], "pr-nnnnnnnn");
}

#[test]
fn refuses_outside_a_project() {
    let dir = TempDir::new();
    let o = skaldsaga(dir.path(), &["check"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&o.stderr).contains("isn't set up for Skaldsaga"));
}
