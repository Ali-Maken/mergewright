//! What is under the diff cursor, as a place in a file: the one answer blame's
//! `B`, history's `⏎` and the trail read (m7-power-tools/20b).
//!
//! The cursor itself is a row of the flattened diff body
//! ([`graft_ui::detail::DiffView::body_row`]); this module turns a row into a
//! line of one side of the file ([`LineRef`]) and back.

use std::path::PathBuf;

use graft_git::diff::FileStatus;
use graft_ui::detail::{
    BodyRow, DetailFocus, DiffMode, DiffView, HunkView, LineKindView, WorkingTreeSection,
};

use crate::update::state::AppState;

use super::state::{DetailSpec, Loading};

/// Which side of a diff a line number counts in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    /// The file before the change.
    Old,
    /// The file after it.
    New,
}

/// One line of one side of a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LineRef {
    /// The side `line` counts in.
    pub side: Side,
    /// 1-based, in that side's file.
    pub line: u32,
}

/// What the diff cursor is on, for either pane.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffCursor {
    /// What the pane is about.
    pub spec: DetailSpec,
    /// The changes-pane group; `Some` only on the changes pane.
    pub section: Option<WorkingTreeSection>,
    /// The file's new-side path.
    pub path: PathBuf,
    /// Where it came from, when renamed (`FileEntry::old_path`).
    pub old_path: Option<PathBuf>,
    /// No text diff: the diff says binary once it has arrived; before that,
    /// the file list's counts are missing on a file that is not untracked.
    pub binary: bool,
    /// The line under the cursor; `None` while the diff has not arrived, or
    /// on a row that names no line.
    pub at: Option<LineRef>,
}

/// What is under the cursor of the focused detail pane: the file row's first
/// changed line with the file list focused, the cursor row's line with the
/// diff focused. `None` while the graph has the keys, the pane is hidden, or
/// the files cursor is on no file (a section or directory row).
#[must_use]
pub fn diff_cursor(state: &AppState) -> Option<DiffCursor> {
    let detail = &state.detail;
    if !detail.shown || detail.focus == DetailFocus::Graph {
        return None;
    }
    let spec = detail.spec?;
    let entry = detail.selected_entry()?;
    let file = detail
        .list
        .as_ref()?
        .files
        .get(usize::try_from(entry).ok()?)?;
    let view = match &detail.diff {
        Loading::Ready(view) => Some(view.as_ref()),
        Loading::Idle | Loading::Pending(_) | Loading::Failed(_) => None,
    };
    let at = view.and_then(|view| match detail.focus {
        DetailFocus::Diff => row_line(view, detail.body_cursor()?, detail.mode),
        DetailFocus::Files | DetailFocus::Graph => view.hunks.iter().find_map(first_change),
    });
    let binary = view.map_or(
        file.added.is_none() && file.removed.is_none() && file.status != FileStatus::Untracked,
        |view| view.binary.is_some(),
    );
    Some(DiffCursor {
        spec,
        section: detail.selected_section(),
        path: bytes_to_pathbuf(&file.path),
        old_path: file.old_path.as_deref().map(bytes_to_pathbuf),
        binary,
        at,
    })
}

/// The line body row `row` names: an added or context line its new number, a
/// removed line its old one, a heading its hunk's first changed line. Side
/// by side, a removed line paired with an added one shares a row with it,
/// and that row has a new side.
pub(super) fn row_line(view: &DiffView, row: usize, mode: DiffMode) -> Option<LineRef> {
    match view.body_row(row)? {
        BodyRow::Heading { hunk } => first_change(view.hunks.get(hunk)?),
        BodyRow::Line { hunk, line } => {
            let hunk = view.hunks.get(hunk)?;
            let at = hunk.lines.get(line)?;
            match at.kind {
                LineKindView::Removed => {
                    let paired_new = (mode == DiffMode::SideBySide)
                        .then(|| {
                            let pair = usize::try_from(at.pair?).ok()?;
                            hunk.lines.get(pair)?.new_no
                        })
                        .flatten();
                    match paired_new {
                        Some(line) => Some(new(line)),
                        None => at.old_no.map(old),
                    }
                }
                LineKindView::Added | LineKindView::Context => at.new_no.map(new),
                LineKindView::HunkHeading => first_change(hunk),
            }
        }
        BodyRow::Truncation => None,
    }
}

/// The body row showing `at`, or the nearest row before it on that side
/// when the line itself is not shown; the first row when nothing on that
/// side comes before it. `None` for a diff with no rows.
pub(super) fn line_row(view: &DiffView, at: LineRef) -> Option<usize> {
    let mut best: Option<(u32, usize)> = None;
    let mut row = 0;
    for hunk in &view.hunks {
        row += 1; // the heading
        for line in hunk
            .lines
            .iter()
            .filter(|line| line.kind != LineKindView::HunkHeading)
        {
            let number = match (at.side, line.kind) {
                (Side::New, LineKindView::Added | LineKindView::Context) => line.new_no,
                (Side::Old, LineKindView::Removed | LineKindView::Context) => line.old_no,
                _ => None,
            };
            if let Some(number) = number
                && number <= at.line
                && best.is_none_or(|(seen, _)| number > seen)
            {
                best = Some((number, row));
            }
            row += 1;
        }
    }
    match best {
        Some((_, row)) => Some(row),
        None => (view.body_len() > 0).then_some(0),
    }
}

/// The body row of the heading of the hunk `row` is in.
pub(super) fn heading_row(view: &DiffView, row: usize) -> usize {
    let mut start = 0;
    for hunk in &view.hunks {
        let len = 1 + hunk
            .lines
            .iter()
            .filter(|line| line.kind != LineKindView::HunkHeading)
            .count();
        if row < start + len {
            return start;
        }
        start += len;
    }
    row
}

/// A hunk's first changed line: its first added line, else its first
/// removed one.
pub(super) fn first_change(hunk: &HunkView) -> Option<LineRef> {
    let added = hunk
        .lines
        .iter()
        .find(|line| line.kind == LineKindView::Added)
        .and_then(|line| line.new_no.map(new));
    added.or_else(|| {
        hunk.lines
            .iter()
            .find(|line| line.kind == LineKindView::Removed)
            .and_then(|line| line.old_no.map(old))
    })
}

fn new(line: u32) -> LineRef {
    LineRef {
        side: Side::New,
        line,
    }
}

fn old(line: u32) -> LineRef {
    LineRef {
        side: Side::Old,
        line,
    }
}

#[cfg(unix)]
fn bytes_to_pathbuf(bytes: &[u8]) -> PathBuf {
    use std::os::unix::ffi::OsStrExt;
    PathBuf::from(std::ffi::OsStr::from_bytes(bytes))
}

#[cfg(not(unix))]
fn bytes_to_pathbuf(bytes: &[u8]) -> PathBuf {
    PathBuf::from(String::from_utf8_lossy(bytes).into_owned())
}

#[cfg(test)]
pub(super) mod tests;
