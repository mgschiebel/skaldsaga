//! The session meter.
//!
//! Claude Code runs the status line command after each turn and passes JSON
//! about the session on stdin, including how full the context window is. The
//! meter records that in `.harness/meter.json`; the interviewer checks it
//! after each saved answer and wraps the session up when it gets too full
//! (the "smart window" ends well before 100%).
//!
//! Everything here is forgiving on purpose: missing or odd data turns into an
//! `Unknown` state, never an error, and the manual session boundary still works.

use serde::{Deserialize, Serialize};

use crate::fsio::{self, FsError};
use crate::json::{self, Value, ValueExt as _};
use crate::project::Project;
use crate::settings::MeterSettings;

/// What the meter learned from one status line run, as stored in
/// `.harness/meter.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reading {
    #[serde(default)]
    pub session_id: Option<String>,
    /// Share of the context window in use (input tokens only). Claude Code
    /// reports `null` early in a session.
    #[serde(default)]
    pub used_percentage: Option<f64>,
    #[serde(default)]
    pub context_window_size: Option<f64>,
    /// Unix seconds when the reading was taken.
    #[serde(default)]
    pub updated_at: u64,
}

/// The parts of Claude Code's status line input the meter uses.
///
/// Parsed field by field rather than with a derived deserializer: this input
/// comes from outside, and one odd field must not throw away the rest.
#[derive(Debug, Clone, PartialEq)]
pub struct StatusInput {
    pub session_id: Option<String>,
    /// The folder Claude Code was started in, if reported.
    pub project_dir: Option<String>,
    pub used_percentage: Option<f64>,
    pub context_window_size: Option<f64>,
}

impl StatusInput {
    /// Parses the status line JSON. Returns `None` if it isn't a JSON object.
    pub fn parse(input: &str) -> Option<StatusInput> {
        let v = json::parse(input).ok().filter(Value::is_object)?;
        let text = |path: &[&str]| v.get_path(path).and_then(Value::as_str).map(str::to_string);
        let number = |path: &[&str]| {
            v.get_path(path)
                .and_then(Value::as_f64)
                .filter(|n| n.is_finite())
        };
        Some(StatusInput {
            session_id: text(&["session_id"]),
            project_dir: text(&["workspace", "project_dir"])
                .or_else(|| text(&["workspace", "current_dir"]))
                .or_else(|| text(&["cwd"])),
            used_percentage: number(&["context_window", "used_percentage"])
                .map(|n| n.clamp(0.0, 100.0)),
            context_window_size: number(&["context_window", "context_window_size"]),
        })
    }

    pub fn to_reading(&self, now: u64) -> Reading {
        Reading {
            session_id: self.session_id.clone(),
            used_percentage: self.used_percentage,
            context_window_size: self.context_window_size,
            updated_at: now,
        }
    }
}

/// Saves a reading to `.harness/meter.json` (atomically).
pub fn write_reading(project: &Project, reading: &Reading) -> Result<(), FsError> {
    let value = serde_json::to_value(reading).unwrap_or_else(|_| json::object());
    project.write_json(&project.meter_path(), &value)
}

/// Loads the last reading. A missing or unreadable file reads as `None`:
/// the meter must never get in the writer's way.
pub fn read_reading(project: &Project) -> Option<Reading> {
    fsio::read_json_object(&project.meter_path())
        .ok()
        .flatten()
        .and_then(|v| serde_json::from_value(v).ok())
}

/// Deletes the reading, for example when a new session starts, so a full
/// meter from the last session can't end the new one early.
pub fn reset(project: &Project) -> Result<bool, FsError> {
    project.remove_optional(&project.meter_path())
}

/// What the interviewer should do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MeterState {
    /// Plenty of room; carry on.
    Ok,
    /// First threshold: recommend wrapping up at the next natural pause (the writer may decline once).
    SuggestWrapUp,
    /// Second threshold: wrap up regardless.
    WrapUp,
    /// No usable reading; carry on and rely on the manual session boundary.
    Unknown,
}

impl MeterState {
    pub fn as_str(self) -> &'static str {
        match self {
            MeterState::Ok => "ok",
            MeterState::SuggestWrapUp => "suggest_wrap_up",
            MeterState::WrapUp => "wrap_up",
            MeterState::Unknown => "unknown",
        }
    }
}

/// The result of checking the meter; also the JSON `meter check` prints.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Evaluation {
    pub state: MeterState,
    pub used_percentage: Option<f64>,
    pub suggest_wrap_up_percent: f64,
    pub wrap_up_percent: f64,
    pub session_id: Option<String>,
    /// Why the state is `Unknown`, in plain words; `None` otherwise.
    pub note: Option<&'static str>,
    /// What decided the state: the context window, or the count of confirmed
    /// answers where the window has no reading.
    pub source: &'static str,
    /// Confirmed answers in this session, when the count decided the state.
    pub answers_this_session: Option<u32>,
}

/// The state came from the context window.
pub const SOURCE_CONTEXT: &str = "context";
/// The state came from the count of confirmed answers.
pub const SOURCE_ANSWERS: &str = "answers";

/// Compares a reading with the thresholds. If `current_session` is given,
/// a reading from a different session is ignored.
pub fn evaluate(
    reading: Option<&Reading>,
    settings: &MeterSettings,
    current_session: Option<&str>,
) -> Evaluation {
    let mut eval = Evaluation {
        state: MeterState::Unknown,
        used_percentage: None,
        suggest_wrap_up_percent: settings.suggest_wrap_up_percent,
        wrap_up_percent: settings.wrap_up_percent,
        session_id: reading.and_then(|r| r.session_id.clone()),
        note: None,
        source: SOURCE_CONTEXT,
        answers_this_session: None,
    };
    let Some(reading) = reading else {
        eval.note = Some("no meter reading yet");
        return eval;
    };
    if let (Some(want), Some(have)) = (current_session, reading.session_id.as_deref())
        && want != have
    {
        eval.note = Some("the reading is from a different session");
        return eval;
    }
    let Some(used) = reading.used_percentage else {
        eval.note = Some("context usage not reported yet");
        return eval;
    };
    eval.used_percentage = Some(used);
    eval.state = if used >= settings.wrap_up_percent {
        MeterState::WrapUp
    } else if used >= settings.suggest_wrap_up_percent {
        MeterState::SuggestWrapUp
    } else {
        MeterState::Ok
    };
    eval
}

/// Lets the count of confirmed answers in the session stand in for the context
/// window where the window has no reading (*Decided November 1, 2026*,
/// planning session 4). A reading always wins: this changes nothing unless the
/// state is `Unknown`. Below the first count the state stays `Unknown`.
pub fn with_answer_count(
    mut eval: Evaluation,
    answers: u32,
    suggest_after: u32,
    wrap_up_after: u32,
) -> Evaluation {
    if eval.state != MeterState::Unknown {
        return eval;
    }
    let state = if answers >= wrap_up_after {
        MeterState::WrapUp
    } else if answers >= suggest_after {
        MeterState::SuggestWrapUp
    } else {
        return eval;
    };
    eval.state = state;
    eval.source = SOURCE_ANSWERS;
    eval.answers_this_session = Some(answers);
    eval.note = None;
    eval
}

impl Evaluation {
    /// Machine-readable result for the skills.
    pub fn to_json(&self) -> Value {
        serde_json::to_value(self).unwrap_or_else(|_| json::object())
    }

    /// The meter's own status line text, shown when the writer has no status
    /// line of their own. Plain words, no internal terms.
    pub fn status_text(&self) -> String {
        match (self.state, self.used_percentage) {
            (MeterState::WrapUp, Some(p)) => {
                format!("Skaldsaga · session {p:.0}% full · time to wrap up")
            }
            (MeterState::SuggestWrapUp, Some(p)) => {
                format!("Skaldsaga · session {p:.0}% full · a break soon")
            }
            (_, Some(p)) => format!("Skaldsaga · session {p:.0}% full"),
            _ => "Skaldsaga".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    fn reading(session: Option<&str>, used: Option<f64>) -> Reading {
        Reading {
            session_id: session.map(str::to_string),
            used_percentage: used,
            context_window_size: Some(200_000.0),
            updated_at: 1,
        }
    }

    #[test]
    fn parses_status_input() {
        let input = StatusInput::parse(
            r#"{"session_id":"s1","cwd":"/c","workspace":{"current_dir":"/c","project_dir":"/p"},
               "context_window":{"used_percentage":42.4,"context_window_size":200000}}"#,
        )
        .unwrap();
        assert_eq!(input.session_id.as_deref(), Some("s1"));
        assert_eq!(input.project_dir.as_deref(), Some("/p"));
        assert_eq!(input.used_percentage, Some(42.4));
        assert_eq!(input.context_window_size, Some(200_000.0));
    }

    #[test]
    fn status_input_tolerates_nulls_and_fallbacks() {
        let input = StatusInput::parse(r#"{"cwd":"/c","context_window":{"used_percentage":null}}"#)
            .unwrap();
        assert_eq!(input.project_dir.as_deref(), Some("/c"));
        assert_eq!(input.used_percentage, None);
        assert!(StatusInput::parse("not json").is_none());
        assert!(StatusInput::parse("[]").is_none());
        let clamped = StatusInput::parse(r#"{"context_window":{"used_percentage":130}}"#).unwrap();
        assert_eq!(clamped.used_percentage, Some(100.0));
    }

    #[test]
    fn the_answer_count_stands_in_only_where_there_is_no_reading() {
        let s = MeterSettings::default();
        let none = evaluate(None, &s, None);
        let at = |n| with_answer_count(none.clone(), n, 25, 35);
        assert_eq!(at(24).state, MeterState::Unknown);
        assert_eq!(at(24).source, SOURCE_CONTEXT);
        assert_eq!(at(25).state, MeterState::SuggestWrapUp);
        assert_eq!(at(34).state, MeterState::SuggestWrapUp);
        assert_eq!(at(35).state, MeterState::WrapUp);
        assert_eq!(at(35).source, SOURCE_ANSWERS);
        assert_eq!(at(35).answers_this_session, Some(35));
        // A real reading is never overruled by the count.
        let read = evaluate(Some(&reading(None, Some(10.0))), &s, None);
        let kept = with_answer_count(read, 99, 25, 35);
        assert_eq!(kept.state, MeterState::Ok);
        assert_eq!(kept.source, SOURCE_CONTEXT);
    }

    #[test]
    fn thresholds_pick_the_state() {
        let s = MeterSettings::default();
        let state = |used| evaluate(Some(&reading(None, Some(used))), &s, None).state;
        assert_eq!(state(10.0), MeterState::Ok);
        assert_eq!(state(49.9), MeterState::Ok);
        assert_eq!(state(50.0), MeterState::SuggestWrapUp);
        assert_eq!(state(69.9), MeterState::SuggestWrapUp);
        assert_eq!(state(70.0), MeterState::WrapUp);
        assert_eq!(state(100.0), MeterState::WrapUp);
    }

    #[test]
    fn unknown_cases_carry_on() {
        let s = MeterSettings::default();
        assert_eq!(evaluate(None, &s, None).state, MeterState::Unknown);
        assert_eq!(
            evaluate(Some(&reading(Some("a"), None)), &s, None).state,
            MeterState::Unknown
        );
        let other = evaluate(Some(&reading(Some("old"), Some(90.0))), &s, Some("new"));
        assert_eq!(other.state, MeterState::Unknown);
        assert_eq!(other.used_percentage, None);
        let same = evaluate(Some(&reading(Some("new"), Some(90.0))), &s, Some("new"));
        assert_eq!(same.state, MeterState::WrapUp);
        // A reading without a session id is trusted.
        assert_eq!(
            evaluate(Some(&reading(None, Some(90.0))), &s, Some("new")).state,
            MeterState::WrapUp
        );
    }

    #[test]
    fn reading_round_trips_through_the_file() {
        let dir = TempDir::new();
        let project = Project::new(dir.path());
        assert!(read_reading(&project).is_none());
        let r = reading(Some("s1"), Some(33.0));
        write_reading(&project, &r).unwrap();
        assert_eq!(read_reading(&project), Some(r));
        assert!(reset(&project).unwrap());
        assert!(read_reading(&project).is_none());
        assert!(!reset(&project).unwrap());
    }

    #[test]
    fn corrupt_meter_file_reads_as_none() {
        let dir = TempDir::new();
        let project = Project::new(dir.path());
        std::fs::create_dir_all(project.harness_dir()).unwrap();
        std::fs::write(project.meter_path(), "{oops").unwrap();
        assert!(read_reading(&project).is_none());
    }

    #[test]
    fn status_text_is_plain() {
        let s = MeterSettings::default();
        let text = |used| evaluate(Some(&reading(None, used)), &s, None).status_text();
        assert_eq!(text(Some(12.3)), "Skaldsaga · session 12% full");
        assert_eq!(
            text(Some(55.0)),
            "Skaldsaga · session 55% full · a break soon"
        );
        assert_eq!(
            text(Some(71.0)),
            "Skaldsaga · session 71% full · time to wrap up"
        );
        assert_eq!(text(None), "Skaldsaga");
    }

    #[test]
    fn evaluation_json_shape() {
        let e = evaluate(
            Some(&reading(Some("s"), Some(55.0))),
            &MeterSettings::default(),
            None,
        );
        assert_eq!(
            e.to_json().to_compact_string(),
            r#"{"state":"suggest_wrap_up","used_percentage":55.0,"suggest_wrap_up_percent":50.0,"wrap_up_percent":70.0,"session_id":"s","note":null,"source":"context","answers_this_session":null}"#
        );
    }
}
