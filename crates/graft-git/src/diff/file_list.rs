//! `parse_file_list`: joins `git diff-tree --raw -z -M -C -r` with
//! `git diff-tree --numstat -z -M -C -r` into one [`FileList`].
//!
//! Both commands walk the same file set in the same tree order (same
//! arguments, same commit), so the two streams are joined positionally, not
//! by matching paths — a mismatch in either the counts or the final path is
//! a [`ParseError`], naming the record index it was found at (`-z` output
//! has no lines to point to).

use super::{FileEntry, FileList, FileStatus, ParseError};

/// Parses `raw_z` and `numstat_z` (both `-z`-terminated) into one
/// [`FileList`].
pub fn parse_file_list(raw_z: &[u8], numstat_z: &[u8]) -> Result<FileList, ParseError> {
    let raw = parse_raw(raw_z)?;
    let counts = parse_numstat(numstat_z)?;
    if raw.len() != counts.len() {
        return Err(ParseError {
            line: raw.len().min(counts.len()),
            what: "raw and numstat reported different numbers of files",
        });
    }

    let mut files = Vec::with_capacity(raw.len());
    let (mut added, mut removed) = (0u64, 0u64);
    for (index, (entry, count)) in raw.into_iter().zip(counts).enumerate() {
        if entry.path != count.path {
            return Err(ParseError {
                line: index,
                what: "raw and numstat disagree on a file's path",
            });
        }
        if let Some(n) = count.added {
            added += u64::from(n);
        }
        if let Some(n) = count.removed {
            removed += u64::from(n);
        }
        files.push(FileEntry {
            path: entry.path,
            old_path: entry.old_path,
            status: entry.status,
            added: count.added,
            removed: count.removed,
        });
    }
    Ok(FileList {
        files,
        added,
        removed,
    })
}

/// One `--raw -z` record, before its counts arrive from `--numstat -z`.
struct RawEntry {
    path: Vec<u8>,
    old_path: Option<Vec<u8>>,
    status: FileStatus,
}

/// One `--numstat -z` record: a path and its counts (`None`/`None` for a
/// binary file, which prints `-`/`-`).
struct NumstatEntry {
    path: Vec<u8>,
    added: Option<u32>,
    removed: Option<u32>,
}

fn parse_raw(raw_z: &[u8]) -> Result<Vec<RawEntry>, ParseError> {
    let tokens = split_nul(raw_z);
    let mut out = Vec::new();
    let mut i = 0;
    let mut record = 0;
    while i < tokens.len() {
        let header = tokens[i];
        let err = |what| ParseError { line: record, what };
        let fields = header
            .strip_prefix(b":")
            .ok_or_else(|| err("raw: record did not start with `:`"))?;
        // `:old_mode new_mode old_sha new_sha status`
        let status_field = fields
            .rsplit(|&b| b == b' ')
            .next()
            .ok_or_else(|| err("raw: header had no fields"))?;
        let (letter, similarity) =
            parse_status(status_field).ok_or_else(|| err("raw: unrecognised status"))?;
        i += 1;
        let (path, old_path) = if matches!(letter, b'R' | b'C') {
            let old = *tokens
                .get(i)
                .ok_or_else(|| err("raw: rename/copy missing its old path"))?;
            let new = *tokens
                .get(i + 1)
                .ok_or_else(|| err("raw: rename/copy missing its new path"))?;
            i += 2;
            (new.to_vec(), Some(old.to_vec()))
        } else {
            let path = *tokens
                .get(i)
                .ok_or_else(|| err("raw: record missing its path"))?;
            i += 1;
            (path.to_vec(), None)
        };
        let status = match letter {
            b'A' => FileStatus::Added,
            b'D' => FileStatus::Deleted,
            b'M' => FileStatus::Modified,
            b'T' => FileStatus::TypeChanged,
            b'U' => FileStatus::Unmerged,
            b'R' => FileStatus::Renamed {
                similarity: similarity.unwrap_or(100),
            },
            b'C' => FileStatus::Copied {
                similarity: similarity.unwrap_or(100),
            },
            _ => return Err(err("raw: unrecognised status")),
        };
        out.push(RawEntry {
            path,
            old_path,
            status,
        });
        record += 1;
    }
    Ok(out)
}

/// Splits a status field (`M`, `A`, `R100`, `C075`, ...) into its letter and,
/// for a rename or copy, the similarity percentage that follows it.
fn parse_status(field: &[u8]) -> Option<(u8, Option<u8>)> {
    let (&letter, digits) = field.split_first()?;
    if digits.is_empty() {
        return Some((letter, None));
    }
    if !digits.iter().all(u8::is_ascii_digit) {
        return None;
    }
    let similarity = std::str::from_utf8(digits).ok()?.parse().ok()?;
    Some((letter, Some(similarity)))
}

fn parse_numstat(numstat_z: &[u8]) -> Result<Vec<NumstatEntry>, ParseError> {
    let tokens = split_nul(numstat_z);
    let mut out = Vec::new();
    let mut i = 0;
    let mut record = 0;
    while i < tokens.len() {
        let head = tokens[i];
        i += 1;
        let err = |what| ParseError { line: record, what };
        let mut fields = head.splitn(3, |&b| b == b'\t');
        let added_field = fields
            .next()
            .ok_or_else(|| err("numstat: record missing its added count"))?;
        let removed_field = fields
            .next()
            .ok_or_else(|| err("numstat: record missing its removed count"))?;
        let path_field = fields
            .next()
            .ok_or_else(|| err("numstat: record missing its path"))?;
        let added = parse_count(added_field);
        let removed = parse_count(removed_field);
        let path = if path_field.is_empty() {
            // A rename/copy: the path is split into two more NUL-terminated
            // tokens instead.
            let old = *tokens
                .get(i)
                .ok_or_else(|| err("numstat: rename/copy missing its old path"))?;
            let new = *tokens
                .get(i + 1)
                .ok_or_else(|| err("numstat: rename/copy missing its new path"))?;
            i += 2;
            let _ = old; // the joined path used to key against `parse_raw` is the new path.
            new.to_vec()
        } else {
            path_field.to_vec()
        };
        out.push(NumstatEntry {
            path,
            added,
            removed,
        });
        record += 1;
    }
    Ok(out)
}

/// `-` (binary) is `None`; otherwise a decimal count.
fn parse_count(field: &[u8]) -> Option<u32> {
    if field == b"-" {
        return None;
    }
    std::str::from_utf8(field).ok()?.parse().ok()
}

/// Splits `bytes` on NUL, dropping the empty tail after the final
/// terminator.
fn split_nul(bytes: &[u8]) -> Vec<&[u8]> {
    if bytes.is_empty() {
        return Vec::new();
    }
    let mut tokens: Vec<&[u8]> = bytes.split(|&b| b == 0).collect();
    if bytes.ends_with(&[0]) {
        tokens.pop();
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_a_plain_modify() {
        let raw = b":100644 100644 aaa bbb M\0file.txt\0";
        let numstat = b"1\t2\tfile.txt\0";
        let list = parse_file_list(raw, numstat).unwrap();
        assert_eq!(list.files.len(), 1);
        assert_eq!(list.files[0].path, b"file.txt");
        assert_eq!(list.files[0].status, FileStatus::Modified);
        assert_eq!(list.files[0].added, Some(1));
        assert_eq!(list.files[0].removed, Some(2));
        assert_eq!(list.added, 1);
        assert_eq!(list.removed, 2);
    }

    #[test]
    fn joins_a_rename_with_similarity() {
        let raw = b":100644 100644 aaa bbb R090\0old.txt\0new.txt\0";
        let numstat = b"1\t0\t\0old.txt\0new.txt\0";
        let list = parse_file_list(raw, numstat).unwrap();
        assert_eq!(list.files[0].path, b"new.txt");
        assert_eq!(list.files[0].old_path, Some(b"old.txt".to_vec()));
        assert_eq!(list.files[0].status, FileStatus::Renamed { similarity: 90 });
        assert_eq!(list.files[0].added, Some(1));
    }

    #[test]
    fn binary_counts_are_none() {
        let raw = b":100644 100644 aaa bbb M\0img.png\0";
        let numstat = b"-\t-\timg.png\0";
        let list = parse_file_list(raw, numstat).unwrap();
        assert_eq!(list.files[0].added, None);
        assert_eq!(list.files[0].removed, None);
        assert_eq!(list.added, 0);
    }

    #[test]
    fn mismatched_counts_error() {
        let raw = b":100644 100644 aaa bbb M\0a\0:100644 100644 aaa bbb M\0b\0";
        let numstat = b"1\t0\ta\0";
        assert!(parse_file_list(raw, numstat).is_err());
    }
}
