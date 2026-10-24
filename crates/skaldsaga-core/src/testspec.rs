//! Small builders for in-memory specs, shared by the tests of the checks and
//! of `complete`.

use indexmap::IndexMap;

use crate::model::{
    Arc, ArcStatus, BeatPlacement, Confidence, Coverage, Fact, FactKind, Node, NodeBody,
    ReaderMoment, RevealLevel, Review, Scene, SceneStatus, Stage, StageEntry, Thread, ThreadKind,
};
use crate::spec::Spec;

pub(crate) fn node(id: &str, body: NodeBody) -> Node {
    Node {
        id: id.into(),
        name: id.into(),
        aliases: vec![],
        summary: None,
        revision: 1,
        body,
        facts: vec![],
        notes: String::new(),
    }
}

pub(crate) fn fact(id: &str) -> Fact {
    Fact {
        id: id.into(),
        kind: FactKind::Truth,
        revision: 1,
        statement: id.into(),
        attribute: None,
        value: None,
        holder: None,
        contradicts: None,
        question: None,
        options: vec![],
        true_from: None,
        ends: None,
        ended_by: None,
        reader: vec![],
        characters: vec![],
        links: vec![],
        confidence: Confidence::Confirmed,
        review: Review::Unreviewed,
        source: "interview".into(),
        role: None,
    }
}

pub(crate) fn moment(scene: &str, level: RevealLevel) -> ReaderMoment {
    ReaderMoment {
        scene: scene.into(),
        level,
    }
}

pub(crate) fn scene(id: &str, thread: &str, stage: Stage) -> Node {
    node(
        id,
        NodeBody::Scene(Scene {
            pov: None,
            location: None,
            participants: vec![],
            coverage: Some(Coverage {
                thread: thread.into(),
                stage,
            }),
            purpose: None,
            when: None,
            beats: vec![],
            reads: vec![],
            turn: None,
            status: SceneStatus::Slot,
        }),
    )
}

pub(crate) fn placed(scene: &str) -> Option<BeatPlacement> {
    Some(BeatPlacement {
        scene: scene.into(),
        confidence: Confidence::Provisional,
    })
}

pub(crate) fn arc(status: ArcStatus, threads: &[&str], landmarks: &[(&str, &str)]) -> Arc {
    Arc {
        status,
        threads: threads.iter().map(|s| s.to_string()).collect(),
        decisions: vec![],
        premise: None,
        dramatic_question: None,
        thematic_question: None,
        framework: None,
        landmarks: landmarks
            .iter()
            .map(|(k, s)| (k.to_string(), placed(s)))
            .collect::<IndexMap<_, _>>(),
        beats: IndexMap::new(),
        characters: vec![],
    }
}

pub(crate) fn thread(id: &str, arc_id: &str, kind: ThreadKind) -> Node {
    node(
        id,
        NodeBody::Thread(Thread {
            kind,
            arc: arc_id.into(),
            characters: vec![],
            decisions: vec![],
            stages: Thread::empty_stages()
                .into_iter()
                .map(|e| StageEntry {
                    summary: Some("x".into()),
                    ..e
                })
                .collect(),
        }),
    )
}

pub(crate) fn spec_of(mut nodes: Vec<Node>, sequence: &[&str]) -> Spec {
    nodes.sort_by(|a, b| a.id.cmp(&b.id));
    Spec {
        nodes,
        sequence: sequence.iter().map(|s| s.to_string()).collect(),
        attributes: vec![],
    }
}

pub(crate) const ALL8: [&str; 8] = ["s-1", "s-2", "s-3", "s-4", "s-5", "s-6", "s-7", "s-8"];
pub(crate) const STAGES8: [Stage; 8] = [
    Stage::Setup,
    Stage::Setup,
    Stage::Complications,
    Stage::Complications,
    Stage::Crisis,
    Stage::Crisis,
    Stage::Resolution,
    Stage::Resolution,
];

/// Eight scenes on one main plot, two per movement, with the four
/// blocking landmarks at s-2, s-4, s-6 and s-8. `extra` nodes are added.
pub(crate) fn story(extra: Vec<Node>) -> Spec {
    let mut nodes = vec![
        node(
            "a-main",
            NodeBody::Arc(arc(
                ArcStatus::Main,
                &["t-plot"],
                &[
                    ("mc_commits", "s-2"),
                    ("midpoint", "s-4"),
                    ("low_point", "s-6"),
                    ("climax", "s-8"),
                ],
            )),
        ),
        thread("t-plot", "a-main", ThreadKind::BigPicture),
    ];
    for (i, stage) in STAGES8.iter().enumerate() {
        nodes.push(scene(&format!("s-{}", i + 1), "t-plot", *stage));
    }
    nodes.extend(extra);
    spec_of(nodes, &ALL8)
}

pub(crate) fn fact_node(f: Fact) -> Node {
    let mut n = node("p-mara", NodeBody::Person);
    n.facts.push(f);
    n
}

pub(crate) fn edit_arc(spec: &mut Spec, id: &str, edit: impl FnOnce(&mut Arc)) {
    let node = spec.nodes.iter_mut().find(|n| n.id == id).unwrap();
    let NodeBody::Arc(a) = &mut node.body else {
        panic!("{id} isn't an arc");
    };
    edit(a);
}

pub(crate) fn edit_scene(spec: &mut Spec, id: &str, edit: impl FnOnce(&mut Scene)) {
    let node = spec.nodes.iter_mut().find(|n| n.id == id).unwrap();
    let NodeBody::Scene(s) = &mut node.body else {
        panic!("{id} isn't a scene");
    };
    edit(s);
}
