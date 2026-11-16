//! Agent records: a verdict, a queued question, and the Interviewer's record of
//! what happened to a question. Each is a small file with front matter that the
//! loaders read back from the story folder.
#![no_main]

use libfuzzer_sys::fuzz_target;
use skaldsaga_core::{asked, question, verdict};

fuzz_target!(|data: &[u8]| {
    // The loaders only ever see text (`read_to_string` refuses anything else).
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let _ = verdict::parse(text);
    let _ = question::parse(text);
    let _ = asked::parse(text);
});
