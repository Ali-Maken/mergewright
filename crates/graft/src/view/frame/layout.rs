//! Where things go: the graph and the detail pane beside or below it, the tab
//! layout under 100 columns, and the rectangles the pixel layer may use.

// One file of `frame`, split for size (m6-07 review, Major 4): it shares
// the parent's imports and siblings rather than restating them.
#[allow(clippy::wildcard_imports)]
use super::*;

/// Would the detail pane share this area as a **tab strip** rather than a
/// split?
///
/// Under 100 columns it does, and that is the one layout the commit editor
/// cannot use: the editor is modal, so the key that would select the other
/// tab never reaches the pane. An editor behind a tab the user cannot press
/// is an editor that did not open. Below that width it takes the whole area
/// and the pane waits — which is what `Esc` gets them back to.
pub(in crate::view) fn tabbed(state: &AppState, area: Rect) -> bool {
    matches!(
        crate::update::detail::effective_layout(
            state.detail.focus,
            &state.detail.pane_size,
            (area.width, area.height),
        ),
        DetailLayout::Tab { .. }
    )
}

/// The graph screen split with the detail pane, per [`DetailLayout`]:
/// side by side (`Right`), stacked (`Below`), or one full-width tab at a
/// time behind a `[Graph] Detail` strip (`Tab`).
pub(super) fn render_graph_with_detail(
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
                keys: &state.keymap,
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
pub(super) fn render_graph_screen(
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
pub(super) fn tab_content_area(state: &AppState, area: Rect) -> Rect {
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
pub(super) fn tab_banner_area(state: &AppState, area: Rect) -> Rect {
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

pub(super) fn split_for_detail(
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
    let tier = tier_in_force(session, state);
    let left = draw_tab_banner(frame, state, area, &session.theme, tier);
    let strip = Rect {
        height: 1.min(left.height),
        ..left
    };
    draw_tab_strip(frame, strip, shown, primary_label, &session.theme);
    draw_tab_hints(frame, state, left, hints, &session.theme, tier);
    // Not a second copy of the arithmetic: the drawing path splits by the
    // very function the mouse mirror is swept against.
    detail_rects(state, layout, area)
}

/// The rectangle the palette (§2.12) is drawn in on a frame of `area`:
/// over the pane the keys go to on the graph, over the whole frame
/// elsewhere. One placement for the drawing (`boxes::draw_palette`) and the
/// mouse (`update::mouse`, m8/11b decision 6), so a click and a cell agree.
pub(super) fn palette_rect(state: &AppState, area: Rect) -> Rect {
    let over = match state.screen {
        Screen::Graph if state.resolver.is_none() && state.finish.is_none() => {
            primary_pane_area(state, area).unwrap_or(area)
        }
        Screen::Graph | Screen::Status => area,
    };
    graft_ui::palette::palette_area(over, area)
}

/// Where a click lands on the palette (m8/11b decision 6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PaletteClick {
    /// Outside [`palette_rect`]: closes it.
    Outside,
    /// Inside, on no row: a border, the query, a header.
    Inside,
    /// On row `i`, over every row with groups flattened.
    Row(usize),
}

/// Where a click at `column`, `row` lands on the palette drawn from `view`
/// over this frame.
pub(crate) fn palette_click(
    state: &AppState,
    view: &graft_ui::palette::PaletteView,
    column: u16,
    row: u16,
) -> PaletteClick {
    let area = palette_rect(state, Rect::new(0, 0, state.size.columns, state.size.rows));
    let inside = column >= area.x && column < area.right() && row >= area.y && row < area.bottom();
    if !inside {
        return PaletteClick::Outside;
    }
    graft_ui::palette::palette_row_at(area, view, column, row)
        .map_or(PaletteClick::Inside, PaletteClick::Row)
}

/// The pane an overlay anchored in the graph floats inside: the whole area
/// when the detail pane is closed, the graph half of the split when it is
/// open. Pure geometry — the same split `render_graph_with_detail` draws
/// into, asked again rather than remembered, so the two cannot drift.
pub(in crate::view) fn primary_pane_area(state: &AppState, area: Rect) -> Option<Rect> {
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
pub(super) fn pixels_above_overlays(
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
