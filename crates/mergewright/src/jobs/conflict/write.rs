//! The resolver's two writes: a take (m6-conflicts/04) and the last pick
//! put back (`u`, ease-pass 05). Thin, as the rest of `jobs::conflict`:
//! `mergewright_ops::conflict::{apply_take, apply_take_all, apply_untake}` decide
//! and write; this opens the repository and sorts the failures by what the
//! app must do about them.

use std::path::Path;

use mergewright_git::conflict::{ConflictFile, Take};
use mergewright_git::run::Stop;
use mergewright_ops::conflict::{Restore, TakeError};

use crate::repos::RepoStore;

/// Why a take did not land, with the same distinction kept the same way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TakeFailure {
    /// `mergewright_ops`' own words, ready to show.
    pub words: String,
    /// The bytes on disk are not the ones the take was chosen against, so
    /// nothing was written and the document must be read again.
    pub reread: bool,
}

/// Takes one side of one block — or of every open block (`TakeAt::Every`)
/// — and writes the file back once, returning the
/// document as it now stands.
///
/// # Errors
/// [`TakeFailure`], carrying `mergewright_ops::conflict::TakeError`'s own
/// `Display`. `Changed` is the one the resolver acts on: it re-reads rather
/// than writing over an edit made outside Mergewright.
pub fn take(
    store: &RepoStore,
    path: &Path,
    file: &ConflictFile,
    at: crate::update::TakeAt,
    side: Take,
) -> Result<ConflictFile, TakeFailure> {
    let repo = store.shared().map_err(|error| TakeFailure {
        words: error.to_string(),
        reread: false,
    })?;
    let handle = repo.handle();
    let taken = match at {
        crate::update::TakeAt::Block(block) => {
            mergewright_ops::conflict::apply_take(&handle, path, file, block, side)
        }
        crate::update::TakeAt::Every => {
            mergewright_ops::conflict::apply_take_all(&handle, path, file, side)
        }
    };
    taken.map_err(|error| TakeFailure {
        words: error.to_string(),
        reread: matches!(error, TakeError::Changed { .. }),
    })
}

/// Why an untake wrote nothing, sorted by what becomes of the pick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UntakeFailure {
    /// The pick no longer applies: the file changed since its write, is
    /// out of step with it, or git no longer calls it conflicted. Nothing
    /// was written, and every pick in the file goes — each was chosen
    /// against a document that is gone. The words follow the path
    /// (`changed on disk since that pick`).
    Stale(String),
    /// The write could not be made (I/O, the repository would not open):
    /// the pick still applies and goes back on the stack. The words are
    /// the error's own.
    Failed(String),
}

/// Puts back what a pick changed in `path`, one write against
/// `file.fingerprint`, and returns the document as it now stands.
///
/// The status is read first, as `open` does: a path staged since the pick
/// — `git add` in the shell — has bytes the pick's fingerprint still
/// matches, and writing its markers back would undo a resolution git
/// already holds.
///
/// # Errors
/// [`UntakeFailure::Stale`] or [`UntakeFailure::Failed`]; never a partial
/// write (`apply_untake` decides the whole document before it writes).
pub fn untake(
    store: &RepoStore,
    path: &Path,
    file: &ConflictFile,
    restore: &[Restore],
    stop: &dyn Stop,
) -> Result<ConflictFile, UntakeFailure> {
    let failed = |error: &dyn std::fmt::Display| UntakeFailure::Failed(error.to_string());
    let repo = store.shared().map_err(|error| failed(&error))?;
    let handle = repo.handle();
    let status = mergewright_git::status::read(&handle, stop).map_err(|error| failed(&error))?;
    if !status.entry(path).is_some_and(|entry| entry.unmerged) {
        return Err(UntakeFailure::Stale(
            "is not conflicted any more".to_owned(),
        ));
    }
    mergewright_ops::conflict::apply_untake(&handle, path, file, restore).map_err(|error| {
        match error {
            TakeError::Changed { .. } => {
                UntakeFailure::Stale("changed on disk since that pick".to_owned())
            }
            TakeError::NoSuchBlock(_) => {
                UntakeFailure::Stale("is out of step with that pick".to_owned())
            }
            TakeError::Write(error) => failed(&error),
        }
    })
}
