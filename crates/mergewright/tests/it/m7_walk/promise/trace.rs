//! Trace: the session fixture, from the diff pane on line 22 of
//! `src/auth/session.rs` at `main` (Lina's line).
//!
//! 1. `B` (1): blame; the cursor row's trailer names Lina.
//! 2. `^` (1): blame before Lina's commit, the cursor on the old line (Tom's,
//!    20). Steps 1–2 are `blame-line-before` (2).
//! 3. `Esc L` (2): back to her blame, then the function's history — Lina's row
//!    selected, the file pane marking her added line 18 `+` (the ticket's
//!    "19" is context in her file; see 26's NOTES). `function-history` is `B
//!    L`; here `Esc` stands in for its `B`.
//! 4. `⏎` (1): the graph on her commit (as `blame-line-to-commit`'s `⏎`).
//! 5. `Esc Esc Esc` (3): history, blame, then the diff pane — same line, same
//!    graph cursor (`commit-back-to-blame` 1, and "`L`/`F`, then `Esc`: back
//!    where he pressed it", twice).
//!
//! Total 2 + 2 + 1 + 3 = 8 keys.

use mergewright::update::AppState;
use mergewright::update::detail::{Side, diff_cursor};
use mergewright_core::event::KeyCode;
use mergewright_ui::detail::DetailFocus;

use super::super::blame::{
    CURSOR_ROW, FILE, LINE, TRAILER, blame_ready, cursor_author, cursor_line, drawn, graph_cursor,
    oid, on_line_22, session,
};
use super::super::history::{history, traced};
use crate::app_harness::App;

/// The scenario's keys: 2 + 2 + 1 + 3.
const TOTAL: usize = 8;

fn press(
    app: &App,
    state: &AppState,
    route: &mut Vec<&'static str>,
    label: &'static str,
    code: KeyCode,
) -> AppState {
    route.push(label);
    app.key(state, code)
}

fn body(state: &AppState) -> String {
    drawn(state, 187, 50).join("\n")
}

#[test]
fn promise_trace_without_leaving() {
    let scratch = session();
    let repo = scratch.path();
    let lina = oid(repo, "main~3");
    let main = oid(repo, "main");
    let (app, state) = on_line_22(repo, 187, 50);
    let start_cursor = graph_cursor(&state);
    assert_eq!(start_cursor, Some(main), "the graph on main");
    let mut route: Vec<&'static str> = Vec::new();

    // 1. `B`: the trailer names Lina.
    let state = press(&app, &state, &mut route, "B", KeyCode::Char('B'));
    let state = blame_ready(&app, state);
    assert_eq!(cursor_line(&state).final_line, LINE);
    let frame = drawn(&state, 187, 50);
    assert!(
        frame
            .iter()
            .any(|row| row.contains(CURSOR_ROW) && row.contains(&format!("{TRAILER} Lina"))),
        "the cursor row's trailer names Lina:\n{}",
        frame.join("\n")
    );

    // 2. `^`: before Lina's commit, the cursor on the old line.
    let state = press(&app, &state, &mut route, "^", KeyCode::Char('^'));
    let state = blame_ready(&app, state);
    assert_eq!(cursor_line(&state).final_line, 20, "Tom's old line");
    assert_eq!(cursor_author(&state), "Tom");
    let short = &lina.to_string()[..7];
    assert!(
        drawn(&state, 187, 50)[0].contains(&format!("before {short}")),
        "the title names the commit it is before:\n{}",
        body(&state)
    );

    // 3. `Esc L`: her blame again, then the function's history.
    let state = function_history(&app, &state, &mut route, lina);

    // 4. `⏎`: the graph on her commit.
    let state = press(&app, &state, &mut route, "⏎", KeyCode::Enter);
    let state = app.pump(state, "the graph on Lina's commit", |state| {
        graph_cursor(state) == Some(lina) && state.trail.hidden()
    });

    // 5. `Esc Esc Esc`: history, blame, the diff pane.
    let state = press(&app, &state, &mut route, "Esc", KeyCode::Escape);
    let state = traced(&app, state);
    let state = press(&app, &state, &mut route, "Esc", KeyCode::Escape);
    let state = press(&app, &state, &mut route, "Esc", KeyCode::Escape);
    let state = app.pump(state, "the diff pane on line 22", |state| {
        state.trail.places.is_empty()
            && diff_cursor(state).is_some_and(|cursor| {
                cursor.path == std::path::Path::new(FILE)
                    && cursor
                        .at
                        .is_some_and(|at| at.side == Side::New && at.line == LINE)
            })
    });
    assert_eq!(state.detail.focus, DetailFocus::Diff, "the diff pane");
    assert_eq!(graph_cursor(&state), start_cursor, "the same graph cursor");
    assert_eq!(route.len(), TOTAL, "the scenario's keys: {route:?}");
    app.close();
}

/// Step 3, `Esc L`: back on her blame at line 22, then the function's
/// history with Lina's row selected.
fn function_history(
    app: &App,
    state: &AppState,
    route: &mut Vec<&'static str>,
    lina: mergewright_core::oid::Oid,
) -> AppState {
    let state = press(app, state, route, "Esc", KeyCode::Escape);
    assert_eq!(state.trail.places.len(), 1, "popped back to her blame");
    assert_eq!(cursor_line(&state).final_line, LINE, "on line 22 again");
    let state = press(app, &state, route, "L", KeyCode::Char('L'));
    let state = traced(app, state);
    let place = history(&state);
    assert_eq!(
        place.selected_row().map(|row| row.id),
        Some(lina),
        "Lina's row selected"
    );
    // Her file on the right, the range ruled `┃`, her added lines marked `+`.
    // The ticket says line 19; her commit adds 18, 21 and 22 (`+3 −2`), and
    // 19 (`loop {`) is context — asserted as drawn, the 19 filed as a
    // question in 26's NOTES.
    let frame = drawn(&state, 187, 50);
    let marked: Vec<&String> = frame
        .iter()
        .filter(|row| row.contains("\u{2503}  18 +"))
        .collect();
    assert_eq!(
        marked.len(),
        1,
        "the file pane marks line 18 `+` inside the range:\n{}",
        frame.join("\n")
    );
    assert!(
        frame.iter().any(|row| row.contains("\u{2503}  19  ")),
        "19 is context, unmarked:\n{}",
        frame.join("\n")
    );
    state
}
