//! The landing's last two rows (ease-pass/08s M4, mock
//! `mergewright-commit-screen.html:178-179`): after a commit the done line
//! takes its own row directly above the bar, and the bar says what `u`
//! gives back; the graph gives up that row while the line stands.

use mergewright_core::event::{Event, KeyCode, KeyPress, Modifiers};

use crate::update::commit_editor::{Draft, LastCommit};
use crate::update::{AppEvent, AppState, Size, Status};
use crate::view::tests::{draw, history_of, oid};

const DONE: &str = "✓ Committed 3 files to main.  u undo · P push";
const BAR: &str = "u undo the commit, your edits come back · P push · c commit again";

/// Five commits, the cursor on the newest — the one just made — and the
/// commit's done line standing.
fn landed(columns: u16, rows: u16) -> AppState {
    AppState {
        history: Some(history_of(5, 0)),
        size: Size { columns, rows },
        status: Status::done_with_keys("Committed 3 files to main.", "u undo · P push"),
        last_commit: Some(Box::new(LastCommit {
            commit: oid(5),
            message: Draft {
                subject: "fix toc".to_owned(),
                ..Draft::default()
            },
        })),
        ..AppState::default()
    }
}

/// The frame's rows, each trimmed.
fn rows(state: &AppState) -> Vec<String> {
    draw(state, state.size.columns, state.size.rows)
        .lines()
        .map(|line| line.trim_matches('"').trim().to_owned())
        .collect()
}

fn last_two(state: &AppState) -> (String, String) {
    let rows = rows(state);
    let at = rows.len();
    (rows[at - 2].clone(), rows[at - 1].clone())
}

#[test]
fn a_commit_landing_draws_the_done_line_above_its_bar() {
    for (columns, height) in [(120, 40), (80, 24)] {
        let state = landed(columns, height);
        let (second_last, last) = last_two(&state);
        assert_eq!(second_last, DONE, "{columns}x{height}");
        // The landing's words lead; the doorway tail every bar ends in
        // follows where it fits.
        if columns == 120 {
            assert!(last.starts_with(BAR), "{columns}x{height}: {last}");
        } else {
            assert!(last.starts_with("u undo"), "{columns}x{height}: {last}");
        }
        // The graph gave the row up: one fewer graph row than with the line
        // gone, and the scrollers count what is drawn.
        let cleared = AppState {
            status: Status::Ready,
            ..state.clone()
        };
        assert_eq!(
            state.graph_pane_rows() + 1,
            cleared.graph_pane_rows(),
            "{columns}x{height}"
        );
    }
}

/// With the detail pane open (below the graph at 120×40, beside it at
/// 160×48) the two rows are the graph half's: the done line, the bar
/// directly under it.
#[test]
fn with_the_detail_pane_open_the_landing_rows_are_the_graph_halfs() {
    for (columns, height) in [(120, 40), (160, 48)] {
        let mut state = landed(columns, height);
        state.detail.shown = true;
        let rows = rows(&state);
        let at = rows
            .iter()
            .position(|row| row.starts_with(DONE))
            .unwrap_or_else(|| panic!("{columns}x{height}: no done line in {rows:#?}"));
        assert!(
            rows[at + 1].starts_with("u undo"),
            "{columns}x{height}: {rows:#?}"
        );
        let cleared = AppState {
            status: Status::Ready,
            ..state.clone()
        };
        assert_eq!(state.graph_pane_rows() + 1, cleared.graph_pane_rows());
    }
}

/// Any key clears a done line; with it the row goes back to the graph and
/// the bar stands alone at the bottom.
#[test]
fn a_key_that_clears_the_done_line_gives_the_row_back() {
    let state = landed(120, 40);
    let (after, _) = crate::update::update(
        &state,
        AppEvent::Core(Event::Key(KeyPress {
            code: KeyCode::Escape,
            modifiers: Modifiers::NONE,
        })),
    );
    assert_eq!(after.status, Status::Ready, "the key cleared it");
    let rows = rows(&after);
    assert!(rows[39].starts_with(BAR), "{:?}", rows[39]);
    assert!(
        rows[38].starts_with('─'),
        "the rule, then the bar: {:?}",
        rows[38]
    );
    assert_eq!(after.graph_pane_rows(), state.graph_pane_rows() + 1);
}

/// A done line with no landing bar under it — a write that is not the
/// commit, or the cursor moved off it — keeps the bar's row, as before.
#[test]
fn a_done_line_off_a_landing_bar_stays_in_the_bars_row() {
    let staged = AppState {
        last_commit: None,
        status: Status::done("staged src/a.rs"),
        ..landed(120, 40)
    };
    let off = AppState {
        history: Some(history_of(5, 2)),
        ..landed(120, 40)
    };
    for state in [staged, off] {
        let rows = rows(&state);
        assert!(rows[39].starts_with('✓'), "{:?}", rows[39]);
        assert!(rows[38].starts_with('─'), "{:?}", rows[38]);
    }
}
