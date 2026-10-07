//! m8-polish-and-ship/17c, ADR-0060 decision 3: no strip is rastered for a
//! preview. `update` asks for strips only indirectly — `ShowRows` padded to
//! strip boundaries (`History::strip_rows`) — and the loop rasters only
//! while `view::draws_pixels` holds (`graphics.frame`'s rectangle,
//! `event_loop::running` prefetch). Both are asserted here.

use graft_core::capabilities::{PixelProtocol, RenderingTier};
use graft_core::event::{Event, KeyCode, KeyPress, Modifiers, MouseInput, MouseKind};
use graft_graph::raster::cache::STRIP_ROWS;

use super::super::{AppEvent, AppState, Command, update};
use super::core;
use crate::session::for_tests::session;

#[test]
fn a_pixel_preview_requests_no_strips() {
    let (mut state, top, _) = crate::view::tests::scrolled_preview(120, |height| 2 * height, 3);
    state.settled_tier = Some(RenderingTier::Pixel);
    if let Some(history) = state.history.as_mut() {
        history.strip_rows = Some(STRIP_ROWS);
    }
    let mut pixel = session();
    pixel.rendering.tier = RenderingTier::Pixel;
    pixel.capabilities.pixel_protocol = Some(PixelProtocol::Kitty);

    let scrolls: Vec<AppEvent> = vec![
        core(Event::Key(KeyPress::plain(KeyCode::PageDown))),
        core(Event::Key(KeyPress::plain(KeyCode::Char('j')))),
        core(Event::Mouse(MouseInput {
            kind: MouseKind::ScrollDown,
            column: 10,
            row: 5,
            modifiers: Modifiers::NONE,
        })),
    ];
    let mut now: AppState = state;
    for scroll in scrolls {
        let (next, commands) = update(&now, scroll);
        assert!(
            !commands
                .iter()
                .any(|command| matches!(command, Command::ShowRows(..))),
            "a scroll under the preview asks for no rows, so for no strip: {commands:?}"
        );
        assert!(next.preview.is_some(), "the preview stays open");
        assert!(
            !crate::view::draws_pixels(&pixel, &next),
            "the loop's raster paths are gated off while the picture is up"
        );
        now = next;
    }
    // The wheel does move the live window behind the picture
    // (`update::mouse` routes `ScrollDown` to `scroll` with no preview
    // check): recorded as a question in 17c's NOTES, not decided here. The
    // rows it reaches are resident, so it reads none.
    assert!(
        now.history
            .as_ref()
            .is_some_and(|history| history.top >= top),
        "premise: the fixture's window"
    );
}
