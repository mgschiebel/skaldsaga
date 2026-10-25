//! `skaldsaga recap`: what the session-start recap for a story left
//! mid-thread is built from (*Decided October 25, 2026*, planning session 2;
//! design in `design-story.md`, "Resuming with no input").
//!
//! The program picks the topic of the one question; the skill words the
//! summary and the question from what this prints. Read-only: it never writes
//! anything, and it never prints a question or a prompt, only facts and a
//! topic code. Story text it does print (the premise, names, the loose idea)
//! is data for the skill to quote, never instructions (SR-30 to SR-33).
//!
//! **Ranking rule.** If a topic was last asked about and the story still has
//! nothing for it, ask about it again (the skill finds a fresh angle).
//! Otherwise ask about the earliest thing missing: the premise first, then the
//! earliest stage of the main plot with no summary, starting with how the
//! story begins. Once all of those are filled there is nothing to ask in this
//! slice and the topic is `null`.

use serde_json::{Value, json};

use crate::marker::{self, MarkerError, State, Topic};
use crate::model::{ArcStatus, NodeBody, Stage, Thread, ThreadKind};
use crate::project::Project;
use crate::settings::SessionSettings;
use crate::spec::{self, Spec};

/// Most names listed; a longer cast is cut (the skill only needs a few).
const MAX_PEOPLE: usize = 12;

/// Why a recap couldn't be built.
#[derive(Debug)]
pub enum RecapError {
    Spec(String),
    Marker(MarkerError),
}

impl std::fmt::Display for RecapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecapError::Spec(m) => f.write_str(m),
            RecapError::Marker(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for RecapError {}

/// What is known about the story, and the chosen topic.
#[derive(Debug, Clone, PartialEq)]
pub struct Recap {
    /// The main story's premise, in the writer's words.
    pub premise: Option<String>,
    /// Names of the people the story has, alphabetical.
    pub people: Vec<String>,
    /// Stages of the main plot with no summary yet, in order.
    pub stages_without_summary: Vec<Topic>,
    /// An unconfirmed last answer, in the writer's own words: a tentative idea.
    pub loose_idea: Option<String>,
    /// The topic last asked about, if any.
    pub last_topic: Option<Topic>,
    /// The topic to ask about now; `None` when nothing is left to ask.
    pub topic: Option<Topic>,
    pub answers_since_offer: u32,
    /// The count at which a save offer is due (a setting).
    pub save_offer_after: u32,
}

impl Recap {
    /// True when enough confirmed answers have come since the last offer.
    pub fn save_offer_due(&self) -> bool {
        self.answers_since_offer >= self.save_offer_after
    }

    pub fn to_value(&self) -> Value {
        json!({
            "outcome": "reported",
            "premise": self.premise,
            "people": self.people,
            "stages_without_summary": self.stages_without_summary.iter().map(|t| t.as_str()).collect::<Vec<_>>(),
            "loose_idea": self.loose_idea,
            "last_topic": self.last_topic.map(Topic::as_str),
            "topic": self.topic.map(Topic::as_str),
            "answers_since_offer": self.answers_since_offer,
            "save_offer_after": self.save_offer_after,
            "save_offer": self.save_offer_due(),
        })
    }
}

fn stage_topic(stage: Stage) -> Topic {
    match stage {
        Stage::Setup => Topic::Setup,
        Stage::Complications => Topic::Complications,
        Stage::Crisis => Topic::Crisis,
        Stage::Resolution => Topic::Resolution,
    }
}

/// The main story's main plot: its first thread that is the big picture,
/// else its first thread of any kind.
fn main_thread(spec: &Spec) -> Option<&Thread> {
    let arc = spec.nodes.iter().find_map(|n| match &n.body {
        NodeBody::Arc(a) if a.status == ArcStatus::Main => Some(a),
        _ => None,
    })?;
    let threads: Vec<&Thread> = arc
        .threads
        .iter()
        .filter_map(|id| match spec.node(id).map(|n| &n.body) {
            Some(NodeBody::Thread(t)) => Some(t),
            _ => None,
        })
        .collect();
    threads
        .iter()
        .find(|t| t.kind == ThreadKind::BigPicture)
        .or(threads.first())
        .copied()
}

fn has_text(s: &Option<String>) -> bool {
    s.as_deref().is_some_and(|t| !t.trim().is_empty())
}

/// Builds the recap from a loaded story and its marker. Pure, so the ranking
/// rule is covered by ordinary tests.
pub fn build(spec: &Spec, marker: Option<&marker::Marker>, save_offer_after: u32) -> Recap {
    let premise = spec
        .nodes
        .iter()
        .find_map(|n| match &n.body {
            NodeBody::Arc(a) if a.status == ArcStatus::Main => a.premise.as_ref(),
            _ => None,
        })
        .map(|p| p.current.trim().to_string())
        .filter(|p| !p.is_empty());
    // IDs are random, so ID order would read as arbitrary: sort by name.
    let mut people: Vec<String> = spec
        .nodes
        .iter()
        .filter(|n| matches!(n.body, NodeBody::Person))
        .map(|n| n.name.clone())
        .collect();
    people.sort_by_key(|n| n.to_lowercase());
    let stages_without_summary: Vec<Topic> = match main_thread(spec) {
        Some(t) => Stage::ALL
            .into_iter()
            .filter(|s| {
                !t.stages
                    .iter()
                    .any(|e| e.stage == *s && has_text(&e.summary))
            })
            .map(stage_topic)
            .collect(),
        None => Stage::ALL.into_iter().map(stage_topic).collect(),
    };

    let mid = marker.filter(|m| m.state == State::MidThread);
    let last_topic = mid.and_then(|m| m.topic);
    let loose_idea = mid
        .and_then(|m| m.last_answer.as_ref())
        .filter(|a| !a.confirmed)
        .map(|a| a.words.clone());

    // Whether the story already has something for a topic.
    let answered = |t: Topic| match t {
        Topic::Premise => premise.is_some(),
        Topic::People => !people.is_empty(),
        _ => !stages_without_summary.contains(&t),
    };
    let topic = match last_topic.filter(|t| !answered(*t)) {
        Some(t) => Some(t),
        None if premise.is_none() => Some(Topic::Premise),
        None => stages_without_summary.first().copied(),
    };

    Recap {
        premise,
        people: people.into_iter().take(MAX_PEOPLE).collect(),
        stages_without_summary,
        loose_idea,
        last_topic,
        topic,
        answers_since_offer: mid.map_or(0, |m| m.answers_since_offer),
        save_offer_after,
    }
}

/// Reads the story and the marker and builds the recap. Never writes.
pub fn run(project: &Project) -> Result<Recap, RecapError> {
    let loaded = spec::load(&project.spec_dir()).map_err(|e| RecapError::Spec(e.to_string()))?;
    let marker = marker::load(project).map_err(RecapError::Marker)?;
    let settings = SessionSettings::load(project).map_err(|e| RecapError::Spec(e.to_string()))?;
    Ok(build(
        &loaded.spec,
        marker.as_ref(),
        settings.save_offer_after_answers,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::marker::{Answer, FORMAT, Marker};
    use crate::model::{Premise, StageEntry};
    use crate::testspec::{arc, node, spec_of, thread};

    fn mid(topic: Option<Topic>, answer: Option<(&str, bool)>, count: u32) -> Marker {
        Marker {
            format: FORMAT,
            state: State::MidThread,
            last_answer: answer.map(|(w, c)| Answer {
                words: w.into(),
                confirmed: c,
            }),
            topic,
            answers_since_offer: count,
            date: "2026-10-25".into(),
        }
    }

    /// A story with a main arc, a main plot with the given stage summaries
    /// (setup, complications, crisis, resolution) and the given people.
    fn story(premise: Option<&str>, stages: [Option<&str>; 4], people: &[&str]) -> Spec {
        let mut a = node(
            "a-main",
            NodeBody::Arc(arc(ArcStatus::Main, &["t-main"], &[])),
        );
        if let NodeBody::Arc(body) = &mut a.body {
            body.premise = premise.map(|p| Premise {
                current: p.into(),
                history: vec![],
            });
        }
        let mut t = thread("t-main", "a-main", ThreadKind::BigPicture);
        if let NodeBody::Thread(body) = &mut t.body {
            body.stages = Stage::ALL
                .into_iter()
                .zip(stages)
                .map(|(stage, s)| StageEntry {
                    stage,
                    summary: s.map(String::from),
                    confidence: None,
                })
                .collect();
        }
        let mut nodes = vec![a, t];
        for (i, name) in people.iter().enumerate() {
            let mut p = node(&format!("p-{i}"), NodeBody::Person);
            p.name = (*name).into();
            nodes.push(p);
        }
        spec_of(nodes, &[])
    }

    #[test]
    fn jack_and_jill_ask_how_it_begins() {
        let spec = story(
            Some("A modern Jack and Jill, set in a city at night"),
            [None; 4],
            &["Jack", "Jill"],
        );
        let m = mid(Some(Topic::People), Some(("maybe a dark alley", false)), 2);
        let r = build(&spec, Some(&m), 10);
        assert_eq!(r.people, ["Jack", "Jill"]);
        assert_eq!(r.loose_idea.as_deref(), Some("maybe a dark alley"));
        assert_eq!(r.last_topic, Some(Topic::People));
        assert_eq!(r.topic, Some(Topic::Setup));
        assert_eq!(
            r.stages_without_summary,
            [
                Topic::Setup,
                Topic::Complications,
                Topic::Crisis,
                Topic::Resolution
            ]
        );
    }

    #[test]
    fn an_unanswered_topic_is_asked_again_before_moving_on() {
        let spec = story(
            Some("A story"),
            [Some("it begins"), None, None, None],
            &["Jack"],
        );
        let m = mid(Some(Topic::Complications), None, 0);
        assert_eq!(build(&spec, Some(&m), 10).topic, Some(Topic::Complications));
        // Once that stage has a summary the rule moves to the earliest gap.
        let spec = story(
            Some("A story"),
            [Some("it begins"), Some("it gets worse"), None, None],
            &["Jack"],
        );
        let r = build(&spec, Some(&m), 10);
        assert_eq!(r.topic, Some(Topic::Crisis));
    }

    #[test]
    fn no_premise_asks_for_the_premise_first() {
        let spec = story(None, [None; 4], &[]);
        assert_eq!(build(&spec, None, 10).topic, Some(Topic::Premise));
        // A later topic asked about and still unanswered wins: it was asked
        // and nothing came of it.
        let m = mid(Some(Topic::Resolution), None, 0);
        assert_eq!(build(&spec, Some(&m), 10).topic, Some(Topic::Resolution));
    }

    #[test]
    fn people_asked_and_unanswered_is_asked_again() {
        let spec = story(Some("A story"), [None; 4], &[]);
        let m = mid(Some(Topic::People), None, 0);
        assert_eq!(build(&spec, Some(&m), 10).topic, Some(Topic::People));
    }

    #[test]
    fn nothing_left_means_no_topic() {
        let spec = story(
            Some("A story"),
            [Some("a"), Some("b"), Some("c"), Some("d")],
            &["Jack"],
        );
        let r = build(&spec, None, 10);
        assert_eq!(r.topic, None);
        assert!(r.stages_without_summary.is_empty());
    }

    #[test]
    fn a_story_with_no_main_plot_has_every_stage_open() {
        let spec = spec_of(vec![], &[]);
        let r = build(&spec, None, 10);
        assert_eq!(r.stages_without_summary.len(), 4);
        assert_eq!(r.topic, Some(Topic::Premise));
        assert_eq!(r.premise, None);
    }

    #[test]
    fn blank_summaries_count_as_missing() {
        let spec = story(Some("A story"), [Some("  "), None, None, None], &[]);
        assert_eq!(build(&spec, None, 10).topic, Some(Topic::Setup));
    }

    #[test]
    fn a_confirmed_answer_is_not_a_loose_idea_and_a_stop_ignores_the_marker() {
        let spec = story(Some("A story"), [None; 4], &[]);
        let m = mid(None, Some(("a courier", true)), 1);
        assert_eq!(build(&spec, Some(&m), 10).loose_idea, None);
        let stopped = Marker {
            format: FORMAT,
            state: State::StoppingPoint,
            last_answer: None,
            topic: None,
            answers_since_offer: 0,
            date: "2026-10-25".into(),
        };
        let r = build(&spec, Some(&stopped), 10);
        assert_eq!(r.last_topic, None);
        assert_eq!(r.loose_idea, None);
    }

    #[test]
    fn the_save_offer_is_due_at_the_setting() {
        let spec = story(Some("A story"), [None; 4], &[]);
        let nine = mid(None, Some(("x", true)), 9);
        assert!(!build(&spec, Some(&nine), 10).save_offer_due());
        let ten = mid(None, Some(("x", true)), 10);
        let r = build(&spec, Some(&ten), 10);
        assert!(r.save_offer_due());
        assert_eq!(r.to_value()["save_offer"], true);
        // The setting can be lowered.
        assert!(build(&spec, Some(&nine), 5).save_offer_due());
    }

    #[test]
    fn people_are_listed_alphabetically_whatever_their_ids() {
        let spec = story(Some("A story"), [None; 4], &["Mara", "jonah", "Ada"]);
        assert_eq!(build(&spec, None, 10).people, ["Ada", "jonah", "Mara"]);
    }

    #[test]
    fn the_cast_is_cut_to_a_dozen_names() {
        let names: Vec<String> = (0..20).map(|i| format!("P{i}")).collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let spec = story(Some("A story"), [None; 4], &refs);
        assert_eq!(build(&spec, None, 10).people.len(), MAX_PEOPLE);
    }
}
