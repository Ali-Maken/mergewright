//! The repository root's `.git-blame-ignore-revs`: found, counted, and
//! summarised for the blame status line.

use std::path::PathBuf;

use super::{IGNORE_REVS_FILE, IgnoreRevs};
use crate::repo::RepoHandle;

/// The root's ignore-revs file, as found on disk.
pub(super) struct FoundIgnoreRevs {
    /// Its absolute path, for `--ignore-revs-file`.
    pub(super) path: PathBuf,
    /// How many revisions it lists; `None` when it exists and could not be
    /// read (git, handed it, reports why).
    revisions: Option<usize>,
}

impl FoundIgnoreRevs {
    /// What [`Blame::ignore_revs`] says: only a readable file that lists
    /// something is worth a line on screen.
    pub(super) fn summary(self, applied: bool) -> Option<IgnoreRevs> {
        let revisions = self.revisions.filter(|&count| count > 0)?;
        Some(IgnoreRevs {
            file: PathBuf::from(IGNORE_REVS_FILE),
            revisions,
            applied,
        })
    }
}

/// The root's `.git-blame-ignore-revs`, if there is one. A bare repository
/// has no root to look in.
pub(super) fn find_ignore_revs(repo: &RepoHandle) -> Option<FoundIgnoreRevs> {
    let path = repo.workdir()?.join(IGNORE_REVS_FILE);
    match std::fs::read(&path) {
        Ok(bytes) => Some(FoundIgnoreRevs {
            path,
            revisions: Some(count_revisions(&bytes)),
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(_unreadable) => Some(FoundIgnoreRevs {
            path,
            revisions: None,
        }),
    }
}

/// Non-comment, non-blank lines: what git reads as revisions.
pub(super) fn count_revisions(bytes: &[u8]) -> usize {
    bytes
        .split(|&b| b == b'\n')
        .map(<[u8]>::trim_ascii)
        .filter(|line| !line.is_empty() && !line.starts_with(b"#"))
        .count()
}
