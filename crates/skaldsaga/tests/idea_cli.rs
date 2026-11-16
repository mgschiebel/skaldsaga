//! End-to-end tests for `skaldsaga idea add|mark|list` and `skaldsaga check
//! --ripple` (brainstorm path, part 1). The unit tests cover the rules; these
//! check the real binary: that a write names its file for git and returns a
//! message file only when something changed (it makes no snapshot of its own,
//! the skill does), that `list` and the ripple preview only read, and that
//! unusable input is refused with nothing written.
#![expect(
    clippy::unwrap_used,
    reason = "a test that cannot go on should fail loudly, and a test failure is a panic"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("skaldsaga-idea-test-{}-{n}", std::process::id()));
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

fn text_file(dir: &Path, name: &str, text: &str) -> String {
    let path = dir.join(name);
    std::fs::write(&path, text).unwrap();
    path.display().to_string()
}

/// Every file under `dir` with its bytes, to prove a command changed nothing.
fn snapshot(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, out);
            } else if path.parent() == Some(root)
                && path.extension().is_some_and(|e| e == "txt" || e == "json")
            {
                // The test's own input files, written beside the story.
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

fn add(dir: &Path, text: &str, origin: &str, touches: Option<&str>) -> serde_json::Value {
    let file = text_file(dir, "idea.txt", text);
    let mut args = vec!["idea", "add", "--text-file", &file, "--origin", origin];
    if let Some(t) = touches {
        args.extend(["--touches", t]);
    }
    json_of(&skaldsaga(dir, &args))
}

#[test]
fn saves_an_idea_and_names_its_file_and_a_message_for_git() {
    let dir = project();
    let p = dir.path();
    let problems_before = json_of(&skaldsaga(p, &["index"]))["problems"].clone();
    let v = add(
        p,
        "The informant is Jonah's sister.",
        "brainstorm",
        Some("p-maraharb,crisis"),
    );
    assert_eq!(v["outcome"], "saved");
    assert_eq!(v["status"], "held");
    let id = v["id"].as_str().unwrap();
    assert!(id.starts_with("idea-"), "{id}");
    assert_eq!(
        v["add_paths"],
        serde_json::json!([format!("spec/ideas/{id}.md")])
    );
    // The write makes no snapshot itself, but returns a message file the skill
    // can commit with. The message holds no idea text (SR-32).
    assert_eq!(v["message_file"], ".harness/commit-message.txt");
    let message = std::fs::read_to_string(p.join(".harness/commit-message.txt")).unwrap();
    assert!(!message.contains("informant"), "{message}");
    let saved = std::fs::read_to_string(p.join(format!("spec/ideas/{id}.md"))).unwrap();
    assert!(saved.contains("type: idea"), "{saved}");
    assert!(saved.contains("origin: brainstorm"), "{saved}");
    // The story still loads with no new problems (the test story has its own).
    let index = json_of(&skaldsaga(p, &["index"]));
    assert_eq!(index["problems"], problems_before);
}

#[test]
fn the_same_idea_is_not_saved_twice() {
    let dir = project();
    let p = dir.path();
    let first = add(p, "A dark alley.", "tangent", None);
    let before = snapshot(p);
    let again = add(p, "a dark alley.", "brainstorm", None);
    assert_eq!(again["outcome"], "exists");
    assert_eq!(again["id"], first["id"]);
    assert_eq!(again["add_paths"], serde_json::json!([]));
    assert!(again.get("message_file").is_none());
    assert!(
        snapshot(p) == before,
        "a repeat save changed the story folder"
    );
}

#[test]
fn marks_an_idea_and_lists_only_held_ones() {
    let dir = project();
    let p = dir.path();
    let alley = add(p, "A dark alley.", "tangent", Some("setup"));
    let sister = add(
        p,
        "The informant is Jonah's sister.",
        "brainstorm",
        Some("p-maraharb"),
    );
    let alley_id = alley["id"].as_str().unwrap();
    let sister_id = sister["id"].as_str().unwrap();

    let all = json_of(&skaldsaga(p, &["idea", "list"]));
    assert_eq!(all["outcome"], "listed");
    assert_eq!(all["ideas"].as_array().unwrap().len(), 2);
    let by_person = json_of(&skaldsaga(p, &["idea", "list", "--touching", "p-maraharb"]));
    let ideas = by_person["ideas"].as_array().unwrap();
    assert_eq!(ideas.len(), 1);
    assert_eq!(ideas[0]["id"], sister_id);
    assert_eq!(ideas[0]["origin"], "brainstorm");
    assert_eq!(ideas[0]["touches"], serde_json::json!(["p-maraharb"]));
    assert_eq!(ideas[0]["text"], "The informant is Jonah's sister.");

    let m = json_of(&skaldsaga(
        p,
        &["idea", "mark", "--id", alley_id, "--status", "set_aside"],
    ));
    assert_eq!(m["outcome"], "marked");
    assert_eq!(m["status"], "set_aside");
    assert_eq!(
        m["add_paths"],
        serde_json::json!([format!("spec/ideas/{alley_id}.md")])
    );
    assert_eq!(m["message_file"], ".harness/commit-message.txt");
    let again = json_of(&skaldsaga(
        p,
        &["idea", "mark", "--id", alley_id, "--status", "set_aside"],
    ));
    assert_eq!(again["outcome"], "unchanged");
    assert_eq!(again["add_paths"], serde_json::json!([]));
    assert!(again.get("message_file").is_none());
    let m = json_of(&skaldsaga(
        p,
        &["idea", "mark", "--id", sister_id, "--status", "picked"],
    ));
    assert_eq!(m["status"], "picked");

    // Neither a set-aside nor a picked idea is listed any more.
    let left = json_of(&skaldsaga(p, &["idea", "list"]));
    assert_eq!(left["ideas"], serde_json::json!([]));
}

#[test]
fn list_and_ripple_only_read() {
    let dir = project();
    let p = dir.path();
    add(p, "A dark alley.", "tangent", Some("setup"));
    let draft = text_file(
        p,
        "change.json",
        r#"{"ops": [{"op": "supersede", "target": "p-jnahharb#f-jnspyhrb", "replacement":
          {"id": "f-jnspy2nd", "kind": "truth", "statement": "Jonah reports to the harbor master",
           "confidence": "provisional", "review": "unreviewed", "source": "interview"}}]}"#,
    );
    let before = snapshot(p);
    json_of(&skaldsaga(p, &["idea", "list"]));
    json_of(&skaldsaga(p, &["idea", "list", "--touching", "setup"]));
    json_of(&skaldsaga(p, &["check", "--ripple", &draft]));
    assert!(snapshot(p) == before, "a read changed the story folder");
}

#[test]
fn ripple_lists_what_would_stop_fitting_and_files_nothing() {
    let dir = project();
    let p = dir.path();
    let breaking = text_file(
        p,
        "breaking.json",
        r#"{"answer": "Jonah really works for the harbor master", "ops": [
          {"op": "supersede", "target": "p-jnahharb#f-jnspyhrb", "replacement":
          {"id": "f-jnspy2nd", "kind": "truth", "statement": "Jonah reports to the harbor master",
           "confidence": "provisional", "review": "unreviewed", "source": "interview"}}]}"#,
    );
    let v = json_of(&skaldsaga(p, &["check", "--ripple", &breaking]));
    assert_eq!(v["outcome"], "ripple");
    assert_eq!(v["would_break"], true);
    let breaks = v["breaks"].as_array().unwrap();
    assert_eq!(breaks.len(), 1, "{v}");
    assert_eq!(breaks[0]["code"], "reads_superseded");
    assert!(v["unusable"].is_null());
    // Only what would break: no lane, fan-out or other routing detail.
    assert!(v.get("lane").is_none() && v.get("fan_out").is_none());
    // Nothing was filed: the inbox holds only its placeholder file.
    let inbox: Vec<_> = std::fs::read_dir(p.join(".harness/inbox"))
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(inbox, [std::ffi::OsString::from(".gitkeep")]);

    let fine = text_file(
        p,
        "fine.json",
        r#"{"ops": [{"op": "set", "target": "p-dadsharb.summary", "value": "Ada's father"}]}"#,
    );
    let v = json_of(&skaldsaga(p, &["check", "--ripple", &fine]));
    assert_eq!(v["would_break"], false);
    assert_eq!(v["breaks"], serde_json::json!([]));
}

#[test]
fn ripple_says_so_when_the_change_cant_apply() {
    let dir = project();
    let p = dir.path();
    let draft = text_file(
        p,
        "bad.json",
        r#"{"ops": [{"op": "delete", "target": "p-nnnnnnnn"}]}"#,
    );
    let v = json_of(&skaldsaga(p, &["check", "--ripple", &draft]));
    assert_eq!(v["would_break"], false);
    assert!(
        v["unusable"]
            .as_str()
            .unwrap()
            .starts_with("step 1 (delete)")
    );
}

#[test]
fn refuses_unusable_input_and_writes_nothing() {
    let dir = project();
    let p = dir.path();
    let before = snapshot(p);
    let ok = text_file(p, "idea.txt", "A dark alley.");

    let long = text_file(p, "long.txt", &"x".repeat(301));
    let mut cases: Vec<(Vec<&str>, &str)> = vec![
        (
            vec!["idea", "add", "--text-file", &long, "--origin", "tangent"],
            "300",
        ),
        (
            vec!["idea", "add", "--text-file", &ok, "--origin", "daydream"],
            "--origin",
        ),
        (
            vec![
                "idea",
                "add",
                "--text-file",
                &ok,
                "--origin",
                "tangent",
                "--touches",
                "middle",
            ],
            "can touch",
        ),
        (
            vec![
                "idea",
                "add",
                "--text-file",
                &ok,
                "--origin",
                "tangent",
                "--touches",
                "p-zzzzzzzz",
            ],
            "isn't a part",
        ),
        (
            vec!["idea", "add", "--origin", "tangent"],
            "--text-file is required",
        ),
        (
            vec![
                "idea",
                "add",
                "--text-file",
                "/nonexistent/idea.txt",
                "--origin",
                "tangent",
            ],
            "doesn't exist",
        ),
        (
            vec![
                "idea",
                "mark",
                "--id",
                "../etc/passwd",
                "--status",
                "picked",
            ],
            "isn't an idea ID",
        ),
        (
            vec![
                "idea",
                "mark",
                "--id",
                "idea-zzzzzzzz",
                "--status",
                "picked",
            ],
            "no saved idea",
        ),
        (
            vec!["idea", "mark", "--id", "idea-zzzzzzzz", "--status", "held"],
            "--status",
        ),
        (
            vec!["idea", "list", "--touching", "not a part"],
            "isn't a story part",
        ),
        (vec!["idea", "bogus"], "unknown idea command"),
        (
            vec!["check", "--ripple", &ok, "--proposal", "pr-aaaaaaaa"],
            "can't be used with",
        ),
    ];
    let bad_json = text_file(p, "notjson.json", "not json");
    let empty_ops = text_file(p, "empty.json", r#"{"ops": []}"#);
    cases.push((vec!["check", "--ripple", &bad_json], "expected shape"));
    cases.push((vec!["check", "--ripple", &empty_ops], "changes nothing"));
    for (args, expect) in cases {
        let out = skaldsaga(p, &args);
        assert!(!out.status.success(), "{args:?} should be refused");
        assert!(
            stderr_of(&out).contains(expect),
            "{args:?}: {}",
            stderr_of(&out)
        );
    }
    assert!(snapshot(p) == before, "a refusal changed the story folder");
}

#[test]
fn refuses_outside_a_project() {
    let dir = TempDir::new();
    let file = text_file(dir.path(), "idea.txt", "A dark alley.");
    for args in [
        vec!["idea", "add", "--text-file", &file, "--origin", "tangent"],
        vec!["idea", "list"],
    ] {
        let out = skaldsaga(dir.path(), &args);
        assert!(!out.status.success());
        assert!(
            stderr_of(&out).contains("isn't set up"),
            "{}",
            stderr_of(&out)
        );
    }
}
