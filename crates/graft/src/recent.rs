//! The repositories Graft opened lately, for the empty state's RECENT list
//! (m8-polish-and-ship/08, mock-ups §1.15).
//!
//! One file, `<config_dir>/graft/recent`: one line per repository,
//! `<unix_secs>\t<canonical path>`, newest first, at most [`MAX_RECENT`],
//! never the same path twice. Written whole to a temporary file beside it
//! and renamed over it, so a crash leaves the old list or the new one, never
//! half of either.
//!
//! A line that does not parse is skipped when reading and counted, so the
//! screen can say so; the next [`remember`] drops it and says that too.

use std::ffi::OsStr;
use std::fmt;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// How many repositories the list keeps.
pub(crate) const MAX_RECENT: usize = 20;

/// One remembered repository.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Recent {
    /// Its canonical working-tree path.
    pub path: PathBuf,
    /// When Graft last opened it.
    pub opened: SystemTime,
}

/// The list as read: newest first, at most [`MAX_RECENT`], deduplicated.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RecentList {
    pub entries: Vec<Recent>,
    /// Lines that did not parse and were left out.
    pub skipped: usize,
}

/// What [`remember`] did besides adding the repository.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Remembered {
    /// Unreadable lines the rewrite dropped.
    pub dropped: usize,
}

/// Why the list could not be read or written. `Display` is the reason half
/// of the status line (`recent: not saved · <reason>`).
#[derive(Debug)]
pub(crate) enum RecentError {
    /// Reading, writing or renaming the file failed.
    Io { path: PathBuf, error: io::Error },
    /// The path has a newline or a tab in it: the file's format cannot hold
    /// it, so it is not remembered (said, never silently skipped).
    Unstorable(PathBuf),
}

impl fmt::Display for RecentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, error } => write!(f, "{}: {error}", path.display()),
            Self::Unstorable(path) => write!(
                f,
                "{} has a newline or tab in its name",
                path.display().to_string().escape_default()
            ),
        }
    }
}

/// The file the list lives in, under `config_dir` (`$XDG_CONFIG_HOME` or
/// `~/.config`).
pub(crate) fn recent_file(config_dir: &Path) -> PathBuf {
    config_dir.join("graft").join("recent")
}

/// The config directory recents live under, from the environment the config
/// file is found by; `None` when there is none (`recent: no config folder`).
pub(crate) fn config_dir() -> Option<PathBuf> {
    crate::config::default_path()
        .as_deref()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .map(Path::to_path_buf)
}

/// Read the list. A missing file is an empty list.
///
/// # Errors
/// The file exists but cannot be read.
pub(crate) fn read_recent(config_dir: &Path) -> Result<RecentList, RecentError> {
    let path = recent_file(config_dir);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(RecentList::default()),
        Err(error) => return Err(RecentError::Io { path, error }),
    };
    Ok(parse(&bytes))
}

/// Put `repo` at the top of the list, stamped `now`, and write the list back.
///
/// # Errors
/// `repo` cannot be stored, or the file cannot be read or written. The list
/// on disk is unchanged in every error case.
pub(crate) fn remember(
    config_dir: &Path,
    repo: &Path,
    now: SystemTime,
) -> Result<Remembered, RecentError> {
    let repo = repo.canonicalize().unwrap_or_else(|_unresolvable| {
        // A path that cannot be resolved (gone between opening it and
        // getting here) is remembered as given; reading it back later shows
        // it as a row whose open fails with its reason, as any stale row.
        repo.to_path_buf()
    });
    if repo.as_os_str().as_bytes().iter().any(|&b| b == b'\n' || b == b'\t') {
        return Err(RecentError::Unstorable(repo));
    }
    let RecentList { entries, skipped } = read_recent(config_dir)?;
    let mut list = vec![Recent {
        path: repo.clone(),
        opened: now,
    }];
    list.extend(entries.into_iter().filter(|entry| entry.path != repo));
    list.truncate(MAX_RECENT);
    write_atomically(&recent_file(config_dir), &render(&list))?;
    Ok(Remembered { dropped: skipped })
}

/// The lines as they are written.
fn render(list: &[Recent]) -> Vec<u8> {
    let mut out = Vec::new();
    for entry in list {
        let secs = entry
            .opened
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_secs());
        out.extend_from_slice(secs.to_string().as_bytes());
        out.push(b'\t');
        out.extend_from_slice(entry.path.as_os_str().as_bytes());
        out.push(b'\n');
    }
    out
}

/// The file's lines, newest first as written; a repeated path keeps its
/// first (newest) line, and only [`MAX_RECENT`] are kept.
fn parse(bytes: &[u8]) -> RecentList {
    let mut list = RecentList::default();
    for line in bytes.split(|&b| b == b'\n').filter(|line| !line.is_empty()) {
        let Some(entry) = parse_line(line) else {
            list.skipped += 1;
            continue;
        };
        if list.entries.iter().any(|seen| seen.path == entry.path) {
            continue;
        }
        list.entries.push(entry);
    }
    // Newest first even if the file was edited by hand out of order.
    list.entries.sort_by(|a, b| b.opened.cmp(&a.opened));
    list.entries.truncate(MAX_RECENT);
    list
}

fn parse_line(line: &[u8]) -> Option<Recent> {
    let tab = line.iter().position(|&b| b == b'\t')?;
    let (secs, path) = (&line[..tab], &line[tab + 1..]);
    let secs: u64 = std::str::from_utf8(secs).ok()?.parse().ok()?;
    if path.is_empty() || path.contains(&b'\t') || path.first() != Some(&b'/') {
        return None;
    }
    Some(Recent {
        path: PathBuf::from(OsStr::from_bytes(path)),
        opened: UNIX_EPOCH + Duration::from_secs(secs),
    })
}

/// Write `bytes` to a temporary file beside `path` and rename it over
/// `path`. On any failure the temporary is removed and `path` is untouched.
fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), RecentError> {
    let io_error = |error| RecentError::Io {
        path: path.to_path_buf(),
        error,
    };
    let dir = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir).map_err(io_error)?;
    let temp = dir.join(format!(".recent.{}.tmp", std::process::id()));
    let written = std::fs::write(&temp, bytes).and_then(|()| std::fs::rename(&temp, path));
    if let Err(error) = written {
        // The rename failed (or the write did): take the temporary away so
        // no half-written list is left lying beside the real one. Failing
        // to remove it too is folded into the reason: both are said.
        return Err(match std::fs::remove_file(&temp) {
            Ok(()) => io_error(error),
            Err(remove) if remove.kind() == io::ErrorKind::NotFound => io_error(error),
            Err(remove) => io_error(io::Error::new(
                error.kind(),
                format!("{error}; and {} was left behind: {remove}", temp.display()),
            )),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        reason = "a scratch directory that cannot be made leaves nothing to test"
    )]

    use std::path::{Path, PathBuf};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use super::{MAX_RECENT, RecentError, read_recent, recent_file, remember, write_atomically};

    fn at(secs: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(secs)
    }

    /// A real folder under `root`, canonical, as `remember` stores it.
    fn repo(root: &Path, name: &str) -> PathBuf {
        let path = root.join(name);
        std::fs::create_dir_all(&path).expect("repo folder");
        path.canonicalize().expect("canonical")
    }

    #[test]
    fn a_missing_file_is_an_empty_list() {
        let config = tempfile::tempdir().expect("config dir");

        let list = read_recent(config.path()).expect("reads");

        assert!(list.entries.is_empty());
        assert_eq!(list.skipped, 0);
    }

    #[test]
    fn the_newest_comes_first() {
        let config = tempfile::tempdir().expect("config dir");
        let (a, b) = (repo(config.path(), "a"), repo(config.path(), "b"));

        remember(config.path(), &a, at(100)).expect("remembers a");
        remember(config.path(), &b, at(200)).expect("remembers b");

        let list = read_recent(config.path()).expect("reads");
        let paths: Vec<_> = list.entries.iter().map(|e| e.path.clone()).collect();
        assert_eq!(paths, vec![b, a]);
        assert_eq!(list.entries[0].opened, at(200));
    }

    /// Opened twice under two spellings of one folder: one row, the newer.
    #[test]
    fn the_same_repository_is_listed_once_by_its_canonical_path() {
        let config = tempfile::tempdir().expect("config dir");
        let (a, b) = (repo(config.path(), "a"), repo(config.path(), "b"));

        remember(config.path(), &a, at(100)).expect("a");
        remember(config.path(), &b, at(150)).expect("b");
        remember(config.path(), &a.join("..").join("a"), at(200)).expect("a again");

        let list = read_recent(config.path()).expect("reads");
        let paths: Vec<_> = list.entries.iter().map(|e| e.path.clone()).collect();
        assert_eq!(paths, vec![a, b]);
    }

    #[test]
    fn the_list_keeps_twenty() {
        let config = tempfile::tempdir().expect("config dir");
        for n in 0..25 {
            let path = repo(config.path(), &format!("r{n}"));
            remember(config.path(), &path, at(1000 + n)).expect("remembers");
        }

        let list = read_recent(config.path()).expect("reads");

        assert_eq!(list.entries.len(), MAX_RECENT);
        assert_eq!(list.entries[0].path, repo(config.path(), "r24"));
    }

    #[test]
    fn unreadable_lines_are_skipped_and_counted_then_dropped() {
        let config = tempfile::tempdir().expect("config dir");
        let good = repo(config.path(), "good");
        let file = recent_file(config.path());
        std::fs::create_dir_all(file.parent().expect("dir")).expect("dir");
        let text = format!(
            "nonsense\n100\t{}\nabc\t/x\n200\trelative/path\n",
            good.display()
        );
        std::fs::write(&file, text).expect("written");

        let list = read_recent(config.path()).expect("reads");
        assert_eq!(list.skipped, 3);
        assert_eq!(list.entries.len(), 1);

        let other = repo(config.path(), "other");
        let remembered = remember(config.path(), &other, at(300)).expect("remembers");
        assert_eq!(remembered.dropped, 3);
        let list = read_recent(config.path()).expect("reads");
        assert_eq!(list.skipped, 0, "the rewrite dropped them");
        assert_eq!(list.entries.len(), 2);
    }

    #[test]
    fn a_path_with_a_newline_is_refused_and_the_file_unchanged() {
        let config = tempfile::tempdir().expect("config dir");
        let odd = repo(config.path(), "two\nlines");

        let answer = remember(config.path(), &odd, at(100));

        assert!(matches!(answer, Err(RecentError::Unstorable(_))), "{answer:?}");
        assert!(!recent_file(config.path()).exists());
    }

    /// The rename cannot land (the target is a directory): the old list is
    /// untouched and no temporary is left beside it.
    #[test]
    fn a_failed_rename_leaves_no_partial_file() {
        let config = tempfile::tempdir().expect("config dir");
        let file = recent_file(config.path());
        std::fs::create_dir_all(&file).expect("a directory where the file goes");
        std::fs::write(file.join("keep"), b"x").expect("non-empty directory");

        let answer = write_atomically(&file, b"100\t/a\n");

        assert!(matches!(answer, Err(RecentError::Io { .. })), "{answer:?}");
        assert_eq!(std::fs::read(file.join("keep")).expect("kept"), b"x");
        let leftovers: Vec<_> = std::fs::read_dir(file.parent().expect("dir"))
            .expect("lists")
            .map(|entry| entry.expect("entry").file_name())
            .collect();
        assert_eq!(leftovers, vec![std::ffi::OsString::from("recent")]);
    }
}
