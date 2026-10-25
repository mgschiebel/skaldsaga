//! End-to-end tests for `skaldsaga scenario basics` and `skaldsaga commit`, following
//! the same steps as the skill's demo.
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
            std::env::temp_dir().join(format!("skaldsaga-commit-test-{}-{n}", std::process::id()));
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

#[test]
fn the_demo_commits_one_answer_and_refuses_the_other() {
    let dir = TempDir::new();
    let p = dir.path();
    // A subfolder, as when a repository is cloned one level down.
    let story = p.join("story");
    std::fs::create_dir_all(&story).unwrap();
    let project = story.to_string_lossy().to_string();
    assert!(
        skaldsaga(p, &["init", "--project", &project])
            .status
            .success()
    );

    let v = json_of(&skaldsaga(
        p,
        &["scenario", "basics", "--project", &project],
    ));
    assert_eq!(v["outcome"], "written");
    assert_eq!(v["scenario"], "basics");
    assert!(v["next_command"].as_str().unwrap().contains("check"));
    assert!(!v["expect"].as_str().unwrap().is_empty());
    let again = json_of(&skaldsaga(
        p,
        &["scenario", "basics", "--project", &project],
    ));
    assert_eq!(again["outcome"], "already_written");

    let v = json_of(&skaldsaga(
        p,
        &["check", "--project", &project, "--proposal", "pr-demstart"],
    ));
    // The sample scenes aren't tied to a thread and stage yet, which is a
    // soft flag: the writer decides, so nothing commits until they do.
    assert_eq!(v["proposals"][0]["status"], "flagged");
    let codes: Vec<&str> = v["proposals"][0]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["code"].as_str().unwrap())
        .collect();
    assert_eq!(codes, ["scene_untied", "scene_untied"]);

    let v = json_of(&skaldsaga(
        p,
        &["commit", "--project", &project, "--proposal", "pr-demstart"],
    ));
    assert_eq!(v["outcome"], "held");
    assert_eq!(v["reason"], "needs_decision");
    assert!(!story.join("spec/people/p-demmara2.md").exists());

    let v = json_of(&skaldsaga(
        p,
        &[
            "commit",
            "--project",
            &project,
            "--proposal",
            "pr-demstart",
            "--decision",
            "keep_anyway",
        ],
    ));
    assert_eq!(v["outcome"], "committed");
    assert_eq!(v["decision"], "keep_anyway");
    assert!(
        v.get("message").is_none(),
        "the text itself is never returned"
    );
    assert_eq!(v["message_file"], ".harness/commit-message.txt");
    assert!(
        std::fs::read_to_string(story.join(".harness/commit-message.txt"))
            .unwrap()
            .starts_with("Skaldsaga: Mara is a courier")
    );
    let add: Vec<&str> = v["add_paths"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
    assert!(add.contains(&"spec/scenes/s-demarrst.md"));
    assert!(add.contains(&".harness/committed/pr-demstart.json"));
    assert!(add.iter().all(|a| !a.starts_with("spec/_index")));
    assert_eq!(v["remove_paths"][0], ".harness/inbox/pr-demstart.json");
    assert!(story.join("spec/people/p-demmara2.md").is_file());

    let v = json_of(&skaldsaga(
        p,
        &["commit", "--project", &project, "--proposal", "pr-demmixup"],
    ));
    assert_eq!(v["outcome"], "held");
    assert_eq!(v["reason"], "failed");
    assert_eq!(v["findings"][0]["code"], "learns_before_true");

    // A scenario never runs in a story that already has content.
    std::fs::remove_file(story.join(".harness/inbox/pr-demmixup.json")).unwrap();
    std::fs::remove_dir_all(story.join(".harness/committed")).unwrap();
    let v = json_of(&skaldsaga(
        p,
        &["scenario", "basics", "--project", &project],
    ));
    assert_eq!(v["outcome"], "spec_not_empty");
}

#[test]
fn commit_reports_usage_mistakes_and_missing_proposals() {
    let dir = TempDir::new();
    let p = dir.path();
    assert!(skaldsaga(p, &["init"]).status.success());
    assert_eq!(skaldsaga(p, &["commit"]).status.code(), Some(64));
    let o = skaldsaga(
        p,
        &["commit", "--proposal", "pr-demstart", "--decision", "maybe"],
    );
    assert_eq!(o.status.code(), Some(64));
    let v = json_of(&skaldsaga(
        p,
        &["commit", "--proposal", "pr-nnnnnnnn", "--cleared"],
    ));
    assert_eq!(v["outcome"], "not_found");
}

/// SR-20: the proposal ID becomes part of a path, so only a well-formed one
/// is accepted; a path-like one never reaches a file.
#[test]
fn a_path_like_proposal_id_is_refused_before_any_file_is_touched() {
    let dir = TempDir::new();
    let p = dir.path();
    assert!(skaldsaga(p, &["init"]).status.success());
    assert!(skaldsaga(p, &["scenario", "basics"]).status.success());
    // A real proposal that sits outside the inbox, reachable with "..".
    let outside = p.join(".harness/pr-escapee.json");
    std::fs::copy(p.join(".harness/inbox/pr-demstart.json"), &outside).unwrap();
    let before = std::fs::read_to_string(&outside).unwrap();

    for id in [
        "../pr-escapee",
        "../../.harness/pr-escapee",
        "pr-escapee/..",
        "",
        "pr-ESCAPEE",
    ] {
        let o = skaldsaga(p, &["commit", "--proposal", id]);
        assert_eq!(o.status.code(), Some(64), "id {id:?}");
    }
    assert_eq!(std::fs::read_to_string(&outside).unwrap(), before);
    assert!(
        !p.join("spec/people")
            .read_dir()
            .map(|mut d| d.next().is_some())
            .unwrap_or(false)
    );
}

#[test]
fn scenario_needs_a_known_name_and_the_old_demo_name_is_gone() {
    let dir = TempDir::new();
    let p = dir.path();
    assert!(skaldsaga(p, &["init"]).status.success());
    assert_eq!(skaldsaga(p, &["scenario"]).status.code(), Some(64));
    assert_eq!(
        skaldsaga(p, &["scenario", "--project", "."]).status.code(),
        Some(64)
    );
    assert_eq!(skaldsaga(p, &["scenario", "nope"]).status.code(), Some(64));
    assert_eq!(skaldsaga(p, &["demo"]).status.code(), Some(64));
    // Nothing was filed by any of the refused calls.
    assert!(!p.join(".harness/inbox/pr-demstart.json").exists());
}
