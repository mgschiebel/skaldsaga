//! The `spec/` folder: loading every node and the reading order.
//!
//! Layout (HANDOFF, Spec and storage): one folder per node type (`people/`,
//! `places/`, `scenes/` …), one file per node named after its ID, plus
//! `sequence.md` (scene IDs in reading order), `attributes.md` (the story's
//! own attribute keys) and the generated `_index/` folder, which loading
//! ignores.
//!
//! Loading never stops at the first problem. It collects every problem it
//! finds, so a check can report them all at once in plain language.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::fsio::{self, EntryKind, FsError, Refused};
use crate::ids::{self, IdKind};
use crate::model::{Node, NodeType};
use crate::node_file;
use crate::registry::{self, ATTRIBUTES_FILE, AttributeDef};

/// File name of the reading order.
pub const SEQUENCE_FILE: &str = "sequence.md";
/// The generated index folder, never hand-edited.
pub const INDEX_DIR: &str = "_index";

/// A spec folder as loaded.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Spec {
    /// Every node that loaded cleanly, sorted by ID.
    pub nodes: Vec<Node>,
    /// Scene IDs in reading order.
    pub sequence: Vec<String>,
    /// The story's own attribute keys, from `attributes.md` (the starter
    /// vocabulary is built in; see [`crate::registry`]).
    pub attributes: Vec<AttributeDef>,
}

impl Spec {
    pub fn node(&self, id: &str) -> Option<&Node> {
        self.nodes
            .binary_search_by(|n| n.id.as_str().cmp(id))
            .ok()
            .and_then(|i| self.nodes.get(i))
    }
}

/// Something wrong with one file in the spec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    /// Path relative to the spec folder.
    pub path: String,
    pub message: String,
}

/// The result of loading: what loaded, and what didn't.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Loaded {
    pub spec: Spec,
    pub problems: Vec<Problem>,
}

fn relative(spec_dir: &Path, path: &Path) -> String {
    path.strip_prefix(spec_dir)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Files and folders loading skips: hidden ones, README files, and anything
/// starting with `_` (the generated index).
fn skipped(name: &str) -> bool {
    name.starts_with('.') || name.starts_with('_') || name.eq_ignore_ascii_case("README.md")
}

/// Loads a spec folder. A missing folder loads as an empty spec.
pub fn load(spec_dir: &Path) -> Result<Loaded, FsError> {
    let mut loaded = Loaded::default();
    let mut problem = |path: &Path, message: String| {
        loaded.problems.push(Problem {
            path: relative(spec_dir, path),
            message,
        })
    };

    let mut nodes = Vec::new();
    for entry in fsio::list_dir(spec_dir)? {
        let path = spec_dir.join(&entry.name);
        if skipped(&entry.name)
            || (!entry.is_dir && (entry.name == SEQUENCE_FILE || entry.name == ATTRIBUTES_FILE))
        {
            continue;
        }
        if matches!(entry.kind, EntryKind::Symlink | EntryKind::Other) {
            problem(
                &path,
                fsio::refusal_message(&Refused::NotARegularFile).into(),
            );
            continue;
        }
        let Some(node_type) = entry
            .is_dir
            .then(|| NodeType::from_folder(&entry.name))
            .flatten()
        else {
            problem(
                &path,
                "Skaldsaga doesn't use this here; story notes belong in a node's notes".into(),
            );
            continue;
        };
        for file in fsio::list_dir(&path)? {
            let file_path = path.join(&file.name);
            if skipped(&file.name) {
                continue;
            }
            if matches!(file.kind, EntryKind::Symlink | EntryKind::Other) {
                problem(
                    &file_path,
                    fsio::refusal_message(&Refused::NotARegularFile).into(),
                );
                continue;
            }
            match load_node(&file_path, &file.name, node_type)? {
                Ok(node) => nodes.push((file_path, node)),
                Err(message) => problem(&file_path, message),
            }
        }
    }

    // IDs must be unique across the whole spec, for nodes and facts alike.
    let mut first_seen: HashMap<String, PathBuf> = HashMap::new();
    let mut unique = Vec::with_capacity(nodes.len());
    for (path, node) in nodes {
        let mut clash = None;
        for id in std::iter::once(&node.id).chain(node.facts.iter().map(|f| &f.id)) {
            if let Some(other) = first_seen.get(id) {
                clash = Some(format!(
                    "ID '{id}' is already used in {}",
                    relative(spec_dir, other)
                ));
                break;
            }
        }
        if let Some(message) = clash {
            problem(&path, message);
            continue;
        }
        for id in std::iter::once(&node.id).chain(node.facts.iter().map(|f| &f.id)) {
            first_seen.insert(id.clone(), path.clone());
        }
        unique.push(node);
    }
    unique.sort_by(|a, b| a.id.cmp(&b.id));

    let seq_path = spec_dir.join(SEQUENCE_FILE);
    let (sequence, seq_problems) = match read_story_text(&seq_path)? {
        Some(Ok(text)) => parse_sequence(&text),
        Some(Err(message)) => (Vec::new(), vec![message]),
        None => (Vec::new(), Vec::new()),
    };
    for message in seq_problems {
        problem(&seq_path, message);
    }

    let attr_path = spec_dir.join(ATTRIBUTES_FILE);
    let attributes = match read_story_text(&attr_path)? {
        Some(Ok(text)) => {
            let (defs, attr_problems) = registry::parse_attributes_file(&text);
            for message in attr_problems {
                problem(&attr_path, message);
            }
            defs
        }
        Some(Err(message)) => {
            problem(&attr_path, message);
            Vec::new()
        }
        None => Vec::new(),
    };

    loaded.spec = Spec {
        nodes: unique,
        sequence,
        attributes,
    };
    Ok(loaded)
}

/// Reads one node file. The outer error is a file-system failure; the inner
/// one is a problem with the file itself.
/// Reads one story file without following links, up to the size cap
/// (SR-21, SR-23). A refusal comes back as a plain-words problem message.
fn read_story_text(path: &Path) -> Result<Option<Result<String, String>>, FsError> {
    Ok(fsio::read_regular_capped(path, fsio::MAX_STORY_FILE_BYTES)?
        .map(|r| r.map_err(|why| fsio::refusal_message(&why).to_string())))
}

fn load_node(
    path: &Path,
    file_name: &str,
    folder_type: NodeType,
) -> Result<Result<Node, String>, FsError> {
    let Some(stem) = file_name.strip_suffix(".md") else {
        return Ok(Err("only Markdown (.md) node files belong here".into()));
    };
    let text = match read_story_text(path)? {
        Some(Ok(text)) => text,
        Some(Err(message)) => return Ok(Err(message)),
        None => return Ok(Err("the file disappeared while loading".into())),
    };
    let node = match node_file::parse_node(&text) {
        Ok(node) => node,
        Err(e) => return Ok(Err(e.0)),
    };
    if node.node_type() != folder_type {
        return Ok(Err(format!(
            "this is a {} node, but it's in the {} folder",
            node.node_type().as_str(),
            folder_type.folder()
        )));
    }
    if node.id != stem {
        return Ok(Err(format!(
            "the file name should match the node's ID ('{}.md')",
            node.id
        )));
    }
    Ok(Ok(node))
}

/// The heading and explanation at the top of `sequence.md`.
const SEQUENCE_HEADER: &str = "\
# Reading order

The scenes of your story, in the order a reader meets them. Skaldsaga keeps
this list up to date; each line is one scene's ID.
";

/// Parses `sequence.md`: every line starting with `- ` names one scene.
/// Returns the scene IDs and any problems found.
pub fn parse_sequence(text: &str) -> (Vec<String>, Vec<String>) {
    let mut ids_out = Vec::new();
    let mut problems = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (n, line) in text.lines().enumerate() {
        let Some(item) = line.trim_start().strip_prefix("- ") else {
            continue;
        };
        let id = item.split_whitespace().next().unwrap_or("");
        if !ids::is_kind(id, IdKind::Scene) {
            problems.push(format!("line {}: '{id}' isn't a scene ID", n + 1));
        } else if !seen.insert(id.to_string()) {
            problems.push(format!("line {}: scene '{id}' is listed twice", n + 1));
        } else {
            ids_out.push(id.to_string());
        }
    }
    (ids_out, problems)
}

/// Renders `sequence.md` for the given scene IDs.
pub fn render_sequence(scene_ids: &[String]) -> String {
    let mut out = String::from(SEQUENCE_HEADER);
    if !scene_ids.is_empty() {
        out.push('\n');
        for id in scene_ids {
            out.push_str("- ");
            out.push_str(id);
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/specs")
            .join(name)
    }

    #[test]
    fn loads_the_courier_fixture_cleanly() {
        let loaded = load(&fixture("courier")).unwrap();
        assert_eq!(loaded.problems, vec![], "{:#?}", loaded.problems);
        let spec = loaded.spec;
        assert_eq!(spec.sequence.len(), 3);
        for scene in &spec.sequence {
            assert_eq!(
                spec.node(scene).map(Node::node_type),
                Some(NodeType::Scene),
                "{scene}"
            );
        }
        let types: std::collections::HashSet<_> = spec.nodes.iter().map(Node::node_type).collect();
        assert_eq!(types.len(), NodeType::ALL.len(), "every node type appears");
    }

    #[test]
    fn reports_every_problem_without_stopping() {
        let loaded = load(&fixture("broken")).unwrap();
        let mut found: Vec<_> = loaded
            .problems
            .iter()
            .map(|p| format!("{}: {}", p.path, p.message))
            .collect();
        found.sort();
        let expect = [
            ("notes.txt", "doesn't use this"),
            ("people/p-9xyz2345.md", "file name should match"),
            ("people/p-bbbbbbbb.md", "already used"),
            ("places/p-cccccccc.md", "in the places folder"),
            ("scenes/s-dddddddd.md", "missing field `status`"),
            ("sequence.md", "isn't a scene ID"),
            ("sequence.md", "listed twice"),
        ];
        assert_eq!(found.len(), expect.len(), "{found:#?}");
        for (path, msg) in expect {
            assert!(
                found.iter().any(|f| f.starts_with(path) && f.contains(msg)),
                "missing {path}: {msg} in {found:#?}"
            );
        }
        // The good node still loads.
        assert!(loaded.spec.node("p-aaaaaaaa").is_some());
    }

    #[test]
    fn missing_folder_is_an_empty_spec() {
        let dir = TempDir::new();
        let loaded = load(&dir.path().join("spec")).unwrap();
        assert_eq!(loaded, Loaded::default());
    }

    #[test]
    fn sequence_round_trips() {
        let ids = vec!["s-7kq2xmhd".to_string(), "s-p3rwe9ka".to_string()];
        let text = render_sequence(&ids);
        assert_eq!(parse_sequence(&text), (ids, vec![]));
        assert_eq!(parse_sequence(&render_sequence(&[])), (vec![], vec![]));
    }
}
