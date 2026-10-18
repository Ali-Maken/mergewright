//! `git log --follow --raw --numstat -z` read into trace rows: the header
//! fields, then each commit's raw and numstat records.

use super::TraceCommit;
use super::log::{Record, parse_header, records};
use crate::blame::bytes_path;
use crate::diff::ParseError;

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

pub(super) fn parse_follow_records(bytes: &[u8]) -> Result<Vec<Record>, ParseError> {
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
