//! The resolver's two jobs: opening a conflicted path, and taking one side
//! of one block (m6-conflicts/04).
//!
//! Both are thin: `graft_ops::conflict` decides everything, this file only
//! opens the repository and turns `ops`' errors into the words the status
//! line shows. The app never calls `graft_git::conflict`'s writing half
//! itself — that is the seam `tests/it/seam_rule.rs` locks.

use std::path::Path;

use graft_git::conflict::{ConflictFile, Take};
use graft_git::repo::RepoHandle;
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
    /// The working-tree file's own lines, read for a whole-file shape's
    /// card (`file: None`) so the resolver can show what is on disk under
    /// it — empty when the shape has no file there (`BothDeleted`, or a
    /// side that removed it) and always empty when `file` is `Some`
    /// (m6-conflicts/05). Raw bytes: sanitised at the view, never here.
    pub lines: Vec<Vec<u8>>,
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
    let lines = if opened.file.is_none() {
        worktree_lines(&handle, path)
    } else {
        Vec::new()
    };
    Ok(OpenedFile {
        file: opened.file,
        how: opened.how,
        binary: opened.binary,
        lines,
    })
}

/// A whole-file shape's card shows the file as it stands on disk, when
/// there is one there (`DeletedByThem` keeps it, `BothDeleted` never has
/// one, …) — read directly rather than through `graft_git::conflict::read`,
/// which parses markers this shape does not have. A path that is not there,
/// or not readable, draws no lines rather than fail the whole open: the
/// card's `how`/`keys` line is what matters, and empty is a fine answer for
/// a file that is gone.
fn worktree_lines(handle: &RepoHandle, path: &Path) -> Vec<Vec<u8>> {
    let full = match handle.workdir() {
        Some(workdir) => workdir.join(path),
        None => path.to_path_buf(),
    };
    let Ok(bytes) = std::fs::read(&full) else {
        return Vec::new();
    };
    let mut lines: Vec<Vec<u8>> = bytes
        .split(|&byte| byte == b'\n')
        .map(|line| line.strip_suffix(b"\r").unwrap_or(line).to_vec())
        .collect();
    // A trailing `\n` splits into one empty line past the last real one;
    // drop it so `wc -l`'s count and this one agree.
    if lines.last().is_some_and(Vec::is_empty) {
        lines.pop();
    }
    lines
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

/// Attaches the merge base to every block of `file` (`B`, m6-conflicts/05).
///
/// `file` is the resolver's own copy, takes and all — `graft_ops::conflict::base_for`
/// mutates it in place rather than reading the path again, so a block
/// already taken keeps its base exactly as an open one does
/// (`ConflictFile::take` moves the block whole, `graft_git::conflict` tests).
///
/// # Errors
/// `graft_ops::conflict::BaseError`'s own words: `this conflict has no
/// common ancestor to show`, `base unavailable after edits`, or git's own
/// `diff3` failure.
pub fn load_base(
    store: &RepoStore,
    path: &Path,
    mut file: ConflictFile,
) -> Result<(ConflictFile, usize), String> {
    let repo = store.shared().map_err(|error| error.to_string())?;
    graft_ops::conflict::base_for(&repo.handle(), path, &mut file)
        .map(|attached| (file, attached))
        .map_err(|error| error.to_string())
}
