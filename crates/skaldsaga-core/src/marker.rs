//! How the writer's last session ended (HANDOFF, Ready to build, Slice 1,
//! item 2).
//!
//! The next session's recap depends on whether the writer was left in the
//! middle of a thread (with the last answer, confirmed or not) or at a
//! stopping point. The program records that after every answer, and the entry
//! routing reads it, so story text never decides how a session opens.
//!
//! Slice 2 adds two small fields: the topic last asked about (a short code,
//! never text or the writer's words) and how many confirmed answers have come
//! since the last save offer. Writing the marker never makes a snapshot of its
//! own: it is committed together with the next real save, or at a stop.
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

/// The topic the interviewer last asked about (*Decided October 25, 2026*,
/// planning session 2). A short code, never text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Topic {
    Premise,
    People,
    Setup,
    Complications,
    Crisis,
    Resolution,
}

impl Topic {
    pub const ALL: [Topic; 6] = [
        Topic::Premise,
        Topic::People,
        Topic::Setup,
        Topic::Complications,
        Topic::Crisis,
        Topic::Resolution,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Topic::Premise => "premise",
            Topic::People => "people",
            Topic::Setup => "setup",
            Topic::Complications => "complications",
            Topic::Crisis => "crisis",
            Topic::Resolution => "resolution",
        }
    }

    pub fn parse(s: &str) -> Option<Topic> {
        Topic::ALL.into_iter().find(|t| t.as_str() == s)
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
    /// The topic last asked about. Only for `mid_thread`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic: Option<Topic>,
    /// Confirmed answers since the last save offer (taken or not).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub answers_since_offer: u32,
    /// `YYYY-MM-DD`.
    pub date: String,
}

fn is_zero(n: &u32) -> bool {
    *n == 0
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
    parse(&text).map(Some)
}

/// Parses and checks the marker's text. Split out of [`load`] so the fuzz
/// targets can feed it arbitrary text without a folder (`fuzz/`).
pub fn parse(text: &str) -> Result<Marker, MarkerError> {
    let rel = marker_rel();
    let marker: Marker =
        serde_json::from_str(text).map_err(|e| MarkerError::Invalid(format!("{rel}: {e}")))?;
    if marker.format != FORMAT {
        return invalid(format!("{rel}: format {} isn't supported", marker.format));
    }
    match (marker.state, &marker.last_answer) {
        (State::MidThread, None) if marker.topic.is_none() => {
            return invalid(format!(
                "{rel}: a thread in progress needs a last answer or a topic"
            ));
        }
        (State::StoppingPoint, Some(_)) => {
            return invalid(format!("{rel}: a stopping point has no last answer"));
        }
        (State::StoppingPoint, None) if marker.topic.is_some() => {
            return invalid(format!("{rel}: a stopping point has no topic"));
        }
        _ => {}
    }
    if let Some(a) = &marker.last_answer
        && a.words.chars().count() > MAX_WORDS_CHARS
    {
        return invalid(format!("{rel}: the last answer is too long"));
    }
    Ok(marker)
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
    let same = existing.as_ref().is_some_and(|m| {
        m.state == marker.state
            && m.last_answer == marker.last_answer
            && m.topic == marker.topic
            && m.answers_since_offer == marker.answers_since_offer
    });
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
    // The topic and the count carry over: an answer doesn't change which
    // topic was asked about, and a newly confirmed answer adds to the count.
    let existing = load(project)?;
    let (topic, mut count) = match &existing {
        Some(m) if m.state == State::MidThread => (m.topic, m.answers_since_offer),
        _ => (None, 0),
    };
    let already_confirmed = existing
        .as_ref()
        .and_then(|m| m.last_answer.as_ref())
        .is_some_and(|a| a.confirmed && a.words == words);
    if confirmed && !already_confirmed {
        count = count.saturating_add(1);
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
            topic,
            answers_since_offer: count,
            date: date.to_string(),
        },
    )
}

/// Records the topic the interviewer is asking about now. Keeps the last
/// answer and the count. Allowed only when the story is mid-thread or has no
/// marker yet (content with no record reads as mid-thread).
pub fn record_ask(project: &Project, topic: Topic, date: &str) -> Result<Recorded, MarkerError> {
    let existing = load(project)?;
    let (last_answer, count) = match existing {
        None => (None, 0),
        Some(m) if m.state == State::MidThread => (m.last_answer, m.answers_since_offer),
        Some(_) => {
            return invalid("the story is at a stopping point, not in the middle of a thread");
        }
    };
    save(
        project,
        &Marker {
            format: FORMAT,
            state: State::MidThread,
            last_answer,
            topic: Some(topic),
            answers_since_offer: count,
            date: date.to_string(),
        },
    )
}

/// Records that a save offer was made (taken or not): the count starts over.
/// With no marker, or a count already at zero, nothing changes.
pub fn record_offer(project: &Project, date: &str) -> Result<Recorded, MarkerError> {
    let Some(mut m) = load(project)? else {
        return Ok(Recorded {
            changed: false,
            path: marker_rel(),
        });
    };
    m.answers_since_offer = 0;
    m.date = date.to_string();
    save(project, &m)
}

/// Whether a marker file is on disk (so its path can go in the next snapshot).
pub fn exists(project: &Project) -> bool {
    matches!(load(project), Ok(Some(_)))
}

/// Records that the session ended at a stopping point.
pub fn record_stopping_point(project: &Project, date: &str) -> Result<Recorded, MarkerError> {
    save(
        project,
        &Marker {
            format: FORMAT,
            state: State::StoppingPoint,
            last_answer: None,
            topic: None,
            answers_since_offer: 0,
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
    fn topics_round_trip_through_their_codes() {
        for t in Topic::ALL {
            assert_eq!(Topic::parse(t.as_str()), Some(t));
        }
        assert_eq!(Topic::parse("themes"), None);
        assert_eq!(Topic::parse(""), None);
    }

    #[test]
    fn asking_with_no_marker_starts_a_thread_with_a_topic_only() {
        let dir = TempDir::new();
        let p = project(&dir);
        let r = record_ask(&p, Topic::Setup, "2026-10-25").unwrap();
        assert!(r.changed);
        let m = load(&p).unwrap().unwrap();
        assert_eq!(m.state, State::MidThread);
        assert_eq!(m.topic, Some(Topic::Setup));
        assert_eq!(m.last_answer, None);
        assert!(!m.has_unconfirmed_answer());
        // Asking the same thing again changes nothing.
        assert!(!record_ask(&p, Topic::Setup, "2026-10-26").unwrap().changed);
    }

    #[test]
    fn an_answer_keeps_the_topic_and_a_new_question_keeps_the_answer() {
        let dir = TempDir::new();
        let p = project(&dir);
        record_answer(&p, "maybe a dark alley", false, "2026-10-25").unwrap();
        record_ask(&p, Topic::People, "2026-10-25").unwrap();
        let m = load(&p).unwrap().unwrap();
        assert_eq!(m.topic, Some(Topic::People));
        assert!(m.has_unconfirmed_answer());
        record_answer(&p, "Jack and Jill", true, "2026-10-25").unwrap();
        let m = load(&p).unwrap().unwrap();
        assert_eq!(m.topic, Some(Topic::People));
        assert_eq!(m.last_answer.unwrap().words, "Jack and Jill");
    }

    #[test]
    fn only_newly_confirmed_answers_are_counted_and_an_offer_resets_the_count() {
        let dir = TempDir::new();
        let p = project(&dir);
        record_answer(&p, "one", false, "2026-10-25").unwrap();
        assert_eq!(load(&p).unwrap().unwrap().answers_since_offer, 0);
        record_answer(&p, "one", true, "2026-10-25").unwrap();
        // Confirming the same words again is not a new answer.
        record_answer(&p, "one", true, "2026-10-25").unwrap();
        record_answer(&p, "two", true, "2026-10-25").unwrap();
        assert_eq!(load(&p).unwrap().unwrap().answers_since_offer, 2);
        let r = record_offer(&p, "2026-10-25").unwrap();
        assert!(r.changed);
        assert_eq!(load(&p).unwrap().unwrap().answers_since_offer, 0);
        assert!(!record_offer(&p, "2026-10-25").unwrap().changed);
    }

    #[test]
    fn an_offer_with_no_marker_changes_nothing() {
        let dir = TempDir::new();
        let p = project(&dir);
        assert!(!record_offer(&p, "2026-10-25").unwrap().changed);
        assert_eq!(load(&p).unwrap(), None);
    }

    #[test]
    fn a_stopping_point_clears_the_topic_and_refuses_a_question() {
        let dir = TempDir::new();
        let p = project(&dir);
        record_ask(&p, Topic::Crisis, "2026-10-25").unwrap();
        record_stopping_point(&p, "2026-10-25").unwrap();
        let m = load(&p).unwrap().unwrap();
        assert_eq!(m.topic, None);
        assert_eq!(m.answers_since_offer, 0);
        assert!(record_ask(&p, Topic::Crisis, "2026-10-25").is_err());
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
            r#"{"format":1,"state":"stopping_point","topic":"setup","date":"2026-10-25"}"#,
            r#"{"format":1,"state":"mid_thread","topic":"themes","date":"2026-10-25"}"#,
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
