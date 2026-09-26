//! [`hunk_patch`]: a complete one-file unified patch for one hunk of a
//! [`FileDiff`], whole or with individual lines selected — what `ops`'s hunk
//! staging (`git apply --cached`) and discard (`git apply -R`) both spawn
//! against.
//!
//! Pure: no `git` spawn, no I/O. [`Selection::WholeHunk`] reproduces the
//! hunk's own bytes, byte-for-byte, as `diff-tree -p` printed them —
//! `parse_unified` is lossless for a hunk's kind/text/no-newline fields, so
//! serialising them back out matches the input exactly, and git accepts it
//! both ways round.
//!
//! [`Selection::Lines`] cannot: a partial patch is built for one direction
//! and refused by the other, so it takes a [`PatchDirection`].
//! [`PatchDirection::Forward`] (`git apply`, `git apply --cached`) matches
//! the hunk's *old* side, so an unselected `Added` line is dropped and an
//! unselected `Removed` line becomes context — `git add -p`'s rule.
//! [`PatchDirection::Reverse`] (`git apply -R`, for unstage and discard)
//! matches the hunk's *new* side, which is what the file actually holds, so
//! the rule mirrors: an unselected `Added` line becomes context and an
//! unselected `Removed` line is dropped. Either way the `@@` header's counts
//! are recomputed from the lines that survived.
//!
//! One shape has no honest partial patch at all: a `\ No newline at end of
//! file` line that the selection turns into context while a later line on
//! the same side survives. Git accepts such a patch and fuses the two lines
//! into one (verified: staging `+three` alone out of `one\ntwo` (no final
//! newline) → `one\ntwo\nthree` stages `one\ntwothree`), because selecting
//! the later line silently entails giving the marker line a newline.
//! [`hunk_patch`] refuses it with [`PatchError::NoNewlineBoundary`] rather
//! than write wrong content; the whole hunk still stages and discards.

use std::collections::HashSet;

use super::{DiffLine, FileDiff, FileStatus, LineKind};

/// Which lines of a hunk go into the patch [`hunk_patch`] builds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Selection {
    /// The whole hunk, unchanged.
    WholeHunk,
    /// Only these lines: indices into [`super::Hunk::lines`], each naming an
    /// `Added` or `Removed` line. What happens to the unselected ones depends
    /// on the [`PatchDirection`] the patch is built for; every `Context` line
    /// stays either way.
    Lines(Vec<usize>),
}

/// Which way the patch [`hunk_patch`] builds will be applied.
///
/// It decides nothing for [`Selection::WholeHunk`] — git's own hunk applies
/// both ways — and everything for [`Selection::Lines`], because a partial
/// patch's preimage is the side git will match against the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatchDirection {
    /// `git apply` / `git apply --cached`: staging a selection into the
    /// index, or applying it to a worktree that holds the old side. The
    /// preimage is the hunk's old side, so an unselected `Added` line is
    /// dropped and an unselected `Removed` line becomes context.
    Forward,
    /// `git apply -R` / `git apply --cached -R`: unstaging a selection, or
    /// discarding it from the worktree. The preimage is the hunk's new side —
    /// what the file holds — so an unselected `Added` line becomes context
    /// and an unselected `Removed` line is dropped.
    Reverse,
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
    /// The selection would have put a `\ No newline at end of file` line in
    /// the middle of a side of the patch: this line carries the marker, and
    /// the selection leaves a later line on the same side alive. Git accepts
    /// such a patch and fuses the two lines into one, so there is no honest
    /// partial patch here — the whole hunk is the only correct unit.
    NoNewlineBoundary {
        /// The index, in [`super::Hunk::lines`], of the marker-carrying line.
        line: usize,
    },
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
            PatchError::NoNewlineBoundary { line } => write!(
                f,
                "line {line} has no newline at end of file and something after it \
                 survived the selection; stage or discard the whole hunk instead"
            ),
        }
    }
}

impl std::error::Error for PatchError {}

/// Builds a complete one-file unified patch holding one hunk of `diff`, for
/// `git apply`/`git apply --cached`/`git apply -R` (`ops`'s job to spawn).
///
/// `--- a/<path>` and `+++ b/<path>` name the diff's path (the old path for a
/// rename or copy on the `a/` side); `/dev/null` stands in for the side that
/// does not exist ([`FileStatus::Added`]/[`FileStatus::Deleted`]), and the
/// header says so with `new file mode`/`deleted file mode` — without it git
/// reads `/dev/null` as a path (see `write_git_headers`). For any other
/// status a mode line pair (`old mode`/`new mode`) is added when
/// `diff.old_mode` and `diff.new_mode` are both known and differ.
///
/// `direction` says which way `ops` will apply the result; it changes nothing
/// for [`Selection::WholeHunk`] and everything for [`Selection::Lines`] (see
/// [`PatchDirection`]).
///
/// # Errors
/// See [`PatchError`].
pub fn hunk_patch(
    diff: &FileDiff,
    hunk: usize,
    selection: &Selection,
    direction: PatchDirection,
) -> Result<Vec<u8>, PatchError> {
    if diff.binary.is_some() {
        return Err(PatchError::Binary);
    }
    let hunk_ref = diff
        .hunks
        .get(hunk)
        .ok_or(PatchError::NoSuchHunk { hunk })?;
    let lines = selected_lines(hunk_ref.lines.as_slice(), selection, direction)?;
    check_no_newline_boundaries(&lines)?;

    let mut out = Vec::new();
    write_git_headers(&mut out, diff);
    write_path_lines(&mut out, diff);
    write_hunk(&mut out, hunk_ref, &lines);
    Ok(out)
}

/// One line of the patch's body, with its final kind and the index it came
/// from in the hunk (so an error can name it). The header's counts are
/// computed from these.
struct BuiltLine<'a> {
    index: usize,
    kind: LineKind,
    text: &'a [u8],
    no_newline_at_eof: bool,
}

impl BuiltLine<'_> {
    /// Whether this line is part of the patch's old side (`-`/` `).
    fn on_old_side(&self) -> bool {
        self.kind != LineKind::Added
    }

    /// Whether this line is part of the patch's new side (`+`/` `).
    fn on_new_side(&self) -> bool {
        self.kind != LineKind::Removed
    }
}

/// Applies `selection` to `lines`, in file order, following `direction`'s
/// rule for the unselected ones (see [`PatchDirection`]); `Context` lines are
/// kept as-is either way.
///
/// # Errors
/// See [`PatchError`].
fn selected_lines<'a>(
    lines: &'a [DiffLine],
    selection: &Selection,
    direction: PatchDirection,
) -> Result<Vec<BuiltLine<'a>>, PatchError> {
    match selection {
        Selection::WholeHunk => Ok(lines
            .iter()
            .enumerate()
            .map(|(index, line)| BuiltLine {
                index,
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
                    // A context line, and any line the caller picked, goes in
                    // as it stands. An unselected change is kept — as context
                    // — only on the side the patch's preimage has to match:
                    // the old side going forward, the new side in reverse.
                    // On the other side it was never there, so it is dropped.
                    let kind = if line.kind == LineKind::Context || selected.contains(&index) {
                        line.kind
                    } else {
                        let preimage_side = match direction {
                            PatchDirection::Forward => LineKind::Removed,
                            PatchDirection::Reverse => LineKind::Added,
                        };
                        if line.kind != preimage_side {
                            return None;
                        }
                        LineKind::Context
                    };
                    Some(BuiltLine {
                        index,
                        kind,
                        text: &line.text,
                        no_newline_at_eof: line.no_newline_at_eof,
                    })
                })
                .collect())
        }
    }
}

/// Refuses a patch whose `\ No newline at end of file` marker would land
/// anywhere but at the end of the side it belongs to.
///
/// The marker says "the line just above ends the file with no newline". It is
/// true of the old side for a `-` line, of the new side for a `+` line, and of
/// both for a context line. A selection that turns a marker-carrying line into
/// context, or that leaves something after it on a side it now claims to end,
/// makes that claim false — and git does not notice: it applies the patch and
/// fuses the marker line with the next one (module doc). Refusing is the only
/// answer that cannot write content the user did not ask for.
///
/// # Errors
/// [`PatchError::NoNewlineBoundary`], naming the marker-carrying line.
fn check_no_newline_boundaries(lines: &[BuiltLine<'_>]) -> Result<(), PatchError> {
    for (position, line) in lines.iter().enumerate() {
        if !line.no_newline_at_eof {
            continue;
        }
        let after = &lines[position + 1..];
        let old_side_broken = line.on_old_side() && after.iter().any(BuiltLine::on_old_side);
        let new_side_broken = line.on_new_side() && after.iter().any(BuiltLine::on_new_side);
        if old_side_broken || new_side_broken {
            return Err(PatchError::NoNewlineBoundary { line: line.index });
        }
    }
    Ok(())
}

/// The `diff --git` header and its extended lines.
///
/// Under a `diff --git` header, `git apply` treats `/dev/null` in the
/// `---`/`+++` lines as an ordinary path unless `new file mode` or
/// `deleted file mode` says the file comes or goes: a deletion without it
/// empties the file and adds one named `dev/null` (m7 review 06b, C1). So a
/// creation or deletion always carries its extended line. When the mode is
/// unknown — git's own diff always prints it, so only a hand-built
/// [`FileDiff`] lacks it — the `diff --git` line is left out instead, and git
/// reads the rest as a traditional patch, where `/dev/null` does mean "no
/// file". Guessing a mode would write one the user never had.
fn write_git_headers(out: &mut Vec<u8>, diff: &FileDiff) {
    let creation_or_deletion = match diff.entry.status {
        FileStatus::Added => Some(("new file mode", diff.new_mode)),
        FileStatus::Deleted => Some(("deleted file mode", diff.old_mode)),
        _ => None,
    };
    match creation_or_deletion {
        None => {
            write_diff_git_header(out, diff);
            write_mode_lines(out, diff);
        }
        Some((label, Some(mode))) => {
            write_diff_git_header(out, diff);
            out.extend_from_slice(format!("{label} {mode:o}\n").as_bytes());
        }
        Some((_, None)) => {}
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
    let old_lines =
        u32::try_from(lines.iter().filter(|line| line.on_old_side()).count()).unwrap_or(u32::MAX);
    let new_lines =
        u32::try_from(lines.iter().filter(|line| line.on_new_side()).count()).unwrap_or(u32::MAX);

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
