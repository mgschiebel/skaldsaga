//! Wiring the meter into Claude Code's status line.
//!
//! A plugin cannot register the main status line, so first-run setup writes
//! it into the project's personal settings, `.claude/settings.local.json`
//! (HANDOFF, Current stage, decision 1). That file stays out of git: the
//! entry holds a path tied to this computer, and a status line is a command
//! Claude Code runs on its own, so a shared one would run for anyone who
//! clones the story. If the writer already has a status line of their own,
//! the meter wraps it: their command keeps running and its output is what
//! they see, while the meter records usage quietly.
//!
//! Claude Code reads settings in this order (highest first): managed, command
//! line, `.claude/settings.local.json`, `.claude/settings.json`, then
//! `~/.claude/settings.json`. The writer's own line is looked for in that
//! order too. The shared `.claude/settings.json` is edited only to take out
//! a meter entry that version 0.3.0 put there (`retire_shared`); the
//! writer's own entries in it are never touched.

use std::path::Path;

use crate::fsio::{self, FsError};
use crate::json::{self, Value, ValueExt as _};
use crate::project::Project;
use crate::settings;

/// The subcommand that marks a status line command as the meter's.
pub const METER_SUBCOMMAND: &str = "meter statusline";

/// True if a status line entry runs the meter.
pub fn is_meter_command(statusline: &Value) -> bool {
    statusline
        .get("command")
        .and_then(Value::as_str)
        .is_some_and(|c| c.contains(METER_SUBCOMMAND))
}

/// What wiring did.
#[derive(Debug, Clone, PartialEq)]
pub enum WireOutcome {
    /// The meter was already wired with this exact command; nothing changed.
    AlreadyWired,
    /// The meter was wired before, under a different command (for example the
    /// plugin moved); the command was updated.
    Updated,
    /// The meter is now wired. `wrapped` holds the writer's own status line,
    /// if there was one, which the meter now runs.
    Wired { wrapped: Option<Value> },
}

/// Errors from wiring.
#[derive(Debug)]
pub enum WireError {
    /// First-run setup hasn't created `.harness/` yet.
    NotInitialized,
    Fs(FsError),
}

impl std::fmt::Display for WireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WireError::NotInitialized => write!(
                f,
                "this folder is not a Skaldsaga project yet (no .harness folder)"
            ),
            WireError::Fs(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for WireError {}

impl From<FsError> for WireError {
    fn from(e: FsError) -> Self {
        WireError::Fs(e)
    }
}

fn statusline_in(doc: Option<&Value>) -> Option<Value> {
    doc.and_then(|d| d.get("statusLine"))
        .filter(|s| s.is_object())
        .cloned()
}

/// Wires the meter into the project's personal status line.
///
/// * `command` is the shell command Claude Code should run (the launcher plus
///   `meter statusline`).
/// * `user_settings` is the writer's `~/.claude/settings.json`, checked for a
///   personal status line to wrap.
pub fn wire(
    project: &Project,
    command: &str,
    user_settings: Option<&Path>,
) -> Result<WireOutcome, WireError> {
    if !project.is_initialized() {
        return Err(WireError::NotInitialized);
    }

    let local_path = project.claude_local_settings_path();
    let mut local_doc = fsio::read_json_object(&local_path)?.unwrap_or_else(json::object);
    let current = statusline_in(Some(&local_doc));

    if let Some(current) = current.as_ref().filter(|s| is_meter_command(s)) {
        if current.get("command").and_then(Value::as_str) == Some(command) {
            return Ok(WireOutcome::AlreadyWired);
        }
        let mut updated = current.clone();
        updated.set("command", command.into());
        local_doc.set("statusLine", updated);
        project.write_json(&local_path, &local_doc)?;
        return Ok(WireOutcome::Updated);
    }

    // The writer's own line, in Claude Code's order: personal project file,
    // shared project file, then their personal settings.
    let wrapped = match current {
        Some(line) => Some(line),
        None => {
            let shared = fsio::read_json_object(&project.claude_settings_path())?;
            match statusline_in(shared.as_ref()) {
                // A meter entry from an earlier version: ours, never wrapped.
                Some(line) if is_meter_command(&line) => None,
                Some(line) => Some(line),
                None => match user_settings {
                    Some(path) => statusline_in(fsio::read_user_json_object(path)?.as_ref())
                        .filter(|s| !is_meter_command(s)),
                    None => None,
                },
            }
        }
    };

    // Record the wrapped line first: if writing the settings then fails,
    // nothing visible has changed.
    if wrapped.is_some() {
        settings::store_wrapped_statusline(project, wrapped.clone())?;
    }

    let mut ours = json::object();
    ours.set("type", "command".into());
    ours.set("command", command.into());
    if let Some(padding) = wrapped.as_ref().and_then(|w| w.get("padding")).cloned() {
        ours.set("padding", padding);
    }
    local_doc.set("statusLine", ours);
    project.write_json(&local_path, &local_doc)?;
    Ok(WireOutcome::Wired { wrapped })
}

/// What `retire_shared` did to the shared `.claude/settings.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Retired {
    /// The meter entry was taken out; the writer's other settings stay.
    Edited,
    /// The meter entry was all the file held, so the file was removed.
    Removed,
}

/// Takes out a meter status line that Skaldsaga 0.3.0 wrote into the shared
/// `.claude/settings.json` (HANDOFF, Current stage, decision 1: the meter
/// belongs in the personal file). That entry holds a path tied to one
/// computer and runs for anyone who clones the story. Only a status line
/// that runs the meter is removed; the writer's own status line and any
/// other settings stay as they are. Returns `None` when there was nothing
/// to take out.
pub fn retire_shared(project: &Project) -> Result<Option<Retired>, FsError> {
    let path = project.claude_settings_path();
    let Some(mut doc) = fsio::read_json_object(&path)? else {
        return Ok(None);
    };
    if !statusline_in(Some(&doc)).is_some_and(|s| is_meter_command(&s)) {
        return Ok(None);
    }
    if let Some(map) = doc.as_object_mut() {
        map.shift_remove("statusLine");
        if map.is_empty() {
            project.remove_optional(&path)?;
            return Ok(Some(Retired::Removed));
        }
    }
    project.write_json(&path, &doc)?;
    Ok(Some(Retired::Edited))
}

/// Keeps an existing meter status line pointing at the current launcher.
///
/// The plugin's folder changes when the plugin updates, which would leave
/// the status line running an old path. The session-start hook calls this
/// with the current path. It only ever updates a meter entry that is already
/// in the personal settings: it never wires the meter for a writer who
/// hasn't set it up or who removed it, and never edits the shared file.
/// Returns true if it changed anything.
pub fn refresh(project: &Project, command: &str) -> Result<bool, FsError> {
    let path = project.claude_local_settings_path();
    let Some(mut doc) = fsio::read_json_object(&path)? else {
        return Ok(false);
    };
    let Some(mut current) = statusline_in(Some(&doc)).filter(is_meter_command) else {
        return Ok(false);
    };
    if current.get("command").and_then(Value::as_str) == Some(command) {
        return Ok(false);
    }
    current.set("command", command.into());
    doc.set("statusLine", current);
    project.write_json(&path, &doc)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::parse;
    use crate::settings::MeterSettings;
    use crate::testutil::TempDir;

    const CMD: &str = "sh /plugin/scripts/skaldsaga meter statusline";

    fn setup() -> (TempDir, Project) {
        let dir = TempDir::new();
        let project = Project::new(dir.path().join("novel"));
        std::fs::create_dir_all(project.harness_dir()).unwrap();
        (dir, project)
    }

    fn write(path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn project_statusline(project: &Project) -> Value {
        fsio::read_json_object(&project.claude_local_settings_path())
            .unwrap()
            .unwrap()
            .get("statusLine")
            .unwrap()
            .clone()
    }

    #[test]
    fn refuses_before_setup() {
        let dir = TempDir::new();
        let project = Project::new(dir.path());
        assert!(matches!(
            wire(&project, CMD, None),
            Err(WireError::NotInitialized)
        ));
    }

    #[test]
    fn fresh_project_gets_the_meter() {
        let (_d, project) = setup();
        assert_eq!(
            wire(&project, CMD, None).unwrap(),
            WireOutcome::Wired { wrapped: None }
        );
        assert_eq!(
            project_statusline(&project).to_compact_string(),
            format!(r#"{{"type":"command","command":"{CMD}"}}"#)
        );
        assert_eq!(
            wire(&project, CMD, None).unwrap(),
            WireOutcome::AlreadyWired
        );
    }

    #[test]
    fn keeps_other_personal_settings() {
        let (_d, project) = setup();
        write(
            &project.claude_local_settings_path(),
            r#"{"permissions":{"allow":["Read"]},"model":"opus"}"#,
        );
        wire(&project, CMD, None).unwrap();
        let doc = fsio::read_json_object(&project.claude_local_settings_path())
            .unwrap()
            .unwrap();
        assert!(doc.get("permissions").is_some());
        assert_eq!(doc.get("model").and_then(Value::as_str), Some("opus"));
    }

    #[test]
    fn wraps_a_shared_project_status_line_without_editing_it() {
        let (_d, project) = setup();
        let shared = r#"{"statusLine":{"type":"command","command":"my-line","padding":2}}"#;
        write(&project.claude_settings_path(), shared);
        let outcome = wire(&project, CMD, None).unwrap();
        let mine = parse(r#"{"type":"command","command":"my-line","padding":2}"#).unwrap();
        assert_eq!(
            outcome,
            WireOutcome::Wired {
                wrapped: Some(mine.clone())
            }
        );
        assert_eq!(
            MeterSettings::load(&project).unwrap().wrapped_statusline,
            Some(mine)
        );
        assert_eq!(
            project_statusline(&project).get("padding"),
            Some(&Value::from(2))
        );
        assert_eq!(
            std::fs::read_to_string(project.claude_settings_path()).unwrap(),
            shared
        );
    }

    #[test]
    fn wraps_a_personal_status_line() {
        let (dir, project) = setup();
        let user = dir.path().join("home/.claude/settings.json");
        write(
            &user,
            r#"{"statusLine":{"type":"command","command":"~/.claude/line.sh"}}"#,
        );
        let outcome = wire(&project, CMD, Some(&user)).unwrap();
        assert!(matches!(outcome, WireOutcome::Wired { wrapped: Some(_) }));
        let wrapped = MeterSettings::load(&project)
            .unwrap()
            .wrapped_statusline
            .unwrap();
        assert_eq!(
            wrapped.get("command").and_then(Value::as_str),
            Some("~/.claude/line.sh")
        );
        // The personal settings file is never edited.
        assert!(std::fs::read_to_string(&user).unwrap().contains("line.sh"));
    }

    #[test]
    fn missing_user_settings_file_is_fine() {
        let (dir, project) = setup();
        let user = dir.path().join("nowhere/settings.json");
        assert_eq!(
            wire(&project, CMD, Some(&user)).unwrap(),
            WireOutcome::Wired { wrapped: None }
        );
    }

    #[test]
    fn wraps_a_personal_project_status_line() {
        let (_d, project) = setup();
        write(
            &project.claude_local_settings_path(),
            r#"{"statusLine":{"type":"command","command":"private"}}"#,
        );
        let outcome = wire(&project, CMD, None).unwrap();
        assert!(matches!(outcome, WireOutcome::Wired { wrapped: Some(_) }));
        assert_eq!(
            project_statusline(&project)
                .get("command")
                .and_then(Value::as_str),
            Some(CMD)
        );
        assert!(!project.claude_settings_path().exists());
    }

    #[test]
    fn an_old_shared_meter_entry_is_left_alone_and_not_wrapped() {
        let (_d, project) = setup();
        let old =
            r#"{"statusLine":{"type":"command","command":"sh /old/skaldsaga meter statusline"}}"#;
        write(&project.claude_settings_path(), old);
        assert_eq!(
            wire(&project, CMD, None).unwrap(),
            WireOutcome::Wired { wrapped: None }
        );
        assert_eq!(
            std::fs::read_to_string(project.claude_settings_path()).unwrap(),
            old
        );
    }

    #[test]
    fn retire_removes_an_old_shared_meter_entry() {
        let (_d, project) = setup();
        // Nothing there: nothing to do.
        assert_eq!(retire_shared(&project).unwrap(), None);

        // Only the meter entry: the file goes.
        write(
            &project.claude_settings_path(),
            r#"{"statusLine":{"type":"command","command":"sh /old/skaldsaga meter statusline"}}"#,
        );
        assert_eq!(retire_shared(&project).unwrap(), Some(Retired::Removed));
        assert!(!project.claude_settings_path().exists());

        // Other settings stay, in order.
        write(
            &project.claude_settings_path(),
            r#"{"model":"opus","statusLine":{"type":"command","command":"x meter statusline"},"env":{"A":"1"}}"#,
        );
        assert_eq!(retire_shared(&project).unwrap(), Some(Retired::Edited));
        let doc = fsio::read_json_object(&project.claude_settings_path())
            .unwrap()
            .unwrap();
        assert_eq!(
            doc.to_compact_string(),
            r#"{"model":"opus","env":{"A":"1"}}"#
        );
        assert_eq!(retire_shared(&project).unwrap(), None);
    }

    #[test]
    fn retire_leaves_the_writers_own_status_line() {
        let (_d, project) = setup();
        let mine = r#"{"statusLine":{"type":"command","command":"my-line"}}"#;
        write(&project.claude_settings_path(), mine);
        assert_eq!(retire_shared(&project).unwrap(), None);
        assert_eq!(
            std::fs::read_to_string(project.claude_settings_path()).unwrap(),
            mine
        );
    }

    #[test]
    fn moved_launcher_updates_the_command() {
        let (_d, project) = setup();
        wire(&project, CMD, None).unwrap();
        let moved = "sh /new/place/skaldsaga meter statusline";
        assert_eq!(wire(&project, moved, None).unwrap(), WireOutcome::Updated);
        assert_eq!(
            project_statusline(&project)
                .get("command")
                .and_then(Value::as_str),
            Some(moved)
        );
    }

    #[test]
    fn bad_personal_settings_are_an_error_and_untouched() {
        let (_d, project) = setup();
        write(&project.claude_local_settings_path(), "{ broken");
        assert!(matches!(wire(&project, CMD, None), Err(WireError::Fs(_))));
        assert_eq!(
            std::fs::read_to_string(project.claude_local_settings_path()).unwrap(),
            "{ broken"
        );
    }

    #[test]
    fn refresh_updates_only_an_existing_meter_entry() {
        let (_d, project) = setup();
        let moved = "sh /plugin/v2/scripts/skaldsaga meter statusline";
        // Nothing wired: refresh never wires the meter itself.
        assert!(!refresh(&project, moved).unwrap());
        assert!(!project.claude_local_settings_path().exists());

        // The writer's own line is left alone.
        write(
            &project.claude_local_settings_path(),
            r#"{"statusLine":{"type":"command","command":"mine"}}"#,
        );
        assert!(!refresh(&project, moved).unwrap());

        // A meter entry with an old path is updated, once.
        std::fs::remove_file(project.claude_local_settings_path()).unwrap();
        wire(&project, CMD, None).unwrap();
        assert!(refresh(&project, moved).unwrap());
        assert_eq!(
            project_statusline(&project)
                .get("command")
                .and_then(Value::as_str),
            Some(moved)
        );
        assert!(!refresh(&project, moved).unwrap());
    }
}
