//! Held ideas: the program side of the brainstorm path (HANDOFF, Planning
//! session 7; layout in `design-system.md`, "Idea").
//!
//! An idea is a spec node of its own (`spec/ideas/idea-xxxxxxxx.md`): one or
//! two sentences of text, where it came from, the parts of the story it
//! touches, and where it stands (`held`, `picked`, `set_aside`). Three
//! operations work on them:
//!
//! * [`add`] saves a new held idea;
//! * [`mark`] records that the writer picked an idea or waved it off;
//! * [`list`] is read-only and returns the held ideas, optionally only those
//!   touching one part of the story.
//!
//! Ideas are written straight into the story folder by this program, not
//! filed as proposals (*Proposed*): an idea is not part of the story until the
//! writer picks it, so the story checks have nothing to say about it. Like the
//! session notes, a write never makes a snapshot of its own; the result names
//! the file so the skill can stage it with the next real save. The rules for
//! anything read or written here are the story-folder rules: the ID is checked
//! before it becomes a path (SR-20), writes refuse linked folders (SR-21),
//! reads are size-capped and refuse links (SR-23), and the text is only ever
//! data (SR-30).

use std::fmt;

use crate::fsio::{self, FsError};
use crate::ids::{self, IdKind};
use crate::index;
use crate::model::{Idea, IdeaOrigin, IdeaStatus, Node, NodeBody, NodeType};
use crate::node_file;
use crate::project::Project;
use crate::spec;

/// The most ideas a story keeps (*Proposed*). Past this, `add` refuses until
/// the writer sets some aside for good by editing the folder; it keeps a
/// runaway loop from filling the story folder.
pub const MAX_IDEAS: usize = 200;

/// Why an idea operation didn't go through.
#[derive(Debug)]
pub enum IdeaError {
    Fs(FsError),
    /// The request was unusable; the message is plain language.
    Invalid(String),
}

impl fmt::Display for IdeaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdeaError::Fs(e) => e.fmt(f),
            IdeaError::Invalid(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for IdeaError {}

impl From<FsError> for IdeaError {
    fn from(e: FsError) -> Self {
        IdeaError::Fs(e)
    }
}

fn invalid<T>(message: impl Into<String>) -> Result<T, IdeaError> {
    Err(IdeaError::Invalid(message.into()))
}

/// The idea's file, relative to the story folder.
fn relative_path(id: &str) -> String {
    format!("spec/{}/{id}.md", NodeType::Idea.folder())
}

/// Text as stored: whitespace runs (and line breaks) collapse to one space.
fn normalized(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// What `add` did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Added {
    pub id: String,
    pub status: IdeaStatus,
    /// The file to stage with the next save, relative to the story folder.
    pub path: String,
    /// True when the same text was already saved (in any state), so nothing
    /// was written and the existing idea is returned. A set-aside idea is
    /// never brought back this way.
    pub existing: bool,
}

/// Saves a new held idea.
pub fn add(
    project: &Project,
    text: &str,
    origin: IdeaOrigin,
    touches: &[String],
) -> Result<Added, IdeaError> {
    let idea = Idea {
        text: normalized(text),
        status: IdeaStatus::Held,
        origin,
        touches: touches.to_vec(),
    };
    if let Some(problem) = idea.problem() {
        return invalid(problem);
    }

    let loaded = spec::load(&project.spec_dir())?;
    let spec = &loaded.spec;
    // The skill names parts of the story it was shown, so a node ID that isn't
    // in the story is a mistake, not a guess to store.
    for part in &idea.touches {
        let is_stage = crate::marker::Topic::parse(part).is_some();
        if !is_stage && spec.node(part).is_none() {
            return invalid(format!("'{part}' isn't a part of this story"));
        }
    }

    let mut held = 0usize;
    for node in &spec.nodes {
        let NodeBody::Idea(other) = &node.body else {
            continue;
        };
        held += 1;
        if other.text.to_lowercase() == idea.text.to_lowercase() {
            return Ok(Added {
                id: node.id.clone(),
                status: other.status,
                path: relative_path(&node.id),
                existing: true,
            });
        }
    }
    if held >= MAX_IDEAS {
        return invalid(format!(
            "this story already holds {MAX_IDEAS} ideas, which is the most it keeps"
        ));
    }

    let dir = project.spec_dir().join(NodeType::Idea.folder());
    let (id, path) = loop {
        let id = ids::new_id(IdKind::Idea).map_err(|e| IdeaError::Invalid(e.to_string()))?;
        let path = dir.join(format!("{id}.md"));
        // IDs are unique across the whole spec, so a clash with any node (or a
        // fact) means drawing again; so does a file that failed to load.
        let used = spec.node(&id).is_some()
            || spec
                .nodes
                .iter()
                .any(|n| n.facts.iter().any(|f| f.id == id))
            || std::fs::symlink_metadata(&path).is_ok();
        if !used {
            break (id, path);
        }
    };
    let node = Node {
        id: id.clone(),
        name: Idea::label_for(&idea.text),
        aliases: Vec::new(),
        summary: None,
        revision: 1,
        body: NodeBody::Idea(idea),
        facts: Vec::new(),
        notes: String::new(),
    };
    let text = node_file::render_node(&node).map_err(|e| IdeaError::Invalid(e.to_string()))?;
    project.ensure_dir(&dir)?;
    project.write_atomic(&path, &text)?;
    index::rebuild(project)?;
    Ok(Added {
        path: relative_path(&id),
        id,
        status: IdeaStatus::Held,
        existing: false,
    })
}

/// What `mark` did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Marked {
    pub id: String,
    pub status: IdeaStatus,
    /// False when the idea already had this status, so nothing was written.
    pub changed: bool,
    /// The file to stage with the next save, relative to the story folder.
    pub path: String,
}

/// Records that the writer picked an idea or set it aside. Only those two
/// statuses can be set: an idea becomes `held` only by being saved.
pub fn mark(project: &Project, id: &str, status: IdeaStatus) -> Result<Marked, IdeaError> {
    // The ID becomes part of a path, so only a well-formed one gets that far.
    if !ids::is_kind(id, IdKind::Idea) {
        return invalid(format!("'{id}' isn't an idea ID"));
    }
    if status == IdeaStatus::Held {
        return invalid("an idea can be marked picked or set aside, not held");
    }
    let path = project
        .spec_dir()
        .join(NodeType::Idea.folder())
        .join(format!("{id}.md"));
    let text = match fsio::read_regular_capped(&path, fsio::MAX_STORY_FILE_BYTES)? {
        Some(Ok(text)) => text,
        Some(Err(why)) => return invalid(fsio::refusal_message(&why)),
        None => return invalid(format!("there's no saved idea '{id}'")),
    };
    let mut node = node_file::parse_node(&text)
        .map_err(|e| IdeaError::Invalid(format!("the idea '{id}' can't be read: {e}")))?;
    if node.id != id {
        return invalid(format!("the file for '{id}' holds a different node"));
    }
    let NodeBody::Idea(idea) = &mut node.body else {
        return invalid(format!("'{id}' isn't an idea"));
    };
    if idea.status == status {
        return Ok(Marked {
            id: id.to_string(),
            status,
            changed: false,
            path: relative_path(id),
        });
    }
    idea.status = status;
    node.revision = node.revision.saturating_add(1);
    let rendered = node_file::render_node(&node).map_err(|e| IdeaError::Invalid(e.to_string()))?;
    project.write_atomic(&path, &rendered)?;
    index::rebuild(project)?;
    Ok(Marked {
        id: id.to_string(),
        status,
        changed: true,
        path: relative_path(id),
    })
}

/// One held idea, as `list` returns it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed {
    pub id: String,
    pub text: String,
    pub origin: IdeaOrigin,
    pub touches: Vec<String>,
}

/// The held ideas, in ID order, optionally only those that touch one part of
/// the story (a node ID or a stage code). Read-only. A part that no longer
/// exists simply matches nothing; ideas picked or set aside are never listed.
pub fn list(project: &Project, touching: Option<&str>) -> Result<Vec<Listed>, IdeaError> {
    if let Some(part) = touching
        && !crate::model::is_idea_touch(part)
    {
        return invalid(format!(
            "'{part}' isn't a story part (use a node ID or one of the stage names)"
        ));
    }
    let loaded = spec::load(&project.spec_dir())?;
    Ok(loaded
        .spec
        .nodes
        .iter()
        .filter_map(|node| match &node.body {
            NodeBody::Idea(idea)
                if idea.status == IdeaStatus::Held
                    && touching.is_none_or(|part| idea.touches.iter().any(|t| t == part)) =>
            {
                Some(Listed {
                    id: node.id.clone(),
                    text: idea.text.clone(),
                    origin: idea.origin,
                    touches: idea.touches.clone(),
                })
            }
            _ => None,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{IDEA_TEXT_MAX_CHARS, IDEA_TOUCHES_MAX};
    use crate::testspec;
    use crate::testutil::TempDir;

    const PERSON: &str = "p-maraharb";

    /// A story folder with one person in it.
    fn story() -> (TempDir, Project) {
        let dir = TempDir::new();
        let project = Project::new(dir.path());
        let people = project.spec_dir().join("people");
        std::fs::create_dir_all(&people).unwrap();
        let node = testspec::node(PERSON, NodeBody::Person);
        std::fs::write(
            people.join(format!("{PERSON}.md")),
            node_file::render_node(&node).unwrap(),
        )
        .unwrap();
        (dir, project)
    }

    fn read_idea(project: &Project, id: &str) -> Node {
        let path = project.root().join(relative_path(id));
        node_file::parse_node(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    fn add_ok(project: &Project, text: &str) -> Added {
        add(project, text, IdeaOrigin::Brainstorm, &[]).unwrap()
    }

    fn message(r: Result<impl std::fmt::Debug, IdeaError>) -> String {
        match r {
            Err(IdeaError::Invalid(m)) => m,
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    #[test]
    fn adds_a_held_idea_as_a_node_file() {
        let (_dir, project) = story();
        let added = add(
            &project,
            "  Maybe the informant\nis Jonah's sister.  ",
            IdeaOrigin::Tangent,
            &[PERSON.to_string(), "crisis".to_string()],
        )
        .unwrap();
        assert!(ids::is_kind(&added.id, IdKind::Idea));
        assert!(!added.existing);
        assert_eq!(added.status, IdeaStatus::Held);
        assert_eq!(added.path, format!("spec/ideas/{}.md", added.id));
        let node = read_idea(&project, &added.id);
        assert_eq!(node.revision, 1);
        assert_eq!(node.name, "Maybe the informant is Jonah's sister.");
        let NodeBody::Idea(idea) = node.body else {
            panic!("not an idea");
        };
        assert_eq!(idea.text, "Maybe the informant is Jonah's sister.");
        assert_eq!(idea.status, IdeaStatus::Held);
        assert_eq!(idea.origin, IdeaOrigin::Tangent);
        assert_eq!(idea.touches, [PERSON, "crisis"]);
    }

    #[test]
    fn a_saved_idea_loads_with_the_story_and_changes_no_problems() {
        let (_dir, project) = story();
        add_ok(&project, "A dark alley.");
        let loaded = spec::load(&project.spec_dir()).unwrap();
        assert_eq!(loaded.problems, vec![]);
        assert_eq!(loaded.spec.nodes.len(), 2);
        // The index was rebuilt and counts the new folder.
        let status =
            std::fs::read_to_string(project.spec_dir().join("_index/status.json")).unwrap();
        assert!(status.contains("\"ideas\": 1"), "{status}");
    }

    #[test]
    fn text_is_limited_by_characters_not_bytes() {
        let (_dir, project) = story();
        let exactly = "é".repeat(IDEA_TEXT_MAX_CHARS);
        assert!(!add_ok(&project, &exactly).existing);
        let too_long = "é".repeat(IDEA_TEXT_MAX_CHARS + 1);
        assert!(message(add(&project, &too_long, IdeaOrigin::Brainstorm, &[])).contains("300"));
        // Whitespace is collapsed before counting.
        let padded = format!("{}   \n   {}", "a".repeat(150), "b".repeat(149));
        assert!(!add_ok(&project, &padded).existing);
    }

    #[test]
    fn refuses_empty_text_and_bad_touches() {
        let (_dir, project) = story();
        for text in ["", "   \n  "] {
            assert!(
                message(add(&project, text, IdeaOrigin::Brainstorm, &[])).contains("some text")
            );
        }
        let touch = |part: &str| {
            message(add(
                &project,
                "An idea",
                IdeaOrigin::Brainstorm,
                &[part.to_string()],
            ))
        };
        // Well-formed, but not in this story.
        assert!(touch("p-zzzzzzzz").contains("isn't a part of this story"));
        // Not an ID or a stage name at all, including path tricks.
        for bad in [
            "",
            "../x",
            "p-0007",
            "middle",
            "f-factfact",
            "idea-abcdefgh",
        ] {
            assert!(touch(bad).contains("can touch"), "{bad}");
        }
        let twice = [PERSON.to_string(), PERSON.to_string()];
        assert!(
            message(add(&project, "An idea", IdeaOrigin::Brainstorm, &twice))
                .contains("listed twice")
        );
        let many: Vec<String> = [
            "premise",
            "people",
            "setup",
            "complications",
            "crisis",
            "resolution",
        ]
        .iter()
        .cycle()
        .take(IDEA_TOUCHES_MAX + 1)
        .map(|s| s.to_string())
        .collect();
        assert!(
            message(add(&project, "An idea", IdeaOrigin::Brainstorm, &many)).contains("at most")
        );
        // Nothing was written by any refusal.
        assert!(!project.spec_dir().join("ideas").exists());
    }

    #[test]
    fn the_same_text_is_not_saved_twice_and_a_set_aside_idea_stays_gone() {
        let (_dir, project) = story();
        let first = add_ok(&project, "A dark alley.");
        let again = add_ok(&project, "a DARK   alley.");
        assert!(again.existing);
        assert_eq!(again.id, first.id);
        assert_eq!(
            std::fs::read_dir(project.spec_dir().join("ideas"))
                .unwrap()
                .count(),
            1
        );

        mark(&project, &first.id, IdeaStatus::SetAside).unwrap();
        let after = add_ok(&project, "A dark alley.");
        assert!(after.existing);
        assert_eq!(after.status, IdeaStatus::SetAside);
        assert_eq!(after.id, first.id);
        assert_eq!(list(&project, None).unwrap(), vec![]);
    }

    #[test]
    fn a_story_keeps_at_most_the_cap() {
        let (_dir, project) = story();
        let dir = project.spec_dir().join("ideas");
        std::fs::create_dir_all(&dir).unwrap();
        for n in 0..MAX_IDEAS {
            let id = ids::new_id(IdKind::Idea).unwrap();
            let node = Node {
                id: id.clone(),
                name: format!("idea {n}"),
                aliases: vec![],
                summary: None,
                revision: 1,
                body: NodeBody::Idea(Idea {
                    text: format!("idea {n}"),
                    status: IdeaStatus::Held,
                    origin: IdeaOrigin::Brainstorm,
                    touches: vec![],
                }),
                facts: vec![],
                notes: String::new(),
            };
            std::fs::write(
                dir.join(format!("{id}.md")),
                node_file::render_node(&node).unwrap(),
            )
            .unwrap();
        }
        assert!(message(add(&project, "one more", IdeaOrigin::Brainstorm, &[])).contains("200"));
    }

    #[test]
    fn marks_an_idea_picked_or_set_aside_and_bumps_its_revision() {
        let (_dir, project) = story();
        let id = add_ok(&project, "An informant in the harbor office.").id;

        let picked = mark(&project, &id, IdeaStatus::Picked).unwrap();
        assert!(picked.changed);
        assert_eq!(picked.path, format!("spec/ideas/{id}.md"));
        let node = read_idea(&project, &id);
        assert_eq!(node.revision, 2);
        assert!(matches!(&node.body, NodeBody::Idea(i) if i.status == IdeaStatus::Picked));

        // The same status again writes nothing.
        let same = mark(&project, &id, IdeaStatus::Picked).unwrap();
        assert!(!same.changed);
        assert_eq!(read_idea(&project, &id).revision, 2);

        // The writer can change their mind.
        mark(&project, &id, IdeaStatus::SetAside).unwrap();
        let node = read_idea(&project, &id);
        assert_eq!(node.revision, 3);
        assert!(matches!(&node.body, NodeBody::Idea(i) if i.status == IdeaStatus::SetAside));
    }

    #[test]
    fn mark_refuses_bad_ids_held_and_things_that_arent_ideas() {
        let (_dir, project) = story();
        let id = add_ok(&project, "An idea").id;
        for bad in [
            "",
            "../people/p-maraharb",
            "idea-abc",
            PERSON,
            "idea-abcdefgh/../x",
        ] {
            assert!(message(mark(&project, bad, IdeaStatus::Picked)).contains("isn't an idea ID"));
        }
        assert!(message(mark(&project, &id, IdeaStatus::Held)).contains("not held"));
        let missing = "idea-zzzzzzzz";
        assert!(message(mark(&project, missing, IdeaStatus::Picked)).contains("no saved idea"));
        // Refusals changed nothing.
        assert_eq!(read_idea(&project, &id).revision, 1);
    }

    #[test]
    fn mark_refuses_a_file_that_is_not_a_good_idea() {
        let (_dir, project) = story();
        let id = add_ok(&project, "An idea").id;
        let path = project.root().join(relative_path(&id));
        std::fs::write(&path, "not a node file").unwrap();
        assert!(message(mark(&project, &id, IdeaStatus::Picked)).contains("can't be read"));
        // A file whose front matter names another ID is refused too.
        let other = ids::new_id(IdKind::Idea).unwrap();
        let (_dir2, project2) = story();
        let real = add_ok(&project2, "Another idea").id;
        let text = std::fs::read_to_string(project2.root().join(relative_path(&real))).unwrap();
        std::fs::write(&path, text.replace(&real, &other)).unwrap();
        assert!(message(mark(&project, &id, IdeaStatus::Picked)).contains("different node"));
    }

    #[test]
    fn lists_only_held_ideas_and_can_filter_by_part() {
        let (_dir, project) = story();
        let alley = add(
            &project,
            "A dark alley.",
            IdeaOrigin::Tangent,
            &["setup".into()],
        )
        .unwrap();
        let sister = add(
            &project,
            "The informant is Jonah's sister.",
            IdeaOrigin::Brainstorm,
            &[PERSON.into(), "crisis".into()],
        )
        .unwrap();
        let loose = add_ok(&project, "A storm.");
        let waved = add_ok(&project, "A circus.");
        let chosen = add_ok(&project, "A wedding.");
        mark(&project, &waved.id, IdeaStatus::SetAside).unwrap();
        mark(&project, &chosen.id, IdeaStatus::Picked).unwrap();

        let all = list(&project, None).unwrap();
        let mut got: Vec<_> = all.iter().map(|i| i.id.clone()).collect();
        got.sort();
        let mut want = vec![alley.id.clone(), sister.id.clone(), loose.id.clone()];
        want.sort();
        assert_eq!(got, want);

        let by_person = list(&project, Some(PERSON)).unwrap();
        assert_eq!(by_person.len(), 1);
        assert_eq!(by_person[0].id, sister.id);
        assert_eq!(by_person[0].origin, IdeaOrigin::Brainstorm);
        assert_eq!(by_person[0].touches, [PERSON, "crisis"]);
        assert_eq!(by_person[0].text, "The informant is Jonah's sister.");

        let by_stage = list(&project, Some("setup")).unwrap();
        assert_eq!(by_stage.len(), 1);
        assert_eq!(by_stage[0].id, alley.id);
        // A well-formed part nothing touches (or that is gone) matches nothing.
        assert_eq!(list(&project, Some("p-zzzzzzzz")).unwrap(), vec![]);
        assert!(message(list(&project, Some("not a part"))).contains("isn't a story part"));
    }

    #[test]
    fn list_changes_nothing() {
        let (dir, project) = story();
        add_ok(&project, "A dark alley.");
        let snapshot = |root: &std::path::Path| {
            let mut found = Vec::new();
            let mut stack = vec![root.to_path_buf()];
            while let Some(path) = stack.pop() {
                for entry in std::fs::read_dir(&path).unwrap() {
                    let entry = entry.unwrap();
                    let p = entry.path();
                    if p.is_dir() {
                        stack.push(p);
                    } else {
                        found.push((p.clone(), std::fs::read(&p).unwrap()));
                    }
                }
            }
            found.sort();
            found
        };
        let before = snapshot(dir.path());
        list(&project, None).unwrap();
        list(&project, Some("setup")).unwrap();
        assert_eq!(snapshot(dir.path()), before);
    }

    #[test]
    fn an_idea_does_not_block_deleting_what_it_touches() {
        let (_dir, project) = story();
        add(
            &project,
            "About Mara.",
            IdeaOrigin::Brainstorm,
            &[PERSON.into()],
        )
        .unwrap();
        let loaded = spec::load(&project.spec_dir()).unwrap();
        let refs = index::references(&loaded.spec);
        assert_eq!(refs, vec![], "ideas are not story references");
    }

    #[cfg(unix)]
    #[test]
    fn refuses_to_write_through_a_linked_ideas_folder() {
        let (dir, project) = story();
        let elsewhere = TempDir::new();
        std::os::unix::fs::symlink(elsewhere.path(), project.spec_dir().join("ideas")).unwrap();
        let result = add(&project, "A dark alley.", IdeaOrigin::Brainstorm, &[]);
        assert!(result.is_err(), "{result:?}");
        assert_eq!(std::fs::read_dir(elsewhere.path()).unwrap().count(), 0);
        drop(dir);
    }
}
