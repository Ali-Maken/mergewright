//! The stash list read, behind the `S` overlay
//! (m3-working-tree-undo-and-stash/06).
//!
//! `graft_git::stash::list` is the whole of it: the reflog of `refs/stash`
//! is the stash list (`as-built.md`, group 02), and one `diff-tree` per
//! entry gives the paths the row counts. Off the loop's thread, like every
//! other read.

use std::sync::Arc;

use graft_git::stash::StashEntry;

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
    graft_git::stash::list(&handle, cancel)
        .map(Arc::new)
        .map_err(|error| error.to_string())
}
