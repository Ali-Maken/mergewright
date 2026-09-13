//! The product's [`CommitSource`]: `graft-git`'s walk behind `graft-graph`'s
//! seam.

use graft_core::commit::Commit;
use graft_core::oid::Oid;
use graft_git::history::{Walk, WalkError, read_commit};
use graft_git::repo::RepoHandle;
use graft_graph::layout::{CommitSource, Pull};

/// A [`Walk`] the layout pulls chunks from, re-reading commits by id through
/// [`read_commit`] when it rebuilds evicted rows or follows a promoted order.
///
/// Borrows the handle rather than owning it: a `RepoHandle` is not `Send`,
/// so the handle, the walk and the layout all live on the one worker thread
/// that runs the job.
pub struct WalkSource<'a> {
    handle: &'a RepoHandle,
    walk: Walk<'a>,
}

impl<'a> WalkSource<'a> {
    /// A source that streams `walk` and re-reads from `handle`.
    #[must_use]
    pub fn new(handle: &'a RepoHandle, walk: Walk<'a>) -> Self {
        WalkSource { handle, walk }
    }
}

impl CommitSource for WalkSource<'_> {
    type Error = WalkError;

    fn next(&mut self, limit: usize, stop: &dyn Fn() -> bool) -> Result<Pull, WalkError> {
        let chunk = self.walk.next_chunk(limit, stop)?;
        Ok(Pull {
            commits: chunk.commits,
            done: chunk.done,
            stopped: chunk.stopped,
        })
    }

    fn read(&mut self, ids: &[Oid]) -> Result<Vec<Commit>, WalkError> {
        ids.iter().map(|&id| read_commit(self.handle, id)).collect()
    }
}
