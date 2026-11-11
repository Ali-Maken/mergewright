//! `mergewright-graph`'s integration tests, in one binary.
//!
//! PROMPT.md §7.1: one test binary per crate, not one per file. A new area
//! gets a module here, not a new `tests/*.rs`.

mod corpus;
mod corpus_seeds;
mod emphasis_layer;
mod lane_slots;
mod node_shots;
mod raster;
mod raster_continuation;
mod raster_pieces;
mod raster_pixels;
mod raster_roles;
mod streaming;
mod strip_cache;
