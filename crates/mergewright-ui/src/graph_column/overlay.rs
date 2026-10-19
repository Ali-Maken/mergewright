//! The fold's overlay on a graph row: the rail folded lanes sit behind,
//! and one stub for the row's folded edges (ADR-0022). Moved out of
//! `graph_column/mod.rs` (ease-pass/07b) when the second line grew it.

use mergewright_graph::fold::{DrawnRow, Fold, Toward};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

use super::style::{lane_x, off_thread};
use super::{GraphColumn, GraphLine, RAIL, STUB_ABOVE, STUB_BELOW};
use crate::paint::{color, foreground};
use crate::theme::Token;

impl GraphColumn<'_> {
    /// What a fold adds to row `y`: the rail, and one stub for the row's
    /// folded edges — none when `stubs_of` is `None` (a second line).
    pub(super) fn fold_overlay(
        &self,
        buf: &mut Buffer,
        area: Rect,
        y: u16,
        line: &GraphLine<'_>,
        fold: &Fold,
        stubs_of: Option<&DrawnRow<'_>>,
    ) {
        // A node whose column folded sits on the rail (ease-pass 06b): the
        // node shows there, not the rail, so no row loses its commit.
        let node_on_rail = fold.drawn(line.row.lane).is_none();
        if let Some(rail) = fold.marker().filter(|_| !node_on_rail) {
            let style = off_thread(foreground(self.theme, Token::TextMuted), line, None);
            self.put(buf, area, lane_x(rail), y, RAIL, style);
        }
        // Several folded edges on one row draw one stub, in the first edge's
        // colour (ADR-0022). It takes the gap cell even when a kept edge runs
        // through it: the edge reads on either side.
        let Some(drawn) = stubs_of else { return };
        if let Some(first) = drawn.folded.first() {
            let glyph = match first.toward {
                Toward::Below => STUB_BELOW,
                Toward::Above => STUB_ABOVE,
            };
            // By role, a folded lane has no drawn column to take one
            // from: it reads as the quiet `Other`, like the rail it sits
            // behind.
            let style = if line.roles.is_some() {
                off_thread(
                    Style::new().fg(color(self.theme.token(Token::RoleOther))),
                    line,
                    None,
                )
            } else {
                off_thread(
                    Style::new().fg(color(self.theme.branch(first.identity))),
                    line,
                    self.faded(first.identity),
                )
            };
            let gap = lane_x(drawn.row.lane).saturating_add(1);
            self.put(buf, area, gap, y, glyph, style);
        }
    }
}
