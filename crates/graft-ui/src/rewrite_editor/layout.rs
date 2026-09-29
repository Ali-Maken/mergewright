//! Where each part of the rewrite editor goes: rows, message, right side, columns.

use super::{
    AfterView, ChangesView, LEFT_WIDTH, NARROW_BELOW, ROWS_MIN, Rect, RewriteEditorView, RightSide,
    SHORT_HEIGHT, SHORT_MESSAGE_LINES, WIDE_MIN,
};

/// One line in the rows' region.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RowLine {
    /// A row (index into `rows`).
    Row(usize),
    /// The field, under its row.
    Field,
}

/// Where everything goes: the one answer the draw and the hit test share.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Layout {
    /// The title's row, over the rows.
    pub(super) title: Option<Rect>,
    /// The rows' region: one line per [`RowLine`], from `top`.
    pub(super) rows: Option<Rect>,
    /// The first [`RowLine`] drawn (the rows scroll to keep the cursor in).
    pub(super) top: usize,
    /// The `│` column (wide only).
    pub(super) rule: Option<Rect>,
    /// The right side, its header on its first row.
    pub(super) right: Option<Rect>,
    /// The message block: a blank line, the label, the `┊` lines.
    pub(super) message: Option<Rect>,
    /// The area's last row.
    pub(super) notice: Rect,
}

/// The rows' lines, in order.
pub(super) fn row_lines(view: &RewriteEditorView) -> Vec<RowLine> {
    let mut lines = Vec::with_capacity(view.rows.len() + 1);
    for index in 0..view.rows.len() {
        lines.push(RowLine::Row(index));
        if view.field.as_ref().is_some_and(|field| field.row == index) {
            lines.push(RowLine::Field);
        }
    }
    lines
}

/// Rows the message block wants: a blank, the label, its lines (capped on
/// a short screen, `…` after the cap).
pub(super) fn message_height(view: &RewriteEditorView, area: Rect) -> u16 {
    let Some(message) = &view.message else {
        return 0;
    };
    let lines = if area.height <= SHORT_HEIGHT && message.lines.len() > SHORT_MESSAGE_LINES {
        SHORT_MESSAGE_LINES + 1
    } else {
        message.lines.len()
    };
    u16::try_from(lines + 2).unwrap_or(u16::MAX)
}

/// Rows the right side wants: its header, a blank, its lines.
pub(super) fn right_height(view: &RewriteEditorView) -> u16 {
    let lines = match &view.right {
        RightSide::After(AfterView::Ready { rows, facts }) => {
            rows.len() + usize::from(!facts.is_empty()) + facts.len()
        }
        RightSide::After(_) => 1,
        RightSide::Changes { files, .. } => match files {
            ChangesView::Files(files) => files.len().max(1),
            _ => 1,
        },
    };
    u16::try_from(lines + 2).unwrap_or(u16::MAX)
}

/// The first line to draw so the cursor's row (and its field) is on screen.
pub(super) fn scroll_top(lines: &[RowLine], cursor: usize, height: u16) -> usize {
    let height = usize::from(height);
    let Some(at) = lines.iter().position(|line| *line == RowLine::Row(cursor)) else {
        return 0;
    };
    let last = if lines.get(at + 1) == Some(&RowLine::Field) {
        at + 1
    } else {
        at
    };
    (last + 1).saturating_sub(height)
}

/// Rect helper: `height` rows from `y` inside `within`'s columns.
pub(super) fn band(within: Rect, y: u16, height: u16) -> Rect {
    Rect::new(within.x, y, within.width, height)
}

/// Where everything goes in `area`.
pub(super) fn layout(view: &RewriteEditorView, area: Rect) -> Layout {
    let notice = Rect::new(area.x, area.bottom().saturating_sub(1), area.width, 1);
    let body = Rect::new(area.x, area.y, area.width, area.height.saturating_sub(1));
    let lines = row_lines(view);
    let natural = u16::try_from(lines.len()).unwrap_or(u16::MAX);
    let empty = Layout {
        title: None,
        rows: None,
        top: 0,
        rule: None,
        right: None,
        message: None,
        notice,
    };
    if body.height < 3 {
        return empty;
    }
    let wide = area.width >= WIDE_MIN;
    let narrow = area.width < NARROW_BELOW;

    if narrow && view.narrow_shows_right {
        return Layout {
            right: Some(inset(body)),
            ..empty
        };
    }

    let (left, rule, right_wide) = columns(body, wide);
    let below = left.y + 2;
    let room = left.bottom().saturating_sub(below);
    let right_need = if wide || narrow {
        0
    } else {
        right_height(view) + 1
    };
    let message_need = message_height(view, area);
    let under = right_need + message_need;
    let rows = band(left, below, rows_height(natural, room, under));
    let mut y = rows.bottom();

    let right = if right_wide.is_some() {
        right_wide
    } else if narrow {
        None
    } else {
        // 120–179: a blank line, then the right side under the rows.
        let height = right_need.min(left.bottom().saturating_sub(y));
        let right = inset(band(left, y + 1, height.saturating_sub(1)));
        y += height;
        (right.height > 0).then_some(right)
    };
    let message_height = message_need.min(left.bottom().saturating_sub(y));
    let message = (message_height > 0).then(|| band(left, y, message_height));

    Layout {
        title: Some(band(left, left.y, 1)),
        rows: Some(rows),
        top: scroll_top(&lines, view.cursor, rows.height),
        rule,
        right,
        message,
        notice,
    }
}

/// The rows' height: all of them when what goes under them (`below`: the
/// AFTER side and the message) still fits in `room`; never fewer than
/// `ROWS_MIN` (or all, if fewer); never more than `room`.
pub(super) fn rows_height(natural: u16, room: u16, below: u16) -> u16 {
    natural
        .min(room.saturating_sub(below))
        .max(natural.min(ROWS_MIN))
        .min(room)
}

/// The left column (the whole width, or the rows' width left of `│`), and
/// when `wide` the rule and the right side beyond it.
pub(super) fn columns(body: Rect, wide: bool) -> (Rect, Option<Rect>, Option<Rect>) {
    if !wide {
        return (body, None, None);
    }
    let left = Rect::new(body.x, body.y, LEFT_WIDTH.min(body.width), body.height);
    let rule = Rect::new(body.x + LEFT_WIDTH, body.y, 1, body.height);
    let right = Rect::new(
        body.x + LEFT_WIDTH + 2,
        body.y,
        body.width.saturating_sub(LEFT_WIDTH + 2),
        body.height,
    );
    (left, Some(rule), Some(right))
}

/// `area` one column in from the left: the AFTER side, when it is not right
/// of the rule, starts where the `MESSAGE` label does.
pub(super) fn inset(area: Rect) -> Rect {
    Rect::new(
        area.x.saturating_add(1).min(area.right()),
        area.y,
        area.width.saturating_sub(1),
        area.height,
    )
}

/// The row (index into `view.rows`) drawn at `(column, row)`, as
/// [`draw_rewrite_editor`] laid it out in `area`; `None` off the rows (the
/// field's line included).
#[must_use]
pub fn rewrite_row_at(
    area: Rect,
    view: &RewriteEditorView,
    column: u16,
    row: u16,
) -> Option<usize> {
    let at = layout(view, area);
    let rows = at.rows?;
    if column < rows.x || column >= rows.right() || row < rows.y || row >= rows.bottom() {
        return None;
    }
    match row_lines(view).get(at.top + usize::from(row - rows.y))? {
        RowLine::Row(index) => Some(*index),
        RowLine::Field => None,
    }
}
