//! The pane's files region: one row per [`FileRow`], cursor highlighted.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::detail::{DetailFocus, DetailView, FileRow, FileRowKind, FileStatusView, SubmoduleView};
use crate::paint::{color, foreground};
use crate::theme::Theme;
use crate::theme::Token;

use super::line_render::draw_line;

pub(crate) fn status_letter(status: &FileStatusView) -> char {
    match status {
        FileStatusView::Added => 'A',
        FileStatusView::Deleted => 'D',
        FileStatusView::Modified => 'M',
        FileStatusView::Renamed => 'R',
        FileStatusView::Copied => 'C',
        FileStatusView::TypeChanged => 'T',
        // Never drawn: the CONFLICTS row's `file_row_text` branches on
        // `Unmerged` before reaching here and draws `how` instead of a
        // letter.
        FileStatusView::Unmerged { .. } => 'U',
        FileStatusView::Untracked => '?',
        // Never drawn either: `file_row_text` draws `▣` and the facts.
        FileStatusView::Submodule(_) => '\u{25a3}',
    }
}

/// A submodule's row: `▣ <path>  submodule · <facts>` (m7-power-tools/13).
/// When the words do not fit `width`, the pointer's ids go first, so every
/// fact still shows; past that the line is cut like any other.
fn submodule_row_text(indent: &str, name: &str, view: &SubmoduleView, width: usize) -> String {
    let row = |ids: bool| format!("{indent}\u{25a3} {name}  {}", view.row_words(ids));
    let full = row(true);
    if full.chars().count() <= width {
        full
    } else {
        row(false)
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
            // An unmerged file has no add/remove counts to show — there is
            // no single "the diff" while both sides are still in the index
            // — so its row is the name, and `how` (`both modified`, …)
            // takes the trailing column instead of a letter and counts.
            if let FileStatusView::Submodule(view) = status {
                (
                    submodule_row_text(&indent, row.name.as_str(), view, width),
                    String::new(),
                )
            } else if let FileStatusView::Unmerged { how } = status {
                (format!("{indent}{}", row.name.as_str()), how.to_string())
            } else {
                let letter = status_letter(status);
                let counts = match (added, removed) {
                    (Some(added), Some(removed)) => format!("+{added} \u{2212}{removed}"),
                    // No counts: an untracked file has no diff against the
                    // index to count (`new`); anything else is binary.
                    _ if matches!(status, FileStatusView::Untracked) => "new".to_string(),
                    _ => "bin".to_string(),
                };
                (format!("{indent}{letter} {}", row.name.as_str()), counts)
            }
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
    let cursor_bg = color(theme.token(Token::Selection));

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

#[cfg(test)]
mod tests {
    use graft_term::sanitize_line;

    use super::*;
    use crate::detail::{FileRow, FileRowKind, WorkingTreeSection};

    /// An unmerged file's row shows the porcelain pair as words (`both
    /// modified`), never the single letter and add/remove counts an
    /// ordinary changed file draws (m5-history-and-ghost-preview/04).
    #[test]
    fn an_unmerged_row_shows_how_as_words_not_a_letter_and_counts() {
        let row = FileRow {
            depth: 1,
            name: sanitize_line("src/auth.rs"),
            kind: FileRowKind::File {
                status: FileStatusView::Unmerged {
                    how: sanitize_line("both modified"),
                },
                added: None,
                removed: None,
            },
            entry: Some(0),
        };
        let text = file_row_text(&row, 40);
        assert!(text.contains("src/auth.rs"), "{text:?}");
        assert!(text.contains("both modified"), "{text:?}");
        assert!(!text.starts_with(" U "), "{text:?}");
    }

    /// `CONFLICTS (n)` is the section heading word for
    /// [`WorkingTreeSection::Unmerged`], drawn first in the changes pane by
    /// the caller's row order — this widget draws whatever rows it is
    /// given, in that order.
    #[test]
    fn the_conflicts_heading_word_and_row_order() {
        let heading = FileRow {
            depth: 0,
            name: sanitize_line(WorkingTreeSection::Unmerged.title()),
            kind: FileRowKind::Section {
                section: WorkingTreeSection::Unmerged,
                count: 2,
            },
            entry: None,
        };
        let text = file_row_text(&heading, 40);
        assert!(text.starts_with("CONFLICTS (2)"), "{text:?}");
    }

    fn counts_row(status: FileStatusView, added: Option<u32>) -> FileRow {
        FileRow {
            depth: 1,
            name: sanitize_line("new.txt"),
            kind: FileRowKind::File {
                status,
                added,
                removed: added.map(|_| 0),
            },
            entry: Some(0),
        }
    }

    /// An untracked file has no counts because it has no diff against the
    /// index, not because it is binary: its row says `new`
    /// (m6-conflicts/70, n1).
    #[test]
    fn an_untracked_file_reads_new_not_bin() {
        let text = file_row_text(&counts_row(FileStatusView::Untracked, None), 40);
        assert!(text.ends_with(" new"), "{text:?}");
        assert!(!text.contains("bin"), "{text:?}");
    }

    /// Any other row with absent counts is a binary file, and still reads
    /// `bin`; a row with counts still reads them (m6-conflicts/70).
    #[test]
    fn a_binary_file_still_reads_bin() {
        let binary = file_row_text(&counts_row(FileStatusView::Modified, None), 40);
        assert!(binary.ends_with(" bin"), "{binary:?}");
        let counted = file_row_text(&counts_row(FileStatusView::Added, Some(1)), 40);
        assert!(counted.ends_with("+1 \u{2212}0"), "{counted:?}");
    }
}
