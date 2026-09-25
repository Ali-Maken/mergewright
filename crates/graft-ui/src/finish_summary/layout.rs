//! Where everything in the finish summary stands: its list lines, the
//! layout, what a click lands on and how far the list scrolls. Pure
//! arithmetic over the view and the area; the widget draws with it and the
//! app's keys and mouse ask it (`graft::update::finish`).

use super::{
    DIFF_ROWS_MAX, DiffView, ExpandedDiff, FINISH_MIN, FinishView, LineKindView, MESSAGE_MIN_ROWS,
    Rect, SummaryState, TWO_COLUMN_MIN,
};

/// One line of the scrolled file list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListLine {
    /// `RESOLVED BY YOU · 2`, `COMBINED BY GIT ON ITS OWN · 1 · …`,
    /// `CHANGED BY THIS MERGE · …`.
    Section(Section),
    /// A file row, by its cursor index.
    Row(usize),
    /// Line `nth` of the diff expanded under row `row`.
    Diff {
        /// The row it hangs under.
        row: usize,
        /// Which of its lines.
        nth: usize,
    },
    /// `… N more lines · ⏎ hides`, under row `row`'s diff.
    More {
        /// The row it hangs under.
        row: usize,
        /// Lines not drawn.
        hidden: usize,
    },
    /// The whole list's one line while the summary has no lists: loading,
    /// or unavailable.
    Status,
}

/// Which section heading a [`ListLine::Section`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    /// `RESOLVED BY YOU · n`.
    ByYou,
    /// `COMBINED BY GIT ON ITS OWN · n · ⏎ shows the change`, or `· none`.
    ByGit,
    /// `CHANGED BY THIS MERGE · n · which were conflicts is not recorded`.
    Unrecorded,
}

/// Where each part of the summary goes in an area.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FinishLayout {
    /// The rounded frame: the whole area. Title in its top border, keys in
    /// its bottom one.
    pub frame: Rect,
    /// The two preview lines, first inside the frame.
    pub preview: Rect,
    /// The scrolled file list.
    pub list: Rect,
    /// `MESSAGE · ⇥ edit`.
    pub message: Rect,
    /// The subject field's row.
    pub subject: Rect,
    /// The body area, its label row first.
    pub body: Rect,
    /// The notice row, the frame's last inside row, when there is a notice.
    pub notice: Option<Rect>,
}

/// Whether `area` is big enough for the summary ([`FINISH_MIN`]).
#[must_use]
pub fn fits(area: Rect) -> bool {
    area.width >= FINISH_MIN.0 && area.height >= FINISH_MIN.1
}

/// Rows an expanded diff draws, and how many of its lines it leaves out.
fn diff_extent(diff: &ExpandedDiff) -> (usize, usize) {
    let DiffView {
        hunks,
        binary,
        truncated,
        ..
    } = match diff {
        ExpandedDiff::Loading | ExpandedDiff::Failed(_) => return (1, 0),
        ExpandedDiff::Ready(view) => view.as_ref(),
    };
    if binary.is_some() || hunks.is_empty() {
        return (1, 0);
    }
    // The rows `diff_body` draws: one per hunk heading, then each hunk's
    // own lines (a heading-kind line inside `lines` is drawn as the heading,
    // not twice — `detail_pane::unified::flatten`).
    let rows: usize = hunks
        .iter()
        .map(|hunk| {
            1 + hunk
                .lines
                .iter()
                .filter(|line| line.kind != LineKindView::HunkHeading)
                .count()
        })
        .sum();
    // Lines the job itself left out (`(shown, total)`) are hidden too.
    let cut = truncated.map_or(0, |(shown, total)| total.saturating_sub(shown));
    let drawn = rows.min(DIFF_ROWS_MAX);
    (drawn, rows - drawn + cut)
}

/// The file list, line by line, from the top: section headings, rows, and
/// the diff under the expanded row.
#[must_use]
pub fn list_lines(view: &FinishView<'_>) -> Vec<ListLine> {
    if !matches!(view.state, SummaryState::Ready) {
        return vec![ListLine::Status];
    }
    let mut lines = Vec::new();
    let push_row = |lines: &mut Vec<ListLine>, row: usize| {
        lines.push(ListLine::Row(row));
        if let Some((expanded, diff)) = &view.expanded
            && *expanded == row
        {
            let (drawn, hidden) = diff_extent(diff);
            lines.extend((0..drawn).map(|nth| ListLine::Diff { row, nth }));
            if hidden > 0 {
                lines.push(ListLine::More { row, hidden });
            }
        }
    };
    if view.recorded {
        lines.push(ListLine::Section(Section::ByYou));
        for row in 0..view.by_you.len() {
            push_row(&mut lines, row);
        }
        lines.push(ListLine::Section(Section::ByGit));
    } else {
        lines.push(ListLine::Section(Section::Unrecorded));
    }
    for row in view.by_you.len()..view.row_count() {
        push_row(&mut lines, row);
    }
    lines
}

/// Where each part goes in `area`, or `None` below [`FINISH_MIN`].
///
/// The frame is the whole area; inside it, one cell of gutter either side,
/// the two preview lines come first. Then, one column below
/// [`TWO_COLUMN_MIN`]: the list (as tall as its lines, leaving the message
/// its [`MESSAGE_MIN_ROWS`]), then the message right under it, its body
/// taking what is left. From 160: the list on the left 60 %, the message on
/// the right, both from the row under the preview lines. The notice, when
/// there is one, takes the frame's last inside row.
#[must_use]
pub fn layout(view: &FinishView<'_>, area: Rect) -> Option<FinishLayout> {
    if !fits(area) {
        return None;
    }
    let row = |y: u16, x: u16, width: u16, height: u16| Rect {
        x,
        y,
        width,
        height,
    };
    let frame = area;
    // Inside the border, one cell of gutter either side: no text touches a
    // border glyph (`frame_box::content_row`'s rule).
    let inside = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width - 4,
        height: area.height - 2,
    };
    let preview = row(inside.y, inside.x, inside.width, 2);
    let notice = view
        .notice
        .as_ref()
        .map(|_| row(inside.bottom() - 1, inside.x, inside.width, 1));
    let top = inside.y + 2;
    let bottom = inside.bottom() - u16::from(notice.is_some());
    let rows = bottom - top;
    if area.width >= TWO_COLUMN_MIN {
        let left = inside.width * 3 / 5;
        let right_x = inside.x + left + 2;
        let right = inside.width - left - 2;
        return Some(FinishLayout {
            frame,
            preview,
            list: row(top, inside.x, left, rows),
            message: row(top, right_x, right, 1),
            subject: row(top + 1, right_x, right, 1),
            body: row(top + 2, right_x, right, rows - 2),
            notice,
        });
    }
    let lines = u16::try_from(list_lines(view).len()).unwrap_or(u16::MAX);
    let list_rows = lines.min(rows - MESSAGE_MIN_ROWS);
    let message_y = top + list_rows;
    Some(FinishLayout {
        frame,
        preview,
        list: row(top, inside.x, inside.width, list_rows),
        message: row(message_y, inside.x, inside.width, 1),
        subject: row(message_y + 1, inside.x, inside.width, 1),
        body: row(
            message_y + 2,
            inside.x,
            inside.width,
            bottom - message_y - 2,
        ),
        notice,
    })
}

/// What a click at `(column, row)` landed on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinishHit {
    /// A file row, by its cursor index.
    Row(usize),
    /// The subject field.
    Subject,
    /// The body area (its label row included).
    Body,
}

/// What a click at `(column, row)` in `area` landed on, if anything the
/// summary answers.
#[must_use]
pub fn hit(view: &FinishView<'_>, area: Rect, column: u16, row: u16) -> Option<FinishHit> {
    let placed = layout(view, area)?;
    let inside = |rect: Rect| {
        column >= rect.x && column < rect.right() && row >= rect.y && row < rect.bottom()
    };
    if inside(placed.subject) {
        return Some(FinishHit::Subject);
    }
    if inside(placed.body) {
        return Some(FinishHit::Body);
    }
    if !inside(placed.list) {
        return None;
    }
    let at = view.top + usize::from(row - placed.list.y);
    match list_lines(view).get(at) {
        Some(ListLine::Row(index)) => Some(FinishHit::Row(*index)),
        _ => None,
    }
}

/// The list's height in `area`, or 0 below [`FINISH_MIN`].
#[must_use]
pub fn list_height(view: &FinishView<'_>, area: Rect) -> usize {
    layout(view, area).map_or(0, |placed| usize::from(placed.list.height))
}

/// The largest `top` that still fills the list: the wheel stops there.
#[must_use]
pub fn max_top(view: &FinishView<'_>, area: Rect) -> usize {
    list_lines(view)
        .len()
        .saturating_sub(list_height(view, area))
}

/// The `top` that puts the cursor row — and the diff under it, as much as
/// fits — on screen, moving `view.top` as little as it must. The section
/// heading right above the first row of a section comes along with it.
#[must_use]
pub fn top_for_cursor(view: &FinishView<'_>, area: Rect) -> usize {
    let height = list_height(view, area);
    let lines = list_lines(view);
    let top = view.top.min(max_top(view, area));
    if height == 0 {
        return top;
    }
    let Some(at) = lines
        .iter()
        .position(|line| *line == ListLine::Row(view.cursor))
    else {
        return top;
    };
    let end = lines[at + 1..]
        .iter()
        .take_while(|line| matches!(line, ListLine::Diff { .. } | ListLine::More { .. }))
        .count()
        + at;
    let first = if at > 0 && matches!(lines[at - 1], ListLine::Section(_)) {
        at - 1
    } else {
        at
    };
    if first < top {
        return first;
    }
    if end >= top + height {
        // As much of the diff as fits, never the row itself off the top.
        return (end + 1 - height).min(at);
    }
    top
}
