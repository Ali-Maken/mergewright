//! The M4 acceptance walk (m4-remotes/07): one scenario per promise the
//! milestone makes — *you never leave Graft to fetch, pull or push, not for
//! a password, a rejection or a hang* — each against a **real remote**: a
//! bare repository over `file://` ([`LocalRemote`]), smart HTTP with Basic
//! auth ([`HttpRemote`]) or a port that never answers ([`HungRemote`]). The
//! real app is driven by keys over the real job pool and real `git`, with
//! `graft` itself as git's askpass helper, and then **repository truth is
//! read back by running git on both the work clone and the bare** — never
//! from `AppState`. `examples/m4_walk.rs` includes this module by `#[path]`
//! and prints one line per scenario; [`every_scenario`] runs the same list
//! under `cargo test`, one test per scenario.
//!
//! Same shape as `m3_walk`: [`Step`]s that press, wait, mark and shell out,
//! and a [`Capture`] naming the keys a pty run replays as the harness
//! prelude. What is new: the fixture is a remote, [`Truth`] carries the bare's
//! refs, the journal, every status line the loop drew and the remote's own
//! counters, and every scenario ends by proving the walk's passwords reached
//! neither the journal on disk nor any event or status line the loop saw.
#![allow(
    clippy::expect_used,
    reason = "a walk step that cannot proceed has nothing left to prove; the panic names why"
)]

mod credentials;
#[cfg(test)]
mod every_scenario;
mod fetch_pull;
mod fixture;
mod hangs_and_undo;
mod push;
mod run;
mod steps;
mod truth;
mod vocabulary;

use graft_core::event::{KeyCode, KeyPress};

pub use run::{prepare_capture, run_scenario};
pub use steps::Scenario;
use steps::{Capture, When};

use crate::repo_truth::keys_label;

/// Every scenario, in the order `TESTING.md` lists them.
#[must_use]
pub fn scenarios() -> Vec<Scenario> {
    let mut all = fetch_pull::fetching();
    all.extend(fetch_pull::pulling());
    all.extend(push::pushing());
    all.extend(push::rejections());
    all.extend(credentials::credentials());
    all.extend(hangs_and_undo::hangs_progress_and_undo());
    all
}

/// The `--stems` line for a capture: stem, scenario, expected text, key
/// label, when — tab-separated, and never the secret.
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

/// The prelude a `--capture` run feeds: the keys, then the secret's
/// characters, when there is one.
#[must_use]
pub fn capture_prelude(capture: &Capture) -> Vec<KeyPress> {
    let mut keys = capture.keys.clone();
    if let Some(secret) = capture.secret {
        keys.extend(secret.chars().map(|c| KeyPress::plain(KeyCode::Char(c))));
    }
    keys
}
