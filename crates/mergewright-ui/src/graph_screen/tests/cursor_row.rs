//! The cursor row (ADR-0058 §2, §3, §5; DESIGN §1.0 "Cursor row"): the fill
//! runs from the marker slot to the body's right edge, the gutter and graph
//! column keep the ground, the cursor is `▸` in gutter cell 0, a drop target
//! is `→` in the slot, and text on the fill clears 4.5:1.

use mergewright_core::capabilities::{ColorDepth, Rgb};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};

use super::super::lanes::graph_width;
use super::super::row_columns::Columns;
use super::super::{GUTTER, GraphScreen, GraphView, Tracking, WalkView};
use super::{branch, linear, view};
use crate::graph_column::RowPaint;
use crate::paint::color;
use crate::test_support::terminal;
use crate::theme::{Theme, ThemeKind, Token, contrast};
use mergewright_term::sanitize_line;

const WIDTH: u16 = 60;
const HEIGHT: u16 = 7;
/// Row 1 of the body is the cursor; the body starts on screen row 1.
const CURSOR: usize = 1;

fn themed(depth: ColorDepth, no_color: bool) -> Theme {
    Theme::new(ThemeKind::Dark, &terminal(depth, no_color))
}

fn render(view: &GraphView<'_>, theme: &Theme) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).expect("test backend opens");
    terminal
        .draw(|frame| frame.render_widget(GraphScreen::new(view, theme), frame.area()))
        .expect("drawing into a test backend cannot fail");
    terminal.backend().buffer().clone()
}

/// The body rect and its columns, as the renderer computes them.
fn geometry(view: &GraphView<'_>, theme: &Theme) -> (Rect, Columns) {
    let screen = GraphScreen::new(view, theme);
    let (body, _, fold) = screen.body_layout(Rect::new(0, 0, WIDTH, HEIGHT));
    let width = graph_width(body, &fold);
    (body, Columns::compute(body, width))
}

fn cursor_y(body: Rect) -> u16 {
    body.y + u16::try_from(CURSOR).expect("small")
}

fn rgb(color: Color) -> Rgb {
    match color {
        Color::Rgb(red, green, blue) => Rgb { red, green, blue },
        other => panic!("truecolor draws Rgb, got {other:?}"),
    }
}

fn badged(rows: &[mergewright_graph::row::Row], ascii: bool) -> GraphView<'_> {
    let mut view = view(rows, WalkView::Complete { rows: 3 }, CURSOR);
    view.ascii = ascii;
    view.rows[CURSOR].badges = vec![branch(
        "feature/csv",
        false,
        Tracking::InSync {
            remote: sanitize_line("origin"),
        },
    )];
    view
}

/// Unicode and ASCII tiers: gutter and graph column on the ground, the fill
/// from the marker slot to the body's right edge, the margin on the ground.
#[test]
fn the_fill_runs_from_the_marker_slot_to_the_body_edge() {
    let rows = linear(3);
    let theme = themed(ColorDepth::TrueColor, false);
    let selection = color(theme.token(Token::Selection));
    for ascii in [false, true] {
        let view = badged(&rows, ascii);
        let buffer = render(&view, &theme);
        let (body, columns) = geometry(&view, &theme);
        let y = cursor_y(body);
        assert!(columns.marker_x > body.x, "the graph column has width");
        for x in 0..columns.marker_x {
            assert_ne!(
                buffer[(x, y)].bg,
                selection,
                "ascii={ascii}: x={x} keeps the ground"
            );
            assert_eq!(
                buffer[(x, y)].bg,
                buffer[(x, y - 1)].bg,
                "ascii={ascii}: x={x} is the ground the row above has"
            );
        }
        for x in columns.marker_x..body.right() {
            assert_eq!(
                buffer[(x, y)].bg,
                selection,
                "ascii={ascii}: x={x} is filled"
            );
        }
        for x in body.right()..WIDTH {
            assert_ne!(buffer[(x, y)].bg, selection, "ascii={ascii}: margin x={x}");
        }
    }
}

/// Pixel tier: the graph column's cells are left blank for the strip, and
/// on the cursor row they show the ground, not the fill.
#[test]
fn at_the_pixel_tier_the_graph_cells_keep_the_ground() {
    let rows = linear(3);
    let theme = themed(ColorDepth::TrueColor, false);
    let selection = color(theme.token(Token::Selection));
    let mut view = badged(&rows, false);
    view.pixel = true;
    let buffer = render(&view, &theme);
    let (body, columns) = geometry(&view, &theme);
    let y = cursor_y(body);
    for x in body.x..columns.marker_x {
        assert_ne!(buffer[(x, y)].bg, selection, "pixel graph cell x={x}");
    }
    assert_eq!(buffer[(columns.marker_x, y)].bg, selection);
}

/// `▸` (ASCII `>`) in gutter cell 0 of the cursor row only, accent and bold;
/// gutter cell 1 blank; the marker slot carries no `▏` for the cursor.
#[test]
fn the_cursor_is_marked_in_the_gutter_not_the_slot() {
    let rows = linear(3);
    let theme = themed(ColorDepth::TrueColor, false);
    let accent = color(theme.token(Token::Accent));
    for (ascii, mark) in [(false, "▸"), (true, ">")] {
        let view = badged(&rows, ascii);
        let buffer = render(&view, &theme);
        let (body, columns) = geometry(&view, &theme);
        let y = cursor_y(body);
        let gutter = body.x - GUTTER;
        assert_eq!(buffer[(gutter, y)].symbol(), mark, "ascii={ascii}");
        assert_eq!(buffer[(gutter, y)].fg, accent);
        assert!(buffer[(gutter, y)].modifier.contains(Modifier::BOLD));
        assert_eq!(buffer[(gutter + 1, y)].symbol(), " ");
        assert_eq!(
            buffer[(columns.marker_x, y)].symbol(),
            " ",
            "no ▏ for the cursor"
        );
        for other in [y - 1, y + 1] {
            assert_eq!(
                buffer[(gutter, other)].symbol(),
                " ",
                "row {other} has no mark"
            );
        }
    }
}

/// The compare base keeps `▏` in the slot, also on the cursor row.
#[test]
fn the_compare_base_keeps_its_rule_on_the_cursor_row() {
    let rows = linear(3);
    let theme = themed(ColorDepth::TrueColor, false);
    let mut view = badged(&rows, false);
    view.rows[CURSOR].compare_marked = true;
    let buffer = render(&view, &theme);
    let (body, columns) = geometry(&view, &theme);
    let y = cursor_y(body);
    assert_eq!(buffer[(columns.marker_x, y)].symbol(), "▏");
    assert_eq!(buffer[(body.x - GUTTER, y)].symbol(), "▸");
}

/// A drop target draws `→` (ASCII `=`) in the slot, keeps the fill and bold,
/// and no gutter glyph; the cursor and the drop target never share a glyph.
#[test]
fn a_drop_target_never_looks_like_the_cursor() {
    let rows = linear(3);
    let theme = themed(ColorDepth::TrueColor, false);
    let selection = color(theme.token(Token::Selection));
    for (ascii, arrow) in [(false, "→"), (true, "=")] {
        let mut view = badged(&rows, ascii);
        view.rows[2].line.paint = RowPaint::DropTarget;
        let buffer = render(&view, &theme);
        let (body, columns) = geometry(&view, &theme);
        let (cursor, drop) = (cursor_y(body), cursor_y(body) + 1);
        let gutter = body.x - GUTTER;

        assert_eq!(buffer[(columns.marker_x, drop)].symbol(), arrow);
        assert_eq!(buffer[(gutter, drop)].symbol(), " ", "no gutter glyph");
        assert_eq!(buffer[(columns.marker_x + 4, drop)].bg, selection);

        let marks = |y: u16| -> Vec<String> {
            [gutter, columns.marker_x]
                .iter()
                .map(|x| buffer[(*x, y)].symbol().to_owned())
                .filter(|s| s.trim() != "")
                .collect()
        };
        let (on_cursor, on_drop) = (marks(cursor), marks(drop));
        assert!(!on_cursor.is_empty() && !on_drop.is_empty());
        assert!(
            on_cursor.iter().all(|glyph| !on_drop.contains(glyph)),
            "ascii={ascii}: cursor {on_cursor:?} vs drop {on_drop:?}"
        );
    }
}

/// Every foreground on the fill clears 4.5:1 against `selection` (dark B):
/// the branch badge, the subject, the author chip, the date and the sha.
#[test]
fn text_on_the_fill_is_legible() {
    let rows = linear(3);
    let theme = themed(ColorDepth::TrueColor, false);
    let selection = rgb(color(theme.token(Token::Selection)));
    let view = badged(&rows, false);
    let buffer = render(&view, &theme);
    let (body, columns) = geometry(&view, &theme);
    let y = cursor_y(body);
    let mut checked = 0;
    for x in columns.marker_x..body.right() {
        let cell = &buffer[(x, y)];
        if cell.symbol().trim().is_empty() {
            continue;
        }
        let ratio = contrast(rgb(cell.fg), selection);
        assert!(ratio >= 4.5, "x={x} {:?}: {ratio:.2}:1", cell.symbol());
        checked += 1;
    }
    assert!(
        checked > 20,
        "badge, subject, author, date, sha drawn: {checked}"
    );
}

/// 16 colours and `NO_COLOR`: no fill; the badge draws `text`, bold; `▸`.
#[test]
fn without_a_fill_the_badge_is_text_and_bold() {
    let rows = linear(3);
    for (depth, no_color) in [
        (ColorDepth::Palette16, false),
        (ColorDepth::TrueColor, true),
    ] {
        let theme = themed(depth, no_color);
        let view = badged(&rows, false);
        let buffer = render(&view, &theme);
        let (body, columns) = geometry(&view, &theme);
        let y = cursor_y(body);
        let text = color(theme.token(Token::Text));
        for x in 0..WIDTH {
            assert_eq!(
                buffer[(x, y)].bg,
                buffer[(x, y - 1)].bg,
                "{depth:?}: no fill at {x}"
            );
        }
        let badge = columns.subject_x;
        assert_eq!(buffer[(badge, y)].fg, text, "{depth:?} no_color={no_color}");
        assert!(buffer[(badge, y)].modifier.contains(Modifier::BOLD));
        assert_eq!(buffer[(body.x - GUTTER, y)].symbol(), "▸");
    }
}
