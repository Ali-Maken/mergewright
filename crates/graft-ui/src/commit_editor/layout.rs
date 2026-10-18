//! The commit screen's one layout (ease-pass/08s M5): the rectangles
//! [`CommitScreen`](super::CommitScreen) draws in and the rectangles
//! `graft::update` hit-tests a click against, from one function, so a click
//! lands on what is drawn. In [`Cells`], not the rendering library's own
//! rectangle: the app's update half must not name it.

use super::{CommitScreenView, DIFF_COLUMNS, FILES_WIDTH, FileGroup};
use crate::Cells;

/// Body rows inside the message box.
const BODY_ROWS: u16 = 3;

/// Rows the screen takes before the file list: header, the box's two
/// borders, the subject, the summary and the rule over the files.
const CHROME: u16 = 6;

/// Where each part of the commit screen is. A part with no room at this
/// size is zero-height (`diff` is `None` under [`DIFF_COLUMNS`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommitLayout {
    /// The top row: the repository, committing to / amending, push state.
    pub header: Cells,
    /// The message box, borders included.
    pub message_box: Cells,
    /// The subject's row inside the box's borders.
    pub subject: Cells,
    /// The body's rows inside the box's borders.
    pub body: Cells,
    /// The sentence above the files.
    pub summary: Cells,
    /// The rule over the files (and the diff).
    pub rule: Cells,
    /// The file list under the rule.
    pub files: Cells,
    /// The cursor file's changes beside the list, from [`DIFF_COLUMNS`] up.
    pub diff: Option<Cells>,
    /// The bar: the screen's last row.
    pub bar: Cells,
}

/// The commit screen's parts in `area`: header on top, bar at the bottom,
/// and between them the message box (the body gives way first, a row at a
/// time), the summary, the rule and `files | diff`.
#[must_use]
pub fn commit_layout(area: Cells) -> CommitLayout {
    let band = |y: u16, height: u16| Cells { y, height, ..area };
    let none = band(area.y, 0);
    let mut layout = CommitLayout {
        header: band(area.y, area.height.min(1)),
        message_box: none,
        subject: none,
        body: none,
        summary: none,
        rule: none,
        files: none,
        diff: None,
        bar: none,
    };
    if area.width == 0 || area.height < 2 {
        return layout;
    }
    let bar_row = area.y + area.height - 1;
    layout.bar = band(bar_row, 1);
    let room = area.height - 2;
    let body_rows = BODY_ROWS.min(room.saturating_sub(CHROME));
    let box_rows = 3 + body_rows;
    if room < box_rows {
        return layout;
    }
    let mut y = area.y + 1;
    layout.message_box = band(y, box_rows);
    let inside = Cells {
        x: area.x + 1,
        width: area.width.saturating_sub(2),
        ..band(y + 1, 1)
    };
    layout.subject = inside;
    layout.body = Cells {
        y: y + 2,
        height: body_rows,
        ..inside
    };
    y += box_rows;
    if y >= bar_row {
        return layout;
    }
    layout.summary = band(y, 1);
    y += 1;
    if y >= bar_row {
        return layout;
    }
    layout.rule = band(y, 1);
    // Zero rows high when the rule takes the last row: the columns still
    // stand, so the rule marks where the diff would go.
    (layout.files, layout.diff) = files_and_diff(band(y + 1, bar_row - y - 1));
    layout
}

/// `below` the rule: `files 44 | diff`, or the files alone under
/// [`DIFF_COLUMNS`].
fn files_and_diff(below: Cells) -> (Cells, Option<Cells>) {
    if below.width < DIFF_COLUMNS {
        return (below, None);
    }
    let files = Cells {
        width: FILES_WIDTH,
        ..below
    };
    let x = below.x + FILES_WIDTH + 2;
    let diff = Cells {
        x,
        width: (below.x + below.width).saturating_sub(x + 1),
        ..below
    };
    (files, Some(diff))
}

/// One line of the file list as drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ListLine {
    /// `Going in (n)` / `Staying out (n)`.
    Heading(FileGroup, usize),
    /// The line under a group with no file in it.
    Nothing(FileGroup),
    /// The file at this index of [`CommitScreenView::files`].
    File(usize),
}

/// The file list's lines in drawn order: `Going in`, its files, then
/// `Staying out`, its files; a group with none says so.
pub(super) fn list_lines(view: &CommitScreenView<'_>) -> Vec<ListLine> {
    let mut lines = Vec::new();
    for group in [FileGroup::GoingIn, FileGroup::StayingOut] {
        let members: Vec<usize> = (0..view.files.len())
            .filter(|at| view.files[*at].group == group)
            .collect();
        lines.push(ListLine::Heading(group, members.len()));
        if members.is_empty() {
            lines.push(ListLine::Nothing(group));
        }
        lines.extend(members.into_iter().map(ListLine::File));
    }
    lines
}

/// The first of `lines` drawn in `height` rows: scrolled just enough that
/// the cursor's file is on screen.
pub(super) fn list_top(lines: &[ListLine], cursor: usize, height: u16) -> usize {
    let cursor_line = lines
        .iter()
        .position(|line| *line == ListLine::File(cursor))
        .unwrap_or(0);
    cursor_line.saturating_sub(usize::from(height).saturating_sub(1))
}

/// The file drawn on screen row `row` of the list in `files`
/// ([`CommitLayout::files`]); a heading, a `nothing` line or a row past
/// the list is `None`. Follows the list's scroll.
#[must_use]
pub fn file_at(view: &CommitScreenView<'_>, files: Cells, row: u16) -> Option<usize> {
    let offset = row.checked_sub(files.y).filter(|at| *at < files.height)?;
    let lines = list_lines(view);
    let top = list_top(&lines, view.cursor, files.height);
    match lines.get(top + usize::from(offset))? {
        ListLine::File(at) => Some(*at),
        ListLine::Heading(..) | ListLine::Nothing(_) => None,
    }
}
