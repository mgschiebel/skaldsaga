//! The small file-access layer the rest of the core goes through.
//!
//! Keeping file access in one place is what lets the core stay portable (a
//! later standalone or mobile app can swap this out) and keeps the safety
//! rules in one spot: every write is atomic, so a reader never sees half a
//! file even when Claude Code cancels the status line script mid-run.

use std::io::{self, Read as _, Write as _};
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

/// Reads a plain file that lives in the story folder, returning `None` when
/// it does not exist. The same rules as `read_regular_capped` (SR-21, SR-23):
/// a link or special file, or one over `MAX_STORY_FILE_BYTES`, is an error
/// here, because every caller wants to stop and say so.
pub fn read_optional(path: &Path) -> Result<Option<String>, FsError> {
    match read_regular_capped(path, MAX_STORY_FILE_BYTES)? {
        None => Ok(None),
        Some(Ok(text)) => Ok(Some(text)),
        Some(Err(why)) => Err(refused_err(path, &why)),
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

fn refused_err(path: &Path, why: &Refused) -> FsError {
    FsError::Io {
        path: path.display().to_string(),
        source: io::Error::new(io::ErrorKind::InvalidData, refusal_message(why)),
    }
}

/// True when both describe the same file on disk. Where the system gives no
/// way to tell (not Unix), the file type check alone has to do.
#[cfg(unix)]
fn same_file(a: &std::fs::Metadata, b: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt as _;
    a.dev() == b.dev() && a.ino() == b.ino()
}

#[cfg(not(unix))]
fn same_file(_a: &std::fs::Metadata, _b: &std::fs::Metadata) -> bool {
    true
}

/// Reads at most `max_bytes` from `reader`, however much it holds. One byte
/// more than the cap is read, which is enough to tell "too big" from "just
/// fits" without ever holding more than the cap plus one byte (SR-23).
fn read_bounded(reader: impl io::Read, max_bytes: u64) -> io::Result<Result<String, Refused>> {
    let mut bytes = Vec::new();
    reader
        .take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)?;
    match u64::try_from(bytes.len()) {
        Ok(len) if len <= max_bytes => {}
        _ => return Ok(Err(Refused::TooBig)),
    }
    Ok(String::from_utf8(bytes).map_err(|_| Refused::NotARegularFile))
}

/// Reads a small regular file for the story folder, refusing symlinks and
/// special files and anything over `max_bytes`. A missing file is `Ok(None)`;
/// a refused one is `Ok(Some(Err(_)))`, so the caller can report it and go on.
///
/// The file is opened once. What was opened has to be the file that was
/// looked at (same device and file number), so a link swapped in after the
/// check is refused, and the read stops at the cap even if the file grew
/// since its size was checked.
pub fn read_regular_capped(
    path: &Path,
    max_bytes: u64,
) -> Result<Option<Result<String, Refused>>, FsError> {
    let before = match std::fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(io_err(path, e)),
    };
    if !before.file_type().is_file() {
        return Ok(Some(Err(Refused::NotARegularFile)));
    }
    if before.len() > max_bytes {
        return Ok(Some(Err(Refused::TooBig)));
    }
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(io_err(path, e)),
    };
    let opened = file.metadata().map_err(|e| io_err(path, e))?;
    if !opened.is_file() || !same_file(&before, &opened) {
        return Ok(Some(Err(Refused::NotARegularFile)));
    }
    match read_bounded(file, max_bytes) {
        Ok(read) => Ok(Some(read)),
        Err(e) => Err(io_err(path, e)),
    }
}

/// Reads a file that belongs to the writer, not to the story (their own
/// Claude Code settings). It may be a link, since many people keep their
/// settings in a folder of dotfiles, but the read is still capped.
pub fn read_user_optional(path: &Path) -> Result<Option<String>, FsError> {
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(io_err(path, e)),
    };
    let opened = file.metadata().map_err(|e| io_err(path, e))?;
    if !opened.is_file() {
        return Err(refused_err(path, &Refused::NotARegularFile));
    }
    match read_bounded(file, MAX_STORY_FILE_BYTES) {
        Ok(Ok(text)) => Ok(Some(text)),
        Ok(Err(why)) => Err(refused_err(path, &why)),
        Err(e) => Err(io_err(path, e)),
    }
}

/// Reads a JSON object file in the story folder. A missing or empty file
/// reads as `None`; links and files over the cap are errors (see
/// `read_optional`).
pub fn read_json_object(path: &Path) -> Result<Option<Value>, FsError> {
    parse_object(path, read_optional(path)?)
}

/// Like `read_json_object`, for a file that belongs to the writer (see
/// `read_user_optional`).
pub fn read_user_json_object(path: &Path) -> Result<Option<Value>, FsError> {
    parse_object(path, read_user_optional(path)?)
}

fn parse_object(path: &Path, text: Option<String>) -> Result<Option<Value>, FsError> {
    let Some(text) = text else {
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

/// What a refused write says, in plain words.
const LINKED_FOLDER: &str = "this folder is a link; Skaldsaga never writes through a link, so nothing was changed. Replace the link with a real folder, or ask where it came from";

/// Refuses to touch anything below `dir` when `dir`, or any folder between
/// `root` and it, is a link (SR-21). `root` is the story folder; a path that
/// is not inside it belongs to the writer (their own settings) and is not
/// checked. A folder that does not exist yet is fine: nothing deeper can be
/// a link either.
///
/// Reading refuses links and so must writing: a story that was cloned or
/// shared can hold a link where a folder should be, and a write would
/// otherwise land wherever the link points.
pub fn refuse_links_in(root: &Path, dir: &Path) -> Result<(), FsError> {
    let Ok(inside) = dir.strip_prefix(root) else {
        return Ok(());
    };
    let mut here = root.to_path_buf();
    for part in inside.components() {
        match part {
            std::path::Component::Normal(name) => here.push(name),
            std::path::Component::CurDir => continue,
            _ => {
                return Err(FsError::Io {
                    path: dir.display().to_string(),
                    source: io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "this path leaves the story folder",
                    ),
                });
            }
        }
        match std::fs::symlink_metadata(&here) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(FsError::Io {
                    path: here.display().to_string(),
                    source: io::Error::new(io::ErrorKind::InvalidInput, LINKED_FOLDER),
                });
            }
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(io_err(&here, e)),
        }
    }
    Ok(())
}

/// `refuse_links_in` for the folder a file is going into. The file itself
/// may be a link: writing replaces the link and never writes through it.
pub fn refuse_links_under(root: &Path, file: &Path) -> Result<(), FsError> {
    match file.parent() {
        Some(parent) => refuse_links_in(root, parent),
        None => Ok(()),
    }
}

/// Counts temporary files so each name in this process is new.
static TMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// How many temporary names to try before giving up.
const TMP_ATTEMPTS: u64 = 100;

fn tmp_path(parent: &Path, file_name: &str, n: u64) -> std::path::PathBuf {
    parent.join(format!(".{file_name}.{}.{n}.tmp", std::process::id()))
}

/// Writes a file atomically: write a temporary file next to it, then rename
/// over the target. Creates the parent folder if needed. The temporary file
/// is created exclusively: if anything already sits at that name (a link
/// planted in a shared story folder, say), it is never opened or written
/// through, and the next name is tried (SR-22).
pub(crate) fn write_atomic(path: &Path, contents: &str) -> Result<(), FsError> {
    let start = TMP_COUNTER.fetch_add(TMP_ATTEMPTS, std::sync::atomic::Ordering::Relaxed);
    write_atomic_from(path, contents, start)
}

fn write_atomic_from(path: &Path, contents: &str, start: u64) -> Result<(), FsError> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent).map_err(|e| io_err(parent, e))?;
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut attempt = 0;
    let (tmp, mut file) = loop {
        let tmp = tmp_path(parent, &file_name, start + attempt);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
        {
            Ok(f) => break (tmp, f),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists && attempt + 1 < TMP_ATTEMPTS => {
                attempt += 1;
            }
            Err(e) => return Err(io_err(&tmp, e)),
        }
    };
    let result = (|| {
        file.write_all(contents.as_bytes())?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&tmp, path)
    })();
    if let Err(e) = result {
        let _ = std::fs::remove_file(&tmp);
        return Err(io_err(path, e));
    }
    Ok(())
}

/// Writes a JSON value atomically, pretty-printed.
pub(crate) fn write_json(path: &Path, value: &Value) -> Result<(), FsError> {
    write_atomic(path, &value.to_pretty_string())
}

/// Removes a file, treating "already gone" as success.
pub(crate) fn remove_optional(path: &Path) -> Result<bool, FsError> {
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
pub(crate) fn ensure_dir(path: &Path) -> Result<bool, FsError> {
    if path.is_dir() {
        return Ok(false);
    }
    std::fs::create_dir_all(path).map_err(|e| io_err(path, e))?;
    Ok(true)
}

/// Writes a file only if nothing is there yet. Returns true if it wrote.
/// Never replaces an existing file, so setup can't overwrite the writer's work.
pub(crate) fn write_if_missing(path: &Path, contents: &str) -> Result<bool, FsError> {
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

    #[test]
    fn a_read_stops_at_the_cap_however_much_the_source_holds() {
        // An endless source must not hang or fill memory: only the cap plus
        // one byte is ever read.
        assert_eq!(
            read_bounded(io::repeat(b'a'), 10).unwrap(),
            Err(Refused::TooBig)
        );
        assert_eq!(
            read_bounded(&b"hello"[..], 5).unwrap(),
            Ok("hello".to_string())
        );
        assert_eq!(
            read_bounded(&b"hello!"[..], 5).unwrap(),
            Err(Refused::TooBig)
        );
        // A multi-byte letter cut by the cap reads as too big, not as broken text.
        assert_eq!(
            read_bounded("éé".as_bytes(), 3).unwrap(),
            Err(Refused::TooBig)
        );
        assert_eq!(
            read_bounded(&[0xff, 0xfe][..], 10).unwrap(),
            Err(Refused::NotARegularFile)
        );
    }

    #[cfg(unix)]
    #[test]
    fn the_file_that_was_opened_must_be_the_one_that_was_checked() {
        let dir = TempDir::new();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        let hard = dir.path().join("hard");
        std::fs::write(&a, "x").unwrap();
        std::fs::write(&b, "x").unwrap();
        std::fs::hard_link(&a, &hard).unwrap();
        let meta = |p: &Path| std::fs::symlink_metadata(p).unwrap();
        assert!(same_file(&meta(&a), &meta(&a)));
        assert!(same_file(&meta(&a), &meta(&hard)));
        assert!(!same_file(&meta(&a), &meta(&b)));
    }

    #[test]
    fn story_reads_refuse_links_folders_and_big_files() {
        let dir = TempDir::new();
        let file = dir.path().join("s.json");
        std::fs::write(&file, "{\"a\": 1}").unwrap();
        assert_eq!(
            read_optional(&file).unwrap(),
            Some("{\"a\": 1}".to_string())
        );
        assert!(read_json_object(&file).unwrap().is_some());
        assert_eq!(read_optional(&dir.path().join("none")).unwrap(), None);

        let big = dir.path().join("big.json");
        std::fs::write(
            &big,
            vec![b'a'; usize::try_from(MAX_STORY_FILE_BYTES).unwrap() + 1],
        )
        .unwrap();
        assert!(read_optional(&big).is_err());
        assert!(read_json_object(&big).is_err());
        assert!(read_optional(dir.path()).is_err(), "a folder is not a file");
        #[cfg(unix)]
        {
            let link = dir.path().join("link.json");
            std::os::unix::fs::symlink(&file, &link).unwrap();
            let err = read_json_object(&link).unwrap_err().to_string();
            assert!(err.contains("never follows links"), "{err}");
            assert!(read_optional(&link).is_err());
        }
    }

    #[test]
    fn the_writers_own_files_may_be_links_but_stay_capped() {
        let dir = TempDir::new();
        let file = dir.path().join("mine.json");
        std::fs::write(&file, "{\"a\": 1}").unwrap();
        assert!(read_user_json_object(&file).unwrap().is_some());
        assert_eq!(read_user_optional(&dir.path().join("none")).unwrap(), None);
        #[cfg(unix)]
        {
            let link = dir.path().join("dotfiles-link.json");
            std::os::unix::fs::symlink(&file, &link).unwrap();
            assert!(read_user_json_object(&link).unwrap().is_some());
        }
        let big = dir.path().join("big.json");
        std::fs::write(
            &big,
            vec![b'a'; usize::try_from(MAX_STORY_FILE_BYTES).unwrap() + 1],
        )
        .unwrap();
        assert!(read_user_json_object(&big).is_err());
        assert!(
            read_user_optional(dir.path()).is_err(),
            "a folder is not a file"
        );
    }

    #[test]
    fn writes_are_refused_below_a_link_but_not_elsewhere() {
        let dir = TempDir::new();
        let root = dir.path().join("story");
        let elsewhere = dir.path().join("elsewhere");
        std::fs::create_dir_all(root.join("spec/people")).unwrap();
        std::fs::create_dir_all(&elsewhere).unwrap();

        // Plain folders, and folders that do not exist yet, are fine.
        assert!(refuse_links_under(&root, &root.join("spec/people/p.md")).is_ok());
        assert!(refuse_links_under(&root, &root.join("spec/new/deeper/p.md")).is_ok());
        assert!(refuse_links_in(&root, &root.join("spec/people")).is_ok());
        // A path outside the story is the writer's own and is not checked.
        assert!(refuse_links_under(&root, &elsewhere.join("x.json")).is_ok());
        // A path that climbs out of the story is refused.
        assert!(refuse_links_under(&root, &root.join("spec/../../x.md")).is_err());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&elsewhere, root.join("spec/places")).unwrap();
            let err = refuse_links_under(&root, &root.join("spec/places/p.md"))
                .unwrap_err()
                .to_string();
            assert!(err.contains("never writes through a link"), "{err}");
            assert!(refuse_links_in(&root, &root.join("spec/places")).is_err());
            assert!(refuse_links_in(&root, &root.join("spec/places/deeper")).is_err());
            // A link in the middle of the way is caught too.
            std::os::unix::fs::symlink(&elsewhere, root.join("spec/people/more")).unwrap();
            assert!(refuse_links_under(&root, &root.join("spec/people/more/x/p.md")).is_err());
            // The file itself may be a link: a write replaces it and never goes through it.
            std::os::unix::fs::symlink(&elsewhere, root.join("spec/people/p.md")).unwrap();
            assert!(refuse_links_under(&root, &root.join("spec/people/p.md")).is_ok());
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

    #[cfg(unix)]
    #[test]
    fn a_planted_link_at_the_temporary_name_is_never_written_through() {
        let dir = TempDir::new();
        let target = dir.path().join("story.json");
        let victim = dir.path().join("victim.txt");
        std::fs::write(&victim, "keep me").unwrap();
        // Occupy the first two names the write would try.
        for n in [7, 8] {
            std::os::unix::fs::symlink(&victim, tmp_path(dir.path(), "story.json", n)).unwrap();
        }
        write_atomic_from(&target, "{\"a\":1}", 7).unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "{\"a\":1}");
        assert_eq!(std::fs::read_to_string(&victim).unwrap(), "keep me");
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
