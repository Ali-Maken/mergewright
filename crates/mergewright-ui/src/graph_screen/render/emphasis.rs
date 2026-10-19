//! The cursor row's emphasis: its bold, its fill from the marker slot to
//! the body's right edge, or a dimmed row's faint ink. Moved out of
//! `render.rs` (ease-pass/07b) when the second line came to share it.

use mergewright_core::capabilities::ColorDepth;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

use super::raised;
use crate::graph_column::RowPaint;
use crate::graph_screen::row_columns::Columns;
use crate::graph_screen::{GraphEntry, GraphScreen};
use crate::paint::color;
use crate::theme::Token;

impl GraphScreen<'_> {
    /// The text style, the trailing-column style, and whether the row falls
    /// back to the Palette16 rule instead of a background fill: `Highlighted`
    /// paints bold and, where the terminal can show it, a raised background;
    /// `Dimmed` paints faint; anything else is plain.
    ///
    /// At [`ColorDepth::Palette16`], `bg`/`surface`/`surface-raised` collapse
    /// to the same index (KNOWN_GAPS.md M0), so the raised background step
    /// would paint nothing a viewer can see — the marker slot's rule
    /// (DESIGN.md §4.6) carries the highlight there instead, so the
    /// background step is skipped rather than drawn for nothing. `NO_COLOR`
    /// ([`ColorDepth::None`]) has no background step either, so it takes the
    /// same rule — uncoloured, its shape alone marks the row.
    ///
    /// The fill starts at the marker slot (ADR-0058 §2): the gutter and the
    /// graph column keep the ground at every tier, so no lane on the cursor
    /// row sinks into the fill, and a Pixel strip's transparent pixels show
    /// the ground as on every other row. It runs to `area`'s right edge —
    /// the body's — so the one-cell margin stays ground too.
    pub(super) fn row_emphasis(
        &self,
        entry: &GraphEntry<'_>,
        y: u16,
        area: Rect,
        columns: &Columns,
        buf: &mut Buffer,
    ) -> (Style, Style) {
        let mut style = self.ink(entry, self.theme.token(Token::Text));
        let mut trailing_style = self.ink(entry, self.theme.token(Token::TextMuted));
        let no_background = matches!(
            self.theme.color_depth(),
            ColorDepth::Palette16 | ColorDepth::None
        );

        if raised(entry) {
            style = style.add_modifier(Modifier::BOLD);
            trailing_style = trailing_style.add_modifier(Modifier::BOLD);
            if !no_background {
                let fill = color(self.theme.token(Token::Selection));
                let x = columns.marker_x.min(area.right());
                buf.set_style(
                    Rect {
                        x,
                        y,
                        width: area.right() - x,
                        height: 1,
                    },
                    Style::new().bg(fill),
                );
            }
        } else if entry.line.paint == RowPaint::Dimmed {
            style = style.add_modifier(Modifier::DIM);
            trailing_style = trailing_style.add_modifier(Modifier::DIM);
        }

        (style, trailing_style)
    }
}
