//! The stash list read, behind the `S` overlay
//! (m3-working-tree-undo-and-stash/06).
//!
//! `mergewright_git::stash::list` is the whole of it: the reflog of `refs/stash`
//! is the stash list (`as-built.md`, group 02), and one `diff-tree` per
//! entry gives the paths the row counts. Off the loop's thread, like every
//! other read.

use std::sync::Arc;

use mergewright_git::stash::StashEntry;

use super::Cancel;
use crate::repos::RepoStore;

/// Every stash entry, `stash@{0}` first, or why the list could not be read.
///
/// # Errors
/// The failure text, ready to show: the repository could not be opened, or
/// the reflog, a stash commit or one of its `diff-tree` spawns failed.
pub(super) fn read_stashes(
    repos: &RepoStore,
    cancel: &Cancel,
) -> Result<Arc<Vec<StashEntry>>, String> {
    let repo = repos.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    mergewright_git::stash::list(&handle, cancel)
        .map(Arc::new)
        .map_err(|error| error.to_string())
}

/// The index now, for a stash clash's tie (ADR-0045 §2, amended
/// 2026-09-25): `mergewright_ops::snapshot::index_state`, the recorder's own read,
/// in the journal's directory as `apply` uses it, so an unchanged index is
/// the same id the journal holds.
///
/// # Errors
/// The failure text, ready to show: the repository or the journal's
/// directory could not be found, or the index copy or `write-tree` failed.
pub(super) fn read_index_state(
    repos: &RepoStore,
    cancel: &Cancel,
) -> Result<mergewright_core::journal::IndexState, String> {
    let repo = repos.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    let journal =
        mergewright_ops::journal::journal_path(&handle).map_err(|error| error.to_string())?;
    let scratch = journal
        .parent()
        .ok_or_else(|| "the journal has no directory to read the index in".to_owned())?;
    mergewright_ops::snapshot::index_state(&handle, scratch, cancel)
        .map_err(|error| error.to_string())
}
