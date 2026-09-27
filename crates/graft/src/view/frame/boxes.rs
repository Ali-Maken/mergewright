//! Everything drawn over a screen: the commit editor, the Ghost Preview, the
//! context menu and the modal boxes, and the bands they cover or clear
//! (ADR-0040).

// One file of `frame`, split for size (m6-07 review, Major 4): it shares
// the parent's imports and siblings rather than restating them.
#[allow(clippy::wildcard_imports)]
use super::*;

/// The commit editor in the graph's area, the detail pane left alone. Always
/// `None`: the editor draws no pixels, whatever the terminal can do.
pub(super) fn draw_commit_editor(
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

/// The three modal boxes, over whatever this frame drew, in the order they
/// stack: the confirm box, the name prompt, git's credential question on top.
/// Each draws only when its state is live; each returns the rectangle it
/// cleared, for the pixel layer to keep the lanes off.
///
/// **One call site, after every screen** (m6-07 F10). A box in
/// `state.confirm` takes the next key wherever the user is — `confirm_key`
/// is the first modal claim in `update::keys::key_event` — so it has to be
/// on screen wherever the user is. When each overlay drew its own boxes, the
/// resolver, the commit editor and the status screen did not, and the
/// resolver's `d` opened a box nobody could see. Pinned by
/// `view::frame::tests::a_live_confirm_box_is_drawn_over_every_overlay`.
pub(super) fn draw_modal_boxes(
    session: &Session,
    state: &AppState,
    frame: &mut Frame,
    area: Rect,
    columns: &[u16],
) -> Vec<Rect> {
    [
        draw_confirm(state, frame, area, &session.theme, columns),
        draw_name_prompt(state, frame, area, session),
        draw_credential(state, frame, area, session),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// Draws the Ghost Preview box over the graph, when one is open
/// (m5-history-and-ghost-preview/08).
///
/// Docked to the bottom of the graph's own area, at the height the widget
/// asks for — the same height `AppState::graph_rows` already took out of
/// the viewport, so the box covers rows the graph did not draw rather than
/// rows it did.
pub(super) fn draw_preview(
    state: &AppState,
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
) -> Option<Rect> {
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

/// What floats over the graph itself, bottom to top: the Ghost Preview, the
/// `.` menu, the branch picker. Each is `None` when it is not up.
pub(super) fn draw_over_graph(
    state: &AppState,
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
) -> [Option<Rect>; 3] {
    [
        draw_preview(state, frame, area, theme),
        draw_context_menu(state, frame, area, theme),
        draw_picker(state, frame, area, theme),
    ]
}

/// The branch picker's box (`C`, m7-power-tools/10) over the lower part of
/// the graph pane: the header row above and the footer row below are left
/// alone, so the graph's top rows and the bar stay in view.
fn draw_picker(state: &AppState, frame: &mut Frame, area: Rect, theme: &Theme) -> Option<Rect> {
    let view = crate::view::picker::picker_view(state)?;
    let pane = super::layout::primary_pane_area(state, area)?;
    let graph = Rect {
        y: pane.y + 1,
        height: pane.height.saturating_sub(crate::update::CHROME_ROWS),
        ..pane
    };
    let rect = graft_ui::picker::picker_area(graph);
    if rect.is_empty() {
        return None;
    }
    clear_and_tint(frame, rect, rect, theme);
    graft_ui::picker::draw_picker(frame, rect, &view, theme);
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
pub(super) fn clear_and_tint(frame: &mut Frame, clear: Rect, tint: Rect, theme: &Theme) {
    frame.render_widget(Clear, clear);
    frame.buffer_mut().set_style(
        tint,
        Style::new().bg(color(theme.token(Token::SurfaceRaised))),
    );
}

/// The band a box occupies: its own rows, the frame's whole width.
///
/// ADR-0040's other answer. Where a box cannot cover whole columns, the rule
/// leaves it two choices and splitting is not one of them, so the band goes
/// blank instead: a reader cannot mistake a clipped column for a box that
/// leaked into it when there is no column left beside the box at all. Only
/// the box's own rows — the footer below and the header above are not the
/// box's to take.
pub(super) fn full_width_band(area: Rect, rect: Rect) -> Rect {
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
pub(super) fn row_columns(
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
pub(super) fn overlay_band(area: Rect, rect: Rect, pane: Option<Rect>) -> Rect {
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
pub(super) fn draw_context_menu(
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
pub(super) fn draw_confirm(
    state: &AppState,
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    columns: &[u16],
) -> Option<Rect> {
    let pending = state.confirm.as_ref()?;
    // The box's own keys ride the bar at the foot of the pane it stands in
    // (`bars::draw_box_bars`). A box centred on the frame that would reach
    // that row hid them: only then it is centred above the bar instead
    // (`confirm_area` reserves its area's last row — the bar row). Every
    // box clear of the bar keeps its place. A frame too short cuts message
    // rows, never the buttons (graft-ui `confirm`) — m7-power-tools/16b.
    let centred = graft_ui::confirm::confirm_area(area, &pending.view).intersection(area);
    let area = match bars::bar_rows(state, area)
        .iter()
        .map(|bar| bar.y)
        .filter(|&y| y > area.y && y >= centred.y && y < centred.bottom())
        .min()
    {
        Some(bar) => Rect {
            height: bar - area.y + 1,
            ..area
        },
        None => area,
    };
    let natural = graft_ui::confirm::confirm_area(area, &pending.view).intersection(area);
    if natural.is_empty() {
        return None;
    }
    // ADR-0040: the box covers whole columns or it clears. This one never
    // clears — the rows beside it are the graph it is about to act on, and
    // showing them is the whole point of confirming over the graph. So it
    // snaps outward onto the column boundaries and takes its height again
    // at the width that gives it.
    //
    // **Over the resolver it clears, full width, at every width** (rulings
    // 07b, Q2; ADR-0040 "What each box does"). The resolver's document has
    // no column boundaries to snap to — below 100 columns it is one column
    // running the whole width — and its rows are code: a cut line, or a
    // side label fused to the border (`▏ THEIRS╭ Start both.txt over?`), is
    // exactly the fragment beside a border that reads as content. The box
    // says what is lost; the document beside it is not what it acts on.
    let over_the_resolver =
        state.screen == Screen::Graph && state.graph_overlay() == Some(GraphOverlay::Resolver);
    let snapped = if over_the_resolver {
        None
    } else {
        snap_to_columns(natural, columns, area)
    };
    let (rect, clear) = match snapped {
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
pub(super) fn draw_name_prompt(
    state: &AppState,
    frame: &mut Frame,
    area: Rect,
    session: &Session,
) -> Option<Rect> {
    let view = crate::view::name_prompt_view(state)?;
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

pub(super) fn draw_credential(
    state: &AppState,
    frame: &mut Frame,
    area: Rect,
    session: &Session,
) -> Option<Rect> {
    let view = crate::view::credential_view(state)?;
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
