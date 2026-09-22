//! The resolver's two jobs: opening a conflicted path, and taking one side
//! of one block (m6-conflicts/04).
//!
//! Both are thin: `graft_ops::conflict` decides everything, this file only
//! opens the repository and turns `ops`' errors into the words the status
//! line shows. The app never calls `graft_git::conflict`'s writing half
//! itself — that is the seam `tests/it/seam_rule.rs` locks.

use std::path::Path;

use graft_git::conflict::{ConflictFile, Take};
use graft_git::run::Stop;
use graft_git::status::UnmergedHow;

use crate::repos::RepoStore;

/// What [`open`] found: `graft_ops::conflict::Opened`, as the app holds it.
///
/// A copy rather than the `ops` type itself because a [`Product`] is
/// compared with `assert_eq!` — every one of them is `PartialEq` — and
/// `Opened` is not. The three fields are, so the mirror is (and `ops` owning
/// its own shape is the point of the seam, not an accident of it).
///
/// [`Product`]: crate::update::Product
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenedFile {
    /// The parsed file — `None` for a whole-file shape (no markers to
    /// parse) and for a binary `UU`/`AA`.
    pub file: Option<ConflictFile>,
    /// The shape git's status read (`both modified`, `deleted by them`, …).
    pub how: UnmergedHow,
    /// git calls the working-tree file binary (a NUL in the first 8 KiB).
    pub binary: bool,
}

/// Why a path could not be opened, with the one distinction the resolver
/// acts on kept as a field rather than left in the words.
///
/// `graft_ops::conflict::OpenError` carries it in its type and this job is
/// the last place that type exists, so the flag is read off it here. The
/// alternative — matching the rendered sentence back at the other end — is
/// a control-flow decision resting on a string nobody promised not to
/// reword.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenFailure {
    /// `graft_ops`' own words, ready to show.
    pub words: String,
    /// git does not call the path unmerged. After a handover that means the
    /// user resolved it in `$EDITOR` or a mergetool; on a first open it
    /// means the row was never a conflict.
    pub not_unmerged: bool,
}

/// Why a take did not land, with the same distinction kept the same way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TakeFailure {
    /// `graft_ops`' own words, ready to show.
    pub words: String,
    /// The bytes on disk are not the ones the take was chosen against, so
    /// nothing was written and the document must be read again.
    pub reread: bool,
}

/// Reads one conflicted path for the resolver.
///
/// The status is read **here**, not taken from `AppState`: this job is what
/// runs after `$EDITOR` or a mergetool has been at the file, and the one
/// question it has to be able to answer is whether the path is still
/// unmerged at all. A status read before the handover could never say no.
///
/// # Errors
/// [`OpenFailure`], carrying `graft_ops::conflict::OpenError`'s own
/// `Display` — which is `Refusal::explain()` for a path git does not call
/// unmerged.
pub fn open(store: &RepoStore, path: &Path, stop: &dyn Stop) -> Result<OpenedFile, OpenFailure> {
    let repo = store.shared().map_err(|error| failed(&error))?;
    let handle = repo.handle();
    let status = graft_git::status::read(&handle, stop).map_err(|error| failed(&error))?;
    let opened = graft_ops::conflict::open_conflict(&handle, &status, path).map_err(|error| {
        OpenFailure {
            words: error.to_string(),
            not_unmerged: matches!(
                error,
                graft_ops::conflict::OpenError::Refused(
                    graft_ops::operation::Refusal::NotUnmerged { .. }
                )
            ),
        }
    })?;
    Ok(OpenedFile {
        file: opened.file,
        how: opened.how,
        binary: opened.binary,
    })
}

/// A repository or status read that failed: real, and never "not unmerged".
fn failed(error: &impl std::fmt::Display) -> OpenFailure {
    OpenFailure {
        words: error.to_string(),
        not_unmerged: false,
    }
}

/// Takes one side of one block and writes the file back, returning the
/// document as it now stands.
///
/// # Errors
/// [`TakeFailure`], carrying `graft_ops::conflict::TakeError`'s own
/// `Display`. `Changed` is the one the resolver acts on: it re-reads rather
/// than writing over an edit made outside Graft.
pub fn take(
    store: &RepoStore,
    path: &Path,
    file: &ConflictFile,
    block: usize,
    side: Take,
) -> Result<ConflictFile, TakeFailure> {
    let repo = store.shared().map_err(|error| TakeFailure {
        words: error.to_string(),
        reread: false,
    })?;
    graft_ops::conflict::apply_take(&repo.handle(), path, file, block, side).map_err(|error| {
        TakeFailure {
            words: error.to_string(),
            reread: matches!(error, graft_ops::conflict::TakeError::Changed { .. }),
        }
    })
}
