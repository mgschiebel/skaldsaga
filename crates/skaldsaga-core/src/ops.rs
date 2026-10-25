//! The operation engine: applies a proposal's operations to a copy of the
//! spec, in memory.
//!
//! `check` uses it to see the spec as it would look after the change and
//! checks that; `commit` (build step 8) will reuse the same engine and write
//! the result to disk, so the two can never disagree about what a proposal
//! does.
//!
//! Field edits go through each node's front matter and are parsed back with
//! the same strict rules as node files, so an edit can never produce a node
//! file the loader would reject.
//!
//! **Revisions** are bumped once per proposal, by comparing the result with
//! the spec before it (HANDOFF, Governance, Version check): a node's revision
//! goes up when one of its node-level fields changes, or when a fact moves in
//! or out of it (`move_fact`, `merge`); a fact's revision goes up when the
//! fact changes. New nodes and facts start at 1, and a moved fact keeps its
//! revision.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde_norway::{Mapping, Value as Yaml};

use crate::ids::{self, IdKind};
use crate::json::Value;
use crate::model::{Fact, FactKind, Node, NodeBody};
use crate::node_file;
use crate::proposal::{ItemRef, Op, Target, parse_item_ref, parse_target};
use crate::registry;
use crate::spec::Spec;

/// Why an operation couldn't be applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpError {
    /// Position of the operation in the proposal, from 0.
    pub op: usize,
    pub message: String,
    /// IDs the problem is about.
    pub ids: Vec<String>,
}

/// The spec after a proposal, and what changed.
#[derive(Debug, Clone, PartialEq)]
pub struct Applied {
    pub spec: Spec,
    /// New node and fact IDs.
    pub created: BTreeSet<String>,
    /// Existing node and fact IDs whose content or revision changed.
    pub changed: BTreeSet<String>,
    /// Node and fact IDs that are gone (deleted, superseded or merged away).
    pub deleted: BTreeSet<String>,
    /// `(old fact, replacement)` pairs.
    pub superseded: Vec<(String, String)>,
    /// Facts whose kind changed.
    pub reclassified: Vec<String>,
    pub sequence_changed: bool,
    pub attributes_changed: bool,
    /// Node files to write (new or changed in any way, notes included).
    pub dirty_nodes: BTreeSet<String>,
    /// Node files to remove.
    pub removed_nodes: BTreeSet<String>,
}

impl Applied {
    /// Every node and fact the proposal writes: created, changed or removed.
    pub fn touched(&self) -> BTreeSet<String> {
        self.created
            .iter()
            .chain(&self.changed)
            .chain(&self.deleted)
            .cloned()
            .collect()
    }
}

/// Applies the operations in order. Stops at the first one that can't be
/// applied, since later operations usually depend on earlier ones.
pub fn apply(spec: &Spec, ops: &[Op]) -> Result<Applied, OpError> {
    let mut work = Work {
        spec: spec.clone(),
        moved: BTreeSet::new(),
        superseded: Vec::new(),
        reclassified: Vec::new(),
        sequence_changed: false,
        attributes_changed: false,
    };
    for (i, op) in ops.iter().enumerate() {
        work.apply(op).map_err(|(message, ids)| OpError {
            op: i,
            message,
            ids,
        })?;
    }
    Ok(work.finish(spec))
}

type Fail = (String, Vec<String>);

fn fail<T>(message: impl Into<String>, ids: &[&str]) -> Result<T, Fail> {
    Err((message.into(), ids.iter().map(|s| s.to_string()).collect()))
}

struct Work {
    spec: Spec,
    /// Nodes a fact moved in or out of.
    moved: BTreeSet<String>,
    superseded: Vec<(String, String)>,
    reclassified: Vec<String>,
    sequence_changed: bool,
    attributes_changed: bool,
}

// ---------------------------------------------------------------------------
// Front matter round trips

fn front_of(node: &Node) -> Result<Mapping, String> {
    let text = node_file::render_node(node).map_err(|e| e.0)?;
    let (front, _) = node_file::split_front_matter(&text).map_err(|e| e.0)?;
    match serde_norway::from_str(front) {
        Ok(Yaml::Mapping(m)) => Ok(m),
        Ok(_) => Err("internal: front matter isn't a mapping".into()),
        Err(e) => Err(e.to_string()),
    }
}

fn node_from(map: Mapping, notes: &str) -> Result<Node, String> {
    let yaml = serde_norway::to_string(&Yaml::Mapping(map)).map_err(|e| e.to_string())?;
    let text = format!("---\n{yaml}---\n\n{notes}");
    node_file::parse_node(&text).map_err(|e| e.0)
}

fn to_yaml(v: &Value) -> Result<Yaml, String> {
    serde_norway::to_value(v).map_err(|e| e.to_string())
}

fn key(k: &str) -> Yaml {
    Yaml::String(k.to_string())
}

/// Replaces every reference `old` with `new`, leaving `id` keys alone.
fn rewrite(value: &mut Yaml, old: &str, new: &str) -> bool {
    match value {
        Yaml::String(s) if s == old => {
            *s = new.to_string();
            true
        }
        Yaml::Sequence(items) => items
            .iter_mut()
            .fold(false, |acc, v| rewrite(v, old, new) | acc),
        Yaml::Mapping(map) => map.iter_mut().fold(false, |acc, (k, v)| {
            if k.as_str() == Some("id") {
                acc
            } else {
                rewrite(v, old, new) | acc
            }
        }),
        _ => false,
    }
}

/// Fills in `revision: 1` when a new node or fact leaves it out.
fn with_revision(v: &Value) -> Value {
    let mut v = v.clone();
    if let Some(map) = v.as_object_mut() {
        map.entry("revision").or_insert(Value::from(1));
    }
    v
}

/// A node position that no longer points at a node. The positions come from
/// lookups made a moment earlier, so this should never happen; it is an error
/// the proposal reports instead of a crash.
fn stale() -> Fail {
    ("internal: a node position went stale".to_string(), vec![])
}

impl Work {
    fn node_at(&self, i: usize) -> Result<&Node, Fail> {
        self.spec.nodes.get(i).ok_or_else(stale)
    }

    fn node_at_mut(&mut self, i: usize) -> Result<&mut Node, Fail> {
        self.spec.nodes.get_mut(i).ok_or_else(stale)
    }

    fn idx(&self, id: &str) -> Option<usize> {
        self.spec
            .nodes
            .binary_search_by(|n| n.id.as_str().cmp(id))
            .ok()
    }

    fn need(&self, id: &str) -> Result<usize, Fail> {
        self.idx(id)
            .map_or_else(|| fail(format!("'{id}' doesn't exist"), &[id]), Ok)
    }

    fn fact_home(&self, fact: &str) -> Option<usize> {
        self.spec
            .nodes
            .iter()
            .position(|n| n.facts.iter().any(|f| f.id == fact))
    }

    /// Finds a fact, checking the node it's said to be on.
    fn need_fact(&self, node: Option<&str>, fact: &str) -> Result<usize, Fail> {
        let Some(home) = self.fact_home(fact) else {
            return fail(format!("fact '{fact}' doesn't exist"), &[fact]);
        };
        if let Some(node) = node {
            let home_id = &self.node_at(home)?.id;
            if home_id != node {
                return fail(
                    format!("fact '{fact}' is on '{home_id}', not '{node}'"),
                    &[fact, node],
                );
            }
        }
        Ok(home)
    }

    fn taken(&self, id: &str) -> bool {
        self.idx(id).is_some() || self.fact_home(id).is_some()
    }

    fn insert_node(&mut self, node: Node) {
        let at = self
            .spec
            .nodes
            .binary_search_by(|n| n.id.cmp(&node.id))
            .unwrap_or_else(|i| i);
        self.spec.nodes.insert(at, node);
    }

    fn replace_node(&mut self, i: usize, node: Node) -> Result<(), Fail> {
        *self.node_at_mut(i)? = node;
        Ok(())
    }

    fn rewrite_all(&mut self, old: &str, new: &str) -> Result<(), Fail> {
        for i in 0..self.spec.nodes.len() {
            let node = self.node_at(i)?;
            let mut map = front_of(node).map_err(|e| (e, vec![]))?;
            let mut v = Yaml::Mapping(std::mem::take(&mut map));
            if rewrite(&mut v, old, new) {
                let Yaml::Mapping(map) = v else {
                    return fail("internal: front matter isn't a mapping", &[]);
                };
                let notes = node.notes.clone();
                let id = node.id.clone();
                let rebuilt = node_from(map, &notes).map_err(|e| {
                    (
                        format!("redirecting '{old}' to '{new}' breaks '{id}': {e}"),
                        vec![id.clone()],
                    )
                })?;
                self.replace_node(i, rebuilt)?;
            }
        }
        Ok(())
    }

    fn parse_fact(&self, v: &Value) -> Result<Fact, Fail> {
        let fact: Fact = serde_json::from_value(with_revision(v))
            .map_err(|e| (format!("the fact isn't well-formed: {e}"), vec![]))?;
        if !ids::is_kind(&fact.id, IdKind::Fact) {
            return fail(format!("'{}' isn't a valid fact ID", fact.id), &[&fact.id]);
        }
        if self.taken(&fact.id) {
            return fail(format!("'{}' is already used", fact.id), &[&fact.id]);
        }
        Ok(fact)
    }

    fn apply(&mut self, op: &Op) -> Result<(), Fail> {
        match op {
            Op::Create { node, notes } => self.create(node, notes.as_deref().unwrap_or("")),
            Op::Set { target, value } => self.set(target, value),
            Op::AddFact { node, fact } => {
                let i = self.need(node)?;
                let fact = self.parse_fact(fact)?;
                self.node_at_mut(i)?.facts.push(fact);
                Ok(())
            }
            Op::AddItem {
                target,
                value,
                after,
                before,
            } => self.add_item(target, value, after.as_deref(), before.as_deref()),
            Op::RemoveItem { target, value } => self.remove_item(target, value),
            Op::Supersede {
                target,
                replacement,
            } => self.supersede(target, replacement),
            Op::Delete { target } => self.delete(target),
            Op::Merge { survivor, merged } => self.merge(survivor, merged),
            Op::MoveFact { fact, to } => {
                let from = self.need_fact(None, fact)?;
                let to_i = self.need(to)?;
                if from == to_i {
                    return fail(format!("fact '{fact}' is already on '{to}'"), &[fact, to]);
                }
                self.node_at(to_i)?;
                let from_node = self.node_at_mut(from)?;
                let Some(pos) = from_node.facts.iter().position(|f| f.id == *fact) else {
                    return Err(stale());
                };
                let moving = from_node.facts.remove(pos);
                let from_id = from_node.id.clone();
                self.moved.insert(from_id);
                self.moved.insert(to.clone());
                self.node_at_mut(to_i)?.facts.push(moving);
                Ok(())
            }
            Op::Reclassify {
                target,
                kind,
                holder,
                contradicts,
            } => self.reclassify(target, *kind, holder, contradicts),
            Op::AddAttribute { attribute } => {
                registry::validate_new(&self.spec.attributes, attribute)
                    .map_err(|e| (e, vec![]))?;
                self.spec.attributes.push(attribute.clone());
                self.attributes_changed = true;
                Ok(())
            }
        }
    }

    fn create(&mut self, node: &Value, notes: &str) -> Result<(), Fail> {
        let mut v = with_revision(node);
        if let Some(facts) = v.get_mut("facts").and_then(Value::as_array_mut) {
            for f in facts.iter_mut() {
                *f = with_revision(f);
            }
        }
        let Yaml::Mapping(map) = to_yaml(&v).map_err(|e| (e, vec![]))? else {
            return fail("a new node must be an object of fields", &[]);
        };
        let node = node_from(map, notes)
            .map_err(|e| (format!("the new node isn't well-formed: {e}"), vec![]))?;
        for id in std::iter::once(&node.id).chain(node.facts.iter().map(|f| &f.id)) {
            if self.taken(id) {
                return fail(format!("'{id}' is already used"), &[id]);
            }
        }
        self.insert_node(node);
        Ok(())
    }

    /// Edits a node's front matter at a target, then parses it back.
    fn edit(
        &mut self,
        node_id: &str,
        fact: Option<&str>,
        field: &str,
        change: impl FnOnce(&mut Mapping) -> Result<(), String>,
    ) -> Result<(), Fail> {
        let i = self.need(node_id)?;
        if let Some(fact) = fact {
            self.need_fact(Some(node_id), fact)?;
        }
        let node = self.node_at(i)?;
        let mut map = front_of(node).map_err(|e| (e, vec![]))?;
        let notes = node.notes.clone();
        let ids: Vec<&str> = std::iter::once(node_id).chain(fact).collect();
        let result = match fact {
            None => change(&mut map),
            Some(fact) => {
                let entry = map
                    .get_mut(key("facts"))
                    .and_then(Yaml::as_sequence_mut)
                    .and_then(|facts| {
                        facts
                            .iter_mut()
                            .find(|f| f.get("id").and_then(Yaml::as_str) == Some(fact))
                    })
                    .and_then(Yaml::as_mapping_mut);
                match entry {
                    Some(entry) => change(entry),
                    None => Err(format!("fact '{fact}' doesn't exist")),
                }
            }
        };
        result.map_err(|e| (e, ids.iter().map(|s| s.to_string()).collect()))?;
        let type_name = node.node_type().as_str();
        let rebuilt = node_from(map, &notes).map_err(|e| {
            (
                format!("can't change '{field}' on this {type_name}: {e}"),
                ids.iter().map(|s| s.to_string()).collect(),
            )
        })?;
        self.replace_node(i, rebuilt)?;
        Ok(())
    }

    fn set(&mut self, target: &str, value: &Value) -> Result<(), Fail> {
        let t = parse_target(target).map_err(|e| (e, vec![]))?;
        let (node, fact, field) = match &t {
            Target::Sequence => {
                return fail(
                    "the reading order changes with add_item and remove_item, not set",
                    &[],
                );
            }
            Target::NodeField { node, field } => (node.as_str(), None, field.as_str()),
            Target::FactField { node, fact, field } => {
                (node.as_str(), Some(fact.as_str()), field.as_str())
            }
        };
        let protected: &[&str] = if fact.is_some() {
            &["id", "revision"]
        } else {
            &["id", "type", "revision", "facts"]
        };
        if protected.contains(&field) {
            return fail(format!("'{field}' can't be set"), &[node]);
        }
        if fact.is_none() && field == "notes" {
            let i = self.need(node)?;
            let text = match value {
                Value::Null => String::new(),
                Value::String(s) => s.clone(),
                _ => return fail("notes are plain text", &[node]),
            };
            self.node_at_mut(i)?.notes = text;
            return Ok(());
        }
        let yaml = to_yaml(value).map_err(|e| (e, vec![]))?;
        self.edit(node, fact, field, |map| {
            if yaml.is_null() {
                map.remove(key(field));
            } else {
                map.insert(key(field), yaml);
            }
            Ok(())
        })
    }

    fn list_target(target: &str) -> Result<Target, Fail> {
        parse_target(target).map_err(|e| (e, vec![]))
    }

    fn add_item(
        &mut self,
        target: &str,
        value: &Value,
        after: Option<&str>,
        before: Option<&str>,
    ) -> Result<(), Fail> {
        match Self::list_target(target)? {
            Target::Sequence => {
                let Some(scene) = value.as_str() else {
                    return fail("the reading order holds scene IDs", &[]);
                };
                let is_scene = self.idx(scene).is_some_and(|i| {
                    self.spec
                        .nodes
                        .get(i)
                        .is_some_and(|n| matches!(n.body, NodeBody::Scene(_)))
                });
                if !is_scene {
                    return fail(format!("'{scene}' isn't a scene"), &[scene]);
                }
                if self.spec.sequence.iter().any(|s| s == scene) {
                    return fail(
                        format!("'{scene}' is already in the reading order"),
                        &[scene],
                    );
                }
                let anchor_pos = |a: &str| {
                    self.spec.sequence.iter().position(|s| s == a).map_or_else(
                        || fail(format!("'{a}' isn't in the reading order"), &[a]),
                        Ok,
                    )
                };
                let at = match (after, before) {
                    (Some(_), Some(_)) => {
                        return fail("give 'after' or 'before', not both", &[scene]);
                    }
                    (Some(a), None) => anchor_pos(a)? + 1,
                    (None, Some(b)) => anchor_pos(b)?,
                    (None, None) => self.spec.sequence.len(),
                };
                self.spec.sequence.insert(at, scene.to_string());
                self.sequence_changed = true;
                Ok(())
            }
            t => {
                if after.is_some() || before.is_some() {
                    return fail("'after' and 'before' only apply to the reading order", &[]);
                }
                let (node, fact, field) = Self::field_parts(&t)?;
                let item = to_yaml(value).map_err(|e| (e, vec![]))?;
                self.edit(&node, fact.as_deref(), &field, |map| {
                    let list = map
                        .entry(key(&field))
                        .or_insert_with(|| Yaml::Sequence(vec![]));
                    if list.is_null() {
                        *list = Yaml::Sequence(vec![]);
                    }
                    let Some(items) = list.as_sequence_mut() else {
                        return Err(format!("'{field}' isn't a list"));
                    };
                    if items.contains(&item) {
                        return Err(format!("that's already listed in '{field}'"));
                    }
                    items.push(item);
                    Ok(())
                })
            }
        }
    }

    fn remove_item(&mut self, target: &str, value: &Value) -> Result<(), Fail> {
        match Self::list_target(target)? {
            Target::Sequence => {
                let scene = value.as_str().unwrap_or_default();
                let Some(pos) = self.spec.sequence.iter().position(|s| s == scene) else {
                    return fail(format!("'{scene}' isn't in the reading order"), &[scene]);
                };
                self.spec.sequence.remove(pos);
                self.sequence_changed = true;
                Ok(())
            }
            t => {
                let (node, fact, field) = Self::field_parts(&t)?;
                let item = to_yaml(value).map_err(|e| (e, vec![]))?;
                self.edit(&node, fact.as_deref(), &field, |map| {
                    let items = map
                        .get_mut(key(&field))
                        .and_then(Yaml::as_sequence_mut)
                        .ok_or_else(|| format!("'{field}' has nothing to remove"))?;
                    let pos = items
                        .iter()
                        .position(|v| *v == item)
                        .ok_or_else(|| format!("that isn't listed in '{field}'"))?;
                    items.remove(pos);
                    if items.is_empty() {
                        map.remove(key(&field));
                    }
                    Ok(())
                })
            }
        }
    }

    fn field_parts(t: &Target) -> Result<(String, Option<String>, String), Fail> {
        match t {
            Target::Sequence => fail("the reading order has no fields", &[]),
            Target::NodeField { node, field } => Ok((node.clone(), None, field.clone())),
            Target::FactField { node, fact, field } => {
                Ok((node.clone(), Some(fact.clone()), field.clone()))
            }
        }
    }

    fn fact_ref(target: &str) -> Result<(Option<String>, String), Fail> {
        match parse_item_ref(target) {
            ItemRef::Fact { node, fact } => Ok((node, fact)),
            ItemRef::Node(_) => fail(
                format!("'{target}' should name a fact ('node#fact')"),
                &[target],
            ),
        }
    }

    fn supersede(&mut self, target: &str, replacement: &Value) -> Result<(), Fail> {
        let (node, old) = Self::fact_ref(target)?;
        let home = self.need_fact(node.as_deref(), &old)?;
        let new = self.parse_fact(replacement)?;
        let new_id = new.id.clone();
        let Some(slot) = self
            .node_at_mut(home)?
            .facts
            .iter_mut()
            .find(|f| f.id == old)
        else {
            return Err(stale());
        };
        *slot = new;
        self.rewrite_all(&old, &new_id)?;
        self.superseded.push((old, new_id));
        Ok(())
    }

    fn delete(&mut self, target: &str) -> Result<(), Fail> {
        match parse_item_ref(target) {
            ItemRef::Node(id) => {
                let i = self.need(&id)?;
                self.spec.nodes.remove(i);
                Ok(())
            }
            ItemRef::Fact { node, fact } => {
                let home = self.need_fact(node.as_deref(), &fact)?;
                self.node_at_mut(home)?.facts.retain(|f| f.id != fact);
                Ok(())
            }
        }
    }

    fn merge(&mut self, survivor: &str, merged: &str) -> Result<(), Fail> {
        if survivor == merged {
            return fail("a node can't be merged into itself", &[survivor]);
        }
        let s = self.need(survivor)?;
        let m = self.need(merged)?;
        let (st, mt) = (self.node_at(s)?.node_type(), self.node_at(m)?.node_type());
        if st != mt {
            return fail(
                format!("a {} can't be merged into a {}", mt.as_str(), st.as_str()),
                &[survivor, merged],
            );
        }
        let gone = self.spec.nodes.remove(m);
        let s = self.need(survivor)?;
        let target = self.node_at_mut(s)?;
        for alias in std::iter::once(&gone.name).chain(&gone.aliases) {
            if *alias != target.name && !target.aliases.contains(alias) {
                target.aliases.push(alias.clone());
            }
        }
        target.facts.extend(gone.facts);
        self.moved.insert(survivor.to_string());
        // The reading order lists each scene once.
        if let Some(pos) = self.spec.sequence.iter().position(|x| x == merged) {
            if self.spec.sequence.iter().any(|x| x == survivor) {
                self.spec.sequence.remove(pos);
            } else {
                if let Some(slot) = self.spec.sequence.get_mut(pos) {
                    *slot = survivor.to_string();
                }
            }
            self.sequence_changed = true;
        }
        self.rewrite_all(merged, survivor)
    }

    fn reclassify(
        &mut self,
        target: &str,
        kind: FactKind,
        holder: &Option<String>,
        contradicts: &Option<String>,
    ) -> Result<(), Fail> {
        let (node, fact) = Self::fact_ref(target)?;
        let home = self.need_fact(node.as_deref(), &fact)?;
        let Some(f) = self
            .node_at_mut(home)?
            .facts
            .iter_mut()
            .find(|f| f.id == fact)
        else {
            return Err(stale());
        };
        if f.kind == kind {
            return fail(
                format!("fact '{fact}' is already a {}", kind_word(kind)),
                &[&fact],
            );
        }
        match kind {
            FactKind::Belief => {
                let Some(holder) = holder else {
                    return fail(
                        "a belief needs a holder: who believes it (a person, or 'reader')",
                        &[&fact],
                    );
                };
                f.holder = Some(holder.clone());
                f.contradicts = contradicts.clone();
            }
            FactKind::Truth => {
                if holder.is_some() || contradicts.is_some() {
                    return fail("a truth has no holder and contradicts nothing", &[&fact]);
                }
                f.holder = None;
                f.contradicts = None;
            }
        }
        f.kind = kind;
        self.reclassified.push(fact);
        Ok(())
    }

    /// Bumps revisions against the spec before the proposal and lists what changed.
    fn finish(mut self, before: &Spec) -> Applied {
        let old_nodes: HashMap<&str, &Node> =
            before.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
        let old_facts: HashMap<&str, &Fact> = before
            .nodes
            .iter()
            .flat_map(|n| n.facts.iter().map(|f| (f.id.as_str(), f)))
            .collect();

        let mut created = BTreeSet::new();
        let mut changed = BTreeSet::new();
        let mut dirty_nodes = BTreeSet::new();
        let mut seen: BTreeSet<String> = BTreeSet::new();

        for node in &mut self.spec.nodes {
            seen.insert(node.id.clone());
            for fact in &mut node.facts {
                seen.insert(fact.id.clone());
                match old_facts.get(fact.id.as_str()) {
                    None => {
                        fact.revision = 1;
                        created.insert(fact.id.clone());
                    }
                    Some(old) => {
                        let differs = {
                            let mut a = (*old).clone();
                            a.revision = 0;
                            let mut b = fact.clone();
                            b.revision = 0;
                            a != b
                        };
                        fact.revision = old.revision + u32::from(differs);
                        if differs {
                            changed.insert(fact.id.clone());
                        }
                    }
                }
            }
            match old_nodes.get(node.id.as_str()) {
                None => {
                    node.revision = 1;
                    created.insert(node.id.clone());
                    dirty_nodes.insert(node.id.clone());
                }
                Some(old) => {
                    let differs =
                        node_level(old) != node_level(node) || self.moved.contains(&node.id);
                    node.revision = old.revision + u32::from(differs);
                    if differs {
                        changed.insert(node.id.clone());
                    }
                    if **old != *node {
                        dirty_nodes.insert(node.id.clone());
                    }
                }
            }
        }

        let deleted: BTreeSet<String> = old_nodes
            .keys()
            .chain(old_facts.keys())
            .filter(|id| !seen.contains(**id))
            .map(|id| id.to_string())
            .collect();
        let removed_nodes = old_nodes
            .keys()
            .filter(|id| !seen.contains(**id))
            .map(|id| id.to_string())
            .collect();

        Applied {
            spec: self.spec,
            created,
            changed,
            deleted,
            superseded: self.superseded,
            reclassified: self.reclassified,
            sequence_changed: self.sequence_changed,
            attributes_changed: self.attributes_changed,
            dirty_nodes,
            removed_nodes,
        }
    }
}

fn kind_word(kind: FactKind) -> &'static str {
    match kind {
        FactKind::Truth => "truth",
        FactKind::Belief => "belief",
    }
}

/// A node's node-level fields only: what its revision covers.
fn node_level(node: &Node) -> Node {
    let mut n = node.clone();
    n.facts.clear();
    n.revision = 0;
    n.notes.clear();
    n
}

/// The current revision of every node and fact, for version checks.
pub fn revisions(spec: &Spec) -> BTreeMap<String, u32> {
    let mut out = BTreeMap::new();
    for node in &spec.nodes {
        out.insert(node.id.clone(), node.revision);
        for fact in &node.facts {
            out.insert(fact.id.clone(), fact.revision);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{NodeType, RevealLevel};
    use crate::spec;
    use std::path::PathBuf;

    fn harbor() -> Spec {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/specs/harbor");
        spec::load(&dir).unwrap().spec
    }

    fn ops(json: &str) -> Vec<Op> {
        serde_json::from_str(json).unwrap()
    }

    fn run(json: &str) -> Result<Applied, OpError> {
        apply(&harbor(), &ops(json))
    }

    fn fact<'a>(spec: &'a Spec, id: &str) -> &'a Fact {
        spec.nodes
            .iter()
            .flat_map(|n| &n.facts)
            .find(|f| f.id == id)
            .unwrap()
    }

    #[test]
    fn node_and_fact_revisions_bump_separately() {
        let before = harbor();
        let a = run(r#"[
            {"op": "set", "target": "p-maraharb.summary", "value": "A courier"},
            {"op": "set", "target": "p-maraharb#f-eyesharb.value", "value": "hazel"},
            {"op": "set", "target": "p-maraharb#f-eyesharb.statement", "value": "Mara's eyes are hazel"}
        ]"#)
        .unwrap();
        let mara = a.spec.node("p-maraharb").unwrap();
        assert_eq!(
            mara.revision,
            before.node("p-maraharb").unwrap().revision + 1
        );
        assert_eq!(mara.summary.as_deref(), Some("A courier"));
        assert_eq!(
            fact(&a.spec, "f-eyesharb").revision,
            2,
            "bumped once per proposal"
        );
        assert_eq!(
            fact(&a.spec, "f-sibharb2").revision,
            1,
            "other facts untouched"
        );
        assert_eq!(
            a.changed,
            BTreeSet::from(["p-maraharb".to_string(), "f-eyesharb".to_string()])
        );
        assert_eq!(a.dirty_nodes, BTreeSet::from(["p-maraharb".to_string()]));
    }

    #[test]
    fn notes_change_the_file_but_not_the_revision() {
        let a = run(r#"[{"op": "set", "target": "p-maraharb.notes", "value": "Counts steps."}]"#)
            .unwrap();
        let mara = a.spec.node("p-maraharb").unwrap();
        assert_eq!(mara.notes, "Counts steps.");
        assert_eq!(mara.revision, harbor().node("p-maraharb").unwrap().revision);
        assert!(a.changed.is_empty());
        assert!(a.dirty_nodes.contains("p-maraharb"));
    }

    #[test]
    fn creates_a_scene_and_places_it_in_the_reading_order() {
        let a = run(r#"[
            {"op": "create", "node": {"id": "s-newscene", "type": "scene", "name": "A new scene",
              "status": "slot", "purpose": "Something happens", "revision": 7,
              "facts": [{"id": "f-newfact2", "kind": "truth", "statement": "x",
                "confidence": "provisional", "review": "unreviewed", "source": "interview"}]},
             "notes": "Idea"},
            {"op": "add_item", "target": "sequence", "value": "s-newscene", "after": "s-harbbbbb"}
        ]"#)
        .unwrap();
        let scene = a.spec.node("s-newscene").unwrap();
        assert_eq!(scene.node_type(), NodeType::Scene);
        assert_eq!(scene.revision, 1, "new nodes start at 1");
        assert_eq!(scene.notes, "Idea");
        assert_eq!(fact(&a.spec, "f-newfact2").revision, 1);
        assert_eq!(a.spec.sequence[2], "s-newscene");
        assert!(a.sequence_changed);
        assert!(a.created.contains("s-newscene") && a.created.contains("f-newfact2"));
    }

    #[test]
    fn list_items_add_and_remove() {
        let a = run(r#"[
            {"op": "add_item", "target": "s-harbaaaa.participants", "value": "p-jnahharb"},
            {"op": "add_item", "target": "p-jnahharb#f-arstharb.reader", "value": {"scene": "s-harbeeee", "level": "hinted"}},
            {"op": "remove_item", "target": "s-harbdddd.participants", "value": "p-jnahharb"},
            {"op": "remove_item", "target": "sequence", "value": "s-harbeeee"},
            {"op": "add_item", "target": "sequence", "value": "s-harbeeee", "before": "s-harbaaaa"}
        ]"#)
        .unwrap();
        let NodeBody::Scene(s) = &a.spec.node("s-harbaaaa").unwrap().body else {
            panic!()
        };
        assert_eq!(s.participants, ["p-jnahharb"]);
        let arrest = fact(&a.spec, "f-arstharb");
        assert_eq!(arrest.reader[0].level, RevealLevel::Hinted);
        assert_eq!(a.spec.sequence[0], "s-harbeeee");
        assert_eq!(a.spec.sequence.len(), 8);
    }

    #[test]
    fn errors_name_the_failing_step() {
        let e = run(r#"[
            {"op": "set", "target": "p-maraharb.summary", "value": "ok"},
            {"op": "remove_item", "target": "s-harbaaaa.participants", "value": "p-jnahharb"}
        ]"#)
        .unwrap_err();
        assert_eq!(e.op, 1);
        assert!(e.message.contains("nothing to remove"), "{}", e.message);

        for (json, needle) in [
            (
                r#"[{"op": "set", "target": "p-maraharb.revision", "value": 9}]"#,
                "can't be set",
            ),
            (
                r#"[{"op": "set", "target": "p-maraharb.status", "value": "x"}]"#,
                "can't change 'status'",
            ),
            (
                r#"[{"op": "set", "target": "s-harbaaaa.status", "value": "finished"}]"#,
                "can't change 'status'",
            ),
            (
                r#"[{"op": "set", "target": "p-nnnnnnnn.name", "value": "x"}]"#,
                "doesn't exist",
            ),
            (
                r#"[{"op": "set", "target": "p-jnahharb#f-eyesharb.value", "value": "x"}]"#,
                "is on 'p-maraharb'",
            ),
            (
                r#"[{"op": "add_fact", "node": "p-jnahharb", "fact": {"id": "f-eyesharb", "kind": "truth", "statement": "x", "confidence": "confirmed", "review": "cleared", "source": "interview"}}]"#,
                "already used",
            ),
            (
                r#"[{"op": "add_item", "target": "sequence", "value": "s-harbaaaa"}]"#,
                "already in the reading order",
            ),
            (
                r#"[{"op": "add_item", "target": "sequence", "value": "p-maraharb"}]"#,
                "isn't a scene",
            ),
            (
                r#"[{"op": "add_item", "target": "s-harbaaaa.purpose", "value": "x"}]"#,
                "isn't a list",
            ),
            (
                r#"[{"op": "merge", "survivor": "p-maraharb", "merged": "l-dcksharb"}]"#,
                "can't be merged",
            ),
            (
                r#"[{"op": "reclassify", "target": "p-jnahharb#f-arstharb", "kind": "belief"}]"#,
                "needs a holder",
            ),
            (
                r#"[{"op": "add_attribute", "attribute": {"key": "sibling_of", "relation": "symmetric", "cardinality": "many"}}]"#,
                "starter list",
            ),
        ] {
            let e = run(json).unwrap_err();
            assert!(e.message.contains(needle), "{json}: {}", e.message);
        }
    }

    #[test]
    fn supersede_redirects_references_to_the_replacement() {
        let a = run(
            r#"[{"op": "supersede", "target": "p-jnahharb#f-jnspyhrb", "replacement":
            {"id": "f-jnspy2nd", "kind": "truth", "statement": "Jonah reports to the harbor master",
             "confidence": "provisional", "review": "unreviewed", "source": "interview"}}]"#,
        )
        .unwrap();
        let NodeBody::Scene(s) = &a.spec.node("s-harbgggg").unwrap().body else {
            panic!()
        };
        assert_eq!(s.reads, ["f-jnspy2nd"]);
        assert_eq!(
            fact(&a.spec, "f-trstharb").contradicts.as_deref(),
            Some("f-jnspy2nd")
        );
        assert!(a.deleted.contains("f-jnspyhrb"));
        assert_eq!(
            a.superseded,
            [("f-jnspyhrb".to_string(), "f-jnspy2nd".to_string())]
        );
        // The scene's node-level field changed, and the belief fact changed.
        assert!(a.changed.contains("s-harbgggg") && a.changed.contains("f-trstharb"));
    }

    #[test]
    fn merge_moves_facts_and_redirects_everything() {
        let a =
            run(r#"[{"op": "merge", "survivor": "p-jnahharb", "merged": "p-dadsharb"}]"#).unwrap();
        assert!(a.spec.node("p-dadsharb").is_none());
        assert!(a.removed_nodes.contains("p-dadsharb"));
        let jonah = a.spec.node("p-jnahharb").unwrap();
        assert!(jonah.aliases.contains(&"The harbor pilot".to_string()));
        assert_eq!(jonah.revision, 2);
        assert_eq!(
            fact(&a.spec, "f-chdrharb").value.as_deref(),
            Some("p-jnahharb")
        );
        let NodeBody::Arc(fog) = &a.spec.node("a-fgsxharb").unwrap().body else {
            panic!()
        };
        assert_eq!(fog.characters, ["p-jnahharb"]);
    }

    #[test]
    fn move_fact_keeps_its_revision_and_bumps_both_nodes() {
        let a = run(r#"[{"op": "move_fact", "fact": "f-arstharb", "to": "p-maraharb"}]"#).unwrap();
        assert_eq!(fact(&a.spec, "f-arstharb").revision, 1);
        assert!(
            a.spec
                .node("p-maraharb")
                .unwrap()
                .facts
                .iter()
                .any(|f| f.id == "f-arstharb")
        );
        assert_eq!(a.spec.node("p-jnahharb").unwrap().revision, 2);
        assert_eq!(a.spec.node("p-maraharb").unwrap().revision, 3);
        assert!(!a.changed.contains("f-arstharb"));
    }

    #[test]
    fn reclassify_and_add_attribute() {
        let a = run(r#"[
            {"op": "reclassify", "target": "f-arstharb", "kind": "belief", "holder": "reader"},
            {"op": "add_attribute", "attribute": {"key": "owes", "relation": "one_way", "cardinality": "per_target"}}
        ]"#)
        .unwrap();
        let f = fact(&a.spec, "f-arstharb");
        assert_eq!(f.kind, FactKind::Belief);
        assert_eq!(f.holder.as_deref(), Some("reader"));
        assert_eq!(f.revision, 2);
        assert_eq!(a.reclassified, ["f-arstharb"]);
        assert!(a.attributes_changed);
        assert!(a.spec.attributes.iter().any(|d| d.key == "owes"));
    }

    #[test]
    fn delete_removes_nodes_and_facts() {
        let a = run(r#"[{"op": "delete", "target": "p-dadsharb"}, {"op": "delete", "target": "p-maraharb#f-eyesharb"}]"#)
            .unwrap();
        assert!(a.deleted.contains("p-dadsharb") && a.deleted.contains("f-eyesharb"));
        assert_eq!(a.removed_nodes, BTreeSet::from(["p-dadsharb".to_string()]));
        // Deleting a fact doesn't bump its node's revision.
        assert!(!a.changed.contains("p-maraharb"));
        assert!(a.dirty_nodes.contains("p-maraharb"));
    }
}
