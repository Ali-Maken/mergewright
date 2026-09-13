//! The product's [`CommitSource`]: `graft-git`'s walk behind `graft-graph`'s
//! seam.

use std::sync::Arc;

use graft_core::commit::Commit;
use graft_core::commit_handle::{CommitHandle, Link};
use graft_core::oid::Oid;
use graft_git::history::{CommitTable, Walk, WalkError, read_commit};
use graft_git::repo::RepoHandle;
use graft_graph::layout::{CommitSource, Pull};

/// A [`Walk`] the layout pulls chunks from — or, once promoted, an order
/// already known in full — re-reading commits by id through [`read_commit`]
/// when it rebuilds evicted rows.
///
/// Borrows the handle rather than owning it: a `RepoHandle` is not `Send`,
/// so the handle, the walk and the layout all live on the one worker thread
/// that runs the job.
pub struct WalkSource<'a> {
    handle: &'a RepoHandle,
    feed: Feed<'a>,
    /// The repository's id table: handles in, ids out.
    table: Arc<CommitTable>,
}

/// Where the next commits come from.
enum Feed<'a> {
    /// A walk still in progress.
    /// Boxed: a walk is far larger than an order's cursor.
    Walk(Box<Walk<'a>>),
    /// An order worked out in full elsewhere (ADR-0008 promotion): handed
    /// out a chunk at a time by reading the commits at the next ids.
    Order { ids: Arc<[Oid]>, next: usize },
}

impl<'a> WalkSource<'a> {
    /// A source that streams `walk` and re-reads from `handle`, its handles
    /// from the walk's own table — taken from the walk rather than passed
    /// beside it, so the two cannot disagree.
    #[must_use]
    pub fn new(handle: &'a RepoHandle, walk: Walk<'a>) -> Self {
        WalkSource {
            handle,
            table: Arc::clone(walk.table()),
            feed: Feed::Walk(Box::new(walk)),
        }
    }

    /// A source that hands out `ids` in order, reading each commit from
    /// `handle` — the promoted topological order.
    ///
    /// A stream rather than `StreamingLayout::from_order`, so that
    /// `graft_graph::Included` can still decide what each view lets through:
    /// the one order serves every view.
    #[must_use]
    pub fn in_order(handle: &'a RepoHandle, ids: Arc<[Oid]>, table: Arc<CommitTable>) -> Self {
        WalkSource {
            handle,
            feed: Feed::Order { ids, next: 0 },
            table,
        }
    }
}

impl CommitSource for WalkSource<'_> {
    type Error = WalkError;

    fn next(&mut self, limit: usize, stop: &dyn Fn() -> bool) -> Result<Pull, WalkError> {
        match &mut self.feed {
            Feed::Walk(walk) => {
                let chunk = walk.next_chunk(limit, stop)?;
                Ok(Pull {
                    commits: chunk.commits,
                    done: chunk.done,
                    stopped: chunk.stopped,
                })
            }
            Feed::Order { ids, next } => {
                let mut commits = Vec::new();
                while commits.len() < limit && *next < ids.len() {
                    if stop() {
                        return Ok(Pull {
                            commits,
                            done: false,
                            stopped: true,
                        });
                    }
                    commits.push(read_commit(self.handle, ids[*next])?);
                    *next += 1;
                }
                Ok(Pull {
                    commits,
                    done: *next == ids.len(),
                    stopped: false,
                })
            }
        }
    }

    fn read(&mut self, ids: &[Oid]) -> Result<Vec<Commit>, WalkError> {
        ids.iter().map(|&id| read_commit(self.handle, id)).collect()
    }

    // Reads each commit whole for its parents: `pull` and `read` still hand
    // out whole commits, so nothing calls this on a hot path yet. The
    // hydrate-on-demand ticket reads parents from the walk instead.
    fn links(&mut self, commits: &[CommitHandle]) -> Result<Vec<Link>, WalkError> {
        commits
            .iter()
            .map(|&commit| {
                let read = read_commit(self.handle, self.table.id_of(commit))?;
                Ok(Link {
                    commit,
                    parents: read
                        .parents
                        .iter()
                        .map(|parent| self.table.intern(parent))
                        .collect(),
                })
            })
            .collect()
    }

    fn handle_of(&self, id: &Oid) -> Option<CommitHandle> {
        self.table.handle_of(id)
    }

    fn id_of(&self, handle: CommitHandle) -> Oid {
        self.table.id_of(handle)
    }
}
