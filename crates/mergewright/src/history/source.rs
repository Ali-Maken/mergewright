//! The product's [`CommitSource`]: `mergewright-git`'s walk behind `mergewright-graph`'s
//! seam.

use std::sync::Arc;

use mergewright_core::commit::Commit;
use mergewright_core::commit_handle::{CommitHandle, Link};
use mergewright_core::oid::Oid;
use mergewright_git::history::{CommitTable, Walk, WalkError, read_commit};
use mergewright_git::repo::RepoHandle;
use mergewright_graph::layout::{CommitSource, Pull};

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
    /// out a chunk at a time by reading the commits at the next handles.
    Order {
        order: Arc<[CommitHandle]>,
        next: usize,
    },
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

    /// A source that hands out `order` a handle at a time, reading each
    /// commit from `handle` — the promoted topological order.
    ///
    /// `table` must already have every handle in `order` interned (it is the
    /// table the order was computed against): a fresh, unrelated table would
    /// leave `handle_of`/`id_of` unable to resolve them.
    ///
    /// A stream rather than `StreamingLayout::from_order`, so that
    /// `mergewright_graph::Included` can still decide what each view lets through:
    /// the one order serves every view.
    #[must_use]
    pub fn in_order(
        handle: &'a RepoHandle,
        table: Arc<CommitTable>,
        order: Arc<[CommitHandle]>,
    ) -> Self {
        WalkSource {
            handle,
            feed: Feed::Order { order, next: 0 },
            table,
        }
    }
}

/// `commit`'s link, parents read from its object and interned in `table`.
fn link_of(
    handle: &RepoHandle,
    table: &CommitTable,
    commit: CommitHandle,
) -> Result<Link, WalkError> {
    let read = read_commit(handle, table.id_of(commit))?;
    Ok(Link {
        commit,
        parents: read
            .parents
            .iter()
            .map(|parent| table.intern(parent))
            .collect(),
    })
}

impl CommitSource for WalkSource<'_> {
    type Error = WalkError;

    fn pull(&mut self, limit: usize, stop: &dyn Fn() -> bool) -> Result<Pull, WalkError> {
        match &mut self.feed {
            Feed::Walk(walk) => {
                let chunk = walk.next_links(limit, stop)?;
                Ok(Pull {
                    links: chunk.links,
                    done: chunk.done,
                    stopped: chunk.stopped,
                })
            }
            Feed::Order { order, next } => {
                let mut links = Vec::new();
                while links.len() < limit && *next < order.len() {
                    if stop() {
                        return Ok(Pull {
                            links,
                            done: false,
                            stopped: true,
                        });
                    }
                    links.push(link_of(self.handle, &self.table, order[*next])?);
                    *next += 1;
                }
                Ok(Pull {
                    links,
                    done: *next == order.len(),
                    stopped: false,
                })
            }
        }
    }

    fn read(&mut self, commits: &[CommitHandle]) -> Result<Vec<Commit>, WalkError> {
        commits
            .iter()
            .map(|&commit| read_commit(self.handle, self.table.id_of(commit)))
            .collect()
    }

    // Reads each commit whole for its parents: a promoted order has no walk
    // to take them from. A batched or commit-graph read is 52b's to measure.
    fn links(&mut self, commits: &[CommitHandle]) -> Result<Vec<Link>, WalkError> {
        commits
            .iter()
            .map(|&commit| link_of(self.handle, &self.table, commit))
            .collect()
    }

    fn handle_of(&self, id: &Oid) -> Option<CommitHandle> {
        self.table.handle_of(id)
    }

    fn id_of(&self, handle: CommitHandle) -> Oid {
        self.table.id_of(handle)
    }
}
