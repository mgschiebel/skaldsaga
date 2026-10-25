//! The single committer (HANDOFF, Governance): applies one checked proposal
//! to the spec files, archives the proposal, and rebuilds the index.
//!
//! Commit always re-checks first, against the spec as it is now, so a
//! proposal can never commit on a stale verdict. What it needs depends on
//! the fresh verdict and the lane:
//!
//! - **failed:** never commits; the answer must be revised.
//! - **flagged:** needs the writer's decision, `keep_anyway` or `not_sure`
//!   (HANDOFF, Flagged proposals). "Not sure yet" commits everything it
//!   writes as provisional, and the flag stays open in the archive as an
//!   assumption to confirm.
//! - **strict lane:** needs the housekeeper's clearance (`cleared`), and that
//!   clearance is only as fresh as what the check examined: if anything in
//!   `revisions.checked` changed since, the proposal waits for a fresh pass
//!   (the version check).
//!
//! Optimistic commits are tagged unreviewed; a commit cleared by the
//! housekeeper is tagged cleared. Committed proposals move to
//! `.harness/committed/` with a note of how they were decided (decided in
//! build step 8), so recaps can quote the writer and open assumptions stay
//! findable.
//!
//! The program writes files; the skill makes the git commit, using the
//! paths in the result (as with setup, decided in build step 5).

use std::collections::BTreeSet;
use std::path::Path;

use serde::Serialize;

use crate::check::{self, Report, Status};
use crate::fsio::{self, FsError};
use crate::index;
use crate::json::Value;
use crate::model::{Confidence, NodeBody, Review};
use crate::node_file;
use crate::ops::{self, Applied};
use crate::project::Project;
use crate::proposal::{self, Lane, Proposal};
use crate::registry::{self, ATTRIBUTES_FILE};
use crate::settings::RoutingSettings;
use crate::spec::{self, SEQUENCE_FILE};

/// The writer's choice on a flagged proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    /// Commit as it is; the flags are recorded as acknowledged.
    KeepAnyway,
    /// Commit as provisional; the flags stay open as assumptions to confirm.
    NotSure,
}

impl Decision {
    pub fn as_str(self) -> &'static str {
        match self {
            Decision::KeepAnyway => "keep_anyway",
            Decision::NotSure => "not_sure",
        }
    }

    pub fn parse(s: &str) -> Option<Decision> {
        match s {
            "keep_anyway" => Some(Decision::KeepAnyway),
            "not_sure" => Some(Decision::NotSure),
            _ => None,
        }
    }
}

/// What the commit was asked to take into account.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CommitOptions {
    pub decision: Option<Decision>,
    /// The housekeeper cleared this proposal (needed in the strict lane).
    pub cleared: bool,
}

/// Why a proposal didn't commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HeldReason {
    /// Hard findings: the answer must be revised.
    Failed,
    /// Soft findings: the writer chooses keep anyway or not sure yet.
    NeedsDecision,
    /// Strict lane: waiting for a housekeeper scoped pass.
    WaitingForHousekeeper,
    /// Strict lane: cleared, but something the check examined has changed
    /// since, so the clearance is stale.
    ClearanceStale,
}

impl HeldReason {
    pub fn as_str(self) -> &'static str {
        match self {
            HeldReason::Failed => "failed",
            HeldReason::NeedsDecision => "needs_decision",
            HeldReason::WaitingForHousekeeper => "waiting_for_housekeeper",
            HeldReason::ClearanceStale => "clearance_stale",
        }
    }
}

/// What a commit did.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Committed {
        /// Spec and index files written or removed, relative to the project.
        files: Vec<String>,
        /// Paths the skill adds to git: written spec files and the archived
        /// proposal (never the index, which git ignores).
        add_paths: Vec<String>,
        /// Paths the skill removes from git: deleted node files and the
        /// inbox file (which git may never have seen).
        remove_paths: Vec<String>,
        /// A one-line commit message.
        message: String,
        report: Report,
        decision: Option<Decision>,
    },
    Held {
        reason: HeldReason,
        report: Report,
    },
    /// No proposal with that ID in the inbox.
    NotFound,
    /// The proposal file couldn't be read.
    Unreadable(String),
}

/// Where committed proposals go.
pub const COMMITTED_DIR: &str = "committed";

/// The file the next git snapshot's message is written to (inside
/// `.harness/`, kept out of git).
pub const MESSAGE_FILE: &str = "commit-message.txt";

/// Writes a snapshot message to `.harness/commit-message.txt` and returns its
/// path relative to the story folder. The message can hold the writer's own
/// words, so the skill passes it to git as a file (`git commit -F`) and never
/// puts it inside a command line (SR-32).
pub fn write_message(project: &Project, message: &str) -> Result<String, FsError> {
    crate::settings::ensure_harness_ignored(
        project,
        MESSAGE_FILE,
        "The text of the next git snapshot, which can hold the writer's own words.",
    )?;
    let text = format!("{}\n", message.trim_end());
    fsio::write_atomic(&project.harness_dir().join(MESSAGE_FILE), &text)?;
    Ok(format!("{}/{MESSAGE_FILE}", crate::project::HARNESS_DIR))
}

/// Commits one proposal from the inbox.
pub fn commit(project: &Project, id: &str, options: CommitOptions) -> Result<Outcome, FsError> {
    // The ID becomes part of a path, so only a well-formed one gets that far (SR-20).
    if !crate::ids::is_kind(id, crate::ids::IdKind::Proposal) {
        return Ok(Outcome::NotFound);
    }
    let inbox_path = project.inbox_dir().join(format!("{id}.json"));
    let text = match fsio::read_regular_capped(&inbox_path, fsio::MAX_STORY_FILE_BYTES)? {
        Some(Ok(text)) => text,
        Some(Err(why)) => return Ok(Outcome::Unreadable(fsio::refusal_message(&why).to_string())),
        None => return Ok(Outcome::NotFound),
    };
    let mut proposal = match proposal::parse(&text) {
        Ok(p) if p.id == id => p,
        Ok(p) => {
            return Ok(Outcome::Unreadable(format!(
                "the file name should match the proposal's ID ('{}.json')",
                p.id
            )));
        }
        Err(e) => return Ok(Outcome::Unreadable(e)),
    };

    let spec_dir = project.spec_dir();
    let loaded = spec::load(&spec_dir)?;
    let spec = &loaded.spec;
    let routing = RoutingSettings::load(project)?;

    // The version check: is the clearance still as fresh as what it examined?
    let clearance_fresh = !proposal.revisions.checked.is_empty()
        && proposal
            .revisions
            .checked
            .iter()
            .all(|(key, recorded)| check::current_value(spec, key).as_ref() == Some(recorded));

    let report = check::check(spec, &proposal, &routing);
    let held = match report.status {
        Status::Failed => Some(HeldReason::Failed),
        Status::Flagged if options.decision.is_none() => Some(HeldReason::NeedsDecision),
        _ if report.lane == Lane::Strict && !options.cleared => {
            Some(HeldReason::WaitingForHousekeeper)
        }
        _ if report.lane == Lane::Strict && !clearance_fresh => Some(HeldReason::ClearanceStale),
        _ => None,
    };
    if let Some(reason) = held {
        // Keep the fresh verdict in the inbox file, as check would.
        proposal.check = Some(report.to_value());
        proposal.revisions.checked = report.checked.clone();
        fsio::write_atomic(&inbox_path, &proposal::render(&proposal))?;
        return Ok(Outcome::Held { reason, report });
    }

    // Apply. The check above already applied it once, so this can't fail.
    let mut applied = match ops::apply(spec, &proposal.ops) {
        Ok(a) => a,
        Err(e) => {
            return Ok(Outcome::Unreadable(format!(
                "step {} could no longer be applied: {}",
                e.op + 1,
                e.message
            )));
        }
    };
    let decision = if report.status == Status::Flagged {
        options.decision
    } else {
        None
    };
    label(&mut applied, report.lane, decision);

    let (written, removed) = write_spec(&spec_dir, spec, &applied)?;
    let rebuilt = index::rebuild(&spec_dir)?;
    let mut files: Vec<String> = written.iter().chain(&removed).cloned().collect();
    files.sort();
    files.extend(rebuilt.files.iter().map(|f| format!("spec/{f}")));

    // Archive the proposal with how it was decided.
    let archive_path = project
        .harness_dir()
        .join(COMMITTED_DIR)
        .join(format!("{id}.json"));
    proposal.check = Some(report.to_value());
    proposal.revisions.checked = report.checked.clone();
    let mut archived = serde_json::to_value(&proposal).unwrap_or(Value::Null);
    if let Some(map) = archived.as_object_mut() {
        let mut committed = serde_json::Map::new();
        committed.insert(
            "lane".into(),
            serde_json::to_value(report.lane).unwrap_or_default(),
        );
        committed.insert(
            "decision".into(),
            decision.map_or(Value::Null, |d| serde_json::to_value(d).unwrap_or_default()),
        );
        committed.insert(
            "open_assumption".into(),
            Value::from(decision == Some(Decision::NotSure)),
        );
        committed.insert("cleared".into(), Value::from(options.cleared));
        committed.insert(
            "committed_by".into(),
            format!("skaldsaga {}", crate::VERSION).into(),
        );
        map.insert("committed".into(), Value::Object(committed));
    }
    let mut archive_text = serde_json::to_string_pretty(&archived).unwrap_or_default();
    archive_text.push('\n');
    fsio::write_atomic(&archive_path, &archive_text)?;
    fsio::remove_optional(&inbox_path)?;

    let mut add_paths = written;
    add_paths.push(format!(".harness/{COMMITTED_DIR}/{id}.json"));
    let mut remove_paths = removed;
    remove_paths.push(format!(".harness/inbox/{id}.json"));
    let message = commit_message(&proposal);
    Ok(Outcome::Committed {
        files,
        add_paths,
        remove_paths,
        message,
        report,
        decision,
    })
}

/// A short git message: the writer's own words when there are some.
fn commit_message(proposal: &Proposal) -> String {
    let words = proposal
        .answer
        .as_deref()
        .map(str::trim)
        .filter(|a| !a.is_empty())
        .map(|a| a.lines().next().unwrap_or(a).to_string());
    match words {
        Some(a) => format!("Skaldsaga: {}", shorten(&a, 72)),
        None => format!("Skaldsaga: answer {}", proposal.id),
    }
}

/// Shortens `text` to at most `max` characters, cutting at the last space
/// so no word is split, and marking the cut with "...". A single word longer
/// than the limit is cut where it must be.
fn shorten(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let head: String = text.chars().take(max - 3).collect();
    // Only cut at a space if it leaves a reasonable amount of text.
    let cut = match head.rfind(' ') {
        Some(i) if i >= head.len() / 2 => head.get(..i).unwrap_or(head.as_str()),
        _ => head.as_str(),
    };
    format!("{}...", cut.trim_end_matches([' ', ',', ';', ':']))
}

/// Sets the review and confidence labels on everything the proposal wrote.
fn label(applied: &mut Applied, lane: Lane, decision: Option<Decision>) {
    let written: BTreeSet<String> = applied
        .created
        .iter()
        .chain(&applied.changed)
        .cloned()
        .collect();
    let review = if lane == Lane::Strict {
        Review::Cleared
    } else {
        Review::Unreviewed
    };
    let not_sure = decision == Some(Decision::NotSure);
    let soften = |c: &mut Confidence| {
        if not_sure && *c == Confidence::Confirmed {
            *c = Confidence::Provisional;
        }
    };
    for node in &mut applied.spec.nodes {
        for fact in &mut node.facts {
            if written.contains(&fact.id) {
                fact.review = review;
                soften(&mut fact.confidence);
            }
        }
        if written.contains(&node.id)
            && let NodeBody::Decision(d) = &mut node.body
        {
            d.review = review;
            soften(&mut d.confidence);
        }
    }
}

/// Writes changed node files, removes deleted ones, and rewrites
/// `sequence.md` and `attributes.md` when they changed. Returns the paths
/// written and removed, relative to the project folder.
fn write_spec(
    spec_dir: &Path,
    before: &spec::Spec,
    applied: &Applied,
) -> Result<(Vec<String>, Vec<String>), FsError> {
    let rel = |p: &str| format!("spec/{p}");
    let mut files = Vec::new();
    let mut removed = Vec::new();
    for id in &applied.dirty_nodes {
        let Some(node) = applied.spec.node(id) else {
            continue;
        };
        let path = format!("{}/{id}.md", node.node_type().folder());
        let text = node_file::render_node(node).map_err(|e| FsError::Io {
            path: path.clone(),
            source: std::io::Error::other(e.0),
        })?;
        fsio::write_atomic(&spec_dir.join(&path), &text)?;
        files.push(rel(&path));
    }
    for id in &applied.removed_nodes {
        let Some(node) = before.node(id) else {
            continue;
        };
        let path = format!("{}/{id}.md", node.node_type().folder());
        fsio::remove_optional(&spec_dir.join(&path))?;
        removed.push(rel(&path));
    }
    if applied.sequence_changed {
        fsio::write_atomic(
            &spec_dir.join(SEQUENCE_FILE),
            &spec::render_sequence(&applied.spec.sequence),
        )?;
        files.push(rel(SEQUENCE_FILE));
    }
    if applied.attributes_changed {
        let path = spec_dir.join(ATTRIBUTES_FILE);
        let notes = fsio::read_regular_capped(&path, fsio::MAX_STORY_FILE_BYTES)?
            .and_then(Result::ok)
            .and_then(|t| {
                node_file::split_front_matter(&t)
                    .ok()
                    .map(|(_, n)| n.to_string())
            })
            .unwrap_or_else(|| registry::ATTRIBUTES_NOTES.to_string());
        let text =
            registry::render_attributes_file(&applied.spec.attributes, &notes).map_err(|e| {
                FsError::Io {
                    path: path.display().to_string(),
                    source: std::io::Error::other(e),
                }
            })?;
        fsio::write_atomic(&path, &text)?;
        files.push(rel(ATTRIBUTES_FILE));
    }
    files.sort();
    removed.sort();
    Ok((files, removed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::FactKind;
    use crate::testutil::TempDir;
    use std::path::PathBuf;

    fn copy_dir(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        for entry in std::fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let target = to.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_dir(&entry.path(), &target);
            } else {
                std::fs::copy(entry.path(), target).unwrap();
            }
        }
    }

    /// A project holding the harbor story, without its deliberately bad
    /// attributes file (so committed files can be reloaded cleanly).
    fn project() -> (TempDir, Project) {
        let dir = TempDir::new();
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/specs/harbor");
        copy_dir(&fixture, &dir.path().join("spec"));
        std::fs::remove_file(dir.path().join("spec/attributes.md")).unwrap();
        std::fs::create_dir_all(dir.path().join(".harness/inbox")).unwrap();
        let project = Project::new(dir.path());
        (dir, project)
    }

    fn file(project: &Project, id: &str, agent: &str, ops: &str) {
        let text = format!(
            r#"{{"id": "{id}", "agent": "{agent}", "answer": "Ada is the pilot's only child", "ops": {ops}}}"#
        );
        std::fs::write(project.inbox_dir().join(format!("{id}.json")), text).unwrap();
    }

    fn load(project: &Project) -> spec::Spec {
        let loaded = spec::load(&project.spec_dir()).unwrap();
        assert_eq!(loaded.problems, vec![], "committed files reload cleanly");
        loaded.spec
    }

    fn fact(spec: &spec::Spec, id: &str) -> crate::model::Fact {
        spec.nodes
            .iter()
            .flat_map(|n| &n.facts)
            .find(|f| f.id == id)
            .unwrap()
            .clone()
    }

    #[test]
    fn a_passing_answer_is_written_archived_and_indexed() {
        let (_dir, project) = project();
        file(
            &project,
            "pr-cmtcmtcm",
            "interviewer",
            r#"[{"op": "set", "target": "p-sisharb2#f-chdrharb.statement", "value": "Ada is the pilot's only child"},
                {"op": "set", "target": "p-dadsharb.summary", "value": "Ada's father"}]"#,
        );
        let out = commit(&project, "pr-cmtcmtcm", CommitOptions::default()).unwrap();
        let Outcome::Committed {
            add_paths,
            remove_paths,
            message,
            report,
            decision,
            ..
        } = out
        else {
            panic!("{out:?}")
        };
        assert_eq!(report.status, Status::Passed);
        assert_eq!(decision, None);
        assert_eq!(message, "Skaldsaga: Ada is the pilot's only child");
        assert_eq!(
            add_paths,
            [
                "spec/people/p-dadsharb.md",
                "spec/people/p-sisharb2.md",
                ".harness/committed/pr-cmtcmtcm.json"
            ]
        );
        assert_eq!(remove_paths, [".harness/inbox/pr-cmtcmtcm.json"]);

        let spec = load(&project);
        let f = fact(&spec, "f-chdrharb");
        assert_eq!(f.statement, "Ada is the pilot's only child");
        assert_eq!(f.revision, 2);
        assert_eq!(
            f.review,
            Review::Unreviewed,
            "optimistic commits are unreviewed"
        );
        assert_eq!(spec.node("p-dadsharb").unwrap().revision, 2);

        assert!(!project.inbox_dir().join("pr-cmtcmtcm.json").exists());
        let archived: Value = serde_json::from_str(
            &std::fs::read_to_string(project.harness_dir().join("committed/pr-cmtcmtcm.json"))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(archived["committed"]["lane"], "optimistic");
        assert_eq!(archived["committed"]["open_assumption"], false);
        assert_eq!(archived["check"]["status"], "passed");
        let status =
            std::fs::read_to_string(project.spec_dir().join("_index/status.json")).unwrap();
        assert!(status.contains("Ada's father"), "the index was rebuilt");
    }

    #[test]
    fn a_failed_answer_is_held_and_nothing_changes() {
        let (_dir, project) = project();
        file(
            &project,
            "pr-faiaiaia",
            "interviewer",
            r#"[{"op": "delete", "target": "p-dadsharb"}]"#,
        );
        let before =
            std::fs::read_to_string(project.spec_dir().join("people/p-dadsharb.md")).unwrap();
        let out = commit(&project, "pr-faiaiaia", CommitOptions::default()).unwrap();
        assert!(
            matches!(
                out,
                Outcome::Held {
                    reason: HeldReason::Failed,
                    ..
                }
            ),
            "{out:?}"
        );
        assert_eq!(
            std::fs::read_to_string(project.spec_dir().join("people/p-dadsharb.md")).unwrap(),
            before
        );
        let inbox = std::fs::read_to_string(project.inbox_dir().join("pr-faiaiaia.json")).unwrap();
        assert!(
            inbox.contains("delete_blocked"),
            "the fresh verdict is saved"
        );
        // Even a decision doesn't commit a hard failure.
        let opts = CommitOptions {
            decision: Some(Decision::KeepAnyway),
            cleared: true,
        };
        let out = commit(&project, "pr-faiaiaia", opts).unwrap();
        assert!(matches!(
            out,
            Outcome::Held {
                reason: HeldReason::Failed,
                ..
            }
        ));
    }

    const HINT_AFTER_REVEAL: &str = r#"[{"op": "add_item", "target": "p-jnahharb#f-jnspyhrb.reader",
        "value": {"scene": "s-harbhhhh", "level": "hinted"}},
        {"op": "set", "target": "p-maraharb#f-eyesharb.statement", "value": "Mara's eyes are grey"}]"#;

    #[test]
    fn a_flagged_answer_needs_the_writers_decision() {
        let (_dir, project) = project();
        file(&project, "pr-fagfagfa", "interviewer", HINT_AFTER_REVEAL);
        let out = commit(&project, "pr-fagfagfa", CommitOptions::default()).unwrap();
        assert!(matches!(
            out,
            Outcome::Held {
                reason: HeldReason::NeedsDecision,
                ..
            }
        ));

        let opts = CommitOptions {
            decision: Some(Decision::NotSure),
            cleared: false,
        };
        let out = commit(&project, "pr-fagfagfa", opts).unwrap();
        assert!(
            matches!(
                out,
                Outcome::Committed {
                    decision: Some(Decision::NotSure),
                    ..
                }
            ),
            "{out:?}"
        );
        let spec = load(&project);
        // "Not sure yet" softens confirmed facts it wrote to provisional.
        assert_eq!(
            fact(&spec, "f-eyesharb").confidence,
            Confidence::Provisional
        );
        let archived =
            std::fs::read_to_string(project.harness_dir().join("committed/pr-fagfagfa.json"))
                .unwrap();
        assert!(archived.contains("\"open_assumption\": true"));
        assert!(archived.contains("\"decision\": \"not_sure\""));
    }

    #[test]
    fn keep_anyway_keeps_labels() {
        let (_dir, project) = project();
        file(&project, "pr-kepkepke", "interviewer", HINT_AFTER_REVEAL);
        let opts = CommitOptions {
            decision: Some(Decision::KeepAnyway),
            cleared: false,
        };
        assert!(matches!(
            commit(&project, "pr-kepkepke", opts).unwrap(),
            Outcome::Committed { .. }
        ));
        assert_eq!(
            fact(&load(&project), "f-eyesharb").confidence,
            Confidence::Confirmed
        );
    }

    #[test]
    fn strict_answers_wait_for_a_fresh_clearance() {
        let (_dir, project) = project();
        let ops = r#"[{"op": "set", "target": "p-dadsharb.summary", "value": "Ada's father"}]"#;
        file(&project, "pr-strstrst", "weaver", ops);
        let out = commit(&project, "pr-strstrst", CommitOptions::default()).unwrap();
        assert!(matches!(
            out,
            Outcome::Held {
                reason: HeldReason::WaitingForHousekeeper,
                ..
            }
        ));
        // The held commit saved the verdict and what it examined, so a
        // clearance given now is fresh.
        let cleared = CommitOptions {
            decision: None,
            cleared: true,
        };

        // Something it examined changes before the clearance is used.
        std::fs::write(
            project.spec_dir().join("people/p-dadsharb.md"),
            "---\nid: p-dadsharb\ntype: person\nname: The harbor pilot\nrevision: 2\n---\n",
        )
        .unwrap();
        let out = commit(&project, "pr-strstrst", cleared).unwrap();
        assert!(
            matches!(
                out,
                Outcome::Held {
                    reason: HeldReason::ClearanceStale,
                    ..
                }
            ),
            "{out:?}"
        );

        // That held commit re-recorded the current revisions; now it's fresh.
        let out = commit(&project, "pr-strstrst", cleared).unwrap();
        assert!(matches!(out, Outcome::Committed { .. }), "{out:?}");
        assert_eq!(
            load(&project)
                .node("p-dadsharb")
                .unwrap()
                .summary
                .as_deref(),
            Some("Ada's father")
        );
    }

    #[test]
    fn deletes_reclassifies_and_new_keys_reach_the_files() {
        let (_dir, project) = project();
        file(
            &project,
            "pr-dxtdxtdx",
            "interviewer",
            r#"[{"op": "remove_item", "target": "sequence", "value": "s-harbeeee"},
                {"op": "delete", "target": "s-harbeeee"},
                {"op": "reclassify", "target": "f-arstharb", "kind": "belief", "holder": "reader"},
                {"op": "add_attribute", "attribute": {"key": "owes", "relation": "one_way", "cardinality": "per_target"}}]"#,
        );
        let out = commit(&project, "pr-dxtdxtdx", CommitOptions::default()).unwrap();
        let Outcome::Committed {
            remove_paths,
            add_paths,
            ..
        } = out
        else {
            panic!("{out:?}")
        };
        assert!(remove_paths.contains(&"spec/scenes/s-harbeeee.md".to_string()));
        assert!(add_paths.contains(&"spec/sequence.md".to_string()));
        assert!(add_paths.contains(&"spec/attributes.md".to_string()));
        assert!(!project.spec_dir().join("scenes/s-harbeeee.md").exists());
        let spec = load(&project);
        assert_eq!(spec.sequence.len(), 7);
        assert_eq!(fact(&spec, "f-arstharb").kind, FactKind::Belief);
        assert_eq!(spec.attributes[0].key, "owes");
        let text = std::fs::read_to_string(project.spec_dir().join("attributes.md")).unwrap();
        assert!(text.contains("Skaldsaga keeps this list up to date"));
    }

    #[test]
    fn missing_and_unreadable_proposals() {
        let (_dir, project) = project();
        assert_eq!(
            commit(&project, "pr-nnnnnnnn", CommitOptions::default()).unwrap(),
            Outcome::NotFound
        );
        std::fs::write(project.inbox_dir().join("pr-badbadba.json"), "{").unwrap();
        assert!(matches!(
            commit(&project, "pr-badbadba", CommitOptions::default()).unwrap(),
            Outcome::Unreadable(_)
        ));
    }

    #[test]
    fn a_hostile_message_is_stored_verbatim_in_an_ignored_file() {
        let dir = TempDir::new();
        let project = Project::new(dir.path());
        std::fs::create_dir_all(project.harness_dir()).unwrap();
        let hostile = "Skaldsaga: \"; touch pwned; $(touch pwned) `touch pwned` \\n it's";
        let path = write_message(&project, hostile).unwrap();
        assert_eq!(path, ".harness/commit-message.txt");
        assert_eq!(
            std::fs::read_to_string(dir.path().join(&path)).unwrap(),
            format!("{hostile}\n")
        );
        let ignore = std::fs::read_to_string(project.harness_dir().join(".gitignore")).unwrap();
        assert!(ignore.lines().any(|l| l == MESSAGE_FILE));
        assert!(!dir.path().join("pwned").exists());
    }

    #[test]
    fn long_messages_are_cut_at_a_word_boundary() {
        let text = "Mara Vance is a dock clerk who learns her brother Jonah was arrested after she crosses the checkpoint";
        let short = shorten(text, 72);
        assert!(short.chars().count() <= 72, "{short}");
        assert!(short.ends_with("..."));
        assert!(text.starts_with(short.trim_end_matches("...")));
        let before = short.trim_end_matches("...");
        // The next character in the original is a space: no word was split.
        assert_eq!(
            text.get(before.len()..).and_then(|t| t.chars().next()),
            Some(' ')
        );
        assert_eq!(shorten("short enough", 72), "short enough");
        // One very long word is still cut.
        let long = "x".repeat(100);
        assert_eq!(shorten(&long, 72).chars().count(), 72);
    }
}
