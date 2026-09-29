//! J as drawn by the whole frame (m7-power-tools/24-b): its status row, the
//! bar's `⏎` words per row, and what draws over it — over the real job
//! pool and real `git`, on the "lost work" fixture.
//!
//! J's own layout is graft-ui's (`journal_screen` tests); the words each
//! row gets are `view::journal`'s. These check the frame the user sees:
//! bottom row first (the bar), the legend above it, J's status row above
//! that.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use graft::update::AppState;
use graft_core::event::KeyCode;

use crate::app_harness::App;
use crate::m7_walk::journal::{drawn, lost_work, opened, selected_row, text};

/// J's rows from the bottom: the bar, the legend, the status row.
const BAR: usize = 1;
const LEGEND: usize = 2;
const STATUS: usize = 3;

/// Row `from_bottom` of `frame` (1 = the last).
fn from_bottom(frame: &[String], from_bottom: usize) -> &str {
    &frame[frame.len() - from_bottom]
}

/// `J`, and every stage answered.
fn in_j(app: &App, state: &AppState) -> AppState {
    let state = app.key(state, KeyCode::Char('J'));
    assert!(state.journal.shown, "J opened");
    app.pump(state, "J's stages 2 and 3", |state| {
        !state.journal.finding_lost()
    })
}

/// A refusal of a key pressed in J lands on J's status row, above the
/// legend, J still up — here `b` on his lost edit, which has no commit to
/// branch at (`update::journal::keys::branch_here`). It was `⏎` there until
/// m7/30o R1: a file put-back now runs over the file as it is.
#[test]
fn a_refusal_in_j_shows_on_its_status_row() {
    let fixture = lost_work();
    let (app, state) = opened(fixture.work());
    let mut state = in_j(&app, &state);
    for _ in 0..2 {
        state = app.key(&state, KeyCode::Char('n'));
    }
    let frame = drawn(&state, 187, 50);
    assert!(
        selected_row(&frame).contains("your edit to src/a.rs"),
        "n n reaches the lost edit:\n{}",
        text(&frame)
    );
    let state = app.key(&state, KeyCode::Char('b'));
    let frame = drawn(&state, 187, 50);
    let body = text(&frame);
    assert!(state.journal.shown, "J still up:\n{body}");
    assert!(
        from_bottom(&frame, STATUS).contains("a lost edit has no commit · ⏎ puts it back"),
        "the refusal on J's status row:\n{body}"
    );
    assert!(
        from_bottom(&frame, LEGEND).starts_with("● done in Graft"),
        "the legend under it:\n{body}"
    );
    drop(state);
    app.close();
}

/// Nothing wrong → no status row: the line above the legend is blank.
#[test]
fn no_status_row_when_nothing_is_wrong() {
    let fixture = lost_work();
    let (app, state) = opened(fixture.work());
    let state = in_j(&app, &state);
    let frame = drawn(&state, 187, 50);
    let body = text(&frame);
    assert!(
        from_bottom(&frame, STATUS).trim().is_empty(),
        "a blank line above the legend:\n{body}"
    );
    assert!(
        from_bottom(&frame, LEGEND).starts_with("● done in Graft"),
        "{body}"
    );
    drop(state);
    app.close();
}

/// Each row of the fixture's J, top to bottom, and how its bar starts.
const BARS: [(&str, &str); 14] = [
    ("fetch origin", "⏎ revert to before this  space command log"),
    (
        "reset main 1 commit back",
        "⏎ put main back to before this  b new branch here",
    ),
    (
        "\"Add rate limiter\" · was main's tip",
        "⏎ put main back to it  b new branch here",
    ),
    (
        "switch from spike to main",
        "⏎ switch back to spike  b new branch here",
    ),
    (
        "\"Try redis cache\" · was spike's tip",
        "⏎ make spike again  b new branch here",
    ),
    (
        "committed \"Try redis cache\" on spike",
        "b new branch here",
    ),
    (
        "switch from main to spike",
        "⏎ switch back to main  b new branch here",
    ),
    ("undo: discard src/a.rs", "⏎ redo this  space command log"),
    ("your edit to src/a.rs", "⏎ put your edit back  J/esc close"),
    (
        "discard src/a.rs",
        "⏎ revert to before this  space command log",
    ),
    (
        "committed \"Add rate limiter\" on main",
        "⏎ put main back to before this",
    ),
    (
        "committed \"Add metrics endpoint\" on main",
        "⏎ put main back to before this",
    ),
    (
        "committed \"Add request logging\" on main",
        "⏎ put main back to before this",
    ),
    ("committed \"Start the api\" on main", "b new branch here"),
];

/// The bar starts with what `⏎` does on the selected row, in that row's
/// own words (J1, keymap §2.11), row by row down the whole list.
#[test]
fn the_bar_names_what_enter_does_on_each_row() {
    let fixture = lost_work();
    let (app, state) = opened(fixture.work());
    let mut state = in_j(&app, &state);
    // From the top. The commit on `spike` has no `⏎`: `branch -D` took
    // spike's own log with it, so only HEAD's record is left, and a row
    // of HEAD's alone names no branch to put back (the bar is from the
    // row alone, ticket "Keys").
    for _ in 0..3 {
        state = app.key(&state, KeyCode::Char('k'));
    }
    for (index, (row, bar)) in BARS.iter().enumerate() {
        if index > 0 {
            state = app.key(&state, KeyCode::Char('j'));
        }
        let frame = drawn(&state, 187, 50);
        let body = text(&frame);
        assert!(
            selected_row(&frame).contains(row),
            "row {index} is {row:?}:\n{body}"
        );
        assert!(
            from_bottom(&frame, BAR).starts_with(bar),
            "on {row:?} the bar starts {bar:?}:\n{body}"
        );
    }
    drop(state);
    app.close();
}

/// A step with neither a branch to put back nor a switch to go back on —
/// the first commit, which had no "before" — offers no `⏎` at all: the
/// bar never promises what the row can't do (UX checklist 16).
#[test]
fn no_enter_hint_on_a_step_with_neither() {
    let fixture = lost_work();
    let (app, state) = opened(fixture.work());
    let mut state = in_j(&app, &state);
    for _ in 0..20 {
        state = app.key(&state, KeyCode::Char('j'));
    }
    let frame = drawn(&state, 187, 50);
    let body = text(&frame);
    assert!(
        selected_row(&frame).contains("committed \"Start the api\" on main"),
        "the oldest row:\n{body}"
    );
    assert!(
        !from_bottom(&frame, BAR).contains('⏎'),
        "no ⏎ hint:\n{body}"
    );
    assert!(
        from_bottom(&frame, BAR).starts_with("b new branch here"),
        "{body}"
    );
    drop(state);
    app.close();
}

/// `b` on a `✗` row opens the name prompt over J: J's title still on top,
/// the prompt drawn over its rows.
#[test]
fn the_name_prompt_draws_over_j() {
    let fixture = lost_work();
    let (app, state) = opened(fixture.work());
    let state = in_j(&app, &state);
    let state = app.key(&state, KeyCode::Char('b'));
    assert!(state.journal.shown, "J stays under the prompt");
    let frame = drawn(&state, 187, 50);
    let body = text(&frame);
    assert!(
        frame[0].starts_with("JOURNAL · what happened in this repo"),
        "J's title on top:\n{body}"
    );
    assert!(
        frame.iter().any(|row| row.contains("╭ New branch at ")),
        "the name prompt:\n{body}"
    );
    drop(state);
    app.close();
}

/// `u` in J while an operation is stopped refuses as it does on the graph,
/// and the refusal lands on J's status row with J still up (the ruling:
/// a refusal stays where he pressed the key).
#[test]
fn u_in_j_during_an_operation_shows_the_refusal_on_js_status_row() {
    let scratch = crate::conflict_interim::about_to_conflict();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = crate::conflict_interim::stopped_on_the_conflict(&app, state);
    let state = app.key(&state, KeyCode::Escape);
    assert!(!state.banner_focus, "the panes have the keyboard");
    let state = in_j(&app, &state);
    let state = app.key(&state, KeyCode::Char('u'));
    let frame = drawn(&state, 187, 50);
    let body = text(&frame);
    assert!(state.journal.shown, "J still up:\n{body}");
    assert!(
        from_bottom(&frame, STATUS).contains("finish or abort the merge first · ⇥ banner"),
        "the refusal on J's status row:\n{body}"
    );
    drop(state);
    app.close();
}
