//! Proposal files, and the targets and references the writer's answers name.
#![no_main]

use libfuzzer_sys::fuzz_target;
use skaldsaga_core::proposal;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let _ = proposal::parse(text);
    let _ = proposal::parse_target(text);
    let _ = proposal::parse_item_ref(text);
});
