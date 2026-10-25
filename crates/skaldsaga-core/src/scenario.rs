//! `skaldsaga scenario <name>`: developer-only test setup. It files the fixed
//! sample answers a slice's test case needs into a set-up project, so the
//! author can then run the command under test.
//!
//! Scenario content is built into the program and never read from the story
//! folder (SR-10 to SR-12). A scenario only runs in a set-up project whose
//! spec is still empty, so it never mixes sample content into a real story.
//!
//! `basics` (the former `demo`, decided in build step 8): the first answer
//! creates a tiny story (Mara, Jonah, the docks, two scenes) and is flagged;
//! the second has Mara learn about Jonah's arrest in a scene that happens
//! before the arrest, which the checker refuses. It stays because it is the
//! only way to see a conflict refused until the interviewer can produce one.
//!
//! `resume` (slice 2): the Jack and Jill story, left mid-thread: a premise,
//! two people, a main plot with empty stages, and a marker holding a loose
//! idea. It goes through the real `propose` and `commit` steps and seeds the
//! marker through the marker code `session answer` and `session ask` use, so
//! it can't drift from the real file formats. Its acceptance check: `/skaldsaga`
//! with no text gives a short summary naming the story and Jack and Jill, one
//! question about how the story begins, and changes nothing on disk.

use crate::commit::{self, CommitOptions, Outcome};
use crate::fsio::{self, FsError};
use crate::marker::{self, MarkerError, Topic};
use crate::project::Project;
use crate::propose::{self, ProposeError};
use crate::spec;

/// A named test setup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scenario {
    /// Mara and Jonah: one answer that is kept, one that is refused.
    Basics,
    /// Jack and Jill, left mid-thread: the setup for the mid-thread recap.
    Resume,
}

/// Every scenario name, for usage messages.
pub const NAMES: [&str; 2] = ["basics", "resume"];

impl Scenario {
    /// Looks a scenario up by name; `None` for an unknown name.
    pub fn parse(name: &str) -> Option<Scenario> {
        match name {
            "basics" => Some(Scenario::Basics),
            "resume" => Some(Scenario::Resume),
            _ => None,
        }
    }

    /// The sample answers filed in the inbox, in the order to try them. A
    /// scenario that commits its own content (`resume`) leaves none.
    pub fn proposals(self) -> &'static [&'static str] {
        match self {
            Scenario::Basics => &["pr-demstart", "pr-demmixup"],
            Scenario::Resume => &[],
        }
    }

    /// The one command to run next.
    pub fn next_command(self) -> &'static str {
        match self {
            Scenario::Basics => "skaldsaga check --proposal pr-demstart --proposal pr-demmixup",
            Scenario::Resume => "/skaldsaga (type nothing after it)",
        }
    }

    /// What to expect from that command.
    pub fn expect(self) -> &'static str {
        match self {
            Scenario::Basics => {
                "pr-demstart comes back flagged (its scenes are not tied to a thread and stage); \
                 pr-demmixup is refused with learns_before_true"
            }
            Scenario::Resume => {
                "two or three plain sentences that name the story (a modern Jack and Jill, set \
                 in a city at night) and Jack and Jill, the dark alley mentioned as a loose \
                 idea, then one question about how the story begins; nothing on disk changes"
            }
        }
    }

    fn texts(self) -> &'static [&'static str] {
        match self {
            Scenario::Basics => &[START, MIXUP],
            Scenario::Resume => &[],
        }
    }
}

const START: &str = r#"{
  "id": "pr-demstart",
  "agent": "interviewer",
  "created": "2026-10-18T12:00",
  "answer": "Mara is a courier at the east docks. Jonah is arrested after she crosses the checkpoint, and she sees it happen.",
  "ops": [
    {"op": "create", "node": {"id": "p-demmara2", "type": "person", "name": "Mara Vance",
      "aliases": ["Mara"], "summary": "Courier who smuggles letters past the blockade"}},
    {"op": "create", "node": {"id": "p-demjnah2", "type": "person", "name": "Jonah",
      "summary": "Mara's partner on the letter runs",
      "facts": [{"id": "f-demarstd", "kind": "truth", "statement": "Jonah is arrested",
        "true_from": {"in": "s-demarrst"},
        "characters": [{"who": "p-demmara2", "learns_in": "s-demarrst", "how": "saw"}],
        "confidence": "provisional", "review": "unreviewed", "source": "interview"}]}},
    {"op": "create", "node": {"id": "l-demdcks2", "type": "place", "name": "The east docks"}},
    {"op": "create", "node": {"id": "s-demchckp", "type": "scene", "name": "The checkpoint",
      "pov": "p-demmara2", "location": "l-demdcks2", "purpose": "Mara gets the letters past the guards",
      "status": "slot"}},
    {"op": "create", "node": {"id": "s-demarrst", "type": "scene", "name": "The arrest",
      "pov": "p-demmara2", "location": "l-demdcks2", "participants": ["p-demmara2", "p-demjnah2"],
      "when": {"after": "s-demchckp", "offset": "that evening"},
      "purpose": "Mara watches Jonah taken away", "status": "slot"}},
    {"op": "add_item", "target": "sequence", "value": "s-demchckp"},
    {"op": "add_item", "target": "sequence", "value": "s-demarrst"}
  ]
}
"#;

const MIXUP: &str = r#"{
  "id": "pr-demmixup",
  "agent": "interviewer",
  "created": "2026-10-18T12:05",
  "answer": "At the checkpoint, Mara already knows Jonah has been arrested.",
  "ops": [
    {"op": "add_item", "target": "p-demjnah2#f-demarstd.characters",
     "value": {"who": "p-demmara2", "learns_in": "s-demchckp", "how": "told"}}
  ]
}
"#;

/// What `write` did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioOutcome {
    /// The scenario is in place.
    Written,
    /// The spec already holds nodes; a scenario only runs in an empty story.
    SpecNotEmpty,
    /// The scenario has already been set up here.
    AlreadyWritten,
}

/// The time to stamp on what a scenario files.
#[derive(Debug, Clone, Copy)]
pub struct When<'a> {
    /// `YYYY-MM-DDTHH:MM`.
    pub created: &'a str,
    /// `YYYY-MM-DD`.
    pub date: &'a str,
}

/// What `write` did, and what to save.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filed {
    pub outcome: ScenarioOutcome,
    /// Paths for `git add`: what the scenario committed and the marker. Empty
    /// for a scenario that only files answers in the inbox.
    pub add_paths: Vec<String>,
    /// A one-line snapshot message, when there are paths to save.
    pub message: Option<String>,
}

impl Filed {
    fn only(outcome: ScenarioOutcome) -> Filed {
        Filed {
            outcome,
            add_paths: Vec::new(),
            message: None,
        }
    }
}

/// Why a scenario couldn't be set up.
#[derive(Debug)]
pub enum ScenarioError {
    Fs(FsError),
    Marker(MarkerError),
    /// A step of the real pipeline refused the scenario's own content; that
    /// is a bug in the scenario, so the message says which step.
    Pipeline(String),
}

impl std::fmt::Display for ScenarioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScenarioError::Fs(e) => e.fmt(f),
            ScenarioError::Marker(e) => e.fmt(f),
            ScenarioError::Pipeline(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for ScenarioError {}

impl From<FsError> for ScenarioError {
    fn from(e: FsError) -> Self {
        ScenarioError::Fs(e)
    }
}

impl From<MarkerError> for ScenarioError {
    fn from(e: MarkerError) -> Self {
        ScenarioError::Marker(e)
    }
}

/// Sets a scenario up in a set-up project with an empty story.
pub fn write(
    project: &Project,
    scenario: Scenario,
    when: When<'_>,
) -> Result<Filed, ScenarioError> {
    let loaded = spec::load(&project.spec_dir())?;
    let archive = project.harness_dir().join(crate::commit::COMMITTED_DIR);
    for id in scenario.proposals() {
        let name = format!("{id}.json");
        if project.inbox_dir().join(&name).exists() || archive.join(&name).exists() {
            return Ok(Filed::only(ScenarioOutcome::AlreadyWritten));
        }
    }
    if scenario == Scenario::Resume && marker::exists(project) {
        return Ok(Filed::only(ScenarioOutcome::AlreadyWritten));
    }
    if !loaded.spec.nodes.is_empty() || !loaded.spec.sequence.is_empty() {
        return Ok(Filed::only(ScenarioOutcome::SpecNotEmpty));
    }
    match scenario {
        Scenario::Basics => {
            for (id, text) in scenario.proposals().iter().zip(scenario.texts()) {
                fsio::write_atomic(&project.inbox_dir().join(format!("{id}.json")), text)?;
            }
            Ok(Filed::only(ScenarioOutcome::Written))
        }
        Scenario::Resume => write_resume(project, when),
    }
}

/// The Jack and Jill answer: the premise, two people, and a main plot with
/// four empty stages. Filed through `propose`, so the program names the IDs.
const RESUME_DRAFT: &str = r#"{
  "answer": "A modern Jack and Jill, set in a city at night. Jack and Jill.",
  "ops": [
    {"op": "create", "node": {"id": "@a:main", "type": "arc", "name": "Main story",
      "status": "main", "threads": ["@t:main"],
      "premise": {"current": "A modern Jack and Jill, set in a city at night"}}},
    {"op": "create", "node": {"id": "@t:main", "type": "thread", "name": "The main plot",
      "kind": "big_picture", "arc": "@a:main",
      "stages": [{"stage": "setup"}, {"stage": "complications"},
                 {"stage": "crisis"}, {"stage": "resolution"}]}},
    {"op": "create", "node": {"id": "@p:jack", "type": "person", "name": "Jack"}},
    {"op": "create", "node": {"id": "@p:jill", "type": "person", "name": "Jill"}}
  ]
}"#;

/// What the writer floated after the second answer: a tentative idea, never
/// confirmed, so it stays out of the story and lives in the marker only.
const RESUME_LOOSE_IDEA: &str = "maybe they enter a dark alley";

fn write_resume(project: &Project, when: When<'_>) -> Result<Filed, ScenarioError> {
    let id = propose::file(project, RESUME_DRAFT, when.created).map_err(|e| match e {
        ProposeError::Fs(e) => ScenarioError::Fs(e),
        ProposeError::Invalid(m) => ScenarioError::Pipeline(format!("propose refused: {m}")),
    })?;
    let outcome = commit::commit(project, &id, CommitOptions::default())?;
    let Outcome::Committed {
        mut add_paths,
        message,
        ..
    } = outcome
    else {
        return Err(ScenarioError::Pipeline(
            "commit didn't take the resume answer".into(),
        ));
    };
    // The marker, through the same code `session answer` and `session ask`
    // use: the loose idea unconfirmed, and people the topic last asked about.
    marker::record_answer(project, RESUME_LOOSE_IDEA, false, when.date)?;
    let recorded = marker::record_ask(project, Topic::People, when.date)?;
    add_paths.push(recorded.path);
    Ok(Filed {
        outcome: ScenarioOutcome::Written,
        add_paths,
        message: Some(message),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proposal;

    #[test]
    fn sample_answers_parse() {
        for text in Scenario::Basics.texts() {
            let p = proposal::parse(text).unwrap();
            assert!(
                crate::ids::is_kind(&p.id, crate::ids::IdKind::Proposal),
                "{}",
                p.id
            );
        }
    }

    #[test]
    fn names_match_the_lookup() {
        for name in NAMES {
            assert!(Scenario::parse(name).is_some(), "{name}");
        }
        assert_eq!(Scenario::parse("demo"), None);
        assert_eq!(Scenario::parse(""), None);
    }
}
