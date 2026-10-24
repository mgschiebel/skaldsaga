//! First-run setup for a fresh project (HANDOFF, Architecture: Starting
//! Skaldsaga).
//!
//! Setup creates the four visible folders (`spec`, `manuscript`, `reviews`,
//! `style-guide`), each with a one-line README, and the hidden `.harness`
//! folder with the default settings. It is safe to run again: it only adds
//! what's missing and never replaces a file, so it also finishes a project
//! whose `.harness` folder was made some other way.
//!
//! Bringing in an existing project (a folder that already holds the writer's
//! files) is a separate, later path. For now setup stops and reports what it
//! found, without touching anything. Git is left to the skill, which can
//! explain any problem to the writer in plain words.

use crate::fsio::{self, FsError};
use crate::json::Value;
use crate::project::Project;
use crate::settings;
use crate::spec;

/// The visible folders, each with the README setup writes into it.
pub const FOLDERS: [(&str, &str); 4] = [
    (
        "spec",
        "Skaldsaga's record of your story: people, places, scenes and every decision you've made. You can read these files any time; changes go through Skaldsaga so it can check them.",
    ),
    (
        "manuscript",
        "Your story's prose: the scenes and drafts you write. Skaldsaga reads them to pick up story facts, but never changes your writing.",
    ),
    (
        "reviews",
        "Feedback on your writing will be kept here in a later version of Skaldsaga.",
    ),
    (
        "style-guide",
        "Notes on how your story sounds (voice, tone, favorite words) will be kept here in a later version of Skaldsaga.",
    ),
];

/// What the hidden folder ignores in git: the meter reading changes every
/// turn and only matters on this computer.
const HARNESS_GITIGNORE: &str = "\
# Changes every turn and only matters on this computer.
meter.json
# Per-machine settings (the writer's own status line); never shared.
local.json
# The text of the next git snapshot, which can hold the writer's own words.
commit-message.txt
";

/// What the story folder's top-level `.gitignore` keeps out of git: personal
/// or secret files (HANDOFF, Current stage, decision 1). The meter's status
/// line lives in `.claude/settings.local.json`, which holds a path tied to
/// this computer and may hold the writer's own secrets.
const ROOT_GITIGNORE_HEADER: &str = "# Skaldsaga: personal or secret files, never shared";
const ROOT_GITIGNORE: [&str; 9] = [
    ".claude/settings.local.json",
    ".claude/*.local.*",
    "CLAUDE.local.md",
    ".env",
    ".env.*",
    "*.pem",
    "*.key",
    ".DS_Store",
    "Thumbs.db",
];

/// Adds the lines from `ROOT_GITIGNORE` that `existing` lacks. Never removes
/// or reorders the writer's own lines. Returns the new text, or `None` when
/// nothing is missing.
fn merged_gitignore(existing: Option<&str>) -> Option<String> {
    let have: std::collections::HashSet<&str> =
        existing.unwrap_or("").lines().map(str::trim).collect();
    let missing: Vec<&str> = ROOT_GITIGNORE
        .iter()
        .copied()
        .filter(|l| !have.contains(l))
        .collect();
    if missing.is_empty() {
        return None;
    }
    let mut text = existing.unwrap_or("").to_string();
    if !text.is_empty() {
        if !text.ends_with('\n') {
            text.push('\n');
        }
        text.push('\n');
    }
    text.push_str(ROOT_GITIGNORE_HEADER);
    text.push('\n');
    for line in missing {
        text.push_str(line);
        text.push('\n');
    }
    Some(text)
}

/// Files that don't make a folder count as holding the writer's work.
const IGNORED_FILES: [&str; 2] = ["Thumbs.db", "desktop.ini"];

/// How many existing entries to list when setup stops.
const MAX_LISTED: usize = 20;

/// What setup did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InitOutcome {
    /// Setup added at least one folder or file.
    Created,
    /// Everything was already in place; nothing changed.
    AlreadySetUp,
    /// The folder already holds files that aren't Skaldsaga's. Nothing was
    /// touched; bringing in an existing project isn't built yet.
    ExistingFiles,
}

impl InitOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            InitOutcome::Created => "created",
            InitOutcome::AlreadySetUp => "already_set_up",
            InitOutcome::ExistingFiles => "existing_files",
        }
    }
}

/// The result of setup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitReport {
    pub outcome: InitOutcome,
    /// Paths setup created or filled in, relative to the project folder.
    pub created: Vec<String>,
    /// For `ExistingFiles`: the first few names found, and how many in all.
    pub existing: Vec<String>,
    pub existing_count: usize,
}

impl InitReport {
    /// The paths the skill should commit after setup: only Skaldsaga's own,
    /// so nothing else in the folder ends up in the writer's history.
    pub fn commit_paths() -> Vec<String> {
        [crate::project::HARNESS_DIR, ".gitignore"]
            .into_iter()
            .chain(FOLDERS.iter().map(|(name, _)| *name))
            .map(str::to_string)
            .collect()
    }
}

/// The writer's own entries in the project folder: everything except hidden
/// files and folders (such as `.git` and `.claude`) and system clutter.
fn writer_entries(project: &Project) -> Result<Vec<String>, FsError> {
    Ok(fsio::list_dir(project.root())?
        .into_iter()
        .map(|e| e.name)
        .filter(|n| !n.starts_with('.') && !IGNORED_FILES.contains(&n.as_str()))
        .collect())
}

fn readme(text: &str) -> String {
    format!("{text}\n")
}

/// Runs first-run setup for a fresh project.
pub fn init(project: &Project) -> Result<InitReport, FsError> {
    if !project.is_initialized() {
        let existing = writer_entries(project)?;
        if !existing.is_empty() {
            return Ok(InitReport {
                outcome: InitOutcome::ExistingFiles,
                created: Vec::new(),
                existing_count: existing.len(),
                existing: existing.into_iter().take(MAX_LISTED).collect(),
            });
        }
    }

    let root = project.root();
    let mut created = Vec::new();
    let mut note = |done: bool, path: &str| {
        if done {
            created.push(path.to_string());
        }
    };

    // The hidden folder first: once it exists, this is a Skaldsaga project.
    note(fsio::ensure_dir(&project.harness_dir())?, ".harness");
    let settings_path = project.settings_path();
    let mut doc = fsio::read_json_object(&settings_path)?.unwrap_or(Value::Null);
    if settings::fill_missing(&mut doc) {
        fsio::write_json(&settings_path, &doc)?;
        note(true, ".harness/settings.json");
    }
    for sub in ["inbox", "sessions"] {
        let keep = project.harness_dir().join(sub).join(".gitkeep");
        note(
            fsio::write_if_missing(&keep, "")?,
            &format!(".harness/{sub}"),
        );
    }
    note(
        fsio::write_if_missing(&project.harness_dir().join(".gitignore"), HARNESS_GITIGNORE)?,
        ".harness/.gitignore",
    );
    // A folder set up by an earlier version lacks the line for local.json.
    note(
        settings::ensure_local_ignored(project)?,
        ".harness/.gitignore",
    );
    note(
        settings::ensure_harness_ignored(
            project,
            crate::commit::MESSAGE_FILE,
            "The text of the next git snapshot, which can hold the writer's own words.",
        )?,
        ".harness/.gitignore",
    );

    let gitignore = project.gitignore_path();
    if let Some(text) = merged_gitignore(fsio::read_optional(&gitignore)?.as_deref()) {
        fsio::write_atomic(&gitignore, &text)?;
        note(true, ".gitignore");
    }

    for (name, text) in FOLDERS {
        let dir = root.join(name);
        note(fsio::ensure_dir(&dir)?, name);
        note(
            fsio::write_if_missing(&dir.join("README.md"), &readme(text))?,
            &format!("{name}/README.md"),
        );
    }
    note(
        fsio::write_if_missing(
            &root.join("spec").join(spec::SEQUENCE_FILE),
            &spec::render_sequence(&[]),
        )?,
        "spec/sequence.md",
    );

    Ok(InitReport {
        outcome: if created.is_empty() {
            InitOutcome::AlreadySetUp
        } else {
            InitOutcome::Created
        },
        created,
        existing: Vec::new(),
        existing_count: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::ValueExt as _;
    use crate::settings::MeterSettings;
    use crate::testutil::TempDir;

    #[test]
    fn sets_up_an_empty_folder() {
        let dir = TempDir::new();
        let project = Project::new(dir.path());
        let report = init(&project).unwrap();
        assert_eq!(report.outcome, InitOutcome::Created);
        assert!(project.is_initialized());
        for (name, _) in FOLDERS {
            assert!(dir.path().join(name).join("README.md").is_file(), "{name}");
        }
        assert!(dir.path().join(".harness/inbox/.gitkeep").is_file());
        assert!(dir.path().join(".harness/sessions/.gitkeep").is_file());
        let ignore = std::fs::read_to_string(dir.path().join(".harness/.gitignore")).unwrap();
        assert!(ignore.lines().any(|l| l == "meter.json"));

        let settings = fsio::read_json_object(&project.settings_path())
            .unwrap()
            .unwrap();
        assert_eq!(
            settings.to_pretty_string(),
            settings::default_settings().to_pretty_string()
        );
        assert_eq!(
            MeterSettings::load(&project).unwrap(),
            MeterSettings::default()
        );

        // The empty spec loads cleanly.
        let loaded = spec::load(&dir.path().join("spec")).unwrap();
        assert_eq!(loaded, spec::Loaded::default());
    }

    #[test]
    fn running_again_changes_nothing() {
        let dir = TempDir::new();
        let project = Project::new(dir.path());
        init(&project).unwrap();
        std::fs::write(dir.path().join("manuscript/README.md"), "my own words\n").unwrap();
        let report = init(&project).unwrap();
        assert_eq!(report.outcome, InitOutcome::AlreadySetUp);
        assert!(report.created.is_empty());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("manuscript/README.md")).unwrap(),
            "my own words\n"
        );
    }

    #[test]
    fn hidden_files_still_count_as_fresh() {
        let dir = TempDir::new();
        for hidden in [".git", ".claude", ".cache"] {
            std::fs::create_dir(dir.path().join(hidden)).unwrap();
        }
        std::fs::write(dir.path().join(".DS_Store"), "").unwrap();
        std::fs::write(dir.path().join("Thumbs.db"), "").unwrap();
        let report = init(&Project::new(dir.path())).unwrap();
        assert_eq!(report.outcome, InitOutcome::Created);
    }

    #[test]
    fn stops_without_touching_an_existing_project() {
        let dir = TempDir::new();
        std::fs::write(dir.path().join("chapter one.docx"), "draft").unwrap();
        std::fs::create_dir(dir.path().join("notes")).unwrap();
        let project = Project::new(dir.path());
        let report = init(&project).unwrap();
        assert_eq!(report.outcome, InitOutcome::ExistingFiles);
        assert_eq!(report.existing, vec!["chapter one.docx", "notes"]);
        assert_eq!(report.existing_count, 2);
        assert!(!project.is_initialized());
        assert!(!dir.path().join("spec").exists());
    }

    #[test]
    fn lists_at_most_twenty_existing_entries() {
        let dir = TempDir::new();
        for n in 0..25 {
            std::fs::write(dir.path().join(format!("draft-{n:02}.md")), "").unwrap();
        }
        let report = init(&Project::new(dir.path())).unwrap();
        assert_eq!(report.existing.len(), 20);
        assert_eq!(report.existing_count, 25);
    }

    #[test]
    fn finishes_a_project_whose_harness_was_made_elsewhere() {
        // The cloud test setup script makes `.harness` before the session,
        // and the meter may already have written settings.
        let dir = TempDir::new();
        let project = Project::new(dir.path());
        std::fs::create_dir_all(project.harness_dir()).unwrap();
        std::fs::write(
            project.settings_path(),
            r#"{"meter":{"wrap_up_percent":65}}"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("leftover.txt"), "").unwrap();
        let report = init(&project).unwrap();
        assert_eq!(report.outcome, InitOutcome::Created);
        assert!(!report.created.contains(&".harness".to_string()));
        assert!(
            report
                .created
                .contains(&".harness/settings.json".to_string())
        );
        assert!(report.created.contains(&"spec".to_string()));
        let s = MeterSettings::load(&project).unwrap();
        assert_eq!(s.wrap_up_percent, 65.0);
    }

    #[test]
    fn commit_paths_are_only_skaldsagas() {
        assert_eq!(
            InitReport::commit_paths(),
            vec![
                ".harness",
                ".gitignore",
                "spec",
                "manuscript",
                "reviews",
                "style-guide"
            ]
        );
    }

    #[test]
    fn writes_a_gitignore_for_personal_files() {
        let dir = TempDir::new();
        let report = init(&Project::new(dir.path())).unwrap();
        assert!(report.created.contains(&".gitignore".to_string()));
        let text = std::fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        assert!(text.starts_with(ROOT_GITIGNORE_HEADER));
        for line in ROOT_GITIGNORE {
            assert!(text.lines().any(|l| l == line), "{line}");
        }
    }

    #[test]
    fn appends_only_missing_lines_to_an_existing_gitignore() {
        let dir = TempDir::new();
        let path = dir.path().join(".gitignore");
        std::fs::write(&path, "build/\n.env").unwrap();
        init(&Project::new(dir.path())).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("build/\n.env\n\n"), "{text}");
        assert_eq!(text.lines().filter(|l| *l == ".env").count(), 1);
        assert!(text.lines().any(|l| l == ".claude/settings.local.json"));
        // Running again adds nothing.
        let report = init(&Project::new(dir.path())).unwrap();
        assert!(!report.created.contains(&".gitignore".to_string()));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
    }
}
