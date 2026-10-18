//! Where everything in the review stands: its list lines, the layout, what
//! a click lands on and how far the list scrolls. Pure arithmetic over the
//! view and the area; the widget draws with it and the app's keys and mouse
//! ask it (`mergewright::update::finish`).

use super::{FINISH_MIN, Rect, ReviewView, SummaryState};

/// One line of the scrolled list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListLine {
    /// A row, by its cursor index.
    Row(usize),
    /// The list's one line while the review has no rows: loading, or
    /// unavailable.
    Status,
}

/// Where each part of the review goes in an area.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FinishLayout {
    /// The rounded frame: the whole area. Keys in its bottom border.
    pub frame: Rect,
    /// The title, a blank row, `This merge will put into …:` and the count
    /// line: the first four rows inside the frame.
    pub head: Rect,
    /// The scrolled list of rows.
    pub list: Rect,
    /// `Message  <subject>`, a blank row under the list.
    pub message: Rect,
    /// The notice row, the frame's last inside row, when there is a notice.
    pub notice: Option<Rect>,
}

/// Rows the head takes: title, blank, the `into` line, the count line.
const HEAD_ROWS: u16 = 4;

/// Whether `area` is big enough for the review ([`FINISH_MIN`]).
#[must_use]
pub fn fits(area: Rect) -> bool {
    area.width >= FINISH_MIN.0 && area.height >= FINISH_MIN.1
}

/// The list, line by line, from the top: one per row.
#[must_use]
pub fn list_lines(view: &ReviewView<'_>) -> Vec<ListLine> {
    if !matches!(view.state, SummaryState::Ready) {
        return vec![ListLine::Status];
    }
    (0..view.row_count()).map(ListLine::Row).collect()
}

/// Where each part goes in `area`, or `None` below [`FINISH_MIN`].
///
/// The frame is the whole area; inside it, one cell of gutter either side.
/// The head's four rows come first, then the list (as tall as its lines,
/// up to what is left), then a blank row and the message. The notice, when
/// there is one, takes the frame's last inside row.
#[must_use]
pub fn layout(view: &ReviewView<'_>, area: Rect) -> Option<FinishLayout> {
    if !fits(area) {
        return None;
    }
    let row = |y: u16, height: u16, inside: Rect| Rect {
        x: inside.x,
        y,
        width: inside.width,
        height,
    };
    // Inside the border, one cell of gutter either side: no text touches a
    // border glyph (`frame_box::content_row`'s rule).
    let inside = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width - 4,
        height: area.height - 2,
    };
    let notice = view
        .notice
        .as_ref()
        .map(|_| row(inside.bottom() - 1, 1, inside));
    let bottom = inside.bottom() - u16::from(notice.is_some());
    let top = inside.y + HEAD_ROWS;
    // The list as tall as its lines, leaving a blank row and the message
    // under it, as the mock has them.
    let room = bottom.saturating_sub(top).saturating_sub(2);
    let lines = u16::try_from(list_lines(view).len()).unwrap_or(u16::MAX);
    let list_rows = lines.min(room);
    let message = row(top + list_rows + 1, 1, inside);
    Some(FinishLayout {
        frame: area,
        head: row(inside.y, HEAD_ROWS, inside),
        list: row(top, list_rows, inside),
        message,
        notice,
    })
}

/// The row a click at `(column, row)` in `area` landed on, if any.
#[must_use]
pub fn hit(view: &ReviewView<'_>, area: Rect, column: u16, row: u16) -> Option<usize> {
    let placed = layout(view, area)?;
    let list = placed.list;
    if column < list.x || column >= list.right() || row < list.y || row >= list.bottom() {
        return None;
    }
    let at = view.top + usize::from(row - list.y);
    match list_lines(view).get(at) {
        Some(ListLine::Row(index)) => Some(*index),
        _ => None,
    }
}

/// The list's height in `area`, or 0 below [`FINISH_MIN`].
#[must_use]
pub fn list_height(view: &ReviewView<'_>, area: Rect) -> usize {
    layout(view, area).map_or(0, |placed| usize::from(placed.list.height))
}

/// The largest `top` that still fills the list: the wheel stops there.
#[must_use]
pub fn max_top(view: &ReviewView<'_>, area: Rect) -> usize {
    list_lines(view)
        .len()
        .saturating_sub(list_height(view, area))
}

/// The `top` that puts the cursor row on screen, moving `view.top` as little as it must.
#[must_use]
pub fn top_for_cursor(view: &ReviewView<'_>, area: Rect) -> usize {
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
    if at < top {
        return at;
    }
    if at >= top + height {
        return at + 1 - height;
    }
    top
}
