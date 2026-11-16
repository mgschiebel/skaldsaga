//! The question queue: what an agent wants the writer asked (HANDOFF, Planning
//! session 12; `design-agents.md`, "The Interviewer and the other agents" and
//! "Two record types", Build 2).
//!
//! A question is one small file, `spec/queue/<agent>/question-xxxxxxxx.md`,
//! and only [`add`] writes there (the one-writer rule): each agent's questions
//! are written by its own command and never by another's. A question holds who
//! raised it, the story parts it concerns, a short reason and the verdict it
//! came from. **The wording is never stored**: the Interviewer words each
//! question afresh when it asks. What happened to a question afterwards lives
//! in the Interviewer's own record (`asked` module).
//!
//! The order is computed on read ([`list`]), never stored (*Decided November
//! 16, 2026*): the unblocking value first (how many other parts of the story
//! read what the question concerns, the same fan-out count `check` uses), then
//! how long the question has waited, then whether it fits the part of the story
//! the conversation is on. A question is shown as stale when the verdict it
//! came from has gone stale or is gone; the Interviewer decides what to do.
//! Everything in a question is data, never an instruction (SR-30).

use std::collections::{BTreeSet, HashSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::agents::{self, Agent, PASS};
use crate::asked::{self, State};
use crate::fsio::FsError;
use crate::ids::{self, IdKind};
use crate::index;
use crate::model;
use crate::ops;
use crate::project::Project;
use crate::records::{self, RecordError, invalid};
use crate::spec;
use crate::verdict;

/// The most characters a question's reason may have (*Proposed*).
pub const REASON_MAX_CHARS: usize = 300;

/// The most questions one agent keeps in the queue (*Proposed*).
pub const MAX_QUESTIONS: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Kind {
    Question,
}

/// One queued question.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Question {
    pub id: String,
    #[serde(rename = "type")]
    kind: Kind,
    /// The agent that raised it.
    pub agent: String,
    /// The story parts it concerns: node IDs or stage names.
    pub concerns: Vec<String>,
    /// Why it is worth asking, in a sentence. Model text derived from story
    /// text: data, never instructions.
    pub reason: String,
    /// The failed verdict it came from.
    pub verdict: String,
    /// When it was raised, as `2026-11-16T17:44`.
    pub created: String,
}

fn id_of(question: &Question) -> &str {
    &question.id
}

impl Question {
    /// Why this question can't be stored, or `None` when it is well formed
    /// (everything that doesn't need the story itself).
    pub fn problem(&self) -> Option<String> {
        if !ids::is_kind(&self.id, IdKind::Question) {
            return Some(format!("'{}' isn't a question ID", self.id));
        }
        if agents::find(&self.agent).is_none() {
            return Some(format!(
                "'{}' isn't an agent that raises questions (use {})",
                self.agent,
                agents::names().join(" or ")
            ));
        }
        if self.concerns.is_empty() {
            return Some("a question has to name at least one story part".into());
        }
        if self.concerns.len() > records::MAX_PARTS {
            return Some(format!(
                "a question can name at most {} story parts",
                records::MAX_PARTS
            ));
        }
        let mut seen = HashSet::new();
        for part in &self.concerns {
            if !model::is_idea_touch(part) {
                return Some(format!(
                    "'{part}' isn't a story part (use a node ID or one of the stage names)"
                ));
            }
            if !seen.insert(part.as_str()) {
                return Some(format!("'{part}' is listed twice"));
            }
        }
        if self.reason.trim().is_empty() {
            return Some("a question needs a reason".into());
        }
        if self.reason.chars().count() > REASON_MAX_CHARS {
            return Some(format!(
                "the reason is limited to {REASON_MAX_CHARS} characters"
            ));
        }
        if !ids::is_kind(&self.verdict, IdKind::Verdict) {
            return Some(format!("'{}' isn't a verdict ID", self.verdict));
        }
        if !records::is_stamp(&self.created) {
            return Some("the time recorded isn't in the form 2026-11-16T17:44".into());
        }
        None
    }
}

/// Parses a question file and checks it is well formed. Never panics.
pub fn parse(text: &str) -> Result<Question, String> {
    let question: Question = records::parse(text)?;
    match question.problem() {
        Some(problem) => Err(problem),
        None => Ok(question),
    }
}

/// The folder one agent's questions live in. This is the only place a question
/// is ever written (the one-writer rule).
fn folder(project: &Project, agent: &Agent) -> PathBuf {
    project
        .spec_dir()
        .join(records::QUEUE_FOLDER)
        .join(agent.name)
}

fn label(agent: &Agent) -> String {
    format!("spec/{}/{}", records::QUEUE_FOLDER, agent.name)
}

fn relative_path(agent: &Agent, id: &str) -> String {
    format!("{}/{id}.md", label(agent))
}

/// Loads one agent's questions.
pub(crate) fn load(project: &Project, agent: &Agent) -> Result<records::Loaded<Question>, FsError> {
    let mut loaded = records::load_folder::<Question>(
        &folder(project, agent),
        &label(agent),
        IdKind::Question,
        id_of,
        MAX_QUESTIONS,
    )?;
    let mut keep = Vec::new();
    for question in loaded.records {
        match question.problem() {
            None if question.agent == agent.name => keep.push(question),
            None => loaded.problems.push(format!(
                "{}: this question belongs to another agent",
                relative_path(agent, &question.id)
            )),
            Some(problem) => loaded
                .problems
                .push(format!("{}: {problem}", relative_path(agent, &question.id))),
        }
    }
    loaded.records = keep;
    Ok(loaded)
}

/// Finds a question by ID in any agent's folder, or `None`.
pub(crate) fn find_any(project: &Project, id: &str) -> Result<Option<Question>, RecordError> {
    for agent in &agents::AGENTS {
        let found =
            records::find_in::<Question>(&folder(project, agent), id, IdKind::Question, id_of)?;
        if let Some(question) = found {
            return match question.problem() {
                None if question.agent == agent.name => Ok(Some(question)),
                None => invalid(format!("'{id}' belongs to another agent")),
                Some(problem) => invalid(format!("'{id}' can't be used: {problem}")),
            };
        }
    }
    Ok(None)
}

/// What the skill hands `queue add`: the parts of a question the agent knows.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Draft {
    concerns: Vec<String>,
    reason: String,
    verdict: String,
}

/// What `add` did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Added {
    pub id: String,
    /// The file to stage with the next save, relative to the story folder.
    pub path: String,
    /// True when the same question was already queued, so nothing was written
    /// and the existing one is returned.
    pub existing: bool,
}

/// Queues one question for `agent_name` from a JSON draft. `created` is the
/// time to record, in the form `2026-11-16T17:44`.
pub fn add(
    project: &Project,
    agent_name: &str,
    draft: &str,
    created: &str,
) -> Result<Added, RecordError> {
    let Some(agent) = agents::find(agent_name) else {
        return invalid(format!(
            "'{agent_name}' isn't an agent that raises questions (use {})",
            agents::names().join(" or ")
        ));
    };
    let draft: Draft = serde_json::from_str(draft)
        .map_err(|e| RecordError::Invalid(format!("the question file isn't usable: {e}")))?;
    let question = Question {
        // Replaced below; a well-formed placeholder so `problem` checks the rest.
        id: ids::new_id(IdKind::Question).map_err(|e| RecordError::Invalid(e.to_string()))?,
        kind: Kind::Question,
        agent: agent.name.to_string(),
        concerns: draft.concerns,
        reason: records::normalized(&draft.reason),
        verdict: draft.verdict,
        created: created.to_string(),
    };
    if let Some(problem) = question.problem() {
        return invalid(problem);
    }
    let loaded = spec::load(&project.spec_dir())?;
    records::check_parts(&loaded.spec, &question.concerns).or_else(invalid)?;

    // It has to come from a verdict this agent recorded, and one that failed:
    // a pass raises nothing.
    let Some(source) = verdict::find(project, agent, &question.verdict)? else {
        return invalid(format!("there's no verdict '{}'", question.verdict));
    };
    if source.result == PASS {
        return invalid("a verdict that passed raises no question");
    }

    let existing = load(project, agent)?;
    if let Some(same) = existing.records.iter().find(|q| {
        q.verdict == question.verdict
            && q.concerns == question.concerns
            && q.reason == question.reason
    }) {
        return Ok(Added {
            id: same.id.clone(),
            path: relative_path(agent, &same.id),
            existing: true,
        });
    }
    if existing.records.len() >= MAX_QUESTIONS {
        return invalid(format!(
            "{} already has {MAX_QUESTIONS} questions waiting, which is the most it keeps",
            agent.name
        ));
    }

    // Question IDs are unique across every agent's queue, because a record of
    // a question asked points at one by ID alone.
    let mut all = BTreeSet::new();
    for other in &agents::AGENTS {
        for q in load(project, other)?.records {
            all.insert(q.id);
        }
    }
    let dir = folder(project, agent);
    let id = records::fresh_id(IdKind::Question, |id| {
        all.contains(id) || std::fs::symlink_metadata(dir.join(format!("{id}.md"))).is_ok()
    })?;
    let question = Question { id, ..question };
    let text = records::render(&question)?;
    project.ensure_dir(&dir)?;
    project.write_atomic(&dir.join(format!("{}.md", question.id)), &text)?;
    Ok(Added {
        path: relative_path(agent, &question.id),
        id: question.id,
        existing: false,
    })
}

/// How a listed question stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standing {
    /// Not put to the writer yet.
    Open,
    /// Put to the writer, no answer yet.
    Asked,
}

impl Standing {
    pub fn as_str(self) -> &'static str {
        match self {
            Standing::Open => "open",
            Standing::Asked => "asked",
        }
    }
}

/// One question as `list` returns it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed {
    pub question: Question,
    pub standing: Standing,
    /// How many other parts of the story read what the question concerns.
    pub unblocking: usize,
    /// True when the verdict it came from has gone stale or is gone.
    pub stale: bool,
    /// True when the question concerns the part asked about with `near`.
    pub near: bool,
}

/// What `list` found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listing {
    /// Best first: unblocking value (most first), then the longest waiting,
    /// then those near the part asked about.
    pub questions: Vec<Listed>,
    /// A note for every file that couldn't be used.
    pub problems: Vec<String>,
}

/// The questions still waiting, ranked. Read-only: the order is worked out
/// here and written nowhere. Questions the writer answered or set aside are
/// left out. `near` is the part of the story the conversation is on (a node ID
/// or a stage name), for the third tie-break.
pub fn list(
    project: &Project,
    agent: Option<&str>,
    near: Option<&str>,
) -> Result<Listing, RecordError> {
    let chosen: Vec<&Agent> = match agent {
        Some(name) => match agents::find(name) {
            Some(a) => vec![a],
            None => {
                return invalid(format!(
                    "'{name}' isn't an agent that raises questions (use {})",
                    agents::names().join(" or ")
                ));
            }
        },
        None => agents::AGENTS.iter().collect(),
    };
    if let Some(part) = near
        && !model::is_idea_touch(part)
    {
        return invalid(format!(
            "'{part}' isn't a story part (use a node ID or one of the stage names)"
        ));
    }

    let loaded = spec::load(&project.spec_dir())?;
    let current = ops::revisions(&loaded.spec);
    let readers = index::readers(&loaded.spec);
    let asked = asked::load(project)?;
    let mut listing = Listing {
        questions: Vec::new(),
        problems: asked.problems,
    };
    for agent in chosen {
        let found = load(project, agent)?;
        listing.problems.extend(found.problems);
        let verdicts = verdict::load(project, agent)?;
        for question in found.records {
            let standing = match asked
                .records
                .iter()
                .find(|a| a.question == question.id)
                .map(|a| a.state)
            {
                None => Standing::Open,
                Some(State::Asked) => Standing::Asked,
                Some(State::Answered | State::SetAside) => continue,
            };
            let concerned: BTreeSet<&str> = question.concerns.iter().map(String::as_str).collect();
            let mut fan: BTreeSet<&str> = BTreeSet::new();
            for id in &concerned {
                for (from, _) in readers.get(*id).into_iter().flatten() {
                    if from != index::SEQUENCE_FROM && !concerned.contains(from.as_str()) {
                        fan.insert(from);
                    }
                }
            }
            let stale = verdicts
                .records
                .iter()
                .find(|v| v.id == question.verdict)
                .is_none_or(|v| verdict::is_stale(v, &current));
            listing.questions.push(Listed {
                unblocking: fan.len(),
                near: near.is_some_and(|part| concerned.contains(part)),
                standing,
                stale,
                question,
            });
        }
    }
    listing.questions.sort_by(|a, b| {
        b.unblocking
            .cmp(&a.unblocking)
            .then_with(|| a.question.created.cmp(&b.question.created))
            .then_with(|| b.near.cmp(&a.near))
            .then_with(|| a.question.id.cmp(&b.question.id))
    });
    Ok(listing)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::records::testkit::{DAD, JONAH, MARA, question_draft, story, verdict_draft};
    use crate::testutil::TempDir;

    const NOW: &str = "2026-11-16T17:44";

    fn refusal(r: Result<Added, RecordError>) -> String {
        match r {
            Err(RecordError::Invalid(m)) => m,
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    /// Records a failed verdict by Continuity and returns its ID.
    fn failed_verdict(project: &Project, why: &str) -> String {
        verdict::add(
            project,
            "continuity",
            &verdict_draft("contradiction_reported", &[(MARA, 2)], why),
            NOW,
        )
        .unwrap()
        .id
    }

    fn queue(project: &Project, concerns: &[&str], reason: &str, verdict: &str, at: &str) -> Added {
        add(
            project,
            "continuity",
            &question_draft(concerns, reason, verdict),
            at,
        )
        .unwrap()
    }

    /// How many other parts of the story read `id`, worked out the way the
    /// ranking does it, so the tests don't hard-code the fixture's links.
    fn readers_of(project: &Project, id: &str) -> usize {
        let loaded = spec::load(&project.spec_dir()).unwrap();
        index::readers(&loaded.spec)
            .get(id)
            .map(|r| {
                r.iter()
                    .map(|(from, _)| from.as_str())
                    .filter(|from| *from != index::SEQUENCE_FROM && *from != id)
                    .collect::<BTreeSet<_>>()
                    .len()
            })
            .unwrap_or(0)
    }

    #[test]
    fn a_question_from_a_failed_verdict_is_queued_in_its_agents_folder() {
        let (_dir, project) = story();
        let verdict = failed_verdict(&project, "Eyes differ.");
        let added = queue(
            &project,
            &[MARA, "crisis"],
            "The eye colour clashes.",
            &verdict,
            NOW,
        );
        assert!(!added.existing);
        assert!(ids::is_kind(&added.id, IdKind::Question));
        assert_eq!(added.path, format!("spec/queue/continuity/{}.md", added.id));
        let text = std::fs::read_to_string(project.root().join(&added.path)).unwrap();
        assert!(text.contains(&format!("verdict: {verdict}")));
        // The wording is never stored: there is nowhere to put it.
        let with_wording = serde_json::json!({
            "concerns": [MARA], "reason": "r", "verdict": verdict, "wording": "Is Mara's hair red?"
        })
        .to_string();
        assert!(refusal(add(&project, "continuity", &with_wording, NOW)).contains("isn't usable"));
        // Queued twice is queued once.
        let again = queue(
            &project,
            &[MARA, "crisis"],
            "The eye colour   clashes.",
            &verdict,
            "2026-11-16T18:00",
        );
        assert!(again.existing);
        assert_eq!(again.id, added.id);
        assert!(!project.spec_dir().join("verdicts/queue").exists());
    }

    #[test]
    fn a_question_needs_a_real_failed_verdict_and_real_story_parts() {
        let (_dir, project) = story();
        let failed = failed_verdict(&project, "Eyes differ.");
        let passed = verdict::add(
            &project,
            "continuity",
            &verdict_draft("pass", &[(JONAH, 1)], "Fine."),
            NOW,
        )
        .unwrap()
        .id;
        let ask = |concerns: &[&str], reason: &str, verdict: &str| {
            refusal(add(
                &project,
                "continuity",
                &question_draft(concerns, reason, verdict),
                NOW,
            ))
        };
        assert!(ask(&[MARA], "r", &passed).contains("a verdict that passed raises no question"));
        assert!(ask(&[MARA], "r", "verdict-zzzzzzzz").contains("there's no verdict"));
        assert!(ask(&[MARA], "r", "../x").contains("isn't a verdict ID"));
        assert!(ask(&[], "r", &failed).contains("at least one story part"));
        assert!(ask(&["p-zzzzzzzz"], "r", &failed).contains("isn't a part of this story"));
        assert!(ask(&["middle"], "r", &failed).contains("isn't a story part"));
        assert!(ask(&["../x"], "r", &failed).contains("isn't a story part"));
        assert!(ask(&[MARA, MARA], "r", &failed).contains("listed twice"));
        assert!(ask(&[MARA], "  ", &failed).contains("needs a reason"));
        assert!(ask(&[MARA], &"x".repeat(REASON_MAX_CHARS + 1), &failed).contains("300"));
        let many: Vec<&str> = vec![MARA; records::MAX_PARTS + 1];
        assert!(ask(&many, "r", &failed).contains("at most"));
        // Nothing was written for any of them.
        assert!(!project.spec_dir().join("queue").exists());
    }

    #[test]
    fn only_the_agent_that_recorded_the_verdict_may_raise_from_it() {
        let (_dir, project) = story();
        let failed = failed_verdict(&project, "Eyes differ.");
        for agent in ["interviewer", "weaver", ""] {
            let m = refusal(add(
                &project,
                agent,
                &question_draft(&[MARA], "r", &failed),
                NOW,
            ));
            assert!(
                m.contains("isn't an agent that raises questions"),
                "{agent}: {m}"
            );
        }
        assert!(matches!(
            list(&project, Some("interviewer"), None),
            Err(RecordError::Invalid(_))
        ));
    }

    #[test]
    fn the_queue_is_ranked_by_unblocking_value_then_waiting_then_nearness() {
        let (_dir, project) = story();
        let people = [MARA, JONAH, DAD];
        let counts: Vec<usize> = people.iter().map(|p| readers_of(&project, p)).collect();
        let hi = people[counts
            .iter()
            .position(|c| *c == *counts.iter().max().unwrap())
            .unwrap()];
        let lo = people[counts
            .iter()
            .position(|c| *c == *counts.iter().min().unwrap())
            .unwrap()];
        assert!(
            readers_of(&project, hi) > readers_of(&project, lo),
            "{counts:?}"
        );

        let verdict = failed_verdict(&project, "Something clashes.");
        // The oldest question concerns the part few others read.
        let old_low = queue(
            &project,
            &[lo],
            "Old and low.",
            &verdict,
            "2026-11-16T09:00",
        );
        let new_high = queue(
            &project,
            &[hi],
            "New and high.",
            &verdict,
            "2026-11-16T17:00",
        );
        // Same unblocking value and age: the one near the conversation first.
        let near_setup = queue(
            &project,
            &[lo, "setup"],
            "Near setup.",
            &verdict,
            "2026-11-16T10:00",
        );
        let near_crisis = queue(
            &project,
            &[lo, "crisis"],
            "Near crisis.",
            &verdict,
            "2026-11-16T10:00",
        );

        let order = |near: Option<&str>| -> Vec<String> {
            list(&project, None, near)
                .unwrap()
                .questions
                .into_iter()
                .map(|l| l.question.id)
                .collect()
        };
        // Unblocking value beats age; then the one waiting longest; then nearness.
        let plain = order(None);
        assert_eq!(plain.first(), Some(&new_high.id), "{plain:?}");
        assert_eq!(plain.get(1), Some(&old_low.id), "{plain:?}");
        let near_a = order(Some("crisis"));
        let near_b = order(Some("setup"));
        let pos = |v: &[String], id: &str| v.iter().position(|x| x == id).unwrap();
        assert!(pos(&near_a, &near_crisis.id) < pos(&near_a, &near_setup.id));
        assert!(pos(&near_b, &near_setup.id) < pos(&near_b, &near_crisis.id));
        // Nothing is written by listing, and `near` has to be a story part.
        assert_eq!(order(None), plain);
        assert!(matches!(
            list(&project, None, Some("../x")),
            Err(RecordError::Invalid(_))
        ));
        let listed = list(&project, None, Some("setup")).unwrap();
        let flagged: Vec<_> = listed
            .questions
            .iter()
            .filter(|l| l.near)
            .map(|l| l.question.id.clone())
            .collect();
        assert_eq!(flagged, [near_setup.id]);
    }

    #[test]
    fn answered_and_set_aside_questions_leave_the_list_and_asked_ones_say_so() {
        let (_dir, project) = story();
        let verdict = failed_verdict(&project, "x");
        let a = queue(&project, &[MARA], "A.", &verdict, "2026-11-16T09:00");
        let b = queue(&project, &[MARA], "B.", &verdict, "2026-11-16T09:01");
        let c = queue(&project, &[MARA], "C.", &verdict, "2026-11-16T09:02");
        let states = |p: &Project| -> Vec<(String, &'static str)> {
            list(p, None, None)
                .unwrap()
                .questions
                .into_iter()
                .map(|l| (l.question.id, l.standing.as_str()))
                .collect()
        };
        assert_eq!(states(&project).len(), 3);
        asked::mark(&project, &a.id, State::Asked).unwrap();
        asked::mark(&project, &b.id, State::Answered).unwrap();
        asked::mark(&project, &c.id, State::SetAside).unwrap();
        assert_eq!(states(&project), [(a.id, "asked")]);
    }

    #[test]
    fn a_question_is_stale_when_its_verdict_is() {
        let (_dir, project) = story();
        let verdict = failed_verdict(&project, "x");
        queue(&project, &[MARA], "Q.", &verdict, NOW);
        assert!(!list(&project, None, None).unwrap().questions[0].stale);
        let path = project.spec_dir().join("people/p-maraharb.md");
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(&path, text.replacen("revision: 2\n", "revision: 3\n", 1)).unwrap();
        assert!(list(&project, None, None).unwrap().questions[0].stale);
        // A verdict that is gone leaves the question stale too, not lost.
        std::fs::remove_dir_all(project.spec_dir().join("verdicts")).unwrap();
        let listing = list(&project, None, None).unwrap();
        assert_eq!(listing.questions.len(), 1);
        assert!(listing.questions[0].stale);
    }

    #[test]
    fn a_damaged_question_file_is_reported_and_never_trusted() {
        let (_dir, project) = story();
        let verdict = failed_verdict(&project, "x");
        let added = queue(&project, &[MARA], "Q.", &verdict, NOW);
        let folder = project.spec_dir().join("queue/continuity");
        let good = std::fs::read_to_string(project.root().join(&added.path)).unwrap();
        std::fs::write(folder.join("question-aaaaaaaa.md"), "junk").unwrap();
        std::fs::write(
            folder.join("question-bbbbbbbb.md"),
            good.replace(&added.id, "question-bbbbbbbb")
                .replace("continuity", "weaver"),
        )
        .unwrap();
        std::fs::write(
            folder.join("question-cccccccc.md"),
            good.replace(&added.id, "question-cccccccc")
                .replace(MARA, "../escape"),
        )
        .unwrap();
        let listing = list(&project, None, None).unwrap();
        assert_eq!(listing.questions.len(), 1);
        assert_eq!(listing.problems.len(), 3, "{:#?}", listing.problems);
        assert!(parse(&good).is_ok());
    }

    #[test]
    fn an_empty_story_lists_nothing_and_a_missing_folder_is_fine() {
        let dir = TempDir::new();
        let project = Project::new(dir.path());
        let listing = list(&project, None, None).unwrap();
        assert!(listing.questions.is_empty() && listing.problems.is_empty());
    }
}
