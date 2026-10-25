//! What the writer knows about the craft, and the concepts we teach it with.
//!
//! HANDOFF open item 19. Teaching is invisible: it happens inside the
//! interviewer's questions, so this module only keeps the facts the
//! interviewer needs to do it well.
//!
//! - **The concept list.** A reviewed list shipped inside the program, in our
//!   own words (`data/concepts.json`), plus concepts researched for one story,
//!   stored in that story's folder and marked provisional. Each concept has one
//!   chosen term, an optional shorthand, terms to avoid, and the concepts it
//!   builds on.
//! - **The writer's record**, `.harness/writer/craft.json`, committed like
//!   decisions so it follows the story between sessions. It holds what the
//!   writer was *taught*, what they *demonstrated* in their own story, and
//!   misconceptions that were corrected. Entries are superseded, never deleted.
//!   The writer's word that they already know a concept only ever reaches
//!   "taught"; "demonstrated" needs evidence from the story.
//! - **Option shuffling**, so the position of a choice never sways the writer.
//!
//! Writer state, not story meaning: none of this is in the spec. Nothing here
//! is read as an instruction; story text only ever appears as data.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::fsio::{self, FsError, Refused};
use crate::ids;
use crate::project::Project;

/// Bumped when a file's shape changes, so later versions can migrate.
pub const FORMAT: u32 = 1;
/// Folder under `.harness/` for writer state.
pub const WRITER_DIR: &str = "writer";
/// The writer's record, inside `WRITER_DIR`.
pub const RECORD_FILE: &str = "craft.json";
/// Folder inside `WRITER_DIR` for researched concepts, one file each.
pub const CONCEPTS_DIR: &str = "concepts";
/// Largest record file we will read.
pub const MAX_RECORD_BYTES: u64 = 256 * 1024;
/// Largest researched-concept file we will read.
pub const MAX_CONCEPT_BYTES: u64 = 32 * 1024;
/// Most researched concepts one story keeps.
pub const MAX_RESEARCHED: usize = 200;
/// Longest note kept with a record, in characters.
pub const MAX_NOTE_CHARS: usize = 500;
/// Longest concept ID.
pub const MAX_SLUG: usize = 40;

const BUILT_IN: &str = include_str!("../data/concepts.json");

/// One craft concept.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Concept {
    pub id: String,
    /// The one chosen craft term.
    pub term: String,
    /// The shorthand used once the term is earned ("MC"), or empty.
    #[serde(default)]
    pub short: String,
    /// Terms we never use for this concept.
    #[serde(default)]
    pub avoid: Vec<String>,
    /// Concepts that should be taught first.
    #[serde(default)]
    pub builds_on: Vec<String>,
    /// The plain explanation, in our own words.
    pub explanation: String,
    /// Researched concepts are provisional until reviewed.
    #[serde(default)]
    pub provisional: bool,
    /// Where a researched concept came from (links and titles, never copied text).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<String>,
    /// Story-specific examples; kept apart from the definition and stripped on export.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConceptFile {
    /// Present in the file and required by `deny_unknown_fields`; never read.
    #[serde(rename = "format")]
    _format: u32,
    concepts: Vec<Concept>,
}

/// Where a concept came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    BuiltIn,
    Researched,
}

/// Why something couldn't be recorded.
#[derive(Debug)]
pub enum CraftError {
    Fs(FsError),
    /// The input or a stored file was unusable; the message is plain language.
    Invalid(String),
}

impl std::fmt::Display for CraftError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CraftError::Fs(e) => e.fmt(f),
            CraftError::Invalid(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for CraftError {}

impl From<FsError> for CraftError {
    fn from(e: FsError) -> Self {
        CraftError::Fs(e)
    }
}

fn invalid<T>(m: impl Into<String>) -> Result<T, CraftError> {
    Err(CraftError::Invalid(m.into()))
}

/// The concepts shipped with the program.
pub fn built_in() -> Vec<Concept> {
    // A test checks that data/concepts.json parses and is not empty.
    serde_json::from_str::<ConceptFile>(BUILT_IN)
        .map(|file| file.concepts)
        .unwrap_or_default()
}

/// True for a well-formed concept ID: lowercase letters, digits and
/// underscores, starting with a letter. Checked before an ID becomes a path.
pub fn is_slug(id: &str) -> bool {
    let mut chars = id.chars();
    id.len() <= MAX_SLUG
        && chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn writer_dir(project: &Project) -> PathBuf {
    project.harness_dir().join(WRITER_DIR)
}

fn record_path(project: &Project) -> PathBuf {
    writer_dir(project).join(RECORD_FILE)
}

fn concept_dir(project: &Project) -> PathBuf {
    writer_dir(project).join(CONCEPTS_DIR)
}

fn record_rel() -> String {
    format!(".harness/{WRITER_DIR}/{RECORD_FILE}")
}

fn concept_rel(id: &str) -> String {
    format!(".harness/{WRITER_DIR}/{CONCEPTS_DIR}/{id}.json")
}

fn refused(path: &str, why: &Refused) -> CraftError {
    CraftError::Invalid(format!("{path}: {}", fsio::refusal_message(why)))
}

/// Every concept: the shipped list, then this story's researched ones. A file
/// that can't be used is reported in `problems` and skipped.
#[derive(Debug, Clone, Default)]
pub struct Concepts {
    pub list: Vec<(Concept, Origin)>,
    pub problems: Vec<String>,
}

impl Concepts {
    pub fn get(&self, id: &str) -> Option<&(Concept, Origin)> {
        self.list.iter().find(|(c, _)| c.id == id)
    }
}

/// Loads the shipped and researched concepts.
pub fn concepts(project: &Project) -> Result<Concepts, CraftError> {
    let mut out = Concepts {
        list: built_in()
            .into_iter()
            .map(|c| (c, Origin::BuiltIn))
            .collect(),
        problems: Vec::new(),
    };
    let mut seen = 0;
    for entry in fsio::list_dir(&concept_dir(project))? {
        let rel = format!(".harness/{WRITER_DIR}/{CONCEPTS_DIR}/{}", entry.name);
        let Some(stem) = entry.name.strip_suffix(".json").filter(|s| is_slug(s)) else {
            out.problems
                .push(format!("{rel}: only <concept>.json files belong here"));
            continue;
        };
        if entry.kind != fsio::EntryKind::File {
            out.problems.push(format!("{rel}: not a regular file"));
            continue;
        }
        seen += 1;
        if seen > MAX_RESEARCHED {
            out.problems.push(format!("{rel}: too many concept files"));
            continue;
        }
        let path = concept_dir(project).join(&entry.name);
        let text = match fsio::read_regular_capped(&path, MAX_CONCEPT_BYTES)? {
            Some(Ok(t)) => t,
            Some(Err(why)) => {
                out.problems.push(refused(&rel, &why).to_string());
                continue;
            }
            None => continue,
        };
        match serde_json::from_str::<Concept>(&text) {
            Ok(c) if c.id != stem => out.problems.push(format!(
                "{rel}: the file name should match the concept's ID"
            )),
            Ok(c) if out.get(&c.id).is_some() => out
                .problems
                .push(format!("{rel}: a concept with this ID already exists")),
            Ok(c) => out.list.push((c, Origin::Researched)),
            Err(e) => out.problems.push(format!("{rel}: {e}")),
        }
    }
    Ok(out)
}

/// A concept as far as the writer has got with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    Untaught,
    Taught,
    Demonstrated,
}

/// What kind of entry a record line is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Taught,
    Demonstrated,
    Misconception,
}

/// How an entry came about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Via {
    /// The interviewer taught it, or saw the writer use it in their story.
    Interviewer,
    /// The writer said so. Reaches "taught" at most.
    WriterSaid,
}

/// One line in the writer's record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub concept: String,
    pub kind: Kind,
    pub via: Via,
    /// `YYYY-MM-DD`.
    pub date: String,
    /// Story IDs showing the writer used the concept (`demonstrated` only).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<String>,
    /// For a misconception: what the writer believed. Their words, shortened.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
    /// For a misconception: whether it has been put right.
    #[serde(default)]
    pub corrected: bool,
    /// Replaced by a newer entry; kept as history.
    #[serde(default)]
    pub superseded: bool,
}

/// How much explaining the writer wants riding along with questions. It never
/// removes the one sentence of why.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Teaching {
    #[default]
    Normal,
    Less,
    MoreWhy,
}

/// The light starting check of the writer's experience.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Experience {
    pub written_before: bool,
    pub read_craft_books: bool,
}

/// The writer's craft record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub format: u32,
    #[serde(default)]
    pub teaching: Teaching,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experience: Option<Experience>,
    #[serde(default)]
    pub entries: Vec<Entry>,
}

impl Default for Record {
    fn default() -> Self {
        Record {
            format: FORMAT,
            teaching: Teaching::Normal,
            experience: None,
            entries: Vec::new(),
        }
    }
}

impl Record {
    /// How far the writer has got with a concept.
    pub fn level(&self, concept: &str) -> Level {
        self.live(concept)
            .map(|e| match e.kind {
                Kind::Demonstrated => Level::Demonstrated,
                Kind::Taught => Level::Taught,
                Kind::Misconception => Level::Untaught,
            })
            .max()
            .unwrap_or(Level::Untaught)
    }

    /// A misconception about the concept that hasn't been put right.
    pub fn open_misconception(&self, concept: &str) -> bool {
        self.live(concept)
            .any(|e| e.kind == Kind::Misconception && !e.corrected)
    }

    fn live<'a>(&'a self, concept: &'a str) -> impl Iterator<Item = &'a Entry> {
        self.entries
            .iter()
            .filter(move |e| e.concept == concept && !e.superseded)
    }
}

/// How the interviewer should refer to a concept right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TermUse {
    /// Not taught yet: explain in plain words and don't use the term.
    Explain,
    /// Taught: use the full term.
    Term,
    /// Earned: the shorthand, or the story's own names once they exist.
    Short,
}

/// The terminology ladder: a term is never used before it is taught, and is
/// shortened only once the writer has shown they can use it.
pub fn term_use(concept: &Concept, level: Level) -> TermUse {
    match level {
        Level::Untaught => TermUse::Explain,
        Level::Taught => TermUse::Term,
        Level::Demonstrated if concept.short.is_empty() => TermUse::Term,
        Level::Demonstrated => TermUse::Short,
    }
}

/// Loads the record. A missing file is an empty record; a file we can't use
/// is an error, so it is never overwritten by accident.
pub fn load(project: &Project) -> Result<Record, CraftError> {
    let rel = record_rel();
    let text = match fsio::read_regular_capped(&record_path(project), MAX_RECORD_BYTES)? {
        None => return Ok(Record::default()),
        Some(Err(why)) => return Err(refused(&rel, &why)),
        Some(Ok(t)) => t,
    };
    let record: Record =
        serde_json::from_str(&text).map_err(|e| CraftError::Invalid(format!("{rel}: {e}")))?;
    if record.format != FORMAT {
        return invalid(format!("{rel}: format {} isn't supported", record.format));
    }
    for e in &record.entries {
        if !is_slug(&e.concept) {
            return invalid(format!("{rel}: a concept ID isn't valid"));
        }
    }
    Ok(record)
}

fn save(project: &Project, record: &Record) -> Result<String, CraftError> {
    let mut text =
        serde_json::to_string_pretty(record).map_err(|e| CraftError::Invalid(e.to_string()))?;
    text.push('\n');
    project.write_atomic(&record_path(project), &text)?;
    Ok(record_rel())
}

/// What a recording command did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recorded {
    /// False when the record already said this and nothing changed.
    pub changed: bool,
    /// The path to hand to `git add`.
    pub path: String,
}

fn known(project: &Project, concept: &str) -> Result<(), CraftError> {
    if !is_slug(concept) {
        return invalid(format!("'{concept}' isn't a valid concept ID"));
    }
    if concepts(project)?.get(concept).is_none() {
        return invalid(format!("there's no concept '{concept}'"));
    }
    Ok(())
}

fn mark_superseded(record: &mut Record, concept: &str, kinds: &[Kind]) {
    for e in &mut record.entries {
        if e.concept == concept && !e.superseded && kinds.contains(&e.kind) {
            e.superseded = true;
        }
    }
}

/// Records that a concept was taught. If it already stands as taught or
/// demonstrated, nothing changes.
pub fn teach(
    project: &Project,
    concept: &str,
    via: Via,
    date: &str,
) -> Result<Recorded, CraftError> {
    known(project, concept)?;
    let mut record = load(project)?;
    if record.level(concept) >= Level::Taught {
        return Ok(Recorded {
            changed: false,
            path: record_rel(),
        });
    }
    record.entries.push(Entry {
        concept: concept.into(),
        kind: Kind::Taught,
        via,
        date: date.into(),
        evidence: Vec::new(),
        note: String::new(),
        corrected: false,
        superseded: false,
    });
    Ok(Recorded {
        changed: true,
        path: save(project, &record)?,
    })
}

/// Records that the writer used a concept correctly in their own story. The
/// evidence is the story IDs where they did; the writer's word alone is not
/// enough, so this is always from the interviewer's observation.
pub fn demonstrate(
    project: &Project,
    concept: &str,
    evidence: &[String],
    date: &str,
) -> Result<Recorded, CraftError> {
    known(project, concept)?;
    if evidence.is_empty() {
        return invalid("a concept is demonstrated only with evidence from the story");
    }
    if evidence.len() > 20 {
        return invalid("too many evidence IDs");
    }
    if let Some(bad) = evidence.iter().find(|id| ids::kind_of(id).is_none()) {
        return invalid(format!("'{bad}' isn't a valid story ID"));
    }
    let mut record = load(project)?;
    if record.level(concept) == Level::Demonstrated {
        return Ok(Recorded {
            changed: false,
            path: record_rel(),
        });
    }
    mark_superseded(&mut record, concept, &[Kind::Taught]);
    let mut evidence = evidence.to_vec();
    evidence.sort();
    evidence.dedup();
    record.entries.push(Entry {
        concept: concept.into(),
        kind: Kind::Demonstrated,
        via: Via::Interviewer,
        date: date.into(),
        evidence,
        note: String::new(),
        corrected: false,
        superseded: false,
    });
    Ok(Recorded {
        changed: true,
        path: save(project, &record)?,
    })
}

/// Records a misconception. It takes the concept back from "demonstrated" to
/// "taught" (the old entry is kept as history), because the writer's
/// understanding turned out not to be what it looked like.
pub fn misconception(
    project: &Project,
    concept: &str,
    note: &str,
    corrected: bool,
    date: &str,
) -> Result<Recorded, CraftError> {
    known(project, concept)?;
    let note = note.trim();
    if note.is_empty() {
        return invalid("say what the writer believed");
    }
    if note.chars().count() > MAX_NOTE_CHARS {
        return invalid(format!("the note is over {MAX_NOTE_CHARS} characters"));
    }
    let mut record = load(project)?;
    mark_superseded(&mut record, concept, &[Kind::Demonstrated]);
    // A newer misconception replaces an older open one on the same concept.
    mark_superseded(&mut record, concept, &[Kind::Misconception]);
    record.entries.push(Entry {
        concept: concept.into(),
        kind: Kind::Misconception,
        via: Via::Interviewer,
        date: date.into(),
        evidence: Vec::new(),
        note: note.into(),
        corrected,
        superseded: false,
    });
    Ok(Recorded {
        changed: true,
        path: save(project, &record)?,
    })
}

/// Sets how much teaching rides along with a question.
pub fn set_teaching(project: &Project, teaching: Teaching) -> Result<Recorded, CraftError> {
    let mut record = load(project)?;
    if record.teaching == teaching {
        return Ok(Recorded {
            changed: false,
            path: record_rel(),
        });
    }
    record.teaching = teaching;
    Ok(Recorded {
        changed: true,
        path: save(project, &record)?,
    })
}

/// Stores the answers to the light starting check.
pub fn set_experience(project: &Project, experience: Experience) -> Result<Recorded, CraftError> {
    let mut record = load(project)?;
    if record.experience == Some(experience) {
        return Ok(Recorded {
            changed: false,
            path: record_rel(),
        });
    }
    record.experience = Some(experience);
    Ok(Recorded {
        changed: true,
        path: save(project, &record)?,
    })
}

/// Stores a researched concept for this story, marked provisional. It must
/// not reuse an existing ID, build on a concept that doesn't exist, or use a
/// term another concept tells us to avoid.
pub fn add_researched(project: &Project, mut concept: Concept) -> Result<Recorded, CraftError> {
    if !is_slug(&concept.id) {
        return invalid(format!("'{}' isn't a valid concept ID", concept.id));
    }
    if concept.term.trim().is_empty() || concept.explanation.trim().is_empty() {
        return invalid("a concept needs a term and an explanation");
    }
    let all = concepts(project)?;
    if all.get(&concept.id).is_some() {
        return invalid(format!("there's already a concept '{}'", concept.id));
    }
    if all
        .list
        .iter()
        .filter(|(_, o)| *o == Origin::Researched)
        .count()
        >= MAX_RESEARCHED
    {
        return invalid("this story has as many researched concepts as it can keep");
    }
    if let Some(missing) = concept.builds_on.iter().find(|b| all.get(b).is_none()) {
        return invalid(format!("it builds on '{missing}', which isn't a concept"));
    }
    let text = format!("{} {} {}", concept.term, concept.short, concept.explanation).to_lowercase();
    for (other, _) in &all.list {
        for avoid in other.avoid.iter().filter(|a| !a.is_empty()) {
            if text.contains(&avoid.to_lowercase()) {
                return invalid(format!("it uses '{avoid}', a term we avoid"));
            }
        }
    }
    concept.provisional = true;
    let mut text =
        serde_json::to_string_pretty(&concept).map_err(|e| CraftError::Invalid(e.to_string()))?;
    text.push('\n');
    if text.len() as u64 > MAX_CONCEPT_BYTES {
        return invalid("the concept is too big");
    }
    let id = concept.id.clone();
    project.write_atomic(&concept_dir(project).join(format!("{id}.json")), &text)?;
    Ok(Recorded {
        changed: true,
        path: concept_rel(&id),
    })
}

/// Reads a researched concept from the text of a file the agent wrote.
pub fn parse_concept(text: &str) -> Result<Concept, CraftError> {
    serde_json::from_str(text).map_err(|e| CraftError::Invalid(format!("the concept file: {e}")))
}

/// Any record or concept as JSON for the command line.
pub fn to_value<T: Serialize>(item: &T) -> crate::json::Value {
    serde_json::to_value(item).unwrap_or(crate::json::Value::Null)
}

/// The researched concepts with everything story-specific removed, ready to
/// be reviewed for the built-in list.
pub fn export_researched(project: &Project) -> Result<Vec<Concept>, CraftError> {
    let mut out: Vec<Concept> = concepts(project)?
        .list
        .into_iter()
        .filter(|(_, o)| *o == Origin::Researched)
        .map(|(mut c, _)| {
            c.examples.clear();
            c
        })
        .collect();
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

/// Where each concept stands, for the interviewer.
#[derive(Debug, Clone, Serialize)]
pub struct ConceptStatus {
    pub id: String,
    pub term: String,
    pub short: String,
    pub origin: Origin,
    pub provisional: bool,
    pub level: Level,
    pub term_use: TermUse,
    pub open_misconception: bool,
    /// Concepts it builds on that haven't been taught yet.
    pub teach_first: Vec<String>,
}

/// Every concept with where the writer stands on it.
pub fn statuses(
    project: &Project,
) -> Result<(Record, Vec<ConceptStatus>, Vec<String>), CraftError> {
    let record = load(project)?;
    let all = concepts(project)?;
    let out = all
        .list
        .iter()
        .map(|(c, origin)| {
            let level = record.level(&c.id);
            ConceptStatus {
                id: c.id.clone(),
                term: c.term.clone(),
                short: c.short.clone(),
                origin: *origin,
                provisional: c.provisional,
                level,
                term_use: term_use(c, level),
                open_misconception: record.open_misconception(&c.id),
                teach_first: c
                    .builds_on
                    .iter()
                    .filter(|b| record.level(b) == Level::Untaught)
                    .cloned()
                    .collect(),
            }
        })
        .collect();
    Ok((record, out, all.problems))
}

/// A random order for `n` options (Fisher–Yates), so where a choice sits
/// never sways the writer. Done in code because asking a model to vary it
/// doesn't hold up.
pub fn shuffled_order(n: usize) -> Result<Vec<usize>, CraftError> {
    let mut order: Vec<usize> = (0..n).collect();
    for i in (1..n).rev() {
        let mut bytes = [0u8; 4];
        getrandom::fill(&mut bytes)
            .map_err(|e| CraftError::Invalid(format!("couldn't get random data: {e}")))?;
        let j = ((u64::from(u32::from_le_bytes(bytes)) * (i as u64 + 1)) >> 32) as usize;
        order.swap(i, j);
    }
    Ok(order)
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

    const D: &str = "2026-10-24";

    fn researched(id: &str) -> Concept {
        Concept {
            id: id.into(),
            term: "unreliable narrator".into(),
            short: String::new(),
            avoid: vec![],
            builds_on: vec!["pov_character".into()],
            explanation: "A narrator whose account the reader learns not to trust.".into(),
            provisional: false,
            sources: vec!["A craft article, 2024".into()],
            examples: vec!["Mara hides the letter, so her telling can't be trusted.".into()],
        }
    }

    #[test]
    fn the_shipped_list_is_sound() {
        let list = built_in();
        assert!(list.len() >= 15);
        let ids: Vec<&str> = list.iter().map(|c| c.id.as_str()).collect();
        for c in &list {
            assert!(is_slug(&c.id), "{}", c.id);
            assert!(!c.term.is_empty() && !c.explanation.is_empty(), "{}", c.id);
            assert_eq!(ids.iter().filter(|i| **i == c.id).count(), 1, "{}", c.id);
            for b in &c.builds_on {
                assert!(ids.contains(&b.as_str()), "{} builds on {b}", c.id);
                assert_ne!(b, &c.id);
            }
            // An avoided term is never used in any explanation or term.
            for other in &list {
                for a in other.avoid.iter().map(|a| a.to_lowercase()) {
                    let said = format!("{} {} {}", c.term, c.short, c.explanation).to_lowercase();
                    assert!(!said.contains(&a), "{} uses avoided '{a}'", c.id);
                }
            }
        }
        // No cycles: every concept can be reached by following builds_on down.
        for c in &list {
            let mut stack = c.builds_on.clone();
            let mut steps = 0;
            while let Some(next) = stack.pop() {
                steps += 1;
                assert!(steps < 200 && next != c.id, "cycle at {}", c.id);
                let n = list.iter().find(|x| x.id == next).unwrap();
                stack.extend(n.builds_on.clone());
            }
        }
    }

    #[test]
    fn slugs_are_strict() {
        for ok in ["main_character", "a", "x1"] {
            assert!(is_slug(ok), "{ok}");
        }
        for bad in [
            "",
            "1a",
            "A",
            "a-b",
            "a/b",
            "../x",
            "a b",
            "é",
            &"a".repeat(41),
        ] {
            assert!(!is_slug(bad), "{bad:?}");
        }
    }

    #[test]
    fn teaching_then_demonstrating_climbs_the_ladder() {
        let (_d, p) = project();
        let built = built_in();
        let mc = built.iter().find(|c| c.id == "main_character").unwrap();
        assert_eq!(load(&p).unwrap().level("main_character"), Level::Untaught);
        assert_eq!(term_use(mc, Level::Untaught), TermUse::Explain);

        let r = teach(&p, "main_character", Via::Interviewer, D).unwrap();
        assert!(r.changed);
        assert_eq!(r.path, ".harness/writer/craft.json");
        assert_eq!(load(&p).unwrap().level("main_character"), Level::Taught);
        assert_eq!(term_use(mc, Level::Taught), TermUse::Term);
        assert!(
            !teach(&p, "main_character", Via::WriterSaid, D)
                .unwrap()
                .changed
        );

        demonstrate(&p, "main_character", &["p-aaaaaaaa".into()], D).unwrap();
        let rec = load(&p).unwrap();
        assert_eq!(rec.level("main_character"), Level::Demonstrated);
        assert_eq!(term_use(mc, Level::Demonstrated), TermUse::Short);
        // The taught entry stays as history.
        assert_eq!(rec.entries.len(), 2);
        assert!(rec.entries[0].superseded);
    }

    #[test]
    fn a_term_without_shorthand_stays_a_term() {
        let built = built_in();
        let c = built.iter().find(|c| c.id == "challenger").unwrap();
        assert_eq!(term_use(c, Level::Demonstrated), TermUse::Term);
    }

    #[test]
    fn the_writers_word_never_reaches_demonstrated() {
        let (_d, p) = project();
        teach(&p, "thematic_question", Via::WriterSaid, D).unwrap();
        assert_eq!(load(&p).unwrap().level("thematic_question"), Level::Taught);
        assert!(demonstrate(&p, "thematic_question", &[], D).is_err());
        assert!(demonstrate(&p, "thematic_question", &["not-an-id".into()], D).is_err());
        assert!(demonstrate(&p, "thematic_question", &["../../x".into()], D).is_err());
        assert_eq!(load(&p).unwrap().level("thematic_question"), Level::Taught);
    }

    #[test]
    fn unknown_and_hostile_concept_ids_are_refused() {
        let (_d, p) = project();
        for bad in ["nope", "../craft", "a/b", ""] {
            assert!(teach(&p, bad, Via::Interviewer, D).is_err(), "{bad:?}");
        }
        assert!(!record_path(&p).exists());
    }

    #[test]
    fn a_misconception_sets_understanding_back_and_keeps_history() {
        let (_d, p) = project();
        demonstrate(&p, "foil", &["p-aaaaaaaa".into()], D).unwrap();
        misconception(&p, "foil", "thought it meant any villain", false, D).unwrap();
        let rec = load(&p).unwrap();
        assert_eq!(rec.level("foil"), Level::Untaught);
        assert!(rec.open_misconception("foil"));
        misconception(&p, "foil", "thought it meant any villain", true, D).unwrap();
        let rec = load(&p).unwrap();
        assert!(!rec.open_misconception("foil"));
        // Nothing was deleted: demonstrated, open misconception, corrected one.
        assert_eq!(rec.entries.len(), 3);
        assert_eq!(rec.entries.iter().filter(|e| !e.superseded).count(), 1);
        assert!(misconception(&p, "foil", "  ", false, D).is_err());
        assert!(misconception(&p, "foil", &"x".repeat(501), false, D).is_err());
    }

    #[test]
    fn preferences_and_experience_are_kept() {
        let (_d, p) = project();
        assert_eq!(load(&p).unwrap().teaching, Teaching::Normal);
        assert!(set_teaching(&p, Teaching::Less).unwrap().changed);
        assert!(!set_teaching(&p, Teaching::Less).unwrap().changed);
        let e = Experience {
            written_before: true,
            read_craft_books: false,
        };
        assert!(set_experience(&p, e).unwrap().changed);
        assert!(!set_experience(&p, e).unwrap().changed);
        let rec = load(&p).unwrap();
        assert_eq!(rec.teaching, Teaching::Less);
        assert_eq!(rec.experience, Some(e));
    }

    #[test]
    fn a_broken_record_is_reported_and_never_overwritten() {
        let (_d, p) = project();
        fsio::write_atomic(&record_path(&p), "{ not json").unwrap();
        assert!(load(&p).is_err());
        assert!(teach(&p, "foil", Via::Interviewer, D).is_err());
        assert_eq!(
            std::fs::read_to_string(record_path(&p)).unwrap(),
            "{ not json"
        );

        fsio::write_atomic(
            &record_path(&p),
            r#"{"format":1,"entries":[{"concept":"../x","kind":"taught","via":"interviewer","date":"d"}]}"#,
        )
        .unwrap();
        assert!(load(&p).is_err());
        fsio::write_atomic(&record_path(&p), r#"{"format":9,"entries":[]}"#).unwrap();
        assert!(load(&p).is_err());
    }

    #[test]
    fn an_oversized_record_is_refused() {
        let (_d, p) = project();
        fsio::write_atomic(&record_path(&p), &" ".repeat(300 * 1024)).unwrap();
        assert!(load(&p).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn a_linked_record_is_refused_not_followed() {
        let (d, p) = project();
        let target = d.path().join("elsewhere.json");
        std::fs::write(&target, r#"{"format":1,"entries":[]}"#).unwrap();
        std::fs::create_dir_all(writer_dir(&p)).unwrap();
        std::os::unix::fs::symlink(&target, record_path(&p)).unwrap();
        assert!(load(&p).is_err());
        assert!(teach(&p, "foil", Via::Interviewer, D).is_err());
    }

    #[test]
    fn researched_concepts_are_provisional_and_export_without_examples() {
        let (_d, p) = project();
        let r = add_researched(&p, researched("unreliable_narrator")).unwrap();
        assert_eq!(r.path, ".harness/writer/concepts/unreliable_narrator.json");
        let all = concepts(&p).unwrap();
        let (c, origin) = all.get("unreliable_narrator").unwrap();
        assert_eq!(*origin, Origin::Researched);
        assert!(c.provisional);
        assert_eq!(c.examples.len(), 1);
        // It can be taught like any other concept.
        teach(&p, "unreliable_narrator", Via::Interviewer, D).unwrap();

        let out = export_researched(&p).unwrap();
        assert_eq!(out.len(), 1);
        assert!(out[0].examples.is_empty());
        assert!(!serde_json::to_string(&out).unwrap().contains("Mara"));
        // The stored file still has its examples.
        assert!(
            std::fs::read_to_string(concept_dir(&p).join("unreliable_narrator.json"))
                .unwrap()
                .contains("Mara")
        );
    }

    #[test]
    fn researched_concepts_must_fit_the_list() {
        let (_d, p) = project();
        assert!(add_researched(&p, researched("main_character")).is_err());
        assert!(add_researched(&p, researched("../x")).is_err());
        let mut c = researched("a_thing");
        c.builds_on = vec!["nothing_here".into()];
        assert!(add_researched(&p, c).is_err());
        let mut c = researched("b_thing");
        c.explanation = "Another word for the protagonist.".into();
        assert!(add_researched(&p, c).is_err());
        let mut c = researched("c_thing");
        c.term = " ".into();
        assert!(add_researched(&p, c).is_err());
        add_researched(&p, researched("d_thing")).unwrap();
        assert!(add_researched(&p, researched("d_thing")).is_err());
    }

    #[test]
    fn bad_researched_files_are_reported_and_skipped() {
        let (_d, p) = project();
        let dir = concept_dir(&p);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("notes.txt"), "hi").unwrap();
        std::fs::write(dir.join("Bad Name.json"), "{}").unwrap();
        std::fs::write(dir.join("x_one.json"), "{ nope").unwrap();
        std::fs::write(
            dir.join("x_two.json"),
            serde_json::to_string(&researched("other_id")).unwrap(),
        )
        .unwrap();
        // A file may not shadow a shipped concept.
        std::fs::write(
            dir.join("foil.json"),
            serde_json::to_string(&researched("foil")).unwrap(),
        )
        .unwrap();
        let all = concepts(&p).unwrap();
        assert_eq!(all.problems.len(), 5, "{:?}", all.problems);
        assert!(all.get("other_id").is_none());
        assert_eq!(all.get("foil").unwrap().1, Origin::BuiltIn);
    }

    #[test]
    fn statuses_say_what_to_teach_first() {
        let (_d, p) = project();
        let (_, st, problems) = statuses(&p).unwrap();
        assert!(problems.is_empty());
        let pov = st.iter().find(|s| s.id == "pov_character").unwrap();
        assert_eq!(pov.teach_first, vec!["main_character".to_string()]);
        assert_eq!(pov.term_use, TermUse::Explain);
        teach(&p, "main_character", Via::Interviewer, D).unwrap();
        let (_, st, _) = statuses(&p).unwrap();
        let pov = st.iter().find(|s| s.id == "pov_character").unwrap();
        assert!(pov.teach_first.is_empty());
    }

    #[test]
    fn shuffling_is_a_permutation_and_actually_moves_things() {
        assert!(shuffled_order(0).unwrap().is_empty());
        assert_eq!(shuffled_order(1).unwrap(), vec![0]);
        let mut first_slot = [0u32; 4];
        for _ in 0..400 {
            let o = shuffled_order(4).unwrap();
            let mut sorted = o.clone();
            sorted.sort_unstable();
            assert_eq!(sorted, vec![0, 1, 2, 3]);
            first_slot[o[0]] += 1;
        }
        // Every option lands first now and then; no slot is favoured.
        assert!(first_slot.iter().all(|n| *n > 40), "{first_slot:?}");
    }
}
