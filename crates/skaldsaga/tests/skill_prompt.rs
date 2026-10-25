//! Script tests on the skill's own text (HANDOFF, Ready to build, Slice 1,
//! item 5; `delivery.md`, Testing and evals, part 3).
//!
//! Every line meant for the writer sits in a ```` ```say ```` block in
//! `SKILL.md`. A deterministic scan checks that none of them uses an internal
//! word, so the writer never meets the program's own vocabulary. The same
//! file's tool permissions and commands are checked too, so a later edit
//! can't quietly widen what the skill may do (SR-24, SR-31, SR-32).

use std::path::PathBuf;

/// Words the writer never sees. Craft terms are taught through the
/// terminology ladder instead, so they are not here.
const INTERNAL_TERMS: [&str; 28] = [
    "map",
    "ticket",
    "tickets",
    "fog",
    "proposal",
    "proposals",
    "gate",
    "gates",
    "weaving",
    "spec",
    "node",
    "nodes",
    "arc",
    "thread",
    "stage",
    "blocker",
    "findings",
    "lane",
    "inbox",
    "harness",
    "commit",
    "provisional",
    "canon",
    "interviewer",
    "orchestrator",
    "housekeeper",
    "weaver",
    "sub-agent",
];

/// Roles that are internal too, checked as whole words.
const INTERNAL_ROLES: [&str; 4] = ["harvester", "researcher", "outliner", "subagent"];

fn skill_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../plugin/skills/skaldsaga/SKILL.md")
}

fn skill_text() -> String {
    std::fs::read_to_string(skill_path()).expect("SKILL.md")
}

/// The text of every ```` ```say ```` block, with the line it starts on.
fn say_blocks(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut current: Option<(usize, String)> = None;
    for (i, line) in text.lines().enumerate() {
        match (&mut current, line.trim_start()) {
            (None, "```say") => current = Some((i + 1, String::new())),
            (Some(_), "```") => out.extend(current.take()),
            (Some((_, body)), _) => {
                body.push_str(line);
                body.push('\n');
            }
            _ => {}
        }
    }
    assert!(current.is_none(), "a say block is never closed");
    out
}

fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '-'))
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// Internal words found in `text`.
fn internal_words(text: &str) -> Vec<String> {
    words(text)
        .into_iter()
        .filter(|w| INTERNAL_TERMS.contains(&w.as_str()) || INTERNAL_ROLES.contains(&w.as_str()))
        .collect()
}

#[test]
fn the_scan_catches_internal_words() {
    // The scan itself must not pass vacuously.
    for bad in [
        "I'll file a proposal for that.",
        "This is the first stage of the thread.",
        "Let me commit it to the spec.",
        "The Housekeeper will look.",
        "Your arc is set.",
    ] {
        assert!(!internal_words(bad).is_empty(), "should flag: {bad}");
    }
    for fine in [
        "What's the story, even a scrap?",
        "Is this a new story, or more about the one we're working on?",
    ] {
        assert!(internal_words(fine).is_empty(), "should pass: {fine}");
    }
    let sample = "text\n```say\nHello there\n```\nmore\n```sh\nnope\n```\n";
    assert_eq!(say_blocks(sample), vec![(2, "Hello there\n".to_string())]);
}

#[test]
fn nothing_the_writer_sees_uses_an_internal_word() {
    let blocks = say_blocks(&skill_text());
    assert!(
        blocks.len() >= 10,
        "expected the opener's lines in say blocks, found {}",
        blocks.len()
    );
    for (line, body) in blocks {
        assert!(!body.trim().is_empty(), "empty say block at line {line}");
        let found = internal_words(&body);
        assert!(
            found.is_empty(),
            "internal word(s) {found:?} in the say block at line {line}: {body}"
        );
    }
}

#[test]
fn every_entry_condition_has_a_section_and_a_writer_line() {
    let text = skill_text();
    for heading in [
        "### no_story_bare",
        "### no_story_with_text",
        "### story_empty",
        "### story_with_text",
        "### story_damaged",
        "### story_mid_thread",
        "### story_stopping_point",
    ] {
        let at = text
            .find(heading)
            .unwrap_or_else(|| panic!("{heading} is missing"));
        let rest = &text[at + heading.len()..];
        let section = &rest[..rest.find("\n##").unwrap_or(rest.len())];
        assert!(
            section.contains("```say"),
            "{heading} has no line for the writer"
        );
    }
    // The opening question and the three loose questions are there.
    for line in [
        "What's the story, even a scrap?",
        "What's it about?",
        "How does it begin?",
        "Where do you see it ending?",
    ] {
        assert!(text.contains(line), "missing writer line: {line}");
    }
}

#[test]
fn the_skill_may_write_only_its_scratch_file_and_never_edit_story_files() {
    let text = skill_text();
    let front = text
        .strip_prefix("---\n")
        .and_then(|t| t.split_once("\n---\n"))
        .map(|(f, _)| f)
        .expect("front matter");
    let allowed = front
        .lines()
        .find_map(|l| l.strip_prefix("allowed-tools:"))
        .expect("allowed-tools");
    // No bare Write, Edit, NotebookEdit or open-ended shell.
    for tool in ["Edit", "NotebookEdit", "WebFetch", "WebSearch", "Agent"] {
        assert!(!allowed.contains(tool), "{tool} must not be allowed");
    }
    assert!(
        !allowed.contains(" Bash,") && !allowed.ends_with("Bash"),
        "no bare Bash"
    );
    let writes: Vec<&str> = allowed
        .match_indices("Write")
        .map(|(i, _)| &allowed[i..])
        .collect();
    assert_eq!(writes.len(), 1);
    assert!(
        writes[0].starts_with("Write(**/.harness/draft.txt)"),
        "Write is limited to the scratch file"
    );
}

#[test]
fn commands_never_carry_the_writers_words() {
    let text = skill_text();
    // Only the two fixed setup messages are typed after `-m`.
    let typed: Vec<&str> = text
        .lines()
        .filter(|l| l.contains("git commit") && l.contains(" -m "))
        .collect();
    for l in &typed {
        assert!(
            l.contains("Skaldsaga: set up the story folders")
                || l.contains("Skaldsaga: move the session meter to personal settings"),
            "an -m message other than the two fixed ones: {l}"
        );
    }
    // Words and drafts reach commands only through the scratch file.
    for l in text
        .lines()
        .filter(|l| l.contains("skaldsaga") && l.contains("--words-file"))
    {
        assert!(
            l.contains("--words-file .harness/draft.txt"),
            "words must come from the scratch file: {l}"
        );
    }
    for l in text.lines().filter(|l| l.contains("skaldsaga propose")) {
        assert!(
            l.contains("--file .harness/draft.txt"),
            "a draft must come from the scratch file: {l}"
        );
    }
}

/// The text of one `###` section of the skill, heading excluded.
fn section(text: &str, heading: &str) -> String {
    let at = text
        .find(heading)
        .unwrap_or_else(|| panic!("{heading} is missing"));
    let rest = &text[at + heading.len()..];
    rest[..rest.find("\n##").unwrap_or(rest.len())].to_string()
}

#[test]
fn the_recap_itself_runs_only_commands_that_read() {
    let text = skill_text();
    let mid = section(&text, "### story_mid_thread");
    let before_answer = &mid[..mid
        .find("3. Wait for the answer")
        .expect("the step that waits for the answer")];
    assert!(before_answer.contains("skaldsaga recap"));
    for writes in [
        "skaldsaga session",
        "skaldsaga propose",
        "skaldsaga commit",
        "git ",
        "draft.txt",
    ] {
        assert!(
            !before_answer.contains(writes),
            "the recap must change nothing on disk, but mentions {writes}"
        );
    }
    // The topic is recorded only once the writer answers, and only as a code.
    let after = &mid[mid.find("3. Wait for the answer").unwrap()..];
    assert!(after.contains("skaldsaga session ask --topic <topic>"));
}

#[test]
fn the_note_of_where_things_stand_never_gets_a_snapshot_of_its_own() {
    let text = skill_text();
    assert!(
        text.contains("git add -- <each path in add_paths> <the path session answer returned>"),
        "a real save carries the note with it"
    );
    assert!(
        !text.contains("Stage and commit the path it returns"),
        "the note must not be committed on its own after an answer"
    );
    // The two moments it is saved alone are an accepted offer and a stop.
    let offer = section(&text, "### Offering to save");
    assert!(offer.contains("session save --taken yes"));
    assert!(offer.contains("session save --taken no"));
    assert!(offer.contains("meter check"));
    // Offers are plain questions, never orders, and say what they save.
    for (line, body) in say_blocks(&offer) {
        assert!(
            body.trim_end().ends_with('?'),
            "an offer is a question (line {line}): {body}"
        );
    }
}
