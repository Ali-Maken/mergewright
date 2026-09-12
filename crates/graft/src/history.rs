//! Reading history for the screen: the walk, run as a job.
//!
//! The walk itself lives in `graft-git` and knows nothing about jobs, states
//! or screens. This is the thin layer between the two: it borrows an already
//! open repository from the [`RepoStore`], drains the walk a chunk at a time,
//! checking after each chunk whether the user has pressed `Esc`, and hands
//! back one plain value the state can keep.
//!
//! Chunks are drained here rather than delivered one by one because a job
//! reports exactly once (`crate::jobs`). The chunk size still matters: it is
//! how often cancellation is noticed, and it is the unit the layout engine
//! will consume when incremental painting arrives with the promotion ticket
//! (ADR-0008, KNOWN_GAPS.md).

use std::sync::Arc;

use graft_core::commit::Commit;
use graft_core::ref_name::RefName;
use graft_git::history::{Order, Walk};

use crate::repos::RepoStore;

/// The commits one walk brought back, and what the user must be told about
/// the order they are in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct History {
    /// The commits, in [`History::order`]. Shared rather than cloned: the
    /// state is copied on every event and a hundred thousand commits must not
    /// be copied with it.
    pub commits: Arc<[Commit]>,
    /// The order that was asked for.
    pub requested: Order,
    /// The order they actually came back in. Different from
    /// [`History::requested`] when there is no commit-graph file and ADR-0008
    /// fell back to commit date.
    pub order: Order,
    /// Which branch owns lane 0 (ADR-0001), or [`None`] in a repository with
    /// no branches.
    pub default_branch: Option<RefName>,
    /// The branches that sit beside lane 0 and never compete for it.
    pub integration: Arc<[RefName]>,
}

impl History {
    /// Is this an order the user should be told is not the final one?
    #[must_use]
    pub fn is_provisional(&self) -> bool {
        self.requested != self.order
    }
}

/// What one walk did.
pub enum Read {
    /// It finished, and this is the history.
    Done(Box<History>),
    /// The user cancelled it. Whatever had been read is dropped here rather
    /// than travelling to a screen that no longer wants it.
    Cancelled,
}

/// Walk `store`'s repository in `order`, stopping when `stop` says to.
///
/// # Errors
/// The failure text, ready to show: the repository could not be opened, or a
/// commit a ref points at could not be read. Formed here because it crosses a
/// thread as part of a job result and there is one thing every caller does
/// with it.
pub fn read(store: &RepoStore, order: Order, stop: &dyn Fn() -> bool) -> Result<Read, String> {
    let branches = store.branches().map_err(|error| error.to_string())?;
    let repo = store.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();

    let mut walk = Walk::all_refs(&handle, order).map_err(|error| error.to_string())?;
    let mut commits = Vec::new();
    loop {
        let chunk = walk
            .next_chunk(graft_git::history::CHUNK, stop)
            .map_err(|error| error.to_string())?;
        commits.extend(chunk.commits);
        if chunk.stopped {
            return Ok(Read::Cancelled);
        }
        if chunk.done {
            break;
        }
    }

    Ok(Read::Done(Box::new(History {
        commits: commits.into(),
        requested: walk.requested_order(),
        order: walk.order_in_force(),
        default_branch: branches.default_branch().cloned(),
        integration: branches.integration().to_vec().into(),
    })))
}
