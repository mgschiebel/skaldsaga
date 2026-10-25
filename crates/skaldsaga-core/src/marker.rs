//! How the writer's last session ended (HANDOFF, Ready to build, Slice 1,
//! item 2).
//!
//! The next session's recap depends on whether the writer was left in the
//! middle of a thread (with the last answer, confirmed or not) or at a
//! stopping point. The program records that after every answer, and the entry
//! routing reads it, so story text never decides how a session opens.
//!
//! One small file, `.harness/writer/last-session.json`, next to the craft
//! record: writer state, not story meaning, committed to git so it follows the
//! story between sessions and devices. Nothing here is read as an instruction;
//! the writer's words only ever appear as data.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::craft::WRITER_DIR;
use crate::decisions::MAX_WORDS_CHARS;
use crate::fsio::{self, FsError};
use crate::project::Project;

/// Bumped when the file's shape changes, so later versions can migrate it.
pub const FORMAT: u32 = 1;
/// The marker's file name inside `.harness/writer/`.
pub const MARKER_FILE: &str = "last-session.json";
/// The file is tiny; anything bigger isn't ours.
pub const MAX_FILE_BYTES: u64 = 16 * 1024;

/// How the last session ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    /// Left in the middle of a thread; the last answer is kept.
    MidThread,
    /// Left at a stopping point (the writer's signals or an exhausted thread).
    StoppingPoint,
}

impl State {
    pub fn as_str(self) -> &'static str {
        match self {
            State::MidThread => "mid_thread",
            State::StoppingPoint => "stopping_point",
        }
    }
}

/// The writer's last answer, in their own words.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Answer {
    pub words: String,
    /// Whether the writer confirmed the reword of it.
    pub confirmed: bool,
}

/// The stored marker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Marker {
    pub format: u32,
    pub state: State,
    /// Present exactly when `state` is `mid_thread`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_answer: Option<Answer>,
    /// `YYYY-MM-DD`.
    pub date: String,
}

impl Marker {
    /// True when the writer's last answer was never confirmed back.
    pub fn has_unconfirmed_answer(&self) -> bool {
        self.last_answer.as_ref().is_some_and(|a| !a.confirmed)
    }
}

/// Why the marker couldn't be read or written.
#[derive(Debug)]
pub enum MarkerError {
    Fs(FsError),
    /// The input or the stored file was unusable; the message is plain language.
    Invalid(String),
}

impl std::fmt::Display for MarkerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MarkerError::Fs(e) => e.fmt(f),
            MarkerError::Invalid(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for MarkerError {}

impl From<FsError> for MarkerError {
    fn from(e: FsError) -> Self {
        MarkerError::Fs(e)
    }
}

fn invalid<T>(m: impl Into<String>) -> Result<T, MarkerError> {
    Err(MarkerError::Invalid(m.into()))
}

/// Path from the project root, with forward slashes, for git.
pub fn marker_rel() -> String {
    format!(".harness/{WRITER_DIR}/{MARKER_FILE}")
}

fn marker_path(project: &Project) -> PathBuf {
    project.harness_dir().join(WRITER_DIR).join(MARKER_FILE)
}

/// Loads the marker. A missing file is `None`; a file we can't use is an
/// error, so it is never overwritten by accident.
pub fn load(project: &Project) -> Result<Option<Marker>, MarkerError> {
    let rel = marker_rel();
    let text = match fsio::read_regular_capped(&marker_path(project), MAX_FILE_BYTES)? {
        None => return Ok(None),
        Some(Err(why)) => {
            return invalid(format!("{rel}: {}", fsio::refusal_message(&why)));
        }
        Some(Ok(t)) => t,
    };
    let marker: Marker =
        serde_json::from_str(&text).map_err(|e| MarkerError::Invalid(format!("{rel}: {e}")))?;
    if marker.format != FORMAT {
        return invalid(format!("{rel}: format {} isn't supported", marker.format));
    }
    match (marker.state, &marker.last_answer) {
        (State::MidThread, None) => {
            return invalid(format!("{rel}: a thread in progress needs a last answer"));
        }
        (State::StoppingPoint, Some(_)) => {
            return invalid(format!("{rel}: a stopping point has no last answer"));
        }
        _ => {}
    }
    if let Some(a) = &marker.last_answer
        && a.words.chars().count() > MAX_WORDS_CHARS
    {
        return invalid(format!("{rel}: the last answer is too long"));
    }
    Ok(Some(marker))
}

/// What a recording did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recorded {
    /// False when the marker already said this and nothing changed.
    pub changed: bool,
    /// The path to hand to `git add`.
    pub path: String,
}

fn save(project: &Project, marker: &Marker) -> Result<Recorded, MarkerError> {
    let existing = load(project)?;
    // A new day alone doesn't make a change worth a snapshot.
    let same = existing
        .as_ref()
        .is_some_and(|m| m.state == marker.state && m.last_answer == marker.last_answer);
    if same {
        return Ok(Recorded {
            changed: false,
            path: marker_rel(),
        });
    }
    let mut text =
        serde_json::to_string_pretty(marker).map_err(|e| MarkerError::Invalid(e.to_string()))?;
    text.push('\n');
    fsio::write_atomic(&marker_path(project), &text)?;
    Ok(Recorded {
        changed: true,
        path: marker_rel(),
    })
}

/// Records the writer's last answer and whether they confirmed the reword.
pub fn record_answer(
    project: &Project,
    words: &str,
    confirmed: bool,
    date: &str,
) -> Result<Recorded, MarkerError> {
    let words = words.trim();
    if words.is_empty() {
        return invalid("the answer is empty");
    }
    if words.chars().count() > MAX_WORDS_CHARS {
        return invalid("the answer is too long to keep");
    }
    save(
        project,
        &Marker {
            format: FORMAT,
            state: State::MidThread,
            last_answer: Some(Answer {
                words: words.to_string(),
                confirmed,
            }),
            date: date.to_string(),
        },
    )
}

/// Records that the session ended at a stopping point.
pub fn record_stopping_point(project: &Project, date: &str) -> Result<Recorded, MarkerError> {
    save(
        project,
        &Marker {
            format: FORMAT,
            state: State::StoppingPoint,
            last_answer: None,
            date: date.to_string(),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    fn project(dir: &TempDir) -> Project {
        std::fs::create_dir_all(dir.path().join(".harness")).unwrap();
        Project::new(dir.path())
    }

    #[test]
    fn a_missing_marker_is_none() {
        let dir = TempDir::new();
        assert_eq!(load(&project(&dir)).unwrap(), None);
    }

    #[test]
    fn records_an_unconfirmed_answer_and_reads_it_back() {
        let dir = TempDir::new();
        let p = project(&dir);
        let r = record_answer(&p, "A smuggler gets caught", false, "2026-10-25").unwrap();
        assert!(r.changed);
        assert_eq!(r.path, ".harness/writer/last-session.json");
        let m = load(&p).unwrap().unwrap();
        assert_eq!(m.state, State::MidThread);
        assert!(m.has_unconfirmed_answer());
        assert_eq!(m.last_answer.unwrap().words, "A smuggler gets caught");
    }

    #[test]
    fn confirming_the_answer_is_a_change() {
        let dir = TempDir::new();
        let p = project(&dir);
        record_answer(&p, "A smuggler gets caught", false, "2026-10-25").unwrap();
        let r = record_answer(&p, "A smuggler gets caught", true, "2026-10-25").unwrap();
        assert!(r.changed);
        assert!(!load(&p).unwrap().unwrap().has_unconfirmed_answer());
    }

    #[test]
    fn saying_the_same_thing_again_changes_nothing() {
        let dir = TempDir::new();
        let p = project(&dir);
        record_answer(&p, "A smuggler gets caught", true, "2026-10-25").unwrap();
        let r = record_answer(&p, "A smuggler gets caught", true, "2026-10-26").unwrap();
        assert!(!r.changed);
        // The earlier date stays: no needless rewrite.
        assert_eq!(load(&p).unwrap().unwrap().date, "2026-10-25");
    }

    #[test]
    fn a_stopping_point_replaces_the_last_answer() {
        let dir = TempDir::new();
        let p = project(&dir);
        record_answer(&p, "A smuggler gets caught", true, "2026-10-25").unwrap();
        record_stopping_point(&p, "2026-10-25").unwrap();
        let m = load(&p).unwrap().unwrap();
        assert_eq!(m.state, State::StoppingPoint);
        assert_eq!(m.last_answer, None);
        assert!(!m.has_unconfirmed_answer());
    }

    #[test]
    fn refuses_empty_and_oversized_answers() {
        let dir = TempDir::new();
        let p = project(&dir);
        assert!(record_answer(&p, "   ", false, "2026-10-25").is_err());
        let long = "x".repeat(MAX_WORDS_CHARS + 1);
        assert!(record_answer(&p, &long, false, "2026-10-25").is_err());
        assert_eq!(load(&p).unwrap(), None);
    }

    #[test]
    fn a_file_we_cant_use_is_an_error_and_is_never_overwritten() {
        let dir = TempDir::new();
        let p = project(&dir);
        let path = dir.path().join(".harness/writer");
        std::fs::create_dir_all(&path).unwrap();
        let file = path.join(MARKER_FILE);
        for bad in [
            "not json",
            r#"{"format":2,"state":"mid_thread","date":"2026-10-25"}"#,
            r#"{"format":1,"state":"mid_thread","date":"2026-10-25"}"#,
            r#"{"format":1,"state":"stopping_point","last_answer":{"words":"x","confirmed":true},"date":"2026-10-25"}"#,
            r#"{"format":1,"state":"stopping_point","date":"2026-10-25","extra":1}"#,
        ] {
            std::fs::write(&file, bad).unwrap();
            assert!(load(&p).is_err(), "should refuse: {bad}");
            assert!(record_answer(&p, "new", true, "2026-10-25").is_err());
            assert_eq!(std::fs::read_to_string(&file).unwrap(), bad);
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_link_in_place_of_the_file_is_refused() {
        let dir = TempDir::new();
        let p = project(&dir);
        let folder = dir.path().join(".harness/writer");
        std::fs::create_dir_all(&folder).unwrap();
        let target = dir.path().join("elsewhere.json");
        std::fs::write(&target, "{}").unwrap();
        std::os::unix::fs::symlink(&target, folder.join(MARKER_FILE)).unwrap();
        assert!(load(&p).is_err());
        assert!(record_stopping_point(&p, "2026-10-25").is_err());
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "{}");
    }
}
