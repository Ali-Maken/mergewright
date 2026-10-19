//! A folded merge's tail short of room, through the real app (ease-pass
//! 06r m2, ordered by 09 ruling W): on `wallet_release` narrowed, the tail
//! sheds ` from …`, then its count, before it drops, two cells after the
//! whole subject; the subject gives way last. The ladder itself is
//! mergewright-ui's `graph_screen::tests::fold_tail` and `give_way`; here a
//! real repository's row produces it, for the capture.
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
/// The PR merge's subject, 38 cells.
const PR_MERGE: &str = "Merged PR 16021: Fix the wallet totals";

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

/// The widest terminal with no branch pane (`branch_pane::PANE_FROM` is
/// 100). Its message column is 64 cells: the whole subject and the whole
/// tail need 38 + 2 + 25 = 65, so the tail sheds ` from …` and keeps the
/// subject whole (ease-pass 09 ruling W). From 100 the pane takes its slot
/// and the tail gives way altogether.
const NARROW: u16 = 99;

/// The capture: `wallet_release` at [`NARROW`] columns, the PR's tail
/// shortened to `▸ 4 commits` after its whole subject once the counts land.
#[test]
fn capture_main_folded_narrow() {
    let scratch = wallet_release();
    let (app, state) = App::open(scratch.path());
    let state = app.resize(&state, 120, ROWS);
    let state = crate::captures::at_rest(&app, state);
    let frame = drawn(&state, NARROW);
    let row = merge_row(&frame);
    assert!(
        row.contains(&format!("{PR_MERGE}  {COUNT}")) && !row.contains(FULL),
        "the tail sheds ` from …` two cells after the whole subject: {row:?}"
    );
    crate::captures::check_shot(
        &format!("ease-pass/main-folded-{NARROW}.txt"),
        &format!(
            "ease-pass 06r m2, {NARROW}x36 (the widest with no branch pane), main screen at \
             first paint, short of room: drawn by \
             `view::view` from a real AppState over the job pool \
             (crates/mergewright/tests/it/fold_tail_narrow.rs, `wallet_release`). Not a pty \
             capture: a character grid, colour unverified (CLAUDE.md rule 5); the look is \
             yours to judge (rule 6).\n\
             What right looks like: the PR's merge reads its whole subject, then `▸ 4 \
             commits` two cells after it, no ` from PR 16021` and no word cut (the subject \
             gives way last, ease-pass 09 ruling W); the named merge's tail shortens the \
             same way or drops whole, never cut.\n\n{}\n",
            frame.join("\n")
        ),
    );
    drop(state);
    app.close();
}
