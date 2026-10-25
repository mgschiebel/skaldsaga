//! SR-21: every write into a story folder goes through a `Project` method,
//! which refuses to pass through a link. The raw writers in `fsio` are
//! crate-private, but code inside the core crate could still call them, or
//! write with `std::fs` directly. This scans the production source (the code
//! above each file's `#[cfg(test)]` line) and fails on either, so a later
//! change can't add a write that skips the check.
#![expect(
    clippy::unwrap_used,
    reason = "a test that cannot go on should fail loudly, and a test failure is a panic"
)]

use std::path::Path;

/// The raw writers: only `fsio` itself and `Project`'s guarded wrappers may call them.
const RAW_WRITERS: &[&str] = &[
    "fsio::write_atomic(",
    "fsio::write_json(",
    "fsio::remove_optional(",
    "fsio::ensure_dir(",
    "fsio::write_if_missing(",
];

/// Ways around `fsio` altogether. Nothing outside `fsio` writes to disk.
const BYPASSES: &[&str] = &[
    "fs::write(",
    "fs::create_dir",
    "fs::rename(",
    "fs::remove_",
    "fs::copy(",
    "fs::hard_link(",
    "File::create",
    "OpenOptions",
    "fs::symlink(",
    "symlink_file(",
    "symlink_dir(",
    "set_permissions(",
];

/// Lines in `text` (production code only) that write without the guard.
fn offences(file: &str, text: &str) -> Vec<String> {
    let may_use_raw = file == "fsio.rs" || file == "project.rs";
    let may_write = file == "fsio.rs";
    let mut found = Vec::new();
    for (n, line) in text.lines().enumerate() {
        if line.trim() == "#[cfg(test)]" {
            break;
        }
        if line.trim_start().starts_with("//") {
            continue;
        }
        let raw = !may_use_raw && RAW_WRITERS.iter().any(|p| line.contains(p));
        let bypass = !may_write && BYPASSES.iter().any(|p| line.contains(p));
        if raw || bypass {
            found.push(format!("{file}:{}: {}", n + 1, line.trim()));
        }
    }
    found
}

fn scan(dir: &Path) -> Vec<String> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "rs") {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            // Test helpers are compiled only for tests.
            if name == "testutil.rs" {
                continue;
            }
            found.extend(offences(&name, &std::fs::read_to_string(&path).unwrap()));
        }
    }
    found
}

#[test]
fn story_writes_all_go_through_the_guarded_project_methods() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let mut found = scan(&crates.join("skaldsaga-core/src"));
    found.extend(scan(&crates.join("skaldsaga/src")));
    assert!(
        found.is_empty(),
        "these lines write without the link check; use the `Project` method instead:\n{}",
        found.join("\n")
    );
}

#[test]
fn the_scan_itself_catches_what_it_is_meant_to() {
    let bad = "fn f(p: &Project) {\n    fsio::write_atomic(&path, text)?;\n}\n";
    assert_eq!(offences("commit.rs", bad).len(), 1);
    assert!(
        offences("project.rs", bad).is_empty(),
        "the guarded wrappers may call it"
    );
    let sneaky = "    std::fs::write(&path, text)?;\n    std::fs::create_dir_all(dir)?;\n";
    assert_eq!(offences("craft.rs", sneaky).len(), 2);
    assert_eq!(
        offences("project.rs", sneaky).len(),
        2,
        "even the wrappers may not bypass fsio"
    );
    assert!(offences("fsio.rs", sneaky).is_empty());
    let after_tests = "#[cfg(test)]\nmod tests {\n    std::fs::write(&path, text).unwrap();\n}\n";
    assert!(
        offences("craft.rs", after_tests).is_empty(),
        "test code is not scanned"
    );
    assert!(
        offences(
            "craft.rs",
            "    // fsio::write_atomic(x) is how it used to work\n"
        )
        .is_empty()
    );
}
