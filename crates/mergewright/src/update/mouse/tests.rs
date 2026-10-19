//! Mouse tests, moved out of `mouse` (ease-pass 09, move-only).

use mergewright_core::event::{Event, Modifiers, MouseButton, MouseInput, MouseKind};

use super::geometry::{self, HEADER_ROWS};
use crate::update::{AppEvent, AppState, Size, update};
use crate::view::tests::{counts, history_of, oid, refs_with_head};

fn click(state: &AppState, row: u16) -> AppState {
    let down = MouseInput {
        kind: MouseKind::Down(MouseButton::Left),
        column: 10,
        row,
        modifiers: Modifiers::NONE,
    };
    update(state, AppEvent::Core(Event::Mouse(down))).0
}

/// m7-power-tools/48: with the uncommitted row pinned, the pane is one
/// row taller than its history rows. The bounds check reads the pane, so
/// the last drawn history row stays clickable and the row below the pane
/// is no graph row.
#[test]
fn click_on_the_last_row_under_the_uncommitted_row() {
    let mut state = AppState {
        history: Some(history_of(60, 5)),
        refs: Some(std::sync::Arc::new(refs_with_head(Some(oid(60))))),
        working_tree: Some(counts(1, 0, 0)),
        size: Size {
            columns: 120,
            rows: 20,
        },
        ..AppState::default()
    };
    assert!(state.uncommitted_row_pinned());
    let (pane, history_rows) = (state.graph_pane_rows(), state.graph_rows());
    // The cursor row's second line takes one too (ease-pass/07b).
    assert_eq!(history_rows + 2, pane, "the pinned row takes one pane row");
    let first = HEADER_ROWS;
    let last = first + u16::try_from(pane - 1).expect("small");

    assert_eq!(
        geometry::graph_row_at(&state, last),
        Some(history_rows - 1),
        "the last drawn history row"
    );
    assert_eq!(
        geometry::graph_row_at(&state, last + 1),
        None,
        "below the pane"
    );
    assert_eq!(
        geometry::graph_row_at(&state, first),
        None,
        "the uncommitted row"
    );

    let picked = click(&state, last);
    assert!(!picked.on_working_tree);
    assert_eq!(
        picked.history.as_ref().map(|history| history.cursor),
        Some(history_rows - 1)
    );

    state = click(&state, first);
    assert!(state.on_working_tree, "a click on the uncommitted row");
}

/// The one-row pane (Decided 4): a click selects what is drawn there.
#[test]
fn a_click_in_a_one_row_pane_selects_what_is_drawn() {
    let mut state = AppState {
        history: Some(history_of(10, 3)),
        refs: Some(std::sync::Arc::new(refs_with_head(Some(oid(10))))),
        working_tree: Some(counts(1, 0, 0)),
        size: Size {
            columns: 120,
            rows: 3,
        },
        ..AppState::default()
    };
    if let Some(history) = state.history.as_mut() {
        history.top = 3;
    }
    assert!(!state.uncommitted_row_pinned());
    assert_eq!(geometry::graph_row_at(&state, HEADER_ROWS), Some(3));
    let picked = click(&state, HEADER_ROWS);
    assert!(!picked.on_working_tree, "the history row drawn there");

    state.on_working_tree = true;
    assert_eq!(geometry::graph_row_at(&state, HEADER_ROWS), None);
    let picked = click(&state, HEADER_ROWS);
    assert!(picked.on_working_tree, "the uncommitted row drawn there");
}
