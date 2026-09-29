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

use graft::update::{AppEvent, AppState, update};
use graft_core::event::{Event, KeyCode, KeyPress, Modifiers};

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
    // A fetch is not undone (m7/30o Minor 4): no `⏎` offered on it.
    ("fetch origin", "/ find  space command log"),
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
    (
        "undo: discard src/a.rs",
        "⏎ redo this  / find  space command log",
    ),
    (
        "your edit to src/a.rs",
        "⏎ put your edit back  / find  J/esc close",
    ),
    (
        "discard src/a.rs",
        "⏎ revert to before this  / find  space command log",
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

/// m7/30o Minor 4: a fetch is a step Graft doesn't undo, so its row offers
/// no `⏎` — "before this" there is "before" the next newer `●` row, which
/// keeps its own.
#[test]
fn no_enter_hint_on_a_fetch_row() {
    let fixture = lost_work();
    let (app, state) = opened(fixture.work());
    let mut state = in_j(&app, &state);
    for _ in 0..3 {
        state = app.key(&state, KeyCode::Char('k'));
    }
    let frame = drawn(&state, 187, 50);
    let body = text(&frame);
    assert!(
        selected_row(&frame).contains("fetch origin"),
        "the top row is the fetch:\n{body}"
    );
    assert!(
        !from_bottom(&frame, BAR).contains('⏎'),
        "no ⏎ hint:\n{body}"
    );
    assert!(
        from_bottom(&frame, BAR).starts_with("/ find  space command log"),
        "{body}"
    );
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

// J's `/` find (m7-power-tools/30o-c R4), key for key as blame's (§2.8,
// `update::place::keys`): not incremental — `⏎` finds — `n`/`N` step and
// wrap, `Esc` drops the find before it closes J.

/// J's title row.
fn title(frame: &[String]) -> &str {
    frame
        .iter()
        .find(|row| row.starts_with("JOURNAL ·"))
        .map_or("", String::as_str)
}

/// `/`, then `query` typed a key at a time.
fn find_typed(app: &App, state: &AppState, query: &str) -> AppState {
    let mut state = app.key(state, KeyCode::Char('/'));
    for typed in query.chars() {
        state = app.key(&state, KeyCode::Char(typed));
    }
    state
}

/// J up on the fixture, every stage in, then `/spike ⏎`.
fn found_spike(app: &App, state: &AppState) -> AppState {
    let state = in_j(app, state);
    let state = find_typed(app, &state, "spike");
    app.key(&state, KeyCode::Enter)
}

/// How many of J's rows on the frame name `spike`.
fn spike_rows(frame: &[String]) -> usize {
    frame
        .iter()
        .skip_while(|row| !row.starts_with("JOURNAL ·"))
        .skip(1)
        .take_while(|row| !row.contains("leaves no trace here"))
        .filter(|row| row.contains("spike"))
        .count()
}

#[test]
fn slash_opens_the_find_on_js_title_row() {
    let fixture = lost_work();
    let (app, state) = opened(fixture.work());
    let state = in_j(&app, &state);
    let state = find_typed(&app, &state, "spi");
    let frame = drawn(&state, 187, 50);
    let body = text(&frame);
    assert!(state.journal.shown, "J still up:\n{body}");
    // As blame's: while he types, the prompt stands where the bar was.
    assert_eq!(from_bottom(&frame, BAR), "/spi", "the prompt:\n{body}");
    let state = app.key(&state, KeyCode::Enter);
    let frame = drawn(&state, 187, 50);
    let body = text(&frame);
    assert!(
        title(&frame).contains(" · find \"spi\" 1 of "),
        "the title carries blame's find tail:\n{body}"
    );
    drop(state);
    app.close();
}

/// Blame's find is not incremental (`place::keys::prompt_key`: the cursor
/// moves on `⏎` only), so neither is J's: typing leaves the cursor, `⏎`
/// takes it to the first row at or below it that holds the words.
#[test]
fn typing_then_enter_moves_to_the_first_row_that_holds_it() {
    let fixture = lost_work();
    let (app, state) = opened(fixture.work());
    let state = in_j(&app, &state);
    let before = drawn(&state, 187, 50);
    let state = find_typed(&app, &state, "redis");
    let typing = drawn(&state, 187, 50);
    assert_eq!(
        selected_row(&typing),
        selected_row(&before),
        "typing moves nothing:\n{}",
        text(&typing)
    );
    let state = app.key(&state, KeyCode::Enter);
    let frame = drawn(&state, 187, 50);
    let body = text(&frame);
    assert!(
        selected_row(&frame).contains("419e109 \"Try redis cache\""),
        "▸ on the first row below that holds `redis`:\n{body}"
    );
    assert!(
        title(&frame).ends_with(" · find \"redis\" 1 of 2"),
        "the title counts the hits:\n{body}"
    );
    drop(state);
    app.close();
}

#[test]
fn enter_keeps_the_find_and_n_steps_the_hits() {
    let fixture = lost_work();
    let (app, state) = opened(fixture.work());
    let state = found_spike(&app, &state);
    let frame = drawn(&state, 187, 50);
    let of = spike_rows(&frame);
    assert!(
        of >= 2,
        "the fixture names spike on several rows:\n{}",
        text(&frame)
    );
    let at = |state: &AppState, at: usize| {
        let frame = drawn(state, 187, 50);
        let body = text(&frame);
        assert!(
            title(&frame).ends_with(&format!(" · find \"spike\" {at} of {of}")),
            "at {at} of {of}:\n{body}"
        );
        assert!(
            selected_row(&frame).contains("spike"),
            "▸ on a hit:\n{body}"
        );
    };
    at(&state, 1);
    // The keys are J's again: the bar is back, `n N match` on it.
    assert!(
        from_bottom(&frame, BAR).contains("/ find  n N match"),
        "{}",
        text(&frame)
    );
    let mut state = state;
    for step in 2..=of {
        state = app.key(&state, KeyCode::Char('n'));
        at(&state, step);
    }
    // `n` on the last wraps to the first; `N` there goes back to the last.
    let state = app.key(&state, KeyCode::Char('n'));
    at(&state, 1);
    let state = app.key(&state, KeyCode::Char('N'));
    at(&state, of);
    drop(state);
    app.close();
}

/// Blame's `Esc` while typing closes the prompt and nothing else — its
/// cursor never moved (`place::keys::prompt_key`), so there is nothing to
/// put back. J's the same: the cursor is where it was at `/`.
#[test]
fn esc_while_typing_puts_the_cursor_back() {
    let fixture = lost_work();
    let (app, state) = opened(fixture.work());
    let state = in_j(&app, &state);
    let before = drawn(&state, 187, 50);
    let state = find_typed(&app, &state, "redis");
    let state = app.key(&state, KeyCode::Escape);
    let frame = drawn(&state, 187, 50);
    let body = text(&frame);
    assert!(state.journal.shown, "J still up:\n{body}");
    assert_eq!(selected_row(&frame), selected_row(&before), "{body}");
    assert!(!title(&frame).contains("find"), "no find kept:\n{body}");
    assert!(
        from_bottom(&frame, BAR).ends_with("J/esc close"),
        "the bar is back:\n{body}"
    );
    drop(state);
    app.close();
}

#[test]
fn esc_drops_a_kept_find_then_closes_j() {
    let fixture = lost_work();
    let (app, state) = opened(fixture.work());
    let state = found_spike(&app, &state);
    let kept = drawn(&state, 187, 50);
    let state = app.key(&state, KeyCode::Escape);
    let frame = drawn(&state, 187, 50);
    let body = text(&frame);
    assert!(state.journal.shown, "the first Esc drops the find:\n{body}");
    assert!(!title(&frame).contains("find"), "{body}");
    assert_eq!(
        selected_row(&frame),
        selected_row(&kept),
        "the cursor stays:\n{body}"
    );
    let state = app.key(&state, KeyCode::Escape);
    assert!(!state.journal.shown, "the next Esc closes J");
    drop(state);
    app.close();
}

#[test]
fn no_match_says_so_and_moves_nothing() {
    let fixture = lost_work();
    let (app, state) = opened(fixture.work());
    let state = in_j(&app, &state);
    let before = drawn(&state, 187, 50);
    let state = find_typed(&app, &state, "nothing-names-this");
    let state = app.key(&state, KeyCode::Enter);
    let frame = drawn(&state, 187, 50);
    let body = text(&frame);
    assert!(
        title(&frame).ends_with(" · find \"nothing-names-this\" · no match"),
        "blame's no-match words:\n{body}"
    );
    assert_eq!(selected_row(&frame), selected_row(&before), "{body}");
    drop(state);
    app.close();
}

#[test]
fn a_paste_goes_into_the_find() {
    let fixture = lost_work();
    let (app, state) = opened(fixture.work());
    let state = in_j(&app, &state);
    let state = app.key(&state, KeyCode::Char('/'));
    let (state, commands) = update(&state, AppEvent::Core(Event::Paste("redis".to_owned())));
    assert!(commands.is_empty(), "a paste runs nothing: {commands:?}");
    let frame = drawn(&state, 187, 50);
    assert_eq!(from_bottom(&frame, BAR), "/redis", "{}", text(&frame));
    let state = app.key(&state, KeyCode::Enter);
    let frame = drawn(&state, 187, 50);
    assert!(
        selected_row(&frame).contains("Try redis cache"),
        "{}",
        text(&frame)
    );
    drop(state);
    app.close();
}

/// The trap (`AppState::text_field_open`): `u`, `U` and `Ctrl+Z` typed
/// into the find are text or nothing, never an undo. Nothing is written:
/// HEAD and the journal's entry count are as they were.
#[test]
fn u_typed_into_the_find_is_text() {
    let fixture = lost_work();
    let repo = fixture.work();
    let (app, state) = opened(repo);
    let state = in_j(&app, &state);
    let head = crate::app_harness::git(repo, &["rev-parse", "HEAD"]);
    let entries = state.journal.newest_first().len();
    let state = find_typed(&app, &state, "uU");
    let state = app.press(
        &state,
        KeyPress {
            code: KeyCode::Char('z'),
            modifiers: Modifiers::CONTROL,
        },
    );
    assert!(
        state.operating.is_none(),
        "no write queued: {:?}",
        state.status
    );
    assert!(state.confirm.is_none(), "no box asked");
    let frame = drawn(&state, 187, 50);
    assert_eq!(from_bottom(&frame, BAR), "/uU", "{}", text(&frame));
    assert_eq!(crate::app_harness::git(repo, &["rev-parse", "HEAD"]), head);
    assert_eq!(
        state.journal.newest_first().len(),
        entries,
        "no entry written"
    );
    drop(state);
    app.close();
}

/// Rows arriving mid-find keep it: stage 3 lands with the find kept, the
/// cursor stays on its row, the count takes the new `✗` hit, and nothing
/// is preselected (`/` is a key).
#[test]
fn a_find_survives_rows_arriving() {
    let fixture = lost_work();
    let (app, state) = opened(fixture.work());
    let state = app.key(&state, KeyCode::Char('J'));
    // Stage 2 in (the shell's `$` rows), stage 3 still out.
    let state = app.pump(state, "stage 2", |state| {
        drawn(state, 187, 50)
            .iter()
            .any(|row| row.contains("$  outside Graft"))
    });
    assert!(state.journal.finding_lost(), "stage 3 is still out");
    let state = find_typed(&app, &state, "spike");
    let state = app.key(&state, KeyCode::Enter);
    let frame = drawn(&state, 187, 50);
    let on = selected_row(&frame).to_owned();
    let before = spike_rows(&frame);
    assert!(on.contains("spike"), "{}", text(&frame));
    assert!(
        title(&frame).contains(&format!(" · find \"spike\" 1 of {before}")),
        "{}",
        text(&frame)
    );
    let state = app.pump(state, "stage 3", |state| !state.journal.finding_lost());
    let frame = drawn(&state, 187, 50);
    let body = text(&frame);
    assert_eq!(selected_row(&frame), on, "the cursor kept its row:\n{body}");
    let after = spike_rows(&frame);
    assert!(
        body.contains("419e109") && after == before + 1,
        "the ✗ row spike held is a new hit ({before} → {after}):\n{body}"
    );
    assert!(
        title(&frame).ends_with(&format!("find \"spike\" 1 of {after}")),
        "the count takes it:\n{body}"
    );
    drop(state);
    app.close();
}
