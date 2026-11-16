//! The typed spec model: nodes, facts and the fields each node type adds.
//!
//! Field names and values follow HANDOFF, Node format. Every type rejects
//! unknown keys, so a typo in a hand-edited file is reported instead of being
//! silently dropped the next time the committer rewrites the file. Optional
//! fields that are empty are left out when written, which keeps node files
//! short enough for a writer to read.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Shared labels

/// How sure the writer is (HANDOFF, Governance: two independent labels).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Confirmed,
    Provisional,
    Unresolved,
}

/// Whether the housekeeper has cleared it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Review {
    Unreviewed,
    Cleared,
}

/// The four stages every thread moves through (HANDOFF, Coverage grid).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Setup,
    Complications,
    Crisis,
    Resolution,
}

impl Stage {
    pub const ALL: [Stage; 4] = [
        Stage::Setup,
        Stage::Complications,
        Stage::Crisis,
        Stage::Resolution,
    ];
}

// ---------------------------------------------------------------------------
// In-world time

/// How exact a time offset is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Precision {
    Exact,
    Approx,
}

/// The bare word `unknown`, for a time nobody has pinned down yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnknownTime {
    Unknown,
}

/// A point in in-world time. Relative by default: anchored to a fact or a
/// scene, with an optional offset. An absolute date is used only when the
/// writer gives one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TimeAnchor {
    /// `unknown`
    Unknown(UnknownTime),
    /// `{after: f-…, offset: "3 days", precision: approx}`; `after` may name a fact or a scene.
    After(AfterAnchor),
    /// `{in: s-…}`: during that scene.
    In(InAnchor),
    /// `{at: f-…}`: at the same moment as that fact (used by `ends`).
    At(AtAnchor),
    /// `{date: "1923-04-02"}`
    Date(DateAnchor),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AfterAnchor {
    pub after: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub precision: Option<Precision>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InAnchor {
    #[serde(rename = "in")]
    pub scene: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AtAnchor {
    pub at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DateAnchor {
    pub date: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub precision: Option<Precision>,
}

// ---------------------------------------------------------------------------
// Facts

/// A truth about the story world, or someone's belief about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactKind {
    Truth,
    Belief,
}

/// How far a reader moment goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RevealLevel {
    Hinted,
    Revealed,
}

/// A scene where the reader is hinted at, or told, a fact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReaderMoment {
    pub scene: String,
    pub level: RevealLevel,
}

/// How a character learns a fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LearnedHow {
    Saw,
    Told,
    WorkedOut,
}

/// A character learning a fact in a scene.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterLearns {
    pub who: String,
    pub learns_in: String,
    pub how: LearnedHow,
}

/// Pressure along the way (HANDOFF, open item 17).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactRole {
    Cost,
    SmallWin,
    Warning,
    RequiredStep,
}

/// One ID'd fact inside an entity node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fact {
    pub id: String,
    pub kind: FactKind,
    /// Bumped by the committer whenever this fact changes.
    pub revision: u32,
    pub statement: String,
    /// Property key (`eye_color`) or relation (`sibling_of`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribute: Option<String>,
    /// Property value, or the other entity's ID for a relation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Beliefs only: a person ID, or `reader`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub holder: Option<String>,
    /// Beliefs only: the truth this belief contradicts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contradicts: Option<String>,
    /// Unresolved only: what's undecided, in plain words.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
    /// Unresolved only: the candidate answers; empty means to be decided.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub true_from: Option<TimeAnchor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ends: Option<TimeAnchor>,
    /// The fact that replaced this one in the story.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_by: Option<String>,
    /// At most one reveal (checked by the dependency checker).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reader: Vec<ReaderMoment>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub characters: Vec<CharacterLearns>,
    /// Other entities this fact involves.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<String>,
    pub confidence: Confidence,
    pub review: Review,
    /// `interview`, `harvest:<file>`, `research:<source>` or `placeholder`.
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<FactRole>,
}

// ---------------------------------------------------------------------------
// Node types

/// The `type` field of a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    Person,
    Place,
    Group,
    Item,
    Scene,
    Decision,
    Thread,
    Style,
    Arc,
    /// A held idea (a brainstormed idea or a parked tangent). Not part of the
    /// story until the writer picks it.
    Idea,
}

impl NodeType {
    pub const ALL: [NodeType; 10] = [
        NodeType::Person,
        NodeType::Place,
        NodeType::Group,
        NodeType::Item,
        NodeType::Scene,
        NodeType::Decision,
        NodeType::Thread,
        NodeType::Style,
        NodeType::Arc,
        NodeType::Idea,
    ];

    /// The ID kind nodes of this type use.
    pub fn id_kind(self) -> crate::ids::IdKind {
        use crate::ids::IdKind;
        match self {
            NodeType::Person => IdKind::Person,
            NodeType::Place => IdKind::Place,
            NodeType::Group => IdKind::Group,
            NodeType::Item => IdKind::Item,
            NodeType::Scene => IdKind::Scene,
            NodeType::Decision => IdKind::Decision,
            NodeType::Thread => IdKind::Thread,
            NodeType::Style => IdKind::Style,
            NodeType::Arc => IdKind::Arc,
            NodeType::Idea => IdKind::Idea,
        }
    }

    /// The folder under `spec/` that holds nodes of this type.
    pub fn folder(self) -> &'static str {
        match self {
            NodeType::Person => "people",
            NodeType::Place => "places",
            NodeType::Group => "groups",
            NodeType::Item => "items",
            NodeType::Scene => "scenes",
            NodeType::Decision => "decisions",
            NodeType::Thread => "threads",
            NodeType::Style => "style",
            NodeType::Arc => "arcs",
            NodeType::Idea => "ideas",
        }
    }

    pub fn from_folder(folder: &str) -> Option<NodeType> {
        NodeType::ALL.into_iter().find(|t| t.folder() == folder)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            NodeType::Person => "person",
            NodeType::Place => "place",
            NodeType::Group => "group",
            NodeType::Item => "item",
            NodeType::Scene => "scene",
            NodeType::Decision => "decision",
            NodeType::Thread => "thread",
            NodeType::Style => "style",
            NodeType::Arc => "arc",
            NodeType::Idea => "idea",
        }
    }
}

/// Which thread and stage a scene covers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Coverage {
    pub thread: String,
    pub stage: Stage,
}

/// The scene's change: what the POV character wants, what gets in the way,
/// and how it ends differently than it began.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Turn {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wants: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub against: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ends: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneStatus {
    Slot,
    Outlined,
    Drafted,
    Reviewed,
    Approved,
}

/// Fields a scene adds. Writes, hints and reveals are not stored here; they
/// are generated from the facts that point at the scene.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scene {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pov: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub participants: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coverage: Option<Coverage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<TimeAnchor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub beats: Vec<String>,
    /// Facts this scene relies on (declared).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reads: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn: Option<Turn>,
    pub status: SceneStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionArea {
    Hero,
    Plot,
    Theme,
    Challenger,
    Relationship,
}

/// Fields a shape decision adds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub question: String,
    pub area: DecisionArea,
    /// Empty means the writer answers in their own words.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<String>,
    /// The answer; `not_explored` records a "keep it simple" choice.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
    /// Scenes and facts the inference came from.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<String>,
    /// The belief whose ending marks the change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pivot: Option<String>,
    pub confidence: Confidence,
    pub review: Review,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThreadKind {
    Personal,
    Challenger,
    Relationship,
    BigPicture,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThreadCharacter {
    pub who: String,
    pub role: String,
}

/// One stage of a thread. A missing summary is an empty grid cell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageEntry {
    pub stage: Stage,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<Confidence>,
}

/// Fields a thread adds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Thread {
    pub kind: ThreadKind,
    pub arc: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub characters: Vec<ThreadCharacter>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub decisions: Vec<String>,
    /// Always the four stages, in order (checked when loading).
    pub stages: Vec<StageEntry>,
}

impl Thread {
    /// Four empty stages, for a new thread.
    pub fn empty_stages() -> Vec<StageEntry> {
        Stage::ALL
            .into_iter()
            .map(|stage| StageEntry {
                stage,
                summary: None,
                confidence: None,
            })
            .collect()
    }
}

/// Viewpoint and style settings, used as defaults and per-viewpoint overrides.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StyleRules {
    /// `first`, `second` or `third`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub person: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tense: Option<String>,
    /// `close`, `distant` or `omniscient`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distance: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tone: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Viewpoint {
    pub who: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overrides: Option<StyleRules>,
}

/// Fields the viewpoint and style node adds.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Style {
    #[serde(default)]
    pub defaults: StyleRules,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub taste_anchors: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub viewpoints: Vec<Viewpoint>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub voice_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArcStatus {
    Main,
    Fog,
    Active,
    Shelved,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PremiseVersion {
    pub date: String,
    pub text: String,
}

/// The premise in the writer's own words, versioned for drift checks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Premise {
    pub current: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub history: Vec<PremiseVersion>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Framework {
    /// A framework name, `undecided`, or `none`.
    pub value: String,
    pub confidence: Confidence,
}

/// The tier 1 landmark keys on an arc, in story order (HANDOFF, Coverage grid
/// and movements; the key names were chosen in build step 6).
pub const LANDMARKS: [&str; 9] = [
    "normal_world",
    "inciting_incident",
    "mc_commits",
    "rising_stakes",
    "midpoint",
    "things_fall_apart",
    "low_point",
    "climax",
    "new_normal",
];

/// The landmarks that end the first three movements: Setup ends at the scene
/// where the MC commits, Complications at the midpoint, Crisis at the low point.
pub const MOVEMENT_BOUNDARIES: [&str; 3] = ["mc_commits", "midpoint", "low_point"];

/// A landmark or framework beat placed on a scene.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BeatPlacement {
    pub scene: String,
    pub confidence: Confidence,
}

/// Fields an arc (the main story, or one subplot) adds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Arc {
    pub status: ArcStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub threads: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub decisions: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub premise: Option<Premise>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dramatic_question: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thematic_question: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub framework: Option<Framework>,
    /// Tier 1: one entry per landmark; `null` means not found yet.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub landmarks: IndexMap<String, Option<BeatPlacement>>,
    /// Tier 2: from the chosen framework.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub beats: IndexMap<String, Option<BeatPlacement>>,
    /// Fog only: who the subplot involves.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub characters: Vec<String>,
}

/// The most characters an idea's text may have (*Decided November 8, 2026*:
/// one or two sentences, enforced as a character count because a sentence
/// count is unreliable). A saved idea is read back into the interviewer's
/// prompt, so a short cap keeps that text small.
pub const IDEA_TEXT_MAX_CHARS: usize = 300;

/// The most story parts one idea may name in `touches` (*Proposed*: a cap in
/// the spirit of the other size limits, so a hand-edited file can't grow a
/// list without end).
pub const IDEA_TOUCHES_MAX: usize = 24;

/// Where an idea stands (*Decided November 8, 2026*). `set_aside` means the
/// writer waved it off, so it never resurfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdeaStatus {
    Held,
    Picked,
    SetAside,
}

impl IdeaStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            IdeaStatus::Held => "held",
            IdeaStatus::Picked => "picked",
            IdeaStatus::SetAside => "set_aside",
        }
    }
}

/// Where an idea came from (*Decided November 8, 2026*): a fixed choice, no
/// session reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdeaOrigin {
    /// Parked by the drift guard.
    Tangent,
    /// Offered in a brainstorming round.
    Brainstorm,
}

impl IdeaOrigin {
    pub fn as_str(self) -> &'static str {
        match self {
            IdeaOrigin::Tangent => "tangent",
            IdeaOrigin::Brainstorm => "brainstorm",
        }
    }
}

/// Fields an idea adds. It stores its text and the parts of the story it
/// touches, not a change list: that is built when the writer leans toward the
/// idea, from the story as it is then, so a preview never goes stale.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Idea {
    /// One or two sentences, at most [`IDEA_TEXT_MAX_CHARS`] characters.
    pub text: String,
    pub status: IdeaStatus,
    pub origin: IdeaOrigin,
    /// Node IDs, or the stage codes `premise`, `people`, `setup`,
    /// `complications`, `crisis`, `resolution`, for a loose idea with no
    /// single node to attach to.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub touches: Vec<String>,
}

impl Idea {
    /// A short label for the node's required `name`, taken from the text
    /// (*Proposed*): the first words, cut at a word boundary.
    pub fn label_for(text: &str) -> String {
        const LABEL_MAX: usize = 60;
        let flat: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
        if flat.chars().count() <= LABEL_MAX {
            return flat;
        }
        let cut: String = flat.chars().take(LABEL_MAX).collect();
        let trimmed = match cut.rsplit_once(' ') {
            Some((head, _)) if !head.is_empty() => head.to_string(),
            _ => cut,
        };
        format!("{trimmed}...")
    }

    /// Why this idea can't be stored, in plain words, or `None` when it is
    /// well formed: the text is non-empty and short enough, and every
    /// `touches` entry is a stage code or the ID of a story node.
    pub fn problem(&self) -> Option<String> {
        if self.text.trim().is_empty() {
            return Some("an idea needs some text".into());
        }
        if self.text.chars().count() > IDEA_TEXT_MAX_CHARS {
            return Some(format!(
                "an idea's text is limited to {IDEA_TEXT_MAX_CHARS} characters; shorten it to one or two sentences"
            ));
        }
        if self.touches.len() > IDEA_TOUCHES_MAX {
            return Some(format!(
                "an idea can name at most {IDEA_TOUCHES_MAX} story parts"
            ));
        }
        let mut seen = std::collections::HashSet::new();
        for part in &self.touches {
            if !is_idea_touch(part) {
                return Some(format!(
                    "'{part}' isn't a story part an idea can touch (use a node ID or one of the stage names)"
                ));
            }
            if !seen.insert(part.as_str()) {
                return Some(format!("'{part}' is listed twice in touches"));
            }
        }
        None
    }
}

/// True if `part` is something an idea may name in `touches`: a stage code,
/// or the well-formed ID of a node of any type except another idea. (A fact
/// lives inside a node, so a fact ID is not a node ID; an idea names the node.)
pub fn is_idea_touch(part: &str) -> bool {
    if crate::marker::Topic::parse(part).is_some() {
        return true;
    }
    NodeType::ALL
        .into_iter()
        .filter(|t| *t != NodeType::Idea)
        .any(|t| crate::ids::is_kind(part, t.id_kind()))
}

/// The fields that depend on a node's type.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeBody {
    Person,
    Place,
    Group,
    Item,
    Scene(Scene),
    Decision(Decision),
    Thread(Thread),
    Style(Style),
    Arc(Arc),
    Idea(Idea),
}

impl NodeBody {
    pub fn node_type(&self) -> NodeType {
        match self {
            NodeBody::Person => NodeType::Person,
            NodeBody::Place => NodeType::Place,
            NodeBody::Group => NodeType::Group,
            NodeBody::Item => NodeType::Item,
            NodeBody::Scene(_) => NodeType::Scene,
            NodeBody::Decision(_) => NodeType::Decision,
            NodeBody::Thread(_) => NodeType::Thread,
            NodeBody::Style(_) => NodeType::Style,
            NodeBody::Arc(_) => NodeType::Arc,
            NodeBody::Idea(_) => NodeType::Idea,
        }
    }
}

/// One spec node: the common core, the type's own fields, its facts, and the
/// free prose notes below the front matter (which no checker relies on).
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub id: String,
    pub name: String,
    pub aliases: Vec<String>,
    /// The one-line index summary.
    pub summary: Option<String>,
    /// Bumped by the committer whenever a node-level field changes.
    pub revision: u32,
    pub body: NodeBody,
    pub facts: Vec<Fact>,
    pub notes: String,
}

impl Node {
    pub fn node_type(&self) -> NodeType {
        self.body.node_type()
    }
}
