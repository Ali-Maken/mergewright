//! Strips the pixel tier draws: whole, aligned, and painted like the text.

use graft_core::commit::{Commit, Identity, Timestamp};
use graft_core::oid::Oid;
use graft_graph::Fold;
use graft_graph::fold::Keep;
use graft_graph::layout::layout;
use graft_graph::raster::cache::{STRIP_ROWS, StripCache};
use graft_graph::raster::{Backdrop, Emphasis, Ring};
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

/// Every strip is rasterised through the cache, which refuses misaligned
/// ones: its backdrop and the selection's emphasis over it.
fn rasterise_all(rows: &PixelRows) {
    let paint = StripPaint::of(&session());
    let mut cache = StripCache::new(STRIP_ROWS, 8);
    for strip in &rows.strips {
        let (frame, emphasis) = strip
            .with_layers(rows, &paint, |plain, selection| {
                let frame = cache.backdrop(strip.first_row, plain, selection.backdrop)?;
                let emphasis = cache.emphasis(strip.first_row, plain, selection)?;
                Ok::<_, graft_graph::raster::cache::CacheError>((frame, emphasis))
            })
            .unwrap_or_else(|why| panic!("strip at {}: {why}", strip.first_row));
        assert_eq!(frame.rows, strip.row_count());
        if let Some(top) = emphasis {
            assert_eq!(top.frame.rows, strip.row_count());
        }
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
    state.refs = Some(std::sync::Arc::new(refs_with_head(Some(oid(3)))));
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
pub(crate) fn fan(chains: u8, length: u8) -> History {
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

/// A graph wider than the lane cap whose fold follows the selection
/// (ADR-0025 Decision 2, class 5): one line of `length` commits on lane 0,
/// whose first `merges` rows are merges. Merge `i`'s second parent is the
/// line's row `reach - i`, so its side lane opens at row `i` and closes into
/// lane 0 at row `reach - i`: nested lanes `1..=merges`, none holding a node.
///
/// With the cursor on merge `i`, the thread lights the side lanes of merges
/// `i..` (`Ancestry::columns`) and lane 0, the selected column, never
/// changes: a `j` drops one lane from `preferred`, which is what decides
/// which side lanes the fold keeps. At 120 columns the cap is
/// `max_lanes_for` (10 or fewer); `merges = 16` opens 17 lanes on rows
/// `merges..reach - merges`. Pick `reach - merges` inside the first two
/// strips so every lane closes in the drawn window.
pub(crate) fn capped(merges: u8, reach: u8, length: u8) -> History {
    assert!(merges < reach - merges && reach < length, "lanes nest");
    let id = |index: u8| Oid::from_bytes(&[0xc0, index].repeat(10)).expect("20 bytes");
    let who = Identity {
        name: b"Test".to_vec(),
        email: b"test@example.invalid".to_vec(),
        when: Timestamp {
            seconds: 0,
            offset_seconds: 0,
        },
    };
    let commits: Vec<Commit> = (0..length)
        .map(|index| {
            let mut parents = Vec::new();
            if index + 1 < length {
                parents.push(id(index + 1));
            }
            if index < merges {
                parents.push(id(reach - index));
            }
            Commit {
                id: id(index),
                parents,
                author: who.clone(),
                committer: who.clone(),
                message: b"row".to_vec(),
            }
        })
        .collect();
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

/// The fold `pixel_rows` draws `state` through and the [`Keep`] it weighed,
/// read through `plan` itself (m7-power-tools/46c). `None` when
/// `pixel_rows` draws nothing. No uncommitted row: the fixtures that use
/// this have none (asserted).
pub(crate) fn fold_plan(state: &AppState, max_lanes: usize) -> Option<(Fold, Keep)> {
    assert!(!state.uncommitted_row_shown(), "fold_plan: no virtual row");
    let history = state.history.as_ref()?;
    let window = history.window.as_ref()?;
    let first = window.first;
    let on_hand = first..first + window.rows.len();
    let ranges = super::strip_ranges(
        history.top..history.top + state.graph_rows(),
        &on_hand,
        STRIP_ROWS,
    );
    if ranges.is_empty() {
        return None;
    }
    let row_at = |index: usize| &window.rows[index - first];
    let thread = super::thread(history);
    let fold = super::plan(
        state,
        history,
        thread.as_ref(),
        &ranges,
        None,
        row_at,
        max_lanes,
    );

    // The same `Keep` `plan` builds, rebuilt here to say which class moved;
    // checked against `plan`'s fold below so the two cannot drift.
    let indices: Vec<usize> = ranges.iter().flat_map(Clone::clone).collect();
    let drawn: Vec<Row> = indices.iter().map(|&index| row_at(index).clone()).collect();
    let cursor = indices.iter().position(|&index| index == history.cursor);
    let integration = history
        .walk
        .as_ref()
        .map_or(0, |walk| walk.integration.len());
    let reserved = (1..=integration).collect();
    let keep = super::keep_for(
        &drawn,
        cursor,
        &reserved,
        thread.as_ref().map(|thread| &thread.ancestry),
    );
    let cap = if state.lanes_expanded {
        usize::MAX
    } else {
        max_lanes
    };
    assert_eq!(
        Fold::plan(cap, &keep, &drawn),
        fold,
        "fold_plan's Keep is plan's"
    );
    Some((fold, keep))
}

#[test]
fn one_fold_spans_every_strip_and_its_marker_reaches_the_strip() {
    // Rows 32..64 hold nodes of chains 1 and 2 only; chains 3..5 pass by.
    let mut state = state(scrolled(fan(5, 40), 40), 20);

    let folded = pixel_rows(&state, 3, STRIP_ROWS);
    assert_eq!(firsts_and_lengths(&folded), vec![(32, 32)]);
    let marker = folded.marker.expect("lanes past the limit fold");
    assert_eq!(
        marker.column + 1,
        folded.lane_columns,
        "the rail is the last drawn column"
    );
    assert!(folded.lane_columns <= 3, "{}", folded.lane_columns);
    let paint = StripPaint::of(&session());
    folded.strips[0].with_layers(&folded, &paint, |strip, _| {
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
    // The cursor's own row carries no row-level emphasis: it is on the
    // thread, which the rasteriser draws at full alpha regardless
    // (ADR-0024).
    assert_eq!(cursor.emphasis, Emphasis::Normal);
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

/// m7-power-tools/16: a commit only a stash reaches is dimmed in its strip,
/// as its text row is; its neighbours are not. Under a selection the
/// rasteriser draws the thread's alpha instead of the row's
/// (`graft_graph::raster::Lit::alpha_at`), so the dimming shows only
/// with no thread in play — pre-existing, the same as Focus mode's. With
/// none, the frame's backdrop is `Faded` for it (ADR-0053, 44 ruling 2).
#[test]
fn a_stash_only_commit_is_dimmed_in_its_strip() {
    let mut state = state(scrolled(history_of(40, 0), 0), 20);
    let marked = state
        .history
        .as_ref()
        .and_then(|history| history.window.as_ref())
        .expect("history_of sends a window")
        .rows[5]
        .commit
        .id;
    state.stash_only.commits = std::sync::Arc::new([(marked, 0)].into());

    let rows = pixel_rows(&state, MAX_LANES, STRIP_ROWS);

    let strip = &rows.strips[0];
    assert_eq!(strip.first_row, 0);
    assert_eq!(strip.rows[5].lit().emphasis, Emphasis::Dimmed);
    assert_eq!(strip.rows[4].lit().emphasis, Emphasis::Normal);
    assert_eq!(strip.rows[6].lit().emphasis, Emphasis::Normal);
    // The cursor's thread is in play and overrides the row's own dimming:
    // nothing draws at `Faded` (`pixel_scroll::stash_only_rows_stay_dimmed_
    // without_a_thread` has it without one).
    assert_eq!(rows.backdrop, Backdrop::OffThread);
    rasterise_all(&rows);
}
