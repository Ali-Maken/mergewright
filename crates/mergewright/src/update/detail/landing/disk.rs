//! Disk landings on a partly staged file: which diff a line of the file on
//! disk is read on (m7-power-tools/20b, split out in 20o).

use mergewright_git::diff::{FileDiff, LineKind};
use mergewright_ui::detail::WorkingTreeSection;

use crate::update::state::AppState;

use super::super::cursor::{LineRef, Side};
use super::super::state::{DetailSpec, Loading};
use super::LineLanding;
use super::path_bytes;
use super::select::row_is;

/// On a disk landing (`WorkingTree`, no section) with the Unstaged row of a
/// partly staged path selected and its diff in: the line's number on the
/// Staged diff when it is not an unstaged line. `None` otherwise — the
/// landing places as it stands.
pub(super) fn index_line(state: &AppState, landing: &LineLanding) -> Option<LineRef> {
    let detail = &state.detail;
    if landing.spec != DetailSpec::WorkingTree
        || landing.section.is_some()
        || detail.spec != Some(DetailSpec::WorkingTree)
        || detail.selected_section() != Some(WorkingTreeSection::Unstaged)
        || !matches!(detail.diff, Loading::Ready(_))
    {
        return None;
    }
    let at = landing.at.filter(|at| at.side == Side::New)?;
    let (Loading::Ready(files), Some(list)) = (&detail.files, detail.list.as_ref()) else {
        return None;
    };
    let wanted = path_bytes(&landing.path);
    let staged_too = (0..files.rows.len()).any(|row| {
        files.section_at(row) == Some(WorkingTreeSection::Staged)
            && row_is(files, list, row, &wanted)
    });
    if !staged_too {
        return None;
    }
    match disk_line_in(detail.diff_raw.as_ref()?, at.line) {
        DiskLine::Unstaged(_) => None,
        DiskLine::Index(line) => Some(LineRef {
            side: Side::New,
            line,
        }),
    }
}

/// Where a line of the file on disk lives, read off the Unstaged diff
/// (index → disk) of a partly staged file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DiskLine {
    /// An uncommitted, unstaged line: this `New` number on the Unstaged diff.
    Unstaged(u32),
    /// Unchanged since the index: this `New` number on the Staged diff.
    Index(u32),
}

/// Disk line `disk_line` against `unstaged`, the file's index → disk diff:
/// an added line is `Unstaged` at its own number; a context line is `Index`
/// at its old number; a line outside every hunk is `Index`, shifted back by
/// what the hunks above it added or took away. A diff cut short, the line
/// below what it shows, is taken as `Unstaged` — it cannot be told — and
/// the landing puts the cursor on the nearest row, as `line_row` does.
///
/// Worked example — one hunk `@@ -8,2 +8,3 @@` adding disk line 9:
///
/// ```text
/// disk 3   above every hunk        → Index(3)
/// disk 8   context, old_no 8       → Index(8)
/// disk 9   added line              → Unstaged(9)
/// disk 20  below it; hunk adds +1  → Index(19)
/// ```
///
/// (Not a doctest: `mergewright` has no doctests and the item is crate-private;
/// the same cases run in `landing/tests/disk.rs`.)
pub(crate) fn disk_line_in(unstaged: &FileDiff, disk_line: u32) -> DiskLine {
    let mut shown_to = 0;
    for line in unstaged.hunks.iter().flat_map(|hunk| &hunk.lines) {
        let Some(new_no) = line.new_no else {
            continue;
        };
        shown_to = shown_to.max(new_no);
        if new_no == disk_line {
            return match (line.kind, line.old_no) {
                (LineKind::Context, Some(old_no)) => DiskLine::Index(old_no),
                _ => DiskLine::Unstaged(disk_line),
            };
        }
    }
    if unstaged.truncated.is_some() && disk_line > shown_to {
        return DiskLine::Unstaged(disk_line);
    }
    // The hunks wholly above the line. git writes a hunk with no new lines
    // (a pure deletion) at the line *before* it, so that line is its end.
    let shift: i64 = unstaged
        .hunks
        .iter()
        .filter(|hunk| {
            let end = if hunk.new_lines == 0 {
                hunk.new_start
            } else {
                hunk.new_start + hunk.new_lines - 1
            };
            end < disk_line
        })
        .map(|hunk| i64::from(hunk.new_lines) - i64::from(hunk.old_lines))
        .sum();
    let line = i64::from(disk_line) - shift;
    // Positive for any diff git writes: the shift counts only hunks above.
    DiskLine::Index(u32::try_from(line).unwrap_or(1))
}
