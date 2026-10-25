//! Where Skaldsaga's files live inside a writer's project folder.

use std::path::{Path, PathBuf};

use crate::fsio::{self, FsError};
use crate::json::Value;

/// The hidden folder that marks a folder as a Skaldsaga project.
/// The per-machine settings file inside `.harness/`.
pub const LOCAL_SETTINGS_FILE: &str = "local.json";

pub const HARNESS_DIR: &str = ".harness";

/// Paths inside one project folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    root: PathBuf,
}

impl Project {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Project { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn harness_dir(&self) -> PathBuf {
        self.root.join(HARNESS_DIR)
    }

    /// True once first-run setup has created `.harness/`. Everything that
    /// runs automatically (status line, hooks) checks this first, so
    /// installing the plugin never affects folders that aren't Skaldsaga
    /// projects.
    pub fn is_initialized(&self) -> bool {
        self.harness_dir().is_dir()
    }

    /// The story spec, the source of truth.
    pub fn spec_dir(&self) -> PathBuf {
        self.root.join("spec")
    }

    /// Pending proposals, one JSON file each.
    pub fn inbox_dir(&self) -> PathBuf {
        self.harness_dir().join("inbox")
    }

    pub fn settings_path(&self) -> PathBuf {
        self.harness_dir().join("settings.json")
    }

    /// Per-machine settings that never go into git (the writer's own status
    /// line command). Nothing in this file is read when it is a symlink, and
    /// the meter never runs a command from any committed file (SR-10).
    pub fn local_settings_path(&self) -> PathBuf {
        self.harness_dir().join(LOCAL_SETTINGS_FILE)
    }

    pub fn meter_path(&self) -> PathBuf {
        self.harness_dir().join("meter.json")
    }

    /// Claude Code's shared project settings. Skaldsaga reads it to find a
    /// status line the writer shares with others, and edits it only to take
    /// out a meter entry an earlier version left there.
    pub fn claude_settings_path(&self) -> PathBuf {
        self.root.join(".claude").join("settings.json")
    }

    /// Claude Code's personal project settings, which override the shared file.
    /// The meter's status line is wired here: it holds a machine-specific
    /// path and runs a command, so it must never be shared through git.
    pub fn claude_local_settings_path(&self) -> PathBuf {
        self.root.join(".claude").join("settings.local.json")
    }

    /// The project's top-level `.gitignore`.
    pub fn gitignore_path(&self) -> PathBuf {
        self.root.join(".gitignore")
    }

    // Every write into the story folder goes through these, never straight to
    // `fsio`: each one first checks that no folder on the way is a link
    // (SR-21). A test (`tests/guarded_writes.rs` in the command-line crate)
    // fails if code elsewhere calls the raw writers.

    /// Writes a file atomically, refusing to go through a linked folder.
    pub fn write_atomic(&self, path: &Path, contents: &str) -> Result<(), FsError> {
        fsio::refuse_links_under(&self.root, path)?;
        fsio::write_atomic(path, contents)
    }

    /// Writes a JSON value atomically, refusing to go through a linked folder.
    pub fn write_json(&self, path: &Path, value: &Value) -> Result<(), FsError> {
        fsio::refuse_links_under(&self.root, path)?;
        fsio::write_json(path, value)
    }

    /// Removes a file (already gone is fine), refusing to go through a linked folder.
    pub fn remove_optional(&self, path: &Path) -> Result<bool, FsError> {
        fsio::refuse_links_under(&self.root, path)?;
        fsio::remove_optional(path)
    }

    /// Makes a folder, refusing if it or a folder on the way is a link.
    pub fn ensure_dir(&self, path: &Path) -> Result<bool, FsError> {
        fsio::refuse_links_in(&self.root, path)?;
        fsio::ensure_dir(path)
    }

    /// Writes a file only if nothing is there yet, refusing to go through a
    /// linked folder.
    pub fn write_if_missing(&self, path: &Path, contents: &str) -> Result<bool, FsError> {
        fsio::refuse_links_under(&self.root, path)?;
        fsio::write_if_missing(path, contents)
    }

    /// Checks that none of these folders is a link, before anything is
    /// written, so a job that writes in several places stops before it has
    /// changed anything instead of half way.
    pub fn refuse_links_in(&self, dirs: &[PathBuf]) -> Result<(), FsError> {
        dirs.iter()
            .try_for_each(|dir| fsio::refuse_links_in(&self.root, dir))
    }
}

/// Where the story folder is, seen from a starting folder (the session's
/// folder). The hook and the skill share this lookup so their rules can't
/// drift apart (HANDOFF, Current stage, decision 2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Location {
    /// The starting folder is a Skaldsaga project.
    Here(PathBuf),
    /// Exactly one visible child folder is a Skaldsaga project. Cloud
    /// sessions clone a repository one level below the session folder.
    Below { path: PathBuf, name: String },
    /// Several child folders are projects; the lookup never guesses.
    Several(Vec<String>),
    /// No project yet, but the only visible folder in the starting folder is
    /// a git repository (loose files beside it don't count): probably the
    /// folder to set up.
    Candidate { path: PathBuf, name: String },
    /// No project here or one level down.
    Nothing,
}

impl Location {
    pub fn as_str(&self) -> &'static str {
        match self {
            Location::Here(_) => "here",
            Location::Below { .. } => "below",
            Location::Several(_) => "several",
            Location::Candidate { .. } => "candidate",
            Location::Nothing => "nothing",
        }
    }

    /// The project found, if exactly one was.
    pub fn project(&self) -> Option<Project> {
        match self {
            Location::Here(p) | Location::Below { path: p, .. } => Some(Project::new(p.clone())),
            _ => None,
        }
    }
}

/// Looks for a Skaldsaga project in `start`, then one level down. Never
/// looks upward: local sessions name the project folder directly.
pub fn locate(start: &Path) -> Location {
    if Project::new(start).is_initialized() {
        return Location::Here(start.to_path_buf());
    }
    let entries: Vec<_> = crate::fsio::list_dir(start)
        .unwrap_or_default()
        .into_iter()
        .filter(|e| !e.name.starts_with('.'))
        .collect();
    let stories: Vec<&str> = entries
        .iter()
        .filter(|e| e.is_dir && start.join(&e.name).join(HARNESS_DIR).is_dir())
        .map(|e| e.name.as_str())
        .collect();
    match stories.as_slice() {
        [name] => {
            return Location::Below {
                path: start.join(name),
                name: name.to_string(),
            };
        }
        [] => {}
        many => return Location::Several(many.iter().map(|s| s.to_string()).collect()),
    }
    // Loose files (notes, an uploaded report) don't stop the guess; another
    // visible folder does, since it may hold the writer's work.
    let folders: Vec<_> = entries.iter().filter(|e| e.is_dir).collect();
    match folders.as_slice() {
        [only] if start.join(&only.name).join(".git").exists() => Location::Candidate {
            path: start.join(&only.name),
            name: only.name.clone(),
        },
        _ => Location::Nothing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    fn mkdir(path: &Path) {
        std::fs::create_dir_all(path).unwrap();
    }

    #[test]
    fn finds_a_project_here_first() {
        let dir = TempDir::new();
        mkdir(&dir.path().join(".harness"));
        mkdir(&dir.path().join("child/.harness"));
        assert_eq!(locate(dir.path()), Location::Here(dir.path().to_path_buf()));
    }

    #[test]
    fn finds_exactly_one_project_below() {
        let dir = TempDir::new();
        mkdir(&dir.path().join("story/.harness"));
        mkdir(&dir.path().join("other"));
        std::fs::write(dir.path().join("notes.txt"), "").unwrap();
        let found = locate(dir.path());
        assert_eq!(
            found,
            Location::Below {
                path: dir.path().join("story"),
                name: "story".into()
            }
        );
        assert!(found.project().is_some());
    }

    #[test]
    fn never_guesses_between_several() {
        let dir = TempDir::new();
        mkdir(&dir.path().join("a/.harness"));
        mkdir(&dir.path().join("b/.harness"));
        assert_eq!(
            locate(dir.path()),
            Location::Several(vec!["a".into(), "b".into()])
        );
    }

    #[test]
    fn hidden_children_and_subfolders_dont_count() {
        let dir = TempDir::new();
        mkdir(&dir.path().join(".hidden/.harness"));
        assert_eq!(locate(dir.path()), Location::Nothing);
        // A subfolder of a story doesn't look upward.
        let story = TempDir::new();
        mkdir(&story.path().join(".harness"));
        mkdir(&story.path().join("spec"));
        assert_eq!(locate(&story.path().join("spec")), Location::Nothing);
    }

    #[test]
    fn a_lone_repository_is_a_candidate() {
        let dir = TempDir::new();
        mkdir(&dir.path().join("repo/.git"));
        assert_eq!(
            locate(dir.path()),
            Location::Candidate {
                path: dir.path().join("repo"),
                name: "repo".into()
            }
        );
        // Loose files beside it don't count.
        std::fs::write(dir.path().join("readme.md"), "").unwrap();
        assert!(matches!(locate(dir.path()), Location::Candidate { .. }));
        // Another visible folder does: no guess.
        mkdir(&dir.path().join("notes"));
        assert_eq!(locate(dir.path()), Location::Nothing);
    }

    #[test]
    fn missing_folder_is_nothing() {
        let dir = TempDir::new();
        assert_eq!(locate(&dir.path().join("gone")), Location::Nothing);
    }
}
