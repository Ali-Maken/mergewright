//! The workspace's single integration-test binary.
//!
//! Every file directly under `tests/` becomes its own binary to compile and
//! link, which is slow and memory-hungry, so all integration tests live in
//! `tests/it/` and are included from here as modules (PROMPT.md 7.1). Add a
//! test by writing `tests/it/<area>.rs` and a `mod <area>;` line below.

mod app_harness;
mod askpass_mode;
#[cfg(feature = "heavy-tests")]
mod budget_run;
mod captures;
mod changes_pane_writes;
mod commit_editor_writes;
mod conflict_interim;
mod context_menu;
mod coverage;
mod credential_by_key;
mod dependency_edges;
#[cfg(feature = "heavy-tests")]
mod frames;
mod gallery;
#[cfg(feature = "heavy-tests")]
mod heavy;
mod hint_bar;
mod lane_cap;
mod m3_walk;
mod m4_walk;
mod m5_walk;
#[cfg(feature = "heavy-tests")]
mod many_refs;
mod name_and_drag;
mod not_a_repo;
mod piped_stdout;
mod preview;
mod refresh_race;
mod remote_by_key;
mod repo_truth;
mod seam_rule;
mod stash_by_key;
mod two_boxes_over_content;
mod undo_by_key;
