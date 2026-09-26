//! `git log -L` and `git log --follow`, run and parsed.

use std::path::{Path, PathBuf};

use graft_core::oid::Oid;

use super::{LineRange, TraceCommit, TraceEnd, TraceError};
use crate::blame::{bytes_path, path_bytes, unquote};
use crate::diff::{LineKind, ParseError, parse_unified};
use crate::repo::RepoHandle;
use crate::run::{GitArg, GitInvocation, RawOutput, RunError, Stop};

/// Each commit's header: a `\x01` to find it by, then the fields
/// NUL-separated. `-L` ends it with a newline; `-z` with a NUL.
const FORMAT: &str = "--format=%x01%H%x00%an%x00%at%x00%s";

/// The flags every trace passes, so user config cannot change the shape
/// parsed: no colour, no external diff, no signature block, `a/`/`b/`
/// prefixes whatever `diff.noprefix` or `diff.mnemonicPrefix` say.
const PLAIN: [&str; 5] = [
    "--no-color",
    "--no-ext-diff",
    "--no-show-signature",
    "--src-prefix=a/",
    "--dst-prefix=b/",
];

/// One parsed commit and whether it started what is traced (added the whole
/// range, or added the file).
type Record = (TraceCommit, bool);

/// Every commit that changed `range` of `path`, newest first, as `git log
/// -L<start>,<end>:<path> <at>` finds them; `range` is as of `at`. At most
/// `cap` rows reach `sink`, in one batch (see the module doc).
///
/// # Errors
/// [`TraceError`]: git could not run, or its log did not parse. git exiting
/// with a message is not an error: it is [`TraceEnd::GitStops`].
pub fn line_history(
    repo: &RepoHandle,
    at: Oid,
    path: &Path,
    range: &LineRange,
    cap: usize,
    sink: &mut dyn FnMut(Vec<TraceCommit>),
    stop: &dyn Stop,
) -> Result<TraceEnd, TraceError> {
    let (start, end) = range.lines();
    let mut spec = format!("{start},{end}:").into_bytes();
    spec.extend(path_bytes(path));
    let raw = GitInvocation::new(repo)
        .arg(GitArg::Flag("log"))
        .args(PLAIN.map(GitArg::Flag))
        .args([
            GitArg::Flag(FORMAT),
            GitArg::Word(format!("--max-count={}", cap.saturating_add(1))),
            GitArg::Glued("-L", spec),
            GitArg::Oid(at),
        ])
        .run_raw(stop);
    finish(
        raw,
        cap,
        sink,
        parse_line_records,
        "no commit changed these lines",
    )
}

/// Every commit that changed `path`, following renames, newest first, as
/// `git log --follow <at> -- <path>` finds them. At most `cap` rows reach
/// `sink`, in one batch.
///
/// # Errors
/// As [`line_history`].
pub fn file_history(
    repo: &RepoHandle,
    at: Oid,
    path: &Path,
    cap: usize,
    sink: &mut dyn FnMut(Vec<TraceCommit>),
    stop: &dyn Stop,
) -> Result<TraceEnd, TraceError> {
    let raw = GitInvocation::new(repo)
        .args([
            GitArg::Flag("log"),
            GitArg::Flag("--follow"),
            GitArg::Flag("--raw"),
            GitArg::Flag("--numstat"),
            GitArg::Flag("-z"),
        ])
        .args(PLAIN.map(GitArg::Flag))
        .args([
            GitArg::Flag(FORMAT),
            GitArg::Word(format!("--max-count={}", cap.saturating_add(1))),
            GitArg::Oid(at),
            GitArg::Separator,
            GitArg::Path(path_bytes(path)),
        ])
        .run_raw(stop);
    finish(
        raw,
        cap,
        sink,
        parse_follow_records,
        "no commit changed this file",
    )
}

/// Turns a finished log run into rows for `sink` and a [`TraceEnd`].
fn finish(
    raw: Result<RawOutput, RunError>,
    cap: usize,
    sink: &mut dyn FnMut(Vec<TraceCommit>),
    parse: fn(&[u8]) -> Result<Vec<Record>, ParseError>,
    nothing: &str,
) -> Result<TraceEnd, TraceError> {
    let raw = match raw {
        Ok(raw) => raw,
        Err(RunError::Stopped) => return Ok(TraceEnd::Stopped),
        Err(other) => return Err(other.into()),
    };
    if raw.output.truncated {
        return Err(TraceError::Run(RunError::Unparsable(
            "git log's output passed the runner's 64 MiB cap".to_owned(),
        )));
    }
    let Some(code) = raw.code else {
        // Only the runner kills without a code, and it has no cap left to
        // hit (checked above): the stop.
        return Ok(TraceEnd::Stopped);
    };
    if code != 0 {
        let reason = crate::commit_graph::first_line(&raw.output.stderr);
        let reason = reason.strip_prefix("fatal: ").unwrap_or(&reason).to_owned();
        // git failed: what it printed before failing is sent if it parses.
        // A half-written record that does not is dropped, safely: git's own
        // reason, returned below, is the answer the screen shows.
        if let Ok(records) = parse(&raw.output.stdout) {
            send(
                records.into_iter().take(cap).map(|(row, _)| row).collect(),
                sink,
            );
        }
        return Ok(TraceEnd::GitStops(reason));
    }
    let mut records = parse(&raw.output.stdout).map_err(TraceError::Parse)?;
    if records.len() > cap {
        records.truncate(cap);
        send(records.into_iter().map(|(row, _)| row).collect(), sink);
        return Ok(TraceEnd::Capped { shown: cap });
    }
    let end = match records.last() {
        Some((row, true)) => TraceEnd::Start { added_by: row.id },
        Some((_, false)) => TraceEnd::GitStops(
            "the history ends before this was added (a shallow clone?)".to_owned(),
        ),
        None => TraceEnd::GitStops(nothing.to_owned()),
    };
    send(records.into_iter().map(|(row, _)| row).collect(), sink);
    Ok(end)
}

fn send(rows: Vec<TraceCommit>, sink: &mut dyn FnMut(Vec<TraceCommit>)) {
    if !rows.is_empty() {
        sink(rows);
    }
}

/// Parses `git log -L… --format=%x01%H%x00%an%x00%at%x00%s` output: each
/// commit's header line, then its `-L` diff. Pure, no spawn; a `cargo-fuzz`
/// target.
///
/// # Errors
/// [`ParseError`] naming the 1-indexed line where the shape broke.
pub fn parse_line_log(bytes: &[u8]) -> Result<Vec<TraceCommit>, ParseError> {
    Ok(parse_line_records(bytes)?
        .into_iter()
        .map(|(row, _)| row)
        .collect())
}

/// Parses `git log --follow --raw --numstat -z
/// --format=%x01%H%x00%an%x00%at%x00%s` output. Pure, no spawn; a
/// `cargo-fuzz` target.
///
/// # Errors
/// [`ParseError`]; `line` is the 1-indexed commit record.
pub fn parse_follow_log(bytes: &[u8]) -> Result<Vec<TraceCommit>, ParseError> {
    Ok(parse_follow_records(bytes)?
        .into_iter()
        .map(|(row, _)| row)
        .collect())
}

/// The input split at each `\x01`, with the 1-indexed line each record's
/// header is on. Anything before the first `\x01` must be blank.
fn records(bytes: &[u8]) -> Result<Vec<(usize, &[u8])>, ParseError> {
    let mut chunks = bytes.split(|&b| b == 0x01);
    // `split` always yields at least one piece (the whole input when there
    // is no `\x01`), so the default never stands in for a failure.
    let lead = chunks.next().unwrap_or_default();
    if !lead.iter().all(u8::is_ascii_whitespace) {
        return Err(ParseError {
            line: 1,
            what: "expected a commit record first",
        });
    }
    let mut line = 1 + newlines(lead);
    let mut out = Vec::new();
    for chunk in chunks {
        out.push((line, chunk));
        line += newlines(chunk);
    }
    Ok(out)
}

/// How many `\n` bytes `bytes` holds.
fn newlines(bytes: &[u8]) -> usize {
    bytes
        .iter()
        .fold(0, |count, &b| count + usize::from(b == b'\n'))
}

/// One commit's `%H\0%an\0%at\0%s` fields; the subject is everything after
/// the third NUL.
struct Header {
    id: Oid,
    author: String,
    when: i64,
    subject: String,
}

fn parse_header(fields: &[u8], line: usize) -> Result<Header, ParseError> {
    let mut parts = fields.splitn(4, |&b| b == 0);
    let mut next = |what| parts.next().ok_or(ParseError { line, what });
    let id = next("missing the commit id")?;
    let author = next("missing the author")?;
    let when = next("missing the author time")?;
    let subject = next("missing the subject")?;
    Ok(Header {
        id: std::str::from_utf8(id)
            .ok()
            .and_then(|hex| Oid::from_hex(hex).ok())
            .ok_or(ParseError {
                line,
                what: "the commit id is not valid hex",
            })?,
        author: String::from_utf8_lossy(author).into_owned(),
        when: std::str::from_utf8(when)
            .ok()
            .and_then(|text| text.parse().ok())
            .ok_or(ParseError {
                line,
                what: "the author time is not a number",
            })?,
        subject: String::from_utf8_lossy(subject).into_owned(),
    })
}

fn parse_line_records(bytes: &[u8]) -> Result<Vec<Record>, ParseError> {
    let mut out = Vec::new();
    for (line, chunk) in records(bytes)? {
        let (header, diff) = match chunk.iter().position(|&b| b == b'\n') {
            Some(newline) => (&chunk[..newline], &chunk[newline + 1..]),
            None => (chunk, &[][..]),
        };
        let header = parse_header(header, line)?;
        let mut row = TraceCommit {
            id: header.id,
            author: header.author,
            when: header.when,
            subject: header.subject,
            path: PathBuf::new(),
            renamed_from: None,
            added: 0,
            removed: 0,
            range_at: None,
        };
        let mut started = false;
        if let Some(at) = find(diff, b"diff --git ") {
            let diff = &diff[at..];
            let skipped = diff_line_offset(chunk, diff);
            let files = parse_unified(diff).map_err(|error| ParseError {
                line: line + skipped + error.line - 1,
                what: error.what,
            })?;
            if let Some(file) = files.first() {
                row.path = bytes_path(&file.entry.path);
                row.renamed_from = file
                    .entry
                    .old_path
                    .clone()
                    .or_else(|| side_path(diff, b"--- ").filter(|old| *old != file.entry.path))
                    .map(|old| bytes_path(&old));
                let lines = file.hunks.iter().flat_map(|hunk| &hunk.lines);
                row.added = count(lines.clone().filter(|l| l.kind == LineKind::Added));
                row.removed = count(lines.filter(|l| l.kind == LineKind::Removed));
                let starts = file.hunks.iter().filter(|h| h.new_lines > 0);
                let first = starts.clone().map(|h| h.new_start).min();
                // `new_lines > 0` here; saturating, since the numbers are git's
                // (or a fuzzer's) and a sum past `u32::MAX` must not panic.
                let last = starts
                    .map(|h| h.new_start.saturating_add(h.new_lines - 1))
                    .max();
                row.range_at = first.zip(last);
                started = !file.hunks.is_empty() && file.hunks.iter().all(|h| h.old_lines == 0);
            }
        }
        out.push((row, started));
    }
    Ok(out)
}

/// How many `\n` precede `inner` within `outer` (a subslice of it).
fn diff_line_offset(outer: &[u8], inner: &[u8]) -> usize {
    let before = (inner.as_ptr() as usize).saturating_sub(outer.as_ptr() as usize);
    newlines(&outer[..before.min(outer.len())])
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// The path on a `--- a/<path>` line, `a/` stripped, C-quoting undone;
/// `None` for `/dev/null` or no such line.
fn side_path(diff: &[u8], prefix: &[u8]) -> Option<Vec<u8>> {
    let line = diff
        .split(|&b| b == b'\n')
        .take_while(|line| !line.starts_with(b"@@"))
        .find_map(|line| line.strip_prefix(prefix))?;
    let path = unquote(line);
    path.strip_prefix(b"a/").map(<[u8]>::to_vec)
}

fn count<'a>(lines: impl Iterator<Item = &'a crate::diff::DiffLine>) -> u32 {
    u32::try_from(lines.count()).unwrap_or(u32::MAX)
}

fn parse_follow_records(bytes: &[u8]) -> Result<Vec<Record>, ParseError> {
    let mut out = Vec::new();
    for (record, (_, chunk)) in records(bytes)?.into_iter().enumerate() {
        let line = record + 1;
        // The header's four fields, then the raw and numstat records, all
        // NUL-terminated; a newline opens the first raw record.
        let mut fields = chunk.splitn(5, |&b| b == 0);
        let header: Vec<&[u8]> = fields.by_ref().take(4).collect();
        if header.len() < 4 {
            return Err(ParseError {
                line,
                what: "a commit record has fewer than four header fields",
            });
        }
        let header = parse_header(&header.join(&0u8), line)?;
        let rest = fields.next().unwrap_or_default();
        let files = parse_follow_files(rest, line)?;
        out.push((
            TraceCommit {
                id: header.id,
                author: header.author,
                when: header.when,
                subject: header.subject,
                path: files.path.map(|p| bytes_path(&p)).unwrap_or_default(),
                renamed_from: files.renamed_from.map(|p| bytes_path(&p)),
                added: files.added,
                removed: files.removed,
                range_at: None,
            },
            files.added_file,
        ));
    }
    Ok(out)
}

/// What one commit's `--raw --numstat -z` records say about the file.
#[derive(Default)]
struct FollowFiles {
    path: Option<Vec<u8>>,
    renamed_from: Option<Vec<u8>>,
    added: u32,
    removed: u32,
    added_file: bool,
}

fn parse_follow_files(rest: &[u8], line: usize) -> Result<FollowFiles, ParseError> {
    let mut files = FollowFiles::default();
    let mut tokens = rest.split(|&b| b == 0).map(<[u8]>::trim_ascii_start);
    let err = |what| ParseError { line, what };
    while let Some(token) = tokens.next() {
        if token.is_empty() {
            continue;
        }
        if let Some(raw) = token.strip_prefix(b":") {
            let status = raw
                .rsplit(|&b| b == b' ')
                .next()
                .and_then(|status| status.first().copied())
                .ok_or(err("a raw record has no status"))?;
            let first = tokens.next().ok_or(err("a raw record has no path"))?;
            match status {
                b'R' | b'C' => {
                    let second = tokens.next().ok_or(err("a rename has no new path"))?;
                    if status == b'R' {
                        files.renamed_from = Some(first.to_vec());
                    }
                    files.path = Some(second.to_vec());
                }
                b'A' => {
                    files.added_file = true;
                    files.path = Some(first.to_vec());
                }
                _ => files.path = Some(first.to_vec()),
            }
            continue;
        }
        let mut numstat = token.splitn(3, |&b| b == b'\t');
        let (Some(added), Some(removed), Some(path)) =
            (numstat.next(), numstat.next(), numstat.next())
        else {
            return Err(err("expected a raw or numstat record"));
        };
        files.added = files.added.saturating_add(stat(added, line)?);
        files.removed = files.removed.saturating_add(stat(removed, line)?);
        if path.is_empty() {
            // A rename: the old and new paths follow as their own tokens.
            tokens
                .next()
                .ok_or(err("a numstat rename has no old path"))?;
            let new = tokens
                .next()
                .ok_or(err("a numstat rename has no new path"))?;
            files.path.get_or_insert_with(|| new.to_vec());
        } else {
            files.path.get_or_insert_with(|| path.to_vec());
        }
    }
    Ok(files)
}

/// A numstat count; `-` (binary) is 0.
fn stat(field: &[u8], line: usize) -> Result<u32, ParseError> {
    if field == b"-" {
        return Ok(0);
    }
    std::str::from_utf8(field)
        .ok()
        .and_then(|text| text.parse().ok())
        .ok_or(ParseError {
            line,
            what: "a numstat count is not a number",
        })
}
