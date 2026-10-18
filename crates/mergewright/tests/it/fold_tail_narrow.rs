//! A folded merge's tail short of room, through the real app (ease-pass
//! 06r m2): on `wallet_release` narrowed, the tail sheds ` from …`, then
//! its count, before it drops, two cells after the subject. The ladder
//! itself is mergewright-ui's `graph_screen::tests::fold_tail`; here a real
//! repository's row produces it, for the capture.
#![allow(
    clippy::expect_used,
    reason = "a fixture that cannot be built has nothing to test; the panic names why"
)]

use mergewright::session::Clock;
use mergewright::update::AppState;

use crate::app_harness::App;
use crate::captures::frame_at;
use crate::fold_by_default::wallet_release;

const ROWS: u16 = 36;
/// After every commit of `wallet_release`, so ages read the same.
const NOW: i64 = 1_700_100_000;
/// The PR's tail, whole.
const FULL: &str = "\u{25b8} 4 commits from PR 16021";
/// The PR's tail without ` from …`; only the PR's row draws `▸ 4`.
const COUNT: &str = "\u{25b8} 4 commits";

fn drawn(state: &AppState, columns: u16) -> Vec<String> {
    frame_at(state, columns, ROWS, Some(Clock::Fixed(NOW)))
}

/// The PR merge's row in `frame`, found by its tail.
fn merge_row(frame: &[String]) -> &str {
    frame
        .iter()
        .find(|line| line.contains(COUNT))
        .unwrap_or_else(|| panic!("the PR's tail is drawn\n{}", frame.join("\n")))
}

/// The widest terminal at which the PR's tail no longer fits whole. At 80
/// it does: the subject yields first, down to one cell (measured
/// 2026-10-18: whole at 51 and 57 to 80, `▸ 4 commits` from 43 to 50, `▸`
/// from 33 to 36).
const NARROW: u16 = 50;

/// The capture: `wallet_release` at [`NARROW`] columns, the PR's tail
/// shortened to `▸ 4 commits` once the counts land; a column wider, whole.
#[test]
fn capture_main_folded_narrow() {
    let scratch = wallet_release();
    let (app, state) = App::open(scratch.path());
    let state = app.resize(&state, 120, ROWS);
    let state = app.pump(state, "the PR's count", |state| {
        drawn(state, 120).iter().any(|line| line.contains(FULL))
    });
    let state = crate::captures::at_rest(&app, state);
    let wider = drawn(&state, NARROW + 1);
    let wider = merge_row(&wider);
    assert!(wider.contains(FULL), "whole a column wider: {wider:?}");
    let frame = drawn(&state, NARROW);
    let row = merge_row(&frame);
    assert!(
        row.contains(&format!("  {COUNT}")) && !row.contains(" from "),
        "the tail sheds ` from …` two cells after the subject: {row:?}"
    );
    crate::captures::check_shot(
        &format!("ease-pass/main-folded-{NARROW}.txt"),
        &format!(
            "ease-pass 06r m2, {NARROW}x36 (the widest that shortens a tail; at 80 none \
             does), main screen at first paint, short of room: drawn by \
             `view::view` from a real AppState over the job pool \
             (crates/mergewright/tests/it/fold_tail_narrow.rs, `wallet_release`). Not a pty \
             capture: a character grid, colour unverified (CLAUDE.md rule 5); the look is \
             yours to judge (rule 6).\n\
             What right looks like: the PR's merge ends `▸ 4 commits`, two cells after its \
             cut subject, no ` from PR 16021` and no word cut (one column wider it is \
             whole: the subject yields first); the named merge's tail shortens the same \
             way or drops whole, never cut.\n\n{}\n",
            frame.join("\n")
        ),
    );
    drop(state);
    app.close();
}
