//! Parsing `git status --porcelain=v2 -z`: one record at a time.

#[allow(
    clippy::wildcard_imports,
    reason = "one module split in files: its parent is its prelude"
)]
use super::*;

/// Parses `git status --porcelain=v2 -z`'s output. Pure: no `git` spawn, a
/// `cargo-fuzz` target (`fuzz/fuzz_targets/parse_porcelain_v2.rs`) — malformed
/// input is an `Err`, never a panic.
///
/// Handles the four record kinds `-z --untracked-files=all --renames`
/// produces: `1` (ordinary), `2` (rename/copy — its NUL-terminated original
/// path is the *next* record in the stream), `u` (unmerged) and `?`
/// (untracked). A `!` (ignored) record is skipped rather than rejected,
/// since a caller could in principle pass `--ignored` even though `read`
/// itself never does. Branch-header `#` records are rejected: `read` never
/// asks for `--branch`, so one appearing means the input is not what this
/// parser was built for.
///
/// # Errors
/// [`StatusError::Malformed`] for anything the format above does not cover.
pub fn parse_porcelain_v2(bytes: &[u8]) -> Result<WorkingTreeStatus, StatusError> {
    let mut entries = Vec::new();
    let mut records = bytes.split(|&b| b == 0);
    while let Some(record) = records.next() {
        if record.is_empty() {
            continue;
        }
        match record[0] {
            b'1' => entries.push(parse_ordinary(record)?),
            b'2' => {
                let mut entry = parse_rename(record)?;
                // An empty next piece reads the same as a missing one: a
                // real path is never empty, so this also catches a rename
                // that is the very last record with no origin following it
                // — the buffer's own trailing NUL would otherwise look like
                // one (a capped or truncated output ends exactly this way).
                let original = records
                    .next()
                    .filter(|piece| !piece.is_empty())
                    .ok_or_else(|| {
                        StatusError::Malformed(
                            "a rename record's original path is missing".to_owned(),
                        )
                    })?;
                entry.original_path = Some(bytes_to_pathbuf(original));
                entries.push(entry);
            }
            b'u' => entries.push(parse_unmerged(record)?),
            b'?' => entries.push(parse_untracked(record)?),
            b'!' => {}
            other => {
                return Err(StatusError::Malformed(format!(
                    "unrecognised porcelain v2 record kind {:?}",
                    other as char
                )));
            }
        }
    }
    Ok(WorkingTreeStatus { entries })
}

/// Splits one record into exactly `count` space-separated fields, the last
/// of which is everything remaining (a path may itself hold spaces).
fn fields(record: &[u8], count: usize) -> Result<Vec<&[u8]>, StatusError> {
    let parts: Vec<&[u8]> = record.splitn(count, |&b| b == b' ').collect();
    if parts.len() != count {
        return Err(StatusError::Malformed(format!(
            "expected {count} fields, found {}",
            parts.len()
        )));
    }
    Ok(parts)
}

/// `XY` is always two ASCII letters.
fn xy(field: &[u8]) -> Result<(u8, u8), StatusError> {
    match field {
        [x, y] => Ok((*x, *y)),
        _ => Err(StatusError::Malformed(
            "an XY field must be two characters".to_owned(),
        )),
    }
}

/// `1 XY sub mH mI mW hH hI path`
fn parse_ordinary(record: &[u8]) -> Result<StatusEntry, StatusError> {
    let f = fields(record, 9)?;
    let (x, y) = xy(f[1])?;
    Ok(StatusEntry {
        path: bytes_to_pathbuf(f[8]),
        original_path: None,
        staged: side_change(x)?,
        unstaged: side_change(y)?,
        untracked: false,
        unmerged: false,
        unmerged_how: None,
        submodule: submodule_status(f[2])?,
    })
}

/// `2 XY sub mH mI mW hH hI X<score> path` (original path is the next
/// record; the caller fills it in).
fn parse_rename(record: &[u8]) -> Result<StatusEntry, StatusError> {
    let f = fields(record, 10)?;
    let (x, y) = xy(f[1])?;
    Ok(StatusEntry {
        path: bytes_to_pathbuf(f[9]),
        original_path: None,
        staged: side_change(x)?,
        unstaged: side_change(y)?,
        untracked: false,
        unmerged: false,
        unmerged_how: None,
        submodule: submodule_status(f[2])?,
    })
}

/// `u XY sub m1 m2 m3 mW h1 h2 h3 path`
fn parse_unmerged(record: &[u8]) -> Result<StatusEntry, StatusError> {
    let f = fields(record, 11)?;
    // `XY` here is a conflict pairing (`UU`, `AA`, `DD`, `AU`, `UA`, `UD`,
    // `DU`), not a `Change`: a `U` would not parse as one. It is kept as
    // `UnmergedHow`, which the resolver reads to choose between a marker
    // file and a whole-file choice (m6-conflicts/01).
    let (x, y) = xy(f[1])?;
    let how = UnmergedHow::from_xy(x, y).ok_or_else(|| {
        StatusError::Malformed(format!(
            "an unmerged record's XY must be one of UU, AA, DD, AU, UA, DU, UD, not {:?}",
            String::from_utf8_lossy(f[1])
        ))
    })?;
    Ok(StatusEntry {
        path: bytes_to_pathbuf(f[10]),
        original_path: None,
        staged: None,
        unstaged: None,
        untracked: false,
        unmerged: true,
        unmerged_how: Some(how),
        submodule: submodule_status(f[2])?,
    })
}

/// `? path`
fn parse_untracked(record: &[u8]) -> Result<StatusEntry, StatusError> {
    let f = fields(record, 2)?;
    Ok(StatusEntry {
        path: bytes_to_pathbuf(f[1]),
        original_path: None,
        staged: None,
        unstaged: None,
        untracked: true,
        unmerged: false,
        unmerged_how: None,
        submodule: None,
    })
}
