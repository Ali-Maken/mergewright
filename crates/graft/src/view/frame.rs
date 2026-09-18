//! The top of the draw: picking the tier and screen, then handing off to
//! [`super::rows`] and [`super::status`] to build the view model.

use graft_core::capabilities::{PixelProtocol, RenderingTier};
use graft_term::SanitizedText;
use graft_term::sanitize_line;
use graft_ui::detail_pane::{DetailLayout, DetailPane};
use graft_ui::graph_screen::{GraphScreen, GraphView, GraphicsStatus, max_lanes_for};
use graft_ui::map_screen::MapScreen;
use graft_ui::paint::color;
use graft_ui::status_screen::StatusScreen;
use graft_ui::theme::{Theme, Token};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;

use crate::session::Session;
use crate::update::{AppState, Screen, Status};

use super::detail::detail_view;
use super::map::map_view;
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
        Screen::Graph if state.map.shown => {
            let tier = tier_in_force(session, state);
            let mut screen = map_view(state, state.harness.clock.now_seconds());
            screen.ascii = tier == RenderingTier::Ascii;

            let graph = if state.detail.shown {
                render_map_with_detail(session, state, frame, &screen, area)
            } else {
                frame.render_widget(
                    MapScreen {
                        view: &screen,
                        theme: &session.theme,
                    },
                    area,
                );
                None
            };
            draw_context_menu(state, frame, area, &session.theme);
            graph
        }
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

            let graph = if state.detail.shown {
                render_graph_with_detail(session, state, frame, &screen, area, pixels)
            } else {
                let widget = GraphScreen::new(&screen, &session.theme);
                let graph = pixels.then(|| widget.graph_area(area));
                frame.render_widget(widget, area);
                graph
            };
            draw_context_menu(state, frame, area, &session.theme);
            graph
        }
        Screen::Status => {
            let screen = status_view(session, state);
            frame.render_widget(StatusScreen::new(&screen, &session.theme), area);
            None
        }
    }
}

/// The graph screen split with the detail pane, per [`DetailLayout`]:
/// side by side (`Right`), stacked (`Below`), or one full-width tab at a
/// time behind a `[Graph] Detail` strip (`Tab`).
fn render_graph_with_detail(
    session: &Session,
    state: &AppState,
    frame: &mut Frame,
    screen: &GraphView<'_>,
    area: Rect,
    pixels: bool,
) -> Option<Rect> {
    let layout = crate::update::detail::effective_layout(
        state.detail.focus,
        &state.detail.pane_size,
        (area.width, area.height),
    );
    let view = detail_view(state);
    let (graph_area, pane_area) = split_for_detail(frame, session, layout, area, "Graph");

    if let Some(pane_area) = pane_area {
        frame.render_widget(
            DetailPane {
                view: &view,
                layout: &layout,
                theme: &session.theme,
            },
            pane_area,
        );
    }

    let graph_area = graph_area?;
    let widget = GraphScreen::new(screen, &session.theme);
    let graph = pixels.then(|| widget.graph_area(graph_area));
    frame.render_widget(widget, graph_area);
    graph
}

/// The Map screen split with the detail pane: the same [`DetailLayout`] the
/// graph uses, so opening the detail pane while `state.map.shown` neither
/// hides the pane nor gives the map the whole frame back
/// (`map-view-screen`, m2-details-and-diffs/19). Its tab strip reads `Map`
/// where the graph's reads `Graph`.
fn render_map_with_detail(
    session: &Session,
    state: &AppState,
    frame: &mut Frame,
    screen: &graft_ui::map_screen::MapView,
    area: Rect,
) -> Option<Rect> {
    let layout = crate::update::detail::effective_layout(
        state.detail.focus,
        &state.detail.pane_size,
        (area.width, area.height),
    );
    let view = detail_view(state);
    let (map_area, pane_area) = split_for_detail(frame, session, layout, area, "Map");

    if let Some(pane_area) = pane_area {
        frame.render_widget(
            DetailPane {
                view: &view,
                layout: &layout,
                theme: &session.theme,
            },
            pane_area,
        );
    }

    let map_area = map_area?;
    frame.render_widget(
        MapScreen {
            view: screen,
            theme: &session.theme,
        },
        map_area,
    );
    None
}

/// `layout`'s primary rectangle (graph or map) and pane rectangle, either of
/// which is absent in `Tab` mode: only the shown side draws. Also draws the
/// tab strip, since only this function knows the divide between the strip
/// and its content. `primary_label` is `"Graph"` or `"Map"`, whichever
/// screen this frame is drawing behind the detail pane.
fn split_for_detail(
    frame: &mut Frame,
    session: &Session,
    layout: DetailLayout,
    area: Rect,
    primary_label: &str,
) -> (Option<Rect>, Option<Rect>) {
    if let DetailLayout::Tab { shown } = layout {
        let strip = Rect {
            height: 1.min(area.height),
            ..area
        };
        draw_tab_strip(frame, strip, shown, primary_label, &session.theme);
    }
    graft_ui::detail_pane::split_area(layout, area)
}

/// The narrow tier's tab strip: `[Graph] Detail` or `Graph [Detail]`
/// (`[Map] Detail` / `Map [Detail]` when `state.map.shown`), bracketing
/// whichever side is on screen.
fn draw_tab_strip(
    frame: &mut Frame,
    area: Rect,
    detail_shown: bool,
    primary_label: &str,
    theme: &graft_ui::theme::Theme,
) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let text = if detail_shown {
        format!("{primary_label}  [Detail]")
    } else {
        format!("[{primary_label}]  Detail")
    };
    let style = Style::default().fg(graft_ui::paint::color(
        theme.token(graft_ui::theme::Token::TextMuted),
    ));
    frame.buffer_mut().set_string(area.x, area.y, text, style);
}

/// Draws the `.`/right-click popup on top of the graph, when one is open
/// (`context-actions-menu`, m2-details-and-diffs/12). The widget itself only
/// paints labels and the reversed cursor row — no theme in its signature —
/// so the `SurfaceRaised` background is this caller's job, painted on the
/// buffer before the widget renders over it.
fn draw_context_menu(state: &AppState, frame: &mut Frame, area: Rect, theme: &Theme) {
    let Some((row, menu)) = state.context_menu.as_ref() else {
        return;
    };
    let Some((x, y, width, height)) = crate::update::context_menu_area(state, *row, menu) else {
        return;
    };
    let rect = Rect {
        x,
        y,
        width,
        height,
    }
    .intersection(area);
    if rect.is_empty() {
        return;
    }
    frame.buffer_mut().set_style(
        rect,
        Style::new().bg(color(theme.token(Token::SurfaceRaised))),
    );
    frame.render_widget(menu, rect);
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
