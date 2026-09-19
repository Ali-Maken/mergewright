//! The commit editor's screen: a title, a subject line, a body, the four
//! option toggles and the `[ Commit ]` button (DESIGN.md §2.10, design note
//! "Commit editor", m3-working-tree-undo-and-stash/05).
//!
//! Owns no state and reads no repository: [`crate::text_input`] holds the
//! text, `graft::update::commit_editor` holds the rest, and
//! `graft::view::commit_editor` turns that into the [`CommitEditorView`] this
//! draws. Like the Map view, it fills the graph area and leaves the detail
//! pane alone.
//!
//! Every fact the user needs is in **characters**, never in colour or
//! weight alone: which field has focus (`▸`), whether an option is on
//! (`[x]`), why the button will not fire (the reason beside it). A capture
//! records the character grid and nothing else, so anything that lives only
//! in an attribute cannot be checked afterwards (CLAUDE.md).

use graft_term::SanitizedText;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;

use crate::fit::fit;
use crate::paint::foreground;
use crate::text_input::{AreaView, AreaWidget, FOCUS, FieldView, FieldWidget};
use crate::theme::{Theme, Token};

/// Characters the subject is meant to stay inside: the `┊` guide sits just
/// past it, and the counter reads `n/50`.
pub const SUBJECT_LIMIT: usize = 50;

/// Characters a body line is meant to stay inside; its `┊` guide's column.
pub const BODY_LIMIT: u16 = 72;

/// What has focus in the editor. `Tab` cycles the four **regions** — subject,
/// body, options, button — and `←`/`→` move between the four options once the
/// options region has it (DESIGN.md §2.10's "move between subject / body /
/// options", ADR-0034's `[ Commit ]` button).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    /// The one-line subject.
    Subject,
    /// The multi-line body.
    Body,
    /// `[ ] amend`.
    Amend,
    /// `[ ] sign-off`.
    SignOff,
    /// `[ ] no-verify`.
    NoVerify,
    /// `[ ] allow empty`.
    AllowEmpty,
    /// `[ Commit ]`.
    CommitButton,
}

impl Field {
    /// The four option toggles, in the order the row draws them.
    pub const OPTIONS: [Field; 4] = [
        Field::Amend,
        Field::SignOff,
        Field::NoVerify,
        Field::AllowEmpty,
    ];

    /// Is this one of the option toggles?
    #[must_use]
    pub fn is_option(self) -> bool {
        Self::OPTIONS.contains(&self)
    }

    /// The option's label, as the row draws it and as a `--flag` would read.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Field::Subject => "Subject",
            Field::Body => "Body",
            Field::Amend => "amend",
            Field::SignOff => "sign-off",
            Field::NoVerify => "no-verify",
            Field::AllowEmpty => "allow empty",
            Field::CommitButton => "Commit",
        }
    }

    /// The next region `Tab` moves to: subject → body → options → button →
    /// subject. Within the options region the first option takes the focus;
    /// `←`/`→` pick among them.
    #[must_use]
    pub fn next(self) -> Field {
        match self {
            Field::Subject => Field::Body,
            Field::Body => Field::Amend,
            Field::CommitButton => Field::Subject,
            option => {
                debug_assert!(option.is_option());
                Field::CommitButton
            }
        }
    }

    /// The previous region, for `Shift+Tab`.
    #[must_use]
    pub fn previous(self) -> Field {
        match self {
            Field::Subject => Field::CommitButton,
            Field::Body => Field::Subject,
            Field::CommitButton => Field::Amend,
            option => {
                debug_assert!(option.is_option());
                Field::Body
            }
        }
    }

    /// The option one step left or right of this one, staying inside the
    /// row. `None` when the focus is not on an option at all.
    #[must_use]
    pub fn sibling_option(self, forward: bool) -> Option<Field> {
        let at = Self::OPTIONS.iter().position(|option| *option == self)?;
        let next = if forward {
            (at + 1).min(Self::OPTIONS.len() - 1)
        } else {
            at.saturating_sub(1)
        };
        Some(Self::OPTIONS[next])
    }
}

/// Everything the commit editor draws.
#[derive(Clone, Debug)]
pub struct CommitEditorView<'a> {
    /// The title row, already assembled: `COMMIT ── ⎇ main ── 2 files
    /// staged`, `AMEND a1b2c3d …`, or `committing…`.
    pub title: SanitizedText,
    /// The subject line.
    pub subject: FieldView<'a>,
    /// The body.
    pub body: AreaView<'a>,
    /// The four toggles and whether each is on, in the row's order.
    pub options: [(Field, bool); 4],
    /// What has focus.
    pub focus: Field,
    /// Why `[ Commit ]` would do nothing, if it would: drawn beside the
    /// dimmed button, and repeated on the title row.
    pub refusal: Option<SanitizedText>,

    /// Why the last write failed, while it is still worth saying: a
    /// pre-commit hook's refusal, an amend whose `HEAD` could not be read, an
    /// `$EDITOR` that would not start. It takes the footer row, because a
    /// reason the user cannot read is the same as no reason at all — and a
    /// rejected commit is an ordinary day, not an edge case
    /// (m3/05 review, Major 1).
    pub status: Option<SanitizedText>,
    /// A commit is running: the button is dimmed and every key but `Esc` is
    /// ignored.
    pub busy: bool,
}

/// [`CommitEditorView`] drawn into the area it is given — the graph's, with
/// the detail pane left alone (design note, hard choice 4).
#[derive(Debug)]
pub struct CommitEditorScreen<'a> {
    /// What to draw.
    pub view: &'a CommitEditorView<'a>,
    /// Colours.
    pub theme: &'a Theme,
}

/// The key hints under the editor (design note, "Commit editor").
const FOOTER: &str = "⌃⏎/Commit commit  ⇥ next  ⌃E $EDITOR  ⌃S sign-off  esc cancel";

/// Rows the editor needs before the body gets any: title, blank, subject,
/// blank, then the options row and the footer at the bottom.
const CHROME_ROWS: u16 = 6;

impl Widget for &CommitEditorScreen<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        // The editor replaces the graph, so it owns its background: without
        // this the rows it does not write keep whatever was drawn there.
        buf.set_style(
            area,
            Style::new().bg(crate::paint::color(self.theme.token(Token::Bg))),
        );

        let width = usize::from(area.width);
        buf.set_string(
            area.x,
            area.y,
            fit(self.view.title.as_str(), width),
            foreground(self.theme, Token::Accent).add_modifier(Modifier::BOLD),
        );
        if area.height == 1 {
            return;
        }

        // The footer and the options row sit at the bottom; the body takes
        // whatever is left between the subject and them.
        let footer_row = area.bottom() - 1;
        // A failure takes the footer from the key hints, the way the graph
        // screen's `problem` does: the hints are still discoverable, the
        // reason a commit was refused is not (m3/05 review, Major 1).
        let (footer, token) = match &self.view.status {
            Some(status) => (
                format!("{} — esc dismisses", status.as_str()),
                Token::Warning,
            ),
            None => (FOOTER.to_owned(), Token::TextMuted),
        };
        buf.set_string(
            area.x,
            footer_row,
            fit(&footer, width),
            foreground(self.theme, token),
        );
        if area.height == 2 {
            return;
        }
        self.draw_options(buf, area, footer_row - 1);
        if area.height == 3 {
            return;
        }

        self.draw_text(buf, area, footer_row);
    }
}

impl CommitEditorScreen<'_> {
    /// The subject line, and the body in whatever rows are left between it
    /// and the options row. Either is skipped when there is no room.
    fn draw_text(&self, buf: &mut Buffer, area: Rect, footer_row: u16) {
        let subject_row = area.y + 2;
        if subject_row >= footer_row - 1 {
            return;
        }
        (&FieldWidget {
            view: &self.view.subject,
            theme: self.theme,
        })
            .render(
                Rect {
                    x: area.x,
                    y: subject_row,
                    width: area.width,
                    height: 1,
                },
                buf,
            );

        let body_top = subject_row + 2;
        let Some(body_height) = (footer_row - 1).checked_sub(body_top) else {
            return;
        };
        if body_height == 0 {
            return;
        }
        (&AreaWidget {
            view: &self.view.body,
            theme: self.theme,
        })
            .render(
                Rect {
                    x: area.x,
                    y: body_top,
                    width: area.width,
                    height: body_height,
                },
                buf,
            );
    }

    /// `▸ [x] amend   [ ] sign-off …   ▸ [ Commit ]  nothing staged`.
    fn draw_options(&self, buf: &mut Buffer, area: Rect, row: u16) {
        let mut line = String::new();
        for (field, on) in &self.view.options {
            let marker = if self.view.focus == *field {
                FOCUS
            } else {
                ' '
            };
            let box_ = if *on { 'x' } else { ' ' };
            line.push_str(&format!("{marker} [{box_}] {}  ", field.label()));
        }
        let marker = if self.view.focus == Field::CommitButton {
            FOCUS
        } else {
            ' '
        };
        line.push_str(&format!("{marker} [ Commit ]"));
        if let Some(refusal) = &self.view.refusal {
            line.push_str("  ");
            line.push_str(refusal.as_str());
        } else if self.view.busy {
            line.push_str("  committing…");
        }
        let token = if self.view.refusal.is_some() || self.view.busy {
            Token::TextMuted
        } else {
            Token::Text
        };
        buf.set_string(
            area.x,
            row,
            fit(&line, usize::from(area.width)),
            foreground(self.theme, token),
        );
    }
}

/// Whether an area is tall enough for the editor to draw a body line at all.
/// Below this it still draws — title, subject, options, footer — but the body
/// is off screen, so the caller can say so.
#[must_use]
pub fn fits(area: Rect) -> bool {
    area.height > CHROME_ROWS && area.width > 0
}

#[cfg(test)]
mod tests;
