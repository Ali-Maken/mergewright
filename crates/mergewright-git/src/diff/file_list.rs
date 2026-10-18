//! `parse_file_list`: joins `git diff-tree --raw -z -M -C -r` with
//! `git diff-tree --numstat -z -M -C -r` into one [`FileList`], and
//! [`parse_worktree_file_list`] does the same for `diff-index` against the
//! working tree.
//!
//! Both commands walk the same file set in the same tree order (same
//! arguments, same commit), so the two streams are walked together in step
//! rather than matched by path across the whole list; a disagreement is a
//! [`ParseError`], naming the record index it was found at (`-z` output has
//! no lines to point to). The one permitted gap is the working tree's
//! stat-dirty record, which [`parse_worktree_file_list`] documents.

use super::{FileEntry, FileList, FileStatus, ParseError};

/// Parses `raw_z` and `numstat_z` (both `-z`-terminated) into one
/// [`FileList`], keeping every record git reported.
pub fn parse_file_list(raw_z: &[u8], numstat_z: &[u8]) -> Result<FileList, ParseError> {
    join(raw_z, numstat_z, StatDirty::Keep)
}

/// The same, for `diff-index` against the working tree, dropping the records
/// that only exist because the index's stat cache is out of date.
///
/// `diff-index` is plumbing: it compares the index's cached mtime/size, so
/// a tracked file whose mtime moved without its content changing (a branch
/// switch, an editor save, a build) comes back in `--raw` as `M`, same mode
/// on both sides, with an all-zero "new" object id — the pane listed files
/// the user never touched.
///
/// `--numstat` is the tell: it is computed from content, after git's own
/// conversions, and it prints **no record at all** for such a file
/// (observed on git 2.54; `--raw` names it, `--numstat` does not). So the
/// two streams are joined by walking them together instead of
/// positionally, and a `--raw` record in that exact shape with no
/// `--numstat` record beside it is dropped: it has no content difference,
/// and nothing downstream could show one. Any other gap between the two
/// streams is still a [`ParseError`].
///
/// Nothing real is lost. A mode change keeps two differing modes and still
/// gets a `0 0` numstat record; a binary file gets `- -`; an added,
/// deleted, renamed or copied file is not `M`. The alternative — porcelain
/// `git diff`, which refreshes the stat cache itself — rewrites
/// `.git/index` and takes `index.lock` while it does, which a read must
/// never do (see [`crate::run`]).
///
/// # Errors
/// See [`ParseError`].
pub fn parse_worktree_file_list(raw_z: &[u8], numstat_z: &[u8]) -> Result<FileList, ParseError> {
    join(raw_z, numstat_z, StatDirty::Drop)
}

/// What [`join`] does with a `--raw` record that `--numstat` left out.
#[derive(Clone, Copy, PartialEq, Eq)]
enum StatDirty {
    /// Nothing may be missing: a tree-to-tree diff has no stat cache in it,
    /// so a gap is a parse error.
    Keep,
    /// A record shaped like the stat cache lying is dropped.
    Drop,
}

fn join(raw_z: &[u8], numstat_z: &[u8], stat_dirty: StatDirty) -> Result<FileList, ParseError> {
    let raw = parse_raw(raw_z)?;
    let counts = parse_numstat(numstat_z)?;

    let mut files = Vec::with_capacity(raw.len());
    let (mut added, mut removed) = (0u64, 0u64);
    let mut counts = counts.into_iter().peekable();
    for (index, entry) in raw.into_iter().enumerate() {
        let Some(count) = counts.next_if(|count| count.path == entry.path) else {
            if stat_dirty == StatDirty::Drop && entry.unchanged_content {
                continue;
            }
            return Err(ParseError {
                line: index,
                what: "raw and numstat disagree on a file's path",
            });
        };
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
    if counts.next().is_some() {
        return Err(ParseError {
            line: files.len(),
            what: "raw and numstat reported different numbers of files",
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
    /// `M`, the same mode on both sides, and an all-zero "new" object id:
    /// everything the header can say about a file whose content may be
    /// identical to what is committed. Only [`StatDirty::Drop`] reads it,
    /// and only for a record `--numstat` left out entirely.
    unchanged_content: bool,
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
        let (letter, similarity, unchanged_content) = parse_raw_header(header).map_err(err)?;
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
            unchanged_content,
        });
        record += 1;
    }
    Ok(out)
}

/// Reads one `--raw` header, `:old_mode new_mode old_sha new_sha status`,
/// into its status letter, a rename/copy's similarity, and whether the
/// record is shaped like the index's stat cache lying (see
/// [`RawEntry::unchanged_content`]).
fn parse_raw_header(header: &[u8]) -> Result<(u8, Option<u8>, bool), &'static str> {
    let fields = header
        .strip_prefix(b":")
        .ok_or("raw: record did not start with `:`")?;
    let mut parts = fields.split(|&b| b == b' ');
    let old_mode = parts.next();
    let new_mode = parts.next();
    let _old_sha = parts.next();
    let new_sha = parts.next();
    let status_field = fields
        .rsplit(|&b| b == b' ')
        .next()
        .ok_or("raw: header had no fields")?;
    let (letter, similarity) = parse_status(status_field).ok_or("raw: unrecognised status")?;
    let unchanged_content = letter == b'M'
        && old_mode == new_mode
        && new_sha.is_some_and(|sha| !sha.is_empty() && sha.iter().all(|&b| b == b'0'));
    Ok((letter, similarity, unchanged_content))
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

    /// The exact shape `diff-index` prints for a file the stat cache calls
    /// dirty: a `--raw` record that `--numstat` has no record for at all.
    /// The real change beside it must survive, with its counts.
    #[test]
    fn a_worktree_list_drops_a_stat_dirty_record() {
        let raw = b":100644 100644 aaa 0000000000000000000000000000000000000000 M\0f.txt\0\
                    :100644 100644 bbb 0000000000000000000000000000000000000000 M\0g.txt\0";
        let numstat = b"1\t0\tg.txt\0";
        let list = parse_worktree_file_list(raw, numstat).unwrap();
        assert_eq!(list.files.len(), 1);
        assert_eq!(list.files[0].path, b"g.txt");
        assert_eq!(list.added, 1);
        assert!(
            parse_file_list(raw, numstat).is_err(),
            "a tree diff has no stat cache, so the gap stays an error there"
        );
    }

    #[test]
    fn a_worktree_list_keeps_a_mode_change_with_no_line_change() {
        let raw = b":100644 100755 aaa 0000000000000000000000000000000000000000 M\0f.txt\0";
        let numstat = b"0\t0\tf.txt\0";
        let list = parse_worktree_file_list(raw, numstat).unwrap();
        assert_eq!(list.files.len(), 1, "the modes differ, so the file changed");
    }

    #[test]
    fn a_worktree_list_keeps_a_changed_binary_file() {
        let raw = b":100644 100644 aaa 0000000000000000000000000000000000000000 M\0img.png\0";
        let numstat = b"-\t-\timg.png\0";
        let list = parse_worktree_file_list(raw, numstat).unwrap();
        assert_eq!(list.files.len(), 1, "`- -` is binary, not `0 0`");
    }

    /// Only `M` with one mode and an all-zero new id may go missing: a
    /// deletion `--numstat` forgot is a parse error, not a file to hide.
    #[test]
    fn a_worktree_list_still_errors_when_a_real_record_has_no_counts() {
        let raw = b":100644 000000 aaa 0000000000000000000000000000000000000000 D\0gone.txt\0";
        assert!(parse_worktree_file_list(raw, b"").is_err());
    }

    #[test]
    fn mismatched_counts_error() {
        let raw = b":100644 100644 aaa bbb M\0a\0:100644 100644 aaa bbb M\0b\0";
        let numstat = b"1\t0\ta\0";
        assert!(parse_file_list(raw, numstat).is_err());
    }
}
