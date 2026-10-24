//! The small file-access layer the rest of the core goes through.
//!
//! Keeping file access in one place is what lets the core stay portable (a
//! later standalone or mobile app can swap this out) and keeps the safety
//! rules in one spot: every write is atomic, so a reader never sees half a
//! file even when Claude Code cancels the status line script mid-run.

use std::io::{self, Write as _};
use std::path::Path;

use crate::json::{self, Value, ValueExt as _};

/// Errors from reading or writing Skaldsaga's own files.
#[derive(Debug)]
pub enum FsError {
    Io {
        path: String,
        source: io::Error,
    },
    Json {
        path: String,
        source: json::ParseError,
    },
    NotAnObject {
        path: String,
    },
}

impl std::fmt::Display for FsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FsError::Io { path, source } => write!(f, "{path}: {source}"),
            FsError::Json { path, source } => write!(f, "{path}: {source}"),
            FsError::NotAnObject { path } => write!(f, "{path}: expected a JSON object"),
        }
    }
}

impl std::error::Error for FsError {}

fn io_err(path: &Path, source: io::Error) -> FsError {
    FsError::Io {
        path: path.display().to_string(),
        source,
    }
}

/// Reads a file, returning `None` when it does not exist.
pub fn read_optional(path: &Path) -> Result<Option<String>, FsError> {
    match std::fs::read_to_string(path) {
        Ok(s) => Ok(Some(s)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_err(path, e)),
    }
}

/// Why a story-side file wasn't read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    /// A symlink, folder or special file: never followed (SR-21).
    NotARegularFile,
    /// Bigger than the cap (SR-23).
    TooBig,
}

/// Reads a small regular file for the story folder, refusing symlinks and
/// special files and anything over `max_bytes`. A missing file is `Ok(None)`;
/// a refused one is `Ok(Some(Err(_)))`, so the caller can report it and go on.
pub fn read_regular_capped(
    path: &Path,
    max_bytes: u64,
) -> Result<Option<Result<String, Refused>>, FsError> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(io_err(path, e)),
    };
    if !meta.file_type().is_file() {
        return Ok(Some(Err(Refused::NotARegularFile)));
    }
    if meta.len() > max_bytes {
        return Ok(Some(Err(Refused::TooBig)));
    }
    match std::fs::read_to_string(path) {
        Ok(s) => Ok(Some(Ok(s))),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) if e.kind() == io::ErrorKind::InvalidData => Ok(Some(Err(Refused::NotARegularFile))),
        Err(e) => Err(io_err(path, e)),
    }
}

/// Reads a JSON object file. A missing or empty file reads as `None`.
pub fn read_json_object(path: &Path) -> Result<Option<Value>, FsError> {
    let Some(text) = read_optional(path)? else {
        return Ok(None);
    };
    if text.trim().is_empty() {
        return Ok(None);
    }
    let v = json::parse(&text).map_err(|source| FsError::Json {
        path: path.display().to_string(),
        source,
    })?;
    if !v.is_object() {
        return Err(FsError::NotAnObject {
            path: path.display().to_string(),
        });
    }
    Ok(Some(v))
}

/// Writes a file atomically: write a temporary file next to it, then rename
/// over the target. Creates the parent folder if needed.
pub fn write_atomic(path: &Path, contents: &str) -> Result<(), FsError> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent).map_err(|e| io_err(parent, e))?;
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp = parent.join(format!(".{file_name}.{}.tmp", std::process::id()));
    let result = (|| {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(contents.as_bytes())?;
        f.sync_all()?;
        std::fs::rename(&tmp, path)
    })();
    if let Err(e) = result {
        let _ = std::fs::remove_file(&tmp);
        return Err(io_err(path, e));
    }
    Ok(())
}

/// Writes a JSON value atomically, pretty-printed.
pub fn write_json(path: &Path, value: &Value) -> Result<(), FsError> {
    write_atomic(path, &value.to_pretty_string())
}

/// Removes a file, treating "already gone" as success.
pub fn remove_optional(path: &Path) -> Result<bool, FsError> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(io_err(path, e)),
    }
}

/// What a folder entry is, without following links (SR-11, SR-21).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Dir,
    Symlink,
    /// A device, socket, pipe or anything else that isn't a plain file.
    Other,
}

/// One entry in a folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    pub name: String,
    /// True only for a real folder; a link to one is `Symlink`, not a folder.
    pub is_dir: bool,
    pub kind: EntryKind,
    /// A plain file with an executable permission bit set (never true on
    /// systems without such bits).
    pub executable: bool,
}

/// Largest story file Skaldsaga reads (SR-23).
pub const MAX_STORY_FILE_BYTES: u64 = 1024 * 1024;

/// The reason a refused file gives, in plain words for a problem list.
pub fn refusal_message(why: &Refused) -> &'static str {
    match why {
        Refused::NotARegularFile => {
            "this is a link or a special file; Skaldsaga only reads plain files and never follows links"
        }
        Refused::TooBig => "this file is too big for Skaldsaga to read",
    }
}

#[cfg(unix)]
fn has_exec_bit(meta: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    meta.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn has_exec_bit(_meta: &std::fs::Metadata) -> bool {
    false
}

/// Lists a folder, sorted by name. A missing folder lists as empty.
pub fn list_dir(path: &Path) -> Result<Vec<DirEntry>, FsError> {
    let read = match std::fs::read_dir(path) {
        Ok(r) => r,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(io_err(path, e)),
    };
    let mut out = Vec::new();
    for entry in read {
        let entry = entry.map_err(|e| io_err(path, e))?;
        let file_type = entry.file_type().map_err(|e| io_err(&entry.path(), e))?;
        let kind = if file_type.is_symlink() {
            EntryKind::Symlink
        } else if file_type.is_dir() {
            EntryKind::Dir
        } else if file_type.is_file() {
            EntryKind::File
        } else {
            EntryKind::Other
        };
        // The entry's own metadata, not the target's: links are never followed.
        let executable = kind == EntryKind::File
            && entry
                .metadata()
                .map(|m| has_exec_bit(&m))
                .map_err(|e| io_err(&entry.path(), e))?;
        out.push(DirEntry {
            name: entry.file_name().to_string_lossy().into_owned(),
            is_dir: kind == EntryKind::Dir,
            kind,
            executable,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Creates a folder (and its parents). Returns true if it didn't exist.
pub fn ensure_dir(path: &Path) -> Result<bool, FsError> {
    if path.is_dir() {
        return Ok(false);
    }
    std::fs::create_dir_all(path).map_err(|e| io_err(path, e))?;
    Ok(true)
}

/// Writes a file only if nothing is there yet. Returns true if it wrote.
/// Never replaces an existing file, so setup can't overwrite the writer's work.
pub fn write_if_missing(path: &Path, contents: &str) -> Result<bool, FsError> {
    if path.exists() {
        return Ok(false);
    }
    write_atomic(path, contents)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    #[test]
    fn capped_reads_refuse_symlinks_folders_and_big_files() {
        let dir = TempDir::new();
        let file = dir.path().join("a.json");
        std::fs::write(&file, "{}").unwrap();
        assert_eq!(
            read_regular_capped(&file, 10).unwrap(),
            Some(Ok("{}".to_string()))
        );
        assert_eq!(
            read_regular_capped(&dir.path().join("nope"), 10).unwrap(),
            None
        );
        assert_eq!(
            read_regular_capped(&file, 1).unwrap(),
            Some(Err(Refused::TooBig))
        );
        assert_eq!(
            read_regular_capped(dir.path(), 10).unwrap(),
            Some(Err(Refused::NotARegularFile))
        );
        #[cfg(unix)]
        {
            let link = dir.path().join("link.json");
            std::os::unix::fs::symlink(&file, &link).unwrap();
            assert_eq!(
                read_regular_capped(&link, 10).unwrap(),
                Some(Err(Refused::NotARegularFile))
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn listing_tells_links_programs_and_plain_files_apart() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = TempDir::new();
        let file = dir.path().join("plain.md");
        std::fs::write(&file, "x").unwrap();
        let prog = dir.path().join("prog.md");
        std::fs::write(&prog, "x").unwrap();
        std::fs::set_permissions(&prog, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::os::unix::fs::symlink(&file, dir.path().join("link.md")).unwrap();
        std::os::unix::fs::symlink(dir.path(), dir.path().join("linkdir")).unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        let by_name = |n: &str| {
            list_dir(dir.path())
                .unwrap()
                .into_iter()
                .find(|e| e.name == n)
                .unwrap()
        };
        assert_eq!(
            (by_name("plain.md").kind, by_name("plain.md").executable),
            (EntryKind::File, false)
        );
        assert!(by_name("prog.md").executable);
        assert_eq!(by_name("link.md").kind, EntryKind::Symlink);
        let linkdir = by_name("linkdir");
        assert_eq!(linkdir.kind, EntryKind::Symlink);
        assert!(!linkdir.is_dir, "a link to a folder is not a folder");
        assert!(by_name("sub").is_dir);
    }

    #[test]
    fn atomic_write_round_trip_and_cleanup() {
        let dir = TempDir::new();
        let path = dir.path().join("nested/a.json");
        write_json(&path, &json::parse(r#"{"a":1}"#).unwrap()).unwrap();
        assert_eq!(
            read_json_object(&path)
                .unwrap()
                .unwrap()
                .to_compact_string(),
            r#"{"a":1}"#
        );
        let leftovers: Vec<_> = std::fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty());
    }

    #[test]
    fn missing_and_empty_files_read_as_none() {
        let dir = TempDir::new();
        let path = dir.path().join("x.json");
        assert!(read_json_object(&path).unwrap().is_none());
        std::fs::write(&path, "  \n").unwrap();
        assert!(read_json_object(&path).unwrap().is_none());
        assert!(!remove_optional(&dir.path().join("nope")).unwrap());
    }

    #[test]
    fn non_object_and_bad_json_are_errors() {
        let dir = TempDir::new();
        let path = dir.path().join("x.json");
        std::fs::write(&path, "[1]").unwrap();
        assert!(matches!(
            read_json_object(&path),
            Err(FsError::NotAnObject { .. })
        ));
        std::fs::write(&path, "{").unwrap();
        assert!(matches!(read_json_object(&path), Err(FsError::Json { .. })));
    }
}
