//! Strips the pixel tier draws: whole, aligned, and painted like the text.

use graft_core::commit::{Commit, Identity, Timestamp};
use graft_core::oid::Oid;
use graft_graph::layout::layout;
use graft_graph::raster::cache::{STRIP_ROWS, StripCache};
use graft_graph::raster::{Emphasis, Ring};
use graft_graph::row::Row;
use graft_graph::seeds::Seeds;

use super::{PixelRows, StripPaint, pixel_rows};
use crate::history::{History, Phase, Window};
use crate::session::for_tests::session;
use crate::update::{AppState, CHROME_ROWS, Size};
use crate::view::tests::{counts, history_of, oid, refs_with_head};

const MAX_LANES: usize = 12;

fn state(history: History, graph_rows: u16) -> AppState {
    AppState {
        history: Some(history),
        size: Size {
            columns: 80,
            rows: graph_rows + CHROME_ROWS,
        },
        ..AppState::default()
    }
}

fn scrolled(mut history: History, top: usize) -> History {
    history.top = top;
    history.cursor = top;
    history
}

fn firsts_and_lengths(rows: &PixelRows) -> Vec<(usize, usize)> {
    rows.strips
        .iter()
        .map(|strip| (strip.first_row, strip.row_count()))
        .collect()
}

/// Every strip is rasterised through the cache, which refuses misaligned ones.
fn rasterise_all(rows: &PixelRows) {
    let paint = StripPaint::of(&session());
    let mut cache = StripCache::new(STRIP_ROWS, 8);
    for strip in &rows.strips {
        let frame = strip
            .with_strip(rows, &paint, |drawn| cache.get(strip.first_row, drawn))
            .unwrap_or_else(|why| panic!("strip at {}: {why}", strip.first_row));
        assert_eq!(frame.rows, strip.row_count());
    }
}

#[test]
fn a_viewport_starting_mid_strip_yields_whole_aligned_strips() {
    let state = state(scrolled(history_of(120, 0), 50), 20);

    let rows = pixel_rows(&state, MAX_LANES, STRIP_ROWS);

    // Rows 50..70 straddle strips 32..64 and 64..96: both whole.
    assert_eq!(firsts_and_lengths(&rows), vec![(32, 32), (64, 32)]);
    rasterise_all(&rows);
}

#[test]
fn the_last_strip_of_history_is_short() {
    let state = state(scrolled(history_of(100, 0), 90), 20);

    let rows = pixel_rows(&state, MAX_LANES, STRIP_ROWS);

    assert_eq!(firsts_and_lengths(&rows), vec![(64, 32), (96, 4)]);
    rasterise_all(&rows);
}

#[test]
fn a_strip_whose_rows_or_row_above_have_not_arrived_is_left_out() {
    // Rows 80..100 straddle strips 64..96 and 96..100.
    let mut history = scrolled(history_of(100, 0), 80);
    let window = history.window.as_mut().expect("history_of sends a window");
    // Rows 64..100 only: strip 64 lacks the row above it.
    window.rows = window.rows[64..].into();
    window.first = 64;

    let rows = pixel_rows(&state(history, 20), MAX_LANES, STRIP_ROWS);

    assert_eq!(firsts_and_lengths(&rows), vec![(96, 4)]);
}

#[test]
fn the_uncommitted_row_sits_on_head_in_its_strip() {
    let mut state = state(history_of(3, 0), 20);
    state.refs = Some(refs_with_head(Some(oid(3))));
    state.working_tree = Some(counts(1, 0, 0));

    let rows = pixel_rows(&state, MAX_LANES, STRIP_ROWS);

    assert_eq!(firsts_and_lengths(&rows), vec![(0, 4)]);
    let strip = &rows.strips[0];
    assert_eq!(strip.uncommitted_at(), Some(0));
    assert_eq!(strip.rows[1].ring, Ring::Head);
    assert_eq!(strip.rows[1].row.commit.id, oid(3));
    rasterise_all(&rows);

    state.working_tree = Some(counts(0, 0, 0));
    let clean = pixel_rows(&state, MAX_LANES, STRIP_ROWS);
    assert_eq!(clean.strips[0].uncommitted_at(), None);
}

/// `chains` parallel lines of `length` commits under one merge at the top,
/// laid out as a complete history in one window.
fn fan(chains: u8, length: u8) -> History {
    let id = |chain: u8, index: u8| Oid::from_bytes(&[chain, index].repeat(10)).expect("20 bytes");
    let who = Identity {
        name: b"Test".to_vec(),
        email: b"test@example.invalid".to_vec(),
        when: Timestamp {
            seconds: 0,
            offset_seconds: 0,
        },
    };
    let commit = |id, parents| Commit {
        id,
        parents,
        author: who.clone(),
        committer: who.clone(),
        message: b"row".to_vec(),
    };
    let mut commits = vec![commit(id(0, 0), (1..=chains).map(|c| id(c, 1)).collect())];
    for chain in 1..=chains {
        for index in 1..=length {
            let parents = if index < length {
                vec![id(chain, index + 1)]
            } else {
                Vec::new()
            };
            commits.push(commit(id(chain, index), parents));
        }
    }
    let rows: Vec<Row> = layout(&commits, Seeds::default());
    History {
        window: Some(Window {
            request: 0,
            first: 0,
            laid_out: rows.len(),
            done: true,
            rows: rows.clone().into(),
            promoted: false,
        }),
        rows: rows.len(),
        phase: Phase::Complete,
        ..History::started()
    }
}

#[test]
fn one_fold_spans_every_strip_and_its_marker_reaches_the_strip() {
    // Rows 32..64 hold nodes of chains 1 and 2 only; chains 3..5 pass by.
    let mut state = state(scrolled(fan(5, 40), 40), 20);

    let folded = pixel_rows(&state, 3, STRIP_ROWS);
    assert_eq!(firsts_and_lengths(&folded), vec![(32, 32)]);
    let marker = folded.marker.expect("lanes past the limit fold");
    assert!(marker.folded > 0);
    assert!(folded.lane_columns <= 3, "{}", folded.lane_columns);
    let paint = StripPaint::of(&session());
    folded.strips[0].with_strip(&folded, &paint, |strip| {
        assert_eq!(strip.marker, Some(marker));
        assert_eq!(strip.lane_columns, folded.lane_columns);
    });
    rasterise_all(&folded);

    state.lanes_expanded = true;
    let expanded = pixel_rows(&state, 3, STRIP_ROWS);
    assert_eq!(expanded.marker, None);
    assert!(expanded.lane_columns > 3);
}

#[test]
fn the_selection_thread_and_focus_reach_the_strip() {
    // Cursor on row 35, in chain 1 (rows 1..=40).
    let mut state = state(scrolled(fan(2, 40), 35), 20);
    state.focus = true;

    let rows = pixel_rows(&state, MAX_LANES, STRIP_ROWS);

    let strip = &rows.strips[0];
    assert_eq!(strip.first_row, 32);
    let cursor = &strip.rows[35 - 32];
    assert_eq!(cursor.emphasis, Emphasis::Highlighted);
    let lit = cursor
        .thread
        .as_ref()
        .expect("a selection lights its thread");
    assert!(lit.contains(&cursor.row.lane), "{lit:?}");
    // Chain 2's rows are off the thread: faded under Focus.
    let unrelated = &strip.rows[50 - 32];
    assert_eq!(unrelated.emphasis, Emphasis::Dimmed);
    assert!(
        !unrelated
            .thread
            .as_ref()
            .expect("thread set")
            .contains(&unrelated.row.lane)
    );

    state.focus = false;
    let unfocused = pixel_rows(&state, MAX_LANES, STRIP_ROWS);
    assert_eq!(unfocused.strips[0].rows[50 - 32].emphasis, Emphasis::Normal);
}

#[test]
fn every_strip_waits_while_the_cursor_row_has_not_arrived() {
    // A page key moved the viewport to 40..120 and the cursor to row 110;
    // the window on hand still ends at row 100. Strips 32 and 64 have their
    // rows, but without the cursor's row there is no thread to light them:
    // drawn now, they would be rasterised and sent unlit, then again once the
    // window arrives and the thread lights them (30b).
    let mut history = scrolled(history_of(200, 0), 40);
    history.cursor = 110;
    let window = history.window.as_mut().expect("history_of sends a window");
    window.rows = window.rows[..100].into();
    window.first = 0;

    let rows = pixel_rows(&state(history, 80), MAX_LANES, STRIP_ROWS);

    assert_eq!(firsts_and_lengths(&rows), vec![]);
}

#[test]
fn strips_draw_when_the_wheel_has_left_the_cursor_row_behind() {
    // The wheel moves the viewport and leaves the selection alone (37): the
    // viewport is 120..200, the cursor stayed on row 5, and the window on
    // hand is 100..200. The cursor's row is not on its way, so waiting for it
    // would blank the graph for as long as the user stays scrolled away.
    let mut history = scrolled(history_of(200, 0), 120);
    history.cursor = 5;
    let window = history.window.as_mut().expect("history_of sends a window");
    window.rows = window.rows[100..].into();
    window.first = 100;

    let rows = pixel_rows(&state(history, 80), MAX_LANES, STRIP_ROWS);

    // Strip 96 lacks rows 96..100; the rest draw, unlit.
    assert_eq!(
        firsts_and_lengths(&rows),
        vec![(128, 32), (160, 32), (192, 8)]
    );
    rasterise_all(&rows);
}
