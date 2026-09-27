//! The marks a blame's lines carry, read from git's **human** format when
//! the porcelain has none (m7-power-tools/18e).
//!
//! git before [`GitVersion::BLAME_PORCELAIN_MARKS`] skips and gives up on
//! lines under an ignore-revs list exactly as later git does, but writes the
//! `ignored`/`unblamable` lines only in the human format: `?` and `*` before
//! the commit id (2.38.5 `builtin/blame.c` `emit_other`, lines 471-487; the
//! porcelain writer, lines 339-376, has none). So on such a git, [`blame`]
//! runs a second `git blame -l -s` and takes each line's marks from there.
//!
//! [`blame`]: super::blame

use super::LineMark;
use crate::diff::ParseError;
use crate::version::GitVersion;

/// Marks from `git blame -l -s` (human format), one per output line, in
/// final-line order.
///
/// Only each line's leading run of `^` (boundary), `*` (unblamable) and `?`
/// (ignored) is read. What follows is the commit id in hex, so a line's
/// content can never be mistaken for a mark. `*` and `?` together (a line
/// passed on by one ignored commit, then stopped at another) is
/// [`LineMark::Unblamable`]: git keeps such a line on the ignored commit
/// (2.38.5 `blame.c` line 1760), and the porcelain reader maps the pair the
/// same way.
///
/// # Errors
/// [`ParseError`] naming the 1-indexed line that has no commit id after its
/// marks.
pub fn parse_marks(bytes: &[u8]) -> Result<Vec<LineMark>, ParseError> {
    let mut marks = Vec::new();
    let mut lines = bytes.split(|&b| b == b'\n').enumerate().peekable();
    while let Some((index, line)) = lines.next() {
        if line.is_empty() && lines.peek().is_none() {
            break; // the tail after the final newline
        }
        let run = line
            .iter()
            .take_while(|&&b| matches!(b, b'^' | b'*' | b'?'))
            .count();
        if !line.get(run).is_some_and(u8::is_ascii_hexdigit) {
            return Err(ParseError {
                line: index + 1,
                what: "expected a commit id after blame's marks",
            });
        }
        let run = &line[..run];
        marks.push(if run.contains(&b'*') {
            LineMark::Unblamable
        } else if run.contains(&b'?') {
            LineMark::Skipped
        } else {
            LineMark::Plain
        });
    }
    Ok(marks)
}

/// Whether blame must read its marks from a second, human-format pass.
///
/// All four must hold:
/// 1. `ignore_revs`: with `false`, blame passes `--no-ignore-revs-file` and
///    git marks nothing;
/// 2. `!porcelain_has_marks`: a git that wrote them has answered;
/// 3. `list_in_effect`: the root's list was passed, or `blame.ignoreRevsFile`
///    is set (no list, nothing to mark);
/// 4. `git` is older than [`GitVersion::BLAME_PORCELAIN_MARKS`], or its
///    version is unknown (`None`): the pass is then slower, never wrong.
pub(crate) fn needs_marks_pass(
    ignore_revs: bool,
    porcelain_has_marks: bool,
    list_in_effect: bool,
    git: Option<&GitVersion>,
) -> bool {
    ignore_revs
        && !porcelain_has_marks
        && list_in_effect
        && git.is_none_or(|version| *version < GitVersion::BLAME_PORCELAIN_MARKS)
}
