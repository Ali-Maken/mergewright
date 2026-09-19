//! Reading the working tree's full status — staged, unstaged and untracked,
//! one entry per changed path — for the uncommitted-changes row and the
//! changes pane alike (PROMPT.md §1.3, m3-working-tree-undo-and-stash/04).
//!
//! Read by a job, never by `update`: `graft_git::status::read` walks the
//! index and the worktree, which touches disk. No cache: unlike refs'
//! ahead/behind counts, there is nothing here worth remembering between
//! reads.

use std::sync::Arc;

use graft_git::run::Stop;
use graft_git::state::InProgress;
use graft_git::status::WorkingTreeStatus;

use crate::repos::RepoStore;

/// One read of the working tree: what changed, and whether the repository is
/// partway through a merge, rebase, cherry-pick, revert or bisect.
///
/// The two travel together because they are read together and used together:
/// the commit editor's refusal preview (ADR-0034) needs both to say whether
/// `[ Commit ]` would do anything, and `in_progress` is a plain filesystem
/// check under the git directory — no extra `git` spawn to piggyback it on
/// the status read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkingTree {
    /// One entry per changed path. `Arc`: shared with the changes pane's
    /// file list rather than copied per event.
    pub status: Arc<WorkingTreeStatus>,
    /// The operation partway through, if there is one.
    pub in_progress: Option<InProgress>,
}

/// Reads the working tree's status and the repository's in-progress state
/// (a job's work).
///
/// # Errors
/// The failure text, ready to show: the repository, its status or its
/// in-progress markers could not be read.
pub fn read(store: &RepoStore, stop: &dyn Stop) -> Result<WorkingTree, String> {
    let repo = store.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    let status = graft_git::status::read(&handle, stop).map_err(|error| error.to_string())?;
    // Not swallowed to `None`: an unreadable git directory is a real failure,
    // and reporting "nothing in progress" would let a commit be offered that
    // git will refuse.
    let in_progress = graft_git::state::in_progress(&handle).map_err(|error| error.to_string())?;
    Ok(WorkingTree {
        status: Arc::new(status),
        in_progress,
    })
}
