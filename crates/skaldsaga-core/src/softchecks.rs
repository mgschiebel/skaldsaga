//! The script soft checks behind `complete` (HANDOFF, Checks: who catches
//! what; Step 9 planning). They are countable, run free and instantly on
//! every commit, and share one finding format. Nothing here writes prose:
//! a finding is a versioned code, the IDs it rests on and a little data, and
//! the orchestrator words it for the writer.
//!
//! Each check starts counting only once the story can support it, so a
//! half-built story doesn't produce findings that only mean "not done yet":
//!
//! | Check | Starts counting when |
//! |---|---|
//! | `unearned_twist` | always |
//! | `similar_attribute_keys` | always |
//! | `dangling_setup` | a main-plot Resolution scene exists |
//! | `early_pivot` | the climax landmark is placed |
//! | `subplot_after_climax` | the main climax landmark is placed |
//! | `scene_count_pacing` | the arc's skeleton exists (all three movement landmarks placed in order) |
//! | `threads_out_of_alignment` | the arc's skeleton exists |
//! | `landmark_missing` | always (the five soft landmarks; the other four block the outline gate) |
//!
//! A check that hasn't started is reported as dormant, so an acknowledgment
//! of one of its findings isn't cleaned up just because the check is quiet.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use serde_json::{Value, json};

use crate::index::{self, ArcMovements, Method};
use crate::model::{
    Arc, ArcStatus, Decision, Fact, Node, NodeBody, RevealLevel, Scene, Stage, ThreadKind,
    TimeAnchor,
};
use crate::registry::{Registry, starter};
use crate::settings::CheckSettings;
use crate::spec::Spec;

/// A check's stable code and version. A changed check gets a new version, so
/// it never matches an acknowledgment of the old one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Check {
    pub code: &'static str,
    pub version: u32,
}

pub const UNEARNED_TWIST: Check = Check {
    code: "unearned_twist",
    version: 1,
};
pub const DANGLING_SETUP: Check = Check {
    code: "dangling_setup",
    version: 1,
};
pub const EARLY_PIVOT: Check = Check {
    code: "early_pivot",
    version: 1,
};
pub const SUBPLOT_AFTER_CLIMAX: Check = Check {
    code: "subplot_after_climax",
    version: 1,
};
pub const SCENE_COUNT_PACING: Check = Check {
    code: "scene_count_pacing",
    version: 1,
};
pub const THREADS_OUT_OF_ALIGNMENT: Check = Check {
    code: "threads_out_of_alignment",
    version: 1,
};
pub const SIMILAR_ATTRIBUTE_KEYS: Check = Check {
    code: "similar_attribute_keys",
    version: 1,
};
pub const LANDMARK_MISSING: Check = Check {
    code: "landmark_missing",
    version: 1,
};

/// Every check, for looking up a code's current version.
pub const ALL_CHECKS: [Check; 8] = [
    UNEARNED_TWIST,
    DANGLING_SETUP,
    EARLY_PIVOT,
    SUBPLOT_AFTER_CLIMAX,
    SCENE_COUNT_PACING,
    THREADS_OUT_OF_ALIGNMENT,
    SIMILAR_ATTRIBUTE_KEYS,
    LANDMARK_MISSING,
];

/// The current version of a check, or `None` for a code we don't know.
pub fn version_of(code: &str) -> Option<u32> {
    ALL_CHECKS
        .iter()
        .find(|c| c.code == code)
        .map(|c| c.version)
}

/// The landmarks that end a movement and block the outline gate with the
/// climax. `mc_commits`, `midpoint` and `low_point` are the movement
/// boundaries; the climax is the fourth.
pub const BLOCKING_LANDMARKS: [&str; 4] = ["mc_commits", "midpoint", "low_point", "climax"];

/// The five landmarks that stay soft suggestions, with the movement each
/// belongs to (raised when the outliner deepens that movement).
pub const SOFT_LANDMARKS: [(&str, Stage); 5] = [
    ("normal_world", Stage::Setup),
    ("inciting_incident", Stage::Setup),
    ("rising_stakes", Stage::Complications),
    ("things_fall_apart", Stage::Crisis),
    ("new_normal", Stage::Resolution),
];

pub fn stage_name(stage: Stage) -> &'static str {
    match stage {
        Stage::Setup => "setup",
        Stage::Complications => "complications",
        Stage::Crisis => "crisis",
        Stage::Resolution => "resolution",
    }
}

/// One thing a check noticed. Always soft: the writer decides.
#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
    pub code: &'static str,
    pub version: u32,
    /// Node and fact IDs the finding rests on, sorted. Kept few, so small
    /// edits elsewhere don't make an acknowledgment lapse.
    pub ids: Vec<String>,
    /// Tells apart findings of one code on the same IDs.
    pub detail: Option<String>,
    /// Numbers and IDs for the orchestrator's wording; never prose.
    pub data: Value,
    /// Whether the finding joins the queue that must be cleared before
    /// weaving and outlining.
    pub gates: bool,
}

impl Finding {
    fn new(check: Check, ids: &[&str], detail: Option<String>, data: Value) -> Self {
        let mut ids: Vec<String> = ids.iter().map(|s| s.to_string()).collect();
        ids.sort();
        ids.dedup();
        Finding {
            code: check.code,
            version: check.version,
            ids,
            detail,
            data,
            gates: check.code != LANDMARK_MISSING.code,
        }
    }
}

/// What the checks found, and which ones haven't started counting.
#[derive(Debug, Default)]
pub struct Soft {
    pub findings: Vec<Finding>,
    pub dormant: BTreeSet<&'static str>,
}

/// The facts about a spec that several checks and the gates need.
pub(crate) struct Ctx<'a> {
    pub(crate) spec: &'a Spec,
    /// Scenes in reading order.
    pub(crate) order: Vec<String>,
    /// Scene ID to its place in the reading order.
    pub(crate) pos: HashMap<String, usize>,
    pub(crate) movements: BTreeMap<String, ArcMovements>,
    /// Thread ID to its arc.
    pub(crate) thread_arc: HashMap<String, String>,
    facts: HashMap<&'a str, &'a Fact>,
}

impl<'a> Ctx<'a> {
    pub(crate) fn new(spec: &'a Spec) -> Self {
        let order = index::reading_order(spec);
        let pos = order
            .iter()
            .enumerate()
            .map(|(i, id)| (id.clone(), i))
            .collect();
        let facts = spec
            .nodes
            .iter()
            .flat_map(|n| n.facts.iter())
            .map(|f| (f.id.as_str(), f))
            .collect();
        Ctx {
            spec,
            order,
            pos,
            movements: index::derived_movements(spec),
            thread_arc: index::thread_arcs(spec),
            facts,
        }
    }

    /// Arcs with this status, as node and arc fields.
    pub(crate) fn arcs(&self, status: ArcStatus) -> impl Iterator<Item = (&'a Node, &'a Arc)> {
        self.spec.nodes.iter().filter_map(move |n| match &n.body {
            NodeBody::Arc(a) if a.status == status => Some((n, a)),
            _ => None,
        })
    }

    pub(crate) fn main_arc(&self) -> Option<(&'a Node, &'a Arc)> {
        self.arcs(ArcStatus::Main).next()
    }

    /// The reading-order place of a landmark, if it's placed on a scene that
    /// exists and is in the reading order.
    pub(crate) fn landmark_pos(&self, arc: &Arc, key: &str) -> Option<usize> {
        let place = arc.landmarks.get(key)?.as_ref()?;
        self.pos.get(&place.scene).copied()
    }

    fn scene(&self, id: &str) -> Option<&'a Scene> {
        match self.spec.node(id).map(|n| &n.body) {
            Some(NodeBody::Scene(s)) => Some(s),
            _ => None,
        }
    }

    /// The decision nodes an arc lists.
    pub(crate) fn decisions(&self, arc: &Arc) -> Vec<(&'a Node, &'a Decision)> {
        arc.decisions
            .iter()
            .filter_map(|id| self.spec.node(id))
            .filter_map(|n| match &n.body {
                NodeBody::Decision(d) => Some((n, d)),
                _ => None,
            })
            .collect()
    }

    /// An arc's threads, sorted by ID.
    pub(crate) fn threads_of(&self, arc_id: &str) -> Vec<&'a Node> {
        let mut ids: Vec<&String> = self
            .thread_arc
            .iter()
            .filter(|(_, a)| a.as_str() == arc_id)
            .map(|(t, _)| t)
            .collect();
        ids.sort();
        ids.into_iter()
            .filter_map(|id| self.spec.node(id))
            .filter(|n| matches!(n.body, NodeBody::Thread(_)))
            .collect()
    }

    /// Where in the reading order a time anchor lands, when it names a scene
    /// (directly, or through the fact it's tied to).
    fn anchor_pos(&self, anchor: &TimeAnchor) -> Option<usize> {
        match anchor {
            TimeAnchor::In(a) => self.pos.get(&a.scene).copied(),
            TimeAnchor::At(a) => match self.facts.get(a.at.as_str())?.true_from.as_ref()? {
                TimeAnchor::In(i) => self.pos.get(&i.scene).copied(),
                _ => None,
            },
            _ => None,
        }
    }

    /// Positions of every scene that belongs to an arc: scenes covering its
    /// threads, and scenes placed as its landmarks or beats.
    fn arc_positions(&self, arc_id: &str, arc: &Arc) -> Vec<usize> {
        let mut out: Vec<usize> = self
            .order
            .iter()
            .enumerate()
            .filter(|(_, id)| {
                self.scene(id)
                    .and_then(|s| s.coverage.as_ref())
                    .and_then(|c| self.thread_arc.get(&c.thread))
                    .is_some_and(|a| a == arc_id)
            })
            .map(|(i, _)| i)
            .collect();
        for place in arc.landmarks.values().chain(arc.beats.values()).flatten() {
            if let Some(&p) = self.pos.get(&place.scene) {
                out.push(p);
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    /// The main plot (the big-picture thread of the main arc) has a scene
    /// covering its Resolution stage.
    fn main_resolution_exists(&self) -> bool {
        let Some((main_id, _)) = self.main_arc().map(|(n, a)| (n.id.as_str(), a)) else {
            return false;
        };
        let plots: HashSet<&str> = self
            .threads_of(main_id)
            .into_iter()
            .filter(|n| matches!(&n.body, NodeBody::Thread(t) if t.kind == ThreadKind::BigPicture))
            .map(|n| n.id.as_str())
            .collect();
        self.order.iter().any(|id| {
            self.scene(id)
                .and_then(|s| s.coverage.as_ref())
                .is_some_and(|c| c.stage == Stage::Resolution && plots.contains(c.thread.as_str()))
        })
    }
}

/// Runs every check.
pub fn run(spec: &Spec, settings: &CheckSettings) -> Soft {
    let ctx = Ctx::new(spec);
    let mut soft = Soft::default();
    unearned_twist(&ctx, &mut soft);
    dangling_setup(&ctx, &mut soft);
    early_pivot(&ctx, settings, &mut soft);
    subplot_after_climax(&ctx, &mut soft);
    scene_count_pacing(&ctx, settings, &mut soft);
    threads_out_of_alignment(&ctx, settings, &mut soft);
    similar_attribute_keys(&ctx, settings, &mut soft);
    landmark_missing(&ctx, &mut soft);
    soft.findings
        .sort_by(|a, b| (a.code, &a.ids, &a.detail).cmp(&(b.code, &b.ids, &b.detail)));
    soft
}

/// A reveal with no earlier hint. Applies to every revealed fact.
fn unearned_twist(ctx: &Ctx, soft: &mut Soft) {
    for node in &ctx.spec.nodes {
        for fact in &node.facts {
            let reveal = fact
                .reader
                .iter()
                .filter(|m| m.level == RevealLevel::Revealed)
                .filter_map(|m| ctx.pos.get(&m.scene).map(|&p| (p, &m.scene)))
                .min();
            let Some((at, scene)) = reveal else { continue };
            let hinted_before = fact.reader.iter().any(|m| {
                m.level == RevealLevel::Hinted && ctx.pos.get(&m.scene).is_some_and(|&p| p < at)
            });
            if !hinted_before {
                soft.findings.push(Finding::new(
                    UNEARNED_TWIST,
                    &[&fact.id],
                    None,
                    json!({"reveal_scene": scene}),
                ));
            }
        }
    }
}

/// A hint never revealed or used. Quiet until the main plot has a Resolution
/// scene, since early on almost every hint is an open promise.
fn dangling_setup(ctx: &Ctx, soft: &mut Soft) {
    if !ctx.main_resolution_exists() {
        soft.dormant.insert(DANGLING_SETUP.code);
        return;
    }
    for node in &ctx.spec.nodes {
        for fact in &node.facts {
            if fact.reader.iter().any(|m| m.level == RevealLevel::Revealed) {
                continue;
            }
            let hint = fact
                .reader
                .iter()
                .filter(|m| m.level == RevealLevel::Hinted)
                .filter_map(|m| ctx.pos.get(&m.scene).map(|&p| (p, &m.scene)))
                .min();
            let Some((at, scene)) = hint else { continue };
            let read_later = ctx.spec.nodes.iter().any(|n| {
                matches!(&n.body, NodeBody::Scene(s) if s.reads.contains(&fact.id))
                    && ctx.pos.get(&n.id).is_some_and(|&p| p > at)
            });
            let learned_later = fact
                .characters
                .iter()
                .any(|c| ctx.pos.get(&c.learns_in).is_some_and(|&p| p > at));
            if !read_later && !learned_later {
                soft.findings.push(Finding::new(
                    DANGLING_SETUP,
                    &[&fact.id],
                    None,
                    json!({"hint_scene": scene}),
                ));
            }
        }
    }
}

/// The pivot belief of a "changes" decision ends well before the climax.
fn early_pivot(ctx: &Ctx, settings: &CheckSettings, soft: &mut Soft) {
    let total = ctx.order.len() as f64;
    let mut any_climax = false;
    for status in [ArcStatus::Main, ArcStatus::Active] {
        for (arc_node, arc) in ctx.arcs(status) {
            let Some(climax) = ctx.landmark_pos(arc, "climax") else {
                continue;
            };
            any_climax = true;
            for (node, decision) in ctx.decisions(arc) {
                if decision.value.as_deref() != Some("changes") {
                    continue;
                }
                let Some(pivot) = decision.pivot.as_deref() else {
                    continue;
                };
                let Some(ends) = ctx.facts.get(pivot).and_then(|f| f.ends.as_ref()) else {
                    continue;
                };
                let Some(end) = ctx.anchor_pos(ends) else {
                    continue;
                };
                let gap = climax.saturating_sub(end);
                if gap as f64 > settings.early_pivot_share * total {
                    soft.findings.push(Finding::new(
                        EARLY_PIVOT,
                        &[&node.id, pivot],
                        None,
                        json!({
                            "arc": arc_node.id,
                            "scenes_before_climax": gap,
                            "scenes_in_story": ctx.order.len(),
                        }),
                    ));
                }
            }
        }
    }
    if !any_climax {
        soft.dormant.insert(EARLY_PIVOT.code);
    }
}

/// A subplot that resolves after the main climax.
fn subplot_after_climax(ctx: &Ctx, soft: &mut Soft) {
    let Some((main_node, main)) = ctx.main_arc() else {
        soft.dormant.insert(SUBPLOT_AFTER_CLIMAX.code);
        return;
    };
    let Some(main_climax) = ctx.landmark_pos(main, "climax") else {
        soft.dormant.insert(SUBPLOT_AFTER_CLIMAX.code);
        return;
    };
    for (node, arc) in ctx.arcs(ArcStatus::Active) {
        let end = ctx
            .landmark_pos(arc, "climax")
            .or_else(|| ctx.arc_positions(&node.id, arc).last().copied());
        if let Some(end) = end
            && end > main_climax
        {
            soft.findings.push(Finding::new(
                SUBPLOT_AFTER_CLIMAX,
                &[&node.id, &main_node.id],
                None,
                json!({"scenes_after_main_climax": end - main_climax}),
            ));
        }
    }
}

/// A movement far shorter or longer than its expected share of the scenes.
/// The expected share comes from the framework; until framework choice
/// exists, every movement expects a quarter (the four-part default).
fn scene_count_pacing(ctx: &Ctx, settings: &CheckSettings, soft: &mut Soft) {
    let mut any = false;
    for (arc_id, m) in &ctx.movements {
        if m.method != Method::Landmarks {
            continue;
        }
        any = true;
        let total: usize = Stage::ALL.iter().map(|s| m.movements.stage(*s).len()).sum();
        if total == 0 {
            continue;
        }
        let expected_share = 0.25;
        let expected = total as f64 * expected_share;
        for stage in Stage::ALL {
            let scenes = m.movements.stage(stage).len();
            let n = scenes as f64;
            let direction = if n < settings.under_share * expected {
                "short"
            } else if n > settings.over_share * expected {
                "long"
            } else {
                continue;
            };
            soft.findings.push(Finding::new(
                SCENE_COUNT_PACING,
                &[arc_id],
                Some(format!("{}:{direction}", stage_name(stage))),
                json!({
                    "movement": stage_name(stage),
                    "direction": direction,
                    "scenes": scenes,
                    "scenes_in_arc": total,
                    "expected_share": expected_share,
                }),
            ));
        }
    }
    if !any {
        soft.dormant.insert(SCENE_COUNT_PACING.code);
    }
}

/// A thread whose scenes for a stage mostly fall in a different movement.
fn threads_out_of_alignment(ctx: &Ctx, settings: &CheckSettings, soft: &mut Soft) {
    let mut any = false;
    for (arc_id, m) in &ctx.movements {
        if m.method != Method::Landmarks {
            continue;
        }
        any = true;
        let movement_of: HashMap<&str, Stage> = Stage::ALL
            .iter()
            .flat_map(|s| {
                m.movements
                    .stage(*s)
                    .iter()
                    .map(move |id| (id.as_str(), *s))
            })
            .collect();
        for thread in ctx.threads_of(arc_id) {
            for stage in Stage::ALL {
                let (mut judged, mut outside) = (0usize, 0usize);
                for id in &ctx.order {
                    let covers = ctx
                        .scene(id)
                        .and_then(|s| s.coverage.as_ref())
                        .is_some_and(|c| c.thread == thread.id && c.stage == stage);
                    if !covers {
                        continue;
                    }
                    let Some(&actual) = movement_of.get(id.as_str()) else {
                        continue;
                    };
                    judged += 1;
                    if actual != stage {
                        outside += 1;
                    }
                }
                if judged > 0 && outside as f64 / judged as f64 > settings.misaligned_share {
                    soft.findings.push(Finding::new(
                        THREADS_OUT_OF_ALIGNMENT,
                        &[&thread.id, arc_id],
                        Some(stage_name(stage).to_string()),
                        json!({
                            "stage": stage_name(stage),
                            "scenes": judged,
                            "scenes_elsewhere": outside,
                        }),
                    ));
                }
            }
        }
    }
    if !any {
        soft.dormant.insert(THREADS_OUT_OF_ALIGNMENT.code);
    }
}

/// Edit distance between two strings, by characters.
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut cur = Vec::with_capacity(prev.len());
        cur.push(i + 1);
        // `left` is the cell just written; `diag` and `up` are the two cells
        // above it in the previous row.
        let mut left = i + 1;
        for ((&cb, &diag), &up) in b.iter().zip(prev.iter()).zip(prev.iter().skip(1)) {
            let cost = usize::from(ca != cb);
            left = (diag + cost).min(up + 1).min(left + 1);
            cur.push(left);
        }
        prev = cur;
    }
    prev.last().copied().unwrap_or(0)
}

/// Two attribute keys that look alike, when at least one is the story's own
/// or is used without being registered (the starter vocabulary is fixed, so
/// two of its keys are never the writer's slip).
fn similar_attribute_keys(ctx: &Ctx, settings: &CheckSettings, soft: &mut Soft) {
    let registry = Registry::new(&ctx.spec.attributes);
    let starter_keys: HashSet<String> = starter()
        .into_iter()
        .flat_map(|d| std::iter::once(d.key).chain(d.inverse))
        .collect();
    let mut keys: BTreeSet<String> = BTreeSet::new();
    for (def, _) in registry.defs() {
        keys.insert(def.key.clone());
        if let Some(inv) = &def.inverse {
            keys.insert(inv.clone());
        }
    }
    for fact in ctx.facts.values() {
        if let Some(k) = &fact.attribute {
            keys.insert(k.clone());
        }
    }
    let keys: Vec<&String> = keys.iter().collect();
    for (i, a) in keys.iter().enumerate() {
        for b in keys.iter().skip(i + 1) {
            if starter_keys.contains(*a) && starter_keys.contains(*b) {
                continue;
            }
            // The two sides of one inverse pair are meant to look alike.
            let paired = |x: &str, y: &str| {
                registry
                    .lookup(x)
                    .is_some_and(|f| f.def.inverse.as_deref() == Some(y))
            };
            if paired(a, b) || paired(b, a) {
                continue;
            }
            let (na, nb) = (a.replace('_', ""), b.replace('_', ""));
            if na.chars().count().min(nb.chars().count()) < 4 {
                continue;
            }
            let d = distance(&na, &nb);
            if d <= settings.similar_key_max_distance {
                soft.findings.push(Finding::new(
                    SIMILAR_ATTRIBUTE_KEYS,
                    &[],
                    Some(format!("{a}|{b}")),
                    json!({"keys": [a, b], "distance": d}),
                ));
            }
        }
    }
}

/// The five landmarks that stay soft suggestions, for the main story. Each
/// carries the movement it belongs to, so the orchestrator raises it when
/// that movement is deepened.
fn landmark_missing(ctx: &Ctx, soft: &mut Soft) {
    let Some((node, arc)) = ctx.main_arc() else {
        return;
    };
    for (key, stage) in SOFT_LANDMARKS {
        if ctx.landmark_pos(arc, key).is_none() {
            soft.findings.push(Finding::new(
                LANDMARK_MISSING,
                &[&node.id],
                Some(key.to_string()),
                json!({"landmark": key, "movement": stage_name(stage)}),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AtAnchor, Confidence, Coverage, DecisionArea, FactKind, InAnchor, Review};
    use crate::registry::{AttributeDef, Cardinality, CardinalitySpec, Relation};
    use crate::testspec::*;

    fn run_default(spec: &Spec) -> Soft {
        run(spec, &CheckSettings::default())
    }

    fn of_code<'a>(soft: &'a Soft, code: &str) -> Vec<&'a Finding> {
        soft.findings.iter().filter(|f| f.code == code).collect()
    }

    // --- tests ------------------------------------------------------------

    #[test]
    fn a_clean_story_has_only_the_soft_landmark_suggestions() {
        let soft = run_default(&story(vec![]));
        let codes: Vec<_> = soft.findings.iter().map(|f| f.code).collect();
        assert_eq!(codes, vec!["landmark_missing"; 5], "{:#?}", soft.findings);
        assert!(soft.findings.iter().all(|f| !f.gates));
        assert!(soft.dormant.is_empty(), "{:?}", soft.dormant);
        let details: Vec<_> = soft
            .findings
            .iter()
            .filter_map(|f| f.detail.as_deref())
            .collect();
        assert_eq!(
            details,
            [
                "inciting_incident",
                "new_normal",
                "normal_world",
                "rising_stakes",
                "things_fall_apart"
            ]
        );
        // Placing one removes its suggestion.
        let mut spec = story(vec![]);
        edit_arc(&mut spec, "a-main", |a| {
            a.landmarks.insert("normal_world".into(), placed("s-1"));
        });
        assert_eq!(of_code(&run_default(&spec), "landmark_missing").len(), 4);
    }

    #[test]
    fn a_reveal_with_no_earlier_hint_is_an_unearned_twist() {
        let reveal_only = |f: &mut Fact| f.reader = vec![moment("s-5", RevealLevel::Revealed)];
        let mut f = fact("f-twist");
        reveal_only(&mut f);
        let soft = run_default(&story(vec![fact_node(f)]));
        let found = of_code(&soft, "unearned_twist");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].ids, ["f-twist"]);
        assert!(found[0].gates);
        assert_eq!(found[0].data["reveal_scene"], "s-5");

        // A hint earlier in the reading order earns it.
        let mut f = fact("f-twist");
        f.reader = vec![
            moment("s-3", RevealLevel::Hinted),
            moment("s-5", RevealLevel::Revealed),
        ];
        assert!(of_code(&run_default(&story(vec![fact_node(f)])), "unearned_twist").is_empty());

        // A hint after the reveal doesn't.
        let mut f = fact("f-twist");
        f.reader = vec![
            moment("s-5", RevealLevel::Revealed),
            moment("s-6", RevealLevel::Hinted),
        ];
        assert_eq!(
            of_code(&run_default(&story(vec![fact_node(f)])), "unearned_twist").len(),
            1
        );

        // Scenes outside the reading order don't count either way.
        let mut f = fact("f-twist");
        f.reader = vec![
            moment("s-alt", RevealLevel::Hinted),
            moment("s-5", RevealLevel::Revealed),
        ];
        let alt = scene("s-alt", "t-plot", Stage::Setup);
        let spec = story(vec![fact_node(f), alt]);
        assert_eq!(of_code(&run_default(&spec), "unearned_twist").len(), 1);
        let mut f = fact("f-twist");
        f.reader = vec![moment("s-alt", RevealLevel::Revealed)];
        let spec = story(vec![fact_node(f), scene("s-alt", "t-plot", Stage::Setup)]);
        assert!(of_code(&run_default(&spec), "unearned_twist").is_empty());
    }

    #[test]
    fn a_hint_nobody_pays_off_is_a_dangling_setup() {
        let hinted = |edit: &dyn Fn(&mut Fact)| {
            let mut f = fact("f-hint");
            f.reader = vec![moment("s-2", RevealLevel::Hinted)];
            edit(&mut f);
            story(vec![fact_node(f)])
        };
        let soft = run_default(&hinted(&|_| {}));
        let found = of_code(&soft, "dangling_setup");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].ids, ["f-hint"]);
        assert_eq!(found[0].data["hint_scene"], "s-2");

        // Revealed anywhere, even in an alternative scene, pays it off.
        let mut spec = hinted(&|f| f.reader.push(moment("s-alt", RevealLevel::Revealed)));
        spec.nodes.push(scene("s-alt", "t-plot", Stage::Setup));
        spec.nodes.sort_by(|a, b| a.id.cmp(&b.id));
        assert!(of_code(&run_default(&spec), "dangling_setup").is_empty());

        // A later scene that reads it uses it; an earlier one doesn't.
        let mut spec = hinted(&|_| {});
        edit_scene(&mut spec, "s-5", |s| s.reads.push("f-hint".into()));
        assert!(of_code(&run_default(&spec), "dangling_setup").is_empty());
        let mut spec = hinted(&|_| {});
        edit_scene(&mut spec, "s-1", |s| s.reads.push("f-hint".into()));
        assert_eq!(of_code(&run_default(&spec), "dangling_setup").len(), 1);

        // So does a character learning it later.
        let spec = hinted(&|f| {
            f.characters.push(crate::model::CharacterLearns {
                who: "p-mara".into(),
                learns_in: "s-6".into(),
                how: crate::model::LearnedHow::Told,
            })
        });
        assert!(of_code(&run_default(&spec), "dangling_setup").is_empty());
    }

    #[test]
    fn dangling_setup_waits_for_a_main_plot_resolution() {
        let mut f = fact("f-hint");
        f.reader = vec![moment("s-2", RevealLevel::Hinted)];
        let mut spec = story(vec![fact_node(f)]);
        // Move the Resolution scenes to another stage: no Resolution scene.
        edit_scene(&mut spec, "s-7", |s| {
            s.coverage.as_mut().unwrap().stage = Stage::Crisis
        });
        edit_scene(&mut spec, "s-8", |s| {
            s.coverage.as_mut().unwrap().stage = Stage::Crisis
        });
        let soft = run_default(&spec);
        assert!(of_code(&soft, "dangling_setup").is_empty());
        assert!(soft.dormant.contains("dangling_setup"));
    }

    fn pivot_story(ends: Option<crate::model::TimeAnchor>, value: &str) -> Spec {
        let mut belief = fact("f-belief");
        belief.kind = FactKind::Belief;
        belief.holder = Some("p-mara".into());
        belief.ends = ends;
        let mut owner = fact_node(belief);
        owner.facts.push({
            let mut f = fact("f-learn");
            f.true_from = Some(crate::model::TimeAnchor::In(InAnchor {
                scene: "s-3".into(),
            }));
            f
        });
        let decision = node(
            "d-chng",
            NodeBody::Decision(crate::model::Decision {
                question: "Does Mara change?".into(),
                area: DecisionArea::Hero,
                choices: vec![],
                value: Some(value.into()),
                options: vec![],
                evidence: vec![],
                pivot: Some("f-belief".into()),
                confidence: Confidence::Confirmed,
                review: Review::Cleared,
            }),
        );
        let mut spec = story(vec![owner, decision]);
        edit_arc(&mut spec, "a-main", |a| a.decisions.push("d-chng".into()));
        spec
    }

    fn in_scene(s: &str) -> Option<crate::model::TimeAnchor> {
        Some(crate::model::TimeAnchor::In(InAnchor { scene: s.into() }))
    }

    #[test]
    fn a_pivot_that_ends_long_before_the_climax_is_early() {
        let soft = run_default(&pivot_story(in_scene("s-2"), "changes"));
        let found = of_code(&soft, "early_pivot");
        assert_eq!(found.len(), 1, "{:#?}", soft.findings);
        assert_eq!(found[0].ids, ["d-chng", "f-belief"]);
        assert_eq!(found[0].data["scenes_before_climax"], 6);

        // Near the climax is fine (a quarter of 8 scenes is 2).
        assert!(
            of_code(
                &run_default(&pivot_story(in_scene("s-7"), "changes")),
                "early_pivot"
            )
            .is_empty()
        );
        assert!(
            of_code(
                &run_default(&pivot_story(in_scene("s-6"), "changes")),
                "early_pivot"
            )
            .is_empty()
        );
        assert_eq!(
            of_code(
                &run_default(&pivot_story(in_scene("s-5"), "changes")),
                "early_pivot"
            )
            .len(),
            1
        );
        // Holding firm has no pivot ending to time.
        assert!(
            of_code(
                &run_default(&pivot_story(in_scene("s-2"), "holds_firm")),
                "early_pivot"
            )
            .is_empty()
        );
        // An end tied to another fact lands where that fact begins (s-3).
        let at = Some(crate::model::TimeAnchor::At(AtAnchor {
            at: "f-learn".into(),
        }));
        assert_eq!(
            of_code(&run_default(&pivot_story(at, "changes")), "early_pivot").len(),
            1
        );
        // No end at all: nothing to judge.
        assert!(of_code(&run_default(&pivot_story(None, "changes")), "early_pivot").is_empty());
    }

    #[test]
    fn early_pivot_and_subplot_timing_wait_for_the_climax() {
        let mut spec = pivot_story(in_scene("s-2"), "changes");
        edit_arc(&mut spec, "a-main", |a| {
            a.landmarks.insert("climax".into(), None);
        });
        let soft = run_default(&spec);
        assert!(of_code(&soft, "early_pivot").is_empty());
        assert!(soft.dormant.contains("early_pivot"));
        assert!(soft.dormant.contains("subplot_after_climax"));
    }

    fn with_subplot(last_scene: &str) -> Spec {
        let mut spec = story(vec![
            node(
                "a-side",
                NodeBody::Arc(arc(ArcStatus::Active, &["t-side"], &[])),
            ),
            thread("t-side", "a-side", ThreadKind::BigPicture),
            scene("s-9", "t-side", Stage::Resolution),
        ]);
        spec.sequence = [
            "s-1", "s-2", "s-3", "s-4", "s-5", "s-6", "s-7", "s-8", "s-9",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        edit_scene(&mut spec, last_scene, |s| {
            s.coverage = Some(Coverage {
                thread: "t-side".into(),
                stage: Stage::Resolution,
            })
        });
        spec
    }

    #[test]
    fn a_subplot_that_ends_after_the_main_climax_is_noted() {
        let soft = run_default(&with_subplot("s-9"));
        let found = of_code(&soft, "subplot_after_climax");
        assert_eq!(found.len(), 1, "{:#?}", soft.findings);
        assert_eq!(found[0].ids, ["a-main", "a-side"]);
        assert_eq!(found[0].data["scenes_after_main_climax"], 1);

        // Ending before the climax is fine.
        let mut spec = with_subplot("s-5");
        edit_scene(&mut spec, "s-9", |s| {
            s.coverage = Some(Coverage {
                thread: "t-plot".into(),
                stage: Stage::Resolution,
            })
        });
        assert!(of_code(&run_default(&spec), "subplot_after_climax").is_empty());

        // Its own climax decides when it has one.
        let mut spec = with_subplot("s-9");
        edit_arc(&mut spec, "a-side", |a| {
            a.landmarks.insert("climax".into(), placed("s-5"));
        });
        assert!(of_code(&run_default(&spec), "subplot_after_climax").is_empty());
    }

    #[test]
    fn a_lopsided_movement_is_a_pacing_note() {
        // Boundaries at the first three scenes: 1, 1, 1 and 5 scenes of 8.
        let mut spec = story(vec![]);
        edit_arc(&mut spec, "a-main", |a| {
            a.landmarks.insert("mc_commits".into(), placed("s-1"));
            a.landmarks.insert("midpoint".into(), placed("s-2"));
            a.landmarks.insert("low_point".into(), placed("s-3"));
        });
        let soft = run_default(&spec);
        let found = of_code(&soft, "scene_count_pacing");
        assert_eq!(found.len(), 1, "{:#?}", soft.findings);
        assert_eq!(found[0].ids, ["a-main"]);
        assert_eq!(found[0].detail.as_deref(), Some("resolution:long"));
        assert_eq!(found[0].data["scenes"], 5);

        // Looser thresholds stay quiet; tighter ones also catch the short ones.
        let loose = CheckSettings {
            over_share: 3.0,
            ..CheckSettings::default()
        };
        assert!(of_code(&run(&spec, &loose), "scene_count_pacing").is_empty());
        let tight = CheckSettings {
            under_share: 0.9,
            ..CheckSettings::default()
        };
        assert_eq!(of_code(&run(&spec, &tight), "scene_count_pacing").len(), 4);
    }

    #[test]
    fn pacing_and_alignment_wait_for_the_skeleton() {
        let mut spec = story(vec![]);
        edit_arc(&mut spec, "a-main", |a| {
            a.landmarks.insert("midpoint".into(), None);
        });
        let soft = run_default(&spec);
        assert!(of_code(&soft, "scene_count_pacing").is_empty());
        assert!(of_code(&soft, "threads_out_of_alignment").is_empty());
        assert!(soft.dormant.contains("scene_count_pacing"));
        assert!(soft.dormant.contains("threads_out_of_alignment"));

        // Landmarks placed out of order are not a skeleton either.
        let mut spec = story(vec![]);
        edit_arc(&mut spec, "a-main", |a| {
            a.landmarks.insert("midpoint".into(), placed("s-7"));
        });
        assert!(run_default(&spec).dormant.contains("scene_count_pacing"));
    }

    #[test]
    fn a_thread_mostly_outside_its_movement_is_out_of_alignment() {
        let mut spec = story(vec![]);
        // Three of the five Resolution scenes sit in the Setup movement.
        for id in ["s-1", "s-2", "s-3"] {
            edit_scene(&mut spec, id, |s| {
                s.coverage.as_mut().unwrap().stage = Stage::Resolution
            });
        }
        let soft = run_default(&spec);
        let found = of_code(&soft, "threads_out_of_alignment");
        assert_eq!(found.len(), 1, "{:#?}", soft.findings);
        assert_eq!(found[0].ids, ["a-main", "t-plot"]);
        assert_eq!(found[0].detail.as_deref(), Some("resolution"));
        assert_eq!(found[0].data["scenes"], 5);
        assert_eq!(found[0].data["scenes_elsewhere"], 3);

        // Exactly half outside isn't more than half.
        let mut spec = story(vec![]);
        for id in ["s-1", "s-2"] {
            edit_scene(&mut spec, id, |s| {
                s.coverage.as_mut().unwrap().stage = Stage::Resolution
            });
        }
        assert!(of_code(&run_default(&spec), "threads_out_of_alignment").is_empty());
    }

    fn def(key: &str, inverse: Option<&str>) -> AttributeDef {
        AttributeDef {
            key: key.into(),
            relation: if inverse.is_some() {
                Relation::Inverse
            } else {
                Relation::Property
            },
            inverse: inverse.map(str::to_string),
            cardinality: CardinalitySpec::All(Cardinality::Many),
            summary: None,
        }
    }

    #[test]
    fn attribute_keys_that_look_alike_are_noted() {
        let mut spec = story(vec![]);
        spec.attributes = vec![def("eye_colour", None)];
        let mut f = fact("f-eyes");
        f.attribute = Some("eye_color".into());
        spec.nodes.push(fact_node(f));
        spec.nodes.sort_by(|a, b| a.id.cmp(&b.id));
        let soft = run_default(&spec);
        let found = of_code(&soft, "similar_attribute_keys");
        assert_eq!(found.len(), 1, "{:#?}", soft.findings);
        assert_eq!(found[0].detail.as_deref(), Some("eye_color|eye_colour"));
        assert!(found[0].ids.is_empty());
        assert_eq!(found[0].data["distance"], 1);

        // Underscores don't matter.
        let mut spec = story(vec![]);
        spec.attributes = vec![def("hair_color", None), def("haircolor", None)];
        assert_eq!(
            of_code(&run_default(&spec), "similar_attribute_keys").len(),
            1
        );

        // The two sides of a pair are meant to look alike; unrelated keys don't.
        let mut spec = story(vec![]);
        spec.attributes = vec![def("mentor_of", Some("mentee_of")), def("hometown", None)];
        assert!(of_code(&run_default(&spec), "similar_attribute_keys").is_empty());

        // Starter keys never clash with each other: with none of the story's
        // own keys, the starter vocabulary alone is quiet.
        assert!(of_code(&run_default(&story(vec![])), "similar_attribute_keys").is_empty());

        // The threshold is a setting.
        let mut spec = story(vec![]);
        spec.attributes = vec![def("eye_color", None), def("eye_colour", None)];
        let strict = CheckSettings {
            similar_key_max_distance: 0,
            ..CheckSettings::default()
        };
        assert!(of_code(&run(&spec, &strict), "similar_attribute_keys").is_empty());
    }

    #[test]
    fn results_are_ordered_and_repeatable() {
        let mut f = fact("f-twist");
        f.reader = vec![moment("s-5", RevealLevel::Revealed)];
        let spec = story(vec![fact_node(f)]);
        let (a, b) = (run_default(&spec), run_default(&spec));
        assert_eq!(a.findings, b.findings);
        let mut sorted = a.findings.clone();
        sorted.sort_by(|x, y| (x.code, &x.ids, &x.detail).cmp(&(y.code, &y.ids, &y.detail)));
        assert_eq!(a.findings, sorted);
        assert_eq!(version_of("early_pivot"), Some(1));
        assert_eq!(version_of("nope"), None);
        assert_eq!(distance("kitten", "sitting"), 3);
        assert_eq!(distance("", "abc"), 3);
        assert_eq!(distance("abc", ""), 3);
        assert_eq!(distance("same", "same"), 0);
        assert_eq!(distance("flaw", "lawn"), 2);
        // Characters, not bytes.
        assert_eq!(distance("café", "cafe"), 1);
    }
}
