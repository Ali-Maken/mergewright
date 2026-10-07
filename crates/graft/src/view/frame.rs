//! The top of the draw: picking the tier and screen, then handing off to
//! [`super::rows`] and [`super::status`] to build the view model.

use graft_core::capabilities::{PixelProtocol, RenderingTier};
use graft_term::SanitizedText;
use graft_term::sanitize_line;
use graft_ui::detail_pane::{DetailLayout, DetailPane};
use graft_ui::graph_screen::{
    GraphScreen, GraphView, GraphicsStatus, HintContext, snap_to_columns,
};
use graft_ui::journal_screen::JournalScreen;
use graft_ui::map_screen::MapScreen;
use graft_ui::overview_strip::{OverviewStrip, StripMarks};
use graft_ui::paint::color;
use graft_ui::stash_screen::StashScreen;
use graft_ui::status_screen::StatusScreen;
use graft_ui::theme::{Theme, Token};
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Clear, Widget};

use crate::session::Session;
use crate::update::remote::ESC_HINT;
use crate::update::{AppState, GraphOverlay, Screen, Status};

use super::detail::detail_view;
use super::journal::journal_view;
use super::map::map_view;
use super::rows::graph_view;
use super::stash::stash_view;
use super::status::status_view;

mod bars;
mod boxes;
mod conflicts;
mod layout;
mod overlays;

// Each child reaches its siblings, and these files' own imports, through
// `use super::*`; what is named here is what more than one of them uses.
use self::bars::{
    bar_rows, draw_box_bars, draw_status_or_hints, draw_tab_banner, draw_tab_hints, draw_tab_strip,
    footer_notice,
};
use self::boxes::{
    draw_commit_editor, draw_context_menu, draw_modal_boxes, draw_over_graph, draw_palette,
    draw_preview, row_columns,
};
use self::conflicts::{draw_finish, draw_resolver};
use self::layout::{
    pixels_above_overlays, render_graph_screen, render_graph_with_detail, split_for_detail,
    tab_banner_area,
};
use self::overlays::{
    draw_journal, draw_map, draw_place, draw_rewrite, draw_stash, render_overlay_with_detail,
};

pub(super) use self::bars::banner_lines;
pub(crate) use self::layout::{PaletteClick, graph_columns, palette_click, screen_area};
pub(super) use self::layout::{primary_pane_area, tabbed};

// `update::tests` measures the detail split against the frame's own.
#[cfg(test)]
pub(crate) use self::layout::detail_rects;

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

    // The screen first, then the modal boxes over it — **outside** the
    // match, so no screen or overlay can leave them out. The resolver did:
    // its `d` set a confirm box that took the next key and was never drawn
    // (m6-07 F10). `columns` is where the graph's row columns stand this
    // frame, for the confirm box to snap onto; empty where no graph rows are
    // on screen.
    let (graph, columns) = match state.screen {
        // Which overlay stands where the graph would be is asked once, of
        // `AppState::graph_overlay`, and `update::banner::graph_screen_on_frame`
        // asks the same function (review M3, m5/31). The two cannot drift:
        // a fifth overlay is a fifth variant and this match stops compiling
        // until it is drawn, while the banner's condition is that function
        // and needs no edit at all.
        Screen::Graph => match state.graph_overlay() {
            // Over everything, the resolver included: the finish summary
            // (m6-conflicts/19), with the banner line on top as the
            // resolver keeps it.
            Some(GraphOverlay::Finish) => (draw_finish(session, state, frame, area), Vec::new()),
            // Above the rest: the resolver covers the detail pane too, and
            // it is the one overlay that keeps the operation banner
            // (`draw_resolver`).
            Some(GraphOverlay::Resolver) => {
                (draw_resolver(session, state, frame, area), Vec::new())
            }
            // The commit editor replaces the graph and leaves the detail
            // pane alone — the rule for every M3 overlay (design note, hard
            // choice 4). It comes before the Map view because `c` is
            // answered wherever the cursor is, the Map view included;
            // `AppState::graph_overlay` holds that order.
            Some(GraphOverlay::CommitEditor) => {
                (draw_commit_editor(session, state, frame, area), Vec::new())
            }
            // A blame place: modal, over the graph and the pane alike
            // (m7-power-tools/20), below the resolver and the finish summary.
            Some(GraphOverlay::Place) => (draw_place(session, state, frame, area), Vec::new()),
            // The rewrite editor: modal, the whole frame (m7-power-tools/08).
            Some(GraphOverlay::Rewrite) => (draw_rewrite(session, state, frame, area), Vec::new()),
            Some(GraphOverlay::Map) => (draw_map(session, state, frame, area), Vec::new()),
            Some(GraphOverlay::Journal) => (draw_journal(session, state, frame, area), Vec::new()),
            Some(GraphOverlay::Stash) => (draw_stash(session, state, frame, area), Vec::new()),
            None => {
                // Kept from the guard this replaced, and now covering the one
                // direction left: that `graph_screen_on_frame` still *is* this
                // function rather than a hand-written condition beside it. It
                // fires in every dev build and test if someone spells the
                // overlays out there again.
                debug_assert!(
                    crate::update::banner::graph_screen_on_frame(state),
                    "no overlay stands where the graph would be, so \
                 `graph_screen_on_frame` must agree that the graph itself is drawn"
                );
                // `graph_view` has already settled everything that decides
                // where a cell lands — the badge glyphs, the lane cap, the
                // harness banner — because `view::hit` builds the same view
                // to hit-test a drag against (m5/33, review M1). What is set
                // here needs the `Session` and moves nothing: the tier's
                // glyph set, the footer notice, the pixel rectangle.
                let mut screen = graph_view(state, state.harness.clock.now_seconds());
                let tier = tier_in_force(session, state);
                screen.ascii = tier == RenderingTier::Ascii;
                // Only below the terminal's own tier: `F3` clears the override
                // when it cycles back to the top (m1-the-graph/33).
                // Fitted to the row it is drawn in: with the detail pane
                // open the footer is the graph half's, not the frame's
                // (review m7-30c-to-10c Major 2). The same pure split the
                // drawing path takes, asked again so the two cannot drift.
                let footer_width =
                    primary_pane_area(state, area).map_or(area.width, |graph| graph.width);
                screen.notice = footer_notice(state, tier, footer_width);
                screen.pixel = pixels;
                screen.graphics = pixels.then(|| graphics.clone());

                let marks = state.strip_shown.then(|| super::strip::strip_marks(state));
                let strip = marks.as_ref().map(|marks| (marks, screen.ascii));

                let graph = if state.detail.shown {
                    render_graph_with_detail(session, state, frame, &screen, area, pixels, strip)
                } else {
                    render_graph_screen(frame, &session.theme, &screen, area, pixels, strip)
                };
                let overlays = draw_over_graph(state, frame, area, &session.theme);
                (
                    pixels_above_overlays(graph, overlays.into_iter().flatten()),
                    row_columns(session, state, &screen, area),
                )
            }
        },
        // A fetch started from the graph can ask for a password while the
        // user is reading this screen: the question follows them, and so
        // does any box that is still waiting for its answer.
        Screen::Status => {
            let screen = status_view(session, state);
            frame.render_widget(StatusScreen::new(&screen, &session.theme), area);
            (None, Vec::new())
        }
    };
    draw_box_bars(session, state, frame, area);
    let palette = draw_palette(session, state, frame, area);
    let boxes = draw_modal_boxes(session, state, frame, area, &columns);
    pixels_above_overlays(graph, palette.into_iter().chain(boxes))
}

#[cfg(test)]
mod tests;
