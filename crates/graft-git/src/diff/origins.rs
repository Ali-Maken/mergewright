//! Where a merge result's new lines came from: yours, theirs, or both —
//! the read-only look's "from yours / from theirs" sample.

use std::collections::HashSet;

use super::{FileDiff, LineKind};

/// Which side a result line came in from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// Added by yours only.
    Yours,
    /// Added by theirs only.
    Theirs,
    /// Added by both sides alike.
    Both,
}

/// One result line and where it came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineOrigin {
    /// 1-based line number in the result.
    pub line: u32,
    /// The side it came from.
    pub origin: Origin,
}

/// Diffs are base→result, base→yours, base→theirs; `line` is 1-based in the
/// result.
///
/// Every line the result adds over the base is matched **by its text**
/// against the lines each side adds over the base: in yours' added lines
/// only → [`Origin::Yours`], theirs' only → [`Origin::Theirs`], both →
/// [`Origin::Both`]. A result line neither side added (a hand edit, say)
/// and every line the result keeps from the base are not listed. Text, not
/// position: a side's line numbers do not survive the merge, its text
/// does. Result order; a binary diff has no lines and lists nothing.
#[must_use]
pub fn line_origins(result: &FileDiff, yours: &FileDiff, theirs: &FileDiff) -> Vec<LineOrigin> {
    let yours = added_texts(yours);
    let theirs = added_texts(theirs);
    added_lines(result)
        .filter_map(|(line, text)| {
            let origin = match (yours.contains(text), theirs.contains(text)) {
                (true, true) => Origin::Both,
                (true, false) => Origin::Yours,
                (false, true) => Origin::Theirs,
                (false, false) => return None,
            };
            Some(LineOrigin { line, origin })
        })
        .collect()
}

/// Each added line's new-side number and text.
fn added_lines(diff: &FileDiff) -> impl Iterator<Item = (u32, &[u8])> {
    diff.hunks
        .iter()
        .flat_map(|hunk| &hunk.lines)
        .filter(|line| line.kind == LineKind::Added)
        .filter_map(|line| Some((line.new_no?, line.text.as_slice())))
}

fn added_texts(diff: &FileDiff) -> HashSet<&[u8]> {
    added_lines(diff).map(|(_, text)| text).collect()
}

#[cfg(test)]
mod tests {
    use super::super::{DiffLine, FileEntry, FileStatus, Hunk};
    use super::*;

    /// One hunk from `(kind, text)` pairs, numbered from line 1 on both sides.
    fn diff(lines: &[(LineKind, &str)]) -> FileDiff {
        let (mut old, mut new) = (0u32, 0u32);
        let lines = lines
            .iter()
            .map(|&(kind, text)| {
                let old_no = (kind != LineKind::Added).then(|| {
                    old += 1;
                    old
                });
                let new_no = (kind != LineKind::Removed).then(|| {
                    new += 1;
                    new
                });
                DiffLine {
                    kind,
                    text: text.as_bytes().to_vec(),
                    old_no,
                    new_no,
                    no_newline_at_eof: false,
                }
            })
            .collect();
        FileDiff {
            entry: FileEntry {
                path: b"f.txt".to_vec(),
                old_path: None,
                status: FileStatus::Modified,
                added: None,
                removed: None,
            },
            hunks: vec![Hunk {
                old_start: 1,
                old_lines: old,
                new_start: 1,
                new_lines: new,
                heading: Vec::new(),
                lines,
            }],
            binary: None,
            truncated: None,
            old_mode: None,
            new_mode: None,
        }
    }

    use LineKind::{Added, Context, Removed};

    #[test]
    fn each_added_line_names_its_side_and_unclaimed_lines_are_not_listed() {
        // Base: `keep`, `old`. Yours rewrote `old` and added `shared`;
        // theirs added `shared` and `theirs only`.
        let yours = diff(&[
            (Context, "keep"),
            (Removed, "old"),
            (Added, "yours only"),
            (Added, "shared"),
        ]);
        let theirs = diff(&[
            (Context, "keep"),
            (Context, "old"),
            (Added, "shared"),
            (Added, "theirs only"),
        ]);
        let result = diff(&[
            (Context, "keep"), // 1: from the base — not listed
            (Removed, "old"),
            (Added, "yours only"),   // 2
            (Added, "shared"),       // 3
            (Added, "theirs only"),  // 4
            (Added, "neither side"), // 5: a hand edit — not listed
        ]);
        assert_eq!(
            line_origins(&result, &yours, &theirs),
            vec![
                LineOrigin {
                    line: 2,
                    origin: Origin::Yours
                },
                LineOrigin {
                    line: 3,
                    origin: Origin::Both
                },
                LineOrigin {
                    line: 4,
                    origin: Origin::Theirs
                },
            ]
        );
    }

    #[test]
    fn a_binary_result_lists_nothing() {
        let mut result = diff(&[]);
        result.hunks.clear();
        result.binary = Some(super::super::BinaryNote::Differ);
        let side = diff(&[(Added, "x")]);
        assert!(line_origins(&result, &side, &side).is_empty());
    }
}
