//! Stable IDs for spec nodes, facts and other records.
//!
//! IDs are opaque and type-prefixed (`p-7kq2xmhd`): a short prefix naming the
//! kind of thing, a hyphen, then eight random characters. They are random
//! rather than counting up (HANDOFF, Spec and storage) so that:
//!
//! * facts can move between projects (a series, a fork) without colliding;
//! * shuffled scenes never look "out of order";
//! * agents can name a new node before it is committed, with no shared counter.
//!
//! The alphabet leaves out look-alikes (`0`/`o`, `1`/`l`), giving 32
//! characters and 2^40 possible IDs per prefix. The committer still rejects
//! duplicates.

use std::fmt;

/// Characters used in the random part of an ID: lowercase letters and digits
/// without `0`, `o`, `1` or `l`. Exactly 32, so one character is five bits.
pub const ALPHABET: &[u8; 32] = b"abcdefghijkmnpqrstuvwxyz23456789";

/// Length of the random part of an ID.
pub const RANDOM_LEN: usize = 8;

/// The kinds of thing that get an ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum IdKind {
    Person,
    Place,
    Group,
    Item,
    Scene,
    Decision,
    Thread,
    Style,
    Arc,
    Fact,
    Edition,
    Proposal,
    /// A decision the writer made about the process (an acknowledged finding),
    /// as opposed to a shape decision node (`d-`). Stored in `.harness/decisions/`.
    WriterDecision,
}

impl IdKind {
    /// Every kind, in a fixed order.
    pub const ALL: [IdKind; 13] = [
        IdKind::Person,
        IdKind::Place,
        IdKind::Group,
        IdKind::Item,
        IdKind::Scene,
        IdKind::Decision,
        IdKind::Thread,
        IdKind::Style,
        IdKind::Arc,
        IdKind::Fact,
        IdKind::Edition,
        IdKind::Proposal,
        IdKind::WriterDecision,
    ];

    /// The prefix written before the hyphen.
    pub fn prefix(self) -> &'static str {
        match self {
            IdKind::Person => "p",
            IdKind::Place => "l",
            IdKind::Group => "g",
            IdKind::Item => "i",
            IdKind::Scene => "s",
            IdKind::Decision => "d",
            IdKind::Thread => "t",
            IdKind::Style => "st",
            IdKind::Arc => "a",
            IdKind::Fact => "f",
            IdKind::Edition => "e",
            IdKind::Proposal => "pr",
            IdKind::WriterDecision => "wd",
        }
    }

    fn from_prefix(prefix: &str) -> Option<IdKind> {
        IdKind::ALL.into_iter().find(|k| k.prefix() == prefix)
    }
}

impl fmt::Display for IdKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            IdKind::Person => "person",
            IdKind::Place => "place",
            IdKind::Group => "group",
            IdKind::Item => "item",
            IdKind::Scene => "scene",
            IdKind::Decision => "decision",
            IdKind::Thread => "thread",
            IdKind::Style => "style",
            IdKind::Arc => "arc",
            IdKind::Fact => "fact",
            IdKind::Edition => "edition",
            IdKind::Proposal => "proposal",
            IdKind::WriterDecision => "writer decision",
        };
        f.write_str(name)
    }
}

/// Returns the kind an ID belongs to, or `None` if it isn't a well-formed ID.
///
/// Accepts only the random format. The sequential IDs used as examples in
/// the design notes (`p-0007`) contain `0` and `1`, which the alphabet leaves
/// out, so they are rejected.
pub fn kind_of(id: &str) -> Option<IdKind> {
    let (prefix, rest) = id.split_once('-')?;
    let kind = IdKind::from_prefix(prefix)?;
    (rest.len() == RANDOM_LEN && rest.bytes().all(|b| ALPHABET.contains(&b))).then_some(kind)
}

/// True if `id` is a well-formed ID of the given kind.
pub fn is_kind(id: &str, kind: IdKind) -> bool {
    kind_of(id) == Some(kind)
}

/// Errors from creating an ID.
#[derive(Debug)]
pub struct RandomError(getrandom::Error);

impl fmt::Display for RandomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "couldn't get random data for a new ID: {}", self.0)
    }
}

impl std::error::Error for RandomError {}

/// Creates a new random ID of the given kind.
pub fn new_id(kind: IdKind) -> Result<String, RandomError> {
    let mut bytes = [0u8; RANDOM_LEN];
    getrandom::fill(&mut bytes).map_err(RandomError)?;
    Ok(from_bytes(kind, bytes))
}

/// Builds an ID from eight random bytes (five bits of each are used, so every
/// alphabet character is equally likely).
fn from_bytes(kind: IdKind, bytes: [u8; RANDOM_LEN]) -> String {
    let mut id = String::with_capacity(kind.prefix().len() + 1 + RANDOM_LEN);
    id.push_str(kind.prefix());
    id.push('-');
    for b in bytes {
        id.push(ALPHABET[usize::from(b & 31)] as char);
    }
    id
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alphabet_has_no_look_alikes_or_repeats() {
        for c in b"01lo" {
            assert!(!ALPHABET.contains(c));
        }
        let mut sorted = ALPHABET.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 32);
    }

    #[test]
    fn prefixes_are_unique() {
        let mut prefixes: Vec<_> = IdKind::ALL.iter().map(|k| k.prefix()).collect();
        prefixes.sort_unstable();
        prefixes.dedup();
        assert_eq!(prefixes.len(), IdKind::ALL.len());
    }

    #[test]
    fn new_ids_round_trip_for_every_kind() {
        for kind in IdKind::ALL {
            let id = new_id(kind).unwrap();
            assert_eq!(kind_of(&id), Some(kind), "{id}");
            assert!(is_kind(&id, kind));
        }
    }

    #[test]
    fn new_ids_differ() {
        let a = new_id(IdKind::Scene).unwrap();
        let b = new_id(IdKind::Scene).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn builds_from_bytes_using_five_bits() {
        assert_eq!(
            from_bytes(IdKind::Fact, [0, 1, 31, 32, 63, 255, 2, 3]),
            "f-ab9a99cd"
        );
    }

    #[test]
    fn rejects_malformed_ids() {
        for bad in [
            "",
            "p",
            "p-",
            "p-0007",           // design-note example, not the real format
            "p-7kq2xmh",        // too short
            "p-7kq2xmhdd",      // too long
            "P-7kq2xmhd",       // uppercase prefix
            "p-7KQ2XMHD",       // uppercase body
            "x-7kq2xmhd",       // unknown prefix
            "p_7kq2xmhd",       // wrong separator
            "s-7kq2xmhd-extra", // trailing text
        ] {
            assert_eq!(kind_of(bad), None, "{bad}");
        }
        assert_eq!(kind_of("pr-7kq2xmhd"), Some(IdKind::Proposal));
        assert_eq!(kind_of("st-7kq2xmhd"), Some(IdKind::Style));
        assert_eq!(kind_of("wd-7kq2xmhd"), Some(IdKind::WriterDecision));
        assert!(!is_kind("s-7kq2xmhd", IdKind::Person));
    }
}
