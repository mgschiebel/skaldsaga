//! End-to-end tests for `skaldsaga route` and `skaldsaga session`: one fixture
//! folder per entry condition, built with the program's own commands.
//! Routing is deterministic and never runs `init`, so story text can't steer it.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("skaldsaga-entry-test-{}-{n}", std::process::id()));
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

fn route(dir: &Path, text: &str) -> serde_json::Value {
    json_of(&skaldsaga(dir, &["route", "--text", text]))
}

fn condition(dir: &Path, text: &str) -> String {
    route(dir, text)["condition"]
        .as_str()
        .unwrap_or("none")
        .to_string()
}

/// A set-up story folder with nothing committed.
fn empty_story() -> TempDir {
    let dir = TempDir::new();
    assert!(skaldsaga(dir.path(), &["init"]).status.success());
    dir
}

/// A set-up story holding the `basics` scenario's first answer.
fn story_with_content() -> TempDir {
    let dir = empty_story();
    let p = dir.path();
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
    dir
}

fn words_file(dir: &Path, text: &str) -> String {
    let path = dir.join("words.txt");
    std::fs::write(&path, text).unwrap();
    path.display().to_string()
}

#[test]
fn no_story_found_depends_only_on_whether_text_was_typed() {
    let dir = TempDir::new();
    let p = dir.path();
    let bare = route(p, "no");
    assert_eq!(bare["condition"], "no_story_bare");
    assert_eq!(bare["found"], "nothing");
    assert_eq!(bare["built"], true);
    assert_eq!(condition(p, "yes"), "no_story_with_text");
}

#[test]
fn routing_never_sets_anything_up() {
    let dir = TempDir::new();
    let p = dir.path();
    for text in ["no", "yes"] {
        route(p, text);
    }
    assert_eq!(
        std::fs::read_dir(p).unwrap().count(),
        0,
        "no init, no folders"
    );
}

#[test]
fn a_story_with_nothing_committed_goes_to_the_opener() {
    let dir = empty_story();
    let p = dir.path();
    assert_eq!(condition(p, "no"), "story_empty");
    assert_eq!(condition(p, "yes"), "story_empty");
}

#[test]
fn a_story_one_folder_down_is_found_and_routed() {
    let dir = TempDir::new();
    let story = dir.path().join("my-story");
    std::fs::create_dir_all(&story).unwrap();
    assert!(skaldsaga(&story, &["init"]).status.success());
    let v = route(dir.path(), "no");
    assert_eq!(v["found"], "below");
    assert_eq!(v["name"], "my-story");
    assert_eq!(v["condition"], "story_empty");
}

#[test]
fn several_stories_wait_for_a_choice() {
    let dir = TempDir::new();
    for name in ["a", "b"] {
        let story = dir.path().join(name);
        std::fs::create_dir_all(&story).unwrap();
        assert!(skaldsaga(&story, &["init"]).status.success());
    }
    let v = route(dir.path(), "yes");
    assert_eq!(v["found"], "several");
    assert!(v["condition"].is_null());
    assert_eq!(v["several"], serde_json::json!(["a", "b"]));
}

#[test]
fn a_story_with_content_asks_new_or_more_when_text_is_typed() {
    let dir = story_with_content();
    let p = dir.path();
    assert_eq!(condition(p, "yes"), "story_with_text");
    // No record of how the last session ended: a thread still open.
    let v = route(p, "no");
    assert_eq!(v["condition"], "story_mid_thread");
    assert_eq!(v["built"], false);
}

#[test]
fn recording_an_answer_makes_the_story_mid_thread_even_when_empty() {
    let dir = empty_story();
    let p = dir.path();
    let words = words_file(p, "A courier smuggles letters past the blockade.");
    let v = json_of(&skaldsaga(
        p,
        &[
            "session",
            "answer",
            "--words-file",
            &words,
            "--confirmed",
            "no",
        ],
    ));
    assert_eq!(v["outcome"], "recorded");
    assert_eq!(
        v["add_paths"],
        serde_json::json!([".harness/writer/last-session.json"])
    );
    assert!(v["message_file"].is_string());
    assert!(
        v.get("words").is_none(),
        "the writer's words never come back"
    );
    assert_eq!(condition(p, "no"), "story_mid_thread");
    assert_eq!(condition(p, "yes"), "story_with_text");

    // Saying the same thing again changes nothing.
    let again = json_of(&skaldsaga(
        p,
        &[
            "session",
            "answer",
            "--words-file",
            &words,
            "--confirmed",
            "no",
        ],
    ));
    assert_eq!(again["outcome"], "unchanged");
    assert_eq!(again["add_paths"], serde_json::json!([]));
}

#[test]
fn a_stopping_point_routes_to_the_stopping_point() {
    let dir = story_with_content();
    let p = dir.path();
    let v = json_of(&skaldsaga(p, &["session", "stop"]));
    assert_eq!(v["outcome"], "recorded");
    assert_eq!(condition(p, "no"), "story_stopping_point");
    assert_eq!(condition(p, "yes"), "story_with_text");
    assert_eq!(route(p, "no")["built"], false);
}

#[test]
fn a_story_file_that_cant_be_read_is_damage() {
    let dir = story_with_content();
    let p = dir.path();
    std::fs::write(p.join("spec/people/p-brokenaa.md"), "not a story file").unwrap();
    for text in ["no", "yes"] {
        let v = route(p, text);
        assert_eq!(v["condition"], "story_damaged");
        assert_eq!(v["built"], true);
        assert_eq!(v["damage"], serde_json::json!(["spec_problems"]));
    }
    // Routing changed nothing.
    assert!(p.join("spec/people/p-brokenaa.md").is_file());
}

#[test]
fn a_marker_that_cant_be_used_is_damage_and_is_never_overwritten() {
    let dir = empty_story();
    let p = dir.path();
    let file = p.join(".harness/writer/last-session.json");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, "not json").unwrap();
    let v = route(p, "no");
    assert_eq!(v["condition"], "story_damaged");
    assert_eq!(v["damage"], serde_json::json!(["marker_unusable"]));
    let words = words_file(p, "anything");
    let r = skaldsaga(
        p,
        &[
            "session",
            "answer",
            "--words-file",
            &words,
            "--confirmed",
            "yes",
        ],
    );
    assert!(!r.status.success());
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "not json");
}

#[test]
fn session_commands_need_a_set_up_folder_and_valid_flags() {
    let dir = TempDir::new();
    let p = dir.path();
    let words = words_file(p, "anything");
    for args in [
        vec!["session", "stop"],
        vec![
            "session",
            "answer",
            "--words-file",
            &words,
            "--confirmed",
            "yes",
        ],
    ] {
        let r = skaldsaga(p, &args);
        assert!(!r.status.success(), "{args:?}");
    }
    assert!(!p.join(".harness").exists());
    // The routing flag must be a plain yes or no.
    assert!(!skaldsaga(p, &["route"]).status.success());
    assert!(!skaldsaga(p, &["route", "--text", "maybe"]).status.success());
    assert!(!skaldsaga(p, &["session", "nonsense"]).status.success());
}

#[test]
fn a_confirmed_answer_is_filed_checked_and_committed() {
    let dir = empty_story();
    let p = dir.path();
    let draft = words_file(
        p,
        r#"{"answer": "A courier smuggles letters past a blockade.",
            "ops": [{"op": "create", "node": {"id": "p-marakest", "type": "person", "name": "Mara"}}]}"#,
    );
    let v = json_of(&skaldsaga(p, &["propose", "--file", &draft]));
    assert_eq!(v["outcome"], "filed");
    let id = v["id"].as_str().unwrap().to_string();
    assert!(p.join(format!(".harness/inbox/{id}.json")).is_file());

    let checked = json_of(&skaldsaga(p, &["check", "--proposal", &id]));
    assert_eq!(checked["proposals"][0]["status"], "passed");
    let c = json_of(&skaldsaga(p, &["commit", "--proposal", &id]));
    assert_eq!(c["outcome"], "committed");
    assert!(p.join("spec/people/p-marakest.md").is_file());
    // Now something is committed, so a bare command is no longer an empty story.
    assert_eq!(condition(p, "no"), "story_mid_thread");
}

#[test]
fn a_draft_that_picks_its_own_agent_is_refused_and_nothing_is_filed() {
    let dir = empty_story();
    let p = dir.path();
    let draft = words_file(
        p,
        r#"{"agent": "weaver", "ops": [{"op": "delete", "target": "p-marakest"}]}"#,
    );
    let r = skaldsaga(p, &["propose", "--file", &draft]);
    assert!(!r.status.success());
    let inbox = p.join(".harness/inbox");
    let files = std::fs::read_dir(inbox)
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().ends_with(".json"))
        .count();
    assert_eq!(files, 0);
    // And a folder that isn't set up refuses before reading anything.
    let bare = TempDir::new();
    let d2 = words_file(bare.path(), "{}");
    assert!(
        !skaldsaga(bare.path(), &["propose", "--file", &d2])
            .status
            .success()
    );
    assert!(!bare.path().join(".harness").exists());
}

/// The drafts the opener files, exactly as the skill writes them.
const OPENER_DRAFT: &str = r#"{
  "answer": "A courier smuggles letters past a blockade. It begins with her first run. It ends with her caught.",
  "ops": [
    {"op": "create", "node": {"id": "@a:main", "type": "arc", "name": "Main story",
      "status": "main", "threads": ["@t:main"],
      "premise": {"current": "A courier smuggles letters past a blockade."}}},
    {"op": "create", "node": {"id": "@t:main", "type": "thread", "name": "The main plot",
      "kind": "big_picture", "arc": "@a:main",
      "stages": [
        {"stage": "setup", "summary": "It begins with her first run.", "confidence": "provisional"},
        {"stage": "complications"},
        {"stage": "crisis"},
        {"stage": "resolution", "summary": "It ends with her caught.", "confidence": "provisional"}
      ]}},
    {"op": "create", "node": {"id": "@p:mara", "type": "person", "name": "Mara"}},
    {"op": "create", "node": {"id": "@l:docks", "type": "place", "name": "The east docks"}}
  ]
}"#;

const SCENE_DRAFT: &str = r#"{
  "answer": "The moment she crosses the checkpoint.",
  "ops": [
    {"op": "create", "node": {"id": "@s:checkpoint", "type": "scene", "name": "The checkpoint",
      "status": "slot", "coverage": {"thread": "THREAD", "stage": "setup"}}},
    {"op": "add_item", "target": "sequence", "value": "@s:checkpoint"}
  ]
}"#;

fn file_check_commit(p: &Path, draft: &str) -> serde_json::Value {
    let f = words_file(p, draft);
    let filed = json_of(&skaldsaga(p, &["propose", "--file", &f]));
    let id = filed["id"].as_str().unwrap().to_string();
    let checked = json_of(&skaldsaga(p, &["check", "--proposal", &id]));
    assert_eq!(
        checked["proposals"][0]["status"], "passed",
        "{}",
        checked["proposals"][0]["findings"]
    );
    json_of(&skaldsaga(p, &["commit", "--proposal", &id]))
}

#[test]
fn the_opener_draft_commits_and_a_tied_scene_follows_without_a_flag() {
    let dir = empty_story();
    let p = dir.path();
    let c = file_check_commit(p, OPENER_DRAFT);
    assert_eq!(c["outcome"], "committed");

    // The main plot's real ID comes from the story folder, not from the draft.
    let thread = std::fs::read_dir(p.join("spec/threads"))
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .find(|n| n.ends_with(".md") && n.starts_with("t-"))
        .unwrap();
    let thread_id = thread.trim_end_matches(".md");
    let c = file_check_commit(p, &SCENE_DRAFT.replace("THREAD", thread_id));
    assert_eq!(c["outcome"], "committed");

    // Nothing in the story is untied, so complete lists no untied scene.
    let v = json_of(&skaldsaga(p, &["complete"]));
    let blockers = v["gates"]["complete"]["blockers"].to_string();
    assert!(!blockers.contains("scene_untied"), "{blockers}");
}
