//! m8-polish-and-ship/06 (ADR-0058 §4, DESIGN.md §1.0): the graph screen's
//! chrome like the mock-up — the header names the repository, its `~` path
//! and the branch; list rows keep a two-cell gutter and a right margin; from
//! 30 rows a blank row sits under the header and a rule over the bar.
//!
//! Two things are checked against the real app (real repository, real
//! update loop): the geometry the mouse uses agrees with what is drawn at
//! every designed size, and the frames are captured for the user's eye.
//! Colour is **unverified** (CLAUDE.md rule 5): the captures are grids.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use graft::session::Clock;
use graft::update::AppState;
use graft_core::event::MouseButton;

use crate::app_harness::App;
use crate::captures::{check_shot, frame_at, radar_has_answered, settled_frame};

/// The sizes the ticket names: the everyday editor terminal, the mock-up's
/// own, and a wide one.
const SIZES: [(u16, u16); 3] = [(80, 24), (120, 36), (187, 50)];

/// Long after the fixture's commits, so their ages read the same every run.
const NOW: i64 = 1_800_000_000;

/// A feature-branch history, opened and settled at `columns`x`rows`.
fn opened(columns: u16, rows: u16) -> (graft_fixtures::Scratch, App, AppState) {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::FeatureBranches);
    let (app, state) = App::open(scratch.path());
    let state = app.resize(&state, columns, rows);
    let state = settled_frame(&app, state);
    // Rows, refs (the rows' branch badges) and the radar (its marks) each
    // land on their own job; a frame taken before all three is a frame that
    // differs from the next run's.
    let state = app.pump(state, "the first rows, the refs and the radar", |state| {
        state
            .history
            .as_ref()
            .and_then(|history| history.window.as_ref())
            .is_some_and(|window| !window.rows.is_empty())
            && state.refs.is_some()
            && radar_has_answered(state)
    });
    let state = settled_frame(&app, state);
    // `settled_frame` reads a 120-column frame, whose header may have no
    // room for the walk's `sorting…`; read this size's own.
    let state = app.pump(state, "nothing loading at this size", |state| {
        frame_at(state, columns, rows, Some(Clock::Fixed(NOW)))
            .iter()
            .all(|row| !row.contains("reading\u{2026}") && !row.contains("sorting\u{2026}"))
    });
    (scratch, app, state)
}

/// The selected row's subject.
fn selected_subject(state: &AppState) -> Option<String> {
    state
        .history
        .as_ref()
        .and_then(graft::history::History::selected)
        .map(|row| {
            String::from_utf8_lossy(&row.commit.message)
                .lines()
                .next()
                .unwrap_or_default()
                .to_owned()
        })
}

/// Acceptance (geometry sweep): at 24×80, 36×120 and 50×187 a click on
/// every drawn graph row selects the commit drawn on that row — the mouse's
/// row math (`update::geometry::rows_above_graph`, `graph_pane_rows`) and
/// the widget's carve (`ScreenRows`, gap and rule included) agree.
#[test]
fn every_drawn_row_is_the_row_a_click_selects_at_every_designed_size() {
    for (columns, rows) in SIZES {
        let (_scratch, app, state) = opened(columns, rows);
        let drawn = frame_at(&state, columns, rows, Some(Clock::Fixed(NOW)));
        let subjects = [
            "three: in progress",
            "two: finish",
            "two: start",
            "one: finish",
            "one: start",
            "Add b.txt",
            "Add a.txt",
            "Add a readme",
        ];
        let mut checked = 0;
        for (y, line) in drawn.iter().enumerate() {
            let Some(subject) = subjects.iter().find(|subject| line.contains(**subject)) else {
                continue;
            };
            let row = u16::try_from(y).expect("a screen is shorter than u16::MAX");
            // Inside the subject column, clear of the gutter and the lanes.
            let clicked = app.mouse_down(&state, MouseButton::Left, 30, row);
            assert_eq!(
                selected_subject(&clicked).as_deref(),
                Some(*subject),
                "{columns}x{rows}: a click on row {row} ({line:?})\n{}",
                drawn.join("\n")
            );
            checked += 1;
        }
        assert!(
            checked >= 5,
            "{columns}x{rows}: only {checked} rows drawn\n{}",
            drawn.join("\n")
        );
        // From 30 rows the second row is blank and the row over the bar is
        // the rule; below, neither is drawn.
        let spaced = graft_ui::graph_screen::spaced(rows);
        assert_eq!(drawn[1].trim().is_empty(), spaced, "{}", drawn.join("\n"));
        let over_the_bar = &drawn[drawn.len() - 2];
        assert_eq!(
            over_the_bar.trim_start().starts_with('\u{2500}'),
            spaced,
            "{columns}x{rows}: {over_the_bar:?}"
        );
        drop(state);
        app.close();
    }
}

/// J22 (0 steps) and the ticket's captures: the first row names the
/// repository, its `~` path and the branch, and never the product.
#[test]
fn capture_the_chrome_at_every_designed_size() {
    for (columns, rows) in SIZES {
        let (_scratch, app, state) = opened(columns, rows);
        let drawn = frame_at(&state, columns, rows, Some(Clock::Fixed(NOW)));
        let header = &drawn[0];
        assert!(
            header.starts_with(" \u{25c6} api  "),
            "the repository's name leads: {header:?}"
        );
        if columns >= 120 {
            assert!(
                header.contains("~/code/api \u{b7} main"),
                "{columns}x{rows}: the `~` path and the branch: {header:?}"
            );
        }
        assert!(!header.contains("graft"), "no product name: {header:?}");
        let caption = format!(
            "m8-polish-and-ship/06 at {columns}x{rows}: what right looks like — the first row \
             is ` ◆ <repo>  ~/<path> · <branch>` with the walk state right-aligned; list rows \
             start two cells in and stop one short of the edge; the bar joins its hints with \
             ` · ` and ends `. more`; from 30 rows a blank row sits under the header and a `─` \
             rule over the bar. Colour unverified (rule 5).\n\n"
        );
        check_shot(
            &format!("m8-look-chrome-{columns}x{rows}.txt"),
            &format!("{caption}{}\n", drawn.join("\n")),
        );
        drop(state);
        app.close();
    }
}
