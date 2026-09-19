//! The pane's files region: one row per [`FileRow`], cursor highlighted.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::detail::{DetailFocus, DetailView, FileRow, FileRowKind, FileStatusView};
use crate::paint::{color, foreground};
use crate::theme::Theme;
use crate::theme::Token;

use super::line_render::draw_line;

pub(crate) fn status_letter(status: FileStatusView) -> char {
    match status {
        FileStatusView::Added => 'A',
        FileStatusView::Deleted => 'D',
        FileStatusView::Modified => 'M',
        FileStatusView::Renamed => 'R',
        FileStatusView::Copied => 'C',
        FileStatusView::TypeChanged => 'T',
        FileStatusView::Unmerged => 'U',
        FileStatusView::Untracked => '?',
    }
}

fn file_row_text(row: &FileRow, width: usize) -> String {
    let indent = "  ".repeat(usize::from(row.depth));
    let (left, right) = match &row.kind {
        FileRowKind::Section { count, .. } => {
            (format!("{} ({count})", row.name.as_str()), String::new())
        }
        FileRowKind::Message { lines } => {
            (format!("\u{2709} message ({lines} lines)"), String::new())
        }
        FileRowKind::Dir { collapsed } => {
            let marker = if *collapsed { "\u{25b8}" } else { "\u{25be}" };
            (
                format!("{indent}{marker} {}", row.name.as_str()),
                String::new(),
            )
        }
        FileRowKind::File {
            status,
            added,
            removed,
        } => {
            let letter = status_letter(*status);
            let counts = match (added, removed) {
                (Some(added), Some(removed)) => format!("+{added} \u{2212}{removed}"),
                _ => "bin".to_string(),
            };
            (format!("{indent}{letter} {}", row.name.as_str()), counts)
        }
    };
    if right.is_empty() {
        left
    } else {
        let gap = width
            .saturating_sub(left.chars().count() + right.chars().count())
            .max(1);
        format!("{left}{}{right}", " ".repeat(gap))
    }
}

pub(crate) fn render_files(view: &DetailView, area: Rect, theme: &Theme, buf: &mut Buffer) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let Some(files) = view.files.as_ref() else {
        return;
    };
    let text_style = if view.focus == DetailFocus::Files {
        foreground(theme, Token::Accent)
    } else {
        foreground(theme, Token::TextMuted)
    };
    let cursor_bg = color(theme.token(Token::SurfaceRaised));

    for (offset, (index, row)) in (0_u16..).zip(files.visible().skip(files.top)) {
        if offset >= area.height {
            break;
        }
        let style = if index == files.cursor {
            text_style.bg(cursor_bg)
        } else {
            text_style
        };
        let text = file_row_text(row, usize::from(area.width));
        draw_line(buf, area, offset, &text, style);
    }
}
