//! The blocks the graph screen draws beside or among its rows, apart from
//! the rows themselves: the still-to-apply block under the banner and the
//! branch pane right of the body (ease-pass 09). Moved out of `render`
//! (move-only for the first) to keep it under its size row.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::Widget;

use super::super::GraphScreen;
use crate::paint::foreground;
use crate::theme::Token;

impl GraphScreen<'_> {
    /// The still-to-apply block (07c), boxed in its own rows under the
    /// banner; nothing when there is none or no room for it.
    pub(super) fn pending_block(&self, area: Rect, buf: &mut Buffer) {
        let Some(pending) = &self.view.pending else {
            return;
        };
        let lines = crate::still_to_apply::still_to_apply_lines(pending, area.width, area.height);
        for (offset, line) in lines.into_iter().enumerate() {
            let y = area
                .y
                .saturating_add(u16::try_from(offset).unwrap_or(u16::MAX));
            let styled = Line::styled(line, foreground(self.theme, Token::TextMuted));
            buf.set_line(area.x, y, &styled, area.width);
        }
    }

    /// The branch pane in its slot right of the body; nothing when the view
    /// has none or the screen is too narrow for one (`ScreenRows::pane`).
    pub(super) fn branch_pane(&self, area: Rect, buf: &mut Buffer) {
        let Some(view) = &self.view.pane else {
            return;
        };
        if area.is_empty() {
            return;
        }
        crate::branch_pane::BranchPane {
            view,
            theme: self.theme,
        }
        .render(area, buf);
    }
}
