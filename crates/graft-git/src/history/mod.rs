//! Reading history out of a repository, in order, a chunk at a time.
//!
//! The layout engine is pure: it takes commits in and produces rows. This is
//! the other half — the thing that produces the commits, in the order the
//! graph is to be drawn in, without reading the whole repository first and
//! without blocking on a walk nobody is waiting for any more.
//!
//! Three orders, one interface ([`Order`]):
//!
//! * [`Order::Topological`] is the one the graph is finally drawn in. It
//!   needs the commit-graph file's generation numbers to emit incrementally;
//!   see [`topological`].
//! * [`Order::CommitDate`] is the natural streaming order and needs nothing
//!   at all. ADR-0008 makes it the first paint when there is no commit-graph
//!   file, with the UI saying so.
//! * [`Order::FirstParent`] follows only each commit's first parent: the
//!   branch's own trunk.
//!
//! Every order breaks a tie the same way — commit date descending, then
//! commit id ascending (DESIGN.md §3.2) — so the same repository draws the
//! same graph twice, which matters because scripted history and
//! `fast-import` produce identical timestamps constantly.
//!
//! A walk is not an iterator. [`Walk::next_chunk`] hands back a bounded
//! batch and keeps its position, so the caller can deliver a chunk to the
//! screen, check whether anyone still wants the rest, and come back for
//! more.

mod divergence;
mod node;
mod read;
mod simple;
mod sorted;
mod table;
mod topological;
mod walk;

pub use divergence::{AheadBehind, ahead_behind};
pub use node::NodeError;
pub use read::{
    ReflogError, WalkError, first_parent_chain, first_parent_merges, previous_tips, read_commit,
};
pub use table::{CommitTable, TableError};
pub use walk::{CHUNK, Chunk, LinkChunk, Order, Walk};

#[cfg(test)]
mod tests;
