//! `update` tests: tab strip geometry. Moved out of `update/tests.rs` (m8/21b-5, move-only).

use super::*;

/// **The class test for the vertical axis.** `update::geometry` exists to
/// mirror the view for hit-testing, and its `Tab` arm did not know the
/// operation banner takes a row above the strip (and a second off the
/// bottom while focused). So below 100 columns with a merge stopped, every
/// click landed one row above the row the user saw, and two while the
/// banner had the keyboard.
///
/// Swept, not sampled, and against the rects the **drawing path** computes
/// (`view::frame::detail_rects` is what `split_for_detail` splits from),
/// not against a second copy of the arithmetic — a mirror checked against
/// a copy of itself agrees by construction. Widths through both
/// breakpoints; heights down to one where the banner's two rows are most
/// of the frame; both tabs; focused and not; banner live and not.
/// The third reader of the tab layout's row budget: `AppState::graph_rows`,
/// which is how many history rows `j`/`k`, `PgDn` and `G` believe are on
/// screen. The `[Graph] Detail` strip is a row the graph never gets, and
/// neither was a term here before review M1 - so the cursor could be moved
/// onto a row the graph had not drawn.
///
/// The strip is all it costs *on the graph tab*: the banner's word rides the
/// graph header there (see `banner::tab_banner_rows`), which is the other
/// half of M2. The number itself is checked against the drawn rects by
/// `the_geometry_mirror_matches_the_rects_the_frame_hands_out`; this test is
/// about who spends it.
#[test]
fn the_tab_strip_comes_out_of_the_graphs_row_budget() {
    use mergewright_ui::detail::DetailFocus;

    let mut tabbed = AppState {
        size: crate::update::state::Size {
            columns: 80,
            rows: 24,
        },
        ..AppState::default()
    };
    tabbed.detail.shown = true;
    tabbed.detail.focus = DetailFocus::Graph;
    let mut wide = tabbed.clone();
    wide.size.columns = 160;

    assert_eq!(
        crate::update::banner::tab_rows_above_graph(&wide),
        0,
        "beside the graph there is no strip and no tab banner"
    );
    assert_eq!(
        crate::update::banner::tab_rows_above_graph(&tabbed),
        1,
        "the `[Graph] Detail` strip"
    );
    assert_eq!(
        tabbed.graph_rows() + 1,
        wide.graph_rows(),
        "the strip is a row the graph does not get"
    );

    // A live operation, banner focused, graph tab still the one shown: the
    // strip is still the whole cost, because the graph header carries the
    // kind word and its keys ride the footer.
    let mut live = tabbed.clone();
    live.banner_focus = true;
    live.in_progress = Some(mergewright_git::state::InProgressDetail {
        kind: mergewright_git::state::InProgress::Merge,
        step: None,
        head_name: None,
        onto: None,
        theirs: Ok(None),
    });
    assert_eq!(
        live.graph_rows(),
        tabbed.graph_rows(),
        "the banner costs the graph tab nothing: it is drawn in the header"
    );
}

/// **The regression this ticket most likely causes.** Below 100 columns
/// with the pane open on the **Graph** tab, a click must still land on the
/// row the user is looking at: the `[Graph] Detail` strip pushes the rows
/// down by one, and reserving the pane's hint bar
/// (`update::detail::tab_hint_rows`, m5-history-and-ghost-preview/29) must
/// not move them again — the bar is the Detail tab's row, not the graph's,
/// so `rows_above_graph` is unchanged and the click keeps landing where it
/// did (ticket 25's fix of exactly this shape, review M1).
///
/// **Mutation:** add one to `rows_above_graph`'s return and this goes red
/// at the assert below — which is what makes it a test of the offset and
/// not just of the click path.
#[test]
fn a_click_below_one_hundred_columns_still_lands_on_the_row_it_named() {
    use mergewright_ui::detail::DetailFocus;

    let id = JobId::new(20);
    let (mut state, merge_id) = merge_on_screen(id, 0);
    state.size = Size {
        columns: 80,
        rows: 12,
    };
    state.detail.shown = true;
    state.detail.focus = DetailFocus::Graph;
    assert!(
        matches!(
            crate::update::detail::effective_layout(
                state.detail.focus,
                &state.detail.pane_size,
                (state.size.columns, state.size.rows),
            ),
            mergewright_ui::detail_pane::DetailLayout::Tab { shown: false }
        ),
        "80 columns with the graph tab up is the layout this is about"
    );
    assert_eq!(
        crate::update::detail::tab_hint_rows(&state),
        0,
        "the Graph tab keeps the graph's own footer; the pane reserves nothing"
    );

    // The strip's row, then the first graph row: the second commit sits one
    // row below that.
    let strip = 1;
    let click = core(Event::Mouse(MouseInput {
        kind: MouseKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_ROWS + strip + 2, // under the cursor row and its second line (ease-pass/07b)
        modifiers: Modifiers::NONE,
    }));
    let (next, _) = update(&state, click);

    assert_eq!(
        next.history.as_ref().map(|history| history.cursor),
        Some(1),
        "the row the pointer was on"
    );
    assert_eq!(
        next.history
            .as_ref()
            .and_then(History::selected)
            .map(|row| row.commit.id),
        Some(merge_id)
    );
}

/// One size, one focus, one banner: the drawn rects and the mirror's.
fn the_mirror_agrees_at(
    width: u16,
    height: u16,
    live: bool,
    focus: mergewright_ui::detail::DetailFocus,
    banner_focus: bool,
) {
    use ratatui::layout::Rect;

    let bounds = |rect: Rect| geometry::Bounds {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
    };
    let mut state = AppState {
        size: crate::update::state::Size {
            columns: width,
            rows: height,
        },
        banner_focus,
        ..AppState::default()
    };
    state.detail.shown = true;
    state.detail.focus = focus;
    if live {
        state.in_progress = Some(mergewright_git::state::InProgressDetail {
            kind: mergewright_git::state::InProgress::Merge,
            step: None,
            head_name: None,
            onto: None,
            theirs: Ok(None),
        });
    }
    let layout =
        crate::update::detail::effective_layout(focus, &state.detail.pane_size, (width, height));
    let area = Rect {
        x: 0,
        y: 0,
        width,
        height,
    };
    let (graph, pane) = crate::view::frame::detail_rects(&state, layout, area);
    let mirrored = geometry::detail_split(
        layout,
        (width, height),
        crate::update::banner::tab_banner_rows(&state),
        crate::update::detail::tab_hint_rows(&state),
    );
    let at = format!(
        "{width}x{height}, live={live}, focus={focus:?}, \
         banner_focus={banner_focus}, layout={layout:?}"
    );
    // The pane rect exactly: it is what `detail_hit` tests a click against.
    assert_eq!(mirrored.1, pane.map(bounds), "pane rect at {at}");
    // The graph rect on the **vertical** axis exactly. Its width is
    // deliberately one wider here in the `Right` layout: m5/19 routes the
    // gutter column to the region it came out of rather than to nothing, so
    // the mirror keeps a cell the drawing path does not paint
    // (`detail_pane/layout.rs`'s `gutter`). That is the horizontal axis and
    // has its own test (`mergewright-ui`'s `pane_boundaries.rs`).
    assert_eq!(
        mirrored.0.map(|rect| (rect.y, rect.height)),
        graph.map(|rect| (rect.y, rect.height)),
        "graph rows at {at}"
    );
    assert_eq!(
        mirrored.0.is_some(),
        graph.is_some(),
        "graph drawn-or-not at {at}"
    );
}

#[test]
fn the_geometry_mirror_matches_the_rects_the_frame_hands_out() {
    use mergewright_ui::detail::DetailFocus;

    for width in 20u16..=240 {
        for height in [3u16, 8, 24, 40, 48] {
            for live in [false, true] {
                for focus in [DetailFocus::Graph, DetailFocus::Files, DetailFocus::Diff] {
                    for banner_focus in [false, true] {
                        the_mirror_agrees_at(width, height, live, focus, banner_focus);
                    }
                }
            }
        }
    }
}
