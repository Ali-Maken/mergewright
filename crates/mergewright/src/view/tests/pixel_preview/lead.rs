//! ease-pass/29 D5, the pixel tier's lead cell: a strip starts in the
//! gutter's gap cell, one cell left of the graph column. These hold what
//! that must not disturb: the gap cell's text, where the nodes sit when the
//! tiers swap, and screens with no gutter.

use mergewright_core::capabilities::RenderingTier;
use mergewright_graph::raster::{STRIP_LEAD_CELLS, lane_centre_x};
use mergewright_ui::graph_screen::GraphicsStatus;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use super::{closed, pixel_session, previewing, previewing_at};
use crate::session::Session;
use crate::session::for_tests::session;
use crate::update::AppState;
use crate::view::view_with_graphics;

/// [`draw_at`]'s cells as a buffer, with a chosen graphics status.
fn buffer_at(
    session: &Session,
    state: &AppState,
    status: &GraphicsStatus,
) -> (Buffer, Option<Rect>) {
    let mut terminal = Terminal::new(TestBackend::new(state.size.columns, state.size.rows))
        .expect("TestBackend reports its size");
    let mut pixels = None;
    terminal
        .draw(|frame| pixels = view_with_graphics(session, state, status, frame))
        .expect("TestBackend cannot fail to draw");
    (terminal.backend().buffer().clone(), pixels)
}

/// A dirty tree over a 100-commit history on a 100x30 screen, `HEAD` (row 0)
/// scrolled off, so the pinned uncommitted row shows; the cursor on `cursor`
/// (row `top + 1` puts its second line, the tall row, in view).
fn pinned(cursor: usize, on_working_tree: bool) -> AppState {
    use crate::update::Size;
    use crate::view::tests::{counts, history_of, oid, refs_with_head};
    let mut history = history_of(100, 0);
    history.top = 18;
    history.cursor = cursor;
    let state = AppState {
        history: Some(history),
        refs: Some(std::sync::Arc::new(refs_with_head(Some(oid(100))))),
        working_tree: Some(counts(1, 0, 0)),
        settled_tier: Some(RenderingTier::Pixel),
        on_working_tree,
        size: Size {
            columns: 100,
            rows: 30,
        },
        ..AppState::default()
    };
    assert!(state.uncommitted_row_pinned(), "premise: the row is pinned");
    state
}

/// ease-pass/29 D5, stop condition (a): a strip's image starts in the
/// gutter's gap cell, so that cell's text must be a space with the background
/// the strip's own first cell has on that row (what the image is drawn over).
/// Cursor row, its second line, the pinned row and a plain row all in view.
#[test]
fn the_gutter_gap_cell_under_a_strip_is_blank() {
    let pixel = pixel_session();
    let states = vec![
        ("cursor mid-viewport", pinned(24, false)),
        ("cursor on the top row", pinned(18, false)),
        ("cursor on the pinned row", pinned(18, true)),
        ("preview fixture, closed", closed(&previewing())),
    ];
    for (name, state) in states {
        let (buffer, graph) = buffer_at(&pixel, &state, &GraphicsStatus::Ok);
        let graph = graph.unwrap_or_else(|| panic!("{name}: strips draw"));
        assert!(graph.x >= 1, "{name}: a gutter cell left of the graph");
        let gap = graph.x - 1;
        for y in graph.y..graph.bottom() {
            let cell = &buffer[(gap, y)];
            assert_eq!(cell.symbol(), " ", "{name}: line {y} gap cell holds text");
            assert_eq!(
                cell.bg,
                buffer[(graph.x, y)].bg,
                "{name}: line {y} gap cell background differs from the strip's first cell"
            );
        }
    }
}

/// The glyph a real (`Commit N`) row draws as its node, and its cell column.
fn node_column(buffer: &Buffer, graph: Rect, y: u16) -> Option<u16> {
    let text: String = (0..buffer.area.width)
        .map(|x| buffer[(x, y)].symbol())
        .collect();
    if !text.contains("Commit ") {
        return None;
    }
    (graph.x..graph.right()).find(|&x| matches!(buffer[(x, y)].symbol(), "●" | "◉" | "○" | "◇"))
}

/// ease-pass/29 D5: the pixel node of a row sits where the Unicode glyph of
/// the same row sits, so a preview opening, a graphics failure and `F3` swap
/// the tiers without moving a node sideways. Pixel node centre in screen
/// pixels = strip's left column x cell width + `lane_centre_x`; glyph centre
/// = (glyph column + 0.5) x cell width. The fixture is one lane (every real
/// row on lane 0); lane k is held by the unit test `pen::tests::
/// a_lane_centres_on_its_unicode_node_cell`.
#[test]
fn preview_nodes_sit_where_the_pixel_nodes_were() {
    let pixel = pixel_session();
    let (open, _) = previewing_at();
    let shut = closed(&open);
    let (_, graph) = buffer_at(&pixel, &shut, &GraphicsStatus::Ok);
    let graph = graph.expect("premise: strips draw, closed");
    let lead = f32::from(u16::try_from(STRIP_LEAD_CELLS).expect("small"));
    // The frames that draw the column as text: a preview's picture, the
    // session a failed graphics probe leaves (Unicode tier), and `F3`. (A
    // `Failed` status alone only adds a header message; the column is then
    // not drawn as text, so there is no node to compare.)
    let unicode_session = session();
    let f3 = AppState {
        renderer_override: Some(RenderingTier::Unicode),
        ..shut.clone()
    };
    let frames = vec![
        ("preview open", &pixel, open.clone()),
        (
            "failed probe (unicode session)",
            &unicode_session,
            shut.clone(),
        ),
        ("F3 to unicode", &pixel, f3),
    ];
    for (name, with, state) in frames {
        let (buffer, _) = buffer_at(with, &state, &GraphicsStatus::Ok);
        let mut checked = 0;
        for y in graph.y..graph.bottom() {
            let Some(column) = node_column(&buffer, graph, y) else {
                continue;
            };
            for width in [6.0_f32, 7.0, 8.0, 9.0, 10.0, 17.0, 24.0] {
                let pixel_node = (f32::from(graph.x) - lead) * width + lane_centre_x(0, width);
                let glyph = (f32::from(column) + 0.5) * width;
                assert!(
                    (pixel_node - glyph).abs() < 0.01,
                    "{name}, line {y}, cell width {width}: pixel node at {pixel_node}, glyph at {glyph}"
                );
            }
            checked += 1;
        }
        assert!(checked > 5, "{name}: premise: real rows drawn ({checked})");
    }
}

/// ease-pass/29 D5.6: below four columns the body has no gutter, so there is
/// no gap cell to start an image in: no pixel rectangle is asked for and the
/// column draws as text (the path a preview takes).
#[test]
fn no_strip_without_a_gutter() {
    let pixel = pixel_session();
    let mut state = closed(&previewing());
    state.size = crate::update::Size {
        columns: 3,
        rows: 30,
    };
    let (_, graph) = buffer_at(&pixel, &state, &GraphicsStatus::Ok);
    assert_eq!(graph, None, "no strip at 3 columns");
    // And the column is text there, not left blank for an image.
    let (text, _) = buffer_at(&pixel, &state, &GraphicsStatus::Ok);
    assert!(
        (0..state.size.rows).any(|y| (0..3).any(|x| text[(x, y)].symbol() != " ")),
        "the 3-column screen holds drawn cells"
    );
    // And one column more has a gutter, hence a strip.
    state.size.columns = 20;
    let (_, graph) = buffer_at(&pixel, &state, &GraphicsStatus::Ok);
    assert!(
        graph.is_some_and(|graph| graph.x >= 1),
        "a strip with a gutter"
    );
}
