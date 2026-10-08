//! The menu's picture: an optional bold title, then one row per item —
//! dimmed when it refuses, reversed under the cursor.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;

use super::{ContextMenu, row_text};
use crate::fit::fit;

impl Widget for &ContextMenu {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let width = usize::from(area.width);
        let mut offset = 0_u16;
        if let Some(title) = &self.title {
            buf.set_stringn(
                area.x,
                area.y,
                fit(&format!(" {}", title.as_str()), width),
                width,
                Style::new().add_modifier(Modifier::BOLD),
            );
            offset = 1;
        }
        for item in &self.items {
            if offset >= area.height {
                break;
            }
            let index = usize::from(offset) - usize::from(self.title.is_some());
            let mut style = if item.disabled.is_some() {
                // Dim: the row is there to be read, not chosen. `Enter` on it
                // says why in words rather than doing nothing.
                Style::new().add_modifier(Modifier::DIM)
            } else {
                Style::default()
            };
            if index == self.cursor {
                // The accent cursor row: reversed so it reads over whatever
                // background the caller painted underneath. A disabled row
                // still takes it — the cursor must be visible wherever it is.
                style = style.add_modifier(Modifier::REVERSED);
            }
            let text = fit(&format!(" {}", row_text(item)), width);
            let y = area.y.saturating_add(offset);
            buf.set_stringn(area.x, y, text, width, style);
            offset = offset.saturating_add(1);
        }
    }
}
