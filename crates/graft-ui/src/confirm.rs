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
use unicode_width::UnicodeWidthChar;

use crate::fit::fit;

/// The most lines a wrapped consequence draws, buttons included in neither:
/// past this many, the rest is dropped and the last line ends in `…`
/// (m4-remotes ticket 03, keeping the box from growing past a small
/// terminal's height).
const MAX_MESSAGE_LINES: usize = 4;

/// Cells `text` occupies, counting a wide character as two — the same
/// measure `fit` clamps to, so a wrapped line never straddles the width it
/// was wrapped for.
fn cells(text: &str) -> usize {
    text.chars().filter_map(UnicodeWidthChar::width).sum()
}

/// One source line, wrapped on spaces to at most `width` cells. A word
/// longer than `width` on its own is placed on its own line and left to
/// `fit`'s own clamp at draw time rather than split mid-word.
fn wrap_line(source: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in source.split(' ') {
        let candidate = if current.is_empty() {
            word.to_owned()
        } else {
            format!("{current} {word}")
        };
        if !current.is_empty() && cells(&candidate) > width {
            lines.push(std::mem::replace(&mut current, word.to_owned()));
        } else {
            current = candidate;
        }
    }
    lines.push(current);
    lines
}

/// `message`'s lines, each wrapped on spaces to at most `width` cells, and
/// the whole result capped at [`MAX_MESSAGE_LINES`] — the rest replaced by a
/// `…` on the last line kept.
fn wrap_message(message: &[String], width: usize) -> Vec<String> {
    let mut lines: Vec<String> = message
        .iter()
        .flat_map(|source| wrap_line(source, width))
        .collect();
    if lines.len() > MAX_MESSAGE_LINES {
        lines.truncate(MAX_MESSAGE_LINES);
        if let Some(last) = lines.last_mut() {
            while cells(last) + 1 > width && !last.is_empty() {
                last.pop();
            }
            last.push('…');
        }
    }
    lines
}

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
    /// What the safe button says, without its brackets.
    pub safe_label: String,
    /// What the acting button says, without its brackets.
    pub confirm_label: String,
}

impl ConfirmView {
    /// A confirm box, cursor on `Cancel`, buttons `Cancel`/`Discard`.
    #[must_use]
    pub fn new(title: impl Into<String>, message: Vec<String>, command: impl Into<String>) -> Self {
        ConfirmView {
            title: title.into(),
            message,
            command: command.into(),
            cursor: Choice::Cancel,
            safe_label: "Cancel".to_owned(),
            confirm_label: "Discard".to_owned(),
        }
    }

    /// The same box with its own button words: the undo drift guard asks
    /// `[ Stop ]` / `[ Undo ]`, where "Discard" would name the opposite of
    /// what the button does (m3-working-tree-undo-and-stash/06).
    #[must_use]
    pub fn with_labels(mut self, safe: impl Into<String>, confirm: impl Into<String>) -> Self {
        self.safe_label = safe.into();
        self.confirm_label = confirm.into();
        self
    }

    /// Moves the cursor to the other button (there are only two).
    pub fn toggle(&mut self) {
        self.cursor = match self.cursor {
            Choice::Cancel => Choice::Confirm,
            Choice::Confirm => Choice::Cancel,
        };
    }
}

/// Rows the box needs to draw itself: title, the message wrapped to `width`
/// (capped at [`MAX_MESSAGE_LINES`]), a blank line, the command, a blank
/// line, the buttons.
///
/// Takes `width` where the previous signature took none: a message that may
/// now wrap cannot say how many rows it needs without knowing how wide it is
/// drawn (m4-remotes ticket 03).
#[must_use]
pub fn confirm_rows(view: &ConfirmView, width: u16) -> u16 {
    let wrapped = wrap_message(&view.message, usize::from(width));
    u16::try_from(wrapped.len().saturating_add(5)).unwrap_or(u16::MAX)
}

/// A box centred in `area`, sized to `view` (width capped so the command
/// line stays readable), never larger than `area` itself.
///
/// Reserves the frame's own last row when there is more than one: the
/// footer's key hints live there, and a box that reaches it hides them
/// instead of drawing over blank space (M3 review 06 Minor 11, the confirm
/// box overdrawing the footer at 120 cols — not repeated here).
#[must_use]
pub fn confirm_area(area: Rect, view: &ConfirmView) -> Rect {
    let width = area.width.min(64);
    let footer_row = u16::from(area.height > 1);
    let usable_height = area.height.saturating_sub(footer_row);
    let height = confirm_rows(view, width).min(usable_height);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + usable_height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width,
        height,
    }
}

/// One button: bracketed when the cursor is on it, spaced the same width
/// when it is not, so the row does not shift as the cursor moves.
fn button(label: &str, selected: bool) -> String {
    if selected {
        format!("[ {label} ]")
    } else {
        format!("  {label}  ")
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
        for line in wrap_message(&self.view.message, width) {
            draw(&line, Style::default(), y);
            y = y.saturating_add(1);
        }
        y = y.saturating_add(1);
        draw(&self.view.command, Style::default(), y);
        y = y.saturating_add(2);
        let cancel = button(&self.view.safe_label, self.view.cursor == Choice::Cancel);
        let confirm = button(
            &self.view.confirm_label,
            self.view.cursor == Choice::Confirm,
        );
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

    /// A consequence longer than the box wraps on spaces instead of being
    /// cut mid-sentence by `fit`'s own truncation.
    #[test]
    fn long_message_wraps() {
        let message = vec!["can't be undone from here — origin keeps what you send".to_owned()];
        let wrapped = wrap_message(&message, 20);
        assert!(wrapped.len() > 1, "{wrapped:?}");
        for line in &wrapped {
            assert!(cells(line) <= 20, "{line:?} wider than 20");
        }
        // Every word survives, in order, none split mid-word.
        let rejoined: Vec<&str> = wrapped.iter().flat_map(|l| l.split(' ')).collect();
        let original: Vec<&str> = message[0].split(' ').collect();
        assert_eq!(rejoined, original);
    }

    /// Past [`MAX_MESSAGE_LINES`] the rest is dropped, marked with `…` on
    /// the last line kept — never grows the box past a small terminal.
    #[test]
    fn message_caps_at_four_lines_with_ellipsis() {
        let long = "word ".repeat(80);
        let wrapped = wrap_message(&[long], 10);
        assert_eq!(wrapped.len(), MAX_MESSAGE_LINES);
        assert!(wrapped.last().unwrap().ends_with('…'), "{wrapped:?}");
    }

    /// M3 review 06 Minor 11: at 120 columns a long consequence must not
    /// grow the box into the frame's last row — that row is the app's own
    /// footer, drawn underneath, and a box reaching it used to hide it.
    #[test]
    fn buttons_stay_above_footer_at_120() {
        let long = vec![
            "2 commits · sets upstream · can't be undone from here — origin keeps what you send \
             · this push rewrites the remote branch and every teammate tracking it will need to \
             fetch before they can push again"
                .to_owned(),
        ];
        let view = ConfirmView::new(
            "Push main → origin/main",
            long,
            "git push --progress -u origin refs/heads/main:refs/heads/main",
        );
        let frame = Rect::new(0, 0, 120, 40);
        let area = confirm_area(frame, &view);
        assert!(
            area.bottom() < frame.bottom(),
            "box reaches the footer row: {area:?}"
        );
    }
}
