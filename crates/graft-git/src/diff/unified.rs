//! `parse_unified`: turns the bytes of `git diff-tree -p -M -C --no-color
//! --no-ext-diff` (one or many files) into [`FileDiff`]s.
//!
//! Pure byte parsing, no `git` spawn — git's own output is the oracle
//! (fixture round-trip test) and this is also a `cargo-fuzz` target, so it
//! must never panic on arbitrary bytes.
//!
//! The byte-level primitives (numbers, percentages, quoted-path unquoting)
//! live in [`primitives`] (m2 review M8) — this file is the header/hunk
//! grammar built on top of them.

mod primitives;

pub(crate) use primitives::{find, unquote};
use primitives::{
    parse_diff_git_header, parse_mode, parse_percent, parse_range, parse_side_path, unquote_maybe,
};

use super::{BinaryNote, DiffLine, FileDiff, FileEntry, FileStatus, Hunk, LineKind, ParseError};

/// Parses one or many files' worth of unified diff output.
///
/// See the module doc for what this accepts. A line git could not have
/// produced from this command fails with [`ParseError`], naming the
/// 1-indexed input line — including a combined diff (`diff --cc`, `@@@`),
/// which is a shape this parser does not read rather than one it can report
/// as no changes.
pub fn parse_unified(bytes: &[u8]) -> Result<Vec<FileDiff>, ParseError> {
    let lines = split_lines(bytes);
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if let Some(rest) = lines[i].strip_prefix(b"diff --git ") {
            let (diff_git_old, diff_git_new) = parse_diff_git_header(rest).ok_or(ParseError {
                line: i + 1,
                what: "malformed `diff --git` header",
            })?;
            i += 1;
            let mut building = Building::new(diff_git_old, diff_git_new);
            i = parse_file_section(&lines, i, &mut building)?;
            out.push(building.finish());
        } else {
            reject_line_outside_a_file_section(lines[i], i)?;
            i += 1;
        }
    }
    Ok(out)
}

/// Fails unless `line` is the one thing that legitimately sits outside a
/// `diff --git` section: the bare object id `git diff-tree -p` prints before
/// the first file (leaving it out takes `--no-commit-id`, which none of our
/// invocations pass).
///
/// It is one named case, not a licence to skip. A combined diff — git's
/// answer for an unmerged path — carries no `diff --git` line anywhere, so
/// *every* line of such a reply arrives here; skipping them all returned
/// `Ok(vec![])`, success with nothing in it, and the caller had no error to
/// show (m5-history-and-ghost-preview/22).
fn reject_line_outside_a_file_section(line: &[u8], idx: usize) -> Result<(), ParseError> {
    reject_combined_diff(line, idx)?;
    if is_bare_object_id(line) {
        return Ok(());
    }
    Err(ParseError {
        line: idx + 1,
        what: "unreadable line outside any `diff --git` section",
    })
}

/// Fails on git's combined format — `diff --cc`, `diff --combined` and their
/// `@@@` hunk headers — which it prints for an unmerged path and for a merge
/// commit shown with `-c`. This parser reads two-sided diffs only; a caller
/// that wants an unmerged path's content reads it against one side.
///
/// Named and refused rather than parsed: a wrong answer here is invisible,
/// and the error says which shape arrived.
fn reject_combined_diff(line: &[u8], idx: usize) -> Result<(), ParseError> {
    if line.starts_with(b"diff --cc ")
        || line.starts_with(b"diff --combined ")
        || line.starts_with(b"@@@ ")
    {
        return Err(ParseError {
            line: idx + 1,
            what: "combined diff (`diff --cc`) is not supported here",
        });
    }
    Ok(())
}

/// A bare object id on its own line: 40 hex digits (sha1) or 64 (sha256).
/// Never abbreviated — abbreviating `diff-tree`'s commit id takes an explicit
/// `--abbrev`, so a short id here is output we did not ask for.
fn is_bare_object_id(line: &[u8]) -> bool {
    matches!(line.len(), 40 | 64) && line.iter().all(u8::is_ascii_hexdigit)
}

/// One file's headers and hunks, as they arrive; turned into a [`FileDiff`]
/// by [`Building::finish`].
struct Building {
    diff_git_old: Vec<u8>,
    diff_git_new: Vec<u8>,
    rename_from: Option<Vec<u8>>,
    rename_to: Option<Vec<u8>>,
    copy_from: Option<Vec<u8>>,
    copy_to: Option<Vec<u8>>,
    dash_old_present: bool,
    dash_old: Option<Vec<u8>>,
    plus_new_present: bool,
    plus_new: Option<Vec<u8>>,
    old_mode: Option<u32>,
    new_mode: Option<u32>,
    deleted_file_mode_seen: bool,
    new_file_mode_seen: bool,
    similarity: Option<u8>,
    binary: Option<BinaryNote>,
    hunks: Vec<Hunk>,
}

impl Building {
    fn new(diff_git_old: Vec<u8>, diff_git_new: Vec<u8>) -> Self {
        Self {
            diff_git_old,
            diff_git_new,
            rename_from: None,
            rename_to: None,
            copy_from: None,
            copy_to: None,
            dash_old_present: false,
            dash_old: None,
            plus_new_present: false,
            plus_new: None,
            old_mode: None,
            new_mode: None,
            deleted_file_mode_seen: false,
            new_file_mode_seen: false,
            similarity: None,
            binary: None,
            hunks: Vec::new(),
        }
    }

    fn is_deleted(&self) -> bool {
        self.deleted_file_mode_seen || (self.plus_new_present && self.plus_new.is_none())
    }

    fn is_added(&self) -> bool {
        self.new_file_mode_seen || (self.dash_old_present && self.dash_old.is_none())
    }

    fn path(&self) -> Vec<u8> {
        if self.is_deleted() {
            self.dash_old
                .clone()
                .unwrap_or_else(|| self.diff_git_old.clone())
        } else {
            self.rename_to
                .clone()
                .or_else(|| self.copy_to.clone())
                .or_else(|| self.plus_new.clone())
                .unwrap_or_else(|| self.diff_git_new.clone())
        }
    }

    fn old_path(&self) -> Option<Vec<u8>> {
        if self.rename_from.is_some() {
            self.rename_from.clone()
        } else {
            self.copy_from.clone()
        }
    }

    fn status(&self) -> FileStatus {
        if self.rename_from.is_some() || self.rename_to.is_some() {
            FileStatus::Renamed {
                similarity: self.similarity.unwrap_or(100),
            }
        } else if self.copy_from.is_some() || self.copy_to.is_some() {
            FileStatus::Copied {
                similarity: self.similarity.unwrap_or(100),
            }
        } else if self.is_deleted() {
            FileStatus::Deleted
        } else if self.is_added() {
            FileStatus::Added
        } else if let (Some(old), Some(new)) = (self.old_mode, self.new_mode) {
            if mode_type(old) == mode_type(new) {
                FileStatus::Modified
            } else {
                FileStatus::TypeChanged
            }
        } else {
            FileStatus::Modified
        }
    }

    fn finish(self) -> FileDiff {
        let (added, removed) = if self.binary.is_some() {
            (None, None)
        } else {
            let count = |kind| {
                let n = self
                    .hunks
                    .iter()
                    .flat_map(|hunk| &hunk.lines)
                    .filter(|line| line.kind == kind)
                    .count();
                u32::try_from(n).unwrap_or(u32::MAX)
            };
            (Some(count(LineKind::Added)), Some(count(LineKind::Removed)))
        };
        FileDiff {
            entry: FileEntry {
                path: self.path(),
                old_path: self.old_path(),
                status: self.status(),
                added,
                removed,
            },
            hunks: self.hunks,
            binary: self.binary,
            truncated: None,
            old_mode: self.old_mode,
            new_mode: self.new_mode,
        }
    }
}

/// The mode's type bits (its leading digits: `100` regular, `120` symlink,
/// `160` gitlink), used only to decide [`FileStatus::TypeChanged`].
fn mode_type(mode: u32) -> u32 {
    mode / 1000
}

/// Reads one file's header and hunk lines, starting right after its
/// `diff --git` line, until the next one or the end of input.
///
/// Returns the index of the next unread line (the next `diff --git`, or
/// `lines.len()`).
fn parse_file_section(
    lines: &[&[u8]],
    mut i: usize,
    building: &mut Building,
) -> Result<usize, ParseError> {
    while i < lines.len() && !lines[i].starts_with(b"diff --git ") {
        let line = lines[i];
        reject_combined_diff(line, i)?;
        if apply_metadata_line(building, line, i)? {
            i += 1;
        } else if line.starts_with(b"Binary files ") && line.ends_with(b" differ") {
            building.binary = Some(BinaryNote::Differ);
            i += 1;
        } else if line == b"GIT binary patch" {
            building.binary = Some(BinaryNote::Patch);
            i += 1;
            while i < lines.len() && !lines[i].starts_with(b"diff --git ") {
                i += 1;
            }
        } else if line.starts_with(b"@@ -") {
            let (hunk, next) = parse_hunk(lines, i)?;
            building.hunks.push(hunk);
            i = next;
        } else {
            // An extended header line this parser reads nothing from: the
            // `index <old>..<new> <mode>` line, or a header a future git
            // adds beside it. Skipped, and unlike the top-level case this
            // cannot hide a whole reply — the file's `diff --git` line was
            // already read, so the file is reported either way and the two
            // shapes that *would* hide one (`diff --cc`, `@@@`) are refused
            // above (m5-history-and-ghost-preview/22).
            i += 1;
        }
    }
    Ok(i)
}

/// Tries `line` against every plain `key: value` header this format has.
/// Returns `Ok(true)` if `line` matched one (whether or not its value
/// parsed), `Ok(false)` if `line` is none of these (the caller tries hunks,
/// binary notes, or gives up and skips it).
fn apply_metadata_line(
    building: &mut Building,
    line: &[u8],
    idx: usize,
) -> Result<bool, ParseError> {
    let err = |what| ParseError {
        line: idx + 1,
        what,
    };
    if let Some(rest) = line.strip_prefix(b"old mode ") {
        building.old_mode = Some(parse_mode(rest).ok_or_else(|| err("malformed `old mode`"))?);
    } else if let Some(rest) = line.strip_prefix(b"new mode ") {
        building.new_mode = Some(parse_mode(rest).ok_or_else(|| err("malformed `new mode`"))?);
    } else if let Some(rest) = line.strip_prefix(b"deleted file mode ") {
        building.deleted_file_mode_seen = true;
        building.old_mode =
            Some(parse_mode(rest).ok_or_else(|| err("malformed `deleted file mode`"))?);
    } else if let Some(rest) = line.strip_prefix(b"new file mode ") {
        building.new_file_mode_seen = true;
        building.new_mode = Some(parse_mode(rest).ok_or_else(|| err("malformed `new file mode`"))?);
    } else if let Some(rest) = line.strip_prefix(b"similarity index ") {
        building.similarity =
            Some(parse_percent(rest).ok_or_else(|| err("malformed `similarity index`"))?);
    } else if let Some(rest) = line.strip_prefix(b"rename from ") {
        building.rename_from = Some(unquote_maybe(rest));
    } else if let Some(rest) = line.strip_prefix(b"rename to ") {
        building.rename_to = Some(unquote_maybe(rest));
    } else if let Some(rest) = line.strip_prefix(b"copy from ") {
        building.copy_from = Some(unquote_maybe(rest));
    } else if let Some(rest) = line.strip_prefix(b"copy to ") {
        building.copy_to = Some(unquote_maybe(rest));
    } else if let Some(rest) = line.strip_prefix(b"--- ") {
        building.dash_old_present = true;
        building.dash_old = parse_side_path(rest);
    } else if let Some(rest) = line.strip_prefix(b"+++ ") {
        building.plus_new_present = true;
        building.plus_new = parse_side_path(rest);
    } else {
        return Ok(false);
    }
    Ok(true)
}

/// Parses one `@@ -old_start,old_lines +new_start,new_lines @@ heading` hunk
/// and its body, starting at `lines[start]`.
///
/// Returns the hunk and the index of the first unread line.
fn parse_hunk(lines: &[&[u8]], start: usize) -> Result<(Hunk, usize), ParseError> {
    let (old_start, old_lines, new_start, new_lines, heading) = parse_hunk_header(lines[start])
        .ok_or(ParseError {
            line: start + 1,
            what: "malformed hunk header",
        })?;
    let mut hunk = Hunk {
        old_start,
        old_lines,
        new_start,
        new_lines,
        heading,
        lines: Vec::new(),
    };
    let next = fill_hunk_body(lines, start + 1, &mut hunk)?;
    Ok((hunk, next))
}

/// Reads a hunk's body lines into `hunk.lines`, starting at `lines[start]`,
/// until its header's declared old/new line counts are both reached.
///
/// Returns the index of the first unread line.
fn fill_hunk_body(lines: &[&[u8]], start: usize, hunk: &mut Hunk) -> Result<usize, ParseError> {
    let mut old_no = hunk.old_start;
    let mut new_no = hunk.new_start;
    let (mut old_count, mut new_count) = (0u32, 0u32);
    let mut i = start;
    while old_count < hunk.old_lines || new_count < hunk.new_lines {
        let err = |what| ParseError { line: i + 1, what };
        let line = *lines.get(i).ok_or(ParseError {
            line: i,
            what: "hunk ended before its declared line counts were reached",
        })?;
        match line.first() {
            Some(b' ') => {
                if old_count >= hunk.old_lines || new_count >= hunk.new_lines {
                    return Err(err("hunk has more context lines than its header declared"));
                }
                hunk.lines.push(new_line(
                    LineKind::Context,
                    &line[1..],
                    Some(old_no),
                    Some(new_no),
                ));
                old_no += 1;
                new_no += 1;
                old_count += 1;
                new_count += 1;
            }
            Some(b'+') => {
                if new_count >= hunk.new_lines {
                    return Err(err("hunk has more added lines than its header declared"));
                }
                hunk.lines
                    .push(new_line(LineKind::Added, &line[1..], None, Some(new_no)));
                new_no += 1;
                new_count += 1;
            }
            Some(b'-') => {
                if old_count >= hunk.old_lines {
                    return Err(err("hunk has more removed lines than its header declared"));
                }
                hunk.lines
                    .push(new_line(LineKind::Removed, &line[1..], Some(old_no), None));
                old_no += 1;
                old_count += 1;
            }
            Some(b'\\') if line == b"\\ No newline at end of file" => match hunk.lines.last_mut() {
                Some(last) => last.no_newline_at_eof = true,
                None => return Err(err("`\\ No newline` before any hunk line")),
            },
            _ => return Err(err("unexpected line inside a hunk")),
        }
        i += 1;
    }
    consume_trailing_no_newline_marker(lines, i, hunk)
}

/// A `\ No newline at end of file` marker for the hunk's very last line sits
/// right after it, but isn't itself counted in the header's old/new line
/// totals — so it can only be found by peeking past them, once the main body
/// loop in [`fill_hunk_body`] is done.
fn consume_trailing_no_newline_marker(
    lines: &[&[u8]],
    i: usize,
    hunk: &mut Hunk,
) -> Result<usize, ParseError> {
    if lines.get(i) != Some(&b"\\ No newline at end of file".as_slice()) {
        return Ok(i);
    }
    match hunk.lines.last_mut() {
        Some(last) => {
            last.no_newline_at_eof = true;
            Ok(i + 1)
        }
        None => Err(ParseError {
            line: i + 1,
            what: "`\\ No newline` before any hunk line",
        }),
    }
}

fn new_line(kind: LineKind, text: &[u8], old_no: Option<u32>, new_no: Option<u32>) -> DiffLine {
    DiffLine {
        kind,
        text: text.to_vec(),
        old_no,
        new_no,
        no_newline_at_eof: false,
    }
}

/// Splits `@@ -a,b +c,d @@ heading` into its four numbers and the heading
/// text (which may itself contain `@@`, so only the *first* ` @@` after the
/// ranges closes the header).
fn parse_hunk_header(header: &[u8]) -> Option<(u32, u32, u32, u32, Vec<u8>)> {
    let rest = header.strip_prefix(b"@@ -")?;
    let close = find(rest, b" @@")?;
    let ranges = &rest[..close];
    let after = &rest[close + 3..];
    let heading = after.strip_prefix(b" ").unwrap_or(after).to_vec();

    let space = ranges.iter().position(|&b| b == b' ')?;
    let (old_start, old_lines) = parse_range(&ranges[..space])?;
    let (new_start, new_lines) = parse_range(ranges[space + 1..].strip_prefix(b"+")?)?;
    Some((old_start, old_lines, new_start, new_lines, heading))
}

/// Splits `bytes` into lines on `\n`, dropping it; a trailing `\r` stays.
/// An input with no trailing `\n` still yields its last, partial line.
fn split_lines(bytes: &[u8]) -> Vec<&[u8]> {
    if bytes.is_empty() {
        return Vec::new();
    }
    let mut lines: Vec<&[u8]> = bytes.split(|&b| b == b'\n').collect();
    // `bytes.split` on a trailing `\n` yields one empty slice past it; that
    // is not a line.
    if bytes.ends_with(b"\n") {
        lines.pop();
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::primitives::unquote;
    use super::*;

    #[test]
    fn unquotes_octal_escapes() {
        assert_eq!(unquote(b"\"a/\\303\\251\""), vec![0x61, 0x2f, 0xc3, 0xa9]);
    }

    #[test]
    fn unquotes_plain_escapes() {
        assert_eq!(unquote(b"\"a\\tb\\\\c\\\"d\""), b"a\tb\\c\"d");
    }

    #[test]
    fn split_lines_drops_only_the_final_newline() {
        assert_eq!(
            split_lines(b"a\nb\n"),
            vec![b"a".as_slice(), b"b".as_slice()]
        );
        assert_eq!(split_lines(b"a\nb"), vec![b"a".as_slice(), b"b".as_slice()]);
        assert_eq!(split_lines(b""), Vec::<&[u8]>::new());
    }

    #[test]
    fn parses_a_simple_hunk_header() {
        let (os, ol, ns, nl, heading) = parse_hunk_header(b"@@ -1,3 +1,4 @@ fn main() {").unwrap();
        assert_eq!((os, ol, ns, nl), (1, 3, 1, 4));
        assert_eq!(heading, b"fn main() {");
    }

    #[test]
    fn parses_a_hunk_header_with_at_at_in_the_heading() {
        let (.., heading) = parse_hunk_header(b"@@ -1,3 +1,4 @@ a @@ b").unwrap();
        assert_eq!(heading, b"a @@ b");
    }

    #[test]
    fn parses_a_hunk_header_without_a_comma() {
        let (os, ol, ns, nl, heading) = parse_hunk_header(b"@@ -1 +1 @@").unwrap();
        assert_eq!((os, ol, ns, nl), (1, 1, 1, 1));
        assert_eq!(heading, b"");
    }

    #[test]
    fn rejects_a_malformed_hunk_header() {
        assert!(parse_hunk_header(b"@@ -a,3 +1,4 @@").is_none());
        assert!(parse_hunk_header(b"not a hunk header").is_none());
    }
}
