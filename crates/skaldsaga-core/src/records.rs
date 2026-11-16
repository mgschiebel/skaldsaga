//! Shared parts of the records agents leave beside the story (HANDOFF,
//! Planning session 12; `design-agents.md`, "The first build's plan", Build 2).
//!
//! Three kinds of record, each one small file with a checked ID, each in a
//! folder that only one command writes (the one-writer rule, *Decided
//! November 16, 2026*):
//!
//! * a **verdict** (`verdict` module): one agent's judgment of one saved
//!   answer, in `spec/verdicts/<agent>/`;
//! * a **question** (`question` module): something an agent wants the writer
//!   asked, in `spec/queue/<agent>/`;
//! * an **asked** record (`asked` module): the Interviewer's note of which
//!   questions it asked and how each ended, in `spec/queue/interviewer/`.
//!
//! They are not story nodes: the story loader skips both folders, so a record
//! never changes a check, a backlink or the index. A file is `---`, a few
//! lines of YAML and `---`, and nothing after. The rules for anything read or
//! written here are the story-folder rules: IDs are checked before they become
//! paths (SR-20), writes refuse linked folders (SR-21), reads refuse links and
//! are size-capped (SR-23), the folder is never searched outside its own
//! agent's name, and every word in a record is data, never an instruction
//! (SR-30).

use std::fmt;
use std::path::Path;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::fsio::{self, EntryKind, FsError};
use crate::ids::{self, IdKind};
use crate::model::{self, NodeType};
use crate::node_file;
use crate::spec::Spec;

/// The folder under `spec/` that holds verdicts, one subfolder per agent.
pub const VERDICTS_FOLDER: &str = "verdicts";

/// The folder under `spec/` that holds queued questions, one subfolder per
/// agent that raises them, plus the Interviewer's own `interviewer` subfolder.
pub const QUEUE_FOLDER: &str = "queue";

/// The most bytes one record file may have (*Proposed*). Every record is a few
/// short fields, so this is far above any real one.
pub const MAX_RECORD_BYTES: u64 = 16 * 1024;

/// The most story parts or cited items one record may name (*Proposed*).
pub const MAX_PARTS: usize = 24;

/// True for the folders under `spec/` that hold records and not story nodes.
pub fn is_record_folder(name: &str) -> bool {
    name == VERDICTS_FOLDER || name == QUEUE_FOLDER
}

/// Why a record operation didn't go through.
#[derive(Debug)]
pub enum RecordError {
    Fs(FsError),
    /// The request was unusable; the message is plain language.
    Invalid(String),
}

impl fmt::Display for RecordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecordError::Fs(e) => e.fmt(f),
            RecordError::Invalid(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for RecordError {}

impl From<FsError> for RecordError {
    fn from(e: FsError) -> Self {
        RecordError::Fs(e)
    }
}

/// An unusable request, with a plain-language reason.
pub fn invalid<T>(message: impl Into<String>) -> Result<T, RecordError> {
    Err(RecordError::Invalid(message.into()))
}

/// Text as stored: whitespace runs (and line breaks) collapse to one space.
pub fn normalized(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Renders a record as its file: front matter and nothing else.
pub fn render<T: Serialize>(record: &T) -> Result<String, RecordError> {
    let yaml = serde_norway::to_string(record)
        .map_err(|e| RecordError::Invalid(format!("the record couldn't be written: {e}")))?;
    Ok(format!("---\n{yaml}---\n"))
}

/// Parses a record file. Never panics, whatever the text.
pub fn parse<T: DeserializeOwned>(text: &str) -> Result<T, String> {
    let (front, rest) = node_file::split_front_matter(text).map_err(|e| e.0)?;
    if !rest.trim().is_empty() {
        return Err("a record holds nothing after its front matter".into());
    }
    serde_norway::from_str(front).map_err(|e| format!("front matter: {e}"))
}

/// True for a timestamp in the shape the program writes: `2026-11-16T17:44`.
pub fn is_stamp(text: &str) -> bool {
    let bytes: Vec<u8> = text.bytes().collect();
    bytes.len() == 16
        && bytes.iter().enumerate().all(|(i, b)| match i {
            4 | 7 => *b == b'-',
            10 => *b == b'T',
            13 => *b == b':',
            _ => b.is_ascii_digit(),
        })
}

/// True if `id` names something in the story itself: a node of any type but an
/// idea, or a fact inside one.
pub fn is_story_id(id: &str) -> bool {
    match ids::kind_of(id) {
        Some(IdKind::Fact) => true,
        Some(kind) => NodeType::ALL
            .iter()
            .any(|t| *t != NodeType::Idea && t.id_kind() == kind),
        None => false,
    }
}

/// Checks the story parts a question names: each is one of the stage codes or
/// the ID of a node that is in this story, with no repeats.
pub fn check_parts(spec: &Spec, parts: &[String]) -> Result<(), String> {
    if parts.len() > MAX_PARTS {
        return Err(format!("at most {MAX_PARTS} story parts can be named"));
    }
    let mut seen = std::collections::HashSet::new();
    for part in parts {
        if !model::is_idea_touch(part) {
            return Err(format!(
                "'{part}' isn't a story part (use a node ID or one of the stage names)"
            ));
        }
        let is_stage = crate::marker::Topic::parse(part).is_some();
        if !is_stage && spec.node(part).is_none() {
            return Err(format!("'{part}' isn't a part of this story"));
        }
        if !seen.insert(part.as_str()) {
            return Err(format!("'{part}' is listed twice"));
        }
    }
    Ok(())
}

/// A new ID of `kind` that `taken` doesn't claim.
pub fn fresh_id(kind: IdKind, taken: impl Fn(&str) -> bool) -> Result<String, RecordError> {
    for _ in 0..64 {
        let id = ids::new_id(kind).map_err(|e| RecordError::Invalid(e.to_string()))?;
        if !taken(&id) {
            return Ok(id);
        }
    }
    invalid("couldn't find an unused ID")
}

/// What loading a record folder found.
#[derive(Debug)]
pub struct Loaded<T> {
    /// Every record that loaded cleanly, in file-name order.
    pub records: Vec<T>,
    /// A plain-language note for each file that didn't, or that was past the cap.
    pub problems: Vec<String>,
}

/// Loads every record in one agent's folder. A missing folder loads as empty.
/// Links and special files, files that are too big, badly named, badly formed
/// or whose ID doesn't match their file name are reported, never followed or
/// trusted. At most `max` records load; the rest are reported.
pub fn load_folder<T: DeserializeOwned>(
    dir: &Path,
    label: &str,
    kind: IdKind,
    id_of: fn(&T) -> &str,
    max: usize,
) -> Result<Loaded<T>, FsError> {
    let mut out = Loaded {
        records: Vec::new(),
        problems: Vec::new(),
    };
    for entry in fsio::list_dir(dir)? {
        if entry.name.starts_with('.') {
            continue;
        }
        let shown = format!("{label}/{}", entry.name);
        if entry.kind != EntryKind::File {
            out.problems.push(format!(
                "{shown}: {}",
                fsio::refusal_message(&fsio::Refused::NotARegularFile)
            ));
            continue;
        }
        let Some(stem) = entry.name.strip_suffix(".md") else {
            out.problems
                .push(format!("{shown}: only Markdown (.md) records belong here"));
            continue;
        };
        if !ids::is_kind(stem, kind) {
            out.problems.push(format!(
                "{shown}: the file name should be a {kind} ID ('{}-' and 8 letters or digits)",
                kind.prefix()
            ));
            continue;
        }
        if out.records.len() >= max {
            out.problems.push(format!(
                "{label}: more than {max} records; the rest are ignored"
            ));
            break;
        }
        match read_one::<T>(&dir.join(&entry.name), MAX_RECORD_BYTES)? {
            Ok(record) if id_of(&record) == stem => out.records.push(record),
            Ok(_) => out.problems.push(format!(
                "{shown}: the file name should match the record's ID"
            )),
            Err(why) => out.problems.push(format!("{shown}: {why}")),
        }
    }
    Ok(out)
}

/// Reads and parses one record file. A refusal or a parse failure comes back
/// as plain words; a file that is gone is also reported that way.
fn read_one<T: DeserializeOwned>(path: &Path, cap: u64) -> Result<Result<T, String>, FsError> {
    Ok(match fsio::read_regular_capped(path, cap)? {
        Some(Ok(text)) => parse(&text),
        Some(Err(why)) => Err(fsio::refusal_message(&why).to_string()),
        None => Err("the file disappeared while loading".into()),
    })
}

/// Reads one record whose ID the caller has already checked, or `None` when it
/// isn't there.
pub fn find_in<T: DeserializeOwned>(
    dir: &Path,
    id: &str,
    kind: IdKind,
    id_of: fn(&T) -> &str,
) -> Result<Option<T>, RecordError> {
    // The ID becomes part of a path, so only a well-formed one gets that far.
    if !ids::is_kind(id, kind) {
        return invalid(format!("'{id}' isn't a {kind} ID"));
    }
    let path = dir.join(format!("{id}.md"));
    match fsio::read_regular_capped(&path, MAX_RECORD_BYTES)? {
        None => Ok(None),
        Some(Err(why)) => invalid(fsio::refusal_message(&why)),
        Some(Ok(text)) => match parse::<T>(&text) {
            Ok(record) if id_of(&record) == id => Ok(Some(record)),
            Ok(_) => invalid(format!("the file for '{id}' holds a different record")),
            Err(why) => invalid(format!("'{id}' can't be read: {why}")),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    struct Sample {
        id: String,
        note: String,
    }

    fn id_of(s: &Sample) -> &str {
        &s.id
    }

    #[test]
    fn a_record_round_trips_and_holds_nothing_after_its_front_matter() {
        let sample = Sample {
            id: "verdict-abcdefgh".into(),
            note: "hello: world # not a comment".into(),
        };
        let text = render(&sample).unwrap();
        assert!(text.starts_with("---\n") && text.ends_with("---\n"));
        assert_eq!(parse::<Sample>(&text).unwrap(), sample);
        let with_notes = format!("{text}\nfree text\n");
        assert!(
            parse::<Sample>(&with_notes)
                .unwrap_err()
                .contains("nothing after")
        );
        assert!(parse::<Sample>("no front matter").is_err());
        assert!(parse::<Sample>("---\nid: x\nextra: 1\n---\n").is_err());
    }

    #[test]
    fn stamps_and_story_ids_are_checked_strictly() {
        assert!(is_stamp("2026-11-16T17:44"));
        for bad in [
            "",
            "2026-11-16",
            "2026-11-16 17:44",
            "2026-11-16T17:44:00",
            "20x6-11-16T17:44",
        ] {
            assert!(!is_stamp(bad), "{bad}");
        }
        assert!(is_story_id("p-abcdefgh"));
        assert!(is_story_id("f-abcdefgh"));
        for bad in [
            "idea-abcdefgh",
            "pr-abcdefgh",
            "verdict-abcdefgh",
            "../x",
            "p-0007",
        ] {
            assert!(!is_story_id(bad), "{bad}");
        }
    }

    #[test]
    fn a_folder_loads_clean_records_and_reports_everything_else() {
        let dir = TempDir::new();
        let folder = dir.path().join("verdicts");
        std::fs::create_dir_all(&folder).unwrap();
        let good = Sample {
            id: "verdict-abcdefgh".into(),
            note: "ok".into(),
        };
        std::fs::write(folder.join("verdict-abcdefgh.md"), render(&good).unwrap()).unwrap();
        std::fs::write(folder.join(".hidden"), "ignored").unwrap();
        std::fs::write(folder.join("notes.txt"), "x").unwrap();
        std::fs::write(folder.join("verdict-bad.md"), "x").unwrap();
        let other = Sample {
            id: "verdict-zzzzzzzz".into(),
            note: "wrong name".into(),
        };
        std::fs::write(folder.join("verdict-cccccccc.md"), render(&other).unwrap()).unwrap();
        std::fs::write(folder.join("verdict-dddddddd.md"), "---\nid: x\n").unwrap();
        std::fs::create_dir(folder.join("verdict-eeeeeeee.md")).unwrap();
        let big = "a".repeat(usize::try_from(MAX_RECORD_BYTES).unwrap() + 1);
        std::fs::write(folder.join("verdict-ffffffff.md"), big).unwrap();

        let loaded =
            load_folder::<Sample>(&folder, "spec/verdicts", IdKind::Verdict, id_of, 10).unwrap();
        assert_eq!(loaded.records, vec![good]);
        assert_eq!(loaded.problems.len(), 6, "{:#?}", loaded.problems);
        assert!(
            loaded
                .problems
                .iter()
                .all(|p| p.starts_with("spec/verdicts/"))
        );
    }

    #[test]
    fn a_missing_folder_is_empty_and_the_cap_is_reported() {
        let dir = TempDir::new();
        let missing =
            load_folder::<Sample>(&dir.path().join("nope"), "x", IdKind::Verdict, id_of, 3)
                .unwrap();
        assert!(missing.records.is_empty() && missing.problems.is_empty());

        let folder = dir.path().join("many");
        std::fs::create_dir_all(&folder).unwrap();
        for id in ["verdict-aaaaaaaa", "verdict-bbbbbbbb", "verdict-cccccccc"] {
            let s = Sample {
                id: id.into(),
                note: "n".into(),
            };
            std::fs::write(folder.join(format!("{id}.md")), render(&s).unwrap()).unwrap();
        }
        let capped = load_folder::<Sample>(&folder, "many", IdKind::Verdict, id_of, 2).unwrap();
        assert_eq!(capped.records.len(), 2);
        assert!(capped.problems[0].contains("more than 2"));
    }

    #[cfg(unix)]
    #[test]
    fn a_linked_record_is_reported_and_never_followed() {
        let dir = TempDir::new();
        let folder = dir.path().join("verdicts");
        std::fs::create_dir_all(&folder).unwrap();
        let target = dir.path().join("elsewhere.md");
        let s = Sample {
            id: "verdict-abcdefgh".into(),
            note: "n".into(),
        };
        std::fs::write(&target, render(&s).unwrap()).unwrap();
        std::os::unix::fs::symlink(&target, folder.join("verdict-abcdefgh.md")).unwrap();
        let loaded = load_folder::<Sample>(&folder, "v", IdKind::Verdict, id_of, 5).unwrap();
        assert!(loaded.records.is_empty());
        assert_eq!(loaded.problems.len(), 1);
        assert!(
            find_in::<Sample>(&folder, "verdict-abcdefgh", IdKind::Verdict, id_of).is_err(),
            "a direct read refuses the link too"
        );
    }
}

/// Shared set-up for the record modules' unit tests: the harbor test story with
/// one saved answer on file.
#[cfg(test)]
pub(crate) mod testkit {
    use std::path::{Path, PathBuf};

    use crate::project::Project;
    use crate::testutil::TempDir;

    /// A saved answer's ID (its file is in `.harness/committed/`).
    pub const ANSWER: &str = "pr-abcdefgh";
    /// A person at revision 2, one at revision 1, and the pilot, whom another
    /// person's fact reads.
    pub const MARA: &str = "p-maraharb";
    pub const JONAH: &str = "p-jnahharb";
    pub const DAD: &str = "p-dadsharb";

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

    pub fn story() -> (TempDir, Project) {
        let dir = TempDir::new();
        let project = Project::new(dir.path());
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/specs/harbor");
        copy_dir(&fixture, &project.spec_dir());
        let committed = project.harness_dir().join(crate::commit::COMMITTED_DIR);
        std::fs::create_dir_all(&committed).unwrap();
        std::fs::write(committed.join(format!("{ANSWER}.json")), "{}\n").unwrap();
        (dir, project)
    }

    /// A verdict draft as the skill would hand it over.
    pub fn verdict_draft(result: &str, cited: &[(&str, u32)], why: &str) -> String {
        let checked: Vec<_> = cited
            .iter()
            .map(|(id, revision)| serde_json::json!({"id": id, "revision": revision}))
            .collect();
        serde_json::json!({
            "answer": ANSWER,
            "result": result,
            "checked": checked,
            "justification": why,
        })
        .to_string()
    }

    /// A question draft.
    pub fn question_draft(concerns: &[&str], reason: &str, verdict: &str) -> String {
        serde_json::json!({"concerns": concerns, "reason": reason, "verdict": verdict}).to_string()
    }
}
