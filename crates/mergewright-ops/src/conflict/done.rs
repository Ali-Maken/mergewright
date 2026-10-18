//! A file this step conflicted on and git no longer calls unmerged, read
//! for the resolver's read-only view (m6-conflicts/25).
//!
//! What is shown is the **working-tree file**: what the takes produced,
//! what `e` opens, and what a reopened file shows. When it differs from
//! what is staged the status says so, and [`DoneFile`] carries that fact
//! rather than the app guessing it.

use std::io;
use std::path::Path;

use mergewright_git::conflict::{ConflictFile, ReadError};
use mergewright_git::repo::RepoHandle;
use mergewright_git::status::WorkingTreeStatus;

use super::{OpenError, in_worktree};
use crate::operation::Refusal;

/// A done file, as the resolver shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DoneFile {
    /// Text: the file on disk, parsed.
    Text {
        /// The parsed file. A file resolved by takes has no blocks left.
        file: ConflictFile,
        /// The status shows a working-tree change: the file on disk is not
        /// what `git add` staged.
        unstaged: bool,
    },
    /// git calls the file on disk binary (a NUL in the first 8 KiB).
    Binary {
        /// As [`DoneFile::Text`]'s.
        unstaged: bool,
    },
    /// Nothing on disk: resolved by removing it.
    Removed,
}

/// Reads a done path for the resolver's read-only view.
///
/// # Errors
/// [`OpenError::Refused`] with [`Refusal::Unmerged`] when the status still
/// calls the path unmerged — a defensive check: the caller routes such a
/// path to [`super::open_conflict`] instead. [`OpenError::TooLarge`],
/// [`OpenError::Markers`] or [`OpenError::Read`] when the file cannot be
/// read.
pub fn open_done(
    repo: &RepoHandle,
    status: &WorkingTreeStatus,
    path: &Path,
) -> Result<DoneFile, OpenError> {
    let entry = status.entry(path);
    if entry.is_some_and(|entry| entry.unmerged) {
        return Err(OpenError::Refused(Refusal::Unmerged {
            paths: vec![path.to_path_buf()],
        }));
    }
    let unstaged = entry.is_some_and(|entry| entry.unstaged.is_some());
    match mergewright_git::conflict::read(&in_worktree(repo, path)) {
        Ok(file) => Ok(DoneFile::Text { file, unstaged }),
        Err(ReadError::Binary) => Ok(DoneFile::Binary { unstaged }),
        Err(ReadError::Io(error)) if error.kind() == io::ErrorKind::NotFound => {
            Ok(DoneFile::Removed)
        }
        Err(ReadError::TooLarge { bytes }) => Err(OpenError::TooLarge { bytes }),
        Err(ReadError::Parse(error)) => Err(OpenError::Markers(error)),
        Err(ReadError::Io(error)) => Err(OpenError::Read(error)),
    }
}
