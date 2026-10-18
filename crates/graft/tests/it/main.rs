//! The workspace's single integration-test binary.
//!
//! Every file directly under `tests/` becomes its own binary to compile and
//! link, which is slow and memory-hungry, so all integration tests live in
//! `tests/it/` and are included from here as modules (PROMPT.md 7.1). Add a
//! test by writing `tests/it/<area>.rs` and a `mod <area>;` line below.
//!
//! No `unsafe` here (m8/13l): ADR-0015 keeps the workspace's `unsafe` to two
//! files to review, and a test that needs a syscall uses a safe wrapper
//! (`quit_waits::pty_session` re-execs itself rather than use `pre_exec`).

#![forbid(unsafe_code)]

mod app_harness;
mod askpass_mode;
mod blame_partly_staged;
mod blame_place;
mod box_owns_the_bar;
#[cfg(feature = "heavy-tests")]
mod budget_run;
mod captures;
mod changes_pane_writes;
mod clash_stops;
mod cli_docs;
mod clock_pin;
mod commit_editor_writes;
mod commit_screen;
mod conflict_interim;
mod context_menu;
mod coverage;
mod credential_by_key;
mod daily_landing;
/// Shared step deadline: turns a hang into a named failure. Used by
/// `app_harness` and `budget_run`.
mod deadline;
mod dependency_edges;
mod diff_cursor;
mod drain_sink;
mod esc_and_the_walk;
mod file_size;
mod finish;
mod first_launch;
mod fold_by_default;
mod fold_tail_narrow;
mod frame_trace;
#[cfg(feature = "heavy-tests")]
mod frames;
mod gallery;
/// m7-power-tools/47: `docs/shots/m7/fix-scroll-ghostty.png`, on the team fixture.
#[cfg(feature = "heavy-tests")]
mod ghostty_shot;
#[cfg(feature = "heavy-tests")]
mod heavy;
#[cfg(feature = "heavy-tests")]
mod held_key;
/// Fast tier: only the pure after-release bound (`held_key_bound.rs`).
#[cfg(not(feature = "heavy-tests"))]
#[path = "held_key_bound.rs"]
mod held_key;
mod hint_bar;
mod history_place;
mod idle_advance;
mod in_the_way_status;
mod journal_put_back;
mod journal_screen;
mod keymap_config;
mod land_on_result;
mod lane_cap;
mod look_chrome;
mod look_polish;
mod m3_walk;
mod m4_walk;
mod m5_walk;
mod m6_stash_put_back;
mod m7_walk;
mod m8_walk;
#[cfg(feature = "heavy-tests")]
mod many_refs;
mod map_coverage;
mod merge_one_file;
mod name_and_drag;
mod not_a_repo;
mod offer_and_blame;
mod outside_repo;
mod palette_find;
mod palette_in_progress;
mod pane_bar_space;
mod partial_fetch;
/// m8 05 Part 2: the gate before wiring emphasis pieces.
#[cfg(feature = "heavy-tests")]
mod piece_gate;
/// m8 05 Part 2: `docs/shots/m8-05-emphasis-pieces.png`, on the team fixture.
#[cfg(feature = "heavy-tests")]
mod pieces_shot;
mod piped_stdout;
mod preview;
mod pty;
mod published_repo;
mod pull_over_edits;
mod put_back_box;
mod quit_waits;
mod radar_bookkeeping;
mod radar_not_checked;
mod rebase_stop_forecast;
mod refresh_race;
mod release_names;
mod remote_by_key;
mod rename_docs;
mod rename_names;
mod rename_opens;
mod rename_status;
mod repo_truth;
mod resolver;
mod rewrite_stop;
mod seam_rule;
mod search_hint;
mod stash_by_key;
mod stash_clash;
mod stash_in_history;
mod stash_nodes;
mod status_under_a_box;
mod stop_ladder_readers;
mod submodule_truth;
mod tour;
mod tour_tasks;
mod tour_walk;
mod two_boxes_over_content;
mod undo_by_key;
mod undo_over_drift;
