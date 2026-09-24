//! The three panes in the app (m6-conflicts/14): a click in any pane picks
//! its block, the draw and the hit-test agree on the arrangement either
//! side of 180 columns, and the cursor block stays on screen when it
//! scrolls or the terminal is resized across 180.
//!
//! No test here moves by `j`/`k`: those keys belong to
//! `17-resolver-file-keys` (ticket 14, ruling S3).

use std::sync::Arc;

use graft_core::event::{Event, Modifiers, MouseButton, MouseInput, MouseKind};
use graft_ui::conflict_resolver::{HEADER_ROWS, Layout, ScreenRow, three_panes};

use super::WALK;
use crate::update::resolver::geometry::{body, scrolled_to};
use crate::update::{AppEvent, AppState, Size, update};

/// The two-block resolver at `columns` × `rows`.
fn at(columns: u16, rows: u16) -> AppState {
    AppState {
        size: Size { columns, rows },
        ..crate::update::tests::resolver_on_screen(WALK)
    }
}

/// A file far longer than any body: three blocks, 40 plain lines before
/// each and 10 after the last.
fn long(columns: u16, rows: u16) -> AppState {
    let mut bytes = String::new();
    for block in 0..3 {
        for line in 0..40 {
            bytes.push_str(&format!("plain {block}.{line}\n"));
        }
        bytes.push_str(&format!(
            "<<<<<<< HEAD\nours {block}\n=======\ntheirs {block}\nand more {block}\n>>>>>>> topic\n"
        ));
    }
    for line in 0..10 {
        bytes.push_str(&format!("tail {line}\n"));
    }
    let file = graft_git::conflict::parse(bytes.as_bytes()).expect("three open blocks");
    let mut state = at(columns, rows);
    let open = state.resolver.as_mut().expect("open");
    open.file = Some(Arc::new(file));
    state
}

/// The whole frame as the app draws it, one string per row.
fn frame(state: &AppState) -> Vec<String> {
    crate::view::resolver::drawn_for_tests(state)
}

fn laid(state: &AppState) -> Layout {
    body(state).expect("a document").0
}

/// A left click at `column`, `row`.
fn click(state: &AppState, column: u16, row: u16) -> AppState {
    update(
        state,
        AppEvent::Core(Event::Mouse(MouseInput {
            kind: MouseKind::Down(MouseButton::Left),
            column,
            row,
            modifiers: Modifiers::NONE,
        })),
    )
    .0
}

fn cursor(state: &AppState) -> Option<usize> {
    state.resolver.as_ref().and_then(|open| open.cursor_block)
}

/// The screen row of layout row `index`, with the window at `top`.
fn screen_row(state: &AppState, index: usize) -> u16 {
    let (_, _, split) = super::super::geometry::areas(state);
    let top = state.resolver.as_ref().map_or(0, |open| open.top);
    let body = u16::try_from(index - top).expect("a small document");
    split.doc.y + HEADER_ROWS + body
}

#[test]
fn a_click_in_any_pane_picks_the_block() {
    let state = at(200, 50);
    let laid = laid(&state);
    assert_eq!(laid.rows[0], ScreenRow::PaneTitles, "three panes at 200");
    assert_eq!(cursor(&state), Some(0));
    let row = screen_row(&state, laid.block_row[1]);
    let (_, _, split) = super::super::geometry::areas(&state);
    let [yours, result, theirs] = three_panes(split.doc);
    let columns = [
        yours.x + 4,
        result.x + 4,
        theirs.x + 4,
        yours.right(),  // the `│` after yours
        result.right(), // the `│` after the result
    ];
    for column in columns {
        assert_eq!(
            cursor(&click(&state, column, row)),
            Some(1),
            "column {column}"
        );
    }
    // The drawn frame agrees: block 1's rule row is on that screen row.
    let drawn = frame(&state);
    assert!(
        drawn[usize::from(row)].contains("── 2 of 2 · open"),
        "{}",
        drawn[usize::from(row)]
    );
}

#[test]
fn draw_and_click_agree_across_180() {
    for columns in [179, 180, 181] {
        let state = at(columns, 50);
        let laid = laid(&state);
        let three = laid.rows[0] == ScreenRow::PaneTitles;
        assert_eq!(three, columns >= 180, "{columns}: geometry's arrangement");
        let drawn = frame(&state);
        let titles = drawn
            .iter()
            .any(|row| row.contains("│ yours ") && row.contains("│ result "));
        assert_eq!(
            titles, three,
            "{columns}: the draw's arrangement\n{drawn:#?}"
        );
        // The first body row of block 1: its first line under the rule
        // (three panes) or its labels row under the header (card).
        let row = screen_row(&state, laid.block_row[1] + 1);
        assert_eq!(cursor(&click(&state, 100, row)), Some(1), "{columns}");
    }
}

#[test]
fn next_block_scrolls_three_panes_to_its_rule_row() {
    let mut state = long(200, 30);
    let laid = laid(&state);
    let (_, height) = body(&state).expect("a document");
    let last = laid.block_row.len() - 1;
    assert!(
        laid.block_row[last] >= usize::from(height),
        "the last block starts on screen already: the test proves nothing"
    );
    let top = scrolled_to(&state, last);
    state.resolver.as_mut().expect("open").top = top;
    let rule = laid.block_row[last];
    assert!(rule >= top && rule < top + usize::from(height), "{top}");
    let drawn = frame(&state);
    assert!(
        drawn[usize::from(screen_row(&state, rule))].contains("── 3 of 3 · open"),
        "{drawn:#?}"
    );
}

/// The frame shows block `n` (1-based) of 3's own rule or header.
fn shows_block(drawn: &[String], n: usize) -> bool {
    drawn
        .iter()
        .any(|row| row.contains(&format!("{n} of 3 · open")))
}

#[test]
fn resize_across_180_keeps_the_cursor_block_in_view() {
    let mut state = long(200, 30);
    // The cursor block's rule on the window's last row: the card has more
    // rows above it (block 0's header, labels and end against one rule
    // row), so crossing 180 pushes it below the window unless the resize
    // scrolls it back.
    let (layout, height) = body(&state).expect("a document");
    let rule = layout.block_row[1];
    let open = state.resolver.as_mut().expect("open");
    open.cursor_block = Some(1);
    open.top = rule + 1 - usize::from(height);
    assert!(shows_block(&frame(&state), 2), "block 2 on screen at 200");
    for (columns, rows) in [(179, 30), (200, 30), (120, 30), (181, 30)] {
        state = update(&state, AppEvent::Core(Event::Resize { columns, rows })).0;
        let drawn = frame(&state);
        assert!(shows_block(&drawn, 2), "{columns}x{rows}:\n{drawn:#?}");
    }
}
