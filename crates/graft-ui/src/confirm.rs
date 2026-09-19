//! The confirmation box: the one destructive-action prompt Graft shows
//! before a write it cannot undo the ordinary way (`d` discard,
//! m3-working-tree-undo-and-stash/04, PROMPT.md §3 "destructive actions").
//!
//! Names the consequence in plain words, shows the exact git command that is
//! about to run, and puts the safe choice (`Cancel`) under the cursor by
//! default — the user must move onto `Discard` on purpose.
//!
//! The widget carries no theme, like `context_menu`: the caller paints the
//! surface underneath and this draws text on top of it.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;

use crate::fit::fit;

/// Which button the cursor is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    /// The safe choice. Starts under the cursor.
    Cancel,
    /// Carries out the operation the box was opened for.
    Confirm,
}

/// Everything the confirm box draws: what is about to happen, in plain
/// words, and the exact command that will run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfirmView {
    /// One line: what is being confirmed (e.g. "Discard changes?").
    pub title: String,
    /// The consequence, in plain words — never just the command.
    pub message: Vec<String>,
    /// The exact git command this will run, shown so the user sees what
    /// "confirm" really does.
    pub command: String,
    /// Which button the cursor is on. `Cancel` on open.
    pub cursor: Choice,
}

impl ConfirmView {
    /// A confirm box, cursor on `Cancel`.
    #[must_use]
    pub fn new(title: impl Into<String>, message: Vec<String>, command: impl Into<String>) -> Self {
        ConfirmView {
            title: title.into(),
            message,
            command: command.into(),
            cursor: Choice::Cancel,
        }
    }

    /// Moves the cursor to the other button (there are only two).
    pub fn toggle(&mut self) {
        self.cursor = match self.cursor {
            Choice::Cancel => Choice::Confirm,
            Choice::Confirm => Choice::Cancel,
        };
    }
}

/// Rows the box needs to draw itself: title, each message line, a blank
/// line, the command, a blank line, the buttons.
#[must_use]
pub fn confirm_rows(view: &ConfirmView) -> u16 {
    u16::try_from(view.message.len().saturating_add(5)).unwrap_or(u16::MAX)
}

/// A box centred in `area`, sized to `view` (width capped so the command
/// line stays readable), never larger than `area` itself.
#[must_use]
pub fn confirm_area(area: Rect, view: &ConfirmView) -> Rect {
    let width = area.width.min(64);
    let height = confirm_rows(view).min(area.height);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width,
        height,
    }
}

/// The confirm box itself.
pub struct Confirm<'a> {
    /// What to draw.
    pub view: &'a ConfirmView,
}

impl Widget for &Confirm<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let width = usize::from(area.width);
        let mut y = area.y;
        let mut draw = |text: &str, style: Style, y: u16| {
            if y < area.y.saturating_add(area.height) {
                buf.set_stringn(area.x, y, fit(text, width), width, style);
            }
        };
        draw(
            &self.view.title,
            Style::new().add_modifier(Modifier::BOLD),
            y,
        );
        y = y.saturating_add(1);
        for line in &self.view.message {
            draw(line, Style::default(), y);
            y = y.saturating_add(1);
        }
        y = y.saturating_add(1);
        draw(&self.view.command, Style::default(), y);
        y = y.saturating_add(2);
        let cancel = if self.view.cursor == Choice::Cancel {
            "[ Cancel ]"
        } else {
            "  Cancel  "
        };
        let confirm = if self.view.cursor == Choice::Confirm {
            "[ Discard ]"
        } else {
            "  Discard  "
        };
        draw(&format!("{cancel}   {confirm}"), Style::default(), y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_with_cancel_under_the_cursor() {
        let view = ConfirmView::new("Discard?", vec!["one line".into()], "git checkout --");
        assert_eq!(view.cursor, Choice::Cancel);
    }

    #[test]
    fn toggle_swaps_between_the_two_choices() {
        let mut view = ConfirmView::new("Discard?", vec![], "git checkout --");
        view.toggle();
        assert_eq!(view.cursor, Choice::Confirm);
        view.toggle();
        assert_eq!(view.cursor, Choice::Cancel);
    }

    #[test]
    fn area_never_exceeds_the_frame() {
        let view = ConfirmView::new("Discard?", vec!["a".into(), "b".into()], "git x");
        let frame = Rect::new(0, 0, 10, 3);
        let area = confirm_area(frame, &view);
        assert!(area.width <= frame.width);
        assert!(area.height <= frame.height);
    }
}
