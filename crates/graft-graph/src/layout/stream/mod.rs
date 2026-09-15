//! The streaming layout: DESIGN.md §3.6 as one type.
//!
//! A [`StreamingLayout`] pulls commits from a [`CommitSource`] in chunks,
//! lays them out with a [`LayoutCursor`], keeps the rows around the viewport
//! resident and evicts the rest, checkpoints the cursor every so many rows,
//! and rebuilds evicted rows from the nearest checkpoint when the viewport
//! comes back. Nothing in it blocks on the whole history: every call takes a
//! budget or works on a range, and the caller decides when to call again.
//!
//! Two ways to start. [`StreamingLayout::new`] streams a walk as it comes.
//! [`StreamingLayout::from_order`] takes a walk order already known in full
//! — the product of ADR-0008's background topological walk — and fetches
//! links by handle as it goes: the promoted view. From then on the two are
//! the same type doing the same work; the only trace of the promotion is
//! that the order runs ahead of the frontier.

use std::ops::Range;

use graft_core::oid::Oid;

#[cfg(doc)]
use crate::layout::source::CommitSource;

use crate::layout::LayoutCursor;
use crate::layout::cache::RowCache;
use crate::layout::checkpoint::Checkpoints;
use crate::layout::order::{EmittedOrder, RepeatedCommit};
use crate::row::AssembleError;

/// The knobs of DESIGN.md §3.6, with its starting values as defaults.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StreamConfig {
    /// Rows per cache chunk: the unit of eviction, and the most one pull
    /// from the source asks for.
    pub chunk_rows: usize,
    /// Rows between checkpoints. A multiple of `chunk_rows`, so a checkpoint
    /// always falls on a chunk boundary.
    pub checkpoint_every: usize,
    /// How many checkpoints to keep on the grid before the stride doubles
    /// (m1-the-graph/60: an even grid, never least-recently-used).
    pub max_checkpoints: usize,
    /// How many chunks may stay resident outside the protected window.
    pub resident_chunks: usize,
    /// Rows above and below the viewport that stay resident with it.
    pub buffer_rows: usize,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            chunk_rows: 500,
            checkpoint_every: 2_000,
            max_checkpoints: 64,
            resident_chunks: 16,
            buffer_rows: 1_000,
        }
    }
}

/// Why a streaming layout could not go on.
#[derive(Debug, thiserror::Error)]
pub enum StreamError<E: std::error::Error + 'static> {
    /// The source failed.
    #[error("reading history")]
    Source(#[source] E),
    /// The walk gave the same commit twice.
    #[error(transparent)]
    Repeated(#[from] RepeatedCommit),
    /// The source returned no commits without saying the walk was over or
    /// stopped — a pull that could be retried forever, so it is not.
    #[error("the walk returned nothing, but reported neither finished nor stopped")]
    Stalled,
    /// A read or re-read by handle came back short.
    #[error("asked for {asked} commits by handle, the source returned {got}")]
    ShortRead {
        /// How many ids were asked for.
        asked: usize,
        /// How many commits came back.
        got: usize,
    },
    /// A read or re-read by handle came back with a different commit.
    #[error("asked for commit {asked} by handle, the source returned {got}")]
    WrongCommit {
        /// The id asked for.
        asked: Oid,
        /// The id that came back in its place.
        got: Oid,
    },
    /// A rebuilt chunk did not reproduce the rows that were evicted.
    #[error("rows {start}..{end} did not rebuild identically to the copy that was evicted")]
    RebuildDiffers {
        /// The first row of the chunk.
        start: usize,
        /// One past the last row the evicted copy held.
        end: usize,
    },
    /// A shape could not become a row.
    #[error(transparent)]
    Assemble(#[from] AssembleError),
}

/// What one [`StreamingLayout::truncate`] threw away: nothing outside it
/// was touched.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Invalidated {
    /// The first row laid out again; every row before it is kept.
    pub from_row: usize,
    /// Chunk indices dropped or cut back (held or evicted), ascending.
    pub chunks: Vec<usize>,
    /// Rows of the checkpoints dropped, ascending.
    pub checkpoints: Vec<usize>,
}

/// What one [`StreamingLayout::advance`] call did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Advance {
    /// Rows laid out by this call.
    pub rows_emitted: usize,
    /// Whether the whole walk is now laid out.
    pub done: bool,
    /// Whether the call gave up early because `stop` asked it to.
    pub stopped: bool,
}

/// A place in the graph that survives the rows moving under it: a commit,
/// and how far down the viewport it sat (DESIGN.md §3.2, ADR-0008 — "never
/// by row index").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Anchor {
    /// The selected commit.
    pub commit: Oid,
    /// How many rows below the top of the viewport it was.
    pub offset: usize,
}

/// What the layout is holding right now — the numbers a memory budget is
/// checked against (PROMPT.md §7.1, ticket m1-16).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Resident {
    /// Rows held in the cache.
    pub rows: usize,
    /// Chunks held in the cache.
    pub chunks: usize,
    /// Checkpoints held.
    pub checkpoints: usize,
    /// Approximate bytes across every checkpoint.
    pub checkpoint_bytes: usize,
    /// Approximate bytes of the frontier cursor: what one more checkpoint
    /// would cost.
    pub frontier_bytes: usize,
    /// Approximate bytes of the emitted order (ids and their rows).
    pub order_bytes: usize,
    /// Rows laid out whose commits were never read: chunks that passed
    /// through outside the viewport's window and have not been asked for.
    pub passed_through: usize,
}

/// How much work the layout has done, for tests and benchmarks that want to
/// know a rebuild replayed from a checkpoint rather than from the top.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Work {
    /// Links pulled from the source's walk.
    pub commits_pulled: usize,
    /// Links re-served from the source by handle: a replay, or an order
    /// already known.
    pub commits_reread: usize,
    /// Rows laid out again while rebuilding evicted chunks.
    pub rows_replayed: usize,
    /// Evicted chunks rebuilt.
    pub chunks_rebuilt: usize,
    /// Whole commits read to assemble rows.
    pub hydrated: usize,
    /// Commits read only to name a merge lane opened above their batch.
    pub merge_reads: usize,
    /// Links the source served, pulled from the walk or re-served by handle.
    pub links_read: usize,
}

/// A graph laid out a chunk at a time, with a bounded resident set.
#[derive(Debug)]
pub struct StreamingLayout<S> {
    source: S,
    config: StreamConfig,
    /// The cursor as it stood before the first row: what a rebuild with no
    /// earlier checkpoint replays from.
    base: LayoutCursor,
    /// The cursor at the furthest row laid out.
    frontier: LayoutCursor,
    order: EmittedOrder,
    cache: RowCache,
    checkpoints: Checkpoints,
    viewport: Range<usize>,
    /// The walk's length when it was known before the first row was laid
    /// out — on a layout built from a finished order.
    known_total: Option<usize>,
    done: bool,
    work: Work,
}

mod advance;
mod introspect;
mod materialise;
mod viewport;

#[cfg(test)]
mod tests;
