//! The history place's list of commits that changed the file.

#[allow(
    clippy::wildcard_imports,
    reason = "one module split in files: its parent is its prelude"
)]
use super::*;

pub(super) fn draw_list(buf: &mut Buffer, area: Rect, view: &HistoryView) {
    if view.tracing {
        return;
    }
    match &view.list {
        HistoryList::Failed(words) => {
            let text = format!(" couldn't trace: {words}");
            put(buf, area.x, area.y, area.width, &text, Style::default());
        }
        HistoryList::Rows {
            rows,
            selected,
            top,
            end,
        } => {
            let initials_only = area.width < LIST_WIDE;
            let height = usize::from(area.height);
            let mut line = 0usize;
            for (index, row) in rows.iter().enumerate().skip(*top) {
                if line >= height {
                    return;
                }
                let chosen = *selected == Some(index);
                let text = list_row(
                    row,
                    chosen,
                    initials_only,
                    usize::from(area.width),
                    view.now,
                );
                let style = if chosen && view.focus == HistoryFocus::List {
                    Style::default().add_modifier(Modifier::REVERSED)
                } else if chosen {
                    Style::default().add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                put(buf, area.x, row_y(area, line), area.width, &text, style);
                line += 1;
                if let Some(was) = row.was {
                    if line >= height {
                        return;
                    }
                    let text = format!("   \u{21b3} was {was}");
                    put(buf, area.x, row_y(area, line), area.width, &text, dim());
                    line += 1;
                }
            }
            // The end row wraps: its words are the reason the list stops,
            // and at 40 columns git's reason would otherwise be cut.
            let indent = 3;
            let room = usize::from(area.width).saturating_sub(indent).max(1);
            for words in end.map(|end| wrap(end, room)).unwrap_or_default() {
                if line >= height {
                    return;
                }
                let text = format!("{}{words}", " ".repeat(indent));
                put(buf, area.x, row_y(area, line), area.width, &text, dim());
                line += 1;
            }
        }
    }
}

pub(super) fn row_y(area: Rect, line: usize) -> u16 {
    area.y + u16::try_from(line).unwrap_or(u16::MAX)
}

/// One commit: `▸ sha7  subject  who age +a −r`, fit to `width`.
fn list_row(row: &HistoryRow, chosen: bool, initials_only: bool, width: usize, now: i64) -> String {
    const WHO: usize = 6;
    const AGE: usize = 4;
    const COUNTS: usize = 6;
    let marker = if chosen { SELECTED } else { ' ' };
    let who = if initials_only {
        initials(row.who)
    } else {
        fit(row.who, WHO)
    };
    let who_cells = if initials_only { 2 } else { WHO };
    // ` ▸ ` + sha7 + a space, then the subject; after it ` who age counts`.
    let fixed = 3 + 7 + 1 + 1 + who_cells + 1 + AGE + 1 + COUNTS;
    let subject_cells = width.saturating_sub(fixed).max(1);
    format!(
        " {marker} {sha:<7} {subject} {who} {age:>AGE$} {counts:<COUNTS$}",
        sha = row.sha7,
        subject = pad(&fit(row.subject, subject_cells), subject_cells),
        who = pad(&who, who_cells),
        age = relative_date(row.when, now),
        counts = counts(row.added, row.removed),
    )
}

/// `+a −r`, each only when non-zero.
fn counts(added: u32, removed: u32) -> String {
    let mut parts = Vec::new();
    if added > 0 {
        parts.push(format!("+{added}"));
    }
    if removed > 0 {
        parts.push(format!("\u{2212}{removed}"));
    }
    parts.join(" ")
}
