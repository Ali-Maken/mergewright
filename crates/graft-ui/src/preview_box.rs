//! Ghost Preview's box: docked to the bottom of the graph, above the
//! footer. Shows what a rebase/merge/reset/revert/time-machine jump would
//! do before the user confirms it — the box names the refs that move, the
//! working tree's fate and the undo it leaves, and shows the exact git
//! command that will run (design note "New UI: Ghost Preview";
//! DESIGN.md §1.7; ADR-0038 §5).
//!
//! Built on [`crate::frame_box`], the same rounded-border frame
//! [`crate::confirm::Confirm`] draws (the "row 484" fix): a title in the top
//! border, a content row per line, the choices, and the box's own key hints
//! in the bottom border — while it is open the main footer draws nothing
//! (`docs/design-notes/m5-history-and-ghost-preview.md`, "New UI" table).

use graft_term::SanitizedText;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;

use crate::frame_box::{border_row, content_row};
use crate::paint::foreground;
use crate::theme::{Theme, Token};

/// Below this width or this height the box does not fit: "a picture that
/// cannot be seen is never confirmed" (design note). `⏎` does nothing at
/// this size — the caller must not act on a plan the user could not read.
pub const PREVIEW_MIN: (u16, u16) = (40, 12);

/// The box's own footer, shown in its bottom border while it is open.
const FOOTER: &str = "←→ choose  ⏎ act  esc cancel";

/// The message drawn instead of the box when the frame is smaller than
/// [`PREVIEW_MIN`].
const TOO_SMALL: &str = "preview needs 40x12 · esc closes";

/// Everything the Ghost Preview box draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewBox {
    /// The top border's title: `Preview: rebase feature/auth onto main`.
    pub title: SanitizedText,
    /// What the plan does, one fact per line: refs that move, the working
    /// tree's fate, the undo it leaves, a forecast conflict.
    pub lines: Vec<SanitizedText>,
    /// The exact git command(s) that will run.
    pub commands: Vec<SanitizedText>,
    /// The choices, left to right; index 0 is always `Cancel`
    /// (ADR-0038 §1, same rule as [`crate::confirm::ConfirmView`]).
    pub choices: Vec<SanitizedText>,
    /// Which choice the cursor is on.
    pub cursor: usize,
    /// The forecast has not landed yet: draws `predicting…` in place of
    /// `lines`, for up to about a second while the plan is worked out. The
    /// ghost rows themselves appear only once this turns false.
    pub planning: bool,
}

impl PreviewBox {
    /// The choice the cursor is on.
    #[must_use]
    pub fn selected(&self) -> usize {
        self.cursor
    }

    /// Moves the cursor one choice left, clamped at the first (no wrap).
    pub fn left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    /// Moves the cursor one choice right, clamped at the last (no wrap).
    pub fn right(&mut self) {
        let last = self.choices.len().saturating_sub(1);
        self.cursor = self.cursor.saturating_add(1).min(last);
    }
}

/// Rows the box takes at `width` — the caller's row budget for docking it
/// above the footer, without the footer row it must never cover. Constant in
/// `width` today (no line wraps); taken anyway so a future wrap never
/// changes this function's callers.
#[must_use]
pub fn preview_rows(view: &PreviewBox, width: u16) -> u16 {
    let _ = width;
    let body = if view.planning { 1 } else { view.lines.len() };
    let rows = 1 // top border + title
        + body
        + view.commands.len()
        + 1 // choices row
        + 1; // bottom border + footer
    u16::try_from(rows).unwrap_or(u16::MAX)
}

/// [`PreviewBox`] drawn into whatever `Rect` the caller docks it to —
/// bottom of the graph area, never over the footer row.
pub struct PreviewBoxWidget<'a> {
    /// What to draw.
    pub view: &'a PreviewBox,
    /// Colours.
    pub theme: &'a Theme,
}

/// One choice: bracketed when the cursor is on it, spaced the same width
/// otherwise, so the row does not shift as the cursor moves.
fn button(label: &str, selected: bool) -> String {
    if selected {
        format!("[ {label} ]")
    } else {
        format!("  {label}  ")
    }
}

impl PreviewBoxWidget<'_> {
    /// The lines, commands and choice buttons between the title and footer
    /// borders — everything `render` draws once it knows the area is big
    /// enough and cleared.
    fn draw_body(&self, area: Rect, width: usize, mut y: u16, buf: &mut Buffer) {
        let view = self.view;
        let text_style = foreground(self.theme, Token::Text);
        let muted = foreground(self.theme, Token::TextMuted);
        let draw = |text: &str, style: Style, y: u16, buf: &mut Buffer| {
            if y < area.bottom().saturating_sub(1) {
                buf.set_string(area.x, y, content_row(text, width), style);
            }
        };

        if view.planning {
            draw("predicting…", muted, y, buf);
            y = y.saturating_add(1);
        } else {
            for line in &view.lines {
                draw(line.as_str(), text_style, y, buf);
                y = y.saturating_add(1);
            }
        }
        for command in &view.commands {
            draw(command.as_str(), muted, y, buf);
            y = y.saturating_add(1);
        }

        let buttons: String = view
            .choices
            .iter()
            .enumerate()
            .map(|(index, label)| button(label.as_str(), index == view.cursor))
            .collect::<Vec<_>>()
            .join("  ");
        draw(&buttons, text_style, y, buf);

        buf.set_string(
            area.x,
            area.bottom() - 1,
            border_row('╰', '╯', FOOTER, width),
            muted,
        );
    }
}

impl Widget for &PreviewBoxWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let width = usize::from(area.width);
        let view = self.view;
        let text_style = foreground(self.theme, Token::Text);

        if area.width < PREVIEW_MIN.0 || area.height < PREVIEW_MIN.1 {
            let message = crate::fit::fit(TOO_SMALL, width);
            buf.set_stringn(
                area.x,
                area.y,
                message,
                width,
                foreground(self.theme, Token::Warning),
            );
            return;
        }

        for y in area.top()..area.bottom() {
            buf.set_stringn(area.x, y, " ".repeat(width), width, Style::default());
        }

        let mut y = area.y;
        buf.set_string(
            area.x,
            y,
            border_row('╭', '╮', view.title.as_str(), width),
            text_style,
        );
        y = y.saturating_add(1);

        self.draw_body(area, width, y, buf);
    }
}

#[cfg(test)]
mod tests;
