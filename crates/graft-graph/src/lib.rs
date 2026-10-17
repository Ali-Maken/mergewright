//! The commit graph: the pure layout engine that turns a rev-walk into lanes
//! and rows, and the pixel, Unicode and ASCII renderers that draw them. Layout
//! is data in, data out — no terminal, no filesystem, no clock — so it can be
//! tested and benchmarked on its own. The pixel renderer's output is a
//! `RasterFrame`; encoding that into a terminal image protocol is `term`'s job,
//! deliberately a different crate. Depends only on `core`; it never calls into
//! `git`, `ops` or `term`.

#![forbid(unsafe_code)]

pub mod ancestry;
pub mod fold;
pub mod identity;
pub mod inclusion;
pub mod lane;
pub mod layout;
pub mod merge_name;
pub mod raster;
pub mod role;
pub mod row;
pub mod seeds;

pub use ancestry::{Ancestry, Kin};
pub use fold::Fold;
pub use identity::BranchIdentity;
pub use inclusion::{Follow, Included, Inclusion};
pub use lane::Lane;
pub use layout::{
    Advance, Anchor, CommitSource, EmittedOrder, Invalidated, LayoutCursor, Pull, Resident,
    StreamConfig, StreamError, StreamingLayout, Work, layout,
};
pub use merge_name::recover_branch_name;
pub use role::{LaneRole, Route, made_on};
pub use row::Row;
pub use seeds::{BranchSeed, Seeds};
