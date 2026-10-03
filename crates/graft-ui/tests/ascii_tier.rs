//! The ASCII tier's glyph table over the graph screen (ticket
//! m1-the-graph/19; DESIGN.md §4.2, ADR-0016): criss-cross, foxtrot, octopus
//! and the widest fixture, at 80×24, with and without colour.
//!
//! Every glyph drawn must come from the six-character ASCII table
//! (`* | / \ - +`) or a space — asserted directly, not just eyeballed.
//! Read the snapshots as pictures too; do not just re-accept.

mod support;

use graft_core::capabilities::ColorDepth;
use graft_fixtures::Shape;
use graft_ui::graph_screen::{CHROME_ROWS, GUTTER, GraphScreen};
use graft_ui::theme::{Theme, ThemeKind};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;

use support::corpus::{Laid, for_each_shape, lay_out};
use support::{screen, themed};

const COLUMNS: u16 = 80;
const ROWS: u16 = 24;

/// The same crowded shapes `lanes_without_hue` checks, plus the widest one
/// in the corpus.
fn shapes() -> Vec<Shape> {
    let mut shapes = vec![
        Shape::CrissCrossMerge,
        Shape::FoxtrotMerge,
        Shape::OctopusMerge,
    ];
    let widest = widest_shape();
    if !shapes.contains(&widest) {
        shapes.push(widest);
    }
    shapes
}

/// The corpus shape whose graph is widest; ties go to the earlier shape.
fn widest_shape() -> Shape {
    let mut widest = (0, Shape::Linear);
    for_each_shape(|shape, dir| {
        let width = lay_out(shape, dir).width();
        if width > widest.0 {
            widest = (width, shape);
        }
    });
    widest.1
}

fn draw_ascii(laid: &Laid, theme: &Theme) -> Buffer {
    let lines = laid.lines();
    let mut view = screen::view(&lines, usize::from(ROWS - CHROME_ROWS), 1);
    view.ascii = true;
    let mut terminal = Terminal::new(TestBackend::new(COLUMNS, ROWS))
        .unwrap_or_else(|why| panic!("a test backend always opens: {why}"));
    terminal
        .draw(|frame| frame.render_widget(GraphScreen::new(&view, theme), frame.area()))
        .unwrap_or_else(|why| panic!("drawing into a test backend cannot fail: {why}"));
    terminal.backend().buffer().clone()
}

#[test]
fn crowded_shapes_draw_only_the_ascii_glyph_table() {
    for shape in shapes() {
        let dir = graft_fixtures::borrow(shape);
        let laid = lay_out(shape, &dir);
        for (name, depth, no_color) in [
            ("truecolor", ColorDepth::TrueColor, false),
            ("no-color", ColorDepth::TrueColor, true),
        ] {
            let theme = themed(ThemeKind::Dark, depth, no_color);
            let buffer = draw_ascii(&laid, &theme);

            let graph_width = laid.width().min(COLUMNS);
            let drawn = laid.rows.len().min(usize::from(ROWS - CHROME_ROWS));
            for y in (1..).take(drawn) {
                // The graph column starts after the two-cell gutter, whose
                // cell 0 holds the cursor's `>` on the cursor row (ADR-0058).
                let gutter = if y == 2 { ">" } else { " " };
                assert_eq!(
                    buffer[(0, y)].symbol(),
                    gutter,
                    "{shape} {name} gutter at row {y}"
                );
                for x in GUTTER..GUTTER + graph_width {
                    let symbol = buffer[(x, y)].symbol();
                    assert!(
                        symbol.chars().all(|ch| "*|/\\-+ ".contains(ch)),
                        "{shape} {name} at {x},{y}: {symbol:?} is not in the ASCII glyph table"
                    );
                }
            }

            let mut text = format!("shape: {shape}, {name}, {COLUMNS}x{ROWS}, cursor on row 2\n\n");
            for y in 0..ROWS {
                let line: String = (0..COLUMNS).map(|x| buffer[(x, y)].symbol()).collect();
                text.push_str(line.trim_end());
                text.push('\n');
            }
            insta::assert_snapshot!(format!("ascii-{shape}-{name}"), text);
        }
    }
}
