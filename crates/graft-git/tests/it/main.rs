//! `graft-git`'s integration tests, in one binary.
//!
//! PROMPT.md §7.1: one test binary per crate, not one per file. A new area
//! gets a module here, not a new `tests/*.rs`.

mod blame;
mod commit_graph;
mod commits_from;
mod compare_and_stash;
mod corpus;
mod diff_cases;
mod diff_fixture;
mod equal_dates;
mod history_reads;
mod merge_forecast;
mod run;
mod session;
mod shallow_history;
mod snapshot_reads;
mod switch_refs;
mod trace;
mod trace_whole;
mod unreached;
