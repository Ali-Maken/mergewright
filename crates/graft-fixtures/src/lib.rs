//! The fixture corpus: one named repository per history or ref shape, built
//! from a script and shared by every test in the workspace.
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
//! [`Shape::NonUtf8Names`] needs a filesystem that accepts a file name which
//! is not valid UTF-8. APFS does not. [`try_borrow`] returns
//! [`CorpusError::Unavailable`] there, carrying a reason that names the
//! filesystem; a test for that shape reports the skip rather than passing
//! quietly.

#![forbid(unsafe_code)]

mod corpus;
mod digest;
mod filesystem;
mod git;
mod scratch;
mod shape;
mod shapes;
mod stream;

use std::path::PathBuf;

pub use corpus::{
    CorpusError, Report, Unavailable, build_into, ensure, try_borrow, try_copy_into_temp,
};
pub use scratch::Scratch;
pub use shape::Shape;

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
