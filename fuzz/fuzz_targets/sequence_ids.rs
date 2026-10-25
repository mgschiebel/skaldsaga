//! `sequence.md`, and the ID checks that guard every path built from input
//! (SR-20).
#![no_main]

use libfuzzer_sys::fuzz_target;
use skaldsaga_core::ids::{self, IdKind};
use skaldsaga_core::spec;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let _ = spec::parse_sequence(text);
    let _ = ids::kind_of(text);
    for kind in IdKind::ALL {
        let _ = ids::is_kind(text, kind);
    }
});
