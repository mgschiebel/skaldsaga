//! End-to-end tests for `skaldsaga index`, running the real binary.
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
            std::env::temp_dir().join(format!("skaldsaga-index-test-{}-{n}", std::process::id()));
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

fn index(dir: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_skaldsaga"))
        .env_remove("CLAUDE_PROJECT_DIR")
        .arg("index")
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

#[test]
fn refuses_outside_a_project_and_writes_nothing() {
    let dir = TempDir::new();
    let o = index(dir.path());
    assert_eq!(o.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&o.stderr).contains("isn't set up for Skaldsaga"));
    assert!(!dir.path().join("spec").exists());
}

#[test]
fn rebuilds_the_index_after_setup() {
    let dir = TempDir::new();
    let init = Command::new(env!("CARGO_BIN_EXE_skaldsaga"))
        .env_remove("CLAUDE_PROJECT_DIR")
        .arg("init")
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(init.status.success());

    // A fresh project indexes cleanly.
    let o = index(dir.path());
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["outcome"], "rebuilt");
    assert_eq!(v["nodes"], 0);
    assert_eq!(v["files"].as_array().unwrap().len(), 5);
    assert!(dir.path().join("spec/_index/.gitignore").is_file());

    // A real story, with its problems counted.
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../skaldsaga-core/tests/fixtures/specs/harbor");
    std::fs::remove_dir_all(dir.path().join("spec")).unwrap();
    copy_dir(&fixture, &dir.path().join("spec"));
    let o = index(dir.path());
    assert!(o.status.success());
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["nodes"], 27);
    assert_eq!(v["problems"], 1);
    assert_eq!(v["broken_references"], 2);
}
