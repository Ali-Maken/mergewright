//! Reading how dirty the working tree is: staged, unstaged and untracked
//! counts, for the uncommitted-changes row (PROMPT.md §1.3).
//!
//! Read by a job, never by `update`: `graft_git::status::counts` walks the
//! index and the worktree, which touches disk. No cache — unlike refs'
//! ahead/behind counts, there is nothing here worth remembering between
//! reads.

use graft_git::status::WorkingTreeCounts;

use crate::repos::RepoStore;

/// Reads the working tree's staged/unstaged/untracked counts (a job's work).
///
/// # Errors
/// The failure text, ready to show: the repository or its status could not
/// be read.
pub fn read(store: &RepoStore) -> Result<WorkingTreeCounts, String> {
    let repo = store.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    graft_git::status::counts(&handle).map_err(|error| error.to_string())
}
