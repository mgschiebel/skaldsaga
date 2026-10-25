//! End-to-end tests for `skaldsaga init`, running the real binary the way the
//! skill will.
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
            std::env::temp_dir().join(format!("skaldsaga-init-test-{}-{n}", std::process::id()));
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

fn init(dir: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_skaldsaga"))
        .env_remove("CLAUDE_PROJECT_DIR")
        .env_remove("CLAUDE_CONFIG_DIR")
        .arg("init")
        .args(extra)
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

#[test]
fn sets_up_then_reports_already_set_up() {
    let dir = TempDir::new();
    let v = json_of(&init(dir.path(), &[]));
    assert_eq!(v["outcome"], "created");
    assert!(v["meter"].is_null());
    let commit: Vec<_> = v["commit_paths"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p.as_str().unwrap())
        .collect();
    assert_eq!(
        commit,
        [
            ".harness",
            ".gitignore",
            "spec",
            "manuscript",
            "reviews",
            "style-guide"
        ]
    );
    for p in commit {
        assert!(dir.path().join(p).exists(), "{p}");
    }

    let v = json_of(&init(dir.path(), &[]));
    assert_eq!(v["outcome"], "already_set_up");
    assert_eq!(v["created"].as_array().unwrap().len(), 0);
}

#[test]
fn wires_the_meter_when_asked() {
    let dir = TempDir::new();
    let user = dir.path().join("no-user-settings.json");
    let v = json_of(&init(
        dir.path(),
        &[
            "--meter-command",
            "sh /plugin/scripts/skaldsaga meter statusline",
            "--user-settings",
            user.to_str().unwrap(),
        ],
    ));
    assert_eq!(v["outcome"], "created");
    assert_eq!(v["meter"]["outcome"], "wired");
    // The meter goes in the personal settings, which the new .gitignore keeps out of git.
    let settings = std::fs::read_to_string(dir.path().join(".claude/settings.local.json")).unwrap();
    assert!(settings.contains("meter statusline"));
    assert!(!dir.path().join(".claude/settings.json").exists());
    let ignore = std::fs::read_to_string(dir.path().join(".gitignore")).unwrap();
    assert!(ignore.lines().any(|l| l == ".claude/settings.local.json"));
}

#[test]
fn moves_a_shared_meter_entry_from_version_0_3_0() {
    let dir = TempDir::new();
    let user = dir.path().join("no-user-settings.json");
    let meter = [
        "--meter-command",
        "sh /plugin/scripts/skaldsaga meter statusline",
        "--user-settings",
        user.to_str().unwrap(),
    ];
    // Set up and wired as 0.3.1 does, then an old shared entry like 0.3.0's.
    init(dir.path(), &meter);
    let shared = dir.path().join(".claude/settings.json");
    std::fs::write(
        &shared,
        r#"{"statusLine":{"type":"command","command":"/root/.claude/plugins/synced/x/skaldsaga/scripts/skaldsaga meter statusline"}}"#,
    )
    .unwrap();

    // Running setup again takes it out, even though nothing else changed.
    let v = json_of(&init(dir.path(), &meter));
    assert_eq!(v["outcome"], "already_set_up");
    assert_eq!(v["meter"]["outcome"], "already_wired");
    assert_eq!(
        v["meter"]["cleaned"],
        serde_json::json!([".claude/settings.json"])
    );
    assert!(!shared.exists());

    // Nothing left to clean the next time.
    let v = json_of(&init(dir.path(), &meter));
    assert_eq!(v["meter"]["cleaned"], serde_json::json!([]));
}

#[test]
fn existing_files_are_reported_and_left_alone() {
    let dir = TempDir::new();
    std::fs::write(dir.path().join("my novel.txt"), "Once upon a time").unwrap();
    let v = json_of(&init(
        dir.path(),
        &["--meter-command", "sh x meter statusline"],
    ));
    assert_eq!(v["outcome"], "existing_files");
    assert_eq!(v["existing"][0], "my novel.txt");
    assert_eq!(v["existing_count"], 1);
    assert!(v.get("meter").is_none());
    assert!(!dir.path().join(".harness").exists());
    assert!(!dir.path().join(".claude").exists());
}

#[test]
fn rejects_a_meter_command_that_isnt_the_meter() {
    let dir = TempDir::new();
    let o = init(dir.path(), &["--meter-command", "echo hi"]);
    assert_eq!(o.status.code(), Some(64));
    assert!(!dir.path().join(".harness").exists());
}

#[test]
fn uses_the_project_flag() {
    let dir = TempDir::new();
    let story = dir.path().join("story");
    std::fs::create_dir(&story).unwrap();
    let v = json_of(&init(dir.path(), &["--project", story.to_str().unwrap()]));
    assert_eq!(v["outcome"], "created");
    assert!(story.join("spec/sequence.md").is_file());
    assert!(!dir.path().join("spec").exists());
}
