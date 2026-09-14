//! `graft-git`'s integration tests, in one binary.
//!
//! PROMPT.md §7.1: one test binary per crate, not one per file. A new area
//! gets a module here, not a new `tests/*.rs`.

mod commit_graph;
mod corpus;
