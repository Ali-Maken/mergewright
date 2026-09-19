//! The top of the draw: picking the tier and screen, then handing off to
//! [`super::rows`] and [`super::status`] to build the view model.

use graft_core::capabilities::{PixelProtocol, RenderingTier};
use graft_term::SanitizedText;
use graft_term::sanitize_line;
use graft_ui::detail_pane::{DetailLayout, DetailPane};
use graft_ui::graph_screen::{GraphScreen, GraphView, GraphicsStatus, max_lanes_for};
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
use ratatui::style::Style;
use ratatui::widgets::Widget;

use crate::session::Session;
use crate::update::remote::ESC_HINT;
use crate::update::{AppState, Screen, Status};

use super::detail::detail_view;
use super::journal::journal_view;
use super::map::map_view;
use super::rows::graph_view;
use super::stash::stash_view;
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

/// The undo journal browser in the graph's area, with the confirm box over
/// it — `Enter` on the next-undo row can open the drift guard without the
/// browser having closed first (m3-working-tree-undo-and-stash/06).
///
/// Never returns a pixel rectangle: like the Map view, the overlay replaces
/// the graph, so there is no graph to draw pixels into.
fn draw_journal(
    session: &Session,
    state: &AppState,
    frame: &mut Frame,
    area: Rect,
) -> Option<Rect> {
    let screen = journal_view(state, state.harness.clock.now_seconds());
    let widget = JournalScreen {
        view: &screen,
        theme: &session.theme,
    };
    if state.detail.shown {
        render_overlay_with_detail(session, state, frame, widget, area, "Journal");
    } else {
        frame.render_widget(widget, area);
    }
    draw_context_menu(state, frame, area, &session.theme);
    draw_confirm(state, frame, area, &session.theme);
    None
}

/// The stash overlay in the graph's area, with the confirm box over it —
/// `d` opens the drop box without the overlay having closed first
/// (m3-working-tree-undo-and-stash/06).
///
/// Never returns a pixel rectangle, for the same reason the journal browser
/// does not: the overlay replaces the graph.
fn draw_stash(session: &Session, state: &AppState, frame: &mut Frame, area: Rect) -> Option<Rect> {
    let screen = stash_view(state, state.harness.clock.now_seconds());
    let widget = StashScreen {
        view: &screen,
        theme: &session.theme,
    };
    if state.detail.shown {
        render_overlay_with_detail(session, state, frame, widget, area, "Stash");
    } else {
        frame.render_widget(widget, area);
    }
    draw_context_menu(state, frame, area, &session.theme);
    draw_confirm(state, frame, area, &session.theme);
    None
}

/// The Map view in the graph's area, with the context menu and the confirm
/// box over it. Returns the rectangle the detail pane's pixels go in.
fn draw_map(session: &Session, state: &AppState, frame: &mut Frame, area: Rect) -> Option<Rect> {
    let tier = tier_in_force(session, state);
    let mut screen = map_view(state, state.harness.clock.now_seconds());
    screen.ascii = tier == RenderingTier::Ascii;

    let graph = if state.detail.shown {
        render_overlay_with_detail(
            session,
            state,
            frame,
            MapScreen {
                view: &screen,
                theme: &session.theme,
            },
            area,
            "Map",
        )
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
    draw_confirm(state, frame, area, &session.theme);
    graph
}

/// The commit editor in the graph's area, the detail pane left alone. Always
/// `None`: the editor draws no pixels, whatever the terminal can do.
fn draw_commit_editor(
    session: &Session,
    state: &AppState,
    frame: &mut Frame,
    area: Rect,
) -> Option<Rect> {
    let editor = crate::view::commit_editor_view(state)?;
    let screen = graft_ui::commit_editor::CommitEditorScreen {
        view: &editor,
        theme: &session.theme,
    };
    if state.detail.shown && !tabbed(state, area) {
        render_overlay_with_detail(session, state, frame, &screen, area, "Commit");
    } else {
        frame.render_widget(&screen, area);
    }
    None
}

/// Would the detail pane share this area as a **tab strip** rather than a
/// split?
///
/// Under 100 columns it does, and that is the one layout the commit editor
/// cannot use: the editor is modal, so the key that would select the other
/// tab never reaches the pane. An editor behind a tab the user cannot press
/// is an editor that did not open. Below that width it takes the whole area
/// and the pane waits — which is what `Esc` gets them back to.
fn tabbed(state: &AppState, area: Rect) -> bool {
    matches!(
        crate::update::detail::effective_layout(
            state.detail.focus,
            &state.detail.pane_size,
            (area.width, area.height),
        ),
        DetailLayout::Tab { .. }
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
        // The commit editor replaces the graph and leaves the detail pane
        // alone — the rule for every M3 overlay (design note, hard choice 4).
        // It is checked before the Map view's arm because `c` is answered
        // wherever the cursor is, the Map view included.
        Screen::Graph if state.commit_editor.is_some() => {
            draw_commit_editor(session, state, frame, area)
        }
        Screen::Graph if state.map.shown => draw_map(session, state, frame, area),
        Screen::Graph if state.journal.shown => draw_journal(session, state, frame, area),
        Screen::Graph if state.stash.shown => draw_stash(session, state, frame, area),
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

            let marks = state.strip_shown.then(|| super::strip::strip_marks(state));
            let strip = marks.as_ref().map(|marks| (marks, screen.ascii));

            let graph = if state.detail.shown {
                render_graph_with_detail(session, state, frame, &screen, area, pixels, strip)
            } else {
                render_graph_screen(frame, &session.theme, &screen, area, pixels, strip)
            };
            draw_context_menu(state, frame, area, &session.theme);
            draw_confirm(state, frame, area, &session.theme);
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
    strip: Option<(&StripMarks, bool)>,
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
    render_graph_screen(frame, &session.theme, screen, graph_area, pixels, strip)
}

/// Draws `screen`'s [`GraphScreen`] into `area`, carving the strip's column
/// off first when `strip` is `Some` (never combined with `pixels`, DESIGN
/// §5.3: no strip at the pixel tier).
///
/// `Columns::compute` (`graph_screen/layout.rs`) lays the trailing
/// sha/when/author columns out right-to-left from `area.right()`, so simply
/// narrowing `area` before handing it to `GraphScreen` would shift those
/// columns left by one cell — breaking the promise that they stay put
/// whether or not the strip is shown. Instead: draw `GraphScreen` into a
/// scratch buffer sized to the *undiminished* `area` (so those columns land
/// exactly where they would with the strip off), copy every column but the
/// last into the real buffer, and paint the strip into the column that copy
/// skipped.
fn render_graph_screen(
    frame: &mut Frame,
    theme: &Theme,
    screen: &GraphView<'_>,
    area: Rect,
    pixels: bool,
    strip: Option<(&StripMarks, bool)>,
) -> Option<Rect> {
    let widget = GraphScreen::new(screen, theme);
    let Some((marks, ascii)) = strip.filter(|_| !pixels && area.width > 1) else {
        let graph = pixels.then(|| widget.graph_area(area));
        frame.render_widget(widget, area);
        return graph;
    };

    let mut scratch = Buffer::empty(area);
    widget.render(area, &mut scratch);

    let strip_x = area.right() - 1;
    let buf = frame.buffer_mut();
    for y in area.top()..area.bottom() {
        for x in area.left()..strip_x {
            buf[(x, y)] = scratch[(x, y)].clone();
        }
    }
    let strip_area = Rect {
        x: strip_x,
        y: area.y,
        width: 1,
        height: area.height,
    };
    OverviewStrip {
        marks,
        theme,
        ascii,
    }
    .render(strip_area, buf);
    None
}

/// An overlay screen (the Map view, the commit editor) split with the detail
/// pane: the same [`DetailLayout`] the graph uses, so opening the detail pane
/// while an overlay is up neither hides the pane nor gives the overlay the
/// whole frame back (`map-view-screen`, m2-details-and-diffs/19; design note
/// hard choice 4 makes this the rule for every M3 overlay). Its tab strip
/// reads `Map` or `Commit` where the graph's reads `Graph`.
fn render_overlay_with_detail(
    session: &Session,
    state: &AppState,
    frame: &mut Frame,
    overlay: impl Widget,
    area: Rect,
    label: &str,
) -> Option<Rect> {
    let layout = crate::update::detail::effective_layout(
        state.detail.focus,
        &state.detail.pane_size,
        (area.width, area.height),
    );
    let view = detail_view(state);
    let (overlay_area, pane_area) = split_for_detail(frame, session, layout, area, label);

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

    let overlay_area = overlay_area?;
    frame.render_widget(overlay, overlay_area);
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

/// Draws the discard confirm box over everything else, when one is open
/// (m3-working-tree-undo-and-stash/04). It is modal — nothing behind it
/// answers a key while it is up — so it is drawn last, after the context
/// menu, and never under it. Like the context menu the widget paints only
/// its own text, so the raised background is this caller's job.
fn draw_confirm(state: &AppState, frame: &mut Frame, area: Rect, theme: &Theme) {
    let Some(pending) = state.confirm.as_ref() else {
        return;
    };
    let rect = graft_ui::confirm::confirm_area(area, &pending.view).intersection(area);
    if rect.is_empty() {
        return;
    }
    frame.buffer_mut().set_style(
        rect,
        Style::new().bg(color(theme.token(Token::SurfaceRaised))),
    );
    frame.render_widget(
        &graft_ui::confirm::Confirm {
            view: &pending.view,
        },
        rect,
    );
}

/// The footer's notice: what is happening or just happened, else `F3`'s
/// renderer line while an override is on. `None` otherwise, so the key
/// hints show.
///
/// `Status::Failed` is not here: a failure is the graph screen's `problem`
/// line, which is louder and does not vanish on the next key.
fn footer_notice(state: &AppState, tier: RenderingTier) -> Option<SanitizedText> {
    match &state.status {
        // A write is running. The only acknowledgement `Space` gives, and the
        // only thing that says why the other write keys do nothing meanwhile
        // (m3/04 review M3).
        Status::Busy(description) if description.ends_with(ESC_HINT) => {
            return Some(sanitize_line(description));
        }
        Status::Busy(description) => {
            return Some(sanitize_line(&format!("{description}…")));
        }
        // It finished. Lasts until the next key, like the copy line.
        Status::Done(description) => {
            return Some(sanitize_line(&format!("{description} ✓")));
        }
        // `y`'s notice, worded as what was sent: whether the terminal honoured
        // it is unknowable.
        Status::Copied { short } => {
            return Some(sanitize_line(&format!("{short}… → clipboard (osc 52)")));
        }
        Status::Ready | Status::Failed(_) | Status::Sticky(_) => {}
    }
    // An operation a previous run left half-done. Louder than the renderer
    // line and quieter than a failure — nothing is broken, but there is a
    // backup waiting and the user should know before they press `u`
    // (m3-working-tree-undo-and-stash/06).
    if let Some(notice) = crate::update::undo::interrupted_notice(state) {
        return Some(sanitize_line(&notice));
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
