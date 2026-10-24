//! ease-pass/30: which failures the loop is told of.

use mergewright_core::capabilities::PixelProtocol;
use mergewright_graph::raster::cache::STRIP_ROWS;

use super::*;
use crate::session::for_tests::session;
use crate::update::Size;

/// A pixel-tier session whose cell no strip can be sized for (the
/// rasteriser's `TooLarge`), and 100 commits to draw.
fn failing() -> (Session, AppState) {
    let mut session = session();
    session.rendering.tier = RenderingTier::Pixel;
    session.capabilities.pixel_protocol = Some(PixelProtocol::Kitty);
    session.rendering.cell_size.height = 3_000_000_000;
    let mut history = crate::view::tests::history_of(100, 0);
    history.strip_rows = Some(STRIP_ROWS);
    let state = AppState {
        history: Some(history),
        settled_tier: Some(RenderingTier::Pixel),
        size: Size {
            columns: 80,
            rows: 24,
        },
        ..AppState::default()
    };
    (session, state)
}

#[test]
fn a_prefetch_failure_drops_nothing() {
    let (session, state) = failing();
    let mut graphics = Graphics::new(NoReplies, &session.capabilities);
    let rasterised = graphics.prefetch(&session, &state, &mut || false);

    assert_eq!(rasterised, 0, "premise: no strip could be rasterised");
    assert_eq!(graphics.take_failure(), None, "nothing to drop for");
    assert_eq!(graphics.status(), &GraphicsStatus::Ok, "nothing to show");
    assert_eq!(graphics.strip_stats().rasterised, 0, "and nothing cached");
}

#[test]
fn a_strip_failing_on_screen_is_told_once() {
    let (session, state) = failing();
    let mut graphics = Graphics::new(NoReplies, &session.capabilities);
    let area = Rect::new(1, 1, 3, 20);
    graphics.frame(&session, &state, Some(area), &mut Vec::new());

    let why = graphics.take_failure().expect("the strip failed on screen");
    assert!(why.as_str().starts_with("a strip of"), "{}", why.as_str());
    assert_eq!(graphics.take_failure(), None, "taken once");
}
