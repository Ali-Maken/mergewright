//! The workspace's single integration-test binary.
//!
//! Every file directly under `tests/` becomes its own binary to compile and
//! link, which is slow and memory-hungry, so all integration tests live in
//! `tests/it/` and are included from here as modules (PROMPT.md 7.1). Add a
//! test by writing `tests/it/<area>.rs` and a `mod <area>;` line below.

mod app_harness;
#[cfg(feature = "heavy-tests")]
mod budget_run;
mod changes_pane_writes;
mod commit_editor_writes;
mod dependency_edges;
#[cfg(feature = "heavy-tests")]
mod frames;
mod gallery;
#[cfg(feature = "heavy-tests")]
mod heavy;
#[cfg(feature = "heavy-tests")]
mod many_refs;
mod not_a_repo;
mod piped_stdout;
mod seam_rule;
mod undo_by_key;
