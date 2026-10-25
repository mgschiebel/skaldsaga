//! The writer's decisions about the process, stored one file per decision.
//!
//! Layout (HANDOFF, Step 9 planning): `.harness/decisions/<kind>/<id>.json`,
//! committed to git like `.harness/committed/`. One file per decision means
//! adding one never rewrites another, a diff shows exactly one decision, and
//! branches or sessions don't conflict. Old ones are deleted cleanly, with git
//! as the archive.
//!
//! Every kind shares one envelope: `format` (for later migrations), `kind`,
//! `id`, the writer's own words, what the decision rests on (node and fact IDs
//! with their revisions), `status` and `date`. The only kind built so far is
//! `finding_ack`: the writer saying a finding from `complete` is fine as it
//! is, which carries the finding it answers.
//!
//! Only the writer creates decisions, and the program writes them on the
//! writer's behalf. Nothing here reads a decision as an instruction.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::fsio::{self, FsError, Refused};
use crate::ids::{self, IdKind};
use crate::project::Project;

/// Bumped when the envelope changes, so later versions can migrate files.
pub const FORMAT: u32 = 1;
/// Folder under `.harness/` that holds the decisions.
pub const DECISIONS_DIR: &str = "decisions";
/// The kind (and folder name) of an acknowledged finding.
pub const KIND_FINDING_ACK: &str = "finding_ack";
/// The only scope there is until editions exist: what is true of the story.
pub const SCOPE_CANON: &str = "canon";
/// Decision files are small; anything bigger isn't one of ours.
pub const MAX_FILE_BYTES: u64 = 64 * 1024;
/// The longest answer kept, in characters.
pub const MAX_WORDS_CHARS: usize = 2000;

/// Whether a decision still stands. Withdrawn and resolved decisions are
/// deleted, not marked, so every stored decision is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Active,
}

/// Which finding an acknowledgment answers. Check codes carry a version, so a
/// changed check never matches an old acknowledgment; the edition scope is
/// part of the key so reader checks can run per edition later.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FindingKey {
    pub code: String,
    pub version: u32,
    pub scope: String,
    /// The IDs the finding rests on, sorted.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ids: Vec<String>,
    /// Tells apart findings of one code on the same IDs (a movement, a stage).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// One stored decision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub format: u32,
    pub kind: String,
    pub id: String,
    /// The writer's own words.
    pub words: String,
    /// Node and fact IDs with the revisions they had when the writer decided.
    pub rests_on: BTreeMap<String, u32>,
    pub status: Status,
    /// `YYYY-MM-DD`.
    pub date: String,
    /// The finding this answers (`finding_ack` only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finding: Option<FindingKey>,
}

/// A decision and where it lives.
#[derive(Debug, Clone, PartialEq)]
pub struct Stored {
    pub decision: Decision,
    /// Path from the project root, with forward slashes, for git.
    pub path: String,
}

/// A file in the decisions folder that couldn't be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreProblem {
    pub path: String,
    pub message: String,
}

/// Everything in the decisions folder.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Store {
    /// Sorted by ID.
    pub decisions: Vec<Stored>,
    pub problems: Vec<StoreProblem>,
}

/// Why a decision couldn't be stored.
#[derive(Debug)]
pub enum DecisionError {
    Fs(FsError),
    /// The input was unusable; the message is plain language.
    Invalid(String),
}

impl std::fmt::Display for DecisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecisionError::Fs(e) => e.fmt(f),
            DecisionError::Invalid(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for DecisionError {}

impl From<FsError> for DecisionError {
    fn from(e: FsError) -> Self {
        DecisionError::Fs(e)
    }
}

fn kind_dir(project: &Project, kind: &str) -> PathBuf {
    project.harness_dir().join(DECISIONS_DIR).join(kind)
}

fn relative_path(kind: &str, id: &str) -> String {
    format!(".harness/{DECISIONS_DIR}/{kind}/{id}.json")
}

/// Loads the whole store. A bad file is reported and skipped, never fatal;
/// symlinks and special files are refused, and nothing is read that is over
/// the size cap.
pub fn load(project: &Project) -> Result<Store, FsError> {
    let root = project.harness_dir().join(DECISIONS_DIR);
    let mut store = Store::default();
    for kind_entry in fsio::list_dir(&root)? {
        let rel = format!(".harness/{DECISIONS_DIR}/{}", kind_entry.name);
        if kind_entry.name.starts_with('.') {
            continue;
        }
        if !kind_entry.is_dir || kind_entry.name != KIND_FINDING_ACK {
            store.problems.push(StoreProblem {
                path: rel,
                message: "Skaldsaga doesn't use this here".into(),
            });
            continue;
        }
        let dir = root.join(&kind_entry.name);
        for file in fsio::list_dir(&dir)? {
            if file.name.starts_with('.') {
                continue;
            }
            let path = format!("{rel}/{}", file.name);
            let mut problem = |message: &str| {
                store.problems.push(StoreProblem {
                    path: path.clone(),
                    message: message.to_string(),
                })
            };
            let Some(stem) = file.name.strip_suffix(".json") else {
                problem("only .json decision files belong here");
                continue;
            };
            if !ids::is_kind(stem, IdKind::WriterDecision) {
                problem("the file name isn't a decision ID");
                continue;
            }
            let text = match fsio::read_regular_capped(&dir.join(&file.name), MAX_FILE_BYTES)? {
                None => continue,
                Some(Ok(text)) => text,
                Some(Err(Refused::NotARegularFile)) => {
                    problem("not a regular file, so it was left alone");
                    continue;
                }
                Some(Err(Refused::TooBig)) => {
                    problem("too big to be a decision file");
                    continue;
                }
            };
            match serde_json::from_str::<Decision>(&text) {
                Err(e) => problem(&format!("couldn't read it: {e}")),
                Ok(d) if d.id != stem => problem("the file name should match the decision's ID"),
                Ok(d) if d.kind != kind_entry.name => {
                    problem("the decision's kind doesn't match its folder")
                }
                Ok(d) if d.format != FORMAT => {
                    problem("a different format than this version reads")
                }
                Ok(d) if !valid_key(&d) => problem("its finding or IDs are malformed"),
                Ok(d) => store.decisions.push(Stored {
                    decision: d,
                    path: path.clone(),
                }),
            }
        }
    }
    store
        .decisions
        .sort_by(|a, b| a.decision.id.cmp(&b.decision.id));
    Ok(store)
}

/// Every ID in a stored decision is well formed, and a `finding_ack` has its finding.
fn valid_key(d: &Decision) -> bool {
    let well_formed = |id: &String| ids::kind_of(id).is_some();
    let finding_ok = match (&d.finding, d.kind.as_str()) {
        (Some(f), KIND_FINDING_ACK) => f.ids.iter().all(well_formed),
        (None, KIND_FINDING_ACK) => false,
        (_, _) => true,
    };
    finding_ok && d.rests_on.keys().all(well_formed)
}

/// What storing an acknowledgment did.
#[derive(Debug, Clone, PartialEq)]
pub struct Acknowledged {
    pub decision: Decision,
    /// Path to hand to `git add`.
    pub path: String,
    /// True when this renewed an earlier acknowledgment of the same finding.
    pub renewed: bool,
}

/// Records that the writer accepts a finding as it is. If the same finding
/// was acknowledged before (and has since resurfaced), that decision is
/// renewed in place: same ID, new words, current revisions, new date.
pub fn acknowledge(
    project: &Project,
    key: FindingKey,
    words: &str,
    rests_on: BTreeMap<String, u32>,
    date: &str,
) -> Result<Acknowledged, DecisionError> {
    let words = words.trim();
    if words.is_empty() {
        return Err(DecisionError::Invalid(
            "the writer's own words are needed to record this".into(),
        ));
    }
    if words.chars().count() > MAX_WORDS_CHARS {
        return Err(DecisionError::Invalid(format!(
            "the writer's words are over {MAX_WORDS_CHARS} characters"
        )));
    }
    if key.ids.iter().any(|i| ids::kind_of(i).is_none()) {
        return Err(DecisionError::Invalid(
            "the finding names something that isn't a valid ID".into(),
        ));
    }
    let existing = load(project)?
        .decisions
        .into_iter()
        .find(|s| s.decision.finding.as_ref() == Some(&key));
    let (id, renewed) = match existing {
        Some(s) => (s.decision.id, true),
        None => (
            ids::new_id(IdKind::WriterDecision)
                .map_err(|e| DecisionError::Invalid(e.to_string()))?,
            false,
        ),
    };
    let decision = Decision {
        format: FORMAT,
        kind: KIND_FINDING_ACK.to_string(),
        id: id.clone(),
        words: words.to_string(),
        rests_on,
        status: Status::Active,
        date: date.to_string(),
        finding: Some(key),
    };
    let mut text = serde_json::to_string_pretty(&decision)
        .map_err(|e| DecisionError::Invalid(e.to_string()))?;
    text.push('\n');
    let path = kind_dir(project, KIND_FINDING_ACK).join(format!("{id}.json"));
    project.write_atomic(&path, &text)?;
    Ok(Acknowledged {
        path: relative_path(KIND_FINDING_ACK, &id),
        decision,
        renewed,
    })
}

/// Deletes a decision (the writer withdraws it, or its finding is resolved).
/// Returns the path to hand to `git rm`, or `None` if there was no such decision.
pub fn remove(project: &Project, id: &str) -> Result<Option<String>, DecisionError> {
    if !ids::is_kind(id, IdKind::WriterDecision) {
        return Err(DecisionError::Invalid(format!(
            "'{id}' isn't a valid decision ID"
        )));
    }
    let path = kind_dir(project, KIND_FINDING_ACK).join(format!("{id}.json"));
    Ok(project
        .remove_optional(&path)?
        .then(|| relative_path(KIND_FINDING_ACK, id)))
}

/// Today's date (UTC) as `YYYY-MM-DD`, from seconds since the Unix epoch.
pub fn date_from_unix(secs: u64) -> String {
    // secs / 86_400 is at most about 2e14, which always fits an i64.
    let z = i64::try_from(secs / 86_400).unwrap_or_default() + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { year + 1 } else { year };
    format!("{year:04}-{month:02}-{day:02}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    fn project() -> (TempDir, Project) {
        let dir = TempDir::new();
        let project = Project::new(dir.path());
        std::fs::create_dir_all(project.harness_dir()).unwrap();
        (dir, project)
    }

    fn key(detail: Option<&str>) -> FindingKey {
        FindingKey {
            code: "unearned_twist".into(),
            version: 1,
            scope: SCOPE_CANON.into(),
            ids: vec!["f-aaaaaaaa".into()],
            detail: detail.map(str::to_string),
        }
    }

    fn rests(rev: u32) -> BTreeMap<String, u32> {
        BTreeMap::from([("f-aaaaaaaa".to_string(), rev)])
    }

    #[test]
    fn dates_come_from_the_epoch() {
        assert_eq!(date_from_unix(0), "1970-01-01");
        assert_eq!(date_from_unix(951_782_400), "2000-02-29");
        assert_eq!(date_from_unix(1_000_000_000), "2001-09-09");
        assert_eq!(date_from_unix(1_792_000_000), "2026-10-14");
    }

    #[test]
    fn an_acknowledgment_is_one_file_that_loads_back() {
        let (_dir, project) = project();
        let a = acknowledge(
            &project,
            key(None),
            "  It's fine.  ",
            rests(3),
            "2026-10-24",
        )
        .unwrap();
        assert!(!a.renewed);
        assert!(a.path.starts_with(".harness/decisions/finding_ack/wd-"));
        assert_eq!(a.decision.words, "It's fine.");

        let store = load(&project).unwrap();
        assert_eq!(store.problems, vec![]);
        assert_eq!(store.decisions.len(), 1);
        assert_eq!(store.decisions[0].decision, a.decision);
        assert_eq!(store.decisions[0].path, a.path);
    }

    #[test]
    fn acknowledging_again_renews_the_same_file() {
        let (_dir, project) = project();
        let first = acknowledge(&project, key(None), "ok", rests(1), "2026-10-24").unwrap();
        let again = acknowledge(&project, key(None), "still ok", rests(2), "2026-10-25").unwrap();
        assert!(again.renewed);
        assert_eq!(again.decision.id, first.decision.id);
        let other = acknowledge(
            &project,
            key(Some("x")),
            "different",
            rests(2),
            "2026-10-25",
        )
        .unwrap();
        assert_ne!(other.decision.id, first.decision.id);

        let store = load(&project).unwrap();
        assert_eq!(store.decisions.len(), 2);
        let renewed = store
            .decisions
            .iter()
            .find(|s| s.decision.id == first.decision.id)
            .unwrap();
        assert_eq!(renewed.decision.words, "still ok");
        assert_eq!(renewed.decision.rests_on, rests(2));
    }

    #[test]
    fn words_are_required_and_capped() {
        let (_dir, project) = project();
        for words in ["", "   \n"] {
            assert!(matches!(
                acknowledge(&project, key(None), words, rests(1), "2026-10-24"),
                Err(DecisionError::Invalid(_))
            ));
        }
        let long = "x".repeat(MAX_WORDS_CHARS + 1);
        assert!(matches!(
            acknowledge(&project, key(None), &long, rests(1), "2026-10-24"),
            Err(DecisionError::Invalid(_))
        ));
        let mut bad = key(None);
        bad.ids = vec!["../escape".into()];
        assert!(matches!(
            acknowledge(&project, bad, "ok", rests(1), "2026-10-24"),
            Err(DecisionError::Invalid(_))
        ));
        assert_eq!(load(&project).unwrap().decisions.len(), 0);
    }

    #[test]
    fn removing_checks_the_id_and_deletes_the_file() {
        let (_dir, project) = project();
        let a = acknowledge(&project, key(None), "ok", rests(1), "2026-10-24").unwrap();
        assert!(matches!(
            remove(&project, "../../settings"),
            Err(DecisionError::Invalid(_))
        ));
        assert_eq!(
            remove(&project, &a.decision.id).unwrap().as_deref(),
            Some(a.path.as_str())
        );
        assert_eq!(remove(&project, &a.decision.id).unwrap(), None);
        assert!(load(&project).unwrap().decisions.is_empty());
    }

    #[test]
    fn bad_files_are_reported_and_skipped() {
        let (_dir, project) = project();
        let good = acknowledge(&project, key(None), "ok", rests(1), "2026-10-24").unwrap();
        let dir = kind_dir(&project, KIND_FINDING_ACK);
        std::fs::write(dir.join("notes.txt"), "hi").unwrap();
        std::fs::write(dir.join("wd-badbadba.json"), "{not json").unwrap();
        // Right shape, but the file name doesn't match the ID inside.
        let mut renamed = good.decision.clone();
        renamed.id = "wd-zzzzzzzz".into();
        std::fs::write(
            dir.join("wd-yyyyyyyy.json"),
            serde_json::to_string(&renamed).unwrap(),
        )
        .unwrap();
        std::fs::create_dir_all(project.harness_dir().join("decisions/other")).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(
            dir.join(format!("{}.json", good.decision.id)),
            dir.join("wd-sympqrst.json"),
        )
        .unwrap();

        let store = load(&project).unwrap();
        assert_eq!(store.decisions.len(), 1, "{:#?}", store.problems);
        let have = |needle: &str| store.problems.iter().any(|p| p.message.contains(needle));
        assert!(have("only .json"));
        assert!(have("couldn't read it"));
        assert!(have("should match"));
        assert!(have("doesn't use this"));
        #[cfg(unix)]
        assert!(have("not a regular file"));
    }

    #[test]
    fn a_missing_store_is_empty() {
        let (_dir, project) = project();
        assert_eq!(load(&project).unwrap(), Store::default());
    }
}
