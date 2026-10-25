//! SR-11: links, executable files and special files in a story folder are
//! reported by init, check, index and complete, and never followed.

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("skaldsaga-unsafe-{}-{n}", std::process::id()));
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

fn flagged(v: &serde_json::Value) -> Vec<(String, String)> {
    let mut out: Vec<_> = v["unsafe_files"]["flagged"]
        .as_array()
        .unwrap_or_else(|| panic!("no unsafe_files in {v}"))
        .iter()
        .map(|f| {
            (
                f["path"].as_str().unwrap().to_string(),
                f["why"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    out.sort();
    out
}

#[test]
fn a_clean_set_up_story_reports_nothing_flagged() {
    let dir = TempDir::new();
    let p = dir.path();
    let init = json_of(&skaldsaga(p, &["init"]));
    assert!(flagged(&init).is_empty());
    assert!(flagged(&json_of(&skaldsaga(p, &["index"]))).is_empty());
}

#[test]
fn links_and_programs_are_reported_and_never_read() {
    let dir = TempDir::new();
    let p = dir.path();
    assert!(skaldsaga(p, &["init"]).status.success());
    assert!(skaldsaga(p, &["demo"]).status.success());
    assert_eq!(
        json_of(&skaldsaga(
            p,
            &[
                "commit",
                "--proposal",
                "pr-demstart",
                "--decision",
                "keep_anyway",
            ]
        ))["outcome"],
        "committed"
    );

    // A link posing as a person, pointing at a real node file elsewhere.
    let people = p.join("spec/people");
    let real = std::fs::read_dir(&people)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|f| f.extension().is_some_and(|x| x == "md"))
        .unwrap();
    let outside = TempDir::new();
    let copy = outside.path().join("p-linklink.md");
    std::fs::copy(&real, &copy).unwrap();
    std::os::unix::fs::symlink(&copy, people.join("p-linklink.md")).unwrap();
    // A program hiding among the manuscript files.
    let prog = p.join("manuscript/run.md");
    std::fs::write(&prog, "x").unwrap();
    std::fs::set_permissions(&prog, std::fs::Permissions::from_mode(0o755)).unwrap();

    let want = vec![
        ("manuscript/run.md".to_string(), "executable".to_string()),
        ("spec/people/p-linklink.md".to_string(), "link".to_string()),
    ];
    assert_eq!(flagged(&json_of(&skaldsaga(p, &["index"]))), want);
    assert_eq!(flagged(&json_of(&skaldsaga(p, &["check"]))), want);
    let complete = json_of(&skaldsaga(p, &["complete"]));
    assert_eq!(flagged(&complete), want);

    // The linked file is a problem and was not loaded as a node.
    assert_eq!(complete["spec_problems"], 1, "{complete}");
    let status = std::fs::read_to_string(p.join("spec/_index/status.json")).unwrap();
    assert!(status.contains("never follows links"), "{status}");
}

#[test]
fn a_linked_proposal_is_not_read() {
    let dir = TempDir::new();
    let p = dir.path();
    assert!(skaldsaga(p, &["init"]).status.success());
    assert!(skaldsaga(p, &["demo"]).status.success());
    let outside = TempDir::new();
    let target = outside.path().join("pr-demwxzqk.json");
    std::fs::copy(p.join(".harness/inbox/pr-demstart.json"), &target).unwrap();
    std::os::unix::fs::symlink(&target, p.join(".harness/inbox/pr-demwxzqk.json")).unwrap();

    let before = std::fs::read_to_string(&target).unwrap();
    let v = json_of(&skaldsaga(p, &["check", "--proposal", "pr-demwxzqk"]));
    assert_eq!(v["proposals"][0]["status"], "unreadable", "{v}");
    assert!(
        v["proposals"][0]["message"]
            .as_str()
            .unwrap()
            .contains("never follows links"),
        "refused because it is a link, not for another reason: {v}"
    );
    assert_eq!(
        std::fs::read_to_string(&target).unwrap(),
        before,
        "the target is untouched"
    );
    let c = json_of(&skaldsaga(p, &["commit", "--proposal", "pr-demwxzqk"]));
    assert_eq!(c["outcome"], "unreadable", "{c}");
}
