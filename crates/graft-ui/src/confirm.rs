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

    /// Draw `view` centred in a `width × height` frame through
    /// `confirm_area` and return its rows as plain strings — the character
    /// grid a capture would show.
    fn rows(view: &ConfirmView, width: u16, height: u16) -> Vec<String> {
        let frame = Rect::new(0, 0, width, height);
        let area = confirm_area(frame, view);
        let mut buf = Buffer::empty(frame);
        (&Confirm { view }).render(area, &mut buf);
        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| buf[(x, y)].symbol().to_owned())
                    .collect::<String>()
                    .trim_end()
                    .to_owned()
            })
            .collect()
    }

    /// A push that also sets upstream: the consequence is long enough to
    /// wrap onto two lines inside the box's own 64-cell cap (ticket scope
    /// item 5's "two-line consequence").
    fn push_view() -> ConfirmView {
        ConfirmView::new(
            "Push main → origin/main",
            vec![
                "2 commits · sets upstream · can't be undone from here — origin keeps \
                 what you send"
                    .to_owned(),
            ],
            "git push --progress -u origin refs/heads/main:refs/heads/main",
        )
    }

    /// An ordinary push, already tracking upstream: short enough to sit on
    /// one line inside the box's 64-cell cap (ticket scope item 5's
    /// "one line" case). The box width is capped at 64 regardless of the
    /// frame (`confirm_area`), so what fits on one line is a property of the
    /// message, not of 80 vs. 160 columns — the two widths in this test
    /// differ only in how the box is centred.
    fn push_view_one_line() -> ConfirmView {
        ConfirmView::new(
            "Push main → origin/main",
            vec!["can't be undone from here — origin keeps what you send".to_owned()],
            "git push --progress origin main",
        )
    }

    /// M4 group 03 review Major 6: nothing rendered the confirm box through
    /// its real widget before this — the ticket's own scope asked for "push
    /// confirm with a two-line consequence at 80 cols and one line at 160",
    /// and the only tests that existed drove the private `wrap_message`
    /// directly, never `Confirm::render`. This proves the box actually
    /// wraps on screen at 80 and fits on one line at 160, and feeds the
    /// `docs/shots/` capture (rule 6 — a visual ticket ends with one).
    #[test]
    fn push_confirm_wraps_at_80_and_fits_one_line_at_160() {
        let view = push_view();

        let narrow = rows(&view, 80, 24);
        let commits_line = narrow
            .iter()
            .position(|line| line.contains("2 commits"))
            .unwrap_or_else(|| panic!("no line has the start of the message: {narrow:#?}"));
        assert!(
            !narrow[commits_line].contains("keeps what you send"),
            "expected the message to wrap before 160 cols' worth: {narrow:#?}"
        );
        assert!(
            narrow
                .iter()
                .any(|line| line.contains("keeps what you send")),
            "expected a second line with the rest of the message: {narrow:#?}"
        );

        let wide = rows(&push_view_one_line(), 160, 48);
        let one_line: Vec<&String> = wide
            .iter()
            .filter(|line| {
                line.contains("undone from here") && line.contains("keeps what you send")
            })
            .collect();
        assert_eq!(
            one_line.len(),
            1,
            "expected the message on one line at 160 cols: {wide:#?}"
        );
    }

    /// Not an assertion — prints the push confirm box at 80×24 (wrapped)
    /// and 160×48 (one line), for a `docs/shots/` capture. `cargo test -p
    /// graft-ui confirm::tests::capture_for_docs_shots -- --ignored
    /// --nocapture`, by hand, when the rendering changes.
    #[test]
    #[ignore = "run by hand to regenerate docs/shots/m4-credential-prompt-and-hints.txt"]
    fn capture_for_docs_shots() {
        for (label, view, w, h) in [
            (
                "push confirm, two-line consequence, 80x24",
                push_view(),
                80,
                24,
            ),
            (
                "push confirm, one-line consequence, 160x48",
                push_view_one_line(),
                160,
                48,
            ),
        ] {
            println!("=== {label} ===");
            for line in rows(&view, w, h) {
                if !line.trim().is_empty() {
                    println!("{line:?}");
                }
            }
        }
    }

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

    /// M3 review 06 Minor 11: a long consequence must not grow the box into
    /// the frame's last row — that row is the app's own footer, drawn
    /// underneath, and a box reaching it used to hide it.
    ///
    /// At 120×40 the box (capped at [`MAX_MESSAGE_LINES`] message lines, so
    /// at most 9 rows) never comes close to the 40-row frame's footer, so
    /// that height could never catch a reservation that was missing (M4
    /// group 03 review Major 4 — the original version of this test could
    /// not fail). A frame whose height is at or below `confirm_rows` is the
    /// case the reservation actually has to act on.
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

        // Exactly as tall as the box would need without the reservation
        // (5 fixed rows + the 4-line-capped message = 9): without the
        // `footer_row` deduction in `confirm_area`, `area.bottom()` would
        // equal `frame.bottom()` here.
        let snug = Rect::new(0, 0, 120, 9);
        let area = confirm_area(snug, &view);
        assert_eq!(area.height, snug.height - 1, "{area:?}");
        assert!(
            area.bottom() < snug.bottom(),
            "box reaches the footer row: {area:?}"
        );

        // Shorter still: the box must give up rows rather than overrun.
        let cramped = Rect::new(0, 0, 120, 6);
        let area = confirm_area(cramped, &view);
        assert_eq!(area.height, cramped.height - 1, "{area:?}");
        assert!(
            area.bottom() < cramped.bottom(),
            "box reaches the footer row: {area:?}"
        );
    }
}
