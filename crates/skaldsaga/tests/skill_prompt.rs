//! Script tests on the skill's own text (HANDOFF, Ready to build, Slice 1,
//! item 5; `delivery.md`, Testing and evals, part 3).
//!
//! Every fixed line meant for the writer sits in a ```` ```say ```` block in
//! `SKILL.md`. A deterministic scan checks that none of them uses an internal
//! word, so the writer never meets the program's own vocabulary. The same
//! file's tool permissions and commands are checked too, so a later edit
//! can't quietly widen what the skill may do (SR-24, SR-31, SR-32).
#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::string_slice,
    reason = "a test that cannot go on should fail loudly, and a test failure is a panic"
)]

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
fn the_skill_may_run_only_the_git_commands_it_uses() {
    let text = skill_text();
    let allowed = text
        .lines()
        .find_map(|l| l.strip_prefix("allowed-tools:"))
        .expect("allowed-tools");
    // SR-31: no whole-command grants. `git diff --output=<file>` writes any file,
    // `git remote add` changes where a push goes, and `git commit -m` takes any text.
    for wide in [
        "Bash(git init:*)",
        "Bash(git add:*)",
        "Bash(git rm:*)",
        "Bash(git ls-files:*)",
        "Bash(git diff:*)",
        "Bash(git commit:*)",
        "Bash(git remote:*)",
    ] {
        assert!(!allowed.contains(wide), "{wide} is too wide");
    }
    for exact in [
        "Bash(git init -q)",
        "Bash(git add -- :*)",
        "Bash(git rm --cached --ignore-unmatch -q -- :*)",
        "Bash(git ls-files --error-unmatch -- :*)",
        "Bash(git diff --cached --quiet)",
        "Bash(git commit -q -F :*)",
        "Bash(git remote)",
    ] {
        assert!(allowed.contains(exact), "{exact} is missing");
    }
    // `-m` is allowed for the two fixed setup messages only.
    assert_eq!(allowed.matches("git commit -q -m").count(), 2);
    assert!(allowed.contains("-m \"Skaldsaga: set up the story folders\")"));
    assert!(allowed.contains("-m \"Skaldsaga: move the session meter to personal settings\")"));
    // Every git command the steps show is one of those forms.
    assert!(!text.contains("git add -A"), "git add -A is not allowed");
    assert!(!text.contains("git diff --cached --quiet -"));
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
        text.contains("git add -- <each path in add_paths> <each path session answer returned>"),
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
    // Offers are described by what they must get across, never given as a sentence to copy.
    assert!(
        say_blocks(&offer).is_empty(),
        "the offers must not carry a say block"
    );
    for needed in [
        "plain question, never an order",
        "where the story stands can be saved, so there is a safe point to come back to",
        "The wording is new each time; never reuse a sentence from an earlier offer",
    ] {
        assert!(offer.contains(needed), "missing from the offers: {needed}");
    }
    for stock in [
        "Want me to save where we are",
        "so there's a safe point to come back to?",
        "Shall I save?",
    ] {
        assert!(
            !text.contains(stock),
            "a stock save offer is back in the skill: {stock}"
        );
    }
}

#[test]
fn the_plain_offer_waits_and_a_due_offer_takes_a_turn_of_its_own() {
    let text = skill_text();
    let offer = section(&text, "### Offering to save");
    // The plain offer waits for the current answer; it never interrupts a question just asked.
    assert!(offer.contains("waits until the current answer is dealt with"));
    assert!(offer.contains("never interrupts a question you just asked"));
    // The strong ones still interrupt.
    assert_eq!(
        offer.matches("It does not wait.").count(),
        2,
        "the strong recommendation and the too-long line do not wait"
    );
    // A due offer is the reflection and then the offer, with no story question.
    assert!(offer.contains("A due offer takes a turn of its own."));
    assert!(offer.contains("with no story question"));
    assert!(offer.contains("the offer is the turn's only question"));
    // All three strengths are still there, keyed to the program's states.
    assert!(offer.contains("`save_offer` true"));
    assert!(offer.contains("`suggest_wrap_up`"));
    assert!(offer.contains("`wrap_up`"));
}

#[test]
fn resuming_from_a_stopping_point_runs_only_commands_that_read_until_the_writer_answers() {
    let text = skill_text();
    let stop = section(&text, "### story_stopping_point");
    let at = stop
        .find("3. Wait for the answer")
        .expect("the step that waits for the answer");
    let before_answer = &stop[..at];
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
    assert!(stop[at..].contains("skaldsaga session ask --topic"));
}

#[test]
fn reaching_a_stopping_point_offers_a_choice_and_a_save_and_records_a_believable_reason() {
    let text = skill_text();
    let reach = section(&text, "### Reaching a stopping point");
    // The choice comes from the program, the suggestion is not the topic that just ran dry.
    assert!(reach.contains("skaldsaga recap"));
    assert!(reach.contains("`choices`"));
    assert!(reach.contains("`last_topic`"));
    // Only a chosen area or a real stop is recorded, and a stop gives one of the three reasons.
    assert!(reach.contains("skaldsaga session ask --topic"));
    for reason in ["writer_signals", "thread_exhausted", "writer_done"] {
        assert!(reach.contains(reason), "missing stop reason {reason}");
    }
    assert!(reach.contains("skaldsaga session stop --reason"));
    // The snapshot is exactly what the stop returned, with the program's message file.
    assert!(reach.contains("git add -- <each path in add_paths>"));
    assert!(reach.contains("git commit -q -F <message_file>"));
    // Nothing in this section is a sentence to copy: the turn, the notice about an answer
    // that isn't saved and the goodbye are described by their meaning, worded afresh.
    assert!(
        say_blocks(&reach).is_empty(),
        "the stopping point must not carry a say block"
    );
    assert!(
        reach.contains("not saved yet") && reach.contains("may be lost the next time they start"),
        "the instructions must say what the unsaved-answer notice means"
    );
    // A deliberate stop checks a waiting answer first; the notice is the fallback.
    assert!(reach.contains("check it before stopping"));
    assert!(reach.contains("give the plain notice instead"));
    // Stopping without saving records nothing.
    assert!(reach.contains("record nothing"));
    // The turn is built from three described parts, in the writer's own words and the
    // rotating reflection moves, so two stopping points in a session do not sound alike.
    for needed in [
        "worded afresh each time, with no sentence to copy",
        "built from what the writer confirmed this session, in their own words",
        "the same rotating moves as a reflection",
        "not the move you used last time",
        "**The areas to look at next:**",
        "**One short line offering to save where things stand.**",
    ] {
        assert!(reach.contains(needed), "missing from the turn: {needed}");
    }
    // The save is an offer beside the choice, so it adds no second question.
    assert!(reach.contains("The choice of areas is the turn's only question"));
    assert!(reach.contains("an offer, not a second question"));
    // The goodbye is one plain line with no question, and never names a next area.
    assert!(reach.contains("**The goodbye** is one plain line with no question"));
    assert!(reach.contains("It never names a next area"));
    assert!(
        !text.contains("Saved. Next time I'll pick it up from here."),
        "the stock goodbye is back in the skill"
    );
}

#[test]
fn the_drift_guard_keeps_the_tangent_as_a_loose_idea_and_never_confirms_it() {
    let text = skill_text();
    let drift = section(&text, "### When the story drifts");
    assert!(
        drift.contains("skaldsaga session answer --words-file .harness/draft.txt --confirmed no")
    );
    assert!(
        !drift.contains("--confirmed yes"),
        "a tangent is never confirmed by the guard itself"
    );
    assert!(!drift.contains("skaldsaga propose"));
    // It asks, it never refuses.
    assert!(drift.contains("Never refuse, never call it off topic"));
}

#[test]
fn the_drift_question_is_a_reminder_and_one_open_question_with_no_sample_text() {
    let text = skill_text();
    let drift = section(&text, "### When the story drifts");
    assert!(
        say_blocks(&drift).is_empty(),
        "the drift question must not carry a say block"
    );
    for needed in [
        "two parts and one question, worded afresh each time, with no sentence to copy",
        "**A general reminder, as a statement,**",
        "with no specific scene",
        "what they said is kept either way",
        "**One open question** about where they are heading with it",
        "It is not a yes/no question",
        "The open question is the turn's only question",
        // A new inspiration becomes the focus; a lost thread goes back.
        "A new inspiration becomes the new focus",
        "A lost thread goes back to the topic",
    ] {
        assert!(
            drift.contains(needed),
            "missing from the drift step: {needed}"
        );
    }
    // The old yes/no question, its stock opening and the follow-or-return question are gone.
    for gone in [
        "Is it part of the story you told me about?",
        "That's interesting.",
        "whether to follow it or go back",
    ] {
        assert!(!text.contains(gone), "an old drift line is back: {gone}");
    }
}

#[test]
fn the_line_at_the_end_of_a_thread_is_described_not_given_as_a_sentence() {
    let text = skill_text();
    let focus = section(&text, "### Following the writer's focus");
    assert!(focus.contains("**The line at the end of a thread.**"));
    assert!(focus.contains("carrying on from here isn't built yet"));
    assert!(focus.contains("its wording is new each time"));
    assert!(
        say_blocks(&focus).len() == 1,
        "only the one question about a moment remains in a say block"
    );
    assert!(
        !text.contains("That's a good start."),
        "the stock end-of-thread line is back in the skill"
    );
}

#[test]
fn reflections_are_described_never_given_as_a_sentence_to_copy() {
    let text = skill_text();
    let after = section(&text, "### After every answer");
    // The step that used to end in a copied sentence has no sample line for the writer.
    assert!(
        say_blocks(&after).is_empty(),
        "the reflection step must not carry a say block"
    );
    for sentence in ["Is that what you meant?", "Is that right?"] {
        assert!(
            !text.contains(sentence),
            "a stock confirming sentence is back in the skill: {sentence}"
        );
    }
    // The moves, the rule against repeating one, and the light matching of the writer's wording.
    for needed in [
        "say it back in their own words",
        "say what it sounds like to you",
        "say what you picture",
        "never use the one you used last time",
        "never reuse a sentence from earlier in the session",
        "Match the writer's wording, lightly",
        "Only the reflecting phrase matches",
        "Never mention this to the writer",
    ] {
        assert!(
            after.contains(needed),
            "missing from the reflection step: {needed}"
        );
    }
}

#[test]
fn a_clear_answer_is_saved_when_the_writer_carries_on_and_a_guess_gets_a_real_question() {
    let text = skill_text();
    let after = section(&text, "### After every answer");
    // The waiting answer is dealt with before the new message, and saved before it is replaced.
    let waiting = after
        .find("**If an answer is waiting")
        .expect("the waiting-answer step");
    let record = after
        .find("skaldsaga session answer --words-file .harness/draft.txt --confirmed no")
        .expect("the step that records the new answer");
    assert!(
        waiting < record,
        "decide about the waiting answer before recording the new one"
    );
    assert!(after.contains("**They carry on**"));
    assert!(after.contains("Save the waiting answer now, exactly as in **On a yes**"));
    // Anything unclear saves nothing.
    assert!(after.contains("When you can't tell whether they corrected it, don't save."));
    // Only a guess gets a real yes/no question, and the first answer of a new story counts as one.
    assert!(after.contains("**A guess:**"));
    assert!(after.contains("one real yes/no question"));
    assert!(after.contains("first answer of a new story"));
    // Carrying on without answering a real question is not a yes.
    assert!(after.contains("Carrying on without answering it is not a yes"));
    // Saving still goes through the same program steps, and the draft file is the only way words travel.
    let yes = section(&text, "### On a yes");
    assert!(yes.contains("skaldsaga propose --file .harness/draft.txt"));
    assert!(yes.contains("skaldsaga check --proposal"));
    assert!(yes.contains("skaldsaga commit --proposal"));
    assert!(yes.contains("confirmed yes"));
}

#[test]
fn the_nine_worded_afresh_lines_are_named_and_kept_out_of_say_blocks() {
    let text = skill_text();
    assert!(
        text.contains("Nine things are never taken from a `say` block or from an earlier turn")
    );
    for line in [
        "the reflection after an answer",
        "the real question about a guess",
        "the quick check when the writer stops",
        "the notice about an answer that isn't saved",
        "the offers to save",
        "the turn at a stopping point",
        "the question when the story drifts",
        "the goodbye",
        "the line at the end of a thread",
    ] {
        assert!(text.contains(line), "not named as worded afresh: {line}");
    }
    // The session-start recaps and the once-per-session lines keep their fixed lines on purpose.
    for kept in [
        "You're writing a modern Jack and Jill, set in a city at night",
        "I'm not finding a story in this folder.",
        "What's the story, even a scrap?",
    ] {
        assert!(text.contains(kept), "a fixed line was removed: {kept}");
    }
}

#[test]
fn openings_and_question_shapes_vary_and_git_is_never_mentioned() {
    let text = skill_text();
    let after = section(&text, "### After every answer");
    // A reflection does not open with the first word of any of the last three, and the two habitual openers are named.
    assert!(
        after.contains("don't open with the same first word as any of your last three reflections")
    );
    assert!(after.contains("\"So\" and \"Got it\""));
    // A real question changes shape, not only wording.
    assert!(after.contains("never ask two real questions in a row that are built the same way"));
    // A stated place is placed without a question; only an unstated one is a guess.
    assert!(text.contains("When they said where it goes"));
    assert!(text.contains("not a guess: place it there"));
    assert!(text.contains("framed differently from the last placement question"));
    // The writer never hears about git steps.
    assert!(text.contains("**Never talk about git to the writer.**"));
    assert!(text.contains("\"saved\" is the whole word"));
}
