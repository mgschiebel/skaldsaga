//! Entry routing: which way a session opens (HANDOFF, Ready to build, Slice 1,
//! item 1; design in `design-story.md`, Audience and interaction principles).
//!
//! The program, not the model, decides which entry condition applies, from
//! the folder state and a single yes/no: whether the writer typed text with
//! the command. The text itself never reaches this code, so story text can't
//! steer the routing. Routing only looks; it never runs `init`, never writes,
//! never creates a folder.

use std::path::Path;

use crate::marker::{self, State};
use crate::project::{Location, Project};
use crate::spec;

/// How a session opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Condition {
    /// No story found, nothing typed.
    NoStoryBare,
    /// No story found, the writer typed an idea (or a draft, or a request to
    /// brainstorm; the model tells those apart, not the program).
    NoStoryWithText,
    /// Left mid-thread (including an answer never confirmed).
    StoryMidThread,
    /// Left at a stopping point.
    StoryStoppingPoint,
    /// A story exists and the writer typed text: ask new story or more of this one.
    StoryWithText,
    /// A story folder with nothing committed yet: straight to the opener.
    StoryEmpty,
    /// The story folder can't be read safely: say so, change nothing.
    StoryDamaged,
}

impl Condition {
    pub fn as_str(self) -> &'static str {
        match self {
            Condition::NoStoryBare => "no_story_bare",
            Condition::NoStoryWithText => "no_story_with_text",
            Condition::StoryMidThread => "story_mid_thread",
            Condition::StoryStoppingPoint => "story_stopping_point",
            Condition::StoryWithText => "story_with_text",
            Condition::StoryEmpty => "story_empty",
            Condition::StoryDamaged => "story_damaged",
        }
    }

    /// Whether the conversation for this condition is built. The others say
    /// plainly that they are not built yet.
    pub fn is_built(self) -> bool {
        !matches!(
            self,
            Condition::StoryMidThread | Condition::StoryStoppingPoint
        )
    }
}

/// Why a story folder counts as damaged. Codes only, never paths or story text.
pub const DAMAGE_SPEC_UNREADABLE: &str = "spec_unreadable";
pub const DAMAGE_SPEC_PROBLEMS: &str = "spec_problems";
pub const DAMAGE_MARKER_UNUSABLE: &str = "marker_unusable";

/// The routing for one session start.
#[derive(Debug, Clone, PartialEq)]
pub struct Routed {
    /// Where the story folder is, seen from the starting folder.
    pub location: Location,
    /// The entry condition; `None` only when several story folders were found
    /// and the writer has to choose one first.
    pub condition: Option<Condition>,
    /// For `StoryDamaged`: the damage codes.
    pub damage: Vec<&'static str>,
}

/// Routes a session that starts in `start`. `has_text` is whether the writer
/// typed anything with the command.
pub fn route(start: &Path, has_text: bool) -> Routed {
    let location = crate::project::locate(start);
    let Some(project) = location.project() else {
        // `Several` waits for the writer's choice; everything else is "no
        // story found" (a lone candidate repository is the skill's question).
        let condition = match location {
            Location::Several(_) => None,
            _ if has_text => Some(Condition::NoStoryWithText),
            _ => Some(Condition::NoStoryBare),
        };
        return Routed {
            location,
            condition,
            damage: Vec::new(),
        };
    };
    let (condition, damage) = route_story(&project, has_text);
    Routed {
        location,
        condition: Some(condition),
        damage,
    }
}

/// Routes inside a story folder that exists.
fn route_story(project: &Project, has_text: bool) -> (Condition, Vec<&'static str>) {
    let mut damage = Vec::new();
    let has_content = match spec::load(&project.spec_dir()) {
        Err(_) => {
            damage.push(DAMAGE_SPEC_UNREADABLE);
            false
        }
        Ok(loaded) => {
            if !loaded.problems.is_empty() {
                damage.push(DAMAGE_SPEC_PROBLEMS);
            }
            !loaded.spec.nodes.is_empty()
        }
    };
    let last = match marker::load(project) {
        Ok(m) => m,
        Err(_) => {
            damage.push(DAMAGE_MARKER_UNUSABLE);
            None
        }
    };
    if !damage.is_empty() {
        return (Condition::StoryDamaged, damage);
    }

    if has_text {
        // Something is already under way (a committed fact, or an answer in
        // the marker): ask whether this is a new story or more of this one.
        // Nothing yet: the text is the idea for the opener (Proposed).
        let under_way = has_content || last.is_some();
        let condition = if under_way {
            Condition::StoryWithText
        } else {
            Condition::StoryEmpty
        };
        return (condition, damage);
    }
    let condition = match last.map(|m| m.state) {
        Some(State::StoppingPoint) => Condition::StoryStoppingPoint,
        Some(State::MidThread) => Condition::StoryMidThread,
        // Content but no record of how the last session ended (Proposed):
        // read it as a thread still open.
        None if has_content => Condition::StoryMidThread,
        None => Condition::StoryEmpty,
    };
    (condition, damage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{self, IdKind};
    use crate::model::NodeBody;
    use crate::node_file;
    use crate::testspec;
    use crate::testutil::TempDir;
    use std::path::PathBuf;

    // Fixture folders, one per entry condition. Each builds only what the
    // condition needs, by hand, so the tests don't depend on `init`.

    fn mkdir(path: &Path) {
        std::fs::create_dir_all(path).unwrap();
    }

    /// An initialized story folder with nothing committed.
    fn empty_story(dir: &TempDir) -> PathBuf {
        mkdir(&dir.path().join(".harness"));
        dir.path().to_path_buf()
    }

    /// A story folder holding one person.
    fn story_with_content(dir: &TempDir) -> PathBuf {
        let root = empty_story(dir);
        let id = ids::new_id(IdKind::Person).unwrap();
        let node = testspec::node(&id, NodeBody::Person);
        mkdir(&root.join("spec/people"));
        std::fs::write(
            root.join(format!("spec/people/{id}.md")),
            node_file::render_node(&node).unwrap(),
        )
        .unwrap();
        root
    }

    fn write_marker(root: &Path, json: &str) {
        mkdir(&root.join(".harness/writer"));
        std::fs::write(root.join(".harness/writer/last-session.json"), json).unwrap();
    }

    const MID_UNCONFIRMED: &str = r#"{"format":1,"state":"mid_thread","last_answer":{"words":"a smuggler","confirmed":false},"date":"2026-10-25"}"#;
    const MID_CONFIRMED: &str = r#"{"format":1,"state":"mid_thread","last_answer":{"words":"a smuggler","confirmed":true},"date":"2026-10-25"}"#;
    const STOPPING: &str = r#"{"format":1,"state":"stopping_point","date":"2026-10-25"}"#;

    fn cond(start: &Path, has_text: bool) -> Option<Condition> {
        route(start, has_text).condition
    }

    #[test]
    fn no_story_found_routes_by_whether_text_was_typed() {
        let dir = TempDir::new();
        assert_eq!(cond(dir.path(), false), Some(Condition::NoStoryBare));
        assert_eq!(cond(dir.path(), true), Some(Condition::NoStoryWithText));
        assert_eq!(route(dir.path(), false).location, Location::Nothing);
    }

    #[test]
    fn a_missing_folder_is_no_story() {
        let dir = TempDir::new();
        let gone = dir.path().join("gone");
        assert_eq!(cond(&gone, false), Some(Condition::NoStoryBare));
    }

    #[test]
    fn a_lone_repository_is_still_no_story_but_is_reported() {
        let dir = TempDir::new();
        mkdir(&dir.path().join("repo/.git"));
        let routed = route(dir.path(), false);
        assert_eq!(routed.condition, Some(Condition::NoStoryBare));
        assert!(matches!(routed.location, Location::Candidate { .. }));
    }

    #[test]
    fn several_stories_need_a_choice_before_any_routing() {
        let dir = TempDir::new();
        mkdir(&dir.path().join("a/.harness"));
        mkdir(&dir.path().join("b/.harness"));
        let routed = route(dir.path(), true);
        assert_eq!(routed.condition, None);
        assert!(matches!(routed.location, Location::Several(_)));
    }

    #[test]
    fn a_story_with_nothing_committed_goes_straight_to_the_opener() {
        let dir = TempDir::new();
        let root = empty_story(&dir);
        assert_eq!(cond(&root, false), Some(Condition::StoryEmpty));
        // Text typed into an empty story is the idea for the opener (Proposed).
        assert_eq!(cond(&root, true), Some(Condition::StoryEmpty));
    }

    #[test]
    fn a_story_below_the_start_folder_routes_like_one_here() {
        let dir = TempDir::new();
        mkdir(&dir.path().join("story/.harness"));
        let routed = route(dir.path(), false);
        assert_eq!(routed.condition, Some(Condition::StoryEmpty));
        assert!(matches!(routed.location, Location::Below { .. }));
    }

    #[test]
    fn a_story_with_content_and_text_asks_new_or_more() {
        let dir = TempDir::new();
        let root = story_with_content(&dir);
        assert_eq!(cond(&root, true), Some(Condition::StoryWithText));
    }

    #[test]
    fn a_story_with_content_and_no_marker_is_mid_thread() {
        // Proposed: nothing says how the last session ended, so the safe
        // reading is a thread still open.
        let dir = TempDir::new();
        let root = story_with_content(&dir);
        assert_eq!(cond(&root, false), Some(Condition::StoryMidThread));
    }

    #[test]
    fn an_unconfirmed_answer_is_mid_thread_even_with_nothing_committed() {
        let dir = TempDir::new();
        let root = empty_story(&dir);
        write_marker(&root, MID_UNCONFIRMED);
        assert_eq!(cond(&root, false), Some(Condition::StoryMidThread));
    }

    #[test]
    fn typed_text_while_an_answer_is_unconfirmed_asks_new_or_more() {
        let dir = TempDir::new();
        let root = empty_story(&dir);
        write_marker(&root, MID_UNCONFIRMED);
        assert_eq!(cond(&root, true), Some(Condition::StoryWithText));
    }

    #[test]
    fn a_confirmed_last_answer_is_mid_thread() {
        let dir = TempDir::new();
        let root = story_with_content(&dir);
        write_marker(&root, MID_CONFIRMED);
        assert_eq!(cond(&root, false), Some(Condition::StoryMidThread));
        assert_eq!(cond(&root, true), Some(Condition::StoryWithText));
    }

    #[test]
    fn a_stopping_point_marker_routes_to_the_stopping_point() {
        let dir = TempDir::new();
        let root = story_with_content(&dir);
        write_marker(&root, STOPPING);
        assert_eq!(cond(&root, false), Some(Condition::StoryStoppingPoint));
        assert_eq!(cond(&root, true), Some(Condition::StoryWithText));
    }

    #[test]
    fn an_unreadable_story_file_is_damage_whatever_was_typed() {
        let dir = TempDir::new();
        let root = story_with_content(&dir);
        std::fs::write(root.join("spec/people/p-broken.md"), "not a node file").unwrap();
        for text in [false, true] {
            let routed = route(&root, text);
            assert_eq!(routed.condition, Some(Condition::StoryDamaged));
            assert_eq!(routed.damage, vec![DAMAGE_SPEC_PROBLEMS]);
        }
    }

    #[test]
    fn a_marker_we_cant_use_is_damage() {
        let dir = TempDir::new();
        let root = empty_story(&dir);
        write_marker(&root, "not json");
        let routed = route(&root, false);
        assert_eq!(routed.condition, Some(Condition::StoryDamaged));
        assert_eq!(routed.damage, vec![DAMAGE_MARKER_UNUSABLE]);
    }

    #[test]
    fn damage_names_every_kind_found() {
        let dir = TempDir::new();
        let root = story_with_content(&dir);
        std::fs::write(root.join("spec/people/p-broken.md"), "nope").unwrap();
        write_marker(&root, "nope");
        let routed = route(&root, false);
        assert_eq!(
            routed.damage,
            vec![DAMAGE_SPEC_PROBLEMS, DAMAGE_MARKER_UNUSABLE]
        );
    }

    #[test]
    fn routing_never_creates_or_changes_anything() {
        fn listing(root: &Path) -> Vec<String> {
            let mut out = Vec::new();
            let mut stack = vec![root.to_path_buf()];
            while let Some(d) = stack.pop() {
                for e in std::fs::read_dir(&d).unwrap().flatten() {
                    let p = e.path();
                    let meta = std::fs::symlink_metadata(&p).unwrap();
                    let rel = p.strip_prefix(root).unwrap().display().to_string();
                    out.push(format!("{rel}:{}", meta.len()));
                    if meta.is_dir() {
                        stack.push(p);
                    }
                }
            }
            out.sort();
            out
        }
        for build in [
            |_: &TempDir| {},
            |d: &TempDir| {
                empty_story(d);
            },
            |d: &TempDir| {
                let r = story_with_content(d);
                write_marker(&r, MID_UNCONFIRMED);
            },
        ] {
            let dir = TempDir::new();
            build(&dir);
            let before = listing(dir.path());
            for text in [false, true] {
                route(dir.path(), text);
            }
            assert_eq!(listing(dir.path()), before);
        }
    }

    #[test]
    fn only_the_two_recap_conditions_are_not_built() {
        let built: Vec<_> = [
            Condition::NoStoryBare,
            Condition::NoStoryWithText,
            Condition::StoryMidThread,
            Condition::StoryStoppingPoint,
            Condition::StoryWithText,
            Condition::StoryEmpty,
            Condition::StoryDamaged,
        ]
        .into_iter()
        .filter(|c| !c.is_built())
        .map(Condition::as_str)
        .collect();
        assert_eq!(built, vec!["story_mid_thread", "story_stopping_point"]);
    }
}
