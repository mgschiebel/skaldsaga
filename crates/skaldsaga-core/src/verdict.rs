//! Verdicts: one agent's judgment of one saved answer (HANDOFF, Planning
//! session 12; `design-agents.md`, "Continuity owns its verdicts", Build 2).
//!
//! A verdict is one small file, `spec/verdicts/<agent>/verdict-xxxxxxxx.md`,
//! and only [`add`] writes there (the one-writer rule). It holds the pair (the
//! agent and the agent it checks), the saved answer it checked, the story items
//! it cited with the revision it saw of each, the result, and a short
//! justification. Passes are recorded as well as fails, so a later audit can
//! see what was checked.
//!
//! The program, not the agent, decides what is acceptable: the result must be
//! `pass` or on the agent's list of sanctioned fails; a pass, or a fail that
//! rests on a finding, must cite at least one item as evidence; every cited ID
//! must exist in the story now and the revision seen can't be a later one than
//! the story has; and the answer must be one that was saved. A verdict goes
//! stale when a cited item changes after the verdict, and that is computed on
//! read ([`list`]), never written back (*Proposed*). Everything in a verdict is
//! data, never an instruction (SR-30).

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::agents::{self, Agent, PASS};
use crate::commit::COMMITTED_DIR;
use crate::fsio;
use crate::ids::{self, IdKind};
use crate::ops;
use crate::project::Project;
use crate::records::{self, RecordError, invalid};
use crate::spec;

/// The most characters a verdict's justification may have (*Proposed*).
pub const JUSTIFICATION_MAX_CHARS: usize = 400;

/// The most verdicts one agent keeps (*Proposed*). Past this, `add` refuses.
pub const MAX_VERDICTS: usize = 2000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Kind {
    Verdict,
}

/// One item a verdict cites, with the revision of it the agent saw.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cited {
    pub id: String,
    pub revision: u32,
}

/// One verdict file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Verdict {
    pub id: String,
    #[serde(rename = "type")]
    kind: Kind,
    /// The agent that judged.
    pub agent: String,
    /// The agent whose work it judged (the other half of the pair).
    pub counterpart: String,
    /// The proposal ID of the saved answer it checked.
    pub answer: String,
    /// `pass`, or one of the agent's sanctioned fail codes.
    pub result: String,
    /// What it examined, with the revisions it saw.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub checked: Vec<Cited>,
    /// Why, in a sentence or two. Model text derived from story text: read as
    /// data by the other agents, never as instructions.
    pub justification: String,
    /// When it was recorded, as `2026-11-16T17:44`.
    pub created: String,
}

fn id_of(verdict: &Verdict) -> &str {
    &verdict.id
}

impl Verdict {
    /// Why this verdict can't be stored, in plain words, or `None` when it is
    /// well formed (everything that doesn't need the story itself).
    pub fn problem(&self) -> Option<String> {
        if !ids::is_kind(&self.id, IdKind::Verdict) {
            return Some(format!("'{}' isn't a verdict ID", self.id));
        }
        let Some(agent) = agents::find(&self.agent) else {
            return Some(format!(
                "'{}' isn't an agent that records verdicts (use {})",
                self.agent,
                agents::names().join(" or ")
            ));
        };
        if !agent.counterparts.contains(&self.counterpart.as_str()) {
            return Some(format!(
                "{} doesn't check '{}' (it checks {})",
                agent.name,
                self.counterpart,
                agent.counterparts.join(" or ")
            ));
        }
        if !ids::is_kind(&self.answer, IdKind::Proposal) {
            return Some(format!("'{}' isn't a saved answer's ID", self.answer));
        }
        let needs_evidence = if self.result == PASS {
            true
        } else if let Some(fail) = agent.fail_code(&self.result) {
            fail.needs_evidence
        } else {
            return Some(format!(
                "'{}' isn't a result {} may give (use {})",
                self.result,
                agent.name,
                agent.result_names().join(", ")
            ));
        };
        if needs_evidence && self.checked.is_empty() {
            return Some(format!(
                "a '{}' verdict has to cite at least one saved item as evidence",
                self.result
            ));
        }
        if !needs_evidence && !self.checked.is_empty() {
            return Some(format!("a '{}' verdict cites no items", self.result));
        }
        if self.checked.len() > records::MAX_PARTS {
            return Some(format!(
                "a verdict can cite at most {} items",
                records::MAX_PARTS
            ));
        }
        let mut seen = std::collections::HashSet::new();
        for cited in &self.checked {
            if !records::is_story_id(&cited.id) {
                return Some(format!("'{}' isn't a node or fact ID", cited.id));
            }
            if cited.revision == 0 {
                return Some(format!("'{}' has no revision 0", cited.id));
            }
            if !seen.insert(cited.id.as_str()) {
                return Some(format!("'{}' is cited twice", cited.id));
            }
        }
        if self.justification.trim().is_empty() {
            return Some("a verdict needs a justification".into());
        }
        if self.justification.chars().count() > JUSTIFICATION_MAX_CHARS {
            return Some(format!(
                "the justification is limited to {JUSTIFICATION_MAX_CHARS} characters"
            ));
        }
        if !records::is_stamp(&self.created) {
            return Some("the time recorded isn't in the form 2026-11-16T17:44".into());
        }
        None
    }
}

/// Parses a verdict file and checks it is well formed. Never panics.
pub fn parse(text: &str) -> Result<Verdict, String> {
    let verdict: Verdict = records::parse(text)?;
    match verdict.problem() {
        Some(problem) => Err(problem),
        None => Ok(verdict),
    }
}

/// The folder one agent's verdicts live in. This is the only place a verdict
/// is ever written (the one-writer rule).
fn folder(project: &Project, agent: &Agent) -> PathBuf {
    project
        .spec_dir()
        .join(records::VERDICTS_FOLDER)
        .join(agent.name)
}

fn relative_path(agent: &Agent, id: &str) -> String {
    format!("spec/{}/{}/{id}.md", records::VERDICTS_FOLDER, agent.name)
}

fn label(agent: &Agent) -> String {
    format!("spec/{}/{}", records::VERDICTS_FOLDER, agent.name)
}

/// Loads one agent's verdicts. Used by `list` and by `question`, which needs
/// to know a verdict exists.
pub(crate) fn load(
    project: &Project,
    agent: &Agent,
) -> Result<records::Loaded<Verdict>, fsio::FsError> {
    let mut loaded = records::load_folder::<Verdict>(
        &folder(project, agent),
        &label(agent),
        IdKind::Verdict,
        id_of,
        MAX_VERDICTS,
    )?;
    // A loaded file must also be a well-formed verdict for this agent's folder.
    let mut keep = Vec::new();
    for verdict in loaded.records {
        match verdict.problem() {
            None if verdict.agent == agent.name => keep.push(verdict),
            None => loaded.problems.push(format!(
                "{}/{}.md: this verdict belongs to another agent",
                label(agent),
                verdict.id
            )),
            Some(problem) => {
                loaded
                    .problems
                    .push(format!("{}/{}.md: {problem}", label(agent), verdict.id))
            }
        }
    }
    loaded.records = keep;
    Ok(loaded)
}

/// Reads one verdict of one agent, or `None` when it isn't there.
pub(crate) fn find(
    project: &Project,
    agent: &Agent,
    id: &str,
) -> Result<Option<Verdict>, RecordError> {
    let found = records::find_in::<Verdict>(&folder(project, agent), id, IdKind::Verdict, id_of)?;
    match found {
        Some(verdict) => match verdict.problem() {
            None if verdict.agent == agent.name => Ok(Some(verdict)),
            None => invalid(format!("'{id}' belongs to another agent")),
            Some(problem) => invalid(format!("'{id}' can't be used: {problem}")),
        },
        None => Ok(None),
    }
}

/// What the skill hands `verdict add`: the parts of a verdict the agent knows.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Draft {
    /// Defaults to the agent's first counterpart.
    #[serde(default)]
    counterpart: Option<String>,
    answer: String,
    result: String,
    #[serde(default)]
    checked: Vec<Cited>,
    justification: String,
}

/// What `add` did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Added {
    pub id: String,
    /// The file to stage with the next save, relative to the story folder.
    pub path: String,
    /// True when the same verdict was already recorded, so nothing was written
    /// and the existing one is returned.
    pub existing: bool,
}

/// Records one verdict for `agent_name` from a JSON draft the agent returned.
/// `created` is the time to record, in the form `2026-11-16T17:44`.
pub fn add(
    project: &Project,
    agent_name: &str,
    draft: &str,
    created: &str,
) -> Result<Added, RecordError> {
    let Some(agent) = agents::find(agent_name) else {
        return invalid(format!(
            "'{agent_name}' isn't an agent that records verdicts (use {})",
            agents::names().join(" or ")
        ));
    };
    let draft: Draft = serde_json::from_str(draft)
        .map_err(|e| RecordError::Invalid(format!("the verdict file isn't usable: {e}")))?;
    let counterpart = match draft.counterpart {
        Some(c) => c,
        None => agent
            .counterparts
            .first()
            .map_or_else(String::new, |c| (*c).to_string()),
    };
    let verdict = Verdict {
        // Replaced below; a well-formed placeholder so `problem` checks the rest.
        id: ids::new_id(IdKind::Verdict).map_err(|e| RecordError::Invalid(e.to_string()))?,
        kind: Kind::Verdict,
        agent: agent.name.to_string(),
        counterpart,
        answer: draft.answer,
        result: draft.result,
        checked: draft.checked,
        justification: records::normalized(&draft.justification),
        created: created.to_string(),
    };
    if let Some(problem) = verdict.problem() {
        return invalid(problem);
    }

    // The answer must be one that was saved. The ID is well formed (checked
    // above), so it is safe to build the path from.
    let archived = project
        .harness_dir()
        .join(COMMITTED_DIR)
        .join(format!("{}.json", verdict.answer));
    match fsio::read_regular_capped(&archived, 1) {
        Ok(None) => return invalid(format!("there's no saved answer '{}'", verdict.answer)),
        // A link or special file isn't a saved answer.
        Ok(Some(Err(fsio::Refused::NotARegularFile))) => {
            return invalid(format!("there's no saved answer '{}'", verdict.answer));
        }
        // Present: the answer is bigger than the one byte read here, which is fine.
        Ok(Some(_)) => {}
        Err(e) => return Err(e.into()),
    }

    // Every cited item must be in the story now, and the revision seen can't
    // be one the story hasn't reached.
    let loaded = spec::load(&project.spec_dir())?;
    let current = ops::revisions(&loaded.spec);
    for cited in &verdict.checked {
        match current.get(&cited.id) {
            None => return invalid(format!("'{}' isn't in this story", cited.id)),
            Some(now) if cited.revision > *now => {
                return invalid(format!(
                    "'{}' is at revision {now}, so revision {} can't have been seen",
                    cited.id, cited.revision
                ));
            }
            Some(_) => {}
        }
    }

    let existing = load(project, agent)?;
    if let Some(same) = existing.records.iter().find(|v| {
        v.counterpart == verdict.counterpart
            && v.answer == verdict.answer
            && v.result == verdict.result
            && v.checked == verdict.checked
            && v.justification == verdict.justification
    }) {
        return Ok(Added {
            id: same.id.clone(),
            path: relative_path(agent, &same.id),
            existing: true,
        });
    }
    if existing.records.len() >= MAX_VERDICTS {
        return invalid(format!(
            "{} already has {MAX_VERDICTS} verdicts, which is the most it keeps",
            agent.name
        ));
    }

    let dir = folder(project, agent);
    let id = records::fresh_id(IdKind::Verdict, |id| {
        existing.records.iter().any(|v| v.id == id)
            || std::fs::symlink_metadata(dir.join(format!("{id}.md"))).is_ok()
    })?;
    let verdict = Verdict { id, ..verdict };
    let text = records::render(&verdict)?;
    project.ensure_dir(&dir)?;
    project.write_atomic(&dir.join(format!("{}.md", verdict.id)), &text)?;
    Ok(Added {
        path: relative_path(agent, &verdict.id),
        id: verdict.id,
        existing: false,
    })
}

/// One verdict as `list` returns it.
#[derive(Debug, Clone, PartialEq)]
pub struct Listed {
    pub verdict: Verdict,
    /// True when an item it cited has changed (or is gone) since. Computed on
    /// read; never stored.
    pub stale: bool,
    /// The revision each cited item has now (`None` when it is gone), in the
    /// order of `verdict.checked`.
    pub current: Vec<Option<u32>>,
}

/// What `list` found.
#[derive(Debug, Clone, PartialEq)]
pub struct Listing {
    pub verdicts: Vec<Listed>,
    /// A note for every file that couldn't be used.
    pub problems: Vec<String>,
}

/// True when any item the verdict cited is at a different revision now.
pub(crate) fn is_stale(verdict: &Verdict, current: &BTreeMap<String, u32>) -> bool {
    verdict
        .checked
        .iter()
        .any(|c| current.get(&c.id) != Some(&c.revision))
}

/// The verdicts on record, oldest first, optionally only one agent's or only
/// those about one saved answer. Read-only; staleness is computed here.
pub fn list(
    project: &Project,
    agent: Option<&str>,
    answer: Option<&str>,
) -> Result<Listing, RecordError> {
    let agents: Vec<&Agent> = match agent {
        Some(name) => match agents::find(name) {
            Some(a) => vec![a],
            None => {
                return invalid(format!(
                    "'{name}' isn't an agent that records verdicts (use {})",
                    agents::names().join(" or ")
                ));
            }
        },
        None => agents::AGENTS.iter().collect(),
    };
    if let Some(answer) = answer
        && !ids::is_kind(answer, IdKind::Proposal)
    {
        return invalid(format!("'{answer}' isn't a saved answer's ID"));
    }
    let loaded = spec::load(&project.spec_dir())?;
    let current = ops::revisions(&loaded.spec);
    let mut listing = Listing {
        verdicts: Vec::new(),
        problems: Vec::new(),
    };
    for agent in agents {
        let found = load(project, agent)?;
        listing.problems.extend(found.problems);
        for verdict in found.records {
            if answer.is_some_and(|a| verdict.answer != a) {
                continue;
            }
            listing.verdicts.push(Listed {
                stale: is_stale(&verdict, &current),
                current: verdict
                    .checked
                    .iter()
                    .map(|c| current.get(&c.id).copied())
                    .collect(),
                verdict,
            });
        }
    }
    listing.verdicts.sort_by(|a, b| {
        (&a.verdict.created, &a.verdict.id).cmp(&(&b.verdict.created, &b.verdict.id))
    });
    Ok(listing)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::records::testkit::{ANSWER, DAD, JONAH, MARA, story, verdict_draft};

    const NOW: &str = "2026-11-16T17:44";

    fn refusal(r: Result<Added, RecordError>) -> String {
        match r {
            Err(RecordError::Invalid(m)) => m,
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    fn fail(why: &str) -> String {
        verdict_draft("contradiction_reported", &[(MARA, 2)], why)
    }

    #[test]
    fn a_pass_and_a_fail_become_files_in_the_agents_own_folder() {
        let (_dir, project) = story();
        // The harbor test story carries one problem of its own; records add none.
        let baseline = spec::load(&project.spec_dir()).unwrap().problems;
        let pass = add(
            &project,
            "continuity",
            &verdict_draft("pass", &[(MARA, 2), (JONAH, 1)], "  Nothing  conflicts.\n"),
            NOW,
        )
        .unwrap();
        assert!(!pass.existing);
        assert!(ids::is_kind(&pass.id, IdKind::Verdict));
        assert_eq!(
            pass.path,
            format!("spec/verdicts/continuity/{}.md", pass.id)
        );
        let text = std::fs::read_to_string(project.root().join(&pass.path)).unwrap();
        assert!(text.starts_with("---\n") && text.contains("justification: Nothing conflicts."));

        let failed = add(&project, "continuity", &fail("Mara's eyes differ."), NOW).unwrap();
        assert_ne!(failed.id, pass.id);

        // The story still loads with no problems: records are not nodes.
        let loaded = spec::load(&project.spec_dir()).unwrap();
        assert_eq!(loaded.problems, baseline);
        // Nothing was written anywhere else under the story.
        assert!(!project.spec_dir().join("queue").exists());
    }

    #[test]
    fn the_same_verdict_twice_is_recorded_once() {
        let (_dir, project) = story();
        let first = add(&project, "continuity", &fail("Same words."), NOW).unwrap();
        let again = add(
            &project,
            "continuity",
            &fail("Same   words."),
            "2026-11-16T18:00",
        )
        .unwrap();
        assert!(again.existing);
        assert_eq!(again.id, first.id);
        assert_eq!(list(&project, None, None).unwrap().verdicts.len(), 1);
    }

    #[test]
    fn the_result_has_to_be_on_the_agents_list_and_a_pass_needs_evidence() {
        let (_dir, project) = story();
        let m = refusal(add(
            &project,
            "continuity",
            &verdict_draft("made_up_failure", &[(MARA, 2)], "x"),
            NOW,
        ));
        assert!(
            m.contains("isn't a result continuity may give") && m.contains("pass"),
            "{m}"
        );
        for (result, cited) in [
            ("pass", vec![]),
            ("contradiction_reported", vec![]),
            ("writer_kept_inconsistency", vec![]),
        ] {
            let m = refusal(add(
                &project,
                "continuity",
                &verdict_draft(result, &cited, "x"),
                NOW,
            ));
            assert!(
                m.contains("at least one saved item as evidence"),
                "{result}: {m}"
            );
        }
        // The one fail that rests on nothing cites nothing.
        assert!(
            add(
                &project,
                "continuity",
                &verdict_draft("nothing_saved_to_check", &[], "No scenes yet."),
                NOW
            )
            .is_ok()
        );
        let m = refusal(add(
            &project,
            "continuity",
            &verdict_draft("nothing_saved_to_check", &[(MARA, 2)], "x"),
            NOW,
        ));
        assert!(m.contains("cites no items"), "{m}");
    }

    #[test]
    fn only_a_registered_agent_with_the_right_counterpart_may_record() {
        let (_dir, project) = story();
        for agent in ["interviewer", "", "../continuity", "Continuity"] {
            let m = refusal(add(&project, agent, &fail("x"), NOW));
            assert!(
                m.contains("isn't an agent that records verdicts"),
                "{agent}: {m}"
            );
        }
        let draft = serde_json::json!({
            "counterpart": "weaver", "answer": ANSWER, "result": "pass",
            "checked": [{"id": MARA, "revision": 2}], "justification": "x"
        })
        .to_string();
        assert!(
            refusal(add(&project, "continuity", &draft, NOW)).contains("doesn't check 'weaver'")
        );
        assert!(!project.spec_dir().join("verdicts").exists());
    }

    #[test]
    fn every_cited_item_has_to_exist_and_the_revision_has_to_be_real() {
        let (_dir, project) = story();
        let cases = [
            (vec![("p-zzzzzzzz", 1)], "isn't in this story"),
            (
                vec![(MARA, 3)],
                "is at revision 2, so revision 3 can't have been seen",
            ),
            (vec![(MARA, 0)], "no revision 0"),
            (vec![(MARA, 2), (MARA, 1)], "cited twice"),
            (vec![("idea-abcdefgh", 1)], "isn't a node or fact ID"),
            (vec![("../etc", 1)], "isn't a node or fact ID"),
        ];
        for (cited, expect) in cases {
            let m = refusal(add(
                &project,
                "continuity",
                &verdict_draft("contradiction_reported", &cited, "x"),
                NOW,
            ));
            assert!(m.contains(expect), "{cited:?}: {m}");
        }
        // A fact inside a node counts, at its own revision.
        assert!(
            add(
                &project,
                "continuity",
                &verdict_draft("pass", &[("f-eyesharb", 1)], "Eyes agree."),
                NOW
            )
            .is_ok()
        );
        let too_many: Vec<(&str, u32)> = vec![(MARA, 2); records::MAX_PARTS + 1];
        let m = refusal(add(
            &project,
            "continuity",
            &verdict_draft("pass", &too_many, "x"),
            NOW,
        ));
        assert!(m.contains("at most"), "{m}");
    }

    #[test]
    fn the_answer_has_to_be_one_that_was_saved() {
        let (_dir, project) = story();
        let other = serde_json::json!({
            "answer": "pr-zzzzzzzz", "result": "pass",
            "checked": [{"id": MARA, "revision": 2}], "justification": "x"
        })
        .to_string();
        assert!(refusal(add(&project, "continuity", &other, NOW)).contains("no saved answer"));
        let bad_id = other.replace("pr-zzzzzzzz", "../committed/x");
        assert!(
            refusal(add(&project, "continuity", &bad_id, NOW))
                .contains("isn't a saved answer's ID")
        );
        #[cfg(unix)]
        {
            let committed = project.harness_dir().join(crate::commit::COMMITTED_DIR);
            let target = committed.join(format!("{ANSWER}.json"));
            let link = committed.join("pr-symkedaa.json");
            std::os::unix::fs::symlink(&target, &link).unwrap();
            let linked = other.replace("pr-zzzzzzzz", "pr-symkedaa");
            assert!(refusal(add(&project, "continuity", &linked, NOW)).contains("no saved answer"));
        }
    }

    #[test]
    fn the_justification_is_short_plain_and_counted_in_characters() {
        let (_dir, project) = story();
        let at = |why: &str| add(&project, "continuity", &fail(why), NOW);
        assert!(at(&"é".repeat(JUSTIFICATION_MAX_CHARS)).is_ok());
        assert!(refusal(at(&"é".repeat(JUSTIFICATION_MAX_CHARS + 1))).contains("400"));
        assert!(refusal(at("  \n ")).contains("needs a justification"));
    }

    #[test]
    fn a_draft_can_carry_only_what_a_verdict_holds() {
        let (_dir, project) = story();
        for bad in [
            "not json".to_string(),
            "[]".to_string(),
            r#"{"answer": "pr-abcdefgh"}"#.to_string(),
            serde_json::json!({
                "answer": ANSWER, "result": "pass", "justification": "x",
                "checked": [{"id": MARA, "revision": 2}], "id": "verdict-abcdefgh"
            })
            .to_string(),
            serde_json::json!({
                "answer": ANSWER, "result": "pass", "justification": "x",
                "checked": [{"id": MARA, "revision": 2, "note": "n"}]
            })
            .to_string(),
        ] {
            assert!(
                refusal(add(&project, "continuity", &bad, NOW)).contains("isn't usable"),
                "{bad}"
            );
        }
        assert!(
            refusal(add(&project, "continuity", &fail("x"), "yesterday"))
                .contains("isn't in the form")
        );
    }

    #[test]
    fn list_marks_a_verdict_stale_when_something_it_cited_changes() {
        let (_dir, project) = story();
        let kept = add(
            &project,
            "continuity",
            &verdict_draft("pass", &[(JONAH, 1)], "Jonah fits."),
            NOW,
        )
        .unwrap();
        let moved = add(
            &project,
            "continuity",
            &fail("Mara's eyes."),
            "2026-11-16T17:45",
        )
        .unwrap();
        let before = list(&project, None, None).unwrap();
        assert_eq!(before.verdicts.len(), 2);
        assert!(before.verdicts.iter().all(|l| !l.stale));
        assert_eq!(
            before.verdicts.first().map(|l| l.verdict.id.clone()),
            Some(kept.id.clone())
        );

        // Mara changes after the verdict.
        let path = project.spec_dir().join("people/p-maraharb.md");
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(&path, text.replacen("revision: 2\n", "revision: 3\n", 1)).unwrap();
        let after = list(&project, None, None).unwrap();
        let stale: Vec<_> = after
            .verdicts
            .iter()
            .map(|l| (l.verdict.id.clone(), l.stale))
            .collect();
        assert_eq!(stale, [(kept.id, false), (moved.id.clone(), true)]);
        let listed = after
            .verdicts
            .iter()
            .find(|l| l.verdict.id == moved.id)
            .unwrap();
        assert_eq!(listed.current, [Some(3)]);

        // And it is only computed: the file on disk is untouched by listing.
        let file = project.root().join(&moved.path);
        assert!(
            std::fs::read_to_string(file)
                .unwrap()
                .contains("revision: 2")
        );

        // A cited item that is gone makes it stale too.
        std::fs::remove_file(project.spec_dir().join("people/p-jnahharb.md")).unwrap();
        let gone = list(&project, None, None).unwrap();
        assert!(gone.verdicts.iter().all(|l| l.stale));
    }

    #[test]
    fn list_filters_by_answer_and_refuses_unknown_filters() {
        let (_dir, project) = story();
        add(&project, "continuity", &fail("x"), NOW).unwrap();
        assert_eq!(
            list(&project, Some("continuity"), Some(ANSWER))
                .unwrap()
                .verdicts
                .len(),
            1
        );
        assert_eq!(
            list(&project, None, Some("pr-zzzzzzzz"))
                .unwrap()
                .verdicts
                .len(),
            0
        );
        assert!(matches!(
            list(&project, Some("nobody"), None),
            Err(RecordError::Invalid(_))
        ));
        assert!(matches!(
            list(&project, None, Some("../x")),
            Err(RecordError::Invalid(_))
        ));
    }

    #[test]
    fn a_damaged_verdict_file_is_reported_and_never_trusted() {
        let (_dir, project) = story();
        let added = add(&project, "continuity", &fail("x"), NOW).unwrap();
        let folder = project.spec_dir().join("verdicts/continuity");
        std::fs::write(
            folder.join("verdict-aaaaaaaa.md"),
            "---\nresult: pass\n---\n",
        )
        .unwrap();
        // A well-formed verdict that claims another agent, or a result off the list.
        let good = std::fs::read_to_string(project.root().join(&added.path)).unwrap();
        std::fs::write(
            folder.join("verdict-bbbbbbbb.md"),
            good.replace(&added.id, "verdict-bbbbbbbb")
                .replace("continuity", "weaver"),
        )
        .unwrap();
        std::fs::write(
            folder.join("verdict-cccccccc.md"),
            good.replace(&added.id, "verdict-cccccccc")
                .replace("contradiction_reported", "invented"),
        )
        .unwrap();
        let listing = list(&project, None, None).unwrap();
        assert_eq!(listing.verdicts.len(), 1);
        assert_eq!(listing.problems.len(), 3, "{:#?}", listing.problems);
        assert!(parse(&good).is_ok());
        assert!(parse(&format!("{good}\nmore")).is_err());
    }

    #[test]
    fn a_linked_folder_stops_a_write_and_changes_nothing() {
        #[cfg(unix)]
        {
            let (dir, project) = story();
            let elsewhere = dir.path().join("elsewhere");
            std::fs::create_dir_all(&elsewhere).unwrap();
            std::fs::create_dir_all(project.spec_dir().join("verdicts")).unwrap();
            std::os::unix::fs::symlink(&elsewhere, project.spec_dir().join("verdicts/continuity"))
                .unwrap();
            assert!(matches!(
                add(&project, "continuity", &fail("x"), NOW),
                Err(RecordError::Fs(_))
            ));
            assert_eq!(std::fs::read_dir(&elsewhere).unwrap().count(), 0);
        }
    }

    #[test]
    fn the_unused_dad_fixture_id_is_a_real_person() {
        // Keeps the shared kit honest: its IDs are in the harbor story.
        let (_dir, project) = story();
        let revisions = ops::revisions(&spec::load(&project.spec_dir()).unwrap().spec);
        for id in [MARA, JONAH, DAD] {
            assert!(revisions.contains_key(id), "{id}");
        }
    }
}
