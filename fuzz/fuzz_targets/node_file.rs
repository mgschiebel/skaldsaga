//! Story files: the front matter of a node, and the attributes file.
#![no_main]

use libfuzzer_sys::fuzz_target;
use skaldsaga_core::{node_file, registry};

fuzz_target!(|data: &[u8]| {
    // The loaders only ever see text (`read_to_string` refuses anything else).
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let _ = node_file::parse_node(text);
    let _ = node_file::split_front_matter(text);
    let _ = registry::parse_attributes_file(text);
});
