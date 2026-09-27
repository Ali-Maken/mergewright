//! History-place journeys (m7-power-tools/21), walked on the session
//! fixture with real `git` and the real job pool: `function-history`
//! (`B L` = 2, from the diff cursor on line 22) and `file-history` (`F` =
//! 1, from the detail pane on the file's row). Setup keys are not counted,
//! as in `m7_walk::blame`.

use std::path::Path;

use graft::update::AppState;
use graft::update::place::{FileLoad, HistoryState, TraceLoad};
use graft_core::event::KeyCode;
use graft_git::trace::LineRange;

use super::blame::{
    FILE, LINE, Route, blame_ready, cursor_line, drawn, oid, on_line_22, open_on_the_file, session,
};
use crate::app_harness::App;

/// The history place on screen.
pub fn history(state: &AppState) -> &HistoryState {
    state
        .trail
        .history_on_screen()
        .unwrap_or_else(|| panic!("no history on screen: {:?}", state.status))
}

/// Pump until the trace has answered, and — when a commit is selected —
/// its whole file.
pub fn traced(app: &App, state: AppState) -> AppState {
    app.pump(state, "the trace and its file", |state| {
        state.trail.history_on_screen().is_some_and(|history| {
            !matches!(history.trace, TraceLoad::Tracing { .. })
                && !matches!(history.file, FileLoad::Reading { .. })
        })
    })
}

/// The journey ends on the history place in the real frame, not only in
/// state (m7-power-tools/21b): its title on row 0.
fn assert_drawn(state: &AppState, columns: u16, rows: u16) {
    let frame = drawn(state, columns, rows);
    assert!(
        frame[0].trim_start().starts_with("HISTORY \u{b7} "),
        "the history place is drawn:\n{}",
        frame.join("\n")
    );
}

/// `function-history`: `B L` from the diff cursor on line 22 — the
/// commits that changed the function around it, the cursor on Lina's (the
/// commit blame showed), her whole file on the right with the range marked.
#[test]
fn function_history() {
    let scratch = session();
    let repo = scratch.path();
    let lina = oid(repo, "main~3");
    let (app, state) = on_line_22(repo, 187, 50);

    let mut route = Route::new("function-history");
    let state = route.press(&app, &state, "B", KeyCode::Char('B'));
    let state = blame_ready(&app, state);
    assert_eq!(cursor_line(&state).final_line, LINE);
    let state = route.press(&app, &state, "L", KeyCode::Char('L'));
    let state = traced(&app, state);
    route.within_count();
    assert_drawn(&state, 187, 50);

    let place = history(&state);
    let range = place.range.clone().expect("a range");
    let (start, end) = range.lines();
    assert!(start <= LINE && LINE <= end, "{range:?} holds line 22");
    assert!(
        matches!(range, LineRange::Function { .. }),
        "a function: {range:?}"
    );
    let selected = place.selected_row().expect("a commit selected");
    assert_eq!(selected.id, lina, "the commit blame showed");
    assert!(
        matches!(place.file, FileLoad::Ready { commit, .. } if commit == lina),
        "Lina's file: {:?}",
        place.file
    );
    assert!(
        place.range_at().is_some(),
        "the range where it sits in hers"
    );
    app.close();
}

/// `file-history`: `F` on the file's row — every commit that changed the
/// file, following Priya's rename, the newest selected, the pane at the top.
#[test]
fn file_history() {
    let scratch = session();
    let repo = scratch.path();
    let (app, state) = open_on_the_file(repo, 140, 40, false);

    let mut route = Route::new("file-history");
    let state = route.press(&app, &state, "F", KeyCode::Char('F'));
    let state = traced(&app, state);
    route.within_count();
    assert_drawn(&state, 140, 40);

    assert!(!state.focus, "not Focus mode");
    let place = history(&state);
    assert_eq!(place.path, Path::new(FILE));
    assert_eq!(place.selected, Some(0), "the newest");
    assert_eq!(place.file_top, 0, "a file row opens at the top");
    let rows = place.rows();
    assert_eq!(rows.len(), 6, "every commit that touched it: {rows:?}");
    assert!(
        rows.iter()
            .any(|row| row.renamed_from.as_deref() == Some(Path::new("src/session.rs"))),
        "Priya's rename says was: {rows:?}"
    );
    assert!(
        matches!(place.file, FileLoad::Ready { .. }),
        "{:?}",
        place.file
    );
    app.close();
}
