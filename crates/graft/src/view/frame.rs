//! The top of the draw: picking the tier and screen, then handing off to
//! [`super::rows`] and [`super::status`] to build the view model.

use graft_core::capabilities::{PixelProtocol, RenderingTier};
use graft_term::SanitizedText;
use graft_term::sanitize_line;
use graft_ui::graph_screen::{GraphScreen, GraphicsStatus, max_lanes_for};
use graft_ui::status_screen::StatusScreen;
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::session::Session;
use crate::update::{AppState, Screen, Status};

use super::rows::graph_view;
use super::status::status_view;

/// The tier's name, as `graph.renderer` and the status screen spell it.
fn tier_name(tier: RenderingTier) -> &'static str {
    match tier {
        RenderingTier::Pixel => "pixel",
        RenderingTier::Unicode => "unicode",
        RenderingTier::Ascii => "ascii",
    }
}

/// Draw the current state.
pub fn view(session: &Session, state: &AppState, frame: &mut Frame) {
    view_with_graphics(session, state, &GraphicsStatus::Ok, frame);
}

/// The tier the graph draws at: `F3`'s override, else the session's.
#[must_use]
pub fn tier_in_force(session: &Session, state: &AppState) -> RenderingTier {
    state.renderer_override.unwrap_or(session.rendering.tier)
}

/// Whether this frame's graph is drawn in pixels: the graph screen, at the
/// pixel tier, over the Kitty protocol or iTerm2 inline images.
///
/// `graph.renderer = pixel` on a terminal that never acked any image
/// protocol attempts Kitty anyway (DESIGN.md §4.2: config wins, and silence
/// is not proof it cannot draw); `F3` drops to Unicode if nothing shows.
#[must_use]
pub fn draws_pixels(session: &Session, state: &AppState) -> bool {
    state.screen == Screen::Graph
        && tier_in_force(session, state) == RenderingTier::Pixel
        && matches!(
            session.capabilities.pixel_protocol,
            Some(PixelProtocol::Kitty | PixelProtocol::Iterm2) | None
        )
}

/// [`view`], saying `graphics` in the header when the graph is drawn in
/// pixels; returns the rectangle those pixels go in, or `None` when this
/// frame draws no pixels.
pub fn view_with_graphics(
    session: &Session,
    state: &AppState,
    graphics: &GraphicsStatus,
    frame: &mut Frame,
) -> Option<Rect> {
    let area = frame.area();
    if area.is_empty() {
        // Nothing to draw into. Not an error — a terminal can legitimately be
        // this size for a moment (ADR-0012) — so there is nothing to report.
        return None;
    }
    let pixels = draws_pixels(session, state);

    match state.screen {
        Screen::Graph => {
            let mut screen = graph_view(state, state.harness.clock.now_seconds());
            screen.icons = session.icons;
            screen.compaction.max_lanes = max_lanes_for(state.size.columns, session.max_lanes);
            let tier = tier_in_force(session, state);
            screen.ascii = tier == RenderingTier::Ascii;
            // Only below the terminal's own tier: `F3` clears the override
            // when it cycles back to the top (m1-the-graph/33).
            screen.notice = footer_notice(state, tier);
            screen.banner = banner_lines(state, tier);
            screen.pixel = pixels;
            screen.graphics = pixels.then(|| graphics.clone());
            let widget = GraphScreen::new(&screen, &session.theme);
            let graph = pixels.then(|| widget.graph_area(area));
            frame.render_widget(widget, area);
            graph
        }
        Screen::Status => {
            let screen = status_view(session, state);
            frame.render_widget(StatusScreen::new(&screen, &session.theme), area);
            None
        }
    }
}

/// The footer's notice: `y`'s copy line until the next key, else `F3`'s
/// renderer line while an override is on. `None` otherwise, so the key
/// hints show.
fn footer_notice(state: &AppState, tier: RenderingTier) -> Option<SanitizedText> {
    // `y`'s notice is the newer news: it lasts one key, the override stays.
    // Worded as what was sent; whether the terminal honoured it is unknowable.
    if let Status::Copied { short } = &state.status {
        return Some(sanitize_line(&format!("{short}… → clipboard (osc 52)")));
    }
    // Only below the terminal's own tier: `F3` clears the override when it
    // cycles back to the top (m1-the-graph/33).
    state
        .renderer_override
        .map(|_| sanitize_line(&format!("renderer: {} (F3 cycles)", tier_name(tier))))
}

/// The harness banner as drawn: its title with ` · <tier>` appended for the
/// tier in force (the harness cannot know it; the probe decides), then its
/// lines. Empty without a harness banner, so the shipped binary draws
/// nothing extra.
fn banner_lines(state: &AppState, tier: RenderingTier) -> Vec<SanitizedText> {
    let Some(banner) = &state.harness.banner else {
        return Vec::new();
    };
    std::iter::once(format!("{} · {}", banner.title, tier_name(tier)))
        .chain(banner.lines.iter().cloned())
        .map(|line| sanitize_line(&line))
        .collect()
}
