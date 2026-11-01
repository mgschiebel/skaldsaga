//! Where a session stands, split between what is committed and what stays on
//! this computer (*Decided October 25, 2026*, planning session 3).
//!
//! The cloud session's own stop check flags any uncommitted change. So the
//! committed note of where things stand (`marker`) now changes only inside a
//! real save: a confirmed answer, a save the writer accepts, or a stop. What
//! changes in between lives in one small ignored file, `.harness/session-local.json`:
//!
//! * the writer's unconfirmed last answer,
//! * the topic last asked about (until the next real save folds it into the
//!   committed note),
//! * a save-offer count the writer declined (same folding),
//! * how many answers were confirmed in this session, which stands in for the
//!   meter where the meter has no reading (*Decided November 1, 2026*,
//!   planning session 4).
//!
//! **Cost, accepted by the author:** an unconfirmed answer is lost if the
//! writer carries on from another device or a fresh copy. Nothing here is read
//! as an instruction; the writer's words only ever appear as data. A local
//! file we can't use reads as empty, because it must never get in the writer's
//! way (the committed note is still strict).

use serde::{Deserialize, Serialize};

use crate::decisions::MAX_WORDS_CHARS;
use crate::fsio;
use crate::marker::{self, Answer, FORMAT, Marker, MarkerError, State, Topic};
use crate::project::Project;
use crate::settings;

/// The per-machine file's name inside `.harness/`.
pub const LOCAL_FILE: &str = "session-local.json";
/// Bumped when the file's shape changes.
pub const LOCAL_FORMAT: u32 = 1;
/// The file is tiny; anything bigger isn't ours.
const MAX_FILE_BYTES: u64 = 16 * 1024;
/// The committed ignore file, handed to git with every real save so the line
/// for [`LOCAL_FILE`] reaches folders set up by an earlier version.
const IGNORE_REL: &str = ".harness/.gitignore";

fn is_zero(n: &u32) -> bool {
    *n == 0
}

/// The per-machine part of the session note.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Local {
    #[serde(default)]
    pub format: u32,
    /// The writer's last answer, in their own words, not yet confirmed back.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unconfirmed: Option<String>,
    /// The topic last asked about, not yet folded into the committed note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic: Option<Topic>,
    /// Confirmed answers since the last save offer, after the writer declined
    /// one. Takes the place of the committed count until the next real save.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answers_since_offer: Option<u32>,
    /// Answers confirmed in this session.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub answers_this_session: u32,
}

impl Local {
    fn is_empty(&self) -> bool {
        self.unconfirmed.is_none()
            && self.topic.is_none()
            && self.answers_since_offer.is_none()
            && self.answers_this_session == 0
    }

    /// Parses the file's text; anything unusable reads as empty.
    pub fn parse(text: &str) -> Local {
        match serde_json::from_str::<Local>(text) {
            Ok(l) if l.format == LOCAL_FORMAT => l,
            _ => Local::default(),
        }
    }
}

fn local_path(project: &Project) -> std::path::PathBuf {
    project.harness_dir().join(LOCAL_FILE)
}

/// Loads this computer's part of the note. Never fails: a missing or unusable
/// file reads as empty.
pub fn load_local(project: &Project) -> Local {
    match fsio::read_regular_capped(&local_path(project), MAX_FILE_BYTES) {
        Ok(Some(Ok(text))) => Local::parse(&text),
        _ => Local::default(),
    }
}

fn save_local(project: &Project, local: &Local) -> Result<(), MarkerError> {
    let path = local_path(project);
    if local.is_empty() {
        project.remove_optional(&path)?;
        return Ok(());
    }
    // The ignore line goes in before the file, so the file is never untracked.
    settings::ensure_harness_ignored(
        project,
        LOCAL_FILE,
        "Where this computer's session stands, including an unconfirmed answer; never shared.",
    )?;
    let stored = Local {
        format: LOCAL_FORMAT,
        ..local.clone()
    };
    let mut text =
        serde_json::to_string_pretty(&stored).map_err(|e| MarkerError::Invalid(e.to_string()))?;
    text.push('\n');
    project.write_atomic(&path, &text)?;
    Ok(())
}

/// The committed note and this computer's part, read together.
#[derive(Debug, Clone)]
pub struct View {
    pub marker: Option<Marker>,
    pub local: Local,
}

/// Reads both parts. The committed note is strict (an unusable one is an
/// error and is never overwritten); the local part never fails.
pub fn view(project: &Project) -> Result<View, MarkerError> {
    Ok(View {
        marker: marker::load(project)?,
        local: load_local(project),
    })
}

impl View {
    fn mid(&self) -> Option<&Marker> {
        self.marker.as_ref().filter(|m| m.state == State::MidThread)
    }

    /// The topic last asked about: this computer's, else the committed one.
    pub fn topic(&self) -> Option<Topic> {
        self.local
            .topic
            .or_else(|| self.mid().and_then(|m| m.topic))
    }

    /// Confirmed answers since the last save offer.
    pub fn answers_since_offer(&self) -> u32 {
        self.local
            .answers_since_offer
            .unwrap_or_else(|| self.mid().map_or(0, |m| m.answers_since_offer))
    }

    /// The writer's last unconfirmed answer, if any. A note written by an
    /// earlier version may still hold one in the committed file.
    pub fn unconfirmed(&self) -> Option<&str> {
        self.local.unconfirmed.as_deref().or_else(|| {
            self.mid()
                .and_then(|m| m.last_answer.as_ref())
                .filter(|a| !a.confirmed)
                .map(|a| a.words.as_str())
        })
    }

    /// True when this computer knows of a thread in progress that the
    /// committed note doesn't show (an unconfirmed answer or a topic).
    pub fn has_thread_in_progress(&self) -> bool {
        self.local.unconfirmed.is_some() || self.local.topic.is_some()
    }
}

/// What a `session` command did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// False when nothing needed recording.
    pub changed: bool,
    /// Paths to stage with the next real save (empty for an unsaved change).
    pub add_paths: Vec<String>,
    /// Whether the writer's unconfirmed answer was pending (stop only).
    pub unconfirmed_pending: bool,
}

impl Outcome {
    fn unchanged() -> Outcome {
        Outcome {
            changed: false,
            add_paths: Vec::new(),
            unconfirmed_pending: false,
        }
    }

    /// Recorded on this computer only: nothing for git.
    fn kept_here() -> Outcome {
        Outcome {
            changed: true,
            add_paths: Vec::new(),
            unconfirmed_pending: false,
        }
    }

    /// Written to the committed note: its path, and the ignore file.
    fn saved(project: &Project, pending: bool) -> Outcome {
        let mut add_paths = vec![marker::marker_rel()];
        if project.harness_dir().join(".gitignore").is_file() {
            add_paths.push(IGNORE_REL.to_string());
        }
        Outcome {
            changed: true,
            add_paths,
            unconfirmed_pending: pending,
        }
    }
}

fn check_words(words: &str) -> Result<String, MarkerError> {
    let words = words.trim();
    if words.is_empty() {
        return Err(MarkerError::Invalid("the answer is empty".into()));
    }
    if words.chars().count() > MAX_WORDS_CHARS {
        return Err(MarkerError::Invalid(
            "the answer is too long to keep".into(),
        ));
    }
    Ok(words.to_string())
}

/// Records the writer's answer. Unconfirmed: kept on this computer only.
/// Confirmed: written to the committed note (inside the real save that
/// follows), which also takes over the topic and the count.
pub fn answer(
    project: &Project,
    words: &str,
    confirmed: bool,
    date: &str,
) -> Result<Outcome, MarkerError> {
    let words = check_words(words)?;
    let mut v = view(project)?;
    if !confirmed {
        if v.local.unconfirmed.as_deref() == Some(words.as_str()) {
            return Ok(Outcome::unchanged());
        }
        v.local.unconfirmed = Some(words);
        save_local(project, &v.local)?;
        return Ok(Outcome::kept_here());
    }
    let already = v
        .marker
        .as_ref()
        .and_then(|m| m.last_answer.as_ref())
        .is_some_and(|a| a.confirmed && a.words == words);
    let mut count = v.answers_since_offer();
    if !already {
        count = count.saturating_add(1);
        v.local.answers_this_session = v.local.answers_this_session.saturating_add(1);
    }
    let note = Marker {
        format: FORMAT,
        state: State::MidThread,
        last_answer: Some(Answer {
            words,
            confirmed: true,
        }),
        topic: v.topic(),
        answers_since_offer: count,
        date: date.to_string(),
    };
    let recorded = marker::write(project, &note)?;
    v.local.unconfirmed = None;
    v.local.topic = None;
    v.local.answers_since_offer = None;
    save_local(project, &v.local)?;
    if recorded.changed {
        Ok(Outcome::saved(project, false))
    } else {
        Ok(Outcome::unchanged())
    }
}

/// Records the topic being asked about. Kept on this computer until the next
/// real save. Allowed at a stopping point too: the writer has picked a new
/// area to look at.
pub fn ask(project: &Project, topic: Topic) -> Result<Outcome, MarkerError> {
    let mut v = view(project)?;
    if v.topic() == Some(topic) {
        return Ok(Outcome::unchanged());
    }
    v.local.topic = Some(topic);
    save_local(project, &v.local)?;
    Ok(Outcome::kept_here())
}

/// Records that a save offer was made. Declined: the count starts over on
/// this computer only. Taken: the committed note is written (the topic folded
/// in, the count at zero) for the snapshot the writer asked for.
pub fn offer(project: &Project, taken: bool, date: &str) -> Result<Outcome, MarkerError> {
    let mut v = view(project)?;
    if !taken {
        if v.answers_since_offer() == 0 {
            return Ok(Outcome::unchanged());
        }
        v.local.answers_since_offer = Some(0);
        save_local(project, &v.local)?;
        return Ok(Outcome::kept_here());
    }
    let topic = v.topic();
    let note = match (&v.marker, topic) {
        (None, None) => return Ok(Outcome::unchanged()),
        (Some(m), None) if m.state == State::StoppingPoint => m.clone(),
        (existing, topic) => Marker {
            format: FORMAT,
            state: State::MidThread,
            // Only a confirmed answer is ever carried into the committed note.
            last_answer: existing
                .as_ref()
                .filter(|m| m.state == State::MidThread)
                .and_then(|m| m.last_answer.clone())
                .filter(|a| a.confirmed),
            topic,
            answers_since_offer: 0,
            date: date.to_string(),
        },
    };
    marker::write(project, &note)?;
    v.local.topic = None;
    v.local.answers_since_offer = None;
    save_local(project, &v.local)?;
    // The writer asked for a safe point: hand the note back even if it didn't
    // change, since earlier writes may not be saved yet.
    Ok(Outcome::saved(project, false))
}

/// Why a thread ended (the skill judges; the program only checks the claim is
/// plausible).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// The writer's answers got short, unsure or empty of detail, twice in a row.
    WriterSignals,
    /// Nothing is left to ask on the topic without repeating.
    ThreadExhausted,
    /// The writer said they are done.
    WriterDone,
}

impl StopReason {
    pub fn parse(s: &str) -> Option<StopReason> {
        match s {
            "writer_signals" => Some(StopReason::WriterSignals),
            "thread_exhausted" => Some(StopReason::ThreadExhausted),
            "writer_done" => Some(StopReason::WriterDone),
            _ => None,
        }
    }

    /// Confirmed answers this session before the claim is believable: signals
    /// need two in a row, an exhausted thread at least one, and the writer may
    /// always stop.
    fn needs_answers(self) -> u32 {
        match self {
            StopReason::WriterSignals => 2,
            StopReason::ThreadExhausted => 1,
            StopReason::WriterDone => 0,
        }
    }
}

/// Records a stopping point. An unconfirmed answer is dropped with the rest of
/// this computer's thread state; `unconfirmed_pending` says there was one, so
/// the interviewer can tell the writer.
pub fn stop(project: &Project, reason: StopReason, date: &str) -> Result<Outcome, MarkerError> {
    let mut v = view(project)?;
    let have = v.local.answers_this_session;
    let need = reason.needs_answers();
    if have < need {
        return Err(MarkerError::Invalid(format!(
            "that stopping point needs at least {need} confirmed answers in this session; this one has {have}"
        )));
    }
    let pending = v.unconfirmed().is_some();
    let recorded = marker::record_stopping_point(project, date)?;
    v.local.unconfirmed = None;
    v.local.topic = None;
    v.local.answers_since_offer = None;
    save_local(project, &v.local)?;
    if recorded.changed {
        Ok(Outcome::saved(project, pending))
    } else {
        Ok(Outcome {
            unconfirmed_pending: pending,
            ..Outcome::unchanged()
        })
    }
}

/// Confirmed answers so far in this session (the meter's stand-in).
pub fn answers_this_session(project: &Project) -> u32 {
    load_local(project).answers_this_session
}

/// Starts the session count over, for a fresh session. Never fails the hook.
pub fn reset_session_count(project: &Project) {
    let mut local = load_local(project);
    if local.answers_this_session == 0 {
        return;
    }
    local.answers_this_session = 0;
    let _ = save_local(project, &local);
}

/// The settings the stand-in uses.
pub fn wrap_up_limits(project: &Project) -> (u32, u32) {
    let s = settings::SessionSettings::load(project).unwrap_or_default();
    (s.suggest_wrap_up_after_answers, s.wrap_up_after_answers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    fn project(dir: &TempDir) -> Project {
        std::fs::create_dir_all(dir.path().join(".harness")).unwrap();
        Project::new(dir.path())
    }

    fn marker_text(dir: &TempDir) -> Option<String> {
        std::fs::read_to_string(dir.path().join(".harness/writer/last-session.json")).ok()
    }

    #[test]
    fn an_unconfirmed_answer_stays_on_this_computer() {
        let dir = TempDir::new();
        let p = project(&dir);
        let out = answer(&p, "maybe a dark alley", false, "2026-11-01").unwrap();
        assert!(out.changed);
        assert!(out.add_paths.is_empty());
        assert_eq!(marker_text(&dir), None);
        let v = view(&p).unwrap();
        assert_eq!(v.unconfirmed(), Some("maybe a dark alley"));
        assert!(v.has_thread_in_progress());
        // The local file is ignored by git.
        let ignore = std::fs::read_to_string(dir.path().join(".harness/.gitignore")).unwrap();
        assert!(ignore.lines().any(|l| l == LOCAL_FILE));
    }

    #[test]
    fn saying_an_unconfirmed_answer_again_changes_nothing() {
        let dir = TempDir::new();
        let p = project(&dir);
        answer(&p, "maybe a dark alley", false, "2026-11-01").unwrap();
        assert!(
            !answer(&p, "maybe a dark alley", false, "2026-11-01")
                .unwrap()
                .changed
        );
    }

    #[test]
    fn a_confirmed_answer_is_committed_and_clears_the_local_answer() {
        let dir = TempDir::new();
        let p = project(&dir);
        ask(&p, Topic::Setup).unwrap();
        answer(&p, "She starts a run", false, "2026-11-01").unwrap();
        let out = answer(&p, "She starts a run", true, "2026-11-01").unwrap();
        assert!(out.changed);
        assert_eq!(
            out.add_paths.first().unwrap(),
            ".harness/writer/last-session.json"
        );
        let m = marker::load(&p).unwrap().unwrap();
        assert_eq!(m.topic, Some(Topic::Setup));
        assert_eq!(m.answers_since_offer, 1);
        assert!(!m.has_unconfirmed_answer());
        let v = view(&p).unwrap();
        assert_eq!(v.unconfirmed(), None);
        assert_eq!(v.local.topic, None);
        assert_eq!(v.local.answers_this_session, 1);
    }

    #[test]
    fn confirming_the_same_words_again_does_not_count_twice() {
        let dir = TempDir::new();
        let p = project(&dir);
        answer(&p, "She starts a run", true, "2026-11-01").unwrap();
        let out = answer(&p, "She starts a run", true, "2026-11-02").unwrap();
        assert!(!out.changed);
        assert_eq!(view(&p).unwrap().local.answers_this_session, 1);
        assert_eq!(view(&p).unwrap().answers_since_offer(), 1);
    }

    #[test]
    fn asking_a_question_writes_nothing_to_the_committed_note() {
        let dir = TempDir::new();
        let p = project(&dir);
        let out = ask(&p, Topic::People).unwrap();
        assert!(out.changed);
        assert!(out.add_paths.is_empty());
        assert_eq!(marker_text(&dir), None);
        assert_eq!(view(&p).unwrap().topic(), Some(Topic::People));
        assert!(!ask(&p, Topic::People).unwrap().changed);
    }

    #[test]
    fn declining_a_save_offer_resets_the_count_here_only() {
        let dir = TempDir::new();
        let p = project(&dir);
        answer(&p, "one", true, "2026-11-01").unwrap();
        answer(&p, "two", true, "2026-11-01").unwrap();
        let before = marker_text(&dir);
        let out = offer(&p, false, "2026-11-02").unwrap();
        assert!(out.add_paths.is_empty());
        assert_eq!(marker_text(&dir), before);
        assert_eq!(view(&p).unwrap().answers_since_offer(), 0);
        // The next confirmed answer starts the count from zero.
        answer(&p, "three", true, "2026-11-02").unwrap();
        assert_eq!(marker::load(&p).unwrap().unwrap().answers_since_offer, 1);
    }

    #[test]
    fn taking_a_save_offer_writes_the_note_with_the_topic_and_a_zero_count() {
        let dir = TempDir::new();
        let p = project(&dir);
        answer(&p, "one", true, "2026-11-01").unwrap();
        ask(&p, Topic::Crisis).unwrap();
        let out = offer(&p, true, "2026-11-01").unwrap();
        assert!(out.changed);
        assert!(
            out.add_paths
                .contains(&".harness/writer/last-session.json".to_string())
        );
        let m = marker::load(&p).unwrap().unwrap();
        assert_eq!(m.topic, Some(Topic::Crisis));
        assert_eq!(m.answers_since_offer, 0);
    }

    #[test]
    fn a_save_offer_with_nothing_to_save_changes_nothing() {
        let dir = TempDir::new();
        let p = project(&dir);
        assert!(!offer(&p, true, "2026-11-01").unwrap().changed);
        assert!(!offer(&p, false, "2026-11-01").unwrap().changed);
    }

    #[test]
    fn a_stop_needs_a_believable_reason() {
        let dir = TempDir::new();
        let p = project(&dir);
        // On the first answer, signals are not believable; the writer's own
        // "I'm done" always is.
        answer(&p, "one", true, "2026-11-01").unwrap();
        assert!(stop(&p, StopReason::WriterSignals, "2026-11-01").is_err());
        assert!(stop(&p, StopReason::ThreadExhausted, "2026-11-01").is_ok());
    }

    #[test]
    fn the_writer_can_always_stop() {
        let dir = TempDir::new();
        let p = project(&dir);
        let out = stop(&p, StopReason::WriterDone, "2026-11-01").unwrap();
        assert!(out.changed);
        assert_eq!(
            marker::load(&p).unwrap().unwrap().state,
            State::StoppingPoint
        );
    }

    #[test]
    fn two_answers_make_signals_believable_and_a_stop_reports_a_pending_answer() {
        let dir = TempDir::new();
        let p = project(&dir);
        answer(&p, "one", true, "2026-11-01").unwrap();
        answer(&p, "two", true, "2026-11-01").unwrap();
        answer(&p, "not sure maybe", false, "2026-11-01").unwrap();
        let out = stop(&p, StopReason::WriterSignals, "2026-11-01").unwrap();
        assert!(out.unconfirmed_pending);
        let v = view(&p).unwrap();
        assert_eq!(v.unconfirmed(), None);
        assert_eq!(v.marker.unwrap().state, State::StoppingPoint);
    }

    #[test]
    fn a_new_question_after_a_stopping_point_is_a_thread_in_progress() {
        let dir = TempDir::new();
        let p = project(&dir);
        stop(&p, StopReason::WriterDone, "2026-11-01").unwrap();
        assert!(!view(&p).unwrap().has_thread_in_progress());
        ask(&p, Topic::Resolution).unwrap();
        assert!(view(&p).unwrap().has_thread_in_progress());
    }

    #[test]
    fn the_session_count_resets_and_an_empty_local_file_is_removed() {
        let dir = TempDir::new();
        let p = project(&dir);
        answer(&p, "one", true, "2026-11-01").unwrap();
        assert_eq!(answers_this_session(&p), 1);
        reset_session_count(&p);
        assert_eq!(answers_this_session(&p), 0);
        assert!(!dir.path().join(".harness").join(LOCAL_FILE).exists());
    }

    #[test]
    fn an_unusable_local_file_reads_as_empty() {
        let dir = TempDir::new();
        let p = project(&dir);
        std::fs::write(dir.path().join(".harness").join(LOCAL_FILE), "{ not json").unwrap();
        assert_eq!(load_local(&p), Local::default());
        std::fs::write(
            dir.path().join(".harness").join(LOCAL_FILE),
            r#"{"format":9,"unconfirmed":"x"}"#,
        )
        .unwrap();
        assert_eq!(load_local(&p), Local::default());
    }

    #[test]
    fn stop_reasons_parse_from_their_codes() {
        assert_eq!(
            StopReason::parse("thread_exhausted"),
            Some(StopReason::ThreadExhausted)
        );
        assert_eq!(StopReason::parse("tired"), None);
    }
}
