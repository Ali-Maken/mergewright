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

use crate::frame_box::{border_row, buttons_row, cells, content_row, reverse_button, wrap_line};
use crate::paint::foreground;
use crate::theme::{Theme, Token};

/// The most lines a wrapped consequence draws: past this many, the rest is
/// dropped and the last line ends in `…` (m4-remotes ticket 03, keeping the
/// box from growing past a small terminal's height).
const MAX_MESSAGE_LINES: usize = 4;

/// `message`'s lines, each wrapped on spaces to at most `width` cells, and
/// the whole result capped at [`MAX_MESSAGE_LINES`] — the rest replaced by a
/// `…` on the last line kept.
fn wrap_message(message: &[SanitizedText], width: usize, max_lines: usize) -> Vec<String> {
    let mut lines: Vec<String> = message
        .iter()
        .flat_map(|source| wrap_line(source.as_str(), width))
        .collect();
    if lines.len() > max_lines {
        lines.truncate(max_lines);
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
    /// Wrapped message rows this box may draw ([`ConfirmView::with_max_lines`]).
    max_lines: usize,
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
            max_lines: MAX_MESSAGE_LINES,
        }
    }

    /// Wrapped message rows this box may draw; `new` sets
    /// [`MAX_MESSAGE_LINES`] (4). For the one box whose message is the
    /// preview itself — the force-push lease box's list of what leaves the
    /// remote (m7-power-tools/16b). A frame too short for the rows still
    /// cuts message rows, never the command or the buttons.
    #[must_use]
    pub fn with_max_lines(mut self, rows: usize) -> Self {
        self.max_lines = rows;
        self
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
/// wrapped to `width` (capped at the view's own line cap, 4 unless raised), a blank row, the
/// command, a blank row, the choices, the bottom border.
///
/// Wrapped to `width - 4`: two borders and a cell of padding on each side
/// (`frame_box::content_row`'s own reservation) — not `width - 3`, which
/// left the longest line exactly filling the border's own gutter cell.
#[must_use]
pub fn confirm_rows(view: &ConfirmView, width: u16) -> u16 {
    let wrapped = wrap_message(
        &view.message,
        usize::from(width).saturating_sub(4),
        view.max_lines,
    );
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
    let x = area.x + area.width.saturating_sub(width) / 2;
    confirm_area_at(area, view, x, width)
}

/// The same box at a horizontal span the caller chose rather than the
/// centred one [`confirm_area`] computes: same vertical rule, height taken
/// again at `width` because a wider box wraps its message into fewer lines.
///
/// This is what the layout site calls after snapping the box onto whole
/// graph columns (ADR-0040). The widget learns a width, which is its own
/// business; it never learns what a column is.
#[must_use]
pub fn confirm_area_at(area: Rect, view: &ConfirmView, x: u16, width: u16) -> Rect {
    let footer_row = u16::from(area.height > 1);
    let usable_height = area.height.saturating_sub(footer_row);
    let height = confirm_rows(view, width).min(usable_height);
    let y = area.y + usable_height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width,
        height,
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

        // Six rows are the box's own: borders, blanks, command, buttons. A
        // frame too short for every message row cuts the message, never
        // those (m7-power-tools/16b).
        let fits = usize::from(area.height).saturating_sub(6).max(1);
        let rows = view.max_lines.min(fits);
        for line in wrap_message(&view.message, width.saturating_sub(4), rows) {
            draw_content(&line, text_style, y, buf);
            y = y.saturating_add(1);
        }
        draw_content("", text_style, y, buf);
        y = y.saturating_add(1);
        draw_content(view.command.as_str(), muted, y, buf);
        y = y.saturating_add(1);
        draw_content("", text_style, y, buf);
        y = y.saturating_add(1);

        let labels: Vec<&str> = view.choices.iter().map(SanitizedText::as_str).collect();
        let (buttons, cursor) = buttons_row(&labels, view.cursor);
        draw_content(&buttons, text_style, y, buf);
        if y < area.bottom() {
            reverse_button(buf, area, y, cursor, text_style);
        }
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
        let wrapped = wrap_message(&message, 20, MAX_MESSAGE_LINES);
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
        let wrapped = wrap_message(&[sanitize_line(&long)], 10, MAX_MESSAGE_LINES);
        assert_eq!(wrapped.len(), MAX_MESSAGE_LINES);
        assert!(wrapped.last().unwrap().ends_with('…'), "{wrapped:?}");
    }

    /// The box's required width is two borders plus two cells of padding
    /// plus the longest content line — `confirm_rows` now wraps
    /// `view.message` to `width - 4`, not `width - 3`, and `Confirm`'s own
    /// `render` wraps to the same width, so the longest wrapped line is
    /// never more than `width - 4` cells and `frame_box::content_row`
    /// (`width - 2` inner, one cell of padding reserved each side) always
    /// has a spare cell on the right. Reproduces the exact fault named in
    /// the ticket: this message, at 80 columns, used to end its longest
    /// line flush against the border (`…what you send│`); it does not here.
    #[test]
    fn the_longest_line_never_touches_the_border_because_the_wrap_width_reserves_its_padding() {
        // Exactly 60 cells: `confirm_area` caps the box's own width at 64
        // whatever the frame is (`width = area.width.min(64)`), so
        // `wrap_message` is called with `64 - 4 = 60` — the longest a line
        // can be without wrapping, and the boundary the fault lived on (one
        // cell longer wraps instead; shorter already had slack). Built from
        // real words so `wrap_line`'s own word-wrap does not itself reject
        // it for having no space to break on.
        let longest_line = {
            let mut line = "a ".repeat(29); // 58 cells (29 "a " pairs)
            line.push_str("bb"); // + 2 = 60 cells exactly
            line
        };
        assert_eq!(
            cells(&longest_line),
            60,
            "test fixture must be exactly 60 cells"
        );

        let view = ConfirmView::new(
            sanitize_line("Push main → origin/main"),
            text(&[longest_line.as_str()]),
            sanitize_line("git push --progress origin refs/heads/main:refs/heads/main"),
        )
        .with_labels("Cancel", "Push");

        let drawn = rows(&view, 80, 24);
        let line = drawn
            .iter()
            .find(|line| line.contains(" bb"))
            .unwrap_or_else(|| panic!("no line has the message: {drawn:#?}"));
        // `rows` trims trailing whitespace before returning, which is
        // exactly why the bug read as invisible in a casual look at the
        // string: the padding this test cares about IS the trailing
        // whitespace. So this reads the raw buffer cell, not the trimmed
        // `String`, and checks it directly.
        let theme = theme();
        let frame = Rect::new(0, 0, 80, 24);
        let area = confirm_area(frame, &view);
        let mut buf = Buffer::empty(frame);
        (&Confirm {
            view: &view,
            theme: &theme,
        })
            .render(area, &mut buf);
        let row_y = drawn
            .iter()
            .position(|drawn_line| drawn_line == line)
            .expect("the line found above is in the same Vec");
        let y = u16::try_from(row_y).expect("a test frame is shorter than u16::MAX");
        let border_x = area.right() - 1;
        assert_eq!(
            buf[(border_x, y)].symbol(),
            "│",
            "the box's own right border must still be exactly where expected"
        );
        assert_eq!(
            buf[(border_x - 1, y)].symbol(),
            " ",
            "the cell just inside the right border must be blank — the reserved padding \
             `content_row` now always leaves, so the longest line's last character never sits \
             directly against `│`"
        );
    }

    /// DESIGN.md §1.7 draws every choice bracketed; only the cursor's
    /// carries `▸`. A bare `Re-push old tip` did not read as a button at
    /// all, on the one box whose whole job is to say what the third choice
    /// does (review Major 7). The `▸` is the cue a capture and a `NO_COLOR`
    /// terminal keep; the `REVERSED` modifier rides on top of it and is
    /// **not** verifiable here (CLAUDE.md rule 5).
    #[test]
    fn every_choice_is_a_bracketed_button_and_only_the_cursor_is_marked() {
        let mut view = three_choice_view();
        let drawn = rows(&view, 80, 24);
        let buttons = drawn
            .iter()
            .find(|line| line.contains("Re-push old tip"))
            .unwrap_or_else(|| panic!("no choices row: {drawn:#?}"));
        assert!(buttons.contains("▸[ Cancel ]"), "{buttons:?}");
        assert!(buttons.contains(" [ Undo locally ]"), "{buttons:?}");
        assert!(buttons.contains(" [ Re-push old tip ]"), "{buttons:?}");
        assert_eq!(buttons.matches('▸').count(), 1, "{buttons:?}");

        view.right();
        let moved = rows(&view, 80, 24);
        let after = moved
            .iter()
            .find(|line| line.contains("Re-push old tip"))
            .unwrap_or_else(|| panic!("no choices row: {moved:#?}"));
        assert!(after.contains("▸[ Undo locally ]"), "{after:?}");
        assert_eq!(
            after.chars().count(),
            buttons.chars().count(),
            "the row must not shift as the cursor moves"
        );
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

    fn six_rows() -> ConfirmView {
        ConfirmView::new(
            sanitize_line("Force-push?"),
            text(&["one", "two", "three", "four", "five", "six"]),
            sanitize_line("git push"),
        )
    }

    fn has(drawn: &[String], needle: &str) -> bool {
        drawn.iter().any(|row| row.contains(needle))
    }

    #[test]
    fn default_cap_is_still_four() {
        let drawn = rows(&six_rows(), 80, 24);
        assert!(has(&drawn, "four…"), "{drawn:#?}");
        assert!(!has(&drawn, "five"), "{drawn:#?}");
    }

    #[test]
    fn a_raised_cap_draws_every_row() {
        let drawn = rows(&six_rows().with_max_lines(10), 80, 24);
        for word in ["one", "four", "five", "six"] {
            assert!(has(&drawn, &format!("│ {word}")), "{word}: {drawn:#?}");
        }
        assert!(!has(&drawn, "…"), "{drawn:#?}");
    }

    /// 10 rows of frame: 9 usable (the footer row kept), 6 of them the
    /// box's own borders, blanks, command and buttons — 3 message rows, the
    /// last one marked cut.
    #[test]
    fn a_short_frame_cuts_message_rows_not_the_buttons() {
        let drawn = rows(&six_rows().with_max_lines(10), 80, 10);
        assert!(has(&drawn, "Cancel"), "{drawn:#?}");
        assert!(has(&drawn, "╰"), "{drawn:#?}");
        assert!(has(&drawn, "git push"), "{drawn:#?}");
        assert!(has(&drawn, "three…"), "{drawn:#?}");
        assert!(!has(&drawn, "four"), "{drawn:#?}");
        assert!(drawn[9].is_empty(), "footer row reached: {drawn:#?}");
    }
}
