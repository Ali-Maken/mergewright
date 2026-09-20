//! The confirmation box: the one destructive-action prompt Graft shows
//! before a write it cannot undo the ordinary way (`d` discard,
//! m3-working-tree-undo-and-stash/04, PROMPT.md §3 "destructive actions"),
//! and — from M5 — an N-choice box for a decision with more than one real
//! answer (`u` after a push, ADR-0038 §4).
//!
//! Names the consequence in plain words, shows the exact git command that is
//! about to run, and puts the safe choice under the cursor by default — the
//! user must move onto the acting choice on purpose. **The default never
//! moves with how dangerous the action is** (ADR-0038 §1): `Cancel` is
//! always the first choice, always where the cursor starts, in every box
//! this type draws, two choices or more.
//!
//! Draws its own rounded-border frame on a cleared area (`KNOWN_GAPS` row 484,
//! m4-07 Minor 2: the box used to draw no frame at all — text straight onto
//! whatever the caller left underneath it, at every size the M4 push box
//! tests used).

use graft_term::{SanitizedText, sanitize_line};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;

use crate::frame_box::{border_row, cells, content_row, wrap_line};
use crate::paint::foreground;
use crate::theme::{Theme, Token};

/// The most lines a wrapped consequence draws: past this many, the rest is
/// dropped and the last line ends in `…` (m4-remotes ticket 03, keeping the
/// box from growing past a small terminal's height).
const MAX_MESSAGE_LINES: usize = 4;

/// `message`'s lines, each wrapped on spaces to at most `width` cells, and
/// the whole result capped at [`MAX_MESSAGE_LINES`] — the rest replaced by a
/// `…` on the last line kept.
fn wrap_message(message: &[SanitizedText], width: usize) -> Vec<String> {
    let mut lines: Vec<String> = message
        .iter()
        .flat_map(|source| wrap_line(source.as_str(), width))
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

/// Everything the confirm box draws: what is about to happen, in plain
/// words, and the exact command that will run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfirmView {
    /// One line: what is being confirmed (e.g. "Discard changes?").
    pub title: SanitizedText,
    /// The consequence, in plain words — never just the command.
    pub message: Vec<SanitizedText>,
    /// The exact git command this will run, shown so the user sees what
    /// the acting choice really does.
    pub command: SanitizedText,
    /// The choices, left to right; index 0 is always the safe one
    /// (ADR-0038 §1 — never reorder this so a dangerous choice ends up
    /// first).
    pub choices: Vec<SanitizedText>,
    /// Which choice the cursor is on. `0` (`Cancel`) on open.
    pub cursor: usize,
}

impl ConfirmView {
    /// A confirm box, cursor on `Cancel`, choices `Cancel`/`Confirm`.
    #[must_use]
    pub fn new(title: SanitizedText, message: Vec<SanitizedText>, command: SanitizedText) -> Self {
        ConfirmView {
            title,
            message,
            command,
            choices: vec![sanitize_line("Cancel"), sanitize_line("Confirm")],
            cursor: 0,
        }
    }

    /// The same box with its own choices — `labels[0]` is always the safe
    /// one and is where the cursor starts (ADR-0038 §1): the undo-after-push
    /// box asks `Cancel` / `Undo locally` / `Re-push old tip`, where two
    /// choices would not say what the third does.
    #[must_use]
    pub fn with_choices(mut self, labels: Vec<SanitizedText>) -> Self {
        self.choices = labels;
        self.cursor = 0;
        self
    }

    /// A shim over the two-choice case: `[ safe ]  [ confirm ]`, where
    /// "Discard" (the old default) would name the opposite of what the
    /// button does for some callers (m3-working-tree-undo-and-stash/06's
    /// `[ Stop ]` / `[ Undo ]`).
    #[must_use]
    pub fn with_labels(self, safe: &str, confirm: &str) -> Self {
        self.with_choices(vec![sanitize_line(safe), sanitize_line(confirm)])
    }

    /// Moves the cursor one choice left, clamped at the first (no wrap: the
    /// safe choice is never one key-repeat away from "one past the end,
    /// wrapped onto the dangerous one").
    pub fn left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    /// Moves the cursor one choice right, clamped at the last.
    pub fn right(&mut self) {
        let last = self.choices.len().saturating_sub(1);
        self.cursor = self.cursor.saturating_add(1).min(last);
    }

    /// Cycles the cursor to the next choice, wrapping past the last back to
    /// the first — the two-choice case's `Tab`.
    pub fn toggle(&mut self) {
        if self.choices.is_empty() {
            return;
        }
        self.cursor = (self.cursor + 1) % self.choices.len();
    }

    /// The choice the cursor is on.
    #[must_use]
    pub fn selected(&self) -> usize {
        self.cursor
    }
}

/// Rows the box needs to draw itself: the top border (title), the message
/// wrapped to `width` (capped at [`MAX_MESSAGE_LINES`]), a blank row, the
/// command, a blank row, the choices, the bottom border.
#[must_use]
pub fn confirm_rows(view: &ConfirmView, width: u16) -> u16 {
    let wrapped = wrap_message(&view.message, usize::from(width).saturating_sub(3));
    u16::try_from(wrapped.len().saturating_add(6)).unwrap_or(u16::MAX)
}

/// A box centred in `area`, sized to `view` (width capped so the command
/// line stays readable), never larger than `area` itself.
///
/// Reserves the frame's own last row when there is more than one: the
/// footer's key hints live there, and a box that reaches it hides them
/// instead of drawing over blank space (M3 review 06 Minor 11).
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

/// One choice: bracketed when the cursor is on it, spaced the same width
/// when it is not, so the row does not shift as the cursor moves.
fn button(label: &str, selected: bool) -> String {
    if selected {
        format!("[ {label} ]")
    } else {
        format!("  {label}  ")
    }
}

/// The confirm box itself: a rounded-border frame on a cleared area, so it
/// never shows the cells that were underneath it (row 484).
pub struct Confirm<'a> {
    /// What to draw.
    pub view: &'a ConfirmView,
    /// Colours.
    pub theme: &'a Theme,
}

impl Widget for &Confirm<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 2 || area.height < 2 {
            return;
        }
        let width = usize::from(area.width);
        let view = self.view;
        let text_style = foreground(self.theme, Token::Text);
        let muted = foreground(self.theme, Token::TextMuted);
        let bold = text_style.add_modifier(Modifier::BOLD);

        for y in area.top()..area.bottom() {
            buf.set_stringn(area.x, y, " ".repeat(width), width, Style::default());
        }

        let mut y = area.y;
        buf.set_string(
            area.x,
            y,
            border_row('╭', '╮', view.title.as_str(), width),
            bold,
        );
        y = y.saturating_add(1);

        let draw_content = |text: &str, style: Style, y: u16, buf: &mut Buffer| {
            if y < area.bottom() {
                buf.set_string(area.x, y, content_row(text, width), style);
            }
        };

        for line in wrap_message(&view.message, width.saturating_sub(3)) {
            draw_content(&line, text_style, y, buf);
            y = y.saturating_add(1);
        }
        draw_content("", text_style, y, buf);
        y = y.saturating_add(1);
        draw_content(view.command.as_str(), muted, y, buf);
        y = y.saturating_add(1);
        draw_content("", text_style, y, buf);
        y = y.saturating_add(1);

        let buttons: String = view
            .choices
            .iter()
            .enumerate()
            .map(|(index, label)| button(label.as_str(), index == view.cursor))
            .collect::<Vec<_>>()
            .join("  ");
        draw_content(&buttons, text_style, y, buf);
        y = y.saturating_add(1);

        if y < area.bottom() {
            buf.set_string(area.x, y, border_row('╰', '╯', "", width), text_style);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::terminal as test_capabilities;
    use crate::theme::ThemeKind;

    fn theme() -> Theme {
        Theme::new(
            ThemeKind::Dark,
            &test_capabilities(graft_core::capabilities::ColorDepth::TrueColor, false),
        )
    }

    /// Draw `view` centred in a `width × height` frame through
    /// `confirm_area` and return its rows as plain strings — the character
    /// grid a capture would show.
    fn rows(view: &ConfirmView, width: u16, height: u16) -> Vec<String> {
        let theme = theme();
        let frame = Rect::new(0, 0, width, height);
        let area = confirm_area(frame, view);
        let mut buf = Buffer::empty(frame);
        (&Confirm {
            view,
            theme: &theme,
        })
            .render(area, &mut buf);
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

    fn text(message: &[&str]) -> Vec<SanitizedText> {
        message.iter().map(|line| sanitize_line(line)).collect()
    }

    /// A push that also sets upstream: the consequence is long enough to
    /// wrap onto two lines inside the box's own 64-cell cap (ticket scope
    /// item 5's "two-line consequence").
    fn push_view() -> ConfirmView {
        ConfirmView::new(
            sanitize_line("Push main → origin/main"),
            text(&[
                "2 commits · sets upstream · can't be undone from here — origin keeps \
                 what you send",
            ]),
            sanitize_line("git push --progress -u origin refs/heads/main:refs/heads/main"),
        )
    }

    /// An ordinary push, already tracking upstream: short enough to sit on
    /// one line inside the box's 64-cell cap (ticket scope item 5's
    /// "one line" case).
    fn push_view_one_line() -> ConfirmView {
        ConfirmView::new(
            sanitize_line("Push main → origin/main"),
            text(&["can't be undone from here — origin keeps what you send"]),
            sanitize_line("git push --progress origin main"),
        )
    }

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

    #[test]
    fn opens_with_cancel_under_the_cursor() {
        let view = ConfirmView::new(
            sanitize_line("Discard?"),
            text(&["one line"]),
            sanitize_line("git checkout --"),
        );
        assert_eq!(view.selected(), 0);
        assert_eq!(view.choices[0].as_str(), "Cancel");
    }

    #[test]
    fn toggle_swaps_between_the_two_choices() {
        let mut view = ConfirmView::new(
            sanitize_line("Discard?"),
            vec![],
            sanitize_line("git checkout --"),
        );
        view.toggle();
        assert_eq!(view.selected(), 1);
        view.toggle();
        assert_eq!(view.selected(), 0);
    }

    #[test]
    fn left_and_right_clamp_without_wrapping() {
        let mut view = ConfirmView::new(sanitize_line("t"), vec![], sanitize_line("c"))
            .with_choices(vec![
                sanitize_line("Cancel"),
                sanitize_line("Undo locally"),
                sanitize_line("Re-push old tip"),
            ]);
        view.left();
        assert_eq!(view.selected(), 0, "left of the first choice stays put");
        view.right();
        view.right();
        view.right();
        assert_eq!(view.selected(), 2, "right of the last choice stays put");
    }

    #[test]
    fn area_never_exceeds_the_frame() {
        let view = ConfirmView::new(
            sanitize_line("Discard?"),
            text(&["a", "b"]),
            sanitize_line("git x"),
        );
        let frame = Rect::new(0, 0, 10, 3);
        let area = confirm_area(frame, &view);
        assert!(area.width <= frame.width);
        assert!(area.height <= frame.height);
    }

    /// A consequence longer than the box wraps on spaces instead of being
    /// cut mid-sentence by `fit`'s own truncation.
    #[test]
    fn long_message_wraps() {
        let message = text(&["can't be undone from here — origin keeps what you send"]);
        let wrapped = wrap_message(&message, 20);
        assert!(wrapped.len() > 1, "{wrapped:?}");
        for line in &wrapped {
            assert!(cells(line) <= 20, "{line:?} wider than 20");
        }
        let rejoined: Vec<&str> = wrapped.iter().flat_map(|l| l.split(' ')).collect();
        let original: Vec<&str> = message[0].split(' ').collect();
        assert_eq!(rejoined, original);
    }

    /// Past [`MAX_MESSAGE_LINES`] the rest is dropped, marked with `…` on
    /// the last line kept — never grows the box past a small terminal.
    #[test]
    fn message_caps_at_four_lines_with_ellipsis() {
        let long = "word ".repeat(80);
        let wrapped = wrap_message(&[sanitize_line(&long)], 10);
        assert_eq!(wrapped.len(), MAX_MESSAGE_LINES);
        assert!(wrapped.last().unwrap().ends_with('…'), "{wrapped:?}");
    }

    /// Row 484 (m4-07 Minor 2): the box must actually draw a frame — a
    /// rounded top-left corner and the title on its first row — at every
    /// size the M4 push box tests used, not just at one lucky size.
    #[test]
    fn confirm_draws_its_frame() {
        for (width, height) in [(80, 24), (160, 48)] {
            let view = push_view();
            let drawn = rows(&view, width, height);
            let top = drawn
                .iter()
                .find(|line| !line.trim().is_empty())
                .unwrap_or_else(|| panic!("nothing drawn at {width}x{height}"));
            assert!(
                top.trim_start().starts_with('╭'),
                "expected a rounded top-left corner at {width}x{height}: {top:?}"
            );
            assert!(
                top.contains("Push main"),
                "expected the title in the top row at {width}x{height}: {top:?}"
            );
        }
    }

    /// M3 review 06 Minor 11: a long consequence must not grow the box into
    /// the frame's last row — that row is the app's own footer.
    #[test]
    fn buttons_stay_above_footer_at_120() {
        let long = text(&[
            "2 commits · sets upstream · can't be undone from here — origin keeps what you send \
             · this push rewrites the remote branch and every teammate tracking it will need to \
             fetch before they can push again",
        ]);
        let view = ConfirmView::new(
            sanitize_line("Push main → origin/main"),
            long,
            sanitize_line("git push --progress -u origin refs/heads/main:refs/heads/main"),
        );

        let snug = Rect::new(0, 0, 120, 10);
        let area = confirm_area(snug, &view);
        assert_eq!(area.height, snug.height - 1, "{area:?}");
        assert!(
            area.bottom() < snug.bottom(),
            "box reaches the footer row: {area:?}"
        );

        let cramped = Rect::new(0, 0, 120, 6);
        let area = confirm_area(cramped, &view);
        assert_eq!(area.height, cramped.height - 1, "{area:?}");
        assert!(
            area.bottom() < cramped.bottom(),
            "box reaches the footer row: {area:?}"
        );
    }

    /// A `[ Cancel ] [ Undo locally ] [ Re-push old tip ]` box: the `u`
    /// after a push case (ADR-0038 §4), and the ticket's N-choice
    /// acceptance — cursor still opens on `Cancel`, index 0.
    fn three_choice_view() -> ConfirmView {
        ConfirmView::new(
            sanitize_line("origin/main moved since you pushed"),
            text(&["u would normally undo your last push, but the remote tip changed."]),
            sanitize_line("git push --force-with-lease origin refs/heads/main"),
        )
        .with_choices(vec![
            sanitize_line("Cancel"),
            sanitize_line("Undo locally"),
            sanitize_line("Re-push old tip"),
        ])
    }

    /// Not an assertion — dumps the 2-choice and 3-choice boxes at 80×24
    /// and 160×48, cursor on `Cancel` in every one, so ADR-0038 §1 (the
    /// default never moves with how dangerous the action is) can be judged
    /// by eye. Run by hand to regenerate: `cargo test -p graft-ui
    /// confirm::tests::capture_for_docs_shots -- --ignored --nocapture`.
    #[test]
    #[ignore = "writes docs/shots/m5-ui-confirm.txt; run by hand to regenerate"]
    fn capture_for_docs_shots() {
        let mut out = String::new();
        out.push_str(
            "# docs/shots/m5-ui-confirm.txt — N-choice confirm, frame fix\n\
             #\n\
             # What right looks like: a rounded-border frame around every box\n\
             # (row 484 fix — it used to draw no frame at all), the title in\n\
             # the top border, and the cursor's reversed button always on\n\
             # `Cancel` — the leftmost choice — whatever the box's own danger\n\
             # level (ADR-0038 §1). Colour (which button reverses) is\n\
             # unverified by this capture: judge the reversed-button glyphs.\n\
             \n",
        );
        for (label, view, w, h) in [
            ("2-choice push, 80x24", push_view(), 80, 24),
            ("2-choice push, 160x48", push_view(), 160, 48),
            (
                "3-choice undo-after-push, 80x24",
                three_choice_view(),
                80,
                24,
            ),
            (
                "3-choice undo-after-push, 160x48",
                three_choice_view(),
                160,
                48,
            ),
        ] {
            out.push_str(&format!("=== {label} ===\n"));
            for line in rows(&view, w, h) {
                if !line.trim().is_empty() {
                    out.push_str(&line);
                    out.push('\n');
                }
            }
            out.push('\n');
        }
        std::fs::write(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../docs/shots/m5-ui-confirm.txt"
            ),
            out,
        )
        .expect("docs/shots is writable");
    }
}
