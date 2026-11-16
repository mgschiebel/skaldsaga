//! The Interviewer's record of which queued questions it asked (HANDOFF,
//! Planning session 12; `design-agents.md`, "Two record types, and one writer
//! per file", Build 2).
//!
//! A queued question (`question` module) is written only by the agent that
//! raised it. What happened to it afterwards (asked, answered, set aside) is
//! the Interviewer's to say, so it lives in its own small file,
//! `spec/queue/interviewer/asked-xxxxxxxx.md`, which only [`mark`] writes. The
//! record points at the question by ID and never copies its words, so a
//! question is never stored twice (and its wording is always made afresh).
//!
//! There is one record per question. `asked` can become `answered` or
//! `set_aside`; those two are final.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::agents::INTERVIEWER;
use crate::fsio::FsError;
use crate::ids::{self, IdKind};
use crate::project::Project;
use crate::question;
use crate::records::{self, RecordError, invalid};

/// The most records of questions asked the story keeps (*Proposed*).
pub const MAX_ASKED: usize = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Kind {
    Asked,
}

/// How a question stands from the Interviewer's side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    /// Put to the writer; no answer yet.
    Asked,
    /// The writer answered it.
    Answered,
    /// The writer waved it off, or it no longer applies.
    SetAside,
}

impl State {
    pub fn as_str(self) -> &'static str {
        match self {
            State::Asked => "asked",
            State::Answered => "answered",
            State::SetAside => "set_aside",
        }
    }

    /// True for the two states that end a question.
    pub fn is_final(self) -> bool {
        self != State::Asked
    }
}

/// One record of a question asked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Asked {
    pub id: String,
    #[serde(rename = "type")]
    kind: Kind,
    /// The queued question this is about.
    pub question: String,
    pub state: State,
}

fn id_of(asked: &Asked) -> &str {
    &asked.id
}

impl Asked {
    /// Why this record can't be used, or `None` when it is well formed.
    pub fn problem(&self) -> Option<String> {
        if !ids::is_kind(&self.id, IdKind::Asked) {
            return Some(format!("'{}' isn't an asked-record ID", self.id));
        }
        if !ids::is_kind(&self.question, IdKind::Question) {
            return Some(format!("'{}' isn't a question ID", self.question));
        }
        None
    }
}

/// Parses an asked record and checks it is well formed. Never panics.
pub fn parse(text: &str) -> Result<Asked, String> {
    let asked: Asked = records::parse(text)?;
    match asked.problem() {
        Some(problem) => Err(problem),
        None => Ok(asked),
    }
}

/// The Interviewer's folder. This is the only place an asked record is ever
/// written (the one-writer rule).
fn folder(project: &Project) -> PathBuf {
    project
        .spec_dir()
        .join(records::QUEUE_FOLDER)
        .join(INTERVIEWER)
}

fn label() -> String {
    format!("spec/{}/{INTERVIEWER}", records::QUEUE_FOLDER)
}

fn relative_path(id: &str) -> String {
    format!("{}/{id}.md", label())
}

/// Loads every record of a question asked. A second record for the same
/// question is reported and ignored.
pub(crate) fn load(project: &Project) -> Result<records::Loaded<Asked>, FsError> {
    let mut loaded =
        records::load_folder::<Asked>(&folder(project), &label(), IdKind::Asked, id_of, MAX_ASKED)?;
    let mut seen = std::collections::HashSet::new();
    let mut keep = Vec::new();
    for asked in loaded.records {
        if let Some(problem) = asked.problem() {
            loaded
                .problems
                .push(format!("{}: {problem}", relative_path(&asked.id)));
        } else if !seen.insert(asked.question.clone()) {
            loaded.problems.push(format!(
                "{}: there is already a record for this question",
                relative_path(&asked.id)
            ));
        } else {
            keep.push(asked);
        }
    }
    loaded.records = keep;
    Ok(loaded)
}

/// What `mark` did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Marked {
    pub id: String,
    pub question: String,
    pub state: State,
    /// False when the question already had this state, so nothing was written.
    pub changed: bool,
    /// The file to stage with the next save, relative to the story folder.
    pub path: String,
}

/// Records what happened to a queued question: it was asked, the writer
/// answered it, or it was set aside. The first mark makes the record; a later
/// one changes its state, unless the state is already final.
pub fn mark(project: &Project, question_id: &str, state: State) -> Result<Marked, RecordError> {
    if !ids::is_kind(question_id, IdKind::Question) {
        return invalid(format!("'{question_id}' isn't a question ID"));
    }
    if question::find_any(project, question_id)?.is_none() {
        return invalid(format!("there's no queued question '{question_id}'"));
    }
    let loaded = load(project)?;
    let existing = loaded.records.iter().find(|a| a.question == question_id);
    let dir = folder(project);
    let record = match existing {
        Some(a) if a.state == state => {
            return Ok(Marked {
                id: a.id.clone(),
                question: a.question.clone(),
                state,
                changed: false,
                path: relative_path(&a.id),
            });
        }
        Some(a) if a.state.is_final() => {
            return invalid(format!(
                "that question is already {}; a final state doesn't change",
                a.state.as_str().replace('_', " ")
            ));
        }
        Some(a) => Asked { state, ..a.clone() },
        None => {
            if loaded.records.len() >= MAX_ASKED {
                return invalid(format!(
                    "this story already keeps {MAX_ASKED} records of questions asked, which is the most"
                ));
            }
            let id = records::fresh_id(IdKind::Asked, |id| {
                loaded.records.iter().any(|a| a.id == id)
                    || std::fs::symlink_metadata(dir.join(format!("{id}.md"))).is_ok()
            })?;
            Asked {
                id,
                kind: Kind::Asked,
                question: question_id.to_string(),
                state,
            }
        }
    };
    let text = records::render(&record)?;
    project.ensure_dir(&dir)?;
    project.write_atomic(&dir.join(format!("{}.md", record.id)), &text)?;
    Ok(Marked {
        path: relative_path(&record.id),
        id: record.id,
        question: record.question,
        state,
        changed: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::records::testkit::{MARA, question_draft, story, verdict_draft};
    use crate::{question, verdict};

    const NOW: &str = "2026-11-16T17:44";

    fn refusal(r: Result<Marked, RecordError>) -> String {
        match r {
            Err(RecordError::Invalid(m)) => m,
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    /// A story with one queued question; returns its ID.
    fn queued() -> (crate::testutil::TempDir, Project, String) {
        let (dir, project) = story();
        let v = verdict::add(
            &project,
            "continuity",
            &verdict_draft("contradiction_reported", &[(MARA, 2)], "Clash."),
            NOW,
        )
        .unwrap();
        let q = question::add(
            &project,
            "continuity",
            &question_draft(&[MARA], "Worth asking.", &v.id),
            NOW,
        )
        .unwrap();
        (dir, project, q.id)
    }

    fn files(project: &Project, folder: &str) -> Vec<String> {
        let dir = project.spec_dir().join(folder);
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .map(|rd| {
                rd.map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        names
    }

    #[test]
    fn the_first_mark_makes_the_record_and_a_later_one_changes_its_state() {
        let (_dir, project, question) = queued();
        let first = mark(&project, &question, State::Asked).unwrap();
        assert!(first.changed);
        assert!(ids::is_kind(&first.id, IdKind::Asked));
        assert_eq!(
            first.path,
            format!("spec/queue/interviewer/{}.md", first.id)
        );
        let text = std::fs::read_to_string(project.root().join(&first.path)).unwrap();
        assert!(text.contains(&format!("question: {question}")) && text.contains("state: asked"));

        let second = mark(&project, &question, State::Answered).unwrap();
        assert!(second.changed);
        assert_eq!(second.id, first.id, "one record per question");
        assert_eq!(files(&project, "queue/interviewer").len(), 1);
        let text = std::fs::read_to_string(project.root().join(&second.path)).unwrap();
        assert!(text.contains("state: answered"));
    }

    #[test]
    fn marking_the_same_state_again_changes_nothing() {
        let (_dir, project, question) = queued();
        let first = mark(&project, &question, State::Asked).unwrap();
        let path = project.root().join(&first.path);
        let before = std::fs::read(&path).unwrap();
        let again = mark(&project, &question, State::Asked).unwrap();
        assert!(!again.changed);
        assert_eq!(again.id, first.id);
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }

    #[test]
    fn a_final_state_does_not_change() {
        let (_dir, project, question) = queued();
        mark(&project, &question, State::SetAside).unwrap();
        let m = refusal(mark(&project, &question, State::Asked));
        assert!(m.contains("already set aside"), "{m}");
        let m = refusal(mark(&project, &question, State::Answered));
        assert!(m.contains("a final state doesn't change"), "{m}");
        // A question can go straight to answered without being marked asked first.
        let (_d2, project2, question2) = queued();
        assert!(
            mark(&project2, &question2, State::Answered)
                .unwrap()
                .changed
        );
    }

    #[test]
    fn only_a_queued_question_can_be_marked_and_the_id_is_checked() {
        let (_dir, project, _question) = queued();
        for bad in ["../etc/passwd", "question-0007", "", "verdict-abcdefgh"] {
            assert!(
                refusal(mark(&project, bad, State::Asked)).contains("isn't a question ID"),
                "{bad}"
            );
        }
        assert!(
            refusal(mark(&project, "question-zzzzzzzz", State::Asked))
                .contains("no queued question")
        );
        assert!(files(&project, "queue/interviewer").is_empty());
    }

    #[test]
    fn marking_writes_only_the_interviewers_folder() {
        let (_dir, project, question) = queued();
        let before_continuity = files(&project, "queue/continuity");
        let before_verdicts = files(&project, "verdicts/continuity");
        mark(&project, &question, State::Asked).unwrap();
        assert_eq!(files(&project, "queue/continuity"), before_continuity);
        assert_eq!(files(&project, "verdicts/continuity"), before_verdicts);
        assert_eq!(files(&project, "queue/interviewer").len(), 1);
    }

    #[test]
    fn a_second_record_for_one_question_is_reported_and_ignored() {
        let (_dir, project, question) = queued();
        let first = mark(&project, &question, State::Asked).unwrap();
        let text = std::fs::read_to_string(project.root().join(&first.path)).unwrap();
        let folder = project.spec_dir().join("queue/interviewer");
        std::fs::write(
            folder.join("asked-zzzzzzzz.md"),
            text.replace(&first.id, "asked-zzzzzzzz"),
        )
        .unwrap();
        std::fs::write(folder.join("asked-aaaaaaaa.md"), "---\nstate: asked\n---\n").unwrap();
        let loaded = load(&project).unwrap();
        assert_eq!(loaded.records.len(), 1);
        assert_eq!(loaded.problems.len(), 2, "{:#?}", loaded.problems);
        assert!(parse(&text).is_ok());
        assert!(
            parse("---\nid: asked-aaaaaaaa\ntype: asked\nquestion: nope\nstate: asked\n---\n")
                .is_err()
        );
    }
}
