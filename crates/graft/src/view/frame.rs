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
    // The rectangles the boxes report are the pixel layer's business
    // (`pixels_above_overlays`), and this overlay draws no pixels — so they
    // are taken and dropped here rather than threaded nowhere.
    let _boxes: [Option<Rect>; 4] = [
        draw_context_menu(state, frame, area, &session.theme),
        draw_confirm(state, frame, area, &session.theme, &[]),
        draw_name_prompt(state, frame, area, session),
        draw_credential(state, frame, area, session),
    ];
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
    // The rectangles the boxes report are the pixel layer's business
    // (`pixels_above_overlays`), and this overlay draws no pixels — so they
    // are taken and dropped here rather than threaded nowhere.
    let _boxes: [Option<Rect>; 4] = [
        draw_context_menu(state, frame, area, &session.theme),
        draw_confirm(state, frame, area, &session.theme, &[]),
        draw_name_prompt(state, frame, area, session),
        draw_credential(state, frame, area, session),
    ];
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
    let overlays = [
        draw_context_menu(state, frame, area, &session.theme),
        draw_confirm(state, frame, area, &session.theme, &[]),
        draw_name_prompt(state, frame, area, session),
        draw_credential(state, frame, area, session),
    ];
    // The Map's own pixels are the detail pane's, and a full-width box
    // covers that pane too (m5-history-and-ghost-preview/34).
    pixels_above_overlays(graph, overlays.into_iter().flatten())
}

/// The conflict resolver over the whole frame. Always `None`: it draws no
/// pixels, whatever the terminal can do.
///
/// **It keeps the banner word, and it is the only overlay that does.** Map,
/// journal, stash and the commit editor all replace the graph screen, and
/// the operation banner rides the graph screen's header — so opening one
/// mid-rebase takes `REBASING step 3 of 7` off the screen. That is a fair
/// trade for a branch list; it is not one for the screen a user opens
/// *because* they are mid-rebase and needs to know which side is which. So
/// the resolver stays an ordinary `GraphOverlay` — the frame's match, the
/// row budget and `banner::graph_screen_on_frame` all keep their one
/// answer — and pays for the banner out of its own rows, on the first line
/// of its own area. `update::resolver::banner_rows` is the one place that
/// row is counted, and the keys and the mouse count from the same function.
///
/// The last row is the resolver's own footer: a failure's line if there is
/// one, else whatever just happened, else the hint bar. The same ladder the
/// graph screen's footer uses, because the resolver covers the graph screen
/// and its footer with it (ADR-0039: a bar is never hidden).
fn draw_resolver(
    session: &Session,
    state: &AppState,
    frame: &mut Frame,
    area: Rect,
) -> Option<Rect> {
    let banner_rows = crate::update::resolver::banner_rows(state);
    let body = draw_resolver_banner(frame, state, area, &session.theme, banner_rows);
    let doc = crate::view::resolver::resolver_view(state, body.width)?;
    let laid_out = graft_ui::conflict_resolver::layout(&doc, body.width);
    let tier = tier_in_force(session, state);
    // The footer's row is not the document's.
    let document = Rect {
        height: body.height.saturating_sub(1),
        ..body
    };
    frame.render_widget(
        graft_ui::conflict_resolver::ResolverScreen {
            doc: &doc,
            layout: &laid_out,
            top: state.resolver.as_ref().map_or(0, |open| open.top),
            theme: &session.theme,
            ascii: tier == RenderingTier::Ascii,
        },
        document,
    );
    draw_resolver_footer(frame, state, body, &session.theme, tier);
    None
}

/// The operation banner on the resolver's first row, and what is left for
/// the resolver itself.
fn draw_resolver_banner(
    frame: &mut Frame,
    state: &AppState,
    area: Rect,
    theme: &Theme,
    rows: u16,
) -> Rect {
    if rows == 0 || area.height <= rows {
        return area;
    }
    if let Some(banner) = crate::update::banner::banner_view(state) {
        let line = graft_ui::graph_screen::banner_line(&banner);
        frame.buffer_mut().set_string(
            area.x,
            area.y,
            graft_ui::fit::fit(&line, usize::from(area.width)),
            Style::default().fg(color(theme.token(Token::Warning))),
        );
    }
    Rect {
        y: area.y + rows,
        height: area.height - rows,
        ..area
    }
}

/// The resolver's bottom row.
fn draw_resolver_footer(
    frame: &mut Frame,
    state: &AppState,
    body: Rect,
    theme: &Theme,
    tier: RenderingTier,
) {
    if body.height == 0 || body.width == 0 {
        return;
    }
    let (text, token) = match super::status::graph_problem(&state.status) {
        Some(problem) => (
            graft_ui::graph_screen::fit_status_line(
                problem.reason.as_str(),
                problem.keys.as_ref().map(SanitizedText::as_str),
                body.width,
            ),
            Token::Warning,
        ),
        None => match footer_notice(state, tier) {
            Some(notice) => (notice.as_str().to_owned(), Token::TextMuted),
            None => (
                graft_ui::graph_screen::hints_text(&graft_ui::graph_screen::hints_for(
                    &HintContext::Resolver {
                        base: false,
                        restart: false,
                    },
                    graft_ui::graph_screen::Divider::Hidden,
                    // Not `body.width`: the floor `hints_for` drops the
                    // resolver's keys at is a *width* one, and the rows can
                    // be short enough to put the widget below its floor at
                    // a perfectly wide screen. Asking with 0 there gives
                    // the same one-key bar the narrow case gets, so the bar
                    // never offers a key `resolver_key` is about to refuse.
                    if crate::update::resolver::fits(state) {
                        body.width
                    } else {
                        0
                    },
                )),
                Token::TextMuted,
            ),
        },
    };
    frame.buffer_mut().set_string(
        body.x,
        body.bottom() - 1,
        graft_ui::fit::fit(&text, usize::from(body.width)),
        Style::default().fg(color(theme.token(token))),
    );
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
pub(super) fn tabbed(state: &AppState, area: Rect) -> bool {
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
        // Which overlay stands where the graph would be is asked once, of
        // `AppState::graph_overlay`, and `update::banner::graph_screen_on_frame`
        // asks the same function (review M3, m5/31). The two cannot drift:
        // a fifth overlay is a fifth variant and this match stops compiling
        // until it is drawn, while the banner's condition is that function
        // and needs no edit at all.
        Screen::Graph => match state.graph_overlay() {
            // The commit editor replaces the graph and leaves the detail
            // pane alone — the rule for every M3 overlay (design note, hard
            // choice 4). It comes before the Map view because `c` is
            // answered wherever the cursor is, the Map view included;
            // `AppState::graph_overlay` holds that order.
            // Above them all: the resolver covers the detail pane too, and
            // it is the one overlay that keeps the operation banner
            // (`draw_resolver`).
            Some(GraphOverlay::Resolver) => draw_resolver(session, state, frame, area),
            Some(GraphOverlay::CommitEditor) => draw_commit_editor(session, state, frame, area),
            Some(GraphOverlay::Map) => draw_map(session, state, frame, area),
            Some(GraphOverlay::Journal) => draw_journal(session, state, frame, area),
            Some(GraphOverlay::Stash) => draw_stash(session, state, frame, area),
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
                screen.notice = footer_notice(state, tier);
                screen.pixel = pixels;
                screen.graphics = pixels.then(|| graphics.clone());

                let marks = state.strip_shown.then(|| super::strip::strip_marks(state));
                let strip = marks.as_ref().map(|marks| (marks, screen.ascii));

                let graph = if state.detail.shown {
                    render_graph_with_detail(session, state, frame, &screen, area, pixels, strip)
                } else {
                    render_graph_screen(frame, &session.theme, &screen, area, pixels, strip)
                };
                let overlays = [
                    draw_preview(state, frame, area, &session.theme),
                    draw_context_menu(state, frame, area, &session.theme),
                    draw_confirm(
                        state,
                        frame,
                        area,
                        &session.theme,
                        &row_columns(session, state, &screen, area),
                    ),
                    draw_name_prompt(state, frame, area, session),
                    draw_credential(state, frame, area, session),
                ];
                pixels_above_overlays(graph, overlays.into_iter().flatten())
            }
        },
        Screen::Status => {
            let screen = status_view(session, state);
            frame.render_widget(StatusScreen::new(&screen, &session.theme), area);
            // A fetch started from the graph can ask for a password while
            // the user is reading this screen: the question follows them.
            draw_name_prompt(state, frame, area, session);
            draw_credential(state, frame, area, session);
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
    let (graph_area, pane_area) = split_for_detail(
        frame,
        session,
        state,
        layout,
        area,
        "Graph",
        &screen.hint_context,
    );

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
    let (overlay_area, pane_area) = split_for_detail(
        frame,
        session,
        state,
        layout,
        area,
        label,
        &crate::view::rows::pane_hint_context(state),
    );

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
/// The columns the graph screen is drawn into this frame: the whole width
/// with no detail pane, and what the split leaves it when there is one.
///
/// ADR-0039's 2026-09-21 amendment, third instance (m5/21 item 4): **ask how
/// much room this thing has, not how big the screen is.** The lane cap came
/// off `state.size.columns`, so a graph squeezed into 67 columns beside a
/// pane on a 160-column terminal folded at the twelve lanes of a wide screen
/// instead of the eight its own width asks for — the breakpoints exist to
/// keep the subject readable, and they were being read off a width the graph
/// does not have.
///
/// `screen` rather than `AppState::size` so the frame passes the area it is
/// really drawing into; the pixel tier, which has no `Frame`, passes its own
/// screen rectangle and the two agree because the split is the same pure
/// geometry ([`graft_ui::detail_pane::split_area`]).
///
/// In the `Tab` layout with the pane shown there is no graph on screen at
/// all; the full width is answered, because nothing is drawn to be wrong.
/// The whole screen as a rectangle, from the size the loop last told
/// `update` about.
///
/// The one answer for "how big is the screen" off the drawing path: a mouse
/// event arrives between frames with no `Frame` to ask, and
/// `event_loop::graphics` has none either. Agreeing with `Frame::area` is
/// what makes the hit-test measure the frame the user is looking at.
#[must_use]
pub(crate) fn screen_area(state: &AppState) -> Rect {
    Rect {
        x: 0,
        y: 0,
        width: state.size.columns,
        height: state.size.rows,
    }
}

pub(crate) fn graph_columns(state: &AppState, screen: Rect) -> u16 {
    if !state.detail.shown {
        return screen.width;
    }
    let layout = crate::update::detail::effective_layout(
        state.detail.focus,
        &state.detail.pane_size,
        (screen.width, screen.height),
    );
    graft_ui::detail_pane::split_area(layout, screen)
        .0
        .map_or(screen.width, |graph| graph.width)
}

/// The rects this frame hands the graph (or overlay) and the detail pane,
/// computed exactly as [`split_for_detail`] does and **without drawing**.
///
/// `update::geometry::detail_split` mirrors this for mouse hit-testing and
/// must agree with it at every size — `update::tests`'
/// `the_geometry_mirror_matches_the_rects_the_frame_hands_out` sweeps the
/// two against each other. It is here rather than there because the rows
/// the tab banner takes are taken *by the drawing path*, and a mirror
/// checked against a second copy of the arithmetic proves nothing (review
/// M1).
#[must_use]
pub(crate) fn detail_rects(
    state: &AppState,
    layout: DetailLayout,
    area: Rect,
) -> (Option<Rect>, Option<Rect>) {
    let DetailLayout::Tab { .. } = layout else {
        return graft_ui::detail_pane::split_area(layout, area);
    };
    graft_ui::detail_pane::split_area(layout, tab_content_area(state, area))
}

/// What is left of `area` for the tab strip and the pane once both of the
/// tab layout's own chrome rows have been taken: the operation banner's
/// ([`tab_banner_area`]) and then the pane's hint bar, one row off the
/// bottom while the Detail tab is up
/// (`update::detail::tab_hint_rows`, m5-history-and-ghost-preview/29).
///
/// Order matters and is the same order `update::geometry::detail_split`
/// mirrors: both rows come out before the strip is carved, so a frame with
/// nothing left carves no strip rather than a negative pane.
fn tab_content_area(state: &AppState, area: Rect) -> Rect {
    let area = tab_banner_area(state, area);
    let rows = crate::update::detail::tab_hint_rows(state).min(area.height);
    Rect {
        height: area.height - rows,
        ..area
    }
}

/// What is left of `area` once the tab banner has taken its rows: one off
/// the top for the banner line, and one off the bottom for its key hints
/// while it has focus. The count is
/// [`crate::update::banner::tab_banner_rows`]'s, so the view, the mouse
/// and `graph_rows` all spend the same rows.
fn tab_banner_area(state: &AppState, area: Rect) -> Rect {
    let rows = crate::update::banner::tab_banner_rows(state);
    if rows == 0 || area.width == 0 || area.height == 0 {
        return area;
    }
    let top = 1.min(area.height);
    let bottom = rows.saturating_sub(1).min(area.height - top);
    Rect {
        y: area.y.saturating_add(top),
        height: area.height - top - bottom,
        ..area
    }
}

fn split_for_detail(
    frame: &mut Frame,
    session: &Session,
    state: &AppState,
    layout: DetailLayout,
    area: Rect,
    primary_label: &str,
    hints: &HintContext,
) -> (Option<Rect>, Option<Rect>) {
    let DetailLayout::Tab { shown } = layout else {
        return graft_ui::detail_pane::split_area(layout, area);
    };
    let left = draw_tab_banner(frame, state, area, &session.theme);
    let strip = Rect {
        height: 1.min(left.height),
        ..left
    };
    draw_tab_strip(frame, strip, shown, primary_label, &session.theme);
    draw_tab_hints(frame, state, left, hints, &session.theme);
    // Not a second copy of the arithmetic: the drawing path splits by the
    // very function the mouse mirror is swept against.
    detail_rects(state, layout, area)
}

/// The detail pane's own hint bar, on the bottom row of the sub-100-column
/// tab layout (`update::detail::tab_hint_rows`,
/// m5-history-and-ghost-preview/29). `left` is what the operation banner
/// left, so the row is that area's last.
///
/// Everywhere else the keys ride the graph screen's footer. On the Detail
/// tab the graph screen is not drawn at all, so the bar went with it and
/// every key was hidden at 80 columns — the one outcome ADR-0037 §4 rules
/// out. The pane that owns the screen owns the keys, so it draws its own
/// `HintContext` here, through the same `hints_for` ladder and floor every
/// other bar uses (ADR-0039; m5/24, m5/28).
///
/// `Divider::Hidden`: the tab layout has no divider to drag, so `<>` is a
/// key that would do nothing — the same reason the focused banner's own
/// hints hide it.
fn draw_tab_hints(
    frame: &mut Frame,
    state: &AppState,
    left: Rect,
    context: &HintContext,
    theme: &Theme,
) {
    let rows = crate::update::detail::tab_hint_rows(state).min(left.height);
    if rows == 0 || left.width == 0 {
        return;
    }
    let hints = graft_ui::graph_screen::hints_for(
        context,
        graft_ui::graph_screen::Divider::Hidden,
        left.width,
    );
    frame.buffer_mut().set_string(
        left.x,
        left.bottom() - 1,
        graft_ui::fit::fit(
            &graft_ui::graph_screen::hints_text(&hints),
            usize::from(left.width),
        ),
        Style::default().fg(color(theme.token(Token::TextMuted))),
    );
}

/// The operation banner in the sub-100-column tab layout: its own row above
/// the `[Graph] Detail` strip, and — while it has focus — its key hints on
/// the bottom row. Returns what is left of `area` for the strip and the pane.
///
/// Everywhere else the banner rides the graph screen's header. Below 100
/// columns the layout switches to tabs (DESIGN §1.1), the shown pane takes
/// the whole screen, the graph screen is not drawn — and the banner went
/// with it. A conflict then stopped the operation, the banner took focus,
/// and it was **modal**: `c`, `s` and `a` were live with nothing on screen
/// to find them by, at 80 columns (m5/16, review C1).
///
/// The rule this settles: *a modal overlay must be visible wherever it has
/// focus.* It is not a pane's content, so it does not travel with one — it
/// draws outside, above the tab line. ADR-0039 guarantees the small,
/// always-needed marker rather than choosing between everything and
/// nothing; this is the same principle for the thing ADR-0039 did not
/// cover.
///
/// The cost, knowingly accepted: **one** row of pane content while an
/// operation is live, **two** while its banner has the keyboard, and none
/// at all the rest of the time.
fn draw_tab_banner(frame: &mut Frame, state: &AppState, area: Rect, theme: &Theme) -> Rect {
    // The row budget is `update`'s, not this function's: `geometry` and
    // `graph_rows` count from the same number, and a second opinion here is
    // exactly how a click comes to land one row off (review M1). `0` also
    // covers the Graph tab, whose graph header already carries the banner
    // (review M2).
    let left = tab_banner_area(state, area);
    if left == area {
        return area;
    }
    let Some(banner) = crate::update::banner::banner_view(state) else {
        return area;
    };
    let width = usize::from(area.width);
    let mut style = Style::default().fg(color(theme.token(Token::Warning)));
    if banner.focused {
        style = style.add_modifier(Modifier::REVERSED);
    }
    let line = graft_ui::graph_screen::banner_line(&banner);
    frame
        .buffer_mut()
        .set_string(area.x, area.y, graft_ui::fit::fit(&line, width), style);
    if left.bottom() == area.bottom() {
        return left;
    }
    // The banner's own keys, on the bottom row — the row `tab_banner_area`
    // just took off the end. `Divider::Hidden`: the tab layout has no
    // divider to drag (`update::detail`'s angle keys are a no-op here), and
    // the modal would swallow `<>` in any case.
    let hints = graft_ui::graph_screen::hints_for(
        &crate::view::rows::banner_context(&banner),
        graft_ui::graph_screen::Divider::Hidden,
        area.width,
    );
    frame.buffer_mut().set_string(
        left.x,
        area.bottom() - 1,
        graft_ui::fit::fit(&graft_ui::graph_screen::hints_text(&hints), width),
        Style::default().fg(color(theme.token(Token::TextMuted))),
    );
    left
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

/// Draws the Ghost Preview box over the graph, when one is open
/// (m5-history-and-ghost-preview/08).
///
/// Docked to the bottom of the graph's own area, at the height the widget
/// asks for — the same height `AppState::graph_rows` already took out of
/// the viewport, so the box covers rows the graph did not draw rather than
/// rows it did.
fn draw_preview(state: &AppState, frame: &mut Frame, area: Rect, theme: &Theme) -> Option<Rect> {
    let view = crate::update::preview::preview_box(state)?;
    let rect = graft_ui::preview_box::preview_box_area(area, &view);
    if rect.is_empty() {
        return None;
    }
    clear_and_tint(frame, rect, rect, theme);
    frame.render_widget(
        &graft_ui::preview_box::PreviewBoxWidget { view: &view, theme },
        rect,
    );
    Some(rect)
}

/// Hands an overlay a rectangle it really owns: `clear` is wiped, then
/// `tint` is painted `SurfaceRaised` so the widget draws onto the overlay's
/// own background. `clear` is the wider of the two where an overlay is
/// narrower than the pane it floats in (the `.` popup's band); everywhere
/// else the two are the same rectangle.
///
/// The clear is the point. `set_style` repaints colour and leaves every
/// cell's *symbol* where it was, so a tint on its own is a claim, not a
/// fact: whatever the widget then declines to write keeps the graph row that
/// was underneath it, now wearing the overlay's background. Each widget does
/// blank its own rows at its own size, which is why the normal path looked
/// right — but their **degraded** paths do not. Below 40x12
/// `PreviewBoxWidget` writes one warning line and returns; below 7 rows
/// `CredentialPrompt` writes one *centred, unpadded* warning and returns,
/// leaving both ends of the row; below 2 rows `Confirm` returns having
/// written nothing at all. Clearing here makes the ownership true for every
/// widget at every size rather than resting on each one filling what it was
/// given (m5-history-and-ghost-preview/13).
fn clear_and_tint(frame: &mut Frame, clear: Rect, tint: Rect, theme: &Theme) {
    frame.render_widget(Clear, clear);
    frame.buffer_mut().set_style(
        tint,
        Style::new().bg(color(theme.token(Token::SurfaceRaised))),
    );
}

/// The pane an overlay anchored in the graph floats inside: the whole area
/// when the detail pane is closed, the graph half of the split when it is
/// open. Pure geometry — the same split `render_graph_with_detail` draws
/// into, asked again rather than remembered, so the two cannot drift.
pub(super) fn primary_pane_area(state: &AppState, area: Rect) -> Option<Rect> {
    if !state.detail.shown {
        return Some(area);
    }
    let layout = crate::update::detail::effective_layout(
        state.detail.focus,
        &state.detail.pane_size,
        (area.width, area.height),
    );
    graft_ui::detail_pane::split_area(layout, area).0
}

/// The graph's pixel rectangle cut back to the rows no overlay stands on
/// (m5-history-and-ghost-preview/34).
///
/// **Why this exists at all.** The pixel tier does not draw the lane column
/// into the text buffer: `view_with_graphics` hands back the rectangle the
/// lanes belong in and `event_loop::running::draw` writes the images *after*
/// the whole frame, so the terminal composites them on top of every cell the
/// frame already wrote. An overlay that covers the graph — the Ghost Preview
/// box, the `.` popup, the confirm box, either prompt — is therefore drawn
/// *under* the lanes it is standing on, however correct its own rectangle is.
///
/// That is the fault the user found: at ~200 columns the preview box's own
/// `│` left border sat at column 0 and the `u` of `undo:` at column 2, and
/// the lane image covered both. Nothing in the character grid was wrong, so
/// no `.txt` capture and no buffer sweep could see it — the two layers only
/// meet on the user's screen.
///
/// **The rule, one for every overlay:** the lanes keep the rows above the
/// topmost overlay that overlaps them, and nothing below it. Cutting rather
/// than splitting because the graphics layer places one rectangle, not a
/// set; an overlay docked to the bottom (the preview box) therefore costs
/// the lanes only the rows it actually covers, and a centred box costs them
/// the rows below it too — which is the safe direction, since those rows are
/// the box's own and the graph text under it was never drawn either.
///
/// `None` when nothing is left, which reads as "no lanes on this frame".
fn pixels_above_overlays(
    graph: Option<Rect>,
    overlays: impl Iterator<Item = Rect>,
) -> Option<Rect> {
    let graph = graph?;
    let mut height = graph.height;
    for overlay in overlays {
        if overlay.is_empty()
            || overlay.right() <= graph.x
            || overlay.x >= graph.right()
            || overlay.bottom() <= graph.y
        {
            continue;
        }
        height = height.min(overlay.y.saturating_sub(graph.y));
    }
    (height > 0).then_some(Rect { height, ..graph })
}

/// The band a box occupies: its own rows, the frame's whole width.
///
/// ADR-0040's other answer. Where a box cannot cover whole columns, the rule
/// leaves it two choices and splitting is not one of them, so the band goes
/// blank instead: a reader cannot mistake a clipped column for a box that
/// leaked into it when there is no column left beside the box at all. Only
/// the box's own rows — the footer below and the header above are not the
/// box's to take.
fn full_width_band(area: Rect, rect: Rect) -> Rect {
    Rect {
        x: area.x,
        y: rect.y,
        width: area.width,
        height: rect.height,
    }
    .intersection(area)
}

/// Where the graph's row columns begin and end this frame, as data, for the
/// overlays that have to keep off the middle of one (ADR-0040).
///
/// Asked of the same [`GraphScreen`] the rows were drawn with, against the
/// same pane, rather than recomputed here — the snap and the draw cannot
/// then disagree about where a column is. Empty when the graph is not on
/// screen, which reads as "no columns here to split".
///
/// With the detail pane open side by side (review Q1), `pane.right()` is
/// **not** the screen's edge — `detail_pane::layout::gutter` already leaves
/// one blank column between the graph and the pane beside it. Left in the
/// boundary list unchanged, `snap_to_columns` would treat that boundary like
/// any interior one and step its own gutter cell back from it (ADR-0040's
/// 2026-09-21 amendment), landing one cell short of the blank column that is
/// already there — on the short-SHA column's own last character rather than
/// beside it. So the boundary that stands for "the pane's own edge" is
/// dropped in favour of where the neighbouring pane's content actually
/// starts, the same distance further out; the amendment's gutter step then
/// lands exactly on the blank column `detail_pane` already reserved.
fn row_columns(
    session: &Session,
    state: &AppState,
    screen: &GraphView<'_>,
    area: Rect,
) -> Vec<u16> {
    let Some(pane) = primary_pane_area(state, area) else {
        return Vec::new();
    };
    let mut boundaries = GraphScreen::new(screen, &session.theme).column_boundaries(pane);
    if state.detail.shown {
        let layout = crate::update::detail::effective_layout(
            state.detail.focus,
            &state.detail.pane_size,
            (area.width, area.height),
        );
        if let DetailLayout::Right { .. } = layout
            && let (_, Some(neighbour)) = graft_ui::detail_pane::split_area(layout, area)
        {
            boundaries.retain(|&x| x != pane.right());
            boundaries.push(neighbour.x);
        }
    }
    // The frame's own edges, so a box straddling the divider into the
    // detail pane snaps out to cover that pane whole rather than
    // stopping part-way across it.
    boundaries.push(area.x);
    boundaries.push(area.right());
    boundaries.sort_unstable();
    boundaries.dedup();
    boundaries
}

/// The band the popup stands on: its own rows, from its left edge out to the
/// end of the pane it floats in. Never narrower than the popup itself, so a
/// pane that somehow ends before the popup does still gets cleared under it.
fn overlay_band(area: Rect, rect: Rect, pane: Option<Rect>) -> Rect {
    let rect_right = rect.x.saturating_add(rect.width);
    let right = pane
        .map_or(rect_right, |pane| pane.x.saturating_add(pane.width))
        .max(rect_right);
    Rect {
        x: rect.x,
        y: rect.y,
        width: right.saturating_sub(rect.x),
        height: rect.height,
    }
    .intersection(area)
}

/// Draws the `.`/right-click popup on top of the graph, when one is open
/// (`context-actions-menu`, m2-details-and-diffs/12). The widget paints only
/// labels and the reversed cursor row — no theme in its signature — so the
/// `SurfaceRaised` background is this caller's job, painted on the buffer
/// before the widget renders over it.
///
/// The popup is only as wide as its longest label, so clearing its own
/// rectangle is not enough: the graph rows it floats over keep drawing their
/// date and sha columns to the right of it, on the popup's own lines, and a
/// verb reads as if it were bound to the sha sitting beside it — `Copy SHA
/// (y) … now 6d3bd01`. So the whole band is cleared first, out to the end of
/// the pane, and only then is the popup's own rectangle tinted and drawn.
/// The popup itself is not widened (m5-history-and-ghost-preview/12, found
/// by the user in `docs/shots/m5-ui-context-menu-verbs-80x24.txt`).
fn draw_context_menu(
    state: &AppState,
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
) -> Option<Rect> {
    let open = state.context_menu.as_ref()?;
    let menu = &open.menu;
    let (x, y, width, height) = crate::update::context_menu_area(state, open.row, menu)?;
    let rect = Rect {
        x,
        y,
        width,
        height,
    }
    .intersection(area);
    if rect.is_empty() {
        return None;
    }
    let band = overlay_band(area, rect, primary_pane_area(state, area));
    clear_and_tint(frame, band, rect, theme);
    frame.render_widget(menu, rect);
    // The band, not the popup: the whole of it is cleared, so the lanes are
    // gone from every cell of it, not only the popup's own.
    Some(band)
}

/// Draws the discard confirm box over everything else, when one is open
/// (m3-working-tree-undo-and-stash/04). It is modal — nothing behind it
/// answers a key while it is up — so it is drawn last, after the context
/// menu, and never under it. Like the context menu the widget paints only
/// its own text, so the raised background is this caller's job.
fn draw_confirm(
    state: &AppState,
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    columns: &[u16],
) -> Option<Rect> {
    let pending = state.confirm.as_ref()?;
    let natural = graft_ui::confirm::confirm_area(area, &pending.view).intersection(area);
    if natural.is_empty() {
        return None;
    }
    // ADR-0040: the box covers whole columns or it clears. This one never
    // clears — the rows beside it are the graph it is about to act on, and
    // showing them is the whole point of confirming over the graph. So it
    // snaps outward onto the column boundaries and takes its height again
    // at the width that gives it.
    let (rect, clear) = match snap_to_columns(natural, columns, area) {
        Some(snapped) => {
            let rect =
                graft_ui::confirm::confirm_area_at(area, &pending.view, snapped.x, snapped.width)
                    .intersection(area);
            (rect, rect)
        }
        // No snap position fits inside the frame. The box may not split a
        // column, so it stops standing on any: it keeps its natural size
        // and the band it occupies is cleared out from under it.
        None => (natural, full_width_band(area, natural)),
    };
    if rect.is_empty() {
        return None;
    }
    clear_and_tint(frame, clear, rect, theme);
    frame.render_widget(
        &graft_ui::confirm::Confirm {
            view: &pending.view,
            theme,
        },
        rect,
    );
    Some(clear)
}

/// git's credential question, over everything — the confirm box included
/// (m4-remotes/06). It is modal in the same way and one step further up:
/// while it is open every key is the field's. Like the confirm box, the
/// widget paints only its own text, so the raised background is this
/// caller's job.
///
/// Draws the masked field at the ASCII tier when that tier is in force
/// (`F3`'s override or the session's), the same rule `draw_map` and the
/// graph screen use for their own `ascii` flags — a masked password must
/// mask with `*` wherever the terminal cannot draw `•`.
/// Draws the name prompt (`b`, `t`, stash `b`) over everything but git's
/// own credential question — which opens by itself, at a moment nobody
/// chose, and so has to be the one box on top.
fn draw_name_prompt(
    state: &AppState,
    frame: &mut Frame,
    area: Rect,
    session: &Session,
) -> Option<Rect> {
    let view = super::name_prompt_view(state)?;
    let rect = graft_ui::name_prompt::name_prompt_area(area).intersection(area);
    if rect.is_empty() {
        return None;
    }
    let theme = &session.theme;
    // ADR-0040: clears rather than snapping, for the credential prompt's
    // reason — it is modal, and the graph behind a box the user is typing
    // into is decoration. Its own rows only, so the footer stays.
    clear_and_tint(frame, full_width_band(area, rect), rect, theme);
    frame.render_widget(
        &graft_ui::name_prompt::NamePrompt { view: &view, theme },
        rect,
    );
    Some(full_width_band(area, rect))
}

fn draw_credential(
    state: &AppState,
    frame: &mut Frame,
    area: Rect,
    session: &Session,
) -> Option<Rect> {
    let view = super::credential_view(state)?;
    let rect = graft_ui::credential_prompt::credential_area(area).intersection(area);
    if rect.is_empty() {
        return None;
    }
    let theme = &session.theme;
    // ADR-0040: this box clears, at every width, rather than snapping. It
    // fragments the sha column when it does not, and nothing behind it
    // serves the task — the prompt is modal (`docs/design/02-keymap.md`
    // §2.14) and the footer already carries the context it needs
    // (`push main → origin/main · esc cancels`). The graph behind a secret
    // is decoration. The band is its own rows only, so that footer stays.
    clear_and_tint(frame, full_width_band(area, rect), rect, theme);
    let ascii = tier_in_force(session, state) == RenderingTier::Ascii;
    frame.render_widget(
        &graft_ui::credential_prompt::CredentialPrompt {
            view: &view,
            theme,
            ascii,
        },
        rect,
    );
    Some(full_width_band(area, rect))
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
        Status::Ready | Status::Failed(_) | Status::Sticky { .. } => {}
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
pub(super) fn banner_lines(state: &AppState) -> Vec<SanitizedText> {
    let Some(banner) = &state.harness.banner else {
        return Vec::new();
    };
    std::iter::once(format!("{} · {}", banner.title, tier_name(state.tier())))
        .chain(banner.lines.iter().cloned())
        .map(|line| sanitize_line(&line))
        .collect()
}

#[cfg(test)]
mod tests;
