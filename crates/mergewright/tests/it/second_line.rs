//! The cursor row's second line through the real app (ease-pass/07b, mock
//! G9), over `fold_by_default`'s `wallet_release`: only the cursor row has
//! one; it reads who, when, `made on`, a known merge's `merged N from`, and
//! ends with the short hash; the lanes continue straight through it; `j`
//! moves it; a click on it selects the cursor row, a click below it the row
//! drawn there; held `j` never leaves it off the body.
#![allow(
    clippy::expect_used,
    reason = "a fixture that cannot be built has nothing to test; the panic names why"
)]

use mergewright::update::AppState;
use mergewright_core::event::{KeyCode, MouseButton};

use crate::app_harness::App;
use crate::fold_by_default::{drawn, first_paint, wallet_release};

const PR_MERGE: &str = "Merged PR 16021: Fix the wallet totals";

fn opened(repo: &std::path::Path) -> (App, AppState) {
    let (app, state) = first_paint(repo);
    let state = app.pump(state, "the refs", |state| state.refs.is_some());
    (app, state)
}

/// The frame's line index holding `needle`.
fn line_of(frame: &[String], needle: &str) -> usize {
    frame
        .iter()
        .position(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("{needle:?} is drawn\n{}", frame.join("\n")))
}

/// The lines that are a second line: they read ` ago · ` and no age column.
fn second_lines(frame: &[String]) -> Vec<usize> {
    frame
        .iter()
        .enumerate()
        .filter(|(_, line)| line.contains(" ago \u{b7} ") || line.contains("not committed yet"))
        .map(|(at, _)| at)
        .collect()
}

fn click_line(app: &App, state: &AppState, y: usize) -> AppState {
    let y = u16::try_from(y).expect("short");
    app.mouse_down(state, MouseButton::Left, 40, y)
}

fn selected(state: &AppState) -> String {
    let row = state
        .history
        .as_ref()
        .and_then(mergewright::history::History::selected)
        .expect("a selected row");
    let message = String::from_utf8_lossy(&row.commit.message).into_owned();
    message.lines().next().unwrap_or_default().to_owned()
}

/// The graph column's cells of a line: the first 14 characters past the
/// gutter, where the lanes are drawn.
fn lanes(line: &str) -> String {
    line.chars().skip(2).take(12).collect()
}

#[test]
fn only_the_cursor_row_has_a_second_line_and_it_reads_the_mock_words() {
    let scratch = wallet_release();
    let (app, state) = opened(scratch.path());
    let frame = drawn(&state);
    let seconds = second_lines(&frame);
    assert_eq!(seconds.len(), 1, "one second line\n{}", frame.join("\n"));
    let at = seconds[0];
    let text = frame[at].trim_end();
    let words = text.trim_start_matches(|c: char| !c.is_alphabetic());
    // The fixture's commits are made by `Graft`, not by the reader.
    assert!(
        words.starts_with("Graft \u{b7} "),
        "starts with who: {text:?}"
    );
    assert!(text.contains(" ago \u{b7} made on "), "{text:?}");
    let sha = text.rsplit(" \u{b7} ").next().expect("a last part");
    assert!(
        sha.len() == 7 && sha.chars().all(|c| c.is_ascii_hexdigit()),
        "ends with a 7-hex hash: {text:?}"
    );
    assert!(frame.iter().all(|line| !line.contains("reading")));
    // The lanes continue straight: every lane cell is a vertical or blank,
    // and each vertical stands under a lane of the row above.
    let above = lanes(&frame[at - 1]);
    for (cell, over) in lanes(&frame[at]).chars().zip(above.chars()) {
        assert!(
            cell == ' ' || "\u{2502}\u{2503}\u{2551}\u{250a}".contains(cell),
            "no node, no connector on the second line: {:?}",
            frame[at]
        );
        assert!(
            cell == ' ' || over != ' ',
            "a lane from nowhere: {:?}",
            frame[at]
        );
    }
    // `j` moves it to the next row's line.
    let moved = app.key(&state, KeyCode::Char('j'));
    let after = drawn(&moved);
    assert_eq!(second_lines(&after), vec![at + 1], "{}", after.join("\n"));
    app.close();
}

#[test]
fn a_known_merge_row_says_how_many_it_merged_and_from_where() {
    let scratch = wallet_release();
    let (app, state) = opened(scratch.path());
    let frame = drawn(&state);
    let state = click_line(&app, &state, line_of(&frame, PR_MERGE));
    let state = app.pump(state, "the fold's count", |state| {
        drawn(state)
            .iter()
            .any(|line| line.contains(" ago \u{b7} ") && line.contains("merged 4"))
    });
    let frame = drawn(&state);
    let text = &frame[second_lines(&frame)[0]];
    assert!(text.contains("merged 4 from "), "{text:?}");
    app.close();
}

#[test]
fn a_click_on_the_second_line_selects_the_cursor_row_and_below_it_the_row_drawn_there() {
    let scratch = wallet_release();
    let (app, state) = opened(scratch.path());
    let frame = drawn(&state);
    let at = second_lines(&frame)[0];
    let cursor = selected(&state);
    let on_second = click_line(&app, &state, at);
    assert_eq!(
        selected(&on_second),
        cursor,
        "the second line is the cursor row's"
    );
    // The line under the second line is the next commit row.
    let below = frame[at + 1].clone();
    let picked = click_line(&app, &state, at + 1);
    let subject = selected(&picked);
    let shown: String = subject.chars().take(12).collect();
    assert!(below.contains(&shown), "{below:?} vs {subject:?}");
    app.close();
}

#[test]
fn forty_held_j_never_leave_the_cursor_or_its_second_line_off_the_body() {
    let scratch = wallet_release();
    let (app, mut state) = opened(scratch.path());
    for press in 0..40 {
        state = app.key(&state, KeyCode::Char('j'));
        let frame = drawn(&state);
        let seconds = second_lines(&frame);
        assert_eq!(seconds.len(), 1, "press {press}\n{}", frame.join("\n"));
        // The body ends above the footer (last line) and its rule, if any:
        // the second line is drawn, so it sits on a body line.
        let at = seconds[0];
        assert!(at < frame.len() - 1, "press {press}: under the footer");
        let cursor_line = at - 1;
        assert!(
            frame[cursor_line].starts_with('\u{25b8}') || frame[cursor_line].starts_with('>'),
            "press {press}: the cursor row sits right above it\n{}",
            frame.join("\n")
        );
    }
    app.close();
}

#[test]
fn the_uncommitted_rows_second_line_says_not_committed_yet() {
    let scratch = wallet_release();
    std::fs::write(scratch.path().join("note.txt"), "draft\n").expect("a file written");
    let (app, state) = opened(scratch.path());
    let state = app.pump(state, "the uncommitted row", |state| {
        drawn(state).iter().any(|line| line.contains("Uncommitted"))
    });
    // `k` from the top commit row, or staying on the uncommitted row.
    let state = app.key(&state, KeyCode::Char('k'));
    let frame = drawn(&state);
    let seconds = second_lines(&frame);
    assert_eq!(seconds.len(), 1, "{}", frame.join("\n"));
    let text = &frame[seconds[0]];
    assert!(
        text.contains("not committed yet \u{b7} 1 file \u{b7} on "),
        "{text:?}"
    );
    app.close();
}
