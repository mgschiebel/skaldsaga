//! End-to-end tests for `skaldsaga hook session-start`, run the way Claude
//! Code runs a plugin hook: JSON on stdin, plugin variables in the environment.
#![expect(
    clippy::unwrap_used,
    reason = "a test that cannot go on should fail loudly, and a test failure is a panic"
)]

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("skaldsaga-hook-test-{}-{n}", std::process::id()));
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

fn hook(project: &Path, stdin: &str, plugin_root: Option<&str>) -> Output {
    hook_with(Some(project), stdin, plugin_root, false)
}

/// Runs the hook. `project` sets CLAUDE_PROJECT_DIR (None leaves it empty,
/// as cloud sessions do, so the hook falls back to the payload's `cwd`).
fn hook_with(
    project: Option<&Path>,
    stdin: &str,
    plugin_root: Option<&str>,
    debug: bool,
) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_skaldsaga"));
    cmd.args(["hook", "session-start"])
        .env(
            "CLAUDE_PROJECT_DIR",
            project.map_or("".into(), |p| p.to_path_buf()),
        )
        .env_remove("CLAUDE_PLUGIN_ROOT")
        .env_remove("SKALDSAGA_HOOK_DEBUG")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(root) = plugin_root {
        cmd.env("CLAUDE_PLUGIN_ROOT", root);
    }
    if debug {
        cmd.env("SKALDSAGA_HOOK_DEBUG", "1");
    }
    let mut child = cmd.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

#[test]
fn silent_outside_a_project() {
    let dir = TempDir::new();
    let o = hook(dir.path(), r#"{"source":"startup"}"#, Some("/plugin"));
    assert!(o.status.success());
    assert_eq!(stdout(&o), "");
    assert!(!dir.path().join(".harness").exists());
}

#[test]
fn adds_context_and_resets_the_meter_in_a_project() {
    let dir = TempDir::new();
    let harness = dir.path().join(".harness");
    std::fs::create_dir_all(&harness).unwrap();
    std::fs::write(
        harness.join("meter.json"),
        r#"{"session_id":"old","used_percentage":90,"updated_at":1}"#,
    )
    .unwrap();

    let o = hook(
        dir.path(),
        r#"{"session_id":"new","source":"clear","cwd":"/elsewhere"}"#,
        None,
    );
    assert!(o.status.success());
    let out: serde_json::Value =
        serde_json::from_str(stdout(&o).trim()).expect("hook prints one JSON object");
    assert_eq!(out["hookSpecificOutput"]["hookEventName"], "SessionStart");
    let context = out["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(context.contains("Skaldsaga project"), "{context}");
    assert!(context.contains("clear"), "{context}");
    assert!(!harness.join("meter.json").exists());
}

#[test]
fn refreshes_the_status_line_after_a_plugin_update() {
    let dir = TempDir::new();
    std::fs::create_dir_all(dir.path().join(".harness")).unwrap();
    std::fs::create_dir_all(dir.path().join(".claude")).unwrap();
    std::fs::write(
        dir.path().join(".claude/settings.local.json"),
        r#"{"statusLine":{"type":"command","command":"sh \"/old/scripts/skaldsaga\" meter statusline"}}"#,
    )
    .unwrap();
    let o = hook(dir.path(), r#"{"source":"startup"}"#, Some("/new"));
    assert!(o.status.success());
    let settings = std::fs::read_to_string(dir.path().join(".claude/settings.local.json")).unwrap();
    assert!(settings.contains(r#"/new/scripts/skaldsaga"#), "{settings}");
}

#[test]
fn garbage_input_still_succeeds() {
    let dir = TempDir::new();
    std::fs::create_dir_all(dir.path().join(".harness")).unwrap();
    for input in ["", "not json", "[]"] {
        let o = hook(dir.path(), input, None);
        assert!(o.status.success(), "{input:?}");
        assert!(stdout(&o).contains("additionalContext"), "{input:?}");
    }
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

/// The cloud-session case from the 0.3.0 phone test: the session starts one
/// folder above the story and CLAUDE_PROJECT_DIR is empty.
#[test]
fn finds_a_story_one_level_below_the_session_folder() {
    let dir = TempDir::new();
    let story = dir.path().join("skaldsaga-test");
    std::fs::create_dir_all(story.join(".harness")).unwrap();
    let payload = format!(
        r#"{{"source":"startup","cwd":{}}}"#,
        serde_json::Value::from(dir.path().to_str().unwrap())
    );
    let o = hook_with(None, &payload, Some("/plugin"), false);
    assert!(o.status.success());
    assert!(stdout(&o).contains("Skaldsaga project found in the `skaldsaga-test` folder"));
    assert_eq!(stderr(&o), "", "quiet without the debug switch");
    // Context only: nothing written into the story.
    assert!(!story.join(".claude").exists());
}

#[test]
fn debug_switch_explains_a_silent_hook() {
    let dir = TempDir::new();
    // Without the switch: nothing at all.
    let o = hook_with(Some(dir.path()), r#"{"source":"startup"}"#, None, false);
    assert_eq!(stdout(&o), "");
    assert_eq!(stderr(&o), "");

    // With it: the reason on stderr and in Claude's context, which is the
    // only place Claude sees it at session start.
    let o = hook_with(Some(dir.path()), r#"{"source":"startup"}"#, None, true);
    assert!(o.status.success());
    assert!(stderr(&o).contains("no .harness folder"), "{}", stderr(&o));
    assert!(
        stdout(&o).contains("Skaldsaga hook debug:"),
        "{}",
        stdout(&o)
    );
    assert!(stdout(&o).contains("no .harness folder"), "{}", stdout(&o));
    assert!(!stdout(&o).contains("Skaldsaga project"), "{}", stdout(&o));

    std::fs::create_dir_all(dir.path().join("a/.harness")).unwrap();
    std::fs::create_dir_all(dir.path().join("b/.harness")).unwrap();
    let o = hook_with(Some(dir.path()), r#"{"source":"startup"}"#, None, true);
    assert!(stderr(&o).contains("not guessing"), "{}", stderr(&o));
    assert!(stdout(&o).contains("not guessing"), "{}", stdout(&o));
    assert!(!stdout(&o).contains("Skaldsaga project"), "{}", stdout(&o));
}
