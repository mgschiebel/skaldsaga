//! The last-session marker, researched concept files, and the short codes
//! that arrive on the command line.
#![no_main]

use libfuzzer_sys::fuzz_target;
use skaldsaga_core::commit::Decision;
use skaldsaga_core::marker::{self, Topic};
use skaldsaga_core::{craft, scenario::Scenario};

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let _ = marker::parse(text);
    let _ = craft::parse_concept(text);
    let _ = Topic::parse(text);
    let _ = Decision::parse(text);
    let _ = Scenario::parse(text);
});
