//! Looks through a story folder for things that must never be followed or
//! run: links, files marked executable, and special files such as devices or
//! pipes (SR-11, SR-21). It only reports; nothing is changed and nothing is
//! followed, so a hostile story can't send the scan outside the folder.
//!
//! Skaldsaga treats a story folder as plain data. A link could point at a
//! file elsewhere on the computer, an executable bit has no place on a
//! Markdown or JSON file, and a device or pipe can hang a reader.

use std::path::Path;

use crate::fsio::{self, EntryKind, FsError};
use crate::project::Project;

/// The folders scanned, relative to the story folder.
pub const SCANNED: [&str; 3] = ["spec", "manuscript", crate::project::HARNESS_DIR];

/// How deep the scan goes below each folder, and how many entries it looks at
/// in all, so an enormous or endlessly nested folder can't stall it.
const MAX_DEPTH: usize = 8;
const MAX_ENTRIES: usize = 20_000;

/// Why a path was flagged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Why {
    /// A symbolic link (never followed).
    Link,
    /// A plain file with an executable permission bit.
    Executable,
    /// A device, socket, pipe or other non-plain file.
    Special,
}

impl Why {
    pub fn code(self) -> &'static str {
        match self {
            Why::Link => "link",
            Why::Executable => "executable",
            Why::Special => "special_file",
        }
    }
}

/// One flagged path, relative to the story folder with `/` separators.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flagged {
    pub path: String,
    pub why: Why,
}

/// What the scan found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scan {
    pub flagged: Vec<Flagged>,
    /// True when the scan stopped early at its depth or entry limit.
    pub truncated: bool,
}

impl Scan {
    pub fn to_value(&self) -> crate::json::Value {
        serde_json::json!({
            "flagged": self.flagged.iter()
                .map(|f| serde_json::json!({"path": f.path, "why": f.why.code()}))
                .collect::<Vec<_>>(),
            "truncated": self.truncated,
        })
    }
}

/// Scans `spec/`, `manuscript/` and `.harness/`. Missing folders are skipped.
pub fn scan(project: &Project) -> Result<Scan, FsError> {
    let mut out = Scan::default();
    let mut seen = 0usize;
    for top in SCANNED {
        walk(project.root(), top, 0, &mut seen, &mut out)?;
    }
    Ok(out)
}

fn walk(
    root: &Path,
    relative: &str,
    depth: usize,
    seen: &mut usize,
    out: &mut Scan,
) -> Result<(), FsError> {
    for entry in fsio::list_dir(&root.join(relative))? {
        *seen += 1;
        if *seen > MAX_ENTRIES {
            out.truncated = true;
            return Ok(());
        }
        let path = format!("{relative}/{}", entry.name);
        match entry.kind {
            EntryKind::Symlink => out.flagged.push(Flagged {
                path,
                why: Why::Link,
            }),
            EntryKind::Other => out.flagged.push(Flagged {
                path,
                why: Why::Special,
            }),
            EntryKind::File => {
                if entry.executable {
                    out.flagged.push(Flagged {
                        path,
                        why: Why::Executable,
                    });
                }
            }
            EntryKind::Dir => {
                if depth + 1 >= MAX_DEPTH {
                    out.truncated = true;
                } else {
                    walk(root, &path, depth + 1, seen, out)?;
                    if out.truncated && *seen > MAX_ENTRIES {
                        return Ok(());
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::testutil::TempDir;
    use std::os::unix::fs::PermissionsExt as _;

    fn project(dir: &TempDir) -> Project {
        for sub in ["spec/people", "manuscript", ".harness/inbox"] {
            std::fs::create_dir_all(dir.path().join(sub)).unwrap();
        }
        Project::new(dir.path())
    }

    #[test]
    fn a_clean_story_has_nothing_flagged() {
        let dir = TempDir::new();
        let p = project(&dir);
        std::fs::write(dir.path().join("spec/people/p-aaaaaaaa.md"), "x").unwrap();
        assert_eq!(scan(&p).unwrap(), Scan::default());
    }

    #[test]
    fn links_programs_and_pipes_are_flagged_in_every_scanned_folder() {
        let dir = TempDir::new();
        let p = project(&dir);
        let outside = dir.path().join("outside.txt");
        std::fs::write(&outside, "secret").unwrap();
        std::os::unix::fs::symlink(&outside, dir.path().join("spec/people/p-linklink.md")).unwrap();
        std::os::unix::fs::symlink(dir.path(), dir.path().join("manuscript/loop")).unwrap();
        let prog = dir.path().join(".harness/inbox/run.json");
        std::fs::write(&prog, "{}").unwrap();
        std::fs::set_permissions(&prog, std::fs::Permissions::from_mode(0o755)).unwrap();
        let fifo = dir.path().join("manuscript/pipe");
        assert!(
            std::process::Command::new("mkfifo")
                .arg(&fifo)
                .status()
                .unwrap()
                .success()
        );

        let found = scan(&p).unwrap();
        let mut got: Vec<_> = found
            .flagged
            .iter()
            .map(|f| (f.path.as_str(), f.why))
            .collect();
        got.sort();
        assert_eq!(
            got,
            vec![
                (".harness/inbox/run.json", Why::Executable),
                ("manuscript/loop", Why::Link),
                ("manuscript/pipe", Why::Special),
                ("spec/people/p-linklink.md", Why::Link),
            ]
        );
        assert!(!found.truncated);
    }

    #[test]
    fn a_linked_folder_is_not_entered() {
        let dir = TempDir::new();
        let p = project(&dir);
        let elsewhere = TempDir::new();
        let secret = elsewhere.path().join("run.sh");
        std::fs::write(&secret, "x").unwrap();
        std::fs::set_permissions(&secret, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::os::unix::fs::symlink(elsewhere.path(), dir.path().join("spec/people/out")).unwrap();
        let found = scan(&p).unwrap();
        assert_eq!(found.flagged.len(), 1);
        assert_eq!(found.flagged[0].path, "spec/people/out");
    }

    #[test]
    fn very_deep_folders_stop_the_scan_and_say_so() {
        let dir = TempDir::new();
        let p = project(&dir);
        let mut deep = dir.path().join("manuscript");
        for i in 0..(MAX_DEPTH + 2) {
            deep = deep.join(format!("d{i}"));
        }
        std::fs::create_dir_all(&deep).unwrap();
        assert!(scan(&p).unwrap().truncated);
    }
}
