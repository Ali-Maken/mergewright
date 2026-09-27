//! The marks a blame's lines carry, read from git's **human** format when
//! the porcelain has none (m7-power-tools/18e).
//!
//! git before [`crate::version::GitVersion::BLAME_PORCELAIN_MARKS`] skips and gives up on
//! lines under an ignore-revs list exactly as later git does, but writes the
//! `ignored`/`unblamable` lines only in the human format: `?` and `*` before
//! the commit id (2.38.5 `builtin/blame.c` `emit_other`, lines 471-487; the
//! porcelain writer, lines 339-376, has none). So on such a git, [`blame`]
//! runs a second `git blame -l -s` and takes each line's marks from there.
//!
//! [`blame`]: super::blame

use super::LineMark;
use crate::diff::ParseError;

/// One line of `git blame -l -s`: its mark, and the id git printed after it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkedLine {
    /// What the line's leading `^`/`*`/`?` run says.
    pub mark: LineMark,
    /// The hex after the marks, as git printed it: a prefix of the line's
    /// full commit id, one digit shorter per mark (2.38.5 `emit_other`,
    /// `length--`); all zeros for a line not committed yet.
    pub id_prefix: String,
}

/// Marks from `git blame -l -s` (human format), one per output line, in
/// final-line order, each with the id prefix that follows it.
///
/// Only each line's leading run of `^` (boundary), `*` (unblamable) and `?`
/// (ignored) is read as marks. What follows is the commit id in hex, so a
/// line's content can never be mistaken for a mark. `*` and `?` together (a
/// line passed on by one ignored commit, then stopped at another) is
/// [`LineMark::Unblamable`]: git keeps such a line on the ignored commit
/// (2.38.5 `blame.c` line 1760), and the porcelain reader maps the pair the
/// same way.
///
/// # Errors
/// [`ParseError`] naming the 1-indexed line that has no commit id after its
/// marks.
pub fn parse_marks(bytes: &[u8]) -> Result<Vec<MarkedLine>, ParseError> {
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
        let (run, rest) = line.split_at(run);
        let hex = rest.iter().take_while(|b| b.is_ascii_hexdigit()).count();
        if hex == 0 {
            return Err(ParseError {
                line: index + 1,
                what: "expected a commit id after blame's marks",
            });
        }
        let mark = if run.contains(&b'*') {
            LineMark::Unblamable
        } else if run.contains(&b'?') {
            LineMark::Skipped
        } else {
            LineMark::Plain
        };
        marks.push(MarkedLine {
            mark,
            // ASCII hex digits only: the lossy conversion never replaces.
            id_prefix: String::from_utf8_lossy(&rest[..hex]).into_owned(),
        });
    }
    Ok(marks)
}

#[cfg(test)]
mod tests;
