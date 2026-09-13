//! The Unicode graph column drawn over every corpus shape.
//!
//! Each snapshot is the column exactly as rendered, then the lane-palette
//! slot every cell paints in (`a` = `lane-1` … `l` = `lane-12`, `.` none),
//! then the commit it draws. Read them as pictures; do not just re-accept.

mod support;

use graft_core::capabilities::{
    Capabilities, CellSize, CellSizeSource, ColorDepth, ProbeOutcome, RenderingTier, Supported,
    Theme as TerminalTheme, ThemeHint, ThemeSource, TmuxStatus,
};
use graft_fixtures::Shape;
use graft_ui::graph_column::{GraphColumn, GraphLine};
use graft_ui::paint::color;
use graft_ui::theme::{LANE_COUNT, Theme, ThemeKind};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::Color;

use support::corpus::{caption, for_each_shape, lay_out};

#[test]
fn every_shape_draws_as_snapshotted_and_the_same_in_dark_and_light() {
    for_each_shape(|shape, dir| {
        let laid = lay_out(shape, dir);
        let lines = laid.lines();
        let width = laid.width().max(1);
        let dark = draw(&lines, ThemeKind::Dark, width);
        let light = draw(&lines, ThemeKind::Light, width);
        assert_eq!(
            dark, light,
            "{shape}: lane colours are independent of the theme (PROMPT.md §5)"
        );

        let theme = theme(ThemeKind::Dark, ColorDepth::TrueColor);
        let mut text = format!("shape: {shape} (dark = light, asserted), truecolor\n\n");
        for (line, y) in lines.iter().zip(0u16..) {
            let (glyphs, slots) = describe(&dark, y, &theme);
            text.push_str(&format!("{glyphs}  {slots}  {}\n", caption(line.row)));
        }
        insta::assert_snapshot!(shape.name(), text);
    });
}

#[test]
fn a_lane_keeps_its_colour_across_viewports_that_overlap_by_half() {
    let dir = graft_fixtures::borrow(Shape::FeatureBranches);
    let laid = lay_out(Shape::FeatureBranches, &dir);
    let lines = laid.lines();
    let width = laid.width();
    let height: u16 = 6;
    let half: u16 = height / 2;

    let top = draw(&lines[..usize::from(height)], ThemeKind::Dark, width);
    let lower = draw(
        &lines[usize::from(half)..usize::from(half + height)],
        ThemeKind::Dark,
        width,
    );

    let mut coloured = 0;
    for y in 0..half {
        for x in 0..width {
            let above = &top[(x, half + y)];
            let below = &lower[(x, y)];
            assert_eq!(above.symbol(), below.symbol(), "glyph at {x},{y}");
            assert_eq!(above.fg, below.fg, "colour at {x},{y}");
            coloured += usize::from(above.fg != Color::Reset);
        }
    }
    assert!(
        coloured > usize::from(half),
        "the overlap draws lanes, not blank space"
    );
}

fn draw(lines: &[GraphLine<'_>], kind: ThemeKind, width: u16) -> Buffer {
    let theme = theme(kind, ColorDepth::TrueColor);
    let height = u16::try_from(lines.len())
        .unwrap_or_else(|_| panic!("{} rows do not fit a test backend", lines.len()));
    let mut terminal = Terminal::new(TestBackend::new(width, height))
        .unwrap_or_else(|why| panic!("a test backend always opens: {why}"));
    terminal
        .draw(|frame| frame.render_widget(GraphColumn::new(lines, &theme), frame.area()))
        .unwrap_or_else(|why| panic!("drawing into a test backend cannot fail: {why}"));
    terminal.backend().buffer().clone()
}

/// Row `y` as its glyphs and as the palette slot of each cell.
fn describe(buf: &Buffer, y: u16, theme: &Theme) -> (String, String) {
    let mut glyphs = String::new();
    let mut slots = String::new();
    for x in 0..buf.area.width {
        let cell = &buf[(x, y)];
        glyphs.push_str(cell.symbol());
        let slot = (0..LANE_COUNT).find(|&lane| color(theme.lane(lane)) == cell.fg);
        slots.push(slot.map_or('.', |lane| char::from(b"abcdefghijkl"[lane])));
    }
    (glyphs, slots)
}

fn theme(kind: ThemeKind, color_depth: ColorDepth) -> Theme {
    Theme::new(
        kind,
        &Capabilities {
            tier: RenderingTier::Unicode,
            pixel_protocol: None,
            kitty_graphics: Supported::No,
            sixel: Supported::No,
            iterm2_images: false,
            kitty_keyboard: Supported::No,
            synchronized_output: Supported::No,
            cell_size: CellSize {
                width: 8,
                height: 16,
                source: CellSizeSource::Assumed,
            },
            theme: ThemeHint {
                theme: TerminalTheme::Dark,
                source: ThemeSource::Assumed,
            },
            color_depth,
            no_color: false,
            is_tty: true,
            over_ssh: false,
            tmux: TmuxStatus::Outside,
            probe: ProbeOutcome::Answered,
            probe_budget: std::time::Duration::from_millis(250),
        },
    )
}
