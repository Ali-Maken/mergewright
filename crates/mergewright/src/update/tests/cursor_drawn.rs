//! `update` tests: cursor drawn. Moved out of `update/tests.rs` (m8/21b-5, move-only).

use super::*;

/// `history_of(count)` at `columns`×`rows`, the tree dirty or clean.
fn sixty_rows(rows: u16, dirty: bool) -> AppState {
    use crate::view::tests::{counts, history_of, oid, refs_with_head};
    AppState {
        history: Some(history_of(60, 0)),
        refs: Some(Arc::new(refs_with_head(Some(oid(60))))),
        working_tree: dirty.then(|| counts(1, 0, 0)),
        size: Size { columns: 120, rows },
        ..AppState::default()
    }
}

/// The cursor's row — a history row, or the uncommitted row while the
/// cursor rests on it — is one of the rows the graph view draws.
fn cursor_drawn(state: &AppState) -> bool {
    let view = crate::view::graph_view(state, state.harness.clock.now_seconds());
    view.rows
        .iter()
        .any(|entry| entry.line.paint == mergewright_ui::graph_column::RowPaint::Highlighted)
}

fn cursor_and_top(state: &AppState) -> (usize, usize) {
    let history = state.history.as_ref().expect("history");
    (history.cursor, history.top)
}

#[test]
fn every_cursor_key_keeps_the_cursor_drawn() {
    // The graph pane's cursor keys (docs/design/02-keymap.md §2.2): each is
    // pressed past the pane's edge, from the end it moves away from.
    let down = [
        KeyCode::Char('j'),
        KeyCode::Down,
        KeyCode::Char('G'),
        KeyCode::End,
        KeyCode::PageDown,
    ];
    let up = [
        KeyCode::Char('k'),
        KeyCode::Up,
        KeyCode::Char('g'),
        KeyCode::Home,
        KeyCode::PageUp,
    ];
    for rows in [20, 50] {
        for dirty in [false, true] {
            let start = sixty_rows(rows, dirty);
            assert_eq!(start.uncommitted_row_pinned(), dirty);
            let (bottom, _) = update(&start, key(KeyCode::Char('G')));
            for (from, code) in down
                .iter()
                .map(|code| (&start, *code))
                .chain(up.iter().map(|code| (&bottom, *code)))
            {
                let mut state = from.clone();
                for press in 1..=70 {
                    state = update(&state, key(code)).0;
                    assert!(
                        cursor_drawn(&state),
                        "{code:?} ×{press} at {rows} rows, dirty {dirty}: cursor/top {:?}, \
                         graph_rows {}",
                        cursor_and_top(&state),
                        state.graph_rows(),
                    );
                }
            }
        }
    }
}

/// Search's `n`/`N` jump the cursor to hits past the pane's edge.
#[test]
fn n_and_capital_n_keep_the_cursor_drawn_under_the_uncommitted_row() {
    let walk = JobId::new(4);
    let mut state = on_hand(walk);
    state.working_tree = Some(crate::view::tests::counts(1, 0, 0));
    assert!(state.uncommitted_row_pinned());
    let (state, _) = typed(&state, "fix");
    let job = JobId::new(9);
    let (state, _) = update(&state, AppEvent::Started(job, Task::Search));
    // Rows 2, 9 (the last drawn at ten pane rows) and 29.
    let (mut state, _) = update(&state, found(job, &[3, 10, 30]));
    assert!(cursor_drawn(&state));
    for code in ['n', 'n', 'n', 'N', 'N', 'N', 'N'] {
        state = update(&state, key(KeyCode::Char(code))).0;
        assert!(
            cursor_drawn(&state),
            "{code}: cursor/top {:?}, graph_rows {}",
            cursor_and_top(&state),
            state.graph_rows()
        );
    }
}

/// A landing (`detail::show_line`, the place an operation or blame sends
/// the cursor) on a dirty tree puts the cursor on a drawn row.
#[test]
fn a_landing_under_the_uncommitted_row_keeps_the_cursor_drawn() {
    for rows in [20, 50] {
        let mut state = sixty_rows(rows, true);
        // Row 55 holds commit 5 (`history_of` is newest first).
        let target = crate::view::tests::oid(5);
        detail::show_line(
            &mut state,
            detail::DetailSpec::Commit(target),
            std::path::Path::new("a.txt"),
            None,
            None,
        );
        assert_eq!(cursor_and_top(&state).0, 55, "landed");
        assert!(
            cursor_drawn(&state),
            "at {rows} rows: cursor/top {:?}",
            cursor_and_top(&state)
        );
    }
}

/// Decided 5: a dirty status that pins the uncommitted row takes a history
/// row; a cursor on the last drawn row stays drawn. A cursor the wheel left
/// off screen stays off screen (m7/37).
#[test]
fn a_dirty_tree_arriving_keeps_a_shown_cursor_shown() {
    let dirty = || {
        AppEvent::Produced(
            JobId::new(1),
            Product::WorkingTree(Ok(Box::new(crate::working_tree::WorkingTree {
                rewrite: None,
                status: crate::view::tests::counts(1, 0, 0),
                in_progress: None,
                mergetool: None,
            }))),
        )
    };
    let mut state = sixty_rows(20, false);
    let clean_rows = state.graph_rows();
    for _ in 1..clean_rows {
        state = update(&state, key(KeyCode::Char('j'))).0;
    }
    assert_eq!(
        cursor_and_top(&state),
        (clean_rows - 1, 0),
        "on the last drawn row"
    );
    let (state, _) = update(&state, dirty());
    assert!(state.uncommitted_row_pinned());
    assert_eq!(state.graph_rows(), clean_rows - 1);
    assert_eq!(
        cursor_and_top(&state),
        (clean_rows - 1, 1),
        "scrolled by one"
    );
    assert!(cursor_drawn(&state));

    // The wheel leaves the cursor behind; the next shrink must not fetch it.
    let mut wheeled = sixty_rows(20, false);
    let wheel = core(Event::Mouse(MouseInput {
        kind: MouseKind::ScrollDown,
        column: 10,
        row: HEADER_ROWS + 2,
        modifiers: Modifiers::NONE,
    }));
    for _ in 0..10 {
        wheeled = update(&wheeled, wheel.clone()).0;
    }
    let (cursor, top) = cursor_and_top(&wheeled);
    assert!(cursor < top, "the wheel left the cursor above the pane");
    let (arrived, _) = update(&wheeled, dirty());
    assert!(arrived.uncommitted_row_pinned());
    assert_eq!(cursor_and_top(&arrived), (cursor, top), "left where it was");
    assert!(!cursor_drawn(&arrived), "still off screen");
}
