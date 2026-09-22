//! The M5 acceptance walk (m5-history-and-ghost-preview/11): one scenario
//! per promise the milestone makes — *you see the result before you
//! confirm* — each against a scratch copy of a corpus shape with a history
//! scripted by git, the real app driven by keys and mouse over the real job
//! pool and real `git`, and then **repository truth read back by running
//! git** — never from `AppState`. The one thing read from `AppState` is the
//! claim: the plan behind the preview box, so [`claim::ghost_equals_git`]
//! can say the ghost equals what git did. `examples/m5_walk.rs` includes
//! this module by `#[path]` and prints one line per scenario;
//! [`every_scenario`] runs the same list under `cargo test`, one test per
//! scenario.
//!
//! Same shape as `m4_walk`: [`Step`]s that press, wait, mark and shell out,
//! and a [`Capture`] naming the keys a pty run replays as the harness
//! prelude. What is new: the scenario's terminal size, the cursor and menu
//! steps, a mouse drag, the claim, and a relaunch mid-conflict.
#![allow(
    clippy::expect_used,
    reason = "a walk step that cannot proceed has nothing left to prove; the panic names why"
)]

mod claim;
mod conflicts;
#[cfg(test)]
mod every_scenario;
mod fixture;
mod gestures;
mod merge_rebase;
mod pick_revert_reset;
mod preview;
mod radar_fixes;
mod refs_and_names;
mod remotes;
mod run;
mod steps;
mod time_machine;
mod truth;
mod vocabulary;

use graft_core::event::KeyPress;

pub use run::{prepare_capture, run_scenario};
pub use steps::Scenario;
use steps::{Capture, When};

use crate::repo_truth::keys_label;

/// Every scenario, in the order `TESTING.md` lists them.
#[must_use]
pub fn scenarios() -> Vec<Scenario> {
    let mut all = preview::previewing();
    all.extend(merge_rebase::merging());
    all.extend(merge_rebase::rebasing());
    all.extend(pick_revert_reset::picking_reverting_resetting());
    all.extend(radar_fixes::fixing_a_foxtrot());
    all.extend(conflicts::conflicting_merges());
    all.extend(time_machine::time_travelling());
    all.extend(remotes::remote_history());
    all.extend(refs_and_names::refs_and_names());
    all
}

/// The `--stems` line for a capture: stem, scenario, expected text, key
/// label, when — tab-separated.
#[must_use]
pub fn stems_line(scenario: &Scenario, capture: &Capture) -> String {
    format!(
        "{}\t{}\t{}\t{}\t{}",
        capture.stem,
        scenario.name,
        capture.on_screen,
        keys_label(&capture.keys),
        match capture.when {
            When::Settled => "settled",
            When::Appears => "appears",
        }
    )
}

/// The prelude a `--capture` run feeds.
#[must_use]
pub fn capture_prelude(capture: &Capture) -> Vec<KeyPress> {
    capture.keys.clone()
}
