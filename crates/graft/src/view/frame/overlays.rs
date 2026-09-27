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

/// The history place (m7-power-tools/21) in `body`, and its bar on the
/// frame's bottom row.
fn draw_history_place(
    session: &Session,
    state: &AppState,
    history: &crate::update::place::HistoryState,
    frame: &mut Frame,
    area: Rect,
    body: Rect,
) {
    history.view(state.harness.clock.now_seconds(), &session.theme, |view| {
        graft_ui::history::draw_history(frame, body, view);
    });
    if area.height == 0 {
        return;
    }
    let bar = (area.x, area.bottom() - 1, area.width);
    // Its keys need `HintContext::History` (graft-ui `hints/context.rs`,
    // outside 21's files: blocked). Until then the bar carries a problem
    // or a notice and no keys.
    let context = crate::view::rows::place_context(state);
    draw_status_or_hints(
        frame,
        state,
        tier_in_force(session, state),
        bar,
        &session.theme,
        || {
            context.map_or_else(String::new, |context| {
                graft_ui::graph_screen::hints_text(&graft_ui::graph_screen::hints_for(
                    &context,
                    graft_ui::graph_screen::Divider::Hidden,
                    area.width,
                ))
            })
        },
    );
}

/// The blame place over the whole frame (m7-power-tools/20; keymap §2.1:
/// a modal pane), and its bar on the bottom row: the `/` prompt while it is
/// open, else a problem or a notice, else the place's keys
/// ([`draw_status_or_hints`], the ladder every bar uses).
///
/// Never returns a pixel rectangle: the place covers the graph.
pub(super) fn draw_place(
    session: &Session,
    state: &AppState,
    frame: &mut Frame,
    area: Rect,
) -> Option<Rect> {
    // The frame less its bottom row, the bar: `update::place::keys`
    // counts rows from the same split for a click.
    let body = Rect {
        height: area.height.saturating_sub(1),
        ..area
    };
    if let Some(history) = state.trail.history_on_screen() {
        draw_history_place(session, state, history, frame, area, body);
        return None;
    }
    let blame = state.trail.on_screen()?;
    let path = graft_term::sanitize::sanitize_path(&blame.path);
    blame.view(path.as_str(), state.harness.clock.now_seconds(), |view| {
        graft_ui::blame::draw_blame(frame, body, view);
    });
    if area.height == 0 {
        return None;
    }
    let bar = (area.x, area.bottom() - 1, area.width);
    if let Some(typed) = &blame.find_prompt {
        let text = format!(" /{}", graft_term::sanitize::sanitize_line(typed).as_str());
        frame.buffer_mut().set_string(
            bar.0,
            bar.1,
            graft_ui::fit::fit(&text, usize::from(bar.2)),
            Style::default().fg(color(session.theme.token(Token::Text))),
        );
        return None;
    }
    let context = crate::view::rows::place_context(state)?;
    draw_status_or_hints(
        frame,
        state,
        tier_in_force(session, state),
        bar,
        &session.theme,
        || {
            graft_ui::graph_screen::hints_text(&graft_ui::graph_screen::hints_for(
                &context,
                graft_ui::graph_screen::Divider::Hidden,
                area.width,
            ))
        },
    );
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
