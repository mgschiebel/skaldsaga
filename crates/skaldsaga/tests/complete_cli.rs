//! End-to-end tests for `skaldsaga complete` and `skaldsaga decisions`.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "skaldsaga-complete-test-{}-{n}",
            std::process::id()
        ));
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

/// A set-up project holding the harbor fixture story (which has findings).
fn harbor() -> TempDir {
    let dir = TempDir::new();
    std::fs::create_dir_all(dir.path().join(".harness")).unwrap();
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../skaldsaga-core/tests/fixtures/specs/harbor");
    copy_dir(&fixture, &dir.path().join("spec"));
    dir
}

fn finding<'a>(report: &'a serde_json::Value, code: &str) -> &'a serde_json::Value {
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["code"] == code)
        .unwrap_or_else(|| panic!("no {code} finding in {report}"))
}

#[test]
fn refuses_outside_a_project_and_writes_nothing() {
    let dir = TempDir::new();
    let out = skaldsaga(dir.path(), &["complete"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("isn't set up"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn a_story_with_no_arc_yet_is_ready_for_nothing() {
    let dir = TempDir::new();
    let p = dir.path();
    assert!(skaldsaga(p, &["init"]).status.success());
    assert!(skaldsaga(p, &["scenario", "basics"]).status.success());
    let c = json_of(&skaldsaga(
        p,
        &[
            "commit",
            "--proposal",
            "pr-demstart",
            "--decision",
            "keep_anyway",
        ],
    ));
    assert_eq!(c["outcome"], "committed");

    let v = json_of(&skaldsaga(p, &["complete"]));
    assert_eq!(v["outcome"], "reported");
    for gate in ["weave", "outline", "complete"] {
        assert_eq!(v["gates"][gate]["ready"], false, "{gate}");
        assert_eq!(v["gates"][gate]["blockers"][0]["code"], "no_main_story");
    }
    assert_eq!(v["remove_paths"], serde_json::json!([]));
    // Nothing was written by asking.
    assert!(!p.join(".harness/decisions").exists());
}

#[test]
fn a_finding_can_be_acknowledged_and_the_acknowledgment_withdrawn() {
    let dir = harbor();
    let p = dir.path();
    let before = json_of(&skaldsaga(p, &["complete"]));
    let f = finding(&before, "threads_out_of_alignment").clone();
    assert_eq!(f["status"], "open");
    assert_eq!(f["gates"], true);
    let queue = |v: &serde_json::Value| {
        v["gates"]["weave"]["blockers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["code"] == "open_findings")
            .map(|b| b["count"].as_u64().unwrap())
            .unwrap_or(0)
    };
    assert_eq!(queue(&before), 1);

    // The writer's words go in a file, never on the command line.
    let words = p.join("words.txt");
    std::fs::write(&words, "The thread is meant to run ahead here.").unwrap();
    let ids = f["ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i.as_str().unwrap())
        .collect::<Vec<_>>()
        .join(",");
    let detail = f["detail"].as_str().unwrap();
    let words_arg = words.to_string_lossy().to_string();
    let ack = json_of(&skaldsaga(
        p,
        &[
            "decisions",
            "acknowledge",
            "--code",
            "threads_out_of_alignment",
            "--ids",
            &ids,
            "--detail",
            detail,
            "--words-file",
            &words_arg,
        ],
    ));
    assert_eq!(ack["outcome"], "acknowledged");
    assert_eq!(ack["renewed"], false);
    let path = ack["add_paths"][0].as_str().unwrap();
    assert!(
        path.starts_with(".harness/decisions/finding_ack/wd-"),
        "{path}"
    );
    let stored = std::fs::read_to_string(p.join(path)).unwrap();
    assert!(stored.contains("The thread is meant to run ahead here."));
    assert!(
        !ack.to_string().contains("run ahead"),
        "words aren't echoed back"
    );

    let after = json_of(&skaldsaga(p, &["complete"]));
    let f = finding(&after, "threads_out_of_alignment");
    assert_eq!(f["status"], "acknowledged");
    assert_eq!(f["decision"], ack["id"]);
    assert_eq!(queue(&after), 0);
    assert_eq!(after["acknowledgments"]["active"], 1);

    // Acknowledging again renews the same file.
    let again = json_of(&skaldsaga(
        p,
        &[
            "decisions",
            "acknowledge",
            "--code",
            "threads_out_of_alignment",
            "--ids",
            &ids,
            "--detail",
            detail,
            "--words-file",
            &words_arg,
        ],
    ));
    assert_eq!(again["renewed"], true);
    assert_eq!(again["id"], ack["id"]);

    // Withdrawing deletes it, and the finding is open again.
    let id = ack["id"].as_str().unwrap();
    let gone = json_of(&skaldsaga(p, &["decisions", "withdraw", "--decision", id]));
    assert_eq!(gone["outcome"], "withdrawn");
    assert_eq!(gone["remove_paths"][0], path);
    assert!(!p.join(path).exists());
    let reopened = json_of(&skaldsaga(p, &["complete"]));
    assert_eq!(
        finding(&reopened, "threads_out_of_alignment")["status"],
        "open"
    );
    let missing = json_of(&skaldsaga(p, &["decisions", "withdraw", "--decision", id]));
    assert_eq!(missing["outcome"], "not_found");
}

#[test]
fn bad_input_is_a_usage_error_and_writes_nothing() {
    let dir = harbor();
    let p = dir.path();
    let words = p.join("words.txt");
    std::fs::write(&words, "fine").unwrap();
    let words_arg = words.to_string_lossy().to_string();
    let code = |args: &[&str]| skaldsaga(p, args).status.code();

    // A finding the story doesn't have.
    assert_eq!(
        code(&[
            "decisions",
            "acknowledge",
            "--code",
            "unearned_twist",
            "--ids",
            "f-nopenope",
            "--words-file",
            &words_arg,
        ]),
        Some(64)
    );
    // Missing arguments, a missing file, and an ID that isn't one.
    assert_eq!(
        code(&["decisions", "acknowledge", "--words-file", &words_arg]),
        Some(64)
    );
    assert_eq!(
        code(&[
            "decisions",
            "acknowledge",
            "--code",
            "x",
            "--words-file",
            "/no/such/file"
        ]),
        Some(64)
    );
    assert_eq!(
        code(&[
            "decisions",
            "withdraw",
            "--decision",
            "../../.harness/settings"
        ]),
        Some(64)
    );
    assert_eq!(code(&["decisions", "withdraw"]), Some(64));
    assert_eq!(code(&["decisions", "nonsense"]), Some(64));
    assert!(!p.join(".harness/decisions").exists());
}

#[cfg(unix)]
#[test]
fn a_words_file_that_is_a_symlink_is_refused() {
    let dir = harbor();
    let p = dir.path();
    let target = p.join("real.txt");
    std::fs::write(&target, "fine").unwrap();
    let link = p.join("link.txt");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    let out = skaldsaga(
        p,
        &[
            "decisions",
            "acknowledge",
            "--code",
            "threads_out_of_alignment",
            "--ids",
            "a-mainharb,t-innrharb",
            "--detail",
            "crisis",
            "--words-file",
            &link.to_string_lossy(),
        ],
    );
    assert_eq!(out.status.code(), Some(64));
    assert!(!p.join(".harness/decisions").exists());
}
