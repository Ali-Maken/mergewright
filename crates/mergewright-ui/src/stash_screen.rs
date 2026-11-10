//! The stash overlay (`S`, DESIGN.md §1.11): every stash entry this
//! repository holds, what each one touched, and the save prompt
//! (m3-working-tree-undo-and-stash/06).
//!
//! An overlay over the graph area, like `journal_screen`: it owns no state
//! and no I/O, and `view::stash::stash_view` (crate `mergewright`) builds its
//! input. Every fact is in the text — `stash@{0}`, `3 files`, the chosen
//! scope's `(•)` — because a capture carries no colour and neither does a
//! `NO_COLOR` terminal.

use mergewright_term::sanitize::SanitizedText;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;
use unicode_width::UnicodeWidthStr;

use crate::detail::{FileListView, FileRowKind};
use crate::failure::marked_if;
use crate::fit::fit;
use crate::graph_screen::hints::{place_close, place_footer_text};
use crate::keymap::{ActionId, Keymap};
use crate::paint::foreground;
use crate::text_input::{FieldView, FieldWidget};
use crate::theme::{Theme, Token};

mod view;

pub use view::{StashPromptView, StashRowView, StashScreen, StashStatus, StashView, files_label};

/// The empty state, which is also the instruction for leaving it.
/// `no stashes — s stashes the working tree`, the key spelled from `keys`;
/// `no stashes` alone when he left `stash.save` unbound.
fn empty(keys: &Keymap) -> String {
    match keys.spell_keys(&[ActionId("stash.save")]) {
        Some(key) => format!("no stashes — {key} stashes the working tree"),
        None => "no stashes".to_owned(),
    }
}

/// `<keys> <verb>` pairs spelled from `keys` ([`Keymap::key_verbs`]).
fn spelled(keys: &Keymap, parts: &[(&'static str, &str)]) -> String {
    keys.key_verbs(parts)
}

/// The message column's width: what is left once the name, the age and the
/// file count have had theirs, capped so one long message does not push
/// every age off a wide screen.
fn message_columns(width: u16) -> usize {
    const NAME: usize = 2 + 11 + 1;
    const AGE_AND_FILES: usize = 9 + 1 + 9;
    usize::from(width)
        .saturating_sub(NAME + AGE_AND_FILES)
        .min(48)
}

/// `text` padded to `columns` display columns, or left alone when it is
/// already wider — the row clips at the right edge either way.
fn pad(text: &str, columns: usize) -> String {
    let mut padded = text.to_owned();
    for _ in text.width()..columns {
        padded.push(' ');
    }
    padded
}

/// One text write, clipped to `area` and dropped when it would fall outside
/// it. Mirrors `journal_screen`'s own guard.
fn set_text(buf: &mut Buffer, x: u16, y: u16, area: Rect, text: &str, style: Style) {
    if x >= area.right() || y >= area.bottom() {
        return;
    }
    let width = usize::from(area.right() - x);
    buf.set_stringn(x, y, fit(text, width), width, style);
}

/// A line centred in `area`, for the three states that have no rows.
fn centred(buf: &mut Buffer, area: Rect, text: &str, style: Style) {
    if area.is_empty() {
        return;
    }
    let width = usize::from(area.width);
    let drawn = fit(text, width);
    let columns = u16::try_from(drawn.chars().count()).unwrap_or(area.width);
    let x = area.x + area.width.saturating_sub(columns) / 2;
    let y = area.y + area.height / 2;
    set_text(buf, x, y, area, &drawn, style);
}

/// The status letter a file row carries in the preview, the same one the
/// detail pane draws.
fn status_letter(row: &crate::detail::FileRow) -> Option<char> {
    match &row.kind {
        FileRowKind::File { status, .. } => Some(crate::detail_pane::status_letter(status)),
        _ => None,
    }
}

impl StashScreen<'_> {
    /// `STASHES (2)`.
    fn header(&self) -> String {
        format!("STASHES ({})", self.view.rows.len())
    }

    /// The keys, spelled out, fitted to `width`. `⏎ apply` first: it is
    /// what the list is for. The list's own keys end with the `? keys`
    /// doorway and shed by the journal's rule (m8/11e F2, E1); the save
    /// prompt's line is the prompt's, with no doorway (`?` types there).
    fn footer(&self, width: u16) -> String {
        // The box over the list holds every key and draws its own footer:
        // the list's bar would name keys it does not answer (review 86, M2).
        if self.view.box_open {
            return String::new();
        }
        if self.view.prompt.is_some() {
            // A text field: its keys are its defaults whatever his config
            // says (ADR-0055), so they spell from the shipped table.
            return spelled(
                Keymap::shipped(),
                &[
                    ("app.open", "save"),
                    ("app.next_pane", "scope"),
                    ("app.back", "back"),
                ],
            );
        }
        let keys = self.keys;
        // `S` and `esc` both close: one clause, both keys (`S/esc close`) —
        // except while a write runs, when `esc` stops it and `S` alone
        // closes (`esc cancels` … `S close`, m8/59 Major A).
        let (cancels, close) = place_close(
            keys,
            ActionId("stash.close"),
            "close",
            self.view.esc_cancels,
        );
        let parts: &[(&[&str], &str)] = match self.view.status {
            StashStatus::Ready => &[
                (&["stash.apply"], "apply"),
                (&["stash.pop"], "pop"),
                (&["stash.save"], "save"),
                (&["stash.drop"], "drop"),
                // A branch from the stash under the cursor (§2.9's order);
                // Ready is the only status with one selected (m8/11e G1).
                (&["stash.branch"], "branch"),
                (&["stash.preview"], "files"),
            ],
            StashStatus::Loading | StashStatus::Empty | StashStatus::Failed(_) => {
                &[(&["stash.save"], "save")]
            }
        };
        let clauses: Vec<String> = cancels
            .into_iter()
            .chain(parts.iter().filter_map(|(ids, verb)| {
                let ids: Vec<ActionId> = ids.iter().map(|id| ActionId(id)).collect();
                keys.key_verb(&ids, verb)
            }))
            .chain(close)
            .collect();
        place_footer_text(&clauses, keys, usize::from(width))
    }

    /// One row of the list.
    fn draw_row(&self, buf: &mut Buffer, area: Rect, y: u16, index: usize) {
        let Some(row) = self.view.rows.get(index) else {
            return;
        };
        let on_cursor = index == self.view.selected;
        let text = foreground(self.theme, Token::Text);
        let style = if on_cursor {
            crate::paint::on_selection(self.theme, text)
                .bg(crate::paint::color(self.theme.token(Token::Selection)))
                .add_modifier(Modifier::BOLD)
        } else {
            text
        };
        if on_cursor {
            // Painted first, so the cursor's background reaches the right
            // edge rather than stopping where the text does.
            let blanks = " ".repeat(usize::from(area.width));
            set_text(buf, area.x, y, area, &blanks, style);
        }
        let prefix = if on_cursor { "▸ " } else { "  " };
        let line = format!(
            "{prefix}{:<11} {} {:<9}{}",
            row.name.as_str(),
            pad(row.message.as_str(), message_columns(area.width)),
            row.age.as_str(),
            files_label(row.files),
        );
        set_text(buf, area.x, y, area, &line, style);
    }

    /// The selected stash's files under the list, once `Space` has asked.
    /// A file list with counts and no diff body: the overlay's job is to
    /// say *what* a stash would bring back, and the diff of a stash is the
    /// detail pane's own view (m3-working-tree-undo-and-stash/06).
    fn draw_preview(&self, buf: &mut Buffer, area: Rect, preview: &FileListView) {
        if area.is_empty() {
            return;
        }
        let muted = foreground(self.theme, Token::TextMuted);
        let text = foreground(self.theme, Token::Text);
        let name = self
            .view
            .rows
            .get(self.view.selected)
            .map_or("", |row| row.name.as_str());
        set_text(buf, area.x, area.y, area, &format!("── {name} ──"), muted);
        let mut y = area.y.saturating_add(1);
        for (_, row) in preview.visible() {
            if y >= area.bottom() {
                return;
            }
            let indent = "  ".repeat(usize::from(row.depth) + 1);
            let line = match &row.kind {
                FileRowKind::File { added, removed, .. } => {
                    let letter = status_letter(row).unwrap_or(' ');
                    // A binary file has no line counts at all, and "+0 −0"
                    // would be a lie rather than an absence.
                    let counts = match (added, removed) {
                        (Some(added), Some(removed)) => format!("  +{added} −{removed}"),
                        _ => "  binary".to_owned(),
                    };
                    format!("{indent}{letter} {}{counts}", row.name.as_str())
                }
                _ => format!("{indent}{}", row.name.as_str()),
            };
            set_text(buf, area.x, y, area, &line, text);
            y = y.saturating_add(1);
        }
    }

    /// The save prompt: title, the message field, the scope options, and
    /// the refusal when there is one.
    fn draw_prompt(&self, buf: &mut Buffer, area: Rect, prompt: &StashPromptView<'_>) {
        if area.is_empty() {
            return;
        }
        let text = foreground(self.theme, Token::Text);
        set_text(
            buf,
            area.x,
            area.y,
            area,
            prompt.title.as_str(),
            text.add_modifier(Modifier::BOLD),
        );
        let field_y = area.y.saturating_add(1);
        if field_y >= area.bottom() {
            return;
        }
        FieldWidget {
            view: &prompt.message,
            theme: self.theme,
            ascii: false,
        }
        .render(
            Rect {
                x: area.x,
                y: field_y,
                width: area.width,
                height: 1,
            },
            buf,
        );
        self.draw_options(buf, area, field_y.saturating_add(1), prompt);
    }

    /// The scope row and the line under it: the refusal when there is one,
    /// else the reassurance that a stash is undoable like every other write.
    fn draw_options(&self, buf: &mut Buffer, area: Rect, y: u16, prompt: &StashPromptView<'_>) {
        let (all, staged) = if prompt.staged_only {
            ("( )", "(•)")
        } else {
            ("(•)", "( )")
        };
        set_text(
            buf,
            area.x,
            y,
            area,
            &format!("  {all} all incl. untracked   {staged} staged only   (tab)"),
            foreground(self.theme, Token::Text),
        );
        let (line, style) = match &prompt.refusal {
            Some(refusal) => (
                format!("  {}", refusal.as_str()),
                foreground(self.theme, Token::Warning),
            ),
            None => (
                "  Mergewright journals it, so u takes the stash back.".to_owned(),
                foreground(self.theme, Token::TextMuted),
            ),
        };
        set_text(buf, area.x, y.saturating_add(1), area, &line, style);
    }

    /// The list, and the selected stash's files under it once `Space` has
    /// asked for them.
    fn draw_list(&self, buf: &mut Buffer, body: Rect) {
        // The list keeps at most half the body once a preview is open, so
        // the files of the selected stash are never one row of context.
        let listed = match self.view.preview {
            Some(_) => self
                .view
                .rows
                .len()
                .min(usize::from(body.height).saturating_div(2).max(1)),
            None => self.view.rows.len().min(usize::from(body.height)),
        };
        for index in 0..listed {
            let y = body
                .y
                .saturating_add(u16::try_from(index).unwrap_or(u16::MAX));
            self.draw_row(buf, body, y, index);
        }
        if let Some(preview) = self.view.preview {
            let used = u16::try_from(listed).unwrap_or(body.height);
            let rest = Rect {
                x: body.x,
                y: body.y.saturating_add(used),
                width: body.width,
                height: body.height.saturating_sub(used),
            };
            self.draw_preview(buf, rest, preview);
        }
    }
}

impl Widget for StashScreen<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let muted = foreground(self.theme, Token::TextMuted);
        set_text(
            buf,
            area.x,
            area.y,
            area,
            &self.header(),
            foreground(self.theme, Token::Text).add_modifier(Modifier::BOLD),
        );
        if area.height == 1 {
            return;
        }
        let footer_y = area.bottom().saturating_sub(1);
        set_text(buf, area.x, footer_y, area, &self.footer(area.width), muted);
        if area.height == 2 {
            return;
        }
        let mut body = Rect {
            x: area.x,
            y: area.y.saturating_add(1),
            width: area.width,
            height: area.height.saturating_sub(2),
        };
        if let Some(notice) = &self.view.notice {
            let notice_y = footer_y.saturating_sub(1);
            let style = if self.view.notice_is_failure {
                foreground(self.theme, Token::Warning)
            } else {
                muted
            };
            let words = marked_if(self.view.notice_failed, notice.as_str(), self.view.ascii);
            set_text(buf, area.x, notice_y, area, &words, style);
            crate::failure::ink_marks(buf, area, self.theme, self.view.ascii);
            body.height = body.height.saturating_sub(1);
        }
        if let Some(prompt) = &self.view.prompt {
            self.draw_prompt(buf, body, prompt);
            return;
        }
        match &self.view.status {
            StashStatus::Ready => {}
            StashStatus::Loading => return centred(buf, body, "reading the stash list…", muted),
            StashStatus::Empty => return centred(buf, body, &empty(self.keys), muted),
            StashStatus::Failed(why) => {
                let why = format!("the stash list could not be read: {}", why.as_str());
                let words = crate::failure::marked(&why, self.view.ascii);
                centred(buf, body, &words, foreground(self.theme, Token::Warning));
                return crate::failure::ink_marks(buf, body, self.theme, self.view.ascii);
            }
        }
        self.draw_list(buf, body);
    }
}

#[cfg(test)]
pub(crate) mod tests;
