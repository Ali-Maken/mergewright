//! The workspace's test fixtures, and the machinery the heavy tier needs.
//!
//! Two things live here. The **corpus** is one named repository per history
//! or ref shape, built from a script and shared by every test in the
//! workspace. The **heavy tier** is a synthetic repository of any size
//! ([`Scale`]), a peak-memory measurement ([`measure`]), and the proptest
//! case count ([`proptest_cases`]) — everything a budget test needs except
//! the budgets. It is all behind the `heavy-tests` feature, so a fast-tier
//! `cargo test` cannot compile it, let alone run it. `TESTING.md` is the
//! human-facing half of this.
//!
//! Nothing ships this crate. It is a dev-dependency everywhere it is used, so
//! it never appears in an internal dependency edge (ARCHITECTURE.md,
//! ADR-0006), and it deliberately depends on no other Graft crate — the
//! corpus can be built before anything that reads it exists.
//!
//! # Borrowing a fixture
//!
//! Read-only, which is what almost every test wants:
//!
//! ```no_run
//! let repo = graft_fixtures::borrow(graft_fixtures::Shape::OctopusMerge);
//! ```
//!
//! Writable, deleted when the returned value is dropped:
//!
//! ```no_run
//! let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Linear);
//! let repo = scratch.path();
//! ```
//!
//! The first call builds the corpus; later calls, in this process or the
//! next, find it already built. To build it by hand:
//!
//! ```text
//! cargo run -p graft-fixtures
//! ```
//!
//! # Shapes a machine cannot hold
//!
//! [`Shape::NonUtf8RefName`] needs a filesystem that accepts a file name
//! which is not valid UTF-8, because a loose ref is a file. APFS does not.
//! ([`Shape::NonUtf8Path`] has the same bytes in a path inside a tree object,
//! so it builds everywhere.) [`try_borrow`] returns
//! [`CorpusError::Unavailable`] there, carrying a reason that names the
//! filesystem; a test for that shape reports the skip rather than passing
//! quietly.
//!
//! # The heavy tier
//!
//! `cargo test` never sees it. The commands, the costs and the manual
//! checklist are in `TESTING.md`.

// `deny`, not `forbid`. The heavy tier's memory budget is measured with
// `getrusage` (PROMPT.md §7.1 names the call), which has no safe equivalent
// in the standard library; ADR-0015 records the one site, why it is sound,
// and what would break it. It carries a narrowly-scoped
// `#[allow(unsafe_code, reason = "...")]`, which `forbid` would turn into a
// compile error rather than a documented exception. This crate and
// `graft-term` are the only two that may do this, and only at the sites
// their ADRs name.
#![deny(unsafe_code)]

mod cache;
mod corpus;
mod digest;
#[cfg(unix)]
pub mod fifo;
mod filesystem;
pub mod git;
mod memory;
mod permissions;
pub mod remote;
mod scratch;
mod shape;
mod shapes;
mod stream;
#[cfg(feature = "heavy-tests")]
pub mod synthetic;
mod tier;

use std::path::PathBuf;

pub use cache::CacheError;
pub use corpus::{
    CorpusError, Report, Unavailable, build_into, ensure, try_borrow, try_copy_into_temp,
};
pub use memory::{Footprint, Measured, MemoryError, footprint, measure, peak_rss};
pub use scratch::Scratch;
pub use shape::Shape;
#[cfg(feature = "heavy-tests")]
pub use synthetic::{CommitGraph, Scale};
pub use tier::proptest_cases;

/// The instant every corpus snapshot and gallery stop is viewed at, Unix
/// seconds (m1-the-graph/53).
///
/// `2020-01-03T00:00:00Z`, two days after the corpus's own fixed clock
/// (`stream::EPOCH`, `2020-01-01T00:00:00Z`). The floor is
/// [`Shape::SkewedDates`]'s "fast clock" commit — the one commit in the
/// whole corpus dated *ahead* of its place in the stream, a day ahead of
/// its children — at `EPOCH + STEP + 1 day` (`stream.rs`'s own `STEP`,
/// 60s), the latest committer time any fixed-clock shape produces. Two
/// days clears it with a day to spare; nothing else in the corpus reaches
/// even one day past `EPOCH` on its own.
pub const CORPUS_NOW: i64 = 1_578_009_600;

/// The directory holding one fixture, ready to open read-only.
///
/// # Panics
///
/// If the corpus cannot be built, or if this machine cannot hold this shape.
/// A test that must survive the second case calls [`try_borrow`] instead.
pub fn borrow(shape: Shape) -> PathBuf {
    try_borrow(shape).unwrap_or_else(|why| panic!("{why}"))
}

/// A writable copy of one fixture, deleted when the returned value is
/// dropped.
///
/// # Panics
///
/// On the same terms as [`borrow`], plus a failure to copy.
pub fn copy_into_temp(shape: Shape) -> Scratch {
    try_copy_into_temp(shape).unwrap_or_else(|why| panic!("{why}"))
}
