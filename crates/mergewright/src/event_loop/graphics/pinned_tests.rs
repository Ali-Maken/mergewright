//! The pinned uncommitted row's image on screen (ease-pass/20): its own
//! image on the column's first line, every strip lowered one row, the
//! cursor row's second line still making room, on Kitty and iTerm2.

use std::collections::BTreeSet;
use std::sync::Arc;

use mergewright_core::capabilities::PixelProtocol;
use mergewright_graph::raster::STRIP_LEAD_CELLS;
use mergewright_graph::raster::cache::STRIP_ROWS;
use mergewright_term::kitty::Layer;
use ratatui::layout::Rect;

use super::place::PINNED_SLOT;
use super::*;
use crate::session::for_tests::session;
use crate::update::{Size, history_row_line};
use crate::view::tests::{counts, history_of, oid, refs_with_head};

/// A dirty tree over `history_of(100)` on a 80x24 screen, `HEAD` (row 0)
/// scrolled off: the viewport at `top` with the cursor on `cursor`.
fn pinned_state(top: usize, cursor: usize) -> AppState {
    let mut history = history_of(100, 0);
    history.strip_rows = Some(STRIP_ROWS);
    history.top = top;
    history.cursor = cursor;
    let state = AppState {
        history: Some(history),
        refs: Some(Arc::new(refs_with_head(Some(oid(100))))),
        working_tree: Some(counts(1, 0, 0)),
        settled_tier: Some(RenderingTier::Pixel),
        size: Size {
            columns: 80,
            rows: 24,
        },
        ..AppState::default()
    };
    assert!(state.uncommitted_row_pinned() && !state.uncommitted_row_attached());
    state
}

fn pixel_session(protocol: PixelProtocol) -> Session {
    let mut session = session();
    session.rendering.tier = RenderingTier::Pixel;
    session.capabilities.pixel_protocol = Some(protocol);
    session
}

/// What a frame draws: the `Drawn` strips and the cursor row's band.
fn drawn_of(
    state: &AppState,
    session: &Session,
    iterm2: bool,
) -> (Graphics<NoReplies>, Vec<Drawn>, Option<Band>) {
    let mut graphics = Graphics::new(NoReplies, &session.capabilities);
    let pixels = pixel_rows(state, crate::view::max_lanes(state), STRIP_ROWS);
    let paint = StripPaint::for_frame(session, state);
    let height = u16::try_from(state.graph_pane_rows()).expect("small");
    let (drawn, band) = graphics.drawn(state, (&pixels, &paint), height, iterm2);
    (graphics, drawn, band)
}

/// The graph column's first line on screen: where the pinned row goes. Read
/// with the cursor well below `top`, so no second line sits above it.
fn column_top(state: &AppState) -> u16 {
    let mut state = state.clone();
    state.on_working_tree = false;
    let history = state.history.as_mut().expect("history");
    let top = history.top;
    history.cursor = top + 5;
    history_row_line(&state, top).expect("top is on screen") - 1
}

/// The checks both cursor placements share: the pinned row on the column's
/// first line in its own slot, each history row on `history_row_line`, and no
/// two images on one line. Returns the line the band sits on, if any.
fn assert_lines_agree(state: &AppState, drawn: &[Drawn], band: Option<&Band>) -> Option<u16> {
    let column = column_top(state);
    let mut lines = BTreeSet::new();
    let mut claim = |line: u16, what: &str| {
        assert!(lines.insert(line), "{what}: two images on line {line}");
    };
    let pinned: Vec<&Drawn> = drawn
        .iter()
        .filter(|d| d.first_row == PINNED_SLOT)
        .collect();
    assert_eq!(pinned.len(), 1, "one pinned image");
    assert_eq!((pinned[0].rows.clone(), pinned[0].at), (0..1, 0));
    claim(column, "the pinned row");
    for strip in drawn.iter().filter(|d| d.first_row != PINNED_SLOT) {
        for (offset, row) in strip.rows.clone().enumerate() {
            let line = column + strip.at + u16::try_from(offset).expect("small");
            assert_eq!(
                history_row_line(state, strip.first_row + row),
                Some(line),
                "history row {} drawn on line {line}",
                strip.first_row + row
            );
            claim(line, "a strip row");
        }
    }
    let graph = Rect::new(
        0,
        column,
        80,
        u16::try_from(state.graph_pane_rows()).expect("small"),
    );
    band.map(|band| {
        let line = band.place(graph).cells.row;
        claim(line, "the continuation band");
        line - column
    })
}

#[test]
fn the_pinned_row_keeps_every_screen_line() {
    // Cursor mid-viewport: its second line falls inside a strip.
    let state = pinned_state(18, 24);
    let (_graphics, drawn, band) = drawn_of(&state, &pixel_session(PixelProtocol::Kitty), false);
    let line = assert_lines_agree(&state, &drawn, band.as_ref());
    // Pinned row on line 0, history row 18 on line 1, the cursor (row 24)
    // on line 7, its band under it.
    assert_eq!(line, Some(8));
    assert_eq!(
        history_row_line(&state, 18).map(|l| l - column_top(&state)),
        Some(1)
    );
}

#[test]
fn the_cursor_band_under_the_pinned_row_keeps_its_lines() {
    // Cursor on the top history row (pane line 1), its second line on line 2:
    // pinned image, the cursor strip's first part (row `top` only), the band,
    // the rest of the strip one line lower.
    let state = pinned_state(18, 18);
    let (_graphics, drawn, band) = drawn_of(&state, &pixel_session(PixelProtocol::Kitty), false);
    let line = assert_lines_agree(&state, &drawn, band.as_ref());
    assert_eq!(line, Some(2));
    let held = drawn.iter().filter(|d| d.first_row == 0).count();
    assert_eq!(held, 2, "the cursor's strip is placed twice");
    let first = drawn
        .iter()
        .find(|d| d.first_row == 0 && d.at == 1)
        .expect("part above");
    assert_eq!(first.rows, 18..19);
    let rest = drawn
        .iter()
        .find(|d| d.first_row == 0 && d.at == 3)
        .expect("part below");
    assert_eq!(rest.rows.start, 19);
}

#[test]
fn the_pinned_row_is_its_own_band() {
    let state = pinned_state(18, 18);
    let kitty = drawn_of(&state, &pixel_session(PixelProtocol::Kitty), false);
    let session = pixel_session(PixelProtocol::Iterm2);
    let (mut graphics, drawn, band) = drawn_of(&state, &session, true);
    let line = assert_lines_agree(&state, &drawn, band.as_ref());
    assert_eq!(line, Some(2));
    // The same placements as Kitty's: pinned row first, strips lowered alike.
    let shape = |drawn: &[Drawn]| -> Vec<(usize, std::ops::Range<usize>, u16)> {
        drawn
            .iter()
            .map(|d| (d.first_row, d.rows.clone(), d.at))
            .collect()
    };
    assert_eq!(shape(&drawn), shape(&kitty.1));
    let pinned = drawn
        .iter()
        .find(|d| d.first_row == PINNED_SLOT)
        .expect("pinned band");
    assert_eq!(pinned.frame.rows, 1, "a one-row band");
    // And the iTerm2 path takes it: one band for the pinned row, no error.
    let (area, mut out) = (Rect::new(0, 0, 80, 20), Vec::new());
    let encoded = graphics.bands_encoded();
    graphics.frame(&session, &state, Some(area), &mut out);
    assert_eq!(*graphics.status(), GraphicsStatus::Ok);
    assert!(graphics.bands_encoded() > encoded, "bands were encoded");
}

#[test]
fn the_cursor_on_the_pinned_row_still_makes_room_for_its_second_line() {
    let mut state = pinned_state(18, 18);
    state.on_working_tree = true;
    let (_graphics, drawn, band) = drawn_of(&state, &pixel_session(PixelProtocol::Kitty), false);
    // Whether a band is drawn is the rasteriser's call (nothing passes through
    // a lane-less row's second line); either way the lines agree: row 18 on
    // line 2, one below where it sits with the cursor elsewhere.
    assert_lines_agree(&state, &drawn, band.as_ref());
    assert_eq!(
        history_row_line(&state, 18).map(|l| l - column_top(&state)),
        Some(2)
    );
}

/// ease-pass/29 D5: every strip, the pinned row, the tall row's band and
/// every emphasis piece is placed from the gutter's gap cell, one cell left
/// of the graph column, and none reaches past the graph column's right edge.
#[test]
fn a_strip_starts_in_the_gutter_gap_cell() {
    let lead = u16::try_from(STRIP_LEAD_CELLS).expect("small");
    let state = pinned_state(18, 18);
    for protocol in [PixelProtocol::Kitty, PixelProtocol::Iterm2] {
        let iterm2 = protocol == PixelProtocol::Iterm2;
        let (_graphics, drawn, band) = drawn_of(&state, &pixel_session(protocol), iterm2);
        let graph = Rect::new(2, 1, 60, 20);
        let mut places = places(&drawn, graph);
        places.extend(band.as_ref().map(|band| band.place(graph)));
        assert!(!places.is_empty(), "{protocol:?}: something is placed");
        let strips = places
            .iter()
            .filter(|place| place.layer == Layer::Strip)
            .count();
        assert_eq!(strips, drawn.len(), "{protocol:?}: one strip image a part");
        for place in &places {
            let cells = place.cells;
            assert!(
                cells.column >= graph.x - lead && cells.column + cells.columns <= graph.right(),
                "{protocol:?}: {:?} spans {}..{} outside the gutter cell and graph {}..{}",
                place.layer,
                cells.column,
                cells.column + cells.columns,
                graph.x - lead,
                graph.right()
            );
            if place.layer == Layer::Strip {
                assert_eq!(cells.column, graph.x - lead, "{protocol:?}: strip left");
            }
        }
        assert!(band.is_some(), "the cursor's second line has a band");
        let band = band.expect("checked");
        assert_eq!(band.place(graph).cells.column, graph.x - lead, "band left");
    }
    assert!(
        drawn_of(&state, &pixel_session(PixelProtocol::Kitty), false)
            .1
            .iter()
            .any(|strip| !strip.emphasis.is_empty()),
        "the fixture draws emphasis pieces, so their placement is checked too"
    );
}
