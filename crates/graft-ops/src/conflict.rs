//! Opening a conflicted file and taking a side per block: the job-side half
//! of the resolver.
//!
//! Nothing here is an [`Operation`](crate::operation::Operation) and nothing
//! here writes a journal line. A take is an `fs` write with no `git` command
//! in it — the same kind of thing a hand edit in `$EDITOR` is — and it
//! happens inside the live operation's `Pending` span, whose own entry
//! already holds the pre-operation backup (design note hard choice 2). The
//! whole-file shapes, which *are* `git` writes, go through
//! [`Operation::ResolveWholeFile`](crate::operation::Operation::ResolveWholeFile)
//! instead.
//!
//! The seam rule still holds: `ops` decides, `app` wires. The app calls
//! these four functions and never `graft_git::conflict::write_back` itself.

use std::io;
use std::path::{Path, PathBuf};

use graft_core::oid::Oid;
use graft_git::conflict::{
    BaseMismatch, ConflictFile, Diff3Error, ParseError, ReadError, Take, WriteError,
};
use graft_git::repo::RepoHandle;
use graft_git::status::{UnmergedHow, WorkingTreeStatus};

use crate::operation::Refusal;

mod done;
pub(crate) mod merge_msg;
mod set;

pub use done::{DoneFile, open_done};
pub(crate) use set::conflicts_of;
pub use set::{ConflictSet, ConflictSetError, operation_conflicts};

/// What the resolver got when it opened a path.
#[derive(Clone, Debug)]
pub struct Opened {
    /// The parsed file — `None` for a whole-file shape (no markers to parse)
    /// and for a binary `UU`/`AA`. The app then offers `o`/`t` over the
    /// whole file rather than per block.
    pub file: Option<ConflictFile>,
    /// The shape the status read.
    pub how: UnmergedHow,
    /// git calls the working-tree file binary (a NUL in the first 8 KiB).
    pub binary: bool,
}

/// Why a conflicted path could not be opened.
#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    /// The path is not one the resolver can open.
    #[error("{}", .0.explain(&crate::keys::NoKeys))]
    Refused(Refusal),
    /// The file could not be read.
    #[error("could not read the file: {0}")]
    Read(#[from] io::Error),
    /// Larger than the resolver handles.
    #[error("the file is {bytes} bytes, which is more than the resolver handles")]
    TooLarge {
        /// The file's size.
        bytes: u64,
    },
    /// The markers do not parse. Surfaced with its line rather than shown as
    /// "no conflicts here": a file whose markers are malformed is exactly
    /// the one the user must be told about.
    #[error("{0}")]
    Markers(#[from] ParseError),
}

/// Reads one conflicted path for the resolver.
///
/// A shape with markers ([`UnmergedHow::has_markers`]) is parsed; every other
/// shape, and a binary file of a marker shape, comes back with `file: None`
/// and is resolved whole.
///
/// # Errors
/// [`OpenError::Refused`] with [`Refusal::NotUnmerged`] when the status does
/// not call the path unmerged; [`OpenError::Markers`] when the file's markers
/// do not parse; [`OpenError::TooLarge`] or [`OpenError::Read`] otherwise.
pub fn open_conflict(
    repo: &RepoHandle,
    status: &WorkingTreeStatus,
    path: &Path,
) -> Result<Opened, OpenError> {
    let how = unmerged_how(status, path).map_err(OpenError::Refused)?;
    if !how.has_markers() {
        return Ok(Opened {
            file: None,
            how,
            binary: false,
        });
    }
    match graft_git::conflict::read(&in_worktree(repo, path)) {
        Ok(file) => Ok(Opened {
            file: Some(file),
            how,
            binary: false,
        }),
        // Binary is not a failure: it is the other way to resolve this path.
        Err(ReadError::Binary) => Ok(Opened {
            file: None,
            how,
            binary: true,
        }),
        Err(ReadError::TooLarge { bytes }) => Err(OpenError::TooLarge { bytes }),
        Err(ReadError::Parse(error)) => Err(OpenError::Markers(error)),
        Err(ReadError::Io(error)) => Err(OpenError::Read(error)),
    }
}

/// Why a take could not be applied.
#[derive(Debug, thiserror::Error)]
pub enum TakeError {
    /// The file on disk is not the one the take was chosen against —
    /// `$EDITOR`, a mergetool or another Graft got there first. Nothing is
    /// written.
    #[error("the file changed on disk since it was read")]
    Changed {
        /// The blob id of what is on disk now.
        now: Oid,
    },
    /// The document has no block by that index.
    #[error("the file has no conflict block {0}")]
    NoSuchBlock(usize),
    /// The file could not be written back.
    #[error(transparent)]
    Write(WriteError),
}

/// Takes one side of one block of `file` and writes the result to disk,
/// returning the document as it now stands.
///
/// `file` is the document the user is looking at, and `file.fingerprint` is
/// what the bytes on disk have to still be: [`graft_git::conflict::write_back`]
/// compares them itself and refuses with [`TakeError::Changed`], leaving the
/// bytes exactly as they are. This is the whole reason a take writes at once
/// rather than buffering (design note hard choice 2): the file on screen and
/// the file on disk are the same file, and an edit made outside Mergewright is
/// never silently undone.
///
/// The document is what carries the takes, not the file on disk. A block
/// that has been taken renders as its taken lines alone, so re-reading the
/// path would lose which blocks were resolved and re-index the rest under
/// the cursor; passing the document in keeps `blocks()` stable for the whole
/// session and lets a take be *replaced* rather than appended.
///
/// # Errors
/// [`TakeError::Changed`], [`TakeError::NoSuchBlock`], or the write error
/// underneath.
pub fn apply_take(
    repo: &RepoHandle,
    path: &Path,
    file: &ConflictFile,
    block: usize,
    take: Take,
) -> Result<ConflictFile, TakeError> {
    let mut next = file.clone();
    next.take(block, take)
        .map_err(|no| TakeError::NoSuchBlock(no.0))?;
    match graft_git::conflict::write_back(&in_worktree(repo, path), &next, file.fingerprint) {
        Ok(fingerprint) => {
            next.fingerprint = fingerprint;
            Ok(next)
        }
        Err(WriteError::Changed { now }) => Err(TakeError::Changed { now }),
        Err(error) => Err(TakeError::Write(error)),
    }
}

/// [`apply_take`] for every block still open in `file`: one write, against
/// `file.fingerprint`. Blocks already taken keep their take and their index
/// — which is why this takes the document, not an `Oid` (ticket m6-16 D1).
///
/// # Errors
/// [`TakeError::Changed`] leaving the bytes untouched, or the write error.
pub fn apply_take_all(
    repo: &RepoHandle,
    path: &Path,
    file: &ConflictFile,
    take: Take,
) -> Result<ConflictFile, TakeError> {
    let mut next = file.clone();
    next.take_all(take);
    match graft_git::conflict::write_back(&in_worktree(repo, path), &next, file.fingerprint) {
        Ok(fingerprint) => {
            next.fingerprint = fingerprint;
            Ok(next)
        }
        Err(WriteError::Changed { now }) => Err(TakeError::Changed { now }),
        Err(error) => Err(TakeError::Write(error)),
    }
}

/// Why the base could not be attached.
#[derive(Debug, thiserror::Error)]
pub enum BaseError {
    /// One of the three stages is missing, so git's own `diff3` has nothing
    /// to reproduce: a whole-file shape, or a two-stage conflict like `AA`.
    #[error("this conflict has no common ancestor to show")]
    NoBase,
    /// The file no longer matches what git produced — a hand edit inside a
    /// block. The honest answer is "base unavailable after edits", never a
    /// base aligned by position.
    #[error("{0}")]
    Mismatch(BaseMismatch),
    /// git's own merge could not be reproduced.
    #[error(transparent)]
    Diff3(#[from] Diff3Error),
}

/// Attaches the common ancestor's lines to every block of `file`, from git's
/// own `merge-file --diff3` over the three stages. Returns how many blocks
/// were given a base.
///
/// # Errors
/// [`BaseError::NoBase`] when a stage is missing, [`BaseError::Mismatch`]
/// when the file has been edited away from what git produced,
/// [`BaseError::Diff3`] when git itself failed.
pub fn base_for(
    repo: &RepoHandle,
    path: &Path,
    file: &mut ConflictFile,
) -> Result<usize, BaseError> {
    let Some(diff3) = graft_git::conflict::diff3_file(repo, path)? else {
        return Err(BaseError::NoBase);
    };
    graft_git::conflict::attach_base(file, &diff3).map_err(BaseError::Mismatch)
}

/// The refusal `⏎` (mark resolved) earns while blocks are still open, or
/// `None` when every block has a side.
///
/// The app calls this before it builds the `Stage` operation, so the user
/// reads *why* in Graft's own words rather than watching a `git add` run and
/// come back refused. `Refusal::MarkersRemain`, which `Operation::check`
/// raises from the facts, stays as the second line of defence: this one can
/// only see the file the resolver is holding.
#[must_use]
pub fn mark_resolved_refusal(path: &Path, file: &ConflictFile) -> Option<Refusal> {
    let open = file.open();
    (open > 0).then(|| Refusal::ConflictsOpen {
        path: path.to_path_buf(),
        open,
    })
}

/// The status's word for a path, or the refusal that it has none.
fn unmerged_how(status: &WorkingTreeStatus, path: &Path) -> Result<UnmergedHow, Refusal> {
    status
        .entry(path)
        .and_then(|entry| entry.unmerged_how)
        .ok_or_else(|| Refusal::NotUnmerged {
            path: path.to_path_buf(),
        })
}

/// A status path is relative to the working tree root; the file reads and
/// writes here need the real one. A bare repository has no working tree and
/// so no unmerged path either, which is why the fallback can simply be the
/// path as given.
fn in_worktree(repo: &RepoHandle, path: &Path) -> PathBuf {
    match repo.workdir() {
        Some(workdir) => workdir.join(path),
        None => path.to_path_buf(),
    }
}

#[cfg(test)]
mod tests;
