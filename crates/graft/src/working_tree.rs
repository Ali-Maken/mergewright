//! Reading the working tree's full status — staged, unstaged and untracked,
//! one entry per changed path — for the uncommitted-changes row and the
//! changes pane alike (PROMPT.md §1.3, m3-working-tree-undo-and-stash/04).
//!
//! Read by a job, never by `update`: `graft_git::status::read` walks the
//! index and the worktree, which touches disk. No cache: unlike refs'
//! ahead/behind counts, there is nothing here worth remembering between
//! reads.

use graft_git::run::Stop;
use graft_git::status::WorkingTreeStatus;

use crate::repos::RepoStore;

/// Reads the working tree's status (a job's work).
///
/// # Errors
/// The failure text, ready to show: the repository or its status could not
/// be read.
pub fn read(store: &RepoStore, stop: &dyn Stop) -> Result<WorkingTreeStatus, String> {
    let repo = store.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    graft_git::status::read(&handle, stop).map_err(|error| error.to_string())
}
