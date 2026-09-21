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
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Clear, Widget};

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
    draw_credential(state, frame, area, session);
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
    draw_credential(state, frame, area, session);
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
    draw_credential(state, frame, area, session);
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
            draw_preview(state, frame, area, &session.theme);
            draw_context_menu(state, frame, area, &session.theme);
            draw_confirm(state, frame, area, &session.theme);
            draw_credential(state, frame, area, session);
            graph
        }
        Screen::Status => {
            let screen = status_view(session, state);
            frame.render_widget(StatusScreen::new(&screen, &session.theme), area);
            // A fetch started from the graph can ask for a password while
            // the user is reading this screen: the question follows them.
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
    let (graph_area, pane_area) = split_for_detail(frame, session, state, layout, area, "Graph");

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
    let (overlay_area, pane_area) = split_for_detail(frame, session, state, layout, area, label);

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
    state: &AppState,
    layout: DetailLayout,
    area: Rect,
    primary_label: &str,
) -> (Option<Rect>, Option<Rect>) {
    let DetailLayout::Tab { shown } = layout else {
        return graft_ui::detail_pane::split_area(layout, area);
    };
    let area = draw_tab_banner(frame, state, area, &session.theme);
    let strip = Rect {
        height: 1.min(area.height),
        ..area
    };
    draw_tab_strip(frame, strip, shown, primary_label, &session.theme);
    graft_ui::detail_pane::split_area(layout, area)
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
    let Some(banner) = crate::update::banner::banner_view(state) else {
        return area;
    };
    if area.width == 0 || area.height == 0 {
        return area;
    }
    let width = usize::from(area.width);
    let mut style = Style::default().fg(color(theme.token(Token::Warning)));
    if banner.focused {
        style = style.add_modifier(Modifier::REVERSED);
    }
    let line = graft_ui::graph_screen::banner_line(&banner, area.width);
    frame
        .buffer_mut()
        .set_string(area.x, area.y, graft_ui::fit::fit(&line, width), style);
    let left = Rect {
        y: area.y.saturating_add(1),
        height: area.height - 1,
        ..area
    };
    if !banner.focused || left.height == 0 {
        return left;
    }
    // The banner's own keys, on the bottom row. `Divider::Hidden`: the tab
    // layout has no divider to drag (`update::detail`'s angle keys are a
    // no-op here), and the modal would swallow `<>` in any case.
    let hints = graft_ui::graph_screen::hints_for(
        &crate::view::rows::banner_context(&banner),
        graft_ui::graph_screen::Divider::Hidden,
        area.width,
    );
    frame.buffer_mut().set_string(
        left.x,
        left.bottom() - 1,
        graft_ui::fit::fit(&graft_ui::graph_screen::hints_text(&hints), width),
        Style::default().fg(color(theme.token(Token::TextMuted))),
    );
    Rect {
        height: left.height - 1,
        ..left
    }
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
fn draw_preview(state: &AppState, frame: &mut Frame, area: Rect, theme: &Theme) {
    let Some(view) = crate::update::preview::preview_box(state) else {
        return;
    };
    let rect = graft_ui::preview_box::preview_box_area(area, &view);
    if rect.is_empty() {
        return;
    }
    clear_and_tint(frame, rect, rect, theme);
    frame.render_widget(
        &graft_ui::preview_box::PreviewBoxWidget { view: &view, theme },
        rect,
    );
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
fn primary_pane_area(state: &AppState, area: Rect) -> Option<Rect> {
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
fn draw_context_menu(state: &AppState, frame: &mut Frame, area: Rect, theme: &Theme) {
    let Some(open) = state.context_menu.as_ref() else {
        return;
    };
    let menu = &open.menu;
    let Some((x, y, width, height)) = crate::update::context_menu_area(state, open.row, menu)
    else {
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
    let band = overlay_band(area, rect, primary_pane_area(state, area));
    clear_and_tint(frame, band, rect, theme);
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
    clear_and_tint(frame, rect, rect, theme);
    frame.render_widget(
        &graft_ui::confirm::Confirm {
            view: &pending.view,
            theme,
        },
        rect,
    );
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
fn draw_credential(state: &AppState, frame: &mut Frame, area: Rect, session: &Session) {
    let Some(view) = super::credential_view(state) else {
        return;
    };
    let rect = graft_ui::credential_prompt::credential_area(area).intersection(area);
    if rect.is_empty() {
        return;
    }
    let theme = &session.theme;
    clear_and_tint(frame, rect, rect, theme);
    let ascii = tier_in_force(session, state) == RenderingTier::Ascii;
    frame.render_widget(
        &graft_ui::credential_prompt::CredentialPrompt {
            view: &view,
            theme,
            ascii,
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
fn banner_lines(state: &AppState, tier: RenderingTier) -> Vec<SanitizedText> {
    let Some(banner) = &state.harness.banner else {
        return Vec::new();
    };
    std::iter::once(format!("{} · {}", banner.title, tier_name(tier)))
        .chain(banner.lines.iter().cloned())
        .map(|line| sanitize_line(&line))
        .collect()
}

#[cfg(test)]
mod tests {
    use graft_core::job::JobId;
    use graft_term::sanitize_line;
    use graft_ui::text_input::TextField;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;
    use crate::session::for_tests::session;
    use crate::update::confirm::{ConfirmThen, PendingConfirm};
    use crate::update::credential::{CredentialPrompt, PromptField};

    const SCREEN: Rect = Rect {
        x: 0,
        y: 0,
        width: 80,
        height: 24,
    };

    fn menu_rect() -> Rect {
        Rect {
            x: 0,
            y: 4,
            width: 33,
            height: 15,
        }
    }

    /// With no detail pane the band runs to the right edge of the screen, so
    /// nothing the graph drew survives beside the popup
    /// (m5-history-and-ghost-preview/12).
    #[test]
    fn the_band_runs_to_the_end_of_the_whole_area_when_there_is_no_split() {
        let band = overlay_band(SCREEN, menu_rect(), Some(SCREEN));
        assert_eq!(band.x, 0);
        assert_eq!(band.width, 80, "out to the screen's edge");
        assert_eq!(band.y, 4);
        assert_eq!(band.height, 15, "only the popup's own rows");
    }

    /// With the detail pane open the band stops at the graph's edge: clearing
    /// past it would erase the pane the user is reading.
    #[test]
    fn the_band_stops_at_the_pane_edge_so_the_detail_pane_survives() {
        let graph = Rect {
            x: 0,
            y: 0,
            width: 48,
            height: 24,
        };
        let band = overlay_band(SCREEN, menu_rect(), Some(graph));
        assert_eq!(band.width, 48, "the graph's width, not the screen's");
        assert_eq!(
            band.x + band.width,
            48,
            "the detail pane's first column is untouched"
        );
    }

    /// A pane that somehow ends before the popup does still gets cleared
    /// under the whole popup — the band is never narrower than what it is
    /// there to cover.
    #[test]
    fn the_band_is_never_narrower_than_the_popup_itself() {
        let narrow = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 24,
        };
        assert_eq!(overlay_band(SCREEN, menu_rect(), Some(narrow)).width, 33);
        assert_eq!(overlay_band(SCREEN, menu_rect(), None).width, 33);
    }

    /// One glyph that appears nowhere in any overlay, so a cell still
    /// holding it is a cell the overlay never wrote.
    const DIRT: char = '#';

    /// Fills every cell with [`DIRT`], lets `draw` put an overlay on top,
    /// and returns what each row of the terminal ended up holding. This is
    /// the graph rows underneath, stood in for by something unmistakable.
    fn over_a_dirty_screen(
        columns: u16,
        rows: u16,
        draw: impl FnOnce(&mut Frame<'_>, Rect),
    ) -> Vec<String> {
        let mut terminal =
            Terminal::new(TestBackend::new(columns, rows)).expect("TestBackend reports its size");
        terminal
            .draw(|frame| {
                let area = frame.area();
                let dirt: String = std::iter::repeat_n(DIRT, usize::from(area.width)).collect();
                for y in area.top()..area.bottom() {
                    frame
                        .buffer_mut()
                        .set_string(area.x, y, &dirt, Style::default());
                }
                draw(frame, area);
            })
            .expect("TestBackend cannot fail to draw");
        let buffer = terminal.backend().buffer();
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect()
            })
            .collect()
    }

    /// Asserts that nothing the overlay was handed still shows what was
    /// under it.
    fn nothing_shows_through(drawn: &[String], rect: Rect, what: &str) {
        for y in rect.top()..rect.bottom() {
            let row: String = drawn[usize::from(y)]
                .chars()
                .skip(usize::from(rect.x))
                .take(usize::from(rect.width))
                .collect();
            assert!(
                !row.contains(DIRT),
                "{what}: row {y} of the overlay's own rectangle still shows \
                 what was underneath it — {row:?}\n{}",
                drawn.join("\n")
            );
        }
    }

    fn a_confirm() -> PendingConfirm {
        PendingConfirm {
            view: graft_ui::confirm::ConfirmView::new(
                sanitize_line("Discard changes?"),
                vec![sanitize_line("3 files lose their edits.")],
                sanitize_line("git restore ."),
            ),
            then: ConfirmThen::Undo { redo: false },
        }
    }

    fn a_credential() -> CredentialPrompt {
        CredentialPrompt {
            title: sanitize_line("Credentials — example.com"),
            prompt: sanitize_line("Username for 'https://example.com':"),
            field: PromptField::Open(TextField::new("")),
            job: JobId::new(1),
        }
    }

    /// The confirm box owns the rectangle the caller tints for it — at every
    /// size, its degraded sizes included. Below two usable rows the widget
    /// draws **nothing**, and without the clear the tint simply recolours
    /// the graph row that was there (m5-history-and-ghost-preview/13).
    #[test]
    fn the_confirm_box_never_shows_what_was_under_it() {
        let state = AppState {
            confirm: Some(a_confirm()),
            ..AppState::default()
        };
        let session = session();
        let view = &state.confirm.as_ref().expect("a box is open").view;
        for (columns, rows) in [(80_u16, 24_u16), (64, 9), (40, 4), (20, 3), (80, 2), (3, 2)] {
            let area = Rect {
                x: 0,
                y: 0,
                width: columns,
                height: rows,
            };
            let drawn = over_a_dirty_screen(columns, rows, |frame, area| {
                draw_confirm(&state, frame, area, &session.theme);
            });
            let rect = graft_ui::confirm::confirm_area(area, view).intersection(area);
            nothing_shows_through(&drawn, rect, &format!("confirm at {columns}x{rows}"));
        }
    }

    /// The same for git's credential question. Below seven rows the widget
    /// draws one *centred* warning and returns, so without the clear the
    /// tinted row keeps the graph at both ends of it.
    #[test]
    fn the_credential_prompt_never_shows_what_was_under_it() {
        let state = AppState {
            credential: Some(a_credential()),
            ..AppState::default()
        };
        let session = session();
        for (columns, rows) in [
            (80_u16, 24_u16),
            (80, 6),
            (60, 4),
            (39, 8),
            (30, 2),
            (80, 1),
        ] {
            let area = Rect {
                x: 0,
                y: 0,
                width: columns,
                height: rows,
            };
            let drawn = over_a_dirty_screen(columns, rows, |frame, area| {
                draw_credential(&state, frame, area, &session);
            });
            let rect = graft_ui::credential_prompt::credential_area(area).intersection(area);
            nothing_shows_through(&drawn, rect, &format!("credential at {columns}x{rows}"));
        }
    }
}
