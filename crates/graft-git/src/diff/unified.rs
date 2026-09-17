//! `parse_unified`: turns the bytes of `git diff-tree -p -M -C --no-color
//! --no-ext-diff` (one or many files) into [`FileDiff`]s.
//!
//! Pure byte parsing, no `git` spawn — git's own output is the oracle
//! (fixture round-trip test) and this is also a `cargo-fuzz` target, so it
//! must never panic on arbitrary bytes.

use super::{BinaryNote, DiffLine, FileDiff, FileEntry, FileStatus, Hunk, LineKind, ParseError};

/// Parses one or many files' worth of unified diff output.
///
/// See the module doc for what this accepts. A line git could not have
/// produced from this command fails with [`ParseError`], naming the
/// 1-indexed input line.
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
            // Before the first file, `diff-tree` prints the commit id on its
            // own line; nothing else here is ours to fail on.
            i += 1;
        }
    }
    Ok(out)
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
            // An unrecognised header line before the first hunk (`index
            // ...`, or anything future git versions add): kept out of the
            // hunk list, never fails the parse.
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

fn parse_range(r: &[u8]) -> Option<(u32, u32)> {
    match r.iter().position(|&b| b == b',') {
        Some(comma) => Some((parse_u32(&r[..comma])?, parse_u32(&r[comma + 1..])?)),
        None => Some((parse_u32(r)?, 1)),
    }
}

fn parse_u32(s: &[u8]) -> Option<u32> {
    if s.is_empty() || !s.iter().all(u8::is_ascii_digit) {
        return None;
    }
    std::str::from_utf8(s).ok()?.parse().ok()
}

fn parse_mode(rest: &[u8]) -> Option<u32> {
    parse_u32(rest)
}

fn parse_percent(rest: &[u8]) -> Option<u8> {
    let digits = rest.strip_suffix(b"%")?;
    if digits.is_empty() || !digits.iter().all(u8::is_ascii_digit) {
        return None;
    }
    std::str::from_utf8(digits).ok()?.parse().ok()
}

/// The `diff --git a/x b/x` line's two paths, `a/`/`b/` prefixes stripped.
///
/// Git quotes both paths together, C-style, whenever either needs it; when
/// neither is quoted, a path containing a literal `" b/"` is ambiguous (a
/// long-known git wart) and is not something this parser resolves — the
/// `---`/`+++`/`rename to`/`copy to` lines are the unambiguous source of the
/// final path whenever they are present.
fn parse_diff_git_header(rest: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    if rest.first() == Some(&b'"') {
        let (a, after) = take_quoted(rest)?;
        let after = after.strip_prefix(b" ")?;
        let b = if after.first() == Some(&b'"') {
            take_quoted(after)?.0
        } else {
            after
        };
        Some((strip_ab(&unquote(a)), strip_ab(&unquote(b))))
    } else {
        let split = find(rest, b" b/")?;
        Some((strip_ab(&rest[..split]), strip_ab(&rest[split + 1..])))
    }
}

/// `--- `/`+++ ` line's path: `None` for `/dev/null`, else unquoted with its
/// `a/`/`b/` prefix stripped.
fn parse_side_path(rest: &[u8]) -> Option<Vec<u8>> {
    if rest == b"/dev/null" {
        None
    } else {
        Some(strip_ab(&unquote_maybe(rest)))
    }
}

/// `rename from`/`rename to`/`copy from`/`copy to` line's path: unquoted,
/// with no `a/`/`b/` prefix (git never adds one to these).
fn unquote_maybe(rest: &[u8]) -> Vec<u8> {
    if rest.first() == Some(&b'"') {
        unquote(rest)
    } else {
        rest.to_vec()
    }
}

fn strip_ab(path: &[u8]) -> Vec<u8> {
    if path.starts_with(b"a/") || path.starts_with(b"b/") {
        path[2..].to_vec()
    } else {
        path.to_vec()
    }
}

/// Finds the first index of `needle` in `hay`.
fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// Takes one C-quoted token (`"..."`, with `\\`-escapes) from the start of
/// `s`. Returns the token including its quotes, and the rest of `s`.
fn take_quoted(s: &[u8]) -> Option<(&[u8], &[u8])> {
    if s.first() != Some(&b'"') {
        return None;
    }
    let mut i = 1;
    while i < s.len() {
        match s[i] {
            b'\\' => i += 2,
            b'"' => return Some((&s[..=i], &s[i + 1..])),
            _ => i += 1,
        }
    }
    None
}

/// Undoes git's C-style path quoting: `\"`, `\\`, the usual C escapes, and
/// `\NNN` octal byte escapes (how a non-UTF-8 or non-ASCII byte is spelled).
/// `token` includes its surrounding quotes; a token that is not quoted is
/// returned as-is.
fn unquote(token: &[u8]) -> Vec<u8> {
    let Some(inner) = token
        .strip_prefix(b"\"")
        .and_then(|s| s.strip_suffix(b"\""))
    else {
        return token.to_vec();
    };
    let mut out = Vec::with_capacity(inner.len());
    let mut i = 0;
    while i < inner.len() {
        if inner[i] != b'\\' || i + 1 >= inner.len() {
            out.push(inner[i]);
            i += 1;
            continue;
        }
        let escape = inner[i + 1];
        if escape.is_ascii_digit() {
            let mut value: u32 = 0;
            let mut j = i + 1;
            let mut digits = 0;
            while digits < 3 && j < inner.len() && inner[j].is_ascii_digit() {
                value = value * 8 + u32::from(inner[j] - b'0');
                j += 1;
                digits += 1;
            }
            // A 3-digit octal escape can spell 0-511; a value past 255 is
            // not a byte git could have written, but the parser never
            // panics on it (this is a `cargo-fuzz` target) — it saturates.
            out.push(u8::try_from(value).unwrap_or(u8::MAX));
            i = j;
            continue;
        }
        let literal = match escape {
            b'"' => b'"',
            b'\\' => b'\\',
            b'a' => 0x07,
            b'b' => 0x08,
            b'f' => 0x0c,
            b'n' => b'\n',
            b'r' => b'\r',
            b't' => b'\t',
            b'v' => 0x0b,
            other => other,
        };
        out.push(literal);
        i += 2;
    }
    out
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
