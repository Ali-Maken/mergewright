//! Drawing one row's text columns (ease-pass G5–G8): the branch column,
//! the message with its fold tail, tag and labels, and who and age.
//! Where each piece lands is [`MessageLayout`]'s; this only paints.

use mergewright_graph::LaneRole;
use ratatui::buffer::Buffer;
use ratatui::style::{Modifier, Style};

use super::render::draw_cell;
use super::row_text::{MessageLayout, branch_words};
use super::{BadgeKind, Columns, GraphEntry, GraphScreen};
use crate::graph_column::role_token;
use crate::theme::Token;

impl GraphScreen<'_> {
    /// The branch column: the name in its role's colour, bold unless the
    /// branch is [`LaneRole::Other`]; dim inside an opened merge. Blank
    /// cells carry the row's own emphasis.
    pub(super) fn draw_branch(
        &self,
        entry: &GraphEntry<'_>,
        y: u16,
        columns: &Columns,
        style: Style,
        buf: &mut Buffer,
    ) {
        let Some(cell) = entry.branch.as_ref() else {
            draw_cell(buf, columns.branch_x, y, columns.branch_width, "", style);
            return;
        };
        let mut ink = style.patch(self.ink(entry, self.theme.token(role_token(cell.role))));
        if cell.role != LaneRole::Other && !cell.dim {
            ink = ink.add_modifier(Modifier::BOLD);
        }
        if cell.dim {
            ink = ink.add_modifier(Modifier::DIM);
        }
        draw_cell(
            buf,
            columns.branch_x,
            y,
            columns.branch_width,
            &branch_words(cell, columns.branch_width),
            ink,
        );
    }

    /// The message column: the subject (dim on another branch's row), a
    /// folding merge's tail, the row tag in the accent, and the labels,
    /// muted — the compare base's in the accent, as before.
    pub(super) fn draw_message(
        &self,
        entry: &GraphEntry<'_>,
        y: u16,
        columns: &Columns,
        style: Style,
        buf: &mut Buffer,
    ) {
        let layout = MessageLayout::of(
            entry,
            (self.view.icons, self.view.keys),
            self.view.ascii,
            columns,
        );
        // The whole column first, so every gap carries the row's emphasis.
        draw_cell(buf, columns.subject_x, y, columns.subject_width, "", style);
        let subject_style = if entry.dim_message {
            style.add_modifier(Modifier::DIM)
        } else {
            style
        };
        draw_cell(
            buf,
            layout.subject.start,
            y,
            layout.subject.end - layout.subject.start,
            entry.subject.as_str(),
            subject_style,
        );
        let muted = style.patch(self.ink(entry, self.theme.token(Token::TextMuted)));
        let accent = style.patch(self.ink(entry, self.theme.token(Token::Accent)));
        if let Some((x, mark)) = &layout.fold {
            draw_cell(buf, *x, y, cells(mark), mark, muted);
        }
        if let Some((x, tag)) = &layout.tag {
            draw_cell(buf, *x, y, cells(tag), tag, accent);
        }
        for label in &layout.labels {
            let ink = match entry.badges[label.badge].kind {
                BadgeKind::CompareBase => accent,
                _ => muted,
            };
            draw_cell(
                buf,
                label.cells.start,
                y,
                label.cells.end - label.cells.start,
                &label.words,
                ink,
            );
        }
    }

    /// Who and age, right-aligned as the mock draws them: the first name,
    /// `you` bold on the user's own commits.
    pub(super) fn draw_who_age(
        &self,
        entry: &GraphEntry<'_>,
        y: u16,
        columns: &Columns,
        trailing: Style,
        buf: &mut Buffer,
    ) {
        let (who, who_style) = match &entry.who {
            Some(cell) if cell.own => (
                cell.name.as_str(),
                trailing
                    .patch(self.ink(entry, self.theme.token(Token::Text)))
                    .add_modifier(Modifier::BOLD),
            ),
            Some(cell) => (cell.name.as_str(), trailing),
            None => ("", trailing),
        };
        draw_right(buf, columns.who_x, y, columns.who_width, who, who_style);
        draw_right(
            buf,
            columns.when_x,
            y,
            columns.when_width,
            &entry.when,
            trailing,
        );
    }
}

/// `text` right-aligned in `width` cells at `(x, y)`; cut with `…` when it
/// is wider.
fn draw_right(buf: &mut Buffer, x: u16, y: u16, width: u16, text: &str, style: Style) {
    let pad = width.saturating_sub(cells(text));
    draw_cell(buf, x, y, pad, "", style);
    draw_cell(buf, x + pad, y, width - pad, text, style);
}

fn cells(text: &str) -> u16 {
    use unicode_width::UnicodeWidthStr;
    u16::try_from(text.width()).unwrap_or(u16::MAX)
}
