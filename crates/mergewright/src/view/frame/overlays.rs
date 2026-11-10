//! The screens that replace the graph — the journal browser, the stash list and
//! the map — and the detail pane beside them.

// One file of `frame`, split for size (m6-07 review, Major 4): it shares
// the parent's imports and siblings rather than restating them.
#[allow(
    clippy::wildcard_imports,
    reason = "one module split in files: its parent is its prelude"
)]
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
    let mut screen = journal_view(state, state.harness.clock.now_seconds());
    // J's status row, worded by the ladder every bar uses: the problem,
    // else the notice (m7-power-tools/24, KNOWN_GAPS M3 :217). A refusal
    // inside J has somewhere to show; nothing wrong → no row.
    screen.status_row = journal_status_row(state, tier_in_force(session, state), area.width);
    screen.ascii = tier_in_force(session, state) == RenderingTier::Ascii;
    // The put-back box over J names every key: J's bar is blank under it
    // (`JournalView::box_open`), as S's is under the stash box.
    let widget = JournalScreen {
        view: &screen,
        theme: &session.theme,
        keys: &state.keymap,
    };
    if state.detail.shown {
        render_overlay_with_detail(session, state, frame, widget, area, "Journal");
    } else {
        frame.render_widget(widget, area);
    }
    // `⏎` on a lost row opens the put-back box over J, which stays under
    // it so `esc` is back on the same row (m7-power-tools/24-a2) — docked
    // as `draw_stash` docks the stash box over S. No pixels here for the
    // box to cut either.
    let _preview: Option<Rect> = draw_preview(state, frame, area, &session.theme);
    // The menu's rectangle is the pixel layer's business
    // (`pixels_above_overlays`), and this overlay draws no pixels — so it is
    // taken and dropped here rather than threaded nowhere.
    let _menu: Option<Rect> = draw_context_menu(state, frame, area, &session.theme);
    None
}

/// J's status row: [`draw_status_or_hints`]'s choice without the hints —
/// the problem (warning), else the notice (muted), else no row.
fn journal_status_row(
    state: &AppState,
    tier: RenderingTier,
    width: u16,
) -> Option<mergewright_ui::journal_screen::JournalStatusRow> {
    if let Some(problem) = crate::view::status::bottom_row_problem(state) {
        let ascii = tier == RenderingTier::Ascii;
        let failure = problem.kind == mergewright_ui::graph_screen::ProblemKind::Failure;
        // The words are cut to what the mark leaves; the mark is the row's.
        let lead = if failure {
            mergewright_ui::failure::lead_width(ascii)
        } else {
            0
        };
        return Some(mergewright_ui::journal_screen::JournalStatusRow {
            words: mergewright_ui::graph_screen::fit_status_line(
                problem.reason.as_str(),
                problem.keys.as_ref().map(SanitizedText::as_str),
                width.saturating_sub(u16::try_from(lead).unwrap_or(u16::MAX)),
            ),
            problem: true,
            failure,
            ascii,
        });
    }
    footer_notice(state, tier, width).map(|notice| {
        mergewright_ui::journal_screen::JournalStatusRow {
            words: mergewright_ui::fit::fit(notice.as_str(), usize::from(width))
                .trim_end()
                .to_owned(),
            problem: false,
            failure: false,
            ascii: false,
        }
    })
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
    let mut screen = stash_view(state, state.harness.clock.now_seconds());
    screen.ascii = tier_in_force(session, state) == RenderingTier::Ascii;
    let widget = StashScreen {
        keys: &state.keymap,
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
/// The blame place's body: everything above the bar, in the session's theme.
fn draw_blame_body(
    session: &Session,
    state: &AppState,
    blame: &crate::update::place::BlameState,
    frame: &mut Frame,
    body: Rect,
) {
    let path = mergewright_term::sanitize::sanitize_path(&blame.path);
    let now = state.harness.clock.now_seconds();
    blame.view(path.as_str(), now, &session.theme, |view| {
        mergewright_ui::blame::draw_blame_in_tier(
            frame,
            body,
            view,
            &state.keymap,
            tier_in_force(session, state) == RenderingTier::Ascii,
        );
    });
}

fn draw_history_place(
    session: &Session,
    state: &AppState,
    history: &crate::update::place::HistoryState,
    frame: &mut Frame,
    area: Rect,
    body: Rect,
) {
    history.view(state.harness.clock.now_seconds(), &session.theme, |view| {
        mergewright_ui::history::draw_history_in_tier(
            frame,
            body,
            view,
            &state.keymap,
            tier_in_force(session, state) == RenderingTier::Ascii,
        );
    });
    if area.height == 0 {
        return;
    }
    let bar = (area.x, area.bottom() - 1, area.width);
    // A problem or a notice first, else `HintContext::History`'s keys
    // (`view::rows::place_context`, 21b).
    let context = crate::view::rows::place_context(state);
    draw_status_or_hints(
        frame,
        state,
        tier_in_force(session, state),
        bar,
        &session.theme,
        || {
            context.map_or_else(String::new, |context| {
                mergewright_ui::graph_screen::hints_text(
                    &mergewright_ui::graph_screen::hints_for(
                        &context,
                        mergewright_ui::graph_screen::Divider::Hidden,
                        area.width,
                        &state.keymap,
                    ),
                    &state.keymap,
                )
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
    draw_blame_body(session, state, blame, frame, body);
    if area.height == 0 {
        return None;
    }
    let bar = (area.x, area.bottom() - 1, area.width);
    if let Some(typed) = &blame.find_prompt {
        let text = format!(
            " /{}",
            mergewright_term::sanitize::sanitize_line(typed).as_str()
        );
        frame.buffer_mut().set_string(
            bar.0,
            bar.1,
            mergewright_ui::fit::fit(&text, usize::from(bar.2)),
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
            mergewright_ui::graph_screen::hints_text(
                &mergewright_ui::graph_screen::hints_for(
                    &context,
                    mergewright_ui::graph_screen::Divider::Hidden,
                    area.width,
                    &state.keymap,
                ),
                &state.keymap,
            )
        },
    );
    None
}

/// The rewrite editor over the whole frame (m7-power-tools/08; keymap §2.7:
/// a modal pane), and its bar on the bottom row: a problem or a notice on
/// the status line, else the editor's keys ([`draw_status_or_hints`], the
/// ladder every bar uses). The body is `view::rewrite_editor::editor_area`,
/// the rectangle the editor's mouse asks too.
///
/// Never returns a pixel rectangle: the editor covers the graph.
pub(super) fn draw_rewrite(
    session: &Session,
    state: &AppState,
    frame: &mut Frame,
    area: Rect,
) -> Option<Rect> {
    frame.render_widget(ratatui::widgets::Clear, area);
    let tier = tier_in_force(session, state);
    let view = crate::view::rewrite_editor::rewrite_view(state, tier == RenderingTier::Ascii)?;
    let body = Rect {
        height: area.height.saturating_sub(1),
        ..area
    };
    mergewright_ui::rewrite_editor::draw_rewrite_editor(
        frame,
        body,
        &view,
        &session.theme,
        &state.keymap,
    );
    if area.height == 0 {
        return None;
    }
    let context = crate::view::rewrite_editor::rewrite_context(state)?;
    let bar = (area.x, area.bottom() - 1, area.width);
    draw_status_or_hints(frame, state, tier, bar, &session.theme, || {
        mergewright_ui::graph_screen::hints_text(
            &mergewright_ui::graph_screen::hints_for(
                &context,
                mergewright_ui::graph_screen::Divider::Hidden,
                area.width,
                &state.keymap,
            ),
            &state.keymap,
        )
    });
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
                keys: &state.keymap,
            },
            area,
            "Map",
        )
    } else {
        frame.render_widget(
            MapScreen {
                view: &screen,
                theme: &session.theme,
                keys: &state.keymap,
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
    let mut view = detail_view(state);
    view.ascii = super::tier_in_force(session, state) == RenderingTier::Ascii;
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
                keys: &state.keymap,
            },
            pane_area,
        );
    }

    let overlay_area = overlay_area?;
    frame.render_widget(overlay, overlay_area);
    None
}
