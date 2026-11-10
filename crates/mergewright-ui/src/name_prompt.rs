//! The name prompt: the one-field modal for naming something new — a
//! branch, a tag, a `stash branch` (`b`, `t`, stash `b`; design note "New
//! UI: Name prompt"). Copies [`crate::credential_prompt`]'s shape (a
//! bordered box, the title in the top border, a `▏` marks the field row,
//! the bottom border carries the key hints) with one addition: an optional
//! toggle (`(•) check out after creating`) and a refusal line for a name
//! git would reject.
//!
//! Pure state and rendering — no I/O, no key handling. `mergewright` (not
//! `mergewright-ui`) owns the prompt's `AppState` and must register it with
//! `text_field_open()`, the same wiring `credential_prompt` needed
//! (m4-remotes ticket 03) — out of this ticket's crate.

use mergewright_term::SanitizedText;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::widgets::Widget;

use crate::frame_box::{border_row, content_row};
use crate::paint::foreground;
use crate::text_input::{FieldView, display_text, draw_line};
use crate::theme::{Theme, Token};

/// Narrower than this and the box does not fit — draw one line instead.
pub const MIN_COLUMNS: u16 = 40;
/// Shorter than this and the box does not fit — draw one line instead.
pub const MIN_ROWS: u16 = 7;

/// The box's own height once it fits at all: title (top border), field,
/// option, refusal, a blank row, footer (bottom border).
const BOX_ROWS: u16 = 6;

/// The message shown when the frame is too small for the box, same reason
/// [`crate::credential_prompt`]'s exists: the field underneath still answers
/// `Enter`/`Esc` blind, and this says why nothing on screen explains that.
const TOO_SMALL: &str = "too small for the prompt";

/// Everything the name prompt draws.
#[derive(Clone)]
pub struct NamePromptView<'a> {
    /// The top border's label: `New branch at a1b2c3d`.
    pub title: SanitizedText,
    /// The name being typed.
    pub field: FieldView<'a>,
    /// The checkout-after-creating toggle, when the caller offers one
    /// (branch and stash-branch do; tag does not): its label and whether it
    /// is on.
    pub option: Option<(SanitizedText, bool)>,
    /// Why the typed name would be refused right now (`exists`, `bad name:
    /// …`), live as the user types. `None` while the name is fine so far.
    pub refusal: Option<SanitizedText>,
}

/// [`NamePromptView`] drawn into whatever `Rect` [`name_prompt_area`] gives
/// it. Draws nothing outside that rect at any size.
pub struct NamePrompt<'a> {
    /// What to draw.
    pub view: &'a NamePromptView<'a>,
    /// Colours.
    pub theme: &'a Theme,
}

/// Where the box goes in `frame`: centred, width `min(frame.width - 4, 60)`,
/// [`BOX_ROWS`] tall, never larger than `frame`. Below [`MIN_COLUMNS`]
/// columns or [`MIN_ROWS`] rows, a one-line rect instead — [`NamePrompt`]
/// draws [`TOO_SMALL`] into it rather than the box.
#[must_use]
pub fn name_prompt_area(frame: Rect) -> Rect {
    if frame.width < MIN_COLUMNS || frame.height < MIN_ROWS {
        let height = frame.height.min(1);
        let y = frame.y + frame.height.saturating_sub(height) / 2;
        return Rect {
            x: frame.x,
            y,
            width: frame.width,
            height,
        };
    }
    let width = frame.width.saturating_sub(4).min(60);
    let height = BOX_ROWS.min(frame.height);
    let x = frame.x + frame.width.saturating_sub(width) / 2;
    let y = frame.y + frame.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width,
        height,
    }
}

/// Cells `text` occupies, counting a wide character as two.
fn cells(text: &str) -> usize {
    text.chars()
        .filter_map(unicode_width::UnicodeWidthChar::width)
        .sum()
}

impl NamePrompt<'_> {
    /// The field row: `│ ▏` then the field itself, masking and cursor
    /// placement owned entirely by `text_input` (never this widget's call —
    /// same rule `credential_prompt` follows). Same reserved gutter cell
    /// before the closing `│` as `credential_prompt::draw_field_row` — a
    /// typed name long enough to fill the field must not run into the
    /// border.
    fn draw_field_row(&self, area: Rect, buf: &mut Buffer, width: usize, y: u16) {
        buf.set_string(area.x, y, "│ ▏", foreground(self.theme, Token::Text));
        if width <= 5 {
            return;
        }
        let field_width = u16::try_from(width - 5).unwrap_or(0);
        let shown = display_text(&self.view.field, false);
        draw_line(
            buf,
            Rect {
                x: area.x + 3,
                y,
                width: field_width,
                height: 1,
            },
            &shown,
            self.view.field.cursor,
            None,
            self.theme,
        );
        let close_x = area.x + 4 + field_width;
        buf.set_string(close_x, y, "│", foreground(self.theme, Token::Text));
    }
}

impl Widget for &NamePrompt<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let width = usize::from(area.width);
        let view = self.view;

        if area.height < BOX_ROWS {
            let message_width = cells(TOO_SMALL).min(width);
            let message = crate::fit::fit(TOO_SMALL, message_width);
            let x = area.x + u16::try_from((width - message_width) / 2).unwrap_or(0);
            buf.set_stringn(
                x,
                area.y,
                message,
                width,
                foreground(self.theme, Token::Warning),
            );
            return;
        }

        buf.set_string(
            area.x,
            area.y,
            border_row('╭', '╮', view.title.as_str(), width),
            foreground(self.theme, Token::Text).add_modifier(Modifier::BOLD),
        );
        self.draw_field_row(area, buf, width, area.y + 1);

        let option_line = option_line(view.option.as_ref());
        buf.set_string(
            area.x,
            area.y + 2,
            content_row(&option_line, width),
            foreground(self.theme, Token::TextMuted),
        );

        let (refusal_text, refusal_style) = match &view.refusal {
            Some(refusal) => (refusal.as_str(), foreground(self.theme, Token::Warning)),
            None => ("", foreground(self.theme, Token::TextMuted)),
        };
        buf.set_string(
            area.x,
            area.y + 3,
            content_row(refusal_text, width),
            refusal_style,
        );

        buf.set_string(
            area.x,
            area.y + 4,
            content_row("", width),
            foreground(self.theme, Token::Text),
        );

        buf.set_string(
            area.x,
            area.y + 5,
            // No keys: the bar names them (`HintContext::NamePrompt`); drawn
            // here too they were said twice (m7-power-tools/15, :856).
            border_row('╰', '╯', "", width),
            foreground(self.theme, Token::TextMuted),
        );
    }
}

/// `(•) <label> · ⇥ toggles`. A text field: its key is the shipped one
/// whatever his config says (ADR-0055). Empty without an option.
fn option_line(option: Option<&(SanitizedText, bool)>) -> String {
    match option {
        Some((label, on)) => crate::keymap::Keymap::shipped().after(
            &format!(
                "{} {}",
                if *on { "(\u{2022})" } else { "( )" },
                label.as_str()
            ),
            crate::keymap::ActionId("app.next_pane"),
            "toggles",
        ),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests;
