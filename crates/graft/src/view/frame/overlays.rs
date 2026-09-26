//! The screens that replace the graph — the journal browser, the stash list and
//! the map — and the detail pane beside them.

// One file of `frame`, split for size (m6-07 review, Major 4): it shares
// the parent's imports and siblings rather than restating them.
#[allow(clippy::wildcard_imports)]
use super::*;

/// The undo journal browser in the graph's area, and the context menu over
/// it. The confirm box — `Enter` on the next-undo row can open the drift
/// guard without the browser having closed first
/// (m3-working-tree-undo-and-stash/06) — is [`draw_modal_boxes`]'s, drawn
/// over every screen alike.
///
/// Never returns a pixel rectangle: like the Map view, the overlay replaces
/// the graph, so there is no graph to draw pixels into.
pub(super) fn draw_journal(
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
    // The menu's rectangle is the pixel layer's business
    // (`pixels_above_overlays`), and this overlay draws no pixels — so it is
    // taken and dropped here rather than threaded nowhere.
    let _menu: Option<Rect> = draw_context_menu(state, frame, area, &session.theme);
    None
}

/// The stash overlay in the graph's area, and the context menu over it.
/// The confirm box `d` opens without the overlay having closed first
/// (m3-working-tree-undo-and-stash/06) is [`draw_modal_boxes`]'s.
///
/// Never returns a pixel rectangle, for the same reason the journal browser
/// does not: the overlay replaces the graph.
pub(super) fn draw_stash(
    session: &Session,
    state: &AppState,
    frame: &mut Frame,
    area: Rect,
) -> Option<Rect> {
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
    // The stash list's `⏎`/`p` open a put-back preview over the list, which
    // stays under it so `esc` is back on the same row (m6-conflicts/86). No
    // pixels here for the box to cut either.
    let _preview: Option<Rect> = draw_preview(state, frame, area, &session.theme);
    // As in `draw_journal`: no pixels here for the menu to cut.
    let _menu: Option<Rect> = draw_context_menu(state, frame, area, &session.theme);
    None
}

/// The Map view in the graph's area, with the context menu over it.
/// Returns the rectangle the detail pane's pixels go in.
pub(super) fn draw_map(
    session: &Session,
    state: &AppState,
    frame: &mut Frame,
    area: Rect,
) -> Option<Rect> {
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
    // The Map's own pixels are the detail pane's, and a full-width box
    // covers that pane too (m5-history-and-ghost-preview/34).
    pixels_above_overlays(
        graph,
        draw_context_menu(state, frame, area, &session.theme).into_iter(),
    )
}

/// An overlay screen (the Map view, the commit editor) split with the detail
/// pane: the same [`DetailLayout`] the graph uses, so opening the detail pane
/// while an overlay is up neither hides the pane nor gives the overlay the
/// whole frame back (`map-view-screen`, m2-details-and-diffs/19; design note
/// hard choice 4 makes this the rule for every M3 overlay). Its tab strip
/// reads `Map` or `Commit` where the graph's reads `Graph`.
pub(super) fn render_overlay_with_detail(
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
