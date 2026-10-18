//! The pure reader of `git blame --porcelain` (and `--line-porcelain`)
//! output: header groups and text lines into [`BlameLine`]s and the
//! [`BlameCommit`]s they name. No spawn; a `cargo-fuzz` target.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mergewright_core::oid::Oid;

use super::{BlameCommit, BlameLine, LineMark, bytes_path};
use crate::diff::{ParseError, unquote};

/// The commit headers one porcelain group may carry, gathered until the
/// line's text.
#[derive(Default)]
struct Headers {
    author: Option<String>,
    author_time: Option<i64>,
    summary: Option<String>,
    path: Option<PathBuf>,
    previous: Option<(Oid, PathBuf)>,
    boundary: bool,
}

/// Parses `git blame --porcelain` (or `--line-porcelain`) output. Pure, no
/// spawn; a `cargo-fuzz` target.
///
/// Each line is `<id> <orig> <final>[ <count>]`, then header lines (the
/// commit's `author`…`summary`, `previous`, `boundary`, `filename` the first
/// time the commit appears; `ignored`/`unblamable` per line), then a tab and
/// the line's text. The all-zero id is a line not committed yet: its line has
/// `commit: None` and no entry in the map.
///
/// # Errors
/// [`ParseError`] naming the 1-indexed line where the shape broke.
pub fn parse_porcelain(
    bytes: &[u8],
) -> Result<(Vec<BlameLine>, BTreeMap<Oid, BlameCommit>), ParseError> {
    let mut lines = Vec::new();
    let mut commits: BTreeMap<Oid, BlameCommit> = BTreeMap::new();
    let mut input = bytes.split(|&b| b == b'\n').enumerate().peekable();
    while let Some((index, line)) = input.next() {
        if line.is_empty() && input.peek().is_none() {
            break; // the tail after the final newline
        }
        let at = index + 1;
        let (id, orig_line, final_line) = parse_group_line(line, at)?;
        let mut headers = Headers::default();
        let mut mark = LineMark::Plain;
        let text = loop {
            let Some((index, header)) = input.next() else {
                return Err(ParseError {
                    line: at,
                    what: "a blame entry ended before its text line",
                });
            };
            if let Some(text) = header.strip_prefix(b"\t") {
                break text.to_vec();
            }
            read_header(header, index + 1, &mut headers, &mut mark)?;
        };
        let commit = (!is_zero(&id)).then_some(id);
        if let Some(id) = commit {
            record_commit(&mut commits, id, headers, at)?;
        }
        lines.push(BlameLine {
            commit,
            orig_line,
            final_line,
            text,
            mark,
        });
    }
    Ok((lines, commits))
}

/// `<id> <orig> <final>[ <count>]`.
fn parse_group_line(line: &[u8], at: usize) -> Result<(Oid, u32, u32), ParseError> {
    let mut fields = line.split(|&b| b == b' ');
    let id = fields.next().and_then(oid).ok_or(ParseError {
        line: at,
        what: "expected `<id> <orig> <final>` to start a blame entry",
    })?;
    let mut number = || {
        fields
            .next()
            .and_then(|field| std::str::from_utf8(field).ok())
            .and_then(|field| field.parse::<u32>().ok())
            .ok_or(ParseError {
                line: at,
                what: "a blame entry's line numbers are missing or not numbers",
            })
    };
    let orig = number()?;
    let fin = number()?;
    Ok((id, orig, fin))
}

/// One header line into `headers`/`mark`; unknown keys are skipped (git adds
/// headers over time — `author-mail`, `committer-*` are not kept).
fn read_header(
    line: &[u8],
    at: usize,
    headers: &mut Headers,
    mark: &mut LineMark,
) -> Result<(), ParseError> {
    let (key, value) = match line.iter().position(|&b| b == b' ') {
        Some(space) => (&line[..space], &line[space + 1..]),
        None => (line, &[][..]),
    };
    match key {
        b"author" => headers.author = Some(lossy(value)),
        b"author-time" => {
            headers.author_time = Some(
                std::str::from_utf8(value)
                    .ok()
                    .and_then(|value| value.parse().ok())
                    .ok_or(ParseError {
                        line: at,
                        what: "`author-time` is not a number",
                    })?,
            );
        }
        b"summary" => headers.summary = Some(lossy(value)),
        b"filename" => headers.path = Some(bytes_path(&unquote(value))),
        b"boundary" => headers.boundary = true,
        // Both on one line (passed on by one ignored commit, stopped at
        // another) is unblamable, whichever git writes first: the line
        // stays on an ignored commit. `parse_marks` maps `*?` the same way.
        b"ignored" => {
            if *mark != LineMark::Unblamable {
                *mark = LineMark::Skipped;
            }
        }
        b"unblamable" => *mark = LineMark::Unblamable,
        b"previous" => {
            let space = value.iter().position(|&b| b == b' ').ok_or(ParseError {
                line: at,
                what: "`previous` has no path",
            })?;
            let id = oid(&value[..space]).ok_or(ParseError {
                line: at,
                what: "`previous` names no valid id",
            })?;
            headers.previous = Some((id, bytes_path(&unquote(&value[space + 1..]))));
        }
        _ => {}
    }
    Ok(())
}

/// Adds a commit the first time its headers appear; later entries for it
/// carry none and change nothing.
fn record_commit(
    commits: &mut BTreeMap<Oid, BlameCommit>,
    id: Oid,
    headers: Headers,
    at: usize,
) -> Result<(), ParseError> {
    if commits.contains_key(&id) {
        return Ok(());
    }
    let (Some(author), Some(author_time), Some(summary), Some(path)) = (
        headers.author,
        headers.author_time,
        headers.summary,
        headers.path,
    ) else {
        return Err(ParseError {
            line: at,
            what: "a commit's first entry lacks `author`, `author-time`, `summary` or `filename`",
        });
    };
    commits.insert(
        id,
        BlameCommit {
            id,
            author,
            author_time,
            summary,
            path,
            previous: headers.previous,
            boundary: headers.boundary,
            shallow: false,
        },
    );
    Ok(())
}

fn oid(hex: &[u8]) -> Option<Oid> {
    Oid::from_hex(std::str::from_utf8(hex).ok()?).ok()
}

fn is_zero(id: &Oid) -> bool {
    id.as_bytes().iter().all(|&b| b == 0)
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}
