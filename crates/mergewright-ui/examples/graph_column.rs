//! Prints the Unicode graph column for corpus shapes in truecolor, for a
//! person to look at in a real terminal (m1-the-graph/08). The graph screen
//! itself arrives in ticket 09; until then this is how to see the column.
//!
//! `cargo run -p mergewright-ui --example graph_column [shape-name …]` — no names
//! prints every shape.

#[path = "../tests/support/corpus.rs"]
mod corpus;

use mergewright_core::capabilities::{
    Capabilities, CellSize, CellSizeSource, ColorDepth, ProbeOutcome, RenderingTier, Supported,
    Theme as TerminalTheme, ThemeHint, ThemeSource, TmuxStatus,
};
use mergewright_ui::graph_column::GraphColumn;
use mergewright_ui::theme::{Theme, ThemeKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};
use ratatui::widgets::Widget;

fn main() {
    let wanted: Vec<String> = std::env::args().skip(1).collect();
    let theme = Theme::new(ThemeKind::Dark, &truecolor_terminal());
    corpus::for_each_shape(|shape, dir| {
        if !wanted.is_empty() && !wanted.iter().any(|name| name == shape.name()) {
            return;
        }
        let laid = corpus::lay_out(shape, dir);
        let lines = laid.lines();
        let area = Rect::new(0, 0, laid.width().max(1), cells(lines.len()));
        let mut buf = Buffer::empty(area);
        GraphColumn::new(&lines, &theme).render(area, &mut buf);

        println!("── {shape} ──");
        for (line, y) in lines.iter().zip(0..area.height) {
            println!("{}  {}", ansi_row(&buf, y), corpus::caption(&line.row));
        }
        println!();
    });
}

/// A count as a cell count. Saturating is safe: a corpus shape is a few
/// dozen rows, and nothing past the buffer is drawn anyway.
fn cells(count: usize) -> u16 {
    u16::try_from(count).unwrap_or(u16::MAX)
}

/// One buffer row as text with SGR colour, reset at the end.
fn ansi_row(buf: &Buffer, y: u16) -> String {
    let mut out = String::new();
    for x in 0..buf.area.width {
        let cell = &buf[(x, y)];
        match cell.fg {
            Color::Rgb(red, green, blue) => {
                out.push_str(&format!("\x1b[38;2;{red};{green};{blue}m"));
            }
            _ => out.push_str("\x1b[39m"),
        }
        if cell.modifier.contains(Modifier::BOLD) {
            out.push_str("\x1b[1m");
        }
        out.push_str(cell.symbol());
        out.push_str("\x1b[0m");
    }
    out
}

fn truecolor_terminal() -> Capabilities {
    Capabilities {
        tier: RenderingTier::Unicode,
        pixel_protocol: None,
        kitty_graphics: Supported::No,
        kitty_deflate: false,
        answers_dsr: false,
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
        color_depth: ColorDepth::TrueColor,
        no_color: false,
        is_tty: true,
        over_ssh: false,
        tmux: TmuxStatus::Outside,
        probe: ProbeOutcome::Answered,
        probe_budget: std::time::Duration::from_millis(250),
    }
}
