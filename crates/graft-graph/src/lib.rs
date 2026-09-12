//! The commit graph: the pure layout engine that turns a rev-walk into lanes
//! and rows, and the pixel, Unicode and ASCII renderers that draw them. Layout
//! is data in, data out — no terminal, no filesystem, no clock — so it can be
//! tested and benchmarked on its own. The pixel renderer's output is a
//! `RasterFrame`; encoding that into a terminal image protocol is `term`'s job,
//! deliberately a different crate. Depends only on `core`; it never calls into
//! `git`, `ops` or `term`.

#![forbid(unsafe_code)]

pub mod identity;
pub mod lane;
pub mod layout;
pub mod merge_name;
pub mod row;
pub mod seeds;

pub use identity::BranchIdentity;
pub use lane::Lane;
pub use layout::{EmittedOrder, LayoutCursor, layout};
pub use merge_name::recover_branch_name;
pub use row::Row;
pub use seeds::{BranchSeed, Seeds};
