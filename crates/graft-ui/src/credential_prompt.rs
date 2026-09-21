//! The credential prompt: the one modal Graft shows when `git` calls back
//! over the askpass socket for a username, a password, or an SSH host-key
//! answer (design note "New UI", DESIGN.md §1.14, m4-remotes ticket 03).
//!
//! A bordered box like [`crate::confirm`]'s: the title sits in the top
//! border, a `▏` marks the field row, the note beneath it is dimmed, and the
//! key hints sit in the bottom border. Centred, modal, one field.
//!
//! Pure state and rendering — no I/O, no threads, no key handling. The app
//! layer owns `CredentialPrompt` state and must add it to
//! `AppState::text_field_open()` (M3 06's C1: an unregistered text field
//! leaks `u`/`U`/`Ctrl+Z` straight through to the graph screen's global
//! keys) — that wiring is out of this ticket's crate (`graft`, not
//! `graft-ui`) and belongs to group 06.

use graft_term::SanitizedText;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::widgets::Widget;

use crate::fit::fit;
use crate::frame_box::{border_row, cells, content_row};
use crate::paint::foreground;
use crate::text_input::{FieldView, display_text, draw_line};
use crate::theme::{Theme, Token};

/// Narrower than this and the box does not fit — draw one line instead.
pub const MIN_COLUMNS: u16 = 40;
/// Shorter than this and the box does not fit — draw one line instead.
pub const MIN_ROWS: u16 = 7;

/// The box's own height once it fits at all: title, prompt, field, note,
/// footer.
const BOX_ROWS: u16 = 5;

/// The message shown when the frame is too small for the box. The app still
/// holds the field underneath, so `Enter`/`Esc` answer it blind — there is
/// nowhere on screen to say so, which is why this sentence exists at all.
///
/// Kept short enough (24 cells) to fit whole at the narrowest width that
/// triggers this branch (just below [`MIN_COLUMNS`] = 40) — a longer
/// sentence here would always be cut by [`fit`], defeating the point of
/// having a fallback message at all.
const TOO_SMALL: &str = "too small for the prompt";

/// Everything the credential prompt draws.
#[derive(Clone)]
pub struct CredentialView<'a> {
    /// The top border's label: `Credentials — 127.0.0.1:8443`.
    pub title: SanitizedText,
    /// What is being asked for: `Password for 'http://user@host':`.
    pub prompt: SanitizedText,
    /// The field itself — masked for a password or passphrase, plain for a
    /// username or an SSH host-key answer (`FieldView::secret`).
    pub field: FieldView<'a>,
    /// The dimmed line under the field: `typed here only · never stored by
    /// Graft`.
    pub note: SanitizedText,
    /// The bottom border's key hints: `⏎ send  ⌃U clear  esc cancel the
    /// push`.
    pub footer: SanitizedText,
}

/// [`CredentialView`] drawn into whatever `Rect` [`credential_area`] gives
/// it. Draws nothing outside that rect at any size.
pub struct CredentialPrompt<'a> {
    /// What to draw.
    pub view: &'a CredentialView<'a>,
    /// Colours.
    pub theme: &'a Theme,
    /// Draw a masked field's glyph as `*` instead of `•` (the ASCII
    /// renderer tier) — the same flag [`crate::text_input::FieldWidget`]
    /// takes, threaded through here rather than decided by this widget.
    pub ascii: bool,
}

/// Where the box goes in `frame`: centred, width `min(frame.width - 4, 72)`,
/// [`BOX_ROWS`] tall, never larger than `frame`. Below [`MIN_COLUMNS`]
/// columns or [`MIN_ROWS`] rows, a one-line rect instead — [`CredentialPrompt`]
/// draws [`TOO_SMALL`] into it rather than the box.
#[must_use]
pub fn credential_area(frame: Rect) -> Rect {
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
    let width = frame.width.saturating_sub(4).min(72);
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

impl CredentialPrompt<'_> {
    /// The field row: `│ ▏` then the field itself, masking and cursor
    /// placement owned entirely by `text_input` — this widget never decides
    /// what string represents the typed text (CLAUDE.md choice 2's "must
    /// never"). Reserves one blank cell between the field and the closing
    /// `│`, the same invariant `frame_box::content_row` reserves for every
    /// other row — without it a typed value long enough to fill the field
    /// exactly ran its last character straight into the border.
    fn draw_field_row(&self, area: Rect, buf: &mut Buffer, width: usize, y: u16) {
        buf.set_string(area.x, y, "│ ▏", foreground(self.theme, Token::Text));
        if width <= 5 {
            return;
        }
        let field_width = u16::try_from(width - 5).unwrap_or(0);
        let shown = display_text(&self.view.field, self.ascii);
        // A secret field's real cursor index is the typed length — the
        // same information the mask glyphs and the missing counter are
        // there to hide, so no cursor is drawn for it at all (`text_input`'s
        // `FieldWidget` makes the same call for the same reason).
        let cursor = self
            .view
            .field
            .secret
            .is_none()
            .then_some(self.view.field.cursor)
            .flatten();
        draw_line(
            buf,
            Rect {
                x: area.x + 3,
                y,
                width: field_width,
                height: 1,
            },
            &shown,
            cursor,
            None,
            self.theme,
        );
        let close_x = area.x + 4 + field_width;
        buf.set_string(close_x, y, "│", foreground(self.theme, Token::Text));
    }
}

impl Widget for &CredentialPrompt<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let width = usize::from(area.width);
        let view = self.view;

        if area.height < BOX_ROWS {
            // `fit` pads to exactly its `width` argument, which would make
            // the message always as wide as the frame and defeat centring
            // — so it is only ever asked to cut, never to pad, by capping
            // the width it's given at the message's own cell width.
            let message_width = cells(TOO_SMALL).min(width);
            let message = fit(TOO_SMALL, message_width);
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
        buf.set_string(
            area.x,
            area.y + 1,
            content_row(view.prompt.as_str(), width),
            foreground(self.theme, Token::Text),
        );
        self.draw_field_row(area, buf, width, area.y + 2);
        buf.set_string(
            area.x,
            area.y + 3,
            content_row(view.note.as_str(), width),
            foreground(self.theme, Token::TextMuted),
        );
        buf.set_string(
            area.x,
            area.y + 4,
            border_row('╰', '╯', view.footer.as_str(), width),
            foreground(self.theme, Token::TextMuted),
        );
    }
}

#[cfg(test)]
mod tests;
