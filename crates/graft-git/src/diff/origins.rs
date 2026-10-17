//! Where a merge result's new lines came from: yours, theirs, or both —
//! the read-only look's "from yours / from theirs" sample.

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
/// Every line the result adds over the base is matched against the lines
/// each side adds over the base, **in order**: the longest run of each
/// side's added lines that appears, in the same order, among the result's
/// added lines. Claimed by yours only → [`Origin::Yours`], theirs only →
/// [`Origin::Theirs`], both → [`Origin::Both`]. Each side's line is claimed
/// once, where its order puts it, so a common text — a blank line, a lone
/// `}` — is not claimed wherever it appears. A side's line numbers do not
/// survive the merge, its text and order do. A result line no side claims
/// (a hand edit, say) and every line the result keeps from the base are not
/// listed. Result order; a binary diff has no lines and lists nothing.
///
/// A side too large to align (over 4 M result × side line pairs)
/// claims nothing: no attribution rather than a wrong one.
#[must_use]
pub fn line_origins(result: &FileDiff, yours: &FileDiff, theirs: &FileDiff) -> Vec<LineOrigin> {
    let result: Vec<(u32, &[u8])> = added_lines(result).collect();
    let texts: Vec<&[u8]> = result.iter().map(|&(_, text)| text).collect();
    let by_yours = claimed(
        &texts,
        &added_lines(yours).map(|(_, t)| t).collect::<Vec<_>>(),
    );
    let by_theirs = claimed(
        &texts,
        &added_lines(theirs).map(|(_, t)| t).collect::<Vec<_>>(),
    );
    result
        .iter()
        .enumerate()
        .filter_map(|(at, &(line, _))| {
            let origin = match (by_yours[at], by_theirs[at]) {
                (true, true) => Origin::Both,
                (true, false) => Origin::Yours,
                (false, true) => Origin::Theirs,
                (false, false) => return None,
            };
            Some(LineOrigin { line, origin })
        })
        .collect()
}

/// The most result × side cells [`line_origins`] aligns: 4 M cells, 16 MiB.
const ALIGN_CAP: usize = 4_000_000;

/// Which of `result`'s lines a longest common subsequence with `side`
/// claims, earliest result line first among equal choices.
fn claimed(result: &[&[u8]], side: &[&[u8]]) -> Vec<bool> {
    let mut claimed = vec![false; result.len()];
    let width = side.len() + 1;
    if (result.len() + 1).saturating_mul(width) > ALIGN_CAP {
        return claimed;
    }
    // `run[i * width + j]`: the longest common run of `result[i..]`, `side[j..]`.
    let mut run = vec![0u32; (result.len() + 1) * width];
    for i in (0..result.len()).rev() {
        for j in (0..side.len()).rev() {
            run[i * width + j] = if result[i] == side[j] {
                run[(i + 1) * width + j + 1] + 1
            } else {
                run[(i + 1) * width + j].max(run[i * width + j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    while i < result.len() && j < side.len() {
        if result[i] == side[j] && run[i * width + j] == run[(i + 1) * width + j + 1] + 1 {
            claimed[i] = true;
            i += 1;
            j += 1;
        } else if run[(i + 1) * width + j] >= run[i * width + j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    claimed
}

/// Each added line's new-side number and text.
fn added_lines(diff: &FileDiff) -> impl Iterator<Item = (u32, &[u8])> {
    diff.hunks
        .iter()
        .flat_map(|hunk| &hunk.lines)
        .filter(|line| line.kind == LineKind::Added)
        .filter_map(|line| Some((line.new_no?, line.text.as_slice())))
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
    fn a_common_text_is_claimed_where_its_side_put_it_not_wherever_it_appears() {
        // Yours added a function and a blank line; theirs another function.
        // Both end in a lone `}`, so `}` alone says nothing of its side.
        let yours = diff(&[(Added, "fn a() {"), (Added, "}"), (Added, "")]);
        let theirs = diff(&[(Added, "fn b() {"), (Added, "}")]);
        let result = diff(&[
            (Added, "fn a() {"), // 1: yours
            (Added, "}"),        // 2: yours' `}`, not theirs'
            (Added, ""),         // 3: yours' blank line
            (Added, "fn b() {"), // 4: theirs
            (Added, "}"),        // 5: theirs' `}`
            (Added, ""),         // 6: typed by hand — not listed
            (Added, "}"),        // 7: typed by hand — not listed
        ]);
        let origin = |line, origin| LineOrigin { line, origin };
        assert_eq!(
            line_origins(&result, &yours, &theirs),
            vec![
                origin(1, Origin::Yours),
                origin(2, Origin::Yours),
                origin(3, Origin::Yours),
                origin(4, Origin::Theirs),
                origin(5, Origin::Theirs),
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
