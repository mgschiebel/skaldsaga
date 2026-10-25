//! Filing a proposal: the one way an interviewer's answer reaches the inbox
//! (HANDOFF, Ready to build, Slice 1, item 4).
//!
//! The skill drafts the answer as a small file (the writer's words and the
//! operations), never as a command argument, and this program does the rest:
//! it checks the draft's shape, chooses the ID, names the agent, stamps the
//! time and writes the proposal into `.harness/inbox/`. A draft can't pick its
//! own ID, agent, lane or verdict, so an answer can never ask for a looser
//! lane or arrive already checked. Nothing in a draft is read as an
//! instruction; it is data until `check` and `commit` have run.
//!
//! A draft names the things it creates with placeholders such as `@p:mara`
//! (a person), `@a:main` (an arc) or `@t:plot` (a thread): a kind, a colon and
//! a short name. The program replaces each distinct placeholder, everywhere in
//! the operations, with one fresh ID of that kind, so the skill never has to
//! make up an ID and two drafts can never collide.

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

use crate::fsio::FsError;
use crate::ids::{self, IdKind};
use crate::project::Project;
use crate::proposal::{self, Op, Proposal, Revisions};

/// The agent every filed answer belongs to in slice 1.
pub const AGENT: &str = "interviewer";
/// Drafts are small; anything bigger isn't one of ours.
pub const MAX_DRAFT_BYTES: u64 = 64 * 1024;
/// The longest answer kept, in characters.
pub const MAX_ANSWER_CHARS: usize = 2000;

/// What a draft may say. Anything else is refused.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Draft {
    /// The writer's answer in their own words, for recaps.
    #[serde(default)]
    answer: Option<String>,
    ops: Vec<Op>,
}

/// Why a draft wasn't filed.
#[derive(Debug)]
pub enum ProposeError {
    Fs(FsError),
    /// The draft was unusable; the message is plain language.
    Invalid(String),
}

impl std::fmt::Display for ProposeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProposeError::Fs(e) => e.fmt(f),
            ProposeError::Invalid(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for ProposeError {}

impl From<FsError> for ProposeError {
    fn from(e: FsError) -> Self {
        ProposeError::Fs(e)
    }
}

fn invalid<T>(m: impl Into<String>) -> Result<T, ProposeError> {
    Err(ProposeError::Invalid(m.into()))
}

/// Files a draft as a new proposal and returns its ID. `created` is the
/// time to stamp, `YYYY-MM-DDTHH:MM`.
pub fn file(project: &Project, draft: &str, created: &str) -> Result<String, ProposeError> {
    let mut value: Value = serde_json::from_str(draft).map_err(|e| {
        ProposeError::Invalid(format!("the answer isn't in the expected shape: {e}"))
    })?;
    if let Some(ops) = value.get_mut("ops") {
        let mut made = HashMap::new();
        fill_placeholders(ops, &mut made)?;
    }
    let draft: Draft = serde_json::from_value(value).map_err(|e| {
        ProposeError::Invalid(format!("the answer isn't in the expected shape: {e}"))
    })?;
    if draft.ops.is_empty() {
        return invalid("the answer changes nothing");
    }
    let answer = draft
        .answer
        .map(|a| a.trim().to_string())
        .filter(|a| !a.is_empty());
    if answer
        .as_deref()
        .is_some_and(|a| a.chars().count() > MAX_ANSWER_CHARS)
    {
        return invalid("the answer is too long to keep");
    }

    let inbox = project.inbox_dir();
    let (id, path) = loop {
        let id = ids::new_id(IdKind::Proposal).map_err(|e| ProposeError::Invalid(e.to_string()))?;
        let path = inbox.join(format!("{id}.json"));
        if std::fs::symlink_metadata(&path).is_err() {
            break (id, path);
        }
    };
    let filed = Proposal {
        id: id.clone(),
        agent: AGENT.to_string(),
        lane: None,
        created: Some(created.to_string()),
        answer,
        ops: draft.ops,
        revisions: Revisions::default(),
        check: None,
    };
    project.write_atomic(&path, &proposal::render(&filed))?;
    Ok(id)
}

/// The kinds of thing a draft may create, by their ID prefix.
fn placeholder_kind(prefix: &str) -> Option<IdKind> {
    IdKind::ALL.into_iter().find(|k| {
        k.prefix() == prefix
            && !matches!(
                k,
                IdKind::Proposal | IdKind::Edition | IdKind::WriterDecision
            )
    })
}

/// Longest name a placeholder may carry.
const MAX_PLACEHOLDER_NAME: usize = 24;

/// Replaces every `@kind:name` in the strings of `value` with a fresh ID,
/// the same one for the same placeholder.
fn fill_placeholders(
    value: &mut Value,
    made: &mut HashMap<String, String>,
) -> Result<(), ProposeError> {
    match value {
        Value::String(s) => {
            if s.contains('@') {
                *s = fill_in_text(s, made)?;
            }
        }
        Value::Array(items) => {
            for v in items {
                fill_placeholders(v, made)?;
            }
        }
        Value::Object(map) => {
            for (_, v) in map.iter_mut() {
                fill_placeholders(v, made)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn fill_in_text(text: &str, made: &mut HashMap<String, String>) -> Result<String, ProposeError> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some((before, tail)) = rest.split_once('@') {
        out.push_str(before);
        let Some((prefix, after)) = tail.split_once(':') else {
            out.push('@');
            rest = tail;
            continue;
        };
        let Some(kind) = placeholder_kind(prefix) else {
            out.push('@');
            rest = tail;
            continue;
        };
        let len = after
            .bytes()
            .take_while(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'_')
            .count();
        if len == 0 || len > MAX_PLACEHOLDER_NAME {
            return invalid(format!(
                "a placeholder like @{prefix}:name needs a short name of lowercase letters, digits or underscores"
            ));
        }
        // The name is ASCII, so `len` bytes always end on a character boundary.
        let Some((name, remainder)) = after.split_at_checked(len) else {
            return invalid("a placeholder name couldn't be read");
        };
        let key = format!("{prefix}:{name}");
        let id = match made.get(&key) {
            Some(id) => id.clone(),
            None => {
                let id = ids::new_id(kind).map_err(|e| ProposeError::Invalid(e.to_string()))?;
                made.insert(key, id.clone());
                id
            }
        };
        out.push_str(&id);
        rest = remainder;
    }
    out.push_str(rest);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    fn project(dir: &TempDir) -> Project {
        std::fs::create_dir_all(dir.path().join(".harness")).unwrap();
        Project::new(dir.path())
    }

    const DRAFT: &str = r#"{
      "answer": "A courier smuggles letters past a blockade.",
      "ops": [{"op": "set", "target": "p-dadsharb.summary", "value": "Ada's father"}]
    }"#;

    #[test]
    fn files_a_draft_as_the_interviewers_proposal() {
        let dir = TempDir::new();
        let p = project(&dir);
        let id = file(&p, DRAFT, "2026-10-25T09:00").unwrap();
        assert!(ids::is_kind(&id, IdKind::Proposal));
        let text = std::fs::read_to_string(p.inbox_dir().join(format!("{id}.json"))).unwrap();
        let filed = proposal::parse(&text).unwrap();
        assert_eq!(filed.id, id);
        assert_eq!(filed.agent, "interviewer");
        assert_eq!(filed.created.as_deref(), Some("2026-10-25T09:00"));
        assert_eq!(
            filed.answer.as_deref(),
            Some("A courier smuggles letters past a blockade.")
        );
        assert_eq!(filed.lane, None);
        assert_eq!(filed.check, None);
        assert_eq!(filed.ops.len(), 1);
    }

    #[test]
    fn two_drafts_get_two_ids() {
        let dir = TempDir::new();
        let p = project(&dir);
        let a = file(&p, DRAFT, "2026-10-25T09:00").unwrap();
        let b = file(&p, DRAFT, "2026-10-25T09:00").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn a_draft_cannot_choose_its_own_id_agent_lane_or_verdict() {
        let dir = TempDir::new();
        let p = project(&dir);
        for field in [
            r#""id": "pr-aaaaaaaa""#,
            r#""agent": "housekeeper""#,
            r#""lane": "optimistic""#,
            r#""created": "2020-01-01T00:00""#,
            r#""check": {"status": "passed"}"#,
            r#""revisions": {}"#,
        ] {
            let draft =
                format!(r#"{{{field}, "ops": [{{"op": "delete", "target": "p-dadsharb"}}]}}"#);
            assert!(matches!(
                file(&p, &draft, "2026-10-25T09:00"),
                Err(ProposeError::Invalid(_))
            ));
        }
        assert!(!p.inbox_dir().exists() || std::fs::read_dir(p.inbox_dir()).unwrap().count() == 0);
    }

    #[test]
    fn refuses_unusable_drafts_and_files_nothing() {
        let dir = TempDir::new();
        let p = project(&dir);
        let long = format!(
            r#"{{"answer": "{}", "ops": [{{"op": "delete", "target": "p-dadsharb"}}]}}"#,
            "x".repeat(MAX_ANSWER_CHARS + 1)
        );
        for bad in [
            "not json",
            "{}",
            r#"{"ops": []}"#,
            r#"{"ops": [{"op": "explode", "target": "x"}]}"#,
            long.as_str(),
        ] {
            assert!(
                matches!(
                    file(&p, bad, "2026-10-25T09:00"),
                    Err(ProposeError::Invalid(_))
                ),
                "should refuse: {bad}"
            );
        }
        assert!(!p.inbox_dir().exists() || std::fs::read_dir(p.inbox_dir()).unwrap().count() == 0);
    }

    #[test]
    fn a_blank_answer_is_left_out() {
        let dir = TempDir::new();
        let p = project(&dir);
        let draft = r#"{"answer": "   ", "ops": [{"op": "delete", "target": "p-dadsharb"}]}"#;
        let id = file(&p, draft, "2026-10-25T09:00").unwrap();
        let text = std::fs::read_to_string(p.inbox_dir().join(format!("{id}.json"))).unwrap();
        assert_eq!(proposal::parse(&text).unwrap().answer, None);
    }

    #[test]
    fn placeholders_become_one_fresh_id_each_everywhere() {
        let dir = TempDir::new();
        let p = project(&dir);
        let draft = r#"{
          "answer": "Mara, at @p:mara's docks",
          "ops": [
            {"op": "create", "node": {"id": "@p:mara", "type": "person", "name": "Mara"}},
            {"op": "create", "node": {"id": "@l:docks", "type": "place", "name": "The docks"}},
            {"op": "set", "target": "@p:mara.summary", "value": "Courier"},
            {"op": "add_item", "target": "@a:main.threads", "value": "@t:plot"}
          ]
        }"#;
        let id = file(&p, draft, "2026-10-25T09:00").unwrap();
        let text = std::fs::read_to_string(p.inbox_dir().join(format!("{id}.json"))).unwrap();
        assert!(!text.contains("@p:") || text.contains("at @p:mara's docks"));
        let filed = proposal::parse(&text).unwrap();
        // The writer's own words are left alone.
        assert_eq!(filed.answer.as_deref(), Some("Mara, at @p:mara's docks"));
        let Op::Create { node, .. } = &filed.ops[0] else {
            panic!("first op");
        };
        let mara = node["id"].as_str().unwrap();
        assert!(ids::is_kind(mara, IdKind::Person));
        let Op::Set { target, .. } = &filed.ops[2] else {
            panic!("third op");
        };
        assert_eq!(target, &format!("{mara}.summary"));
        let Op::Create { node, .. } = &filed.ops[1] else {
            panic!("second op");
        };
        assert!(ids::is_kind(node["id"].as_str().unwrap(), IdKind::Place));
        let Op::AddItem { target, value, .. } = &filed.ops[3] else {
            panic!("fourth op");
        };
        assert!(target.starts_with("a-") && target.ends_with(".threads"));
        assert!(ids::is_kind(value.as_str().unwrap(), IdKind::Thread));
    }

    #[test]
    fn the_same_placeholder_in_two_drafts_gets_different_ids() {
        let dir = TempDir::new();
        let p = project(&dir);
        let draft = r#"{"ops": [{"op": "create", "node": {"id": "@p:mara", "type": "person", "name": "Mara"}}]}"#;
        let ids_of = |id: &str| {
            let text = std::fs::read_to_string(p.inbox_dir().join(format!("{id}.json"))).unwrap();
            text.split('"')
                .find(|t| t.starts_with("p-"))
                .unwrap()
                .to_string()
        };
        let a = file(&p, draft, "2026-10-25T09:00").unwrap();
        let b = file(&p, draft, "2026-10-25T09:00").unwrap();
        assert_ne!(ids_of(&a), ids_of(&b));
    }

    #[test]
    fn only_story_things_can_be_placeholders() {
        let dir = TempDir::new();
        let p = project(&dir);
        // Proposal, edition and writer-decision IDs are the program's alone.
        for text in ["@pr:x", "@e:x", "@wd:x"] {
            let draft = format!(
                r#"{{"ops": [{{"op": "set", "target": "p-dadsharb.summary", "value": "{text}"}}]}}"#
            );
            let id = file(&p, &draft, "2026-10-25T09:00").unwrap();
            let body = std::fs::read_to_string(p.inbox_dir().join(format!("{id}.json"))).unwrap();
            assert!(body.contains(text), "{text} is left as plain text");
        }
        // A broken placeholder is refused rather than guessed at.
        let draft = r#"{"ops": [{"op": "set", "target": "p-dadsharb.summary", "value": "@p:"}]}"#;
        assert!(matches!(
            file(&p, draft, "2026-10-25T09:00"),
            Err(ProposeError::Invalid(_))
        ));
        let long = format!(
            r#"{{"ops": [{{"op": "set", "target": "p-dadsharb.summary", "value": "@p:{}"}}]}}"#,
            "a".repeat(MAX_PLACEHOLDER_NAME + 1)
        );
        assert!(file(&p, &long, "2026-10-25T09:00").is_err());
    }
}
