//! [`hunk_patch`]: a complete one-file unified patch for one hunk of a
//! [`FileDiff`], whole or with individual lines selected — what `ops`'s hunk
//! staging (`git apply --cached`) and discard (`git apply -R`) both spawn
//! against.
//!
//! Pure: no `git` spawn, no I/O. [`Selection::WholeHunk`] reproduces the
//! hunk's own bytes, byte-for-byte, as `diff-tree -p` printed them —
//! `parse_unified` is lossless for a hunk's kind/text/no-newline fields, so
//! serialising them back out matches the input exactly. [`Selection::Lines`]
//! drops unselected `Added` lines and turns unselected `Removed` lines back
//! into context, the way `git add -p` builds a patch for a partial hunk, and
//! recomputes the `@@` header's counts to match.

use std::collections::HashSet;

use super::{DiffLine, FileDiff, FileStatus, LineKind};

/// Which lines of a hunk go into the patch [`hunk_patch`] builds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Selection {
    /// The whole hunk, unchanged.
    WholeHunk,
    /// Only these lines: indices into [`super::Hunk::lines`], each naming an
    /// `Added` or `Removed` line. An unselected `Added` line is dropped, an
    /// unselected `Removed` line becomes context; every `Context` line stays.
    Lines(Vec<usize>),
}

/// Why [`hunk_patch`] could not build a patch.
#[derive(Debug, PartialEq, Eq)]
pub enum PatchError {
    /// `hunk` names no hunk of the diff.
    NoSuchHunk {
        /// The hunk index that was asked for.
        hunk: usize,
    },
    /// A [`Selection::Lines`] index names no line of the hunk.
    NoSuchLine {
        /// The line index that was asked for.
        line: usize,
    },
    /// A [`Selection::Lines`] index names a line that is `Context`, not
    /// `Added` or `Removed` — there is nothing to select about a line
    /// already present on both sides.
    NotAChangedLine {
        /// The line index that was asked for.
        line: usize,
    },
    /// The file is binary: there are no hunks to select lines from.
    Binary,
    /// A [`Selection::Lines`] selected no lines, so there is nothing to
    /// stage or discard.
    NothingSelected,
}

impl std::fmt::Display for PatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatchError::NoSuchHunk { hunk } => write!(f, "no hunk {hunk} in this diff"),
            PatchError::NoSuchLine { line } => write!(f, "no line {line} in this hunk"),
            PatchError::NotAChangedLine { line } => {
                write!(f, "line {line} is context, not a change")
            }
            PatchError::Binary => write!(f, "the file is binary; there are no hunks"),
            PatchError::NothingSelected => write!(f, "nothing was selected"),
        }
    }
}

impl std::error::Error for PatchError {}

/// Builds a complete one-file unified patch holding one hunk of `diff`, for
/// `git apply`/`git apply --cached`/`git apply -R` (`ops`'s job to spawn).
///
/// `--- a/<path>` and `+++ b/<path>` name the diff's path (the old path for a
/// rename or copy on the `a/` side); `/dev/null` stands in for the side that
/// does not exist ([`FileStatus::Added`]/[`FileStatus::Deleted`]). A mode
/// line pair (`old mode`/`new mode`) is added when `diff.old_mode` and
/// `diff.new_mode` are both known and differ.
///
/// # Errors
/// See [`PatchError`].
pub fn hunk_patch(
    diff: &FileDiff,
    hunk: usize,
    selection: &Selection,
) -> Result<Vec<u8>, PatchError> {
    if diff.binary.is_some() {
        return Err(PatchError::Binary);
    }
    let hunk_ref = diff
        .hunks
        .get(hunk)
        .ok_or(PatchError::NoSuchHunk { hunk })?;
    let lines = selected_lines(hunk_ref.lines.as_slice(), selection)?;

    let mut out = Vec::new();
    write_diff_git_header(&mut out, diff);
    write_mode_lines(&mut out, diff);
    write_path_lines(&mut out, diff);
    write_hunk(&mut out, hunk_ref, &lines);
    Ok(out)
}

/// One line of the patch's body, with its final kind and renumbered old/new
/// line numbers (the header's counts are computed from these).
struct BuiltLine<'a> {
    kind: LineKind,
    text: &'a [u8],
    no_newline_at_eof: bool,
}

/// Applies `selection` to `lines`, in file order, and reports the result:
/// an unselected `Added` line is dropped, an unselected `Removed` line
/// becomes `Context`, everything else is kept as-is.
///
/// # Errors
/// See [`PatchError`].
fn selected_lines<'a>(
    lines: &'a [DiffLine],
    selection: &Selection,
) -> Result<Vec<BuiltLine<'a>>, PatchError> {
    match selection {
        Selection::WholeHunk => Ok(lines
            .iter()
            .map(|line| BuiltLine {
                kind: line.kind,
                text: &line.text,
                no_newline_at_eof: line.no_newline_at_eof,
            })
            .collect()),
        Selection::Lines(indices) => {
            if indices.is_empty() {
                return Err(PatchError::NothingSelected);
            }
            let mut selected = HashSet::with_capacity(indices.len());
            for &index in indices {
                let line = lines
                    .get(index)
                    .ok_or(PatchError::NoSuchLine { line: index })?;
                if line.kind == LineKind::Context {
                    return Err(PatchError::NotAChangedLine { line: index });
                }
                selected.insert(index);
            }
            Ok(lines
                .iter()
                .enumerate()
                .filter_map(|(index, line)| {
                    let kind = match line.kind {
                        LineKind::Added if selected.contains(&index) => LineKind::Added,
                        LineKind::Added => return None,
                        LineKind::Removed if selected.contains(&index) => LineKind::Removed,
                        LineKind::Context | LineKind::Removed => LineKind::Context,
                    };
                    Some(BuiltLine {
                        kind,
                        text: &line.text,
                        no_newline_at_eof: line.no_newline_at_eof,
                    })
                })
                .collect())
        }
    }
}

fn write_diff_git_header(out: &mut Vec<u8>, diff: &FileDiff) {
    let old_path = diff.entry.old_path.as_ref().unwrap_or(&diff.entry.path);
    out.extend_from_slice(b"diff --git a/");
    out.extend_from_slice(old_path);
    out.extend_from_slice(b" b/");
    out.extend_from_slice(&diff.entry.path);
    out.push(b'\n');
}

fn write_mode_lines(out: &mut Vec<u8>, diff: &FileDiff) {
    if let (Some(old_mode), Some(new_mode)) = (diff.old_mode, diff.new_mode)
        && old_mode != new_mode
    {
        out.extend_from_slice(format!("old mode {old_mode:o}\n").as_bytes());
        out.extend_from_slice(format!("new mode {new_mode:o}\n").as_bytes());
    }
}

fn write_path_lines(out: &mut Vec<u8>, diff: &FileDiff) {
    let old_path = diff.entry.old_path.as_ref().unwrap_or(&diff.entry.path);
    out.extend_from_slice(b"--- ");
    if diff.entry.status == FileStatus::Added {
        out.extend_from_slice(b"/dev/null");
    } else {
        out.extend_from_slice(b"a/");
        out.extend_from_slice(old_path);
    }
    out.push(b'\n');

    out.extend_from_slice(b"+++ ");
    if diff.entry.status == FileStatus::Deleted {
        out.extend_from_slice(b"/dev/null");
    } else {
        out.extend_from_slice(b"b/");
        out.extend_from_slice(&diff.entry.path);
    }
    out.push(b'\n');
}

/// `start,count`, or bare `start` when `count == 1` — the form git itself
/// omits the count for (verified against real git output: a one-line hunk
/// prints `@@ -0,0 +1 @@`, never `@@ -0,0 +1,1 @@`).
fn range(start: u32, count: u32) -> String {
    if count == 1 {
        start.to_string()
    } else {
        format!("{start},{count}")
    }
}

fn write_hunk(out: &mut Vec<u8>, hunk: &super::Hunk, lines: &[BuiltLine<'_>]) {
    let old_lines = u32::try_from(
        lines
            .iter()
            .filter(|line| line.kind != LineKind::Added)
            .count(),
    )
    .unwrap_or(u32::MAX);
    let new_lines = u32::try_from(
        lines
            .iter()
            .filter(|line| line.kind != LineKind::Removed)
            .count(),
    )
    .unwrap_or(u32::MAX);

    out.extend_from_slice(
        format!(
            "@@ -{} +{} @@",
            range(hunk.old_start, old_lines),
            range(hunk.new_start, new_lines)
        )
        .as_bytes(),
    );
    if !hunk.heading.is_empty() {
        out.push(b' ');
        out.extend_from_slice(&hunk.heading);
    }
    out.push(b'\n');

    for line in lines {
        out.push(match line.kind {
            LineKind::Context => b' ',
            LineKind::Added => b'+',
            LineKind::Removed => b'-',
        });
        out.extend_from_slice(line.text);
        out.push(b'\n');
        if line.no_newline_at_eof {
            out.extend_from_slice(b"\\ No newline at end of file\n");
        }
    }
}

#[cfg(test)]
mod tests;
