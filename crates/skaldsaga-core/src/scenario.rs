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

use crate::fsio::{self, FsError};
use crate::project::Project;
use crate::spec;

/// A named test setup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scenario {
    /// Mara and Jonah: one answer that is kept, one that is refused.
    Basics,
}

/// Every scenario name, for usage messages.
pub const NAMES: [&str; 1] = ["basics"];

impl Scenario {
    /// Looks a scenario up by name; `None` for an unknown name.
    pub fn parse(name: &str) -> Option<Scenario> {
        match name {
            "basics" => Some(Scenario::Basics),
            _ => None,
        }
    }

    /// The sample answers, in the order to try them.
    pub fn proposals(self) -> &'static [&'static str] {
        match self {
            Scenario::Basics => &["pr-demstart", "pr-demmixup"],
        }
    }

    /// The one command to run next.
    pub fn next_command(self) -> &'static str {
        match self {
            Scenario::Basics => "skaldsaga check --proposal pr-demstart --proposal pr-demmixup",
        }
    }

    /// What to expect from that command.
    pub fn expect(self) -> &'static str {
        match self {
            Scenario::Basics => {
                "pr-demstart comes back flagged (its scenes are not tied to a thread and stage); \
                 pr-demmixup is refused with learns_before_true"
            }
        }
    }

    fn texts(self) -> &'static [&'static str] {
        match self {
            Scenario::Basics => &[START, MIXUP],
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
    /// The sample answers are in the inbox.
    Written,
    /// The spec already holds nodes; a scenario only runs in an empty story.
    SpecNotEmpty,
    /// A sample answer is already in the inbox or the archive.
    AlreadyWritten,
}

/// Writes the sample answers into the inbox.
pub fn write(project: &Project, scenario: Scenario) -> Result<ScenarioOutcome, FsError> {
    let loaded = spec::load(&project.spec_dir())?;
    let archive = project.harness_dir().join(crate::commit::COMMITTED_DIR);
    for id in scenario.proposals() {
        let name = format!("{id}.json");
        if project.inbox_dir().join(&name).exists() || archive.join(&name).exists() {
            return Ok(ScenarioOutcome::AlreadyWritten);
        }
    }
    if !loaded.spec.nodes.is_empty() || !loaded.spec.sequence.is_empty() {
        return Ok(ScenarioOutcome::SpecNotEmpty);
    }
    for (id, text) in scenario.proposals().iter().zip(scenario.texts()) {
        fsio::write_atomic(&project.inbox_dir().join(format!("{id}.json")), text)?;
    }
    Ok(ScenarioOutcome::Written)
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
