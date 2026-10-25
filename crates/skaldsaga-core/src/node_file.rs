//! Reading and writing spec nodes as Markdown files.
//!
//! A node file is YAML front matter between `---` lines, followed by free
//! prose notes for the writer:
//!
//! ```text
//! ---
//! id: p-7kq2xmhd
//! type: person
//! name: Mara Vance
//! revision: 1
//! facts: [...]
//! ---
//!
//! Notes nobody's checks rely on.
//! ```
//!
//! Written files always use the same key order (the common core, then the
//! type's own fields, then facts), so a rewrite by the committer only changes
//! the lines that actually changed.

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_norway::{Mapping, Value};

use crate::ids::{self, IdKind};
use crate::model::{Fact, Node, NodeBody, NodeType, Thread};

/// Why a node file couldn't be read or written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeError(pub String);

impl std::fmt::Display for NodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for NodeError {}

fn err(msg: impl Into<String>) -> NodeError {
    NodeError(msg.into())
}

const DELIMITER: &str = "---";
/// Keys of the common core, in written order (`type` comes from the body).
const CORE_KEYS: [&str; 5] = ["id", "name", "aliases", "summary", "revision"];

/// Splits a Markdown file into its front matter and the notes after it.
pub fn split_front_matter(text: &str) -> Result<(&str, &str), NodeError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines = text.split_inclusive('\n');
    let first = lines.next().unwrap_or("");
    if first.trim_end() != DELIMITER {
        return Err(err("the file doesn't start with a '---' line"));
    }
    let start = first.len();
    let mut pos = start;
    for line in lines {
        if line.trim_end() == DELIMITER {
            let (Some(front), Some(notes)) = (text.get(start..pos), text.get(pos + line.len()..))
            else {
                return Err(err("the front matter couldn't be split from the notes"));
            };
            return Ok((front, notes));
        }
        pos += line.len();
    }
    Err(err("the front matter has no closing '---' line"))
}

fn take<T: DeserializeOwned>(map: &mut Mapping, key: &str) -> Result<Option<T>, NodeError> {
    match map.remove(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => serde_norway::from_value(v)
            .map(Some)
            .map_err(|e| err(format!("'{key}': {e}"))),
    }
}

fn require<T: DeserializeOwned>(map: &mut Mapping, key: &str) -> Result<T, NodeError> {
    take(map, key)?.ok_or_else(|| err(format!("'{key}' is missing")))
}

fn body_from<T: DeserializeOwned>(rest: Mapping, node_type: NodeType) -> Result<T, NodeError> {
    serde_norway::from_value(Value::Mapping(rest))
        .map_err(|e| err(format!("{} fields: {e}", node_type.as_str())))
}

/// Parses a node file. Checks the shape of every field, that the ID matches
/// the node's type, and that fact IDs are well-formed and unique in the node.
/// Checks that need the rest of the spec (references, duplicates across
/// files) belong to the loader and the checker.
pub fn parse_node(text: &str) -> Result<Node, NodeError> {
    let (front, notes) = split_front_matter(text)?;
    let value: Value =
        serde_norway::from_str(front).map_err(|e| err(format!("front matter: {e}")))?;
    let Value::Mapping(mut map) = value else {
        return Err(err("the front matter isn't a list of 'key: value' fields"));
    };

    let node_type: NodeType = require(&mut map, "type")?;
    let id: String = require(&mut map, "id")?;
    if !ids::is_kind(&id, node_type.id_kind()) {
        return Err(err(format!(
            "'{id}' isn't a valid {} ID (expected '{}-' and 8 letters or digits)",
            node_type.as_str(),
            node_type.id_kind().prefix()
        )));
    }
    let name: String = require(&mut map, "name")?;
    let aliases: Vec<String> = take(&mut map, "aliases")?.unwrap_or_default();
    let summary: Option<String> = take(&mut map, "summary")?;
    let revision: u32 = require(&mut map, "revision")?;
    let facts: Vec<Fact> = take(&mut map, "facts")?.unwrap_or_default();

    let mut seen = std::collections::HashSet::new();
    for fact in &facts {
        if !ids::is_kind(&fact.id, IdKind::Fact) {
            return Err(err(format!("'{}' isn't a valid fact ID", fact.id)));
        }
        if !seen.insert(fact.id.as_str()) {
            return Err(err(format!("fact '{}' appears twice", fact.id)));
        }
    }

    let simple = |body: NodeBody| {
        if let Some((k, _)) = map.iter().next() {
            let key = k.as_str().unwrap_or("?");
            return Err(err(format!(
                "unknown field '{key}' for a {}",
                node_type.as_str()
            )));
        }
        Ok(body)
    };
    let body = match node_type {
        NodeType::Person => simple(NodeBody::Person)?,
        NodeType::Place => simple(NodeBody::Place)?,
        NodeType::Group => simple(NodeBody::Group)?,
        NodeType::Item => simple(NodeBody::Item)?,
        NodeType::Scene => NodeBody::Scene(body_from(map, node_type)?),
        NodeType::Decision => NodeBody::Decision(body_from(map, node_type)?),
        NodeType::Thread => {
            let thread: Thread = body_from(map, node_type)?;
            let stages: Vec<_> = thread.stages.iter().map(|s| s.stage).collect();
            if stages != crate::model::Stage::ALL {
                return Err(err(
                    "a thread needs exactly four stages, in order: setup, complications, crisis, resolution",
                ));
            }
            NodeBody::Thread(thread)
        }
        NodeType::Style => NodeBody::Style(body_from(map, node_type)?),
        NodeType::Arc => NodeBody::Arc(body_from(map, node_type)?),
    };

    Ok(Node {
        id,
        name,
        aliases,
        summary,
        revision,
        body,
        facts,
        notes: notes.trim_start_matches(['\n', '\r']).to_string(),
    })
}

fn to_value(v: &impl Serialize) -> Result<Value, NodeError> {
    serde_norway::to_value(v).map_err(|e| err(e.to_string()))
}

fn extend(map: &mut Mapping, body: &impl Serialize) -> Result<(), NodeError> {
    match to_value(body)? {
        Value::Mapping(m) => {
            map.extend(m);
            Ok(())
        }
        _ => Err(err("internal: node fields didn't serialize as a mapping")),
    }
}

/// Renders a node as a Markdown file, in the fixed key order.
pub fn render_node(node: &Node) -> Result<String, NodeError> {
    let mut map = Mapping::new();
    let s = |v: &str| Value::String(v.to_string());
    map.insert(s(CORE_KEYS[0]), s(&node.id));
    map.insert(s("type"), s(node.node_type().as_str()));
    map.insert(s(CORE_KEYS[1]), s(&node.name));
    if !node.aliases.is_empty() {
        map.insert(s(CORE_KEYS[2]), to_value(&node.aliases)?);
    }
    if let Some(summary) = &node.summary {
        map.insert(s(CORE_KEYS[3]), s(summary));
    }
    map.insert(s(CORE_KEYS[4]), Value::Number(node.revision.into()));
    match &node.body {
        NodeBody::Person | NodeBody::Place | NodeBody::Group | NodeBody::Item => {}
        NodeBody::Scene(b) => extend(&mut map, b)?,
        NodeBody::Decision(b) => extend(&mut map, b)?,
        NodeBody::Thread(b) => extend(&mut map, b)?,
        NodeBody::Style(b) => extend(&mut map, b)?,
        NodeBody::Arc(b) => extend(&mut map, b)?,
    }
    if !node.facts.is_empty() {
        map.insert(s("facts"), to_value(&node.facts)?);
    }
    let yaml = serde_norway::to_string(&Value::Mapping(map)).map_err(|e| err(e.to_string()))?;
    let mut out = format!("{DELIMITER}\n{yaml}{DELIMITER}\n");
    let notes = node.notes.trim_end();
    if !notes.is_empty() {
        out.push('\n');
        out.push_str(notes);
        out.push('\n');
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    const PERSON: &str = "\
---
id: p-7kq2xmhd
type: person
name: Mara Vance
aliases:
- Mara
- the courier
summary: Courier who smuggles letters past the blockade
revision: 12
facts:
- id: f-3wq8zkpa
  kind: truth
  revision: 3
  statement: Mara's eyes are grey-green
  attribute: eye_color
  value: grey-green
  true_from: unknown
  confidence: confirmed
  review: cleared
  source: interview
- id: f-9hd4mtre
  kind: belief
  revision: 1
  statement: Jonah can be trusted
  holder: p-7kq2xmhd
  contradicts: f-u6bn2xce
  true_from:
    after: f-3wq8zkpa
    offset: 3 days
    precision: approx
  ends:
    at: f-k7yp5vgq
  ended_by: f-k7yp5vgq
  reader:
  - scene: s-4hq8nzwt
    level: hinted
  - scene: s-p3rwe9ka
    level: revealed
  characters:
  - who: p-7kq2xmhd
    learns_in: s-p3rwe9ka
    how: worked_out
  confidence: provisional
  review: unreviewed
  source: harvest:manuscript/ch1.md
  role: warning
---

She counts steps when she's nervous.
";

    #[test]
    fn person_round_trips_exactly() {
        let node = parse_node(PERSON).unwrap();
        assert_eq!(node.node_type(), NodeType::Person);
        assert_eq!(node.revision, 12);
        assert_eq!(node.facts.len(), 2);
        assert_eq!(
            node.facts[0].true_from,
            Some(TimeAnchor::Unknown(UnknownTime::Unknown))
        );
        assert!(matches!(
            node.facts[1].true_from,
            Some(TimeAnchor::After(_))
        ));
        assert!(matches!(node.facts[1].ends, Some(TimeAnchor::At(_))));
        assert_eq!(node.facts[1].role, Some(FactRole::Warning));
        assert_eq!(node.notes, "She counts steps when she's nervous.\n");
        assert_eq!(render_node(&node).unwrap(), PERSON);
    }

    #[test]
    fn scene_round_trips() {
        let text = "\
---
id: s-p3rwe9ka
type: scene
name: The checkpoint
revision: 2
pov: p-7kq2xmhd
location: l-5mx9vcbt
participants:
- p-7kq2xmhd
- p-2ab3cdef
coverage:
  thread: t-6jw4kqzn
  stage: crisis
purpose: Mara discovers Jonah lied
when:
  after: s-4hq8nzwt
  offset: next morning
beats:
- The guard looks twice
reads:
- f-3wq8zkpa
turn:
  wants: Get the letters past the checkpoint
  against: The guard recognizes her
  ends: She gets through, but Jonah is arrested
status: outlined
---
";
        let node = parse_node(text).unwrap();
        let NodeBody::Scene(scene) = &node.body else {
            panic!("not a scene")
        };
        assert_eq!(scene.status, SceneStatus::Outlined);
        assert_eq!(scene.coverage.as_ref().unwrap().stage, Stage::Crisis);
        assert_eq!(render_node(&node).unwrap(), text);
    }

    #[test]
    fn thread_arc_decision_and_style_round_trip() {
        let thread = "\
---
id: t-6jw4kqzn
type: thread
name: Mara's inner journey
revision: 1
kind: personal
arc: a-8rt2wxyz
characters:
- who: p-7kq2xmhd
  role: lead
decisions:
- d-4kx7mnpq
stages:
- stage: setup
  summary: Mara trusts no one
  confidence: provisional
- stage: complications
  summary: null
- stage: crisis
  summary: null
- stage: resolution
  summary: null
---
";
        let arc = "\
---
id: a-8rt2wxyz
type: arc
name: The main story
revision: 4
status: main
threads:
- t-6jw4kqzn
premise:
  current: A courier smuggles letters past a blockade
  history:
  - date: 2026-10-18
    text: A courier carries letters
dramatic_question: Will Mara get the letters through?
framework:
  value: undecided
  confidence: provisional
landmarks:
  inciting_incident:
    scene: s-4hq8nzwt
    confidence: provisional
  midpoint: null
---
";
        let decision = "\
---
id: d-4kx7mnpq
type: decision
name: Changes or holds firm
revision: 1
question: By the end, has Mara become a different person?
area: hero
choices:
- changes
- holds_firm
value: changes
evidence:
- s-p3rwe9ka
pivot: f-9hd4mtre
confidence: provisional
review: unreviewed
---
";
        let style = "\
---
id: st-3nb5qwer
type: style
name: Viewpoint and style
revision: 1
defaults:
  person: third
  tense: past
  distance: close
viewpoints:
- who: p-7kq2xmhd
  overrides:
    person: first
---
";
        for text in [thread, arc, decision, style] {
            let node = parse_node(text).unwrap();
            assert_eq!(render_node(&node).unwrap(), text);
        }
    }

    #[test]
    fn rejects_problems_with_a_plain_message() {
        let cases = [
            ("no front matter", "id: p-7kq2xmhd\n", "doesn't start"),
            ("unclosed", "---\nid: p-7kq2xmhd\n", "no closing"),
            (
                "missing type",
                "---\nid: p-7kq2xmhd\n---\n",
                "'type' is missing",
            ),
            (
                "wrong prefix",
                "---\nid: s-7kq2xmhd\ntype: person\nname: X\nrevision: 1\n---\n",
                "valid person ID",
            ),
            (
                "unknown field on person",
                "---\nid: p-7kq2xmhd\ntype: person\nname: X\nrevision: 1\neyes: grey\n---\n",
                "unknown field 'eyes'",
            ),
            (
                "unknown field on scene",
                "---\nid: s-7kq2xmhd\ntype: scene\nname: X\nrevision: 1\nstatus: slot\nmood: dark\n---\n",
                "mood",
            ),
            (
                "bad enum value",
                "---\nid: s-7kq2xmhd\ntype: scene\nname: X\nrevision: 1\nstatus: finished\n---\n",
                "scene fields",
            ),
            (
                "bad fact id",
                "---\nid: p-7kq2xmhd\ntype: person\nname: X\nrevision: 1\nfacts:\n- {id: f-1, kind: truth, revision: 1, statement: x, confidence: confirmed, review: cleared, source: interview}\n---\n",
                "valid fact ID",
            ),
            (
                "three stages",
                "---\nid: t-7kq2xmhd\ntype: thread\nname: X\nrevision: 1\nkind: personal\narc: a-7kq2xmhd\nstages:\n- {stage: setup}\n- {stage: crisis}\n- {stage: resolution}\n---\n",
                "four stages",
            ),
        ];
        for (label, text, expected) in cases {
            let e = parse_node(text).unwrap_err();
            assert!(e.0.contains(expected), "{label}: {e}");
        }
    }

    #[test]
    fn duplicate_fact_ids_are_rejected() {
        let fact = "- {id: f-7kq2xmhd, kind: truth, revision: 1, statement: x, confidence: confirmed, review: cleared, source: interview}\n";
        let text = format!(
            "---\nid: p-7kq2xmhd\ntype: person\nname: X\nrevision: 1\nfacts:\n{fact}{fact}---\n"
        );
        assert!(parse_node(&text).unwrap_err().0.contains("twice"));
    }

    #[test]
    fn accepts_windows_line_endings_and_a_bom() {
        let text = "\u{feff}---\r\nid: p-7kq2xmhd\r\ntype: person\r\nname: X\r\nrevision: 1\r\n---\r\nNotes\r\n";
        let node = parse_node(text).unwrap();
        assert_eq!(node.name, "X");
        assert_eq!(node.notes.trim_end(), "Notes");
    }

    #[test]
    fn new_thread_gets_four_empty_stages() {
        let node = Node {
            id: "t-7kq2xmhd".into(),
            name: "Main plot".into(),
            aliases: vec![],
            summary: None,
            revision: 1,
            body: NodeBody::Thread(Thread {
                kind: ThreadKind::BigPicture,
                arc: "a-7kq2xmhd".into(),
                characters: vec![],
                decisions: vec![],
                stages: Thread::empty_stages(),
            }),
            facts: vec![],
            notes: String::new(),
        };
        let text = render_node(&node).unwrap();
        assert_eq!(parse_node(&text).unwrap(), node);
    }
}
