//! The dependency checker (HANDOFF, Governance): a deterministic check run on
//! every proposal before anything reaches the spec.
//!
//! It applies the proposal to an in-memory copy of the spec (see
//! [`crate::ops`]) and checks the result, in three parts:
//!
//! 1. **Validity:** every reference resolves to the right kind of thing,
//!    facts have the fields their kind needs, and nothing still points at
//!    something the proposal deletes.
//! 2. **Ordering and uniqueness:** a character can't learn a fact before it
//!    becomes true, no scene reads a fact after it has ended, the timeline
//!    has no loops, a fact has at most one reveal, and an entity has at most
//!    one live value for a single-valued key at any in-world moment. Only
//!    what the spec pins down fails; what it can't decide is a soft finding.
//! 3. **Routing:** fan-out (how many nodes read what changed), whether a
//!    confirmed decision is touched, and from those the lane.
//!
//! Only problems the proposal **introduces** count: findings already present
//! in the spec before it are left out, so an old, unrelated problem never
//! blocks a new answer.
//!
//! The verdict: **failed** (a hard finding; revise only), **flagged** (soft
//! findings only; the writer decides: revise, keep anyway, or not sure yet),
//! or **passed**.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use serde::Serialize;

use crate::ids::{self, IdKind};
use crate::index;
use crate::json::Value;
use crate::model::{Confidence, FactKind, NodeBody, NodeType, RevealLevel, TimeAnchor};
use crate::ops::{self, Applied};
use crate::proposal::{Lane, Proposal};
use crate::registry::{Cardinality, Registry, Relation};
use crate::settings::RoutingSettings;
use crate::spec::Spec;
use crate::timeline::{Point, TimeGraph};

/// Hard findings can't be true and block the proposal; soft ones are
/// possible issues the writer decides on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Hard,
    Soft,
}

/// One problem found in a proposal.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Finding {
    pub severity: Severity,
    /// A stable code, for agents and tests (`broken_reference`, …).
    pub code: &'static str,
    /// Plain language, for the interviewer to explain.
    pub message: String,
    /// The IDs the finding rests on.
    pub ids: Vec<String>,
}

impl Finding {
    fn new(severity: Severity, code: &'static str, message: String, ids: &[&str]) -> Self {
        let mut ids: Vec<String> = ids.iter().map(|s| s.to_string()).collect();
        ids.sort();
        ids.dedup();
        Finding {
            severity,
            code,
            message,
            ids,
        }
    }

    fn hard(code: &'static str, message: String, ids: &[&str]) -> Self {
        Finding::new(Severity::Hard, code, message, ids)
    }

    fn soft(code: &'static str, message: String, ids: &[&str]) -> Self {
        Finding::new(Severity::Soft, code, message, ids)
    }

    /// What makes two findings "the same problem" before and after.
    fn key(&self) -> (&'static str, &[String]) {
        (self.code, &self.ids)
    }
}

/// The checker's verdict on one proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// No findings.
    Passed,
    /// Soft findings only: held for the writer to decide.
    Flagged,
    /// At least one hard finding: held; the answer must be revised.
    Failed,
}

/// Everything `check` decides about a proposal.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Report {
    pub status: Status,
    pub lane: Lane,
    /// How many other nodes read what the proposal changes.
    pub fan_out: usize,
    pub touches_confirmed: bool,
    pub findings: Vec<Finding>,
    /// Revisions of everything examined, for the committer's version check.
    #[serde(skip)]
    pub checked: BTreeMap<String, Value>,
}

impl Report {
    /// The `check` section written into the proposal file.
    pub fn to_value(&self) -> Value {
        let mut v = serde_json::to_value(self).unwrap_or(Value::Null);
        if let Some(map) = v.as_object_mut() {
            map.insert(
                "checked_by".into(),
                format!("skaldsaga {}", crate::VERSION).into(),
            );
        }
        v
    }
}

/// The agents that may write proposals, and the lane each starts in
/// (decided in build step 7). The housekeeper never edits the spec.
pub fn default_lane(agent: &str) -> Option<Lane> {
    match agent {
        "interviewer" => Some(Lane::Optimistic),
        "harvester" | "researcher" | "weaver" | "outliner" => Some(Lane::Strict),
        _ => None,
    }
}

/// Key under which `sequence.md` is recorded in revision maps.
pub const SEQUENCE_KEY: &str = "sequence.md";
/// Key under which `attributes.md` is recorded in revision maps.
pub const ATTRIBUTES_KEY: &str = "attributes.md";

/// A short fingerprint of a file without a revision (FNV-1a, 64 bits).
fn fingerprint(text: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}")
}

fn sequence_fingerprint(spec: &Spec) -> String {
    fingerprint(&spec.sequence.join("\n"))
}

fn attributes_fingerprint(spec: &Spec) -> String {
    fingerprint(&serde_json::to_string(&spec.attributes).unwrap_or_default())
}

/// Checks one proposal against the spec.
pub fn check(spec: &Spec, proposal: &Proposal, routing: &RoutingSettings) -> Report {
    let mut findings = Vec::new();
    let agent_lane = default_lane(&proposal.agent);
    match (proposal.agent.as_str(), agent_lane) {
        ("housekeeper", _) => findings.push(Finding::hard(
            "housekeeper_edit",
            "the housekeeper never changes the spec; it raises flags instead".into(),
            &[],
        )),
        (agent, None) => findings.push(Finding::soft(
            "unknown_agent",
            format!("'{agent}' isn't a known agent, so this goes through the strict checks"),
            &[],
        )),
        _ => {}
    }
    if !ids::is_kind(&proposal.id, IdKind::Proposal) {
        findings.push(Finding::hard(
            "bad_id",
            format!("'{}' isn't a valid proposal ID", proposal.id),
            &[],
        ));
    }
    findings.extend(stale(spec, proposal));

    let mut lane = agent_lane
        .unwrap_or(Lane::Strict)
        .max(proposal.lane.unwrap_or(Lane::Optimistic));

    let applied = match ops::apply(spec, &proposal.ops) {
        Ok(a) => a,
        Err(e) => {
            let name = proposal.ops.get(e.op).map_or("?", |o| o.name());
            let ids: Vec<&str> = e.ids.iter().map(String::as_str).collect();
            findings.push(Finding::hard(
                "op_failed",
                format!("step {} ({name}): {}", e.op + 1, e.message),
                &ids,
            ));
            return finish(findings, lane, 0, false, BTreeMap::new());
        }
    };

    // Only what the proposal introduces counts.
    let registry_before = Registry::new(&spec.attributes);
    let registry_after = Registry::new(&applied.spec.attributes);
    let before: HashSet<_> = problems(spec, &registry_before, &BTreeSet::new())
        .into_iter()
        .map(|f| (f.code, f.ids))
        .collect();
    for f in problems(&applied.spec, &registry_after, &applied.deleted) {
        // A delete that leaves references behind is new by definition.
        let same_as_before = before.contains(&(f.code, f.ids.clone()))
            || (f.code == "delete_blocked"
                && before.contains(&("broken_reference", f.ids.clone())));
        if !same_as_before {
            findings.push(f);
        }
    }
    findings.extend(change_notes(spec, &applied));

    // Routing.
    let readers = index::readers(spec);
    let touched = applied.touched();
    let mut fan: BTreeSet<&str> = BTreeSet::new();
    for id in &touched {
        for (from, _) in readers.get(id.as_str()).into_iter().flatten() {
            if from != index::SEQUENCE_FROM && !touched.contains(from) {
                fan.insert(from);
            }
        }
    }
    let fan_out = fan.len();
    let touches_confirmed = touches_confirmed(spec, &applied, &touched);
    if touches_confirmed || fan_out >= routing.strict_fan_out {
        lane = Lane::Strict;
    }

    // What this verdict rests on.
    let current = ops::revisions(spec);
    let mut checked: BTreeMap<String, Value> = BTreeMap::new();
    for id in touched
        .iter()
        .map(String::as_str)
        .chain(fan.iter().copied())
    {
        if let Some(rev) = current.get(id) {
            checked.insert(id.to_string(), Value::from(*rev));
        }
    }
    checked.insert(SEQUENCE_KEY.into(), sequence_fingerprint(spec).into());
    checked.insert(ATTRIBUTES_KEY.into(), attributes_fingerprint(spec).into());

    finish(findings, lane, fan_out, touches_confirmed, checked)
}

fn finish(
    mut findings: Vec<Finding>,
    lane: Lane,
    fan_out: usize,
    touches_confirmed: bool,
    checked: BTreeMap<String, Value>,
) -> Report {
    findings.sort();
    findings.dedup_by(|a, b| a.key() == b.key());
    let status = if findings.iter().any(|f| f.severity == Severity::Hard) {
        Status::Failed
    } else if findings.is_empty() {
        Status::Passed
    } else {
        Status::Flagged
    };
    Report {
        status,
        lane,
        fan_out,
        touches_confirmed,
        findings,
        checked,
    }
}

/// The current revision (or content fingerprint, for `sequence.md` and
/// `attributes.md`) recorded under `key` in a revision map; `None` when the
/// node or fact no longer exists.
pub fn current_value(spec: &Spec, key: &str) -> Option<Value> {
    match key {
        SEQUENCE_KEY => Some(sequence_fingerprint(spec).into()),
        ATTRIBUTES_KEY => Some(attributes_fingerprint(spec).into()),
        _ => {
            if let Some(n) = spec.node(key) {
                return Some(Value::from(n.revision));
            }
            spec.nodes
                .iter()
                .flat_map(|n| &n.facts)
                .find(|f| f.id == key)
                .map(|f| Value::from(f.revision))
        }
    }
}

/// The author's view must still be current: everything it wrote or read
/// must be at the revision it recorded.
fn stale(spec: &Spec, proposal: &Proposal) -> Vec<Finding> {
    let mut out = Vec::new();
    for (id, recorded) in proposal
        .revisions
        .writes
        .iter()
        .chain(&proposal.revisions.read)
    {
        match current_value(spec, id) {
            None => out.push(Finding::hard(
                "stale",
                format!("'{id}' no longer exists, so this answer needs another look"),
                &[id],
            )),
            Some(now) if now != *recorded => out.push(Finding::hard(
                "stale",
                format!("'{id}' changed after this answer was formed, so it needs another look"),
                &[id],
            )),
            Some(_) => {}
        }
    }
    out
}

/// True when the proposal writes a confirmed shape decision, or the pivot
/// belief of one.
fn touches_confirmed(before: &Spec, applied: &Applied, touched: &BTreeSet<String>) -> bool {
    let confirmed = |spec: &Spec, id: &str| {
        matches!(spec.node(id).map(|n| &n.body),
            Some(NodeBody::Decision(d)) if d.confidence == Confidence::Confirmed)
    };
    let pivots: HashSet<&str> = before
        .nodes
        .iter()
        .filter_map(|n| match &n.body {
            NodeBody::Decision(d) if d.confidence == Confidence::Confirmed => d.pivot.as_deref(),
            _ => None,
        })
        .collect();
    touched.iter().any(|id| {
        confirmed(before, id) || confirmed(&applied.spec, id) || pivots.contains(id.as_str())
    })
}

/// Soft notes about what a proposal changes for others: scenes that read a
/// fact the writer changed their mind about, or one that switched between
/// truth and belief.
fn change_notes(before: &Spec, applied: &Applied) -> Vec<Finding> {
    let readers = index::readers(before);
    let mut out = Vec::new();
    let reading_scenes = |fact: &str| -> Vec<String> {
        readers
            .get(fact)
            .into_iter()
            .flatten()
            .filter(|(_, via)| via == "reads")
            .map(|(from, _)| from.clone())
            .collect()
    };
    for (old, new) in &applied.superseded {
        for scene in reading_scenes(old) {
            out.push(Finding::soft(
                "reads_superseded",
                format!(
                    "{} relies on {}, which this answer replaces; check the scene still fits",
                    label(before, &scene),
                    label(before, old)
                ),
                &[&scene, old, new],
            ));
        }
    }
    for fact in &applied.reclassified {
        for scene in reading_scenes(fact) {
            out.push(Finding::soft(
                "reads_reclassified",
                format!(
                    "{} relies on {}, which this answer turns from true to believed (or back); check the scene still fits",
                    label(before, &scene),
                    label(before, fact)
                ),
                &[&scene, fact],
            ));
        }
    }
    out
}

/// A readable name for an ID: the node's name or the fact's statement.
fn label(spec: &Spec, id: &str) -> String {
    if let Some(n) = spec.node(id) {
        return format!("'{}' ({id})", n.name);
    }
    for n in &spec.nodes {
        if let Some(f) = n.facts.iter().find(|f| f.id == id) {
            return format!("\"{}\" ({id})", f.statement);
        }
    }
    format!("'{id}'")
}

// ---------------------------------------------------------------------------
// Problems in a spec (computed before and after; only new ones count)

/// Every hard problem the spec has right now, as `complete` reports them:
/// broken references, impossible facts, loops and violated orderings.
pub fn spec_problems(spec: &Spec) -> Vec<Finding> {
    let registry = Registry::new(&spec.attributes);
    problems(spec, &registry, &BTreeSet::new())
        .into_iter()
        .filter(|f| f.severity == Severity::Hard)
        .collect()
}

fn problems(spec: &Spec, registry: &Registry, deleted: &BTreeSet<String>) -> Vec<Finding> {
    let mut out = Vec::new();
    references(spec, deleted, &mut out);
    fact_shapes(spec, &mut out);
    let graph = TimeGraph::build(spec);
    ordering(spec, &graph, &mut out);
    uniqueness(spec, registry, &graph, &mut out);
    out
}

/// What kind of thing a reference may point at, by the field it's in.
fn expected(via: &str) -> Option<&'static [IdKind]> {
    use IdKind::*;
    let field = via.rsplit('.').next().unwrap_or(via);
    Some(match field {
        "pov" | "participants" | "characters" | "viewpoints" | "holder" => &[Person],
        "location" => &[Place],
        "coverage" | "threads" => &[Thread],
        "arc" => &[Arc],
        "decisions" => &[Decision],
        "reads" | "contradicts" | "ended_by" | "pivot" => &[Fact],
        "evidence" => &[Scene, Fact],
        "hinted" | "revealed" | "learns_in" | "sequence" => &[Scene],
        _ if via.starts_with("landmark:") || via.starts_with("beat:") => &[Scene],
        // Time anchors are checked by variant below; links and relation
        // values may name any node.
        _ => return None,
    })
}

fn kind_word(kinds: &[IdKind]) -> String {
    let word = |k: &IdKind| match k {
        IdKind::Person => "a person",
        IdKind::Place => "a place",
        IdKind::Group => "a group",
        IdKind::Item => "an item",
        IdKind::Scene => "a scene",
        IdKind::Decision => "a shape decision",
        IdKind::Thread => "a thread",
        IdKind::Style => "the style node",
        IdKind::Arc => "a story arc",
        IdKind::Fact => "a fact",
        IdKind::Edition => "an edition",
        IdKind::Proposal => "a proposal",
        IdKind::WriterDecision => "a writer decision",
    };
    kinds.iter().map(word).collect::<Vec<_>>().join(" or ")
}

fn references(spec: &Spec, deleted: &BTreeSet<String>, out: &mut Vec<Finding>) {
    let facts: HashMap<&str, (FactKind, &str)> = spec
        .nodes
        .iter()
        .flat_map(|n| {
            n.facts
                .iter()
                .map(move |f| (f.id.as_str(), (f.kind, n.id.as_str())))
        })
        .collect();
    let kind_of = |id: &str| -> Option<IdKind> {
        if facts.contains_key(id) {
            Some(IdKind::Fact)
        } else {
            spec.node(id).map(|n| n.node_type().id_kind())
        }
    };
    for (target, from, via) in index::references(spec) {
        let Some(kind) = kind_of(&target) else {
            let (code, message) = if deleted.contains(&target) {
                (
                    "delete_blocked",
                    format!(
                        "{target} is still referenced by {} ({via}); change that first",
                        label(spec, &from)
                    ),
                )
            } else {
                (
                    "broken_reference",
                    format!(
                        "{} points at '{target}' ({via}), which doesn't exist",
                        label(spec, &from)
                    ),
                )
            };
            out.push(Finding::hard(code, message, &[&from, &target]));
            continue;
        };
        if let Some(allowed) = expected(&via)
            && !allowed.contains(&kind)
        {
            out.push(Finding::hard(
                "wrong_kind",
                format!(
                    "{} names {} as its {via}, but that should be {}",
                    label(spec, &from),
                    label(spec, &target),
                    kind_word(allowed)
                ),
                &[&from, &target],
            ));
        }
    }
    // Time anchors: `in` names a scene and `at` a fact.
    let mut anchor = |from: &str, field: &str, a: &Option<TimeAnchor>| {
        let bad = match a {
            Some(TimeAnchor::In(i)) => (kind_of(&i.scene).is_some_and(|k| k != IdKind::Scene))
                .then_some((i.scene.as_str(), "a scene")),
            Some(TimeAnchor::At(t)) => (kind_of(&t.at).is_some_and(|k| k != IdKind::Fact))
                .then_some((t.at.as_str(), "a fact")),
            Some(TimeAnchor::After(t)) => (kind_of(&t.after)
                .is_some_and(|k| k != IdKind::Fact && k != IdKind::Scene))
            .then_some((t.after.as_str(), "a scene or a fact")),
            _ => None,
        };
        if let Some((target, want)) = bad {
            out.push(Finding::hard(
                "wrong_kind",
                format!(
                    "{} anchors its {field} to {}, but that should be {want}",
                    label(spec, from),
                    label(spec, target)
                ),
                &[from, target],
            ));
        }
    };
    for node in &spec.nodes {
        if let NodeBody::Scene(s) = &node.body {
            anchor(&node.id, "time", &s.when);
        }
        for f in &node.facts {
            anchor(&f.id, "start", &f.true_from);
            anchor(&f.id, "end", &f.ends);
        }
    }
    // A belief contradicts a truth.
    for node in &spec.nodes {
        for f in &node.facts {
            if let Some(c) = &f.contradicts
                && let Some((FactKind::Belief, _)) = facts.get(c.as_str())
            {
                out.push(Finding::hard(
                    "contradicts_belief",
                    format!(
                        "{} contradicts {}, but that's a belief; a belief contradicts a truth",
                        label(spec, &f.id),
                        label(spec, c)
                    ),
                    &[&f.id, c],
                ));
            }
        }
    }
}

/// Fields each kind of fact needs or can't have.
fn fact_shapes(spec: &Spec, out: &mut Vec<Finding>) {
    for node in &spec.nodes {
        for f in &node.facts {
            let mut bad = |msg: &str| {
                out.push(Finding::hard(
                    "fact_shape",
                    format!("{} {msg}", label(spec, &f.id)),
                    &[&f.id],
                ));
            };
            match f.kind {
                FactKind::Belief => match &f.holder {
                    None => bad("is a belief but doesn't say who holds it"),
                    Some(h) if h != "reader" && !ids::is_kind(h, IdKind::Person) => {
                        bad("is held by something that isn't a person or the reader")
                    }
                    _ => {}
                },
                FactKind::Truth => {
                    if f.holder.is_some() || f.contradicts.is_some() {
                        bad("is a truth, so it has no holder and contradicts nothing");
                    }
                }
            }
            if f.confidence != Confidence::Unresolved
                && (f.question.is_some() || !f.options.is_empty())
            {
                bad("has an open question or options but isn't marked unresolved");
            }
            if f.ended_by.is_some() && f.ends.is_none() {
                bad("says what replaced it but not when it ended");
            }
        }
    }
}

fn ordering(spec: &Spec, graph: &TimeGraph, out: &mut Vec<Finding>) {
    for group in graph.loops() {
        let ids: Vec<&str> = group.iter().map(String::as_str).collect();
        out.push(Finding::hard(
            "timeline_loop",
            format!(
                "the in-world times of {} loop back on themselves: something would happen before its own cause",
                ids.iter()
                    .map(|id| label(spec, id))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            &ids,
        ));
    }

    let position: HashMap<&str, usize> = spec
        .sequence
        .iter()
        .enumerate()
        .map(|(i, s)| (s.as_str(), i))
        .collect();

    for node in &spec.nodes {
        for f in &node.facts {
            let start = Point::Start(f.id.clone());
            for c in &f.characters {
                if graph.before(&Point::Scene(c.learns_in.clone()), &start) {
                    out.push(Finding::hard(
                        "learns_before_true",
                        format!(
                            "{} learns {} in {}, before it becomes true",
                            label(spec, &c.who),
                            label(spec, &f.id),
                            label(spec, &c.learns_in)
                        ),
                        &[&f.id, &c.who, &c.learns_in],
                    ));
                }
            }
            // Alternative scenes (outside the reading order) may reveal a
            // fact too, since an edition can swap one in; the reading order
            // itself reveals each fact at most once.
            let reveals: Vec<&str> = f
                .reader
                .iter()
                .filter(|m| m.level == RevealLevel::Revealed)
                .map(|m| m.scene.as_str())
                .filter(|s| position.contains_key(s))
                .collect();
            if reveals.len() > 1 {
                let mut ids = vec![f.id.as_str()];
                ids.extend(&reveals);
                out.push(Finding::hard(
                    "two_reveals",
                    format!(
                        "{} is revealed to the reader more than once in the reading order; a fact has at most one reveal",
                        label(spec, &f.id)
                    ),
                    &ids,
                ));
            }
            // A hint after the reveal (in reading order) no longer hints.
            if let Some(reveal_at) = reveals.iter().filter_map(|s| position.get(s)).min() {
                for m in &f.reader {
                    if m.level == RevealLevel::Hinted
                        && position
                            .get(m.scene.as_str())
                            .is_some_and(|p| p > reveal_at)
                    {
                        out.push(Finding::soft(
                            "hint_after_reveal",
                            format!(
                                "{} hints at {} after the reader has already been told it",
                                label(spec, &m.scene),
                                label(spec, &f.id)
                            ),
                            &[&f.id, &m.scene],
                        ));
                    }
                }
            }
        }
    }

    // No scene reads a fact after it has ended.
    let ends: HashMap<&str, bool> = spec
        .nodes
        .iter()
        .flat_map(|n| n.facts.iter().map(|f| (f.id.as_str(), f.ends.is_some())))
        .collect();
    for node in &spec.nodes {
        let NodeBody::Scene(s) = &node.body else {
            continue;
        };
        for fact in &s.reads {
            if ends.get(fact.as_str()) == Some(&true)
                && graph.before(&Point::End(fact.clone()), &Point::Scene(node.id.clone()))
            {
                out.push(Finding::hard(
                    "reads_after_end",
                    format!(
                        "{} relies on {}, but that stopped being true before the scene",
                        label(spec, &node.id),
                        label(spec, fact)
                    ),
                    &[&node.id, fact],
                ));
            }
        }
    }
}

/// (entity, key, target or empty) to the facts in that group, each with
/// whether it has an end.
type Groups<'a> = BTreeMap<(String, String, String), Vec<(&'a str, bool)>>;

/// One live value per entity for single-valued keys, and per entity and
/// target for per-target keys (HANDOFF, Node format: cardinality).
fn uniqueness(spec: &Spec, registry: &Registry, graph: &TimeGraph, out: &mut Vec<Finding>) {
    let mut groups: Groups = BTreeMap::new();
    for node in &spec.nodes {
        for f in &node.facts {
            if f.kind != FactKind::Truth || f.confidence == Confidence::Unresolved {
                continue;
            }
            let Some(key) = &f.attribute else { continue };
            let Some(found) = registry.lookup(key) else {
                continue;
            };
            let def = found.def;
            let ends = f.ends.is_some();
            // Who the fact is about, from each side that has a rule.
            let mut subjects: Vec<(String, Option<String>)> = Vec::new();
            match def.relation {
                Relation::Property | Relation::Related => {
                    subjects.push((node.id.clone(), None));
                }
                Relation::OneWay => subjects.push((node.id.clone(), f.value.clone())),
                Relation::Symmetric => {
                    subjects.push((node.id.clone(), f.value.clone()));
                    if let Some(v) = &f.value {
                        subjects.push((v.clone(), Some(node.id.clone())));
                    }
                }
                Relation::Inverse => {
                    // Normalize to the registered direction.
                    if found.is_inverse_side {
                        if let Some(v) = &f.value {
                            subjects.push((v.clone(), Some(node.id.clone())));
                        }
                    } else {
                        subjects.push((node.id.clone(), f.value.clone()));
                    }
                }
            }
            for (subject, target) in subjects {
                let Some(subject_type) = subject_type(spec, &subject) else {
                    continue;
                };
                let rule = def.cardinality.for_type(subject_type);
                let group_target = match rule {
                    Some(Cardinality::Single) => String::new(),
                    Some(Cardinality::PerTarget) => target.unwrap_or_default(),
                    Some(Cardinality::Many) | None => continue,
                };
                groups
                    .entry((subject, def.key.clone(), group_target))
                    .or_default()
                    .push((f.id.as_str(), ends));
            }
        }
    }
    for ((subject, key, _), facts) in groups {
        for (i, (a, a_ends)) in facts.iter().enumerate() {
            for (b, b_ends) in &facts[i + 1..] {
                let a_first = *a_ends
                    && graph.at_or_before(&Point::End(a.to_string()), &Point::Start(b.to_string()));
                let b_first = *b_ends
                    && graph.at_or_before(&Point::End(b.to_string()), &Point::Start(a.to_string()));
                if a_first || b_first {
                    continue;
                }
                let ids = [subject.as_str(), a, b];
                if !a_ends && !b_ends {
                    out.push(Finding::hard(
                        "two_live_values",
                        format!(
                            "{} would have two '{key}' values at once: {} and {}; one needs to end before the other starts",
                            label(spec, &subject),
                            label(spec, a),
                            label(spec, b)
                        ),
                        &ids,
                    ));
                } else {
                    out.push(Finding::soft(
                        "maybe_two_live_values",
                        format!(
                            "{} may have two '{key}' values at once ({} and {}); the timeline doesn't say which ends first",
                            label(spec, &subject),
                            label(spec, a),
                            label(spec, b)
                        ),
                        &ids,
                    ));
                }
            }
        }
    }
}

fn subject_type(spec: &Spec, id: &str) -> Option<NodeType> {
    spec.node(id).map(|n| n.node_type())
}

// ---------------------------------------------------------------------------
// The inbox

/// What happened to one file in the inbox.
#[derive(Debug, Clone, PartialEq)]
pub enum InboxEntry {
    /// Checked; the verdict was written back into the file.
    Checked { id: String, report: Report },
    /// The file couldn't be read as a proposal; it was left untouched.
    Unreadable { file: String, message: String },
}

/// What `check_inbox` did.
#[derive(Debug, Clone, PartialEq)]
pub struct InboxReport {
    /// Files in the spec that couldn't be loaded (they're missing from the
    /// spec the proposals were checked against).
    pub spec_problems: usize,
    pub entries: Vec<InboxEntry>,
}

/// Checks the proposals in a project's inbox (all of them, or only the IDs
/// given) against its spec, and writes each verdict back into its file,
/// with the revisions it checked. Nothing in the spec changes.
pub fn check_inbox(
    project: &crate::project::Project,
    only: &[String],
) -> Result<InboxReport, crate::fsio::FsError> {
    use crate::fsio;
    let loaded = crate::spec::load(&project.spec_dir())?;
    let routing = RoutingSettings::load(project)?;
    let dir = project.inbox_dir();
    let mut entries = Vec::new();
    for file in fsio::list_dir(&dir)? {
        let Some(stem) = file.name.strip_suffix(".json") else {
            continue;
        };
        if file.is_dir || file.name.starts_with('.') {
            continue;
        }
        if !only.is_empty() && !only.iter().any(|id| id == stem) {
            continue;
        }
        let path = dir.join(&file.name);
        let text = fsio::read_optional(&path)?.unwrap_or_default();
        let mut proposal = match crate::proposal::parse(&text) {
            Ok(p) => p,
            Err(message) => {
                entries.push(InboxEntry::Unreadable {
                    file: file.name.clone(),
                    message,
                });
                continue;
            }
        };
        if proposal.id != stem {
            entries.push(InboxEntry::Unreadable {
                file: file.name.clone(),
                message: format!(
                    "the file name should match the proposal's ID ('{}.json')",
                    proposal.id
                ),
            });
            continue;
        }
        let report = check(&loaded.spec, &proposal, &routing);
        proposal.check = Some(report.to_value());
        proposal.revisions.checked = report.checked.clone();
        fsio::write_atomic(&path, &crate::proposal::render(&proposal))?;
        entries.push(InboxEntry::Checked {
            id: proposal.id,
            report,
        });
    }
    Ok(InboxReport {
        spec_problems: loaded.problems.len(),
        entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proposal;
    use crate::spec;
    use std::path::PathBuf;

    fn harbor() -> Spec {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/specs/harbor");
        spec::load(&dir).unwrap().spec
    }

    fn proposal_of(agent: &str, ops: &str) -> Proposal {
        proposal::parse(&format!(
            r#"{{"id": "pr-testtest", "agent": "{agent}", "ops": {ops}}}"#
        ))
        .unwrap()
    }

    fn run(ops: &str) -> Report {
        check(
            &harbor(),
            &proposal_of("interviewer", ops),
            &RoutingSettings::default(),
        )
    }

    fn codes(r: &Report) -> Vec<&'static str> {
        r.findings.iter().map(|f| f.code).collect()
    }

    #[test]
    fn a_clean_answer_passes_in_the_optimistic_lane() {
        let r = run(r#"[{"op": "set", "target": "p-dadsharb.summary", "value": "Ada's father"}]"#);
        assert_eq!(codes(&r), Vec::<&str>::new());
        assert_eq!(r.status, Status::Passed);
        assert_eq!(r.lane, Lane::Optimistic);
        assert!(!r.touches_confirmed);
        // The fixture's own problems (a stray thread, a broken link) don't count.
        assert!(r.checked.contains_key(SEQUENCE_KEY));
        assert_eq!(r.checked["p-dadsharb"], 1);
        let v = r.to_value();
        assert_eq!(v["status"], "passed");
        assert!(v.get("checked").is_none());
        assert!(v["checked_by"].as_str().unwrap().starts_with("skaldsaga "));
    }

    #[test]
    fn deleting_something_referenced_is_blocked() {
        let r = run(r#"[{"op": "delete", "target": "p-dadsharb"}]"#);
        assert_eq!(r.status, Status::Failed);
        assert!(
            codes(&r).iter().all(|c| *c == "delete_blocked"),
            "{:?}",
            r.findings
        );
        assert_eq!(
            r.findings.len(),
            2,
            "the fog arc and Ada's fact: {:?}",
            r.findings
        );

        // A scene still in the reading order is referenced by it.
        let r = run(r#"[{"op": "delete", "target": "s-harbeeee"}]"#);
        assert_eq!(codes(&r), ["delete_blocked"]);
        let r = run(
            r#"[{"op": "remove_item", "target": "sequence", "value": "s-harbeeee"},
                        {"op": "delete", "target": "s-harbeeee"}]"#,
        );
        assert_eq!(r.status, Status::Passed, "{:?}", r.findings);
    }

    #[test]
    fn references_must_exist_and_be_the_right_kind() {
        let r = run(
            r#"[{"op": "set", "target": "s-harbaaaa.pov", "value": "l-dcksharb"},
                        {"op": "set", "target": "s-harbaaaa.location", "value": "l-nnnnnnnn"}]"#,
        );
        let mut c = codes(&r);
        c.sort();
        assert_eq!(c, ["broken_reference", "wrong_kind"]);
        let r = run(
            r#"[{"op": "set", "target": "p-maraharb#f-trstharb.true_from", "value": {"in": "p-jnahharb"}}]"#,
        );
        assert_eq!(codes(&r), ["wrong_kind"]);
    }

    #[test]
    fn fact_shapes() {
        let r = run(r#"[{"op": "set", "target": "p-maraharb#f-trstharb.holder", "value": null}]"#);
        assert_eq!(codes(&r), ["fact_shape"]);
        let r = run(
            r#"[{"op": "set", "target": "p-maraharb#f-trstharb.contradicts", "value": "f-trstharb"}]"#,
        );
        assert_eq!(codes(&r), ["contradicts_belief"]);
    }

    #[test]
    fn a_character_cant_learn_a_fact_before_it_is_true() {
        let r = run(r#"[
            {"op": "set", "target": "s-harbdddd.when", "value": {"after": "s-harbcccc"}},
            {"op": "add_item", "target": "p-jnahharb#f-arstharb.characters",
             "value": {"who": "p-maraharb", "learns_in": "s-harbcccc", "how": "told"}}
        ]"#);
        assert_eq!(codes(&r), ["learns_before_true"]);
        // Without the pinned order there's nothing to fail.
        let r = run(
            r#"[{"op": "add_item", "target": "p-jnahharb#f-arstharb.characters",
             "value": {"who": "p-maraharb", "learns_in": "s-harbcccc", "how": "told"}}]"#,
        );
        assert_eq!(r.status, Status::Passed);
    }

    #[test]
    fn no_scene_reads_a_fact_after_it_ended() {
        // Mara's trust ends in s-harbffff; s-harbgggg happens after it.
        let r = run(r#"[{"op": "add_item", "target": "s-harbgggg.reads", "value": "f-trstharb"}]"#);
        assert_eq!(codes(&r), ["reads_after_end"]);
        let r = run(r#"[{"op": "add_item", "target": "s-harbffff.reads", "value": "f-trstharb"}]"#);
        assert_eq!(r.status, Status::Passed, "the same scene is fine");
    }

    #[test]
    fn the_timeline_cant_loop() {
        let r = run(
            r#"[{"op": "set", "target": "s-harbffff.when", "value": {"after": "s-harbgggg"}}]"#,
        );
        assert_eq!(codes(&r), ["timeline_loop"]);
    }

    #[test]
    fn reveals_and_hints() {
        // The alternative scene's reveal doesn't count; a second one in the
        // reading order does.
        let r = run(
            r#"[{"op": "add_item", "target": "p-jnahharb#f-jnspyhrb.reader",
            "value": {"scene": "s-harbhhhh", "level": "revealed"}}]"#,
        );
        assert_eq!(codes(&r), ["two_reveals"]);
        let r = run(
            r#"[{"op": "add_item", "target": "p-jnahharb#f-jnspyhrb.reader",
            "value": {"scene": "s-harbhhhh", "level": "hinted"}}]"#,
        );
        assert_eq!(codes(&r), ["hint_after_reveal"]);
        assert_eq!(r.status, Status::Flagged);
    }

    const SPOUSE_A: &str = r#"{"op": "add_fact", "node": "p-maraharb", "fact": {"id": "f-spsearrr",
        "kind": "truth", "statement": "Mara is married to Jonah", "attribute": "spouse_of",
        "value": "p-jnahharb", "confidence": "provisional", "review": "unreviewed", "source": "interview"}}"#;

    #[test]
    fn single_valued_keys_have_one_live_value() {
        let second = r#"{"op": "add_fact", "node": "p-sisharb2", "fact": {"id": "f-spsebbbb",
            "kind": "truth", "statement": "Ada is married to Mara", "attribute": "spouse_of",
            "value": "p-maraharb", "confidence": "provisional", "review": "unreviewed", "source": "interview"}}"#;
        let r = run(&format!("[{SPOUSE_A}, {second}]"));
        assert_eq!(codes(&r), ["two_live_values"], "{:?}", r.findings);
        assert!(r.findings[0].ids.contains(&"p-maraharb".to_string()));
    }

    #[test]
    fn overlap_the_timeline_cant_decide_is_soft() {
        let ended = r#"{"op": "set", "target": "p-maraharb#f-spsearrr.ends", "value": {"in": "s-harbcccc"}}"#;
        let second = r#"{"op": "add_fact", "node": "p-maraharb", "fact": {"id": "f-spsebbbb",
            "kind": "truth", "statement": "Mara remarries, to the pilot", "attribute": "spouse_of",
            "value": "p-dadsharb", "true_from": {"in": "s-harbdddd"},
            "confidence": "provisional", "review": "unreviewed", "source": "interview"}}"#;
        let r = run(&format!("[{SPOUSE_A}, {ended}, {second}]"));
        assert_eq!(codes(&r), ["maybe_two_live_values"]);
        // Pin s-harbdddd after s-harbcccc and the first marriage ends first.
        let pin = r#"{"op": "set", "target": "s-harbdddd.when", "value": {"after": "s-harbcccc"}}"#;
        let r = run(&format!("[{SPOUSE_A}, {ended}, {second}, {pin}]"));
        assert_eq!(r.status, Status::Passed, "{:?}", r.findings);
    }

    #[test]
    fn per_target_keys_are_unique_per_target() {
        let bond = |id: &str| {
            format!(
                r#"{{"op": "add_fact", "node": "p-maraharb", "fact": {{"id": "{id}", "kind": "truth",
                "statement": "Mara trusts Jonah", "attribute": "bond", "value": "p-jnahharb",
                "confidence": "provisional", "review": "unreviewed", "source": "interview"}}}}"#
            )
        };
        let r = run(&format!("[{}, {}]", bond("f-bndaaaaa"), bond("f-bndbbbbb")));
        assert_eq!(codes(&r), ["two_live_values"]);
        let r = run(&format!("[{}]", bond("f-bndaaaaa")));
        assert_eq!(r.status, Status::Passed);
    }

    #[test]
    fn stale_views_need_another_look() {
        let mut p = proposal_of(
            "interviewer",
            r#"[{"op": "set", "target": "p-maraharb.summary", "value": "x"}]"#,
        );
        p.revisions
            .writes
            .insert("p-maraharb".into(), Value::from(1));
        p.revisions
            .read
            .insert(SEQUENCE_KEY.into(), Value::from("0000"));
        p.revisions.read.insert("p-gonegone".into(), Value::from(1));
        let r = check(&harbor(), &p, &RoutingSettings::default());
        assert_eq!(codes(&r), ["stale", "stale", "stale"]);
        // Up to date passes.
        let spec = harbor();
        let mut p = proposal_of(
            "interviewer",
            r#"[{"op": "set", "target": "p-dadsharb.summary", "value": "x"}]"#,
        );
        p.revisions
            .writes
            .insert("p-dadsharb".into(), Value::from(1));
        p.revisions
            .read
            .insert(SEQUENCE_KEY.into(), sequence_fingerprint(&spec).into());
        assert_eq!(
            check(&spec, &p, &RoutingSettings::default()).status,
            Status::Passed
        );
    }

    #[test]
    fn lanes_follow_the_agent_fan_out_and_confirmed_decisions() {
        let rename = r#"[{"op": "set", "target": "p-maraharb.name", "value": "Mara Vane"}]"#;
        let r = run(rename);
        assert!(r.fan_out >= 10, "{}", r.fan_out);
        assert_eq!(r.lane, Lane::Strict);
        let relaxed = RoutingSettings { strict_fan_out: 50 };
        let r = check(&harbor(), &proposal_of("interviewer", rename), &relaxed);
        assert_eq!(r.lane, Lane::Optimistic);

        let r = run(r#"[{"op": "set", "target": "d-chngharb.confidence", "value": "confirmed"}]"#);
        assert!(r.touches_confirmed);
        assert_eq!(r.lane, Lane::Strict);

        // Touching the pivot belief of a confirmed decision.
        let mut spec = harbor();
        let d = spec
            .nodes
            .iter_mut()
            .find(|n| n.id == "d-chngharb")
            .unwrap();
        let NodeBody::Decision(dec) = &mut d.body else {
            panic!()
        };
        dec.confidence = Confidence::Confirmed;
        let p = proposal_of(
            "interviewer",
            r#"[{"op": "set", "target": "p-maraharb#f-trstharb.statement", "value": "Jonah is loyal"}]"#,
        );
        let r = check(&spec, &p, &RoutingSettings::default());
        assert!(r.touches_confirmed);
        assert_eq!(r.lane, Lane::Strict);

        let small = r#"[{"op": "set", "target": "p-dadsharb.summary", "value": "x"}]"#;
        let r = check(
            &harbor(),
            &proposal_of("weaver", small),
            &RoutingSettings::default(),
        );
        assert_eq!(r.lane, Lane::Strict);
        let mut p = proposal_of("interviewer", small);
        p.lane = Some(Lane::Strict);
        assert_eq!(
            check(&harbor(), &p, &RoutingSettings::default()).lane,
            Lane::Strict
        );

        let r = check(
            &harbor(),
            &proposal_of("housekeeper", small),
            &RoutingSettings::default(),
        );
        assert_eq!(codes(&r), ["housekeeper_edit"]);
        let r = check(
            &harbor(),
            &proposal_of("muse", small),
            &RoutingSettings::default(),
        );
        assert_eq!((r.status, r.lane), (Status::Flagged, Lane::Strict));
    }

    #[test]
    fn changes_others_rely_on_are_flagged() {
        let r = run(
            r#"[{"op": "supersede", "target": "p-jnahharb#f-jnspyhrb", "replacement":
            {"id": "f-jnspy2nd", "kind": "truth", "statement": "Jonah reports to the harbor master",
             "confidence": "provisional", "review": "unreviewed", "source": "interview"}}]"#,
        );
        assert_eq!(codes(&r), ["reads_superseded"]);
        assert_eq!(r.status, Status::Flagged);
        // Turning Jonah's lie into a belief also leaves Mara's belief
        // contradicting another belief, which can't be.
        let r = run(
            r#"[{"op": "reclassify", "target": "f-jnspyhrb", "kind": "belief", "holder": "p-maraharb"}]"#,
        );
        assert_eq!(codes(&r), ["contradicts_belief", "reads_reclassified"]);
        let r = run(
            r#"[{"op": "set", "target": "p-maraharb#f-trstharb.contradicts", "value": null},
            {"op": "reclassify", "target": "f-jnspyhrb", "kind": "belief", "holder": "p-maraharb"}]"#,
        );
        assert_eq!(codes(&r), ["reads_reclassified"]);
    }

    #[test]
    fn an_operation_that_cant_apply_fails_the_proposal() {
        let r = run(r#"[{"op": "delete", "target": "p-nnnnnnnn"}]"#);
        assert_eq!(codes(&r), ["op_failed"]);
        assert!(r.findings[0].message.starts_with("step 1 (delete)"));
        assert!(r.checked.is_empty());
    }
}
