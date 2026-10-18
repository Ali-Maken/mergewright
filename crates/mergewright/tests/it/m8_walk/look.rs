//! The look clause's captures at the sizes no ticket shot (m8-polish-and-ship
//! /28, Scope 3): the cursor row on a branch label at 80×24 and 187×50, and
//! `?` over the graph at 187×50 — each beside the mock-up it is judged
//! against. Drawn by `view::view` from a real `AppState`; not a pty capture,
//! so no colour (rule 5, colour unverified).
#![allow(
    clippy::expect_used,
    reason = "a capture whose fixture cannot be built has nothing to show; the panic names why"
)]

use mergewright::session::Clock;
use mergewright_core::event::{KeyCode, MouseButton};

use crate::captures::{check_shot, frame_at, settled_frame};
use crate::look_chrome::{NOW, opened, opened_in, with_feature_two_kept};

/// The cursor clicked onto `feature/two  two: finish` at `columns`×`rows`:
/// `▸` leads the row and the branch badge is on it.
#[test]
fn capture_the_cursor_row_at_80x24_and_187x50() {
    for (columns, rows) in [(80, 24), (187, 50)] {
        // `feature/two` folds into `main` by default; `with_feature_two_kept`
        // keeps its row (ease-pass 06 D3, route 2).
        let (_scratch, app, state) = opened_in(with_feature_two_kept(), columns, rows);
        let drawn = frame_at(&state, columns, rows, Some(Clock::Fixed(NOW)));
        let y = drawn
            .iter()
            .position(|line| line.contains("two: finish"))
            .expect("the row is drawn");
        let row = u16::try_from(y).expect("short screen");
        let state = app.mouse_down(&state, MouseButton::Left, 30, row);
        let drawn = frame_at(&state, columns, rows, Some(Clock::Fixed(NOW)));
        assert!(drawn[y].starts_with('\u{25b8}'), "{:?}", drawn[y]);
        assert!(drawn[y].contains("feature/two"), "{:?}", drawn[y]);
        let caption = format!(
            "m8-polish-and-ship/28, {columns}x{rows}, cursor on `feature/two  two: finish`: \
             what right looks like — `▸` in the gutter's first cell and the branch \
             column's `feature/two` on the same row; the lanes as on every other row; no `▏` in the \
             marker slot. Judge beside m8-look-cursor-row-dark-120x36.txt and the mock-up \
             m8-proto-4a-dark-a-b.txt. Colour unverified (rule 5).\n\n"
        );
        check_shot(
            &format!("m8-look-cursor-row-{columns}x{rows}.txt"),
            &format!("{caption}{}\n", drawn.join("\n")),
        );
        drop(state);
        app.close();
    }
}

/// `?` over the graph at 187×50: the keys palette open on HERE.
#[test]
fn capture_help_over_the_graph_at_187x50() {
    let (columns, rows) = (187, 50);
    let (_scratch, app, state) = opened(columns, rows);
    let state = app.key(&state, KeyCode::Char('?'));
    let state = settled_frame(&app, state);
    let drawn = frame_at(&state, columns, rows, Some(Clock::Fixed(NOW)));
    assert!(
        drawn.iter().any(|line| line.contains("HERE"))
            && drawn.iter().any(|line| line.contains("EVERYWHERE")),
        "{}",
        drawn.join("\n")
    );
    let caption = "m8-polish-and-ship/28, 187x50, `?` on the graph: what right looks like — the \
                   keys palette over the graph, HERE's actions first with their keys, then \
                   EVERYWHERE; the bar holds the palette's own keys. Judge beside \
                   m8-help-graph-120x36.txt and the mock-up m8-proto-2d-help-x.txt. Colour \
                   unverified (rule 5).\n\n";
    check_shot(
        "m8-look-help-graph-187x50.txt",
        &format!("{caption}{}\n", drawn.join("\n")),
    );
    drop(state);
    app.close();
}
