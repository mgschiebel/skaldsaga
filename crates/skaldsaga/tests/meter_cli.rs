//! End-to-end tests for `skaldsaga meter ...`, running the real binary the
//! way Claude Code and the skills will.

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
            std::env::temp_dir().join(format!("skaldsaga-cli-test-{}-{n}", std::process::id()));
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

fn bin() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_skaldsaga"));
    c.env_remove("CLAUDE_PROJECT_DIR")
        .env_remove("CLAUDE_CONFIG_DIR");
    c
}

fn run(args: &[&str], cwd: &Path, stdin: Option<&str>) -> Output {
    let mut child = bin()
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let mut pipe = child.stdin.take().unwrap();
        if let Some(text) = stdin {
            pipe.write_all(text.as_bytes()).unwrap();
        }
    }
    child.wait_with_output().unwrap()
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn novel(dir: &TempDir) -> PathBuf {
    let root = dir.path().join("novel");
    std::fs::create_dir_all(root.join(".harness")).unwrap();
    root
}

fn status_json(project: &Path, session: &str, used: &str) -> String {
    format!(
        r#"{{"session_id":"{session}","workspace":{{"project_dir":"{}"}},"context_window":{{"used_percentage":{used},"context_window_size":200000}}}}"#,
        project.display()
    )
}

#[test]
fn version_and_help() {
    let dir = TempDir::new();
    let v = run(&["--version"], dir.path(), None);
    assert!(v.status.success());
    assert_eq!(
        stdout(&v),
        format!("skaldsaga {}\n", env!("CARGO_PKG_VERSION"))
    );
    assert!(stdout(&run(&[], dir.path(), None)).contains("meter statusline"));
}

#[test]
fn unknown_commands_and_bad_meter_flags_are_usage_errors() {
    let dir = TempDir::new();
    assert_eq!(
        run(&["frobnicate"], dir.path(), None).status.code(),
        Some(64)
    );
    assert_eq!(
        run(&["meter", "check", "--bogus", "x"], dir.path(), None)
            .status
            .code(),
        Some(64)
    );
    assert_eq!(
        run(&["meter", "check", "--session"], dir.path(), None)
            .status
            .code(),
        Some(64)
    );
}

#[test]
fn statusline_records_and_check_reports() {
    let dir = TempDir::new();
    let project = novel(&dir);

    let o = run(
        &["meter", "statusline"],
        dir.path(),
        Some(&status_json(&project, "s1", "55.2")),
    );
    assert!(o.status.success());
    assert_eq!(stdout(&o), "Skaldsaga · session 55% full · a break soon\n");
    assert!(project.join(".harness/meter.json").is_file());

    let check = run(
        &[
            "meter",
            "check",
            "--project",
            project.to_str().unwrap(),
            "--session",
            "s1",
        ],
        dir.path(),
        None,
    );
    assert!(check.status.success());
    let text = stdout(&check);
    assert!(text.contains(r#""state": "suggest_wrap_up""#), "{text}");
    assert!(text.contains(r#""used_percentage": 55.2"#), "{text}");

    let other = run(
        &[
            "meter",
            "check",
            "--project",
            project.to_str().unwrap(),
            "--session",
            "s2",
        ],
        dir.path(),
        None,
    );
    assert!(stdout(&other).contains(r#""state": "unknown""#));
}

#[test]
fn check_uses_claude_project_dir_and_settings() {
    let dir = TempDir::new();
    let project = novel(&dir);
    std::fs::write(
        project.join(".harness/settings.json"),
        r#"{"meter":{"suggest_wrap_up_percent":20,"wrap_up_percent":30}}"#,
    )
    .unwrap();
    run(
        &["meter", "statusline"],
        dir.path(),
        Some(&status_json(&project, "s1", "31")),
    );
    let o = bin()
        .args(["meter", "check"])
        .env("CLAUDE_PROJECT_DIR", &project)
        .output()
        .unwrap();
    let text = stdout(&o);
    assert!(text.contains(r#""state": "wrap_up""#), "{text}");
    assert!(text.contains(r#""wrap_up_percent": 30"#), "{text}");
}

#[test]
fn reset_clears_the_reading() {
    let dir = TempDir::new();
    let project = novel(&dir);
    run(
        &["meter", "statusline"],
        dir.path(),
        Some(&status_json(&project, "s1", "90")),
    );
    let p = project.to_str().unwrap();
    assert!(
        run(&["meter", "reset", "--project", p], dir.path(), None)
            .status
            .success()
    );
    assert!(!project.join(".harness/meter.json").exists());
    assert!(
        stdout(&run(&["meter", "check", "--project", p], dir.path(), None))
            .contains(r#""state": "unknown""#)
    );
    // Outside a project, reset quietly does nothing.
    assert!(
        run(
            &["meter", "reset", "--project", dir.path().to_str().unwrap()],
            dir.path(),
            None
        )
        .status
        .success()
    );
}

#[test]
fn statusline_stays_quiet_outside_a_project() {
    let dir = TempDir::new();
    let o = run(
        &["meter", "statusline"],
        dir.path(),
        Some(&status_json(dir.path(), "s1", "40")),
    );
    assert!(o.status.success());
    assert_eq!(stdout(&o), "\n");
    assert!(!dir.path().join(".harness").exists());
}

#[test]
fn statusline_never_fails_on_garbage() {
    let dir = TempDir::new();
    let project = novel(&dir);
    for input in [
        "",
        "not json",
        "[1,2]",
        r#"{"context_window":{"used_percentage":"lots"}}"#,
    ] {
        let o = bin()
            .args(["meter", "statusline"])
            .env("CLAUDE_PROJECT_DIR", &project)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .and_then(|mut c| {
                c.stdin.take().unwrap().write_all(input.as_bytes())?;
                c.wait_with_output()
            })
            .unwrap();
        assert!(o.status.success(), "{input:?}");
        assert_eq!(stdout(&o), "Skaldsaga\n", "{input:?}");
    }
}

#[cfg(unix)]
#[test]
fn wraps_the_writers_status_line() {
    let dir = TempDir::new();
    let project = novel(&dir);
    let user = dir.path().join("home/.claude/settings.json");
    std::fs::create_dir_all(user.parent().unwrap()).unwrap();
    // The writer's own line reads the same JSON on stdin.
    std::fs::write(
        &user,
        r#"{"statusLine":{"type":"command","command":"printf 'mine:'; cat | head -c 13"}}"#,
    )
    .unwrap();

    let cmd = "sh /plugin/scripts/skaldsaga meter statusline";
    let p = project.to_str().unwrap();
    let o = run(
        &[
            "meter",
            "wire",
            "--command",
            cmd,
            "--project",
            p,
            "--user-settings",
            user.to_str().unwrap(),
        ],
        dir.path(),
        None,
    );
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert!(stdout(&o).contains(r#""outcome": "wired""#));
    let project_settings =
        std::fs::read_to_string(project.join(".claude/settings.local.json")).unwrap();
    assert!(project_settings.contains("meter statusline"));

    let line = run(
        &["meter", "statusline"],
        dir.path(),
        Some(&status_json(&project, "s1", "12")),
    );
    assert_eq!(stdout(&line), r#"mine:{"session_id""#);
    assert!(
        project.join(".harness/meter.json").is_file(),
        "usage still recorded"
    );

    let again = run(
        &[
            "meter",
            "wire",
            "--command",
            cmd,
            "--project",
            p,
            "--user-settings",
            user.to_str().unwrap(),
        ],
        dir.path(),
        None,
    );
    assert!(stdout(&again).contains(r#""outcome": "already_wired""#));
}

#[cfg(unix)]
#[test]
fn broken_wrapped_line_falls_back_to_ours() {
    let dir = TempDir::new();
    let project = novel(&dir);
    std::fs::write(
        project.join(".harness/settings.json"),
        r#"{"meter":{"wrapped_statusline":{"type":"command","command":"exit 1"}}}"#,
    )
    .unwrap();
    let o = run(
        &["meter", "statusline"],
        dir.path(),
        Some(&status_json(&project, "s1", "80")),
    );
    assert_eq!(
        stdout(&o),
        "Skaldsaga · session 80% full · time to wrap up\n"
    );
}

#[test]
fn wire_requires_setup_and_a_meter_command() {
    let dir = TempDir::new();
    let p = dir.path().to_str().unwrap();
    let o = run(
        &[
            "meter",
            "wire",
            "--command",
            "x meter statusline",
            "--project",
            p,
        ],
        dir.path(),
        None,
    );
    assert_eq!(o.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&o.stderr).contains("not a Skaldsaga project"));
    let o = run(
        &["meter", "wire", "--command", "echo hi", "--project", p],
        dir.path(),
        None,
    );
    assert_eq!(o.status.code(), Some(64));
    let o = run(&["meter", "wire", "--project", p], dir.path(), None);
    assert_eq!(o.status.code(), Some(64));
}
