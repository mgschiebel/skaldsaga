//! `complete`: how close the whole story is (HANDOFF, Step 9 planning).
//!
//! It runs right after each commit and on its own when the writer asks. It
//! reports to the agents only, as codes and IDs, never prose: each gate
//! (`weave`, `outline`, `complete`, internal names the writer never hears)
//! gets its readiness and a list of blockers, plus the soft findings from the
//! script checks in [`crate::softchecks`]. All writer-facing wording lives in
//! the orchestrator.
//!
//! * **weave:** the story's argument is settled enough to put its threads
//!   together: every counted arc has its dramatic question and a thematic
//!   answer (or "keep it simple"), no shape decision is open, fog subplots
//!   have been mapped, shelved or folded in, and the queue of findings is
//!   clear or acknowledged.
//! * **outline:** the skeleton exists, so deepening can start: the three
//!   movement landmarks and the climax are placed, in order, in the main
//!   story (provisional placements count).
//! * **complete:** the completeness bar: no empty grid cells, nothing
//!   unresolved.
//!
//! Each list is complete for its gate: `outline` includes everything `weave`
//! needs, and `complete` everything `outline` needs.
//!
//! Findings the writer has acknowledged (see [`crate::decisions`]) stop
//! blocking. An acknowledgment lapses when any node it rests on changes
//! revision; the finding then comes back as `resurfaced`, never silently. A
//! finding that no longer appears marks its acknowledgment resolved, and
//! `complete` deletes the file (skipped while the spec has loading problems,
//! and for checks that haven't started counting).

use std::collections::{BTreeMap, BTreeSet, HashSet};

use serde_json::{Value, json};

use crate::check;
use crate::decisions::{self, FindingKey, SCOPE_CANON, Store, Stored};
use crate::fsio::{self, FsError};
use crate::ids::{self, IdKind};
use crate::index;
use crate::model::{Arc, ArcStatus, Confidence, DecisionArea, Node, NodeBody, Stage, ThreadKind};
use crate::ops;
use crate::project::Project;
use crate::settings::CheckSettings;
use crate::softchecks::{self, BLOCKING_LANDMARKS, Ctx, Finding};
use crate::spec::{self, Spec};

/// `not_explored` is how a "keep it simple" choice is recorded.
const NOT_EXPLORED: &str = index::NOT_EXPLORED;

/// Something standing between the story and a gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Blocker {
    pub code: &'static str,
    pub arc: Option<String>,
    pub ids: Vec<String>,
    pub detail: Option<String>,
    pub count: Option<usize>,
}

impl Blocker {
    fn new(code: &'static str) -> Self {
        Blocker {
            code,
            arc: None,
            ids: Vec::new(),
            detail: None,
            count: None,
        }
    }

    fn arc(mut self, id: &str) -> Self {
        self.arc = Some(id.to_string());
        self
    }

    fn ids(mut self, ids: &[&str]) -> Self {
        self.ids = ids.iter().map(|s| s.to_string()).collect();
        self
    }

    fn detail(mut self, detail: &str) -> Self {
        self.detail = Some(detail.to_string());
        self
    }

    fn count(mut self, n: usize) -> Self {
        self.count = Some(n);
        self
    }

    fn to_value(&self) -> Value {
        let mut v = json!({"code": self.code});
        if let Value::Object(m) = &mut v {
            if let Some(arc) = &self.arc {
                m.insert("arc".into(), json!(arc));
            }
            if !self.ids.is_empty() {
                m.insert("ids".into(), json!(self.ids));
            }
            if let Some(detail) = &self.detail {
                m.insert("detail".into(), json!(detail));
            }
            if let Some(count) = self.count {
                m.insert("count".into(), json!(count));
            }
        }
        v
    }
}

/// One gate's readiness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gate {
    pub ready: bool,
    pub blockers: Vec<Blocker>,
}

impl Gate {
    fn new(blockers: Vec<Blocker>) -> Self {
        Gate {
            ready: blockers.is_empty(),
            blockers,
        }
    }

    fn to_value(&self) -> Value {
        json!({
            "ready": self.ready,
            "blockers": self.blockers.iter().map(Blocker::to_value).collect::<Vec<_>>(),
        })
    }
}

/// Where a finding stands against the writer's acknowledgments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standing {
    /// Nobody has answered it.
    Open,
    /// The writer said it was fine and nothing it rests on has changed.
    Acknowledged,
    /// The writer said it was fine, but something it rests on has changed
    /// since: ask again ("Earlier you said this was fine; the story has
    /// changed since").
    Resurfaced,
}

impl Standing {
    pub fn as_str(self) -> &'static str {
        match self {
            Standing::Open => "open",
            Standing::Acknowledged => "acknowledged",
            Standing::Resurfaced => "resurfaced",
        }
    }
}

/// A soft finding with its standing and the numbers that rank it.
#[derive(Debug, Clone, PartialEq)]
pub struct Reported {
    pub finding: Finding,
    pub standing: Standing,
    /// The acknowledging decision, when there is one.
    pub decision: Option<String>,
    /// IDs the acknowledgment rested on that have changed (resurfaced only).
    pub changed: Vec<String>,
    /// Other nodes that read what the finding rests on.
    pub fan_out: usize,
    /// The finding rests on a confirmed shape decision or its pivot belief.
    pub touches_confirmed: bool,
}

impl Reported {
    fn to_value(&self) -> Value {
        let f = &self.finding;
        let mut v = json!({
            "code": f.code,
            "version": f.version,
            "severity": "soft",
            "ids": f.ids,
            "data": f.data,
            "gates": f.gates,
            "status": self.standing.as_str(),
            "fan_out": self.fan_out,
            "touches_confirmed": self.touches_confirmed,
        });
        if let Value::Object(m) = &mut v {
            if let Some(detail) = &f.detail {
                m.insert("detail".into(), json!(detail));
            }
            if let Some(decision) = &self.decision {
                m.insert("decision".into(), json!(decision));
            }
            if !self.changed.is_empty() {
                m.insert("changed".into(), json!(self.changed));
            }
        }
        v
    }
}

/// A hard problem the spec has right now (a hand edit can cause one; commits
/// refuse them).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hard {
    pub code: &'static str,
    pub ids: Vec<String>,
}

/// Everything `complete` reports.
#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    pub spec_problems: usize,
    pub hard: Vec<Hard>,
    pub weave: Gate,
    pub outline: Gate,
    pub complete: Gate,
    /// Soft findings, ranked: confirmed decisions first, then by fan-out.
    pub findings: Vec<Reported>,
    /// "Not sure yet" answers still open, as proposal IDs.
    pub assumptions: Vec<String>,
    /// Acknowledgments in force after cleanup.
    pub acknowledgments: usize,
    /// Decision IDs deleted because their finding was resolved.
    pub removed: Vec<String>,
    /// True when cleanup was skipped because the spec has loading problems.
    pub cleanup_skipped: bool,
    pub store_problems: Vec<decisions::StoreProblem>,
    /// Paths for the skill to `git rm`.
    pub remove_paths: Vec<String>,
}

impl Report {
    pub fn to_value(&self) -> Value {
        json!({
            "outcome": "reported",
            "spec_problems": self.spec_problems,
            "hard_findings": self.hard.iter().map(|h| json!({"code": h.code, "ids": h.ids})).collect::<Vec<_>>(),
            "gates": {
                "weave": self.weave.to_value(),
                "outline": self.outline.to_value(),
                "complete": self.complete.to_value(),
            },
            "findings": self.findings.iter().map(Reported::to_value).collect::<Vec<_>>(),
            "assumptions": self.assumptions,
            "acknowledgments": {
                "active": self.acknowledgments,
                "removed": self.removed,
                "cleanup_skipped": self.cleanup_skipped,
            },
            "decision_problems": self.store_problems.iter()
                .map(|p| json!({"path": p.path, "message": p.message}))
                .collect::<Vec<_>>(),
            "remove_paths": self.remove_paths,
        })
    }
}

/// What `evaluate` works from.
pub struct Inputs<'a> {
    pub spec: &'a Spec,
    /// Files in the spec that couldn't be loaded.
    pub spec_problems: usize,
    pub hard: Vec<Hard>,
    pub store: &'a Store,
    pub settings: &'a CheckSettings,
    pub assumptions: Vec<String>,
}

/// The pure part: the report as it stands before any cleanup, and the
/// acknowledgments that are now resolved.
pub struct Evaluation {
    pub report: Report,
    pub resolved: Vec<Stored>,
}

/// The key a finding is acknowledged under.
pub fn finding_key(f: &Finding) -> FindingKey {
    FindingKey {
        code: f.code.to_string(),
        version: f.version,
        scope: SCOPE_CANON.to_string(),
        ids: f.ids.clone(),
        detail: f.detail.clone(),
    }
}

/// Arcs that count toward the gates: the main story and active subplots.
fn counted_arcs<'a>(ctx: &Ctx<'a>) -> Vec<(&'a Node, &'a Arc)> {
    ctx.spec
        .nodes
        .iter()
        .filter_map(|n| match &n.body {
            NodeBody::Arc(a) if matches!(a.status, ArcStatus::Main | ArcStatus::Active) => {
                Some((n, a))
            }
            _ => None,
        })
        .collect()
}

fn blank(s: &Option<String>) -> bool {
    s.as_deref().is_none_or(|t| t.trim().is_empty())
}

/// Blockers that stand in the way of weaving.
fn weave_blockers(ctx: &Ctx, inputs: &Inputs, open_findings: usize) -> Vec<Blocker> {
    let mut out = Vec::new();
    if ctx.main_arc().is_none() {
        out.push(Blocker::new("no_main_story"));
    }
    if inputs.spec_problems > 0 {
        out.push(Blocker::new("spec_problems").count(inputs.spec_problems));
    }
    if !inputs.hard.is_empty() {
        out.push(Blocker::new("hard_findings").count(inputs.hard.len()));
    }
    for (node, arc) in counted_arcs(ctx) {
        if blank(&arc.dramatic_question) {
            out.push(Blocker::new("dramatic_question_open").arc(&node.id));
        }
        let decisions = ctx.decisions(arc);
        let theme_answered = !blank(&arc.thematic_question)
            || decisions
                .iter()
                .any(|(_, d)| d.area == DecisionArea::Theme && d.value.is_some());
        if !theme_answered {
            out.push(Blocker::new("thematic_question_open").arc(&node.id));
        }
        for (dn, d) in decisions {
            if d.value.is_none() || d.confidence == Confidence::Unresolved {
                out.push(Blocker::new("decision_open").arc(&node.id).ids(&[&dn.id]));
            }
        }
    }
    for n in &ctx.spec.nodes {
        if matches!(&n.body, NodeBody::Arc(a) if a.status == ArcStatus::Fog) {
            out.push(Blocker::new("fog_subplot_open").arc(&n.id));
        }
    }
    if open_findings > 0 {
        out.push(Blocker::new("open_findings").count(open_findings));
    }
    out
}

/// The main story's movement landmarks and climax, which block deepening.
fn outline_blockers(ctx: &Ctx) -> Vec<Blocker> {
    let Some((node, arc)) = ctx.main_arc() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for key in BLOCKING_LANDMARKS {
        if ctx.landmark_pos(arc, key).is_none() {
            out.push(Blocker::new("landmark_open").arc(&node.id).detail(key));
        }
    }
    let at: Vec<usize> = BLOCKING_LANDMARKS
        .iter()
        .filter_map(|k| ctx.landmark_pos(arc, k))
        .collect();
    // The four are placed in story order: commits, midpoint, low point, climax.
    if at.len() == BLOCKING_LANDMARKS.len() && !at.is_sorted_by(|a, b| a < b) {
        out.push(Blocker::new("landmarks_out_of_order").arc(&node.id));
    }
    out
}

/// Threads the writer chose to keep out of the story ("keep it simple").
fn thread_exempt(decisions: &[(&Node, &crate::model::Decision)], kind: ThreadKind) -> bool {
    let all_kept_simple = |area: DecisionArea| {
        let mut of_area = decisions.iter().filter(|(_, d)| d.area == area).peekable();
        of_area.peek().is_some() && of_area.all(|(_, d)| d.value.as_deref() == Some(NOT_EXPLORED))
    };
    match kind {
        ThreadKind::Challenger => all_kept_simple(DecisionArea::Challenger),
        // Without a challenger there is no relationship either.
        ThreadKind::Relationship => {
            all_kept_simple(DecisionArea::Relationship) || all_kept_simple(DecisionArea::Challenger)
        }
        _ => false,
    }
}

/// What the completeness bar adds: no empty cells, nothing unresolved.
fn complete_blockers(ctx: &Ctx) -> Vec<Blocker> {
    let mut out = Vec::new();
    let covered: HashSet<(&str, Stage)> = ctx
        .spec
        .nodes
        .iter()
        .filter_map(|n| match &n.body {
            NodeBody::Scene(s) => s.coverage.as_ref(),
            _ => None,
        })
        .map(|c| (c.thread.as_str(), c.stage))
        .collect();
    for (arc_node, arc) in counted_arcs(ctx) {
        let decisions = ctx.decisions(arc);
        for thread in ctx.threads_of(&arc_node.id) {
            let NodeBody::Thread(t) = &thread.body else {
                continue;
            };
            if thread_exempt(&decisions, t.kind) {
                continue;
            }
            for entry in &t.stages {
                let planned = !blank(&entry.summary);
                if !planned && !covered.contains(&(thread.id.as_str(), entry.stage)) {
                    out.push(
                        Blocker::new("grid_cell_empty")
                            .arc(&arc_node.id)
                            .ids(&[&thread.id])
                            .detail(softchecks::stage_name(entry.stage)),
                    );
                }
            }
        }
    }
    for n in &ctx.spec.nodes {
        for f in &n.facts {
            if f.confidence == Confidence::Unresolved {
                out.push(Blocker::new("unresolved_fact").ids(&[&f.id]));
            }
        }
    }
    // Every scene is tied to a thread and stage. Only this gate blocks on it,
    // so scenes can be drafted, or harvested from a draft, before structure.
    for n in &ctx.spec.nodes {
        if matches!(&n.body, NodeBody::Scene(s) if s.coverage.is_none()) {
            out.push(Blocker::new("scene_untied").ids(&[&n.id]));
        }
    }
    out
}

/// Evaluates the story. Reads nothing and writes nothing.
pub fn evaluate(inputs: &Inputs) -> Evaluation {
    let spec = inputs.spec;
    let ctx = Ctx::new(spec);
    let soft = softchecks::run(spec, inputs.settings);

    let revisions = ops::revisions(spec);
    let readers = index::readers(spec);
    let confirmed: HashSet<&str> = spec
        .nodes
        .iter()
        .filter_map(|n| match &n.body {
            NodeBody::Decision(d) if d.confidence == Confidence::Confirmed => Some((n, d)),
            _ => None,
        })
        .flat_map(|(n, d)| std::iter::once(n.id.as_str()).chain(d.pivot.as_deref()))
        .collect();

    let mut findings = Vec::new();
    let mut matched: HashSet<FindingKey> = HashSet::new();
    for f in soft.findings {
        let key = finding_key(&f);
        let ack = inputs
            .store
            .decisions
            .iter()
            .find(|s| s.decision.finding.as_ref() == Some(&key));
        let (standing, decision, changed) = match ack {
            None => (Standing::Open, None, Vec::new()),
            Some(s) => {
                let changed: Vec<String> = s
                    .decision
                    .rests_on
                    .iter()
                    .filter(|(id, rev)| revisions.get(*id) != Some(*rev))
                    .map(|(id, _)| id.clone())
                    .collect();
                let standing = if changed.is_empty() {
                    Standing::Acknowledged
                } else {
                    Standing::Resurfaced
                };
                (standing, Some(s.decision.id.clone()), changed)
            }
        };
        matched.insert(key);
        let fan: BTreeSet<&str> = f
            .ids
            .iter()
            .flat_map(|id| readers.get(id).into_iter().flatten())
            .map(|(from, _)| from.as_str())
            .filter(|from| *from != index::SEQUENCE_FROM && !f.ids.iter().any(|id| id == from))
            .collect();
        let touches_confirmed = f.ids.iter().any(|id| confirmed.contains(id.as_str()));
        findings.push(Reported {
            fan_out: fan.len(),
            touches_confirmed,
            finding: f,
            standing,
            decision,
            changed,
        });
    }
    findings.sort_by(|a, b| {
        (
            !a.touches_confirmed,
            std::cmp::Reverse(a.fan_out),
            a.finding.code,
            &a.finding.ids,
            &a.finding.detail,
        )
            .cmp(&(
                !b.touches_confirmed,
                std::cmp::Reverse(b.fan_out),
                b.finding.code,
                &b.finding.ids,
                &b.finding.detail,
            ))
    });

    // Acknowledgments whose finding is gone: resolved. A check that hasn't
    // started counting, or a code this version doesn't know, leaves them be.
    let resolved: Vec<Stored> = inputs
        .store
        .decisions
        .iter()
        .filter(|s| {
            s.decision.finding.as_ref().is_some_and(|k| {
                !matched.contains(k)
                    && softchecks::version_of(&k.code).is_some()
                    && !soft.dormant.contains(k.code.as_str())
            })
        })
        .cloned()
        .collect();

    let open_findings = findings
        .iter()
        .filter(|r| r.finding.gates && r.standing != Standing::Acknowledged)
        .count();
    let weave = weave_blockers(&ctx, inputs, open_findings);
    let mut outline = weave.clone();
    outline.extend(outline_blockers(&ctx));
    let mut complete = outline.clone();
    complete.extend(complete_blockers(&ctx));

    let report = Report {
        spec_problems: inputs.spec_problems,
        hard: inputs.hard.clone(),
        weave: Gate::new(weave),
        outline: Gate::new(outline),
        complete: Gate::new(complete),
        findings,
        assumptions: inputs.assumptions.clone(),
        acknowledgments: inputs.store.decisions.len(),
        removed: Vec::new(),
        cleanup_skipped: false,
        store_problems: inputs.store.problems.clone(),
        remove_paths: Vec::new(),
    };
    Evaluation { report, resolved }
}

/// "Not sure yet" answers that are still open, from `.harness/committed/`.
fn open_assumptions(project: &Project) -> Result<Vec<String>, FsError> {
    let dir = project.harness_dir().join(crate::commit::COMMITTED_DIR);
    let mut out = Vec::new();
    for entry in fsio::list_dir(&dir)? {
        let Some(stem) = entry.name.strip_suffix(".json") else {
            continue;
        };
        if entry.is_dir || !ids::is_kind(stem, IdKind::Proposal) {
            continue;
        }
        let Some(Ok(text)) = fsio::read_regular_capped(&dir.join(&entry.name), 1024 * 1024)? else {
            continue;
        };
        let open = serde_json::from_str::<Value>(&text)
            .ok()
            .and_then(|v| {
                v.pointer("/committed/open_assumption")
                    .and_then(Value::as_bool)
            })
            .unwrap_or(false);
        if open {
            out.push(stem.to_string());
        }
    }
    Ok(out)
}

/// Runs `complete` for a project: evaluates the story, then deletes the
/// acknowledgments whose findings are resolved (unless the spec has loading
/// problems, when a finding may be missing only because a file didn't load).
pub fn run(project: &Project) -> Result<Report, FsError> {
    let loaded = spec::load(&project.spec_dir())?;
    let store = decisions::load(project)?;
    let settings = CheckSettings::load(project).unwrap_or_default();
    let hard = check::spec_problems(&loaded.spec)
        .into_iter()
        .map(|f| Hard {
            code: f.code,
            ids: f.ids,
        })
        .collect();
    let eval = evaluate(&Inputs {
        spec: &loaded.spec,
        spec_problems: loaded.problems.len(),
        hard,
        store: &store,
        settings: &settings,
        assumptions: open_assumptions(project)?,
    });
    let mut report = eval.report;
    if loaded.problems.is_empty() {
        for s in &eval.resolved {
            // A decision that vanished in the meantime is just as resolved.
            if let Ok(Some(path)) = decisions::remove(project, &s.decision.id) {
                report.remove_paths.push(path);
                report.removed.push(s.decision.id.clone());
            }
        }
        report.acknowledgments -= report.removed.len();
    } else if !eval.resolved.is_empty() {
        report.cleanup_skipped = true;
    }
    Ok(report)
}

/// Acknowledges a finding the story has right now, in the writer's words.
/// The finding is named the way `complete` reports it (code, IDs, detail), and
/// must be current: an acknowledgment can only answer something that was asked.
pub fn acknowledge(
    project: &Project,
    code: &str,
    ids_given: &[String],
    detail: Option<&str>,
    words: &str,
    date: &str,
) -> Result<decisions::Acknowledged, decisions::DecisionError> {
    let loaded = spec::load(&project.spec_dir())?;
    let settings = CheckSettings::load(project).unwrap_or_default();
    let soft = softchecks::run(&loaded.spec, &settings);
    let mut wanted: Vec<String> = ids_given.to_vec();
    wanted.sort();
    wanted.dedup();
    let Some(found) = soft
        .findings
        .iter()
        .find(|f| f.code == code && f.ids == wanted && f.detail.as_deref() == detail)
    else {
        return Err(decisions::DecisionError::Invalid(
            "the story has no such finding right now, so there is nothing to acknowledge".into(),
        ));
    };
    let revisions = ops::revisions(&loaded.spec);
    let rests_on: BTreeMap<String, u32> = found
        .ids
        .iter()
        .filter_map(|id| revisions.get(id).map(|r| (id.clone(), *r)))
        .collect();
    decisions::acknowledge(project, finding_key(found), words, rests_on, date)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Decision, RevealLevel, Review};
    use crate::testspec::*;
    use crate::testutil::TempDir;

    fn blocker_codes(gate: &Gate) -> Vec<&'static str> {
        gate.blockers.iter().map(|b| b.code).collect()
    }

    fn eval_with(spec: &Spec, store: &Store) -> Report {
        evaluate(&Inputs {
            spec,
            spec_problems: 0,
            hard: vec![],
            store,
            settings: &CheckSettings::default(),
            assumptions: vec![],
        })
        .report
    }

    fn eval(spec: &Spec) -> Report {
        eval_with(spec, &Store::default())
    }

    /// The eight-scene story with the argument settled.
    fn settled() -> Spec {
        let mut spec = story(vec![]);
        edit_arc(&mut spec, "a-main", |a| {
            a.dramatic_question = Some("Will Mara get the letters through?".into());
            a.thematic_question = Some("Is loyalty worth lying for?".into());
        });
        spec
    }

    fn shape(id: &str, area: DecisionArea, value: Option<&str>, conf: Confidence) -> Node {
        node(
            id,
            NodeBody::Decision(Decision {
                question: "?".into(),
                area,
                choices: vec![],
                value: value.map(str::to_string),
                options: vec![],
                evidence: vec![],
                pivot: None,
                confidence: conf,
                review: Review::Unreviewed,
            }),
        )
    }

    fn with_decisions(mut spec: Spec, nodes: Vec<Node>) -> Spec {
        let ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();
        spec.nodes.extend(nodes);
        spec.nodes.sort_by(|a, b| a.id.cmp(&b.id));
        edit_arc(&mut spec, "a-main", |a| a.decisions.extend(ids));
        spec
    }

    #[test]
    fn a_scene_with_no_thread_and_stage_blocks_complete_and_nothing_earlier() {
        let mut spec = settled();
        edit_scene(&mut spec, "s-3", |s| s.coverage = None);
        let r = eval(&spec);
        assert!(r.weave.ready, "{:?}", r.weave.blockers);
        assert!(r.outline.ready, "{:?}", r.outline.blockers);
        assert!(!r.complete.ready);
        let untied: Vec<_> = r
            .complete
            .blockers
            .iter()
            .filter(|b| b.code == "scene_untied")
            .collect();
        assert_eq!(untied.len(), 1);
        assert_eq!(untied[0].ids, vec!["s-3".to_string()]);
    }

    #[test]
    fn every_untied_scene_is_listed_and_tying_them_clears_the_blocker() {
        let mut spec = settled();
        for id in ["s-1", "s-5"] {
            edit_scene(&mut spec, id, |s| s.coverage = None);
        }
        let r = eval(&spec);
        let ids: Vec<_> = r
            .complete
            .blockers
            .iter()
            .filter(|b| b.code == "scene_untied")
            .map(|b| b.ids[0].as_str())
            .collect();
        assert_eq!(ids, ["s-1", "s-5"]);
        for id in ["s-1", "s-5"] {
            edit_scene(&mut spec, id, |s| {
                s.coverage = Some(crate::model::Coverage {
                    thread: "t-plot".into(),
                    stage: Stage::Setup,
                });
            });
        }
        let r = eval(&spec);
        assert!(
            r.complete.blockers.iter().all(|b| b.code != "scene_untied"),
            "{:?}",
            r.complete.blockers
        );
    }

    #[test]
    fn an_empty_story_is_ready_for_nothing() {
        let r = eval(&Spec::default());
        for gate in [&r.weave, &r.outline, &r.complete] {
            assert!(!gate.ready);
            assert_eq!(blocker_codes(gate), ["no_main_story"]);
        }
    }

    #[test]
    fn a_settled_story_with_a_skeleton_is_ready_all_the_way() {
        let r = eval(&settled());
        assert!(
            r.weave.ready && r.outline.ready && r.complete.ready,
            "{r:#?}"
        );
        // The five soft landmarks are suggestions, not part of the queue.
        assert_eq!(r.findings.len(), 5);
        assert!(r.findings.iter().all(|f| !f.finding.gates));
    }

    #[test]
    fn the_argument_must_be_settled_before_weaving() {
        let r = eval(&story(vec![]));
        assert_eq!(
            blocker_codes(&r.weave),
            ["dramatic_question_open", "thematic_question_open"]
        );
        // The same blockers carry through to the later gates.
        assert_eq!(blocker_codes(&r.complete), blocker_codes(&r.weave));
        assert_eq!(r.weave.blockers[0].arc.as_deref(), Some("a-main"));

        // A blank question doesn't count; "keep it simple" does for the theme.
        let mut spec = story(vec![]);
        edit_arc(&mut spec, "a-main", |a| {
            a.dramatic_question = Some("  ".into())
        });
        let spec = with_decisions(
            spec,
            vec![shape(
                "d-them",
                DecisionArea::Theme,
                Some("not_explored"),
                Confidence::Confirmed,
            )],
        );
        assert_eq!(
            blocker_codes(&eval(&spec).weave),
            ["dramatic_question_open"]
        );
    }

    #[test]
    fn open_decisions_block_weaving_until_answered_or_kept_simple() {
        let spec = with_decisions(
            settled(),
            vec![
                shape("d-open", DecisionArea::Hero, None, Confidence::Provisional),
                shape(
                    "d-simple",
                    DecisionArea::Plot,
                    Some("not_explored"),
                    Confidence::Confirmed,
                ),
                shape(
                    "d-prov",
                    DecisionArea::Plot,
                    Some("ends"),
                    Confidence::Provisional,
                ),
                shape(
                    "d-unres",
                    DecisionArea::Hero,
                    Some("changes"),
                    Confidence::Unresolved,
                ),
            ],
        );
        let r = eval(&spec);
        let open: Vec<_> = r
            .weave
            .blockers
            .iter()
            .filter(|b| b.code == "decision_open")
            .map(|b| b.ids[0].as_str())
            .collect();
        assert_eq!(open, ["d-open", "d-unres"]);
    }

    #[test]
    fn fog_subplots_must_be_dealt_with_before_weaving() {
        let mut spec = settled();
        spec.nodes
            .push(node("a-fog", NodeBody::Arc(arc(ArcStatus::Fog, &[], &[]))));
        spec.nodes.sort_by(|a, b| a.id.cmp(&b.id));
        let r = eval(&spec);
        assert_eq!(blocker_codes(&r.weave), ["fog_subplot_open"]);
        assert_eq!(r.weave.blockers[0].arc.as_deref(), Some("a-fog"));
        // Shelved ones are not gaps.
        edit_arc(&mut spec, "a-fog", |a| a.status = ArcStatus::Shelved);
        assert!(eval(&spec).weave.ready);
    }

    #[test]
    fn the_skeleton_gates_deepening_but_not_weaving() {
        let mut spec = settled();
        edit_arc(&mut spec, "a-main", |a| {
            a.landmarks.insert("midpoint".into(), None);
            a.landmarks.shift_remove("climax");
        });
        let r = eval(&spec);
        assert!(r.weave.ready);
        assert!(!r.outline.ready);
        let open: Vec<_> = r
            .outline
            .blockers
            .iter()
            .map(|b| (b.code, b.detail.as_deref()))
            .collect();
        assert_eq!(
            open,
            [
                ("landmark_open", Some("midpoint")),
                ("landmark_open", Some("climax"))
            ]
        );

        // A provisional placement passes; the wrong order does not.
        let mut spec = settled();
        edit_arc(&mut spec, "a-main", |a| {
            a.landmarks.insert("midpoint".into(), placed("s-7"));
        });
        let r = eval(&spec);
        assert_eq!(blocker_codes(&r.outline), ["landmarks_out_of_order"]);

        // Subplots don't block deepening.
        let mut spec = settled();
        spec.nodes.push(node(
            "a-side",
            NodeBody::Arc(arc(ArcStatus::Active, &[], &[])),
        ));
        spec.nodes.sort_by(|a, b| a.id.cmp(&b.id));
        edit_arc(&mut spec, "a-side", |a| {
            a.dramatic_question = Some("?".into());
            a.thematic_question = Some("?".into());
        });
        assert!(eval(&spec).outline.ready);
    }

    #[test]
    fn empty_grid_cells_and_unresolved_facts_block_completeness_only() {
        let mut spec = settled();
        for n in &mut spec.nodes {
            if let NodeBody::Thread(t) = &mut n.body {
                t.stages[2].summary = None;
            }
        }
        // Setup..Resolution scenes still cover the Crisis cell.
        assert!(eval(&spec).complete.ready);
        for id in ["s-5", "s-6"] {
            edit_scene(&mut spec, id, |s| s.coverage = None);
        }
        let r = eval(&spec);
        assert!(r.outline.ready && !r.complete.ready);
        let b = &r.complete.blockers[0];
        assert_eq!(
            (b.code, b.detail.as_deref()),
            ("grid_cell_empty", Some("crisis"))
        );
        assert_eq!(
            (b.arc.as_deref(), b.ids.as_slice()),
            (Some("a-main"), ["t-plot".to_string()].as_slice())
        );

        let mut spec = settled();
        let mut f = fact("f-open");
        f.confidence = Confidence::Unresolved;
        spec.nodes.push(fact_node(f));
        spec.nodes.sort_by(|a, b| a.id.cmp(&b.id));
        let r = eval(&spec);
        assert!(r.outline.ready);
        assert_eq!(blocker_codes(&r.complete), ["unresolved_fact"]);
    }

    #[test]
    fn a_kept_simple_challenger_leaves_its_threads_out_of_the_grid() {
        let mut spec = settled();
        let mut chall = thread("t-chal", "a-main", ThreadKind::Challenger);
        let mut rel = thread("t-rel", "a-main", ThreadKind::Relationship);
        for n in [&mut chall, &mut rel] {
            if let NodeBody::Thread(t) = &mut n.body {
                t.stages[0].summary = None;
            }
        }
        spec.nodes.extend([chall, rel]);
        spec.nodes.sort_by(|a, b| a.id.cmp(&b.id));
        edit_arc(&mut spec, "a-main", |a| {
            a.threads.extend(["t-chal".into(), "t-rel".into()])
        });
        assert_eq!(eval(&spec).complete.blockers.len(), 2);

        let spec = with_decisions(
            spec,
            vec![shape(
                "d-chal",
                DecisionArea::Challenger,
                Some("not_explored"),
                Confidence::Confirmed,
            )],
        );
        let r = eval(&spec);
        assert!(r.complete.ready, "{:#?}", r.complete.blockers);
    }

    fn twist_story() -> Spec {
        let mut f = fact("f-twist");
        f.reader = vec![moment("s-5", RevealLevel::Revealed)];
        let mut spec = settled();
        spec.nodes.push(fact_node(f));
        spec.nodes.sort_by(|a, b| a.id.cmp(&b.id));
        spec
    }

    fn ack(spec: &Spec, store: &mut Store, words: &str) {
        let found = softchecks::run(spec, &CheckSettings::default())
            .findings
            .into_iter()
            .find(|f| f.code == "unearned_twist")
            .unwrap();
        let revisions = ops::revisions(spec);
        store.decisions.push(Stored {
            decision: decisions::Decision {
                format: decisions::FORMAT,
                kind: decisions::KIND_FINDING_ACK.into(),
                id: "wd-aaaaaaaa".into(),
                words: words.into(),
                rests_on: found
                    .ids
                    .iter()
                    .filter_map(|id| revisions.get(id).map(|r| (id.clone(), *r)))
                    .collect(),
                status: decisions::Status::Active,
                date: "2026-10-24".into(),
                finding: Some(finding_key(&found)),
            },
            path: ".harness/decisions/finding_ack/wd-aaaaaaaa.json".into(),
        });
    }

    #[test]
    fn open_findings_block_until_acknowledged_and_resurface_when_the_story_moves() {
        let spec = twist_story();
        let r = eval(&spec);
        assert_eq!(blocker_codes(&r.weave), ["open_findings"]);
        assert_eq!(r.weave.blockers[0].count, Some(1));
        let twist = r
            .findings
            .iter()
            .find(|f| f.finding.code == "unearned_twist")
            .unwrap();
        assert_eq!(twist.standing, Standing::Open);

        // The writer says it's fine: it stops blocking.
        let mut store = Store::default();
        ack(&spec, &mut store, "It's a surprise on purpose.");
        let r = eval_with(&spec, &store);
        assert!(r.weave.ready, "{:#?}", r.weave.blockers);
        let twist = r
            .findings
            .iter()
            .find(|f| f.finding.code == "unearned_twist")
            .unwrap();
        assert_eq!(twist.standing, Standing::Acknowledged);
        assert_eq!(twist.decision.as_deref(), Some("wd-aaaaaaaa"));

        // The fact it rests on changes: the finding comes back, never silently.
        let mut moved = spec.clone();
        for n in &mut moved.nodes {
            for f in &mut n.facts {
                f.revision += 1;
            }
        }
        let r = eval_with(&moved, &store);
        let twist = r
            .findings
            .iter()
            .find(|f| f.finding.code == "unearned_twist")
            .unwrap();
        assert_eq!(twist.standing, Standing::Resurfaced);
        assert_eq!(twist.changed, ["f-twist"]);
        assert_eq!(blocker_codes(&r.weave), ["open_findings"]);
    }

    #[test]
    fn a_resolved_finding_marks_its_acknowledgment_for_removal() {
        let spec = twist_story();
        let mut store = Store::default();
        ack(&spec, &mut store, "fine");
        // Give the reveal a hint: the finding goes away.
        let mut fixed = spec.clone();
        for n in &mut fixed.nodes {
            for f in &mut n.facts {
                f.reader.push(moment("s-3", RevealLevel::Hinted));
                f.revision += 1;
            }
        }
        let eval = evaluate(&Inputs {
            spec: &fixed,
            spec_problems: 0,
            hard: vec![],
            store: &store,
            settings: &CheckSettings::default(),
            assumptions: vec![],
        });
        assert_eq!(eval.resolved.len(), 1);
        assert_eq!(eval.resolved[0].decision.id, "wd-aaaaaaaa");

        // A check that hasn't started counting leaves acknowledgments alone.
        let mut store = Store::default();
        let mut f = fact("f-hint");
        f.reader = vec![moment("s-2", RevealLevel::Hinted)];
        let mut spec = settled();
        spec.nodes.push(fact_node(f));
        spec.nodes.sort_by(|a, b| a.id.cmp(&b.id));
        let found = softchecks::run(&spec, &CheckSettings::default())
            .findings
            .into_iter()
            .find(|f| f.code == "dangling_setup")
            .unwrap();
        store.decisions.push(Stored {
            decision: decisions::Decision {
                format: decisions::FORMAT,
                kind: decisions::KIND_FINDING_ACK.into(),
                id: "wd-bbbbbbbb".into(),
                words: "later".into(),
                rests_on: BTreeMap::new(),
                status: decisions::Status::Active,
                date: "2026-10-24".into(),
                finding: Some(finding_key(&found)),
            },
            path: String::new(),
        });
        let mut dormant = spec.clone();
        for id in ["s-7", "s-8"] {
            edit_scene(&mut dormant, id, |s| {
                s.coverage.as_mut().unwrap().stage = Stage::Crisis
            });
        }
        let eval = evaluate(&Inputs {
            spec: &dormant,
            spec_problems: 0,
            hard: vec![],
            store: &store,
            settings: &CheckSettings::default(),
            assumptions: vec![],
        });
        assert!(eval.resolved.is_empty());
    }

    #[test]
    fn findings_are_ranked_by_confirmed_decisions_then_fan_out() {
        let mut spec = twist_story();
        let mut other = fact("f-aaaa");
        other.reader = vec![moment("s-4", RevealLevel::Revealed)];
        spec.nodes.push({
            let mut n = node("p-jonah", NodeBody::Person);
            n.facts.push(other);
            n
        });
        spec.nodes.sort_by(|a, b| a.id.cmp(&b.id));
        // A scene that reads f-twist gives it a reader.
        edit_scene(&mut spec, "s-6", |s| s.reads.push("f-twist".into()));
        let r = eval(&spec);
        let gating: Vec<_> = r
            .findings
            .iter()
            .filter(|f| f.finding.code == "unearned_twist")
            .map(|f| (f.finding.ids[0].as_str(), f.fan_out))
            .collect();
        assert_eq!(gating, [("f-twist", 1), ("f-aaaa", 0)]);
    }

    #[test]
    fn spec_problems_and_hard_findings_block_every_gate() {
        let spec = settled();
        let r = evaluate(&Inputs {
            spec: &spec,
            spec_problems: 2,
            hard: vec![Hard {
                code: "broken_reference",
                ids: vec!["s-1".into()],
            }],
            store: &Store::default(),
            settings: &CheckSettings::default(),
            assumptions: vec!["pr-aaaaaaaa".into()],
        })
        .report;
        assert_eq!(blocker_codes(&r.weave), ["spec_problems", "hard_findings"]);
        assert_eq!(
            blocker_codes(&r.complete),
            ["spec_problems", "hard_findings"]
        );
        // Open assumptions are listed, never blocking.
        assert_eq!(r.assumptions, ["pr-aaaaaaaa"]);
    }

    #[test]
    fn the_report_is_codes_and_ids_only() {
        let spec = twist_story();
        let v = eval(&spec).to_value();
        assert_eq!(v["outcome"], "reported");
        assert_eq!(v["gates"]["weave"]["ready"], false);
        assert_eq!(
            v["gates"]["weave"]["blockers"][0],
            json!({"code": "open_findings", "count": 1})
        );
        let twist = v["findings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["code"] == "unearned_twist")
            .unwrap();
        assert_eq!(twist["status"], "open");
        assert_eq!(twist["ids"], json!(["f-twist"]));
        assert_eq!(twist["severity"], "soft");
        // No finding or blocker carries a sentence.
        let text = v.to_string();
        assert!(!text.contains("message"), "{text}");
    }

    // --- against a real folder ---------------------------------------------

    fn copy_dir(from: &std::path::Path, to: &std::path::Path) {
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

    fn harbor_project() -> (TempDir, Project) {
        let dir = TempDir::new();
        let project = Project::new(dir.path());
        std::fs::create_dir_all(project.harness_dir()).unwrap();
        let fixture = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/specs/harbor");
        copy_dir(&fixture, &project.spec_dir());
        (dir, project)
    }

    #[test]
    fn run_reads_a_folder_and_is_repeatable() {
        let (_dir, project) = harbor_project();
        let a = run(&project).unwrap();
        let b = run(&project).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.removed, Vec::<String>::new());
        assert!(!a.findings.is_empty());
    }

    #[test]
    fn acknowledging_through_the_folder_clears_the_queue_and_cleans_up_when_resolved() {
        let (_dir, project) = harbor_project();
        let before = run(&project).unwrap();
        let gating: Vec<&Reported> = before.findings.iter().filter(|f| f.finding.gates).collect();
        assert!(
            !gating.is_empty(),
            "the harbor fixture is expected to have a finding in the queue"
        );
        let f = &gating[0].finding;
        let ack = acknowledge(
            &project,
            f.code,
            &f.ids,
            f.detail.as_deref(),
            "That's deliberate.",
            "2026-10-24",
        )
        .unwrap();
        assert!(!ack.renewed);

        let after = run(&project).unwrap();
        let now = after
            .findings
            .iter()
            .find(|r| {
                r.finding.code == f.code && r.finding.ids == f.ids && r.finding.detail == f.detail
            })
            .unwrap();
        assert_eq!(now.standing, Standing::Acknowledged);
        assert_eq!(after.acknowledgments, 1);

        // Something that isn't a current finding can't be acknowledged.
        let err = acknowledge(
            &project,
            "unearned_twist",
            &["f-nopenope".into()],
            None,
            "x",
            "2026-10-24",
        );
        assert!(matches!(err, Err(decisions::DecisionError::Invalid(_))));
    }

    #[test]
    fn resolved_acknowledgments_are_deleted_only_when_the_spec_loads_cleanly() {
        let (_dir, project) = harbor_project();
        // An acknowledgment of a finding the story doesn't have.
        let orphan = decisions::acknowledge(
            &project,
            FindingKey {
                code: "unearned_twist".into(),
                version: 1,
                scope: SCOPE_CANON.into(),
                ids: vec!["f-zzzzzzzz".into()],
                detail: None,
            },
            "It was fine once.",
            BTreeMap::new(),
            "2026-10-24",
        )
        .unwrap();
        let on_disk = project.root().join(&orphan.path);

        // The fixture has a file that doesn't load, so a missing finding proves
        // nothing: leave the acknowledgment and say so.
        let first = run(&project).unwrap();
        assert!(first.spec_problems > 0);
        assert!(first.cleanup_skipped);
        assert!(first.removed.is_empty() && first.remove_paths.is_empty());
        assert!(on_disk.exists());
        assert_eq!(first.acknowledgments, 1);

        // Once everything loads, the resolved acknowledgment goes.
        std::fs::remove_file(project.spec_dir().join("attributes.md")).unwrap();
        let second = run(&project).unwrap();
        assert_eq!(second.spec_problems, 0);
        assert!(!second.cleanup_skipped);
        assert_eq!(second.removed, std::slice::from_ref(&orphan.decision.id));
        assert_eq!(second.remove_paths, std::slice::from_ref(&orphan.path));
        assert_eq!(second.acknowledgments, 0);
        assert!(!on_disk.exists());
        assert!(decisions::load(&project).unwrap().decisions.is_empty());
    }
}
