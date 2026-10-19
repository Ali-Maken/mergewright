//! The cursor row's second line (ease-pass/07b, mock G9): who, when, where
//! the commit was made and its short hash, dim, from the message column to
//! the right edge, on the cursor row's fill. The graph column draws the
//! lanes continuing through it (`GraphColumn::second_line_after`).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;

use super::{GraphEntry, GraphScreen, draw_cell};
use crate::graph_screen::row_columns::Columns;

impl GraphScreen<'_> {
    /// Draw `entry`'s second line at `y`. Nothing when it has none.
    pub(super) fn draw_second_line(
        &self,
        entry: &GraphEntry<'_>,
        y: u16,
        area: Rect,
        columns: &Columns,
        buf: &mut Buffer,
    ) {
        let Some(second) = &entry.second else {
            return;
        };
        // The cursor row's fill, and its muted ink without the bold.
        let (_, muted) = self.row_emphasis(entry, y, area, columns, buf);
        let words = second.words.as_str();
        let text = if second.sha7.as_str().is_empty() {
            words.to_owned()
        } else {
            format!("{words} \u{b7} {}", second.sha7.as_str())
        };
        let width = area.right().saturating_sub(columns.subject_x);
        draw_cell(
            buf,
            columns.subject_x,
            y,
            width,
            &text,
            muted.remove_modifier(Modifier::BOLD),
        );
    }
}
