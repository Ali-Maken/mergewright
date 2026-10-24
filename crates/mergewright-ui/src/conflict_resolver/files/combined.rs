//! The column's last section in a stopped merge (ease-pass/11b, mock M6):
//! the files git combined on its own, `Combined by git (N)`, then their
//! names, muted. Information only: no row is selectable and a click on one
//! does nothing — it is not in the [`Plan`](super::Plan) `file_at` reads.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use unicode_width::UnicodeWidthStr;

use mergewright_term::SanitizedText;

use super::column::COLUMN_HEAD;
use super::{FileListView, Glyphs, Placement, cells_u16, plan, tail};
use crate::fit::fit;
use crate::paint::foreground;
use crate::theme::{Theme, Token};

/// The files git combined on its own, as the list draws them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum CombinedList {
    /// Not a merge: no section (Q12 a).
    #[default]
    NotMerge,
    /// The read is out: `Combined by git · reading…`.
    Reading,
    /// The read failed: `Combined by git · can't tell:`, the reason under it.
    Failed(SanitizedText),
    /// The names, in git's order. Empty draws no section (status rule).
    Files(Vec<SanitizedText>),
}

/// The heading's words.
const HEADING: &str = "Combined by git";

/// The section's rows, `(row, text)` from the list's origin, in a column
/// `width` x `height`: one blank row under the files (and the tail), the
/// heading, then the names packed `, `-separated, `✓` first, the last row
/// `… K more` when they do not all fit. Empty when there is no section or
/// no row left for its heading.
pub(super) fn combined_rows(
    view: &FileListView,
    width: u16,
    height: u16,
    g: Glyphs,
) -> Vec<(u16, String)> {
    let full = usize::from(width);
    let heading = match &view.combined {
        CombinedList::NotMerge => return Vec::new(),
        CombinedList::Files(names) if names.is_empty() => return Vec::new(),
        CombinedList::Reading => format!("{HEADING} {} reading{}", g.dot, g.more),
        CombinedList::Failed(_) => format!("{HEADING} {} can't tell:", g.dot),
        CombinedList::Files(names) => format!("{HEADING} ({})", names.len()),
    };
    let placement = Placement::Column { width };
    let below_files = plan(view, placement, width, height, g)
        .cells
        .iter()
        .map(|cell| cell.y.saturating_add(1))
        .max()
        .unwrap_or(COLUMN_HEAD);
    let below = tail::tail_row(view, placement, width, height, g)
        .map_or(below_files, |(y, _)| y.saturating_add(1));
    let top = below.saturating_add(1);
    if top >= height {
        return Vec::new();
    }
    // The mark leads the reason, in the two cells its indent takes. With no
    // row for the reason, the heading carries the mark instead (its words
    // are cut by `fit`, never the mark).
    let reason_row = matches!(view.combined, CombinedList::Failed(_)) && top + 1 < height;
    let heading = if matches!(view.combined, CombinedList::Failed(_)) && !reason_row {
        crate::failure::marked(&heading, g.ascii)
    } else {
        heading
    };
    let mut rows = vec![(top, fit(&heading, full))];
    // The reason on its own row: beside the heading a 30-cell column
    // would cut it to nothing.
    if let (CombinedList::Failed(why), true) = (&view.combined, top + 1 < height) {
        crate::failure::check_unmarked(why.as_str());
        let lead = crate::failure::mark(g.ascii);
        let pad = " ".repeat(2usize.saturating_sub(lead.width()).max(1));
        rows.push((top + 1, fit(&format!("{lead}{pad}{}", why.as_str()), full)));
    }
    if let CombinedList::Files(names) = &view.combined {
        let room = usize::from(height - top - 1);
        let lines = names_rows(names, full, room, g);
        rows.extend(
            lines
                .into_iter()
                .enumerate()
                .map(|(i, line)| (top + 1 + cells_u16(i), line)),
        );
    }
    rows
}

/// `names` packed into at most `room` rows of `full` cells: `✓ a, b,` then
/// `  c, d`; when they do not all fit, the last row is `  … K more`.
fn names_rows(names: &[SanitizedText], full: usize, room: usize, g: Glyphs) -> Vec<String> {
    // Each packed row and how many names it holds.
    let mut packed: Vec<(String, usize)> = Vec::new();
    for (i, name) in names.iter().enumerate() {
        let token = if i + 1 < names.len() {
            format!("{},", name.as_str())
        } else {
            name.as_str().to_owned()
        };
        match packed.last_mut() {
            Some((line, count)) if line.width() + 1 + token.width() <= full => {
                line.push(' ');
                line.push_str(&token);
                *count += 1;
            }
            _ => {
                let lead = if packed.is_empty() {
                    format!("{} ", g.check)
                } else {
                    "  ".to_owned()
                };
                let first = fit(&format!("{lead}{token}"), full);
                packed.push((first.trim_end().to_owned(), 1));
            }
        }
    }
    if packed.len() <= room {
        return packed.into_iter().map(|(line, _)| line).collect();
    }
    if room == 0 {
        return Vec::new();
    }
    let kept: Vec<(String, usize)> = packed.into_iter().take(room - 1).collect();
    let shown: usize = kept.iter().map(|(_, count)| count).sum();
    let mut out: Vec<String> = kept.into_iter().map(|(line, _)| line).collect();
    out.push(fit(
        &format!("  {} {} more", g.more, names.len() - shown),
        full,
    ));
    out
}

/// Draws the section into the list's `area`, muted, a column only (the
/// strip below `INLINE_FROM` keeps its one to three rows for files).
/// Colour unverified by capture.
pub(super) fn draw_combined(
    view: &FileListView,
    placement: Placement,
    area: Rect,
    buf: &mut Buffer,
    theme: &Theme,
    g: Glyphs,
) {
    let Placement::Column { .. } = placement else {
        return;
    };
    for (y, text) in combined_rows(view, area.width, area.height, g) {
        super::set_text(
            buf,
            area.x,
            area.y.saturating_add(y),
            area,
            &text,
            foreground(theme, Token::TextMuted),
        );
    }
    crate::failure::ink_marks(buf, area, theme, g.ascii);
}
