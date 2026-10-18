//! A blame as the picture draws it: one [`BlameRowView`] per line, blocks
//! bracketed, each row's mark by the shallow ruling's precedence.

use mergewright_git::blame::{Blame, BlameLine, LineMark};
use mergewright_term::sanitize::sanitize_line;
use mergewright_ui::blame::{BlameRowView, BlockPos, RowMark, Who};

use super::short;

/// Every line of `blame`, in file order.
pub(super) fn blame_rows(blame: &Blame) -> Vec<BlameRowView> {
    let lines = &blame.lines;
    lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            let same = |other: Option<&BlameLine>| other.is_some_and(|o| o.commit == line.commit);
            let before = index.checked_sub(1).and_then(|at| lines.get(at));
            let block = match (same(before), same(lines.get(index + 1))) {
                (false, false) => BlockPos::Single,
                (false, true) => BlockPos::First,
                (true, true) => BlockPos::Middle,
                (true, false) => BlockPos::Last,
            };
            let commit = line.commit.and_then(|id| blame.commits.get(&id));
            let who = commit.map(|commit| Who {
                sha7: short(commit.id),
                name: sanitize_line(&commit.author).as_str().to_owned(),
                author_time: commit.author_time,
                subject: sanitize_line(&commit.summary).as_str().to_owned(),
            });
            BlameRowView {
                line_no: line.final_line,
                text: sanitize_line(&String::from_utf8_lossy(&line.text))
                    .as_str()
                    .to_owned(),
                who,
                block,
                mark: mark(line, commit.is_some_and(|commit| commit.shallow)),
            }
        })
        .collect()
}

/// Precedence (20's shallow ruling): not committed → `Unblamable` →
/// `Shallow` → `Skipped` → `Plain`.
fn mark(line: &BlameLine, shallow: bool) -> RowMark {
    if line.commit.is_none() {
        return RowMark::NotCommitted;
    }
    match line.mark {
        LineMark::Unblamable => RowMark::Unblamable,
        _ if shallow => RowMark::Shallow,
        LineMark::Skipped => RowMark::Skipped,
        LineMark::Plain => RowMark::Plain,
    }
}

/// The row index of final line `line`, clamped to the file.
pub(super) fn row_of_line(blame: &Blame, line: u32) -> usize {
    blame
        .lines
        .iter()
        .position(|at| at.final_line >= line)
        .unwrap_or(blame.lines.len().saturating_sub(1))
}
