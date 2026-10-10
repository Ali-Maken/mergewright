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

use graft_term::{SanitizedText, sanitize_line};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;

use crate::frame_box::{border_row, buttons_row, cells, content_row, reverse_button};
use crate::keymap::Keymap;

mod footer;

use crate::paint::foreground;
use crate::theme::{Theme, Token};
pub use footer::Again;
pub(crate) use footer::footer_label;
use footer::too_small;

/// Below this width or this height **the screen** does not fit the box: "a
/// picture that cannot be seen is never confirmed" (design note). `⏎` does
/// nothing at this size — the caller must not act on a plan the user could
/// not read.
///
/// Measured against the frame, not against the rect the box is docked into:
/// the box's own height is [`preview_rows`], which is 7 for a typical plan
/// and so would fail a 12-row test of its own rect every time. Use
/// [`preview_box_area`], which applies this to the frame and hands the
/// widget either a box-sized rect or the one row the message needs.
pub const PREVIEW_MIN: (u16, u16) = (40, 12);

/// The fewest rows a box can be: the title border, one content row, the
/// choices row, the footer border. Given less than this the widget draws
/// `too_small` rather than a frame with nothing legible inside it.
pub const MIN_BOX_ROWS: u16 = 4;

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
    /// Private, with [`PreviewBox::new`] the only way in, so no caller can
    /// build a box whose first choice is the acting one — the same reason
    /// `SecretField` lost its `Default` in M5 group 01.
    choices: Vec<SanitizedText>,
    /// Which choice the cursor is on. `0` (`Cancel`) on open, and private
    /// for the same reason: the default never varies with how dangerous the
    /// action is (ADR-0038 §1).
    cursor: usize,
    /// The forecast has not landed yet: draws `predicting…` in place of
    /// `lines`, for up to about a second while the plan is worked out. The
    /// ghost rows themselves appear only once this turns false.
    pub planning: bool,
    /// Draw `y confirm` in the footer: true for every operation preview,
    /// false for a journal revert (which `y` does not confirm).
    pub confirm_key: bool,
    /// The confirm named by its verb (`y apply`, `y put it back`) or by
    /// the opening verb, when pressing it again confirms (`m merge`,
    /// `r rebase`); `None` for every other opener. Spelled at render from
    /// the keymap ([`Again`]).
    pub again: Option<Again>,
    /// Draw `R keep` right after the `again` slot: `R` opens the rewrite
    /// editor on this rebase with its suggested drops marked
    /// (rebase-preview-keeps-drops). Whole or absent, never cut.
    pub keep: bool,
    /// **No longer read** (m7-power-tools/36): the footer is `esc back`
    /// whenever the box has one choice. Still set for the stash put-back's
    /// closing boxes only because `graft/src/update/stash/tests.rs` reads it
    /// and that file is another ticket's this wave; removed with that read.
    pub back_only: bool,
}

impl PreviewBox {
    /// An empty box titled `title`, choices `Cancel`/`Confirm`, cursor on
    /// `Cancel` — the only way to build one, so the safe choice is first and
    /// under the cursor in every preview, whatever the plan does
    /// (ADR-0038 §1). Fill `lines`, `commands` and `planning` after.
    #[must_use]
    pub fn new(title: SanitizedText) -> Self {
        PreviewBox {
            title,
            lines: Vec::new(),
            commands: Vec::new(),
            choices: vec![sanitize_line("Cancel"), sanitize_line("Confirm")],
            cursor: 0,
            planning: false,
            confirm_key: false,
            again: None,
            keep: false,
            back_only: false,
        }
    }

    /// The same box with its own choices — `labels[0]` is the safe one and
    /// is where the cursor goes back to (ADR-0038 §1).
    #[must_use]
    pub fn with_choices(mut self, labels: Vec<SanitizedText>) -> Self {
        self.choices = labels;
        self.cursor = 0;
        self
    }

    /// The choices, left to right.
    #[must_use]
    pub fn choices(&self) -> &[SanitizedText] {
        &self.choices
    }

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
pub fn preview_rows(view: &PreviewBox, _width: u16) -> u16 {
    let body = if view.planning { 1 } else { view.lines.len() };
    let rows = 1 // top border + title
        + body
        + view.commands.len()
        + 1 // choices row
        + 1; // bottom border + footer
    u16::try_from(rows).unwrap_or(u16::MAX)
}

/// Where the box goes in `frame`: docked to the bottom, at its own height
/// ([`preview_rows`]), with the frame's last row left to the screen footer
/// — and, from 30 rows, the rule above it too
/// ([`crate::graph_screen::spaced`], ADR-0058 §4), so the box never stands
/// on the line that separates the rows from the bar.
///
/// When the **frame** is smaller than [`PREVIEW_MIN`] this returns the one
/// row the "preview needs 40x12" message takes instead, still above the
/// footer — the same shape as [`crate::name_prompt::name_prompt_area`]. The
/// size decision lives here so the widget is only ever handed a rect it can
/// draw a real box in.
#[must_use]
pub fn preview_box_area(frame: Rect, view: &PreviewBox) -> Rect {
    crate::rect_of(dock_cells(
        crate::cells_of(frame),
        preview_rows(view, frame.width),
    ))
}

/// [`preview_box_area`] in plain numbers, for a caller that must not name
/// the rendering library (`graft::update` hit-tests the box's buttons
/// against it). `wanted` is the box's own height, [`preview_rows`] of the
/// view at `frame`'s width; it is not read below [`PREVIEW_MIN`].
#[must_use]
pub fn dock_cells(frame: crate::Cells, wanted: u16) -> crate::Cells {
    let (x, y, width, height) = frame;
    let footer_row = u16::from(height > 1);
    let rule_row = u16::from(crate::graph_screen::spaced(height));
    let usable = height.saturating_sub(footer_row + rule_row);
    let rows = if width < PREVIEW_MIN.0 || height < PREVIEW_MIN.1 {
        usable.min(1)
    } else {
        wanted.max(MIN_BOX_ROWS).min(usable)
    };
    (x, y + usable.saturating_sub(rows), width, rows)
}

/// [`PreviewBox`] drawn into whatever `Rect` the caller docks it to —
/// bottom of the graph area, never over the footer row.
pub struct PreviewBoxWidget<'a> {
    /// What to draw.
    pub view: &'a PreviewBox,
    /// Colours.
    pub theme: &'a Theme,
    /// His keymap: the footer's keys are spelled from it.
    pub keys: &'a Keymap,
}

impl PreviewBoxWidget<'_> {
    /// The lines, commands and choice buttons between the title and footer
    /// borders. **Every** interior row is a `│ … │` content row, blank ones
    /// included: docked into a rect taller than [`preview_rows`] the box
    /// still reads as a box, instead of a title and a footer with a hole
    /// between them (M5 group 04 review, Critical 1).
    ///
    /// The buttons sit on the last interior row, right above the footer, so
    /// the slack goes between the plan and the choices.
    fn draw_body(&self, area: Rect, width: usize, buf: &mut Buffer) {
        let view = self.view;
        let text_style = foreground(self.theme, Token::Text);
        let muted = foreground(self.theme, Token::TextMuted);

        let mut body: Vec<(&str, Style)> = Vec::new();
        if view.planning {
            body.push(("predicting…", muted));
        } else {
            body.extend(view.lines.iter().map(|line| (line.as_str(), text_style)));
        }
        body.extend(view.commands.iter().map(|line| (line.as_str(), muted)));

        let first = area.y.saturating_add(1);
        // The buttons row; `render` has already checked there is one.
        let buttons_y = area.bottom().saturating_sub(2);
        let room = usize::from(buttons_y.saturating_sub(first));
        for (index, (text, style)) in body.iter().take(room).enumerate() {
            let y = first.saturating_add(u16::try_from(index).unwrap_or(u16::MAX));
            buf.set_string(area.x, y, content_row(text, width), *style);
        }
        let filled = u16::try_from(body.len().min(room)).unwrap_or(u16::MAX);
        for y in first.saturating_add(filled)..buttons_y {
            buf.set_string(area.x, y, content_row("", width), text_style);
        }

        let labels: Vec<&str> = view.choices.iter().map(SanitizedText::as_str).collect();
        let (buttons, cursor) = buttons_row(&labels, view.cursor);
        buf.set_string(area.x, buttons_y, content_row(&buttons, width), text_style);
        reverse_button(buf, area, buttons_y, cursor, text_style);

        buf.set_string(
            area.x,
            area.bottom().saturating_sub(1),
            border_row(
                '╰',
                '╯',
                &footer_label(view, width.saturating_sub(2), self.keys),
                width,
            ),
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

        // Against the rect, not against `PREVIEW_MIN`: the screen-size
        // decision is `preview_box_area`'s, and a box at its own height is
        // 7 rows, not 12. What the widget checks is whether *this* rect can
        // hold a frame with something legible in it.
        if area.width < PREVIEW_MIN.0 || area.height < MIN_BOX_ROWS {
            let message = crate::fit::fit(&too_small(self.keys), width);
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

        buf.set_string(
            area.x,
            area.y,
            border_row('╭', '╮', view.title.as_str(), width),
            text_style,
        );

        self.draw_body(area, width, buf);
    }
}

/// The set-aside row a rebase or checkout over a dirty tree adds under
/// `working tree:` (m6-conflicts/74; ADR-0045 §4, amended 2026-09-26 b):
/// `files` tracked files set aside. Wordings longest first,
/// `your edits: N files set aside, put back after`, then
/// `your edits: N files set aside`. No staging words: the put-back keeps
/// the staging, as `git switch` does. [`fitting_line`] picks one for a width.
///
/// A count, not `graft_ops::operation::SetAside`: `ui` does not depend on
/// `ops` (ARCHITECTURE.md crate rules).
#[must_use]
pub fn set_aside_row(files: usize) -> Vec<String> {
    let head = format!(
        "your edits: {} set aside",
        crate::stash_screen::files_label(files)
    );
    let after = format!("{head}, put back after");
    vec![after, head]
}

/// The longest of `wordings` (longest first) that fits the box's text
/// column at `width` columns — the borders and their one-cell gutters
/// taken off. When none fits, the last, which the row's own `fit` then
/// cuts from the right; no wordings at all is an empty row.
#[must_use]
pub fn fitting_line(wordings: &[String], width: u16) -> String {
    let text = usize::from(width).saturating_sub(4);
    match wordings.iter().find(|words| cells(words) <= text) {
        Some(fits) => fits.clone(),
        None => wordings.last().cloned().unwrap_or_else(String::new),
    }
}

#[cfg(test)]
mod tests;
