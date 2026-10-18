//! Lanes where hue can't carry them: 256 colours, 16 colours and `NO_COLOR`,
//! over the shapes that crowd lanes together, on an 80×24 graph screen.
//!
//! Each snapshot is the screen's text, then each graph row's colour per cell
//! (`0`–`f` a 16-colour index, `a`–`l` a 256-colour lane slot as
//! `graph_column`'s snapshots spell it, `.` no colour). At 16 colours and
//! `NO_COLOR` lanes in odd columns draw dashed (`╎`, `╌`). Read them as
//! pictures; do not just re-accept.

mod support;

use mergewright_core::capabilities::ColorDepth;
use mergewright_fixtures::Shape;
use mergewright_ui::graph_screen::{CHROME_ROWS, GUTTER, GraphScreen};
use mergewright_ui::paint::color;
use mergewright_ui::theme::{LANE_COUNT, Theme, ThemeKind};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::Color;

use support::corpus::{Laid, for_each_shape, lay_out};
use support::{screen, themed};

const COLUMNS: u16 = 80;
const ROWS: u16 = 24;

/// The three depths below truecolor, as snapshot names spell them.
const DEPTHS: [(&str, ColorDepth, bool); 3] = [
    ("256", ColorDepth::Palette256, false),
    ("16", ColorDepth::Palette16, false),
    ("no-color", ColorDepth::TrueColor, true),
];

#[test]
fn crowded_shapes_at_every_depth_below_truecolor() {
    let mut shapes = vec![
        Shape::CrissCrossMerge,
        Shape::FoxtrotMerge,
        Shape::OctopusMerge,
        Shape::Linear,
        // Compaction's shape (m8-polish-and-ship/22): 15 columns, outside
        // `Shape::ALL`, so `widest_shape` never picks it.
        Shape::ManyLanes,
    ];
    let widest = widest_shape();
    if !shapes.contains(&widest) {
        shapes.push(widest);
    }

    for shape in shapes {
        let dir = mergewright_fixtures::borrow(shape);
        let laid = lay_out(shape, &dir);
        for (name, depth, no_color) in DEPTHS {
            let theme = themed(ThemeKind::Dark, depth, no_color);
            let (buffer, graph_width) = draw_screen(&laid, &theme);
            let mut text = format!(
                "shape: {shape}{}, {name}, {COLUMNS}x{ROWS}, cursor on row 2\n\n",
                if shape == widest { " (widest)" } else { "" }
            );
            for y in 0..ROWS {
                let line: String = (0..COLUMNS).map(|x| buffer[(x, y)].symbol()).collect();
                text.push_str(line.trim_end());
                text.push('\n');
            }
            text.push_str("\ngraph colour per cell:\n");
            let drawn = laid.rows.len().min(usize::from(ROWS - CHROME_ROWS));
            for y in (1..).take(drawn) {
                // The graph column starts after the two-cell gutter (ADR-0058).
                let codes: String = (GUTTER..GUTTER + graph_width)
                    .map(|x| code(buffer[(x, y)].fg, &theme))
                    .collect();
                text.push_str(&codes);
                text.push('\n');
            }
            insta::assert_snapshot!(format!("{shape}-{name}"), text);
        }
    }
}

#[test]
fn neighbouring_lanes_that_share_a_colour_never_share_a_glyph() {
    let mut shared = 0;
    for_each_shape(|shape, dir| {
        let laid = lay_out(shape, dir);
        for (name, depth, no_color) in &DEPTHS[1..] {
            let theme = themed(ThemeKind::Dark, *depth, *no_color);
            let (buffer, graph_width) = draw_screen(&laid, &theme);
            let drawn = laid.rows.len().min(usize::from(ROWS - CHROME_ROWS));
            for y in (1..).take(drawn) {
                // Lane cells are the even ones; compare each with the next.
                // After the two-cell gutter (ADR-0058).
                for x in (GUTTER..GUTTER + graph_width.saturating_sub(2)).step_by(2) {
                    let (left, right) = (&buffer[(x, y)], &buffer[(x + 2, y)]);
                    let straight = |s: &str| matches!(s, "│" | "╎");
                    if straight(left.symbol()) && straight(right.symbol()) && left.fg == right.fg {
                        shared += 1;
                        assert_ne!(
                            left.symbol(),
                            right.symbol(),
                            "{shape} at {name}: same colour and glyph side by side at {x},{y}"
                        );
                    }
                }
            }
        }
    });
    assert!(
        shared > 0,
        "the corpus has same-coloured neighbours to check"
    );
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

/// The screen drawn, and its graph column's width.
fn draw_screen(laid: &Laid, theme: &Theme) -> (Buffer, u16) {
    let lines = laid.lines();
    let view = screen::view(&lines, usize::from(ROWS - CHROME_ROWS), 1);
    let graph_width = screen::graph_width(&view, theme, COLUMNS, ROWS);
    let mut terminal = Terminal::new(TestBackend::new(COLUMNS, ROWS))
        .unwrap_or_else(|why| panic!("a test backend always opens: {why}"));
    terminal
        .draw(|frame| frame.render_widget(GraphScreen::new(&view, theme), frame.area()))
        .unwrap_or_else(|why| panic!("drawing into a test backend cannot fail: {why}"));
    (terminal.backend().buffer().clone(), graph_width)
}

/// One cell's foreground as a single character (see the module docs).
fn code(fg: Color, theme: &Theme) -> char {
    match fg {
        Color::Reset => '.',
        Color::Indexed(index) if index < 16 => {
            char::from_digit(u32::from(index), 16).unwrap_or('?')
        }
        other => (0..LANE_COUNT)
            .find(|&lane| color(theme.lane(lane)) == other)
            .map_or('?', |lane| char::from(b"abcdefghijkl"[lane])),
    }
}
