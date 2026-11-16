//! The agent registry: which agents leave records beside the story, which
//! judgments each may record, and who it checks (HANDOFF, Planning session 12;
//! `design-agents.md`, "The first build's plan", Build 2).
//!
//! The program, not the agent, holds the closed list of outcomes. A verdict
//! (`verdict` module) is accepted only if its result is `pass` or one of its
//! agent's sanctioned fail codes, so an agent cannot invent a way to fail. The
//! code names are *Proposed* (`design-agents.md`, "Continuity's contract").
//!
//! Only Continuity is registered for now. The Interviewer is not in the list:
//! it records no verdicts and raises no questions of its own; it keeps one
//! record of which questions it asked (`asked` module), under its own name
//! below.

/// One sanctioned way for an agent's check to fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FailCode {
    pub code: &'static str,
    /// True when a verdict with this code must cite at least one saved item
    /// as evidence. A pass always needs evidence; the one fail that doesn't is
    /// "there is nothing saved to check".
    pub needs_evidence: bool,
}

/// An agent whose verdicts and questions the program stores.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Agent {
    /// The name used in commands and as the record folder's name.
    pub name: &'static str,
    /// The sanctioned fails, besides `pass`.
    pub fail_codes: &'static [FailCode],
    /// The agents whose output this one checks (the other half of its pair).
    pub counterparts: &'static [&'static str],
}

/// The Interviewer's name: the owner of the records of questions asked.
pub const INTERVIEWER: &str = "interviewer";

/// The result every agent's check can have besides a fail code.
pub const PASS: &str = "pass";

/// Continuity: checks what the Interviewer proposed from the writer's answer
/// against what is saved (*Decided November 16, 2026*; the code names are
/// *Proposed*).
pub const CONTINUITY: Agent = Agent {
    name: "continuity",
    fail_codes: &[
        FailCode {
            code: "contradiction_reported",
            needs_evidence: true,
        },
        FailCode {
            code: "nothing_saved_to_check",
            needs_evidence: false,
        },
        FailCode {
            code: "writer_kept_inconsistency",
            needs_evidence: true,
        },
    ],
    counterparts: &[INTERVIEWER],
};

/// Every registered agent.
pub const AGENTS: [Agent; 1] = [CONTINUITY];

/// The registered agent with this name.
pub fn find(name: &str) -> Option<&'static Agent> {
    AGENTS.iter().find(|agent| agent.name == name)
}

impl Agent {
    /// The fail code with this name, if it is on this agent's list.
    pub fn fail_code(&self, code: &str) -> Option<&'static FailCode> {
        self.fail_codes.iter().find(|f| f.code == code)
    }

    /// The names of every sanctioned result (`pass` first), for messages.
    pub fn result_names(&self) -> Vec<&'static str> {
        std::iter::once(PASS)
            .chain(self.fail_codes.iter().map(|f| f.code))
            .collect()
    }
}

/// The registered agent names, for messages.
pub fn names() -> Vec<&'static str> {
    AGENTS.iter().map(|agent| agent.name).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continuity_has_the_three_sanctioned_fails() {
        let codes: Vec<_> = CONTINUITY.fail_codes.iter().map(|f| f.code).collect();
        assert_eq!(
            codes,
            [
                "contradiction_reported",
                "nothing_saved_to_check",
                "writer_kept_inconsistency"
            ]
        );
        assert_eq!(CONTINUITY.result_names()[0], "pass");
        assert!(CONTINUITY.fail_code("made_up").is_none());
    }

    #[test]
    fn agents_are_found_by_name_and_the_interviewer_is_not_one() {
        assert_eq!(find("continuity"), Some(&CONTINUITY));
        assert_eq!(find(INTERVIEWER), None);
        assert_eq!(find(""), None);
        assert_eq!(names(), ["continuity"]);
    }

    #[test]
    fn names_are_safe_as_folder_names() {
        for agent in AGENTS {
            assert!(
                agent
                    .name
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b == b'_'),
                "{}",
                agent.name
            );
        }
    }
}
