//! The stage-tree codec (m6-conflicts/08, Decision 1): an unmerged index
//! written as one tree `{ 0/<path>, 1/<path>, 2/<path>, 3/<path> }`, and the
//! `(mode, oid, stage)` triples read back out of one.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use graft_core::journal::IndexState;
use graft_core::oid::Oid;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, Stop};

use super::plumbing::{TempIndex, path_of, run_git, split_nul, stopped, write_tree};
use super::{SnapshotError, StageEntry};

/// The stage tree of an unmerged index copy (decision 1).
///
/// `ls-files --stage` on the copy, every record rewritten to `<stage>/<path>`
/// at stage 0 on a **second**, empty temporary index, then `write-tree`. The
/// real index is never read or written by either step.
pub(super) fn stage_tree(
    repo: &RepoHandle,
    index: &Path,
    scratch: &Path,
    stop: &dyn Stop,
) -> Result<Oid, SnapshotError> {
    let listing = GitInvocation::new(repo)
        .env("GIT_INDEX_FILE", index)
        .arg(GitArg::Flag("ls-files"))
        .arg(GitArg::Flag("--stage"))
        .arg(GitArg::Flag("-z"))
        .run(stop)
        .map_err(stopped)?;
    let stdin = prefixed_stage_records(&listing.stdout)?;
    let staged = TempIndex::fresh(scratch)?;
    GitInvocation::new(repo)
        .env("GIT_INDEX_FILE", staged.path())
        .arg(GitArg::Flag("update-index"))
        .arg(GitArg::Flag("-z"))
        .arg(GitArg::Flag("--index-info"))
        .stdin(stdin)
        .run(stop)
        .map_err(stopped)?;
    write_tree(repo, staged.path(), stop)
}

/// `<mode> <oid> <stage>\t<path>\0` → `<mode> <oid> 0\t<stage>/<path>\0`.
///
/// Pure, so the bytes fed to `update-index --index-info` are testable without
/// a repository. A path is never parsed as text: it is moved whole, after the
/// one tab that separates it from the fixed-shape head.
fn prefixed_stage_records(listing: &[u8]) -> Result<Vec<u8>, SnapshotError> {
    let mut out = Vec::with_capacity(listing.len() + listing.len() / 8);
    for record in split_nul(listing) {
        let read = split_stage_record(&record, "ls-files --stage")?;
        out.extend_from_slice(read.mode);
        out.push(b' ');
        out.extend_from_slice(read.oid);
        out.extend_from_slice(b" 0\t");
        out.extend_from_slice(read.stage);
        out.push(b'/');
        out.extend_from_slice(read.path);
        out.push(0);
    }
    Ok(out)
}

/// One `ls-files --stage` record, in the bytes git printed: nothing is parsed
/// here, because the only use rewrites the path and hands it straight back.
struct StageRecord<'a> {
    mode: &'a [u8],
    oid: &'a [u8],
    stage: &'a [u8],
    path: &'a [u8],
}

/// `<mode> <oid> <stage>\t<path>` split into its four parts, unparsed.
fn split_stage_record<'a>(
    record: &'a [u8],
    what: &'static str,
) -> Result<StageRecord<'a>, SnapshotError> {
    let unreadable = || SnapshotError::NotAnOid {
        what,
        printed: String::from_utf8_lossy(record).into_owned(),
    };
    let tab = record
        .iter()
        .position(|byte| *byte == b'\t')
        .ok_or_else(unreadable)?;
    let (head, rest) = record.split_at(tab);
    let mut fields = head.split(|byte| *byte == b' ');
    let mode = fields.next().ok_or_else(unreadable)?;
    let oid = fields.next().ok_or_else(unreadable)?;
    let stage = fields.next().ok_or_else(unreadable)?;
    if fields.next().is_some() || mode.is_empty() || oid.is_empty() || stage.len() != 1 {
        return Err(unreadable());
    }
    Ok(StageRecord {
        mode,
        oid,
        stage,
        path: &rest[1..],
    })
}

/// Every path a recorded index state holds, with its entries, newest read
/// straight out of the tree. `only` narrows it to one path.
pub(super) fn tree_stage_entries(
    repo: &RepoHandle,
    index: IndexState,
    only: Option<&[PathBuf]>,
    stop: &dyn Stop,
) -> Result<BTreeMap<PathBuf, Vec<StageEntry>>, SnapshotError> {
    if only.is_some_and(<[PathBuf]>::is_empty) {
        return Ok(BTreeMap::new());
    }
    let mut args = vec![
        GitArg::Flag("--literal-pathspecs"),
        GitArg::Flag("ls-tree"),
        GitArg::Flag("-r"),
        GitArg::Flag("-z"),
        GitArg::Oid(index.tree()),
    ];
    if let Some(paths) = only {
        args.push(GitArg::Separator);
        for path in paths {
            if index.is_unmerged() {
                for stage in 0..4u8 {
                    args.push(GitArg::Path(stage_prefixed(stage, path)));
                }
            } else {
                args.push(GitArg::Path(path.as_os_str().as_encoded_bytes().to_vec()));
            }
        }
    }
    let output = run_git(repo, args, None, stop)?;
    let mut held: BTreeMap<PathBuf, Vec<StageEntry>> = BTreeMap::new();
    for record in split_nul(&output.stdout) {
        let entry = ls_tree_entry(&record, index.is_unmerged())?;
        held.entry(entry.0).or_default().push(entry.1);
    }
    for entries in held.values_mut() {
        // By stage, explicitly. `StageEntry`'s derived order is mode, then
        // oid, then stage — and three stages of one path share a mode, so a
        // plain sort orders them by blob id, which is to say at random.
        entries.sort_unstable_by_key(|entry| entry.stage);
    }
    Ok(held)
}

/// `<mode> <type> <oid>\t<name>` from `ls-tree -r -z`, as a path and one
/// entry. The stage comes from the name's first component for a stage tree,
/// and is 0 for a plain one.
pub(super) fn ls_tree_entry(
    record: &[u8],
    unmerged: bool,
) -> Result<(PathBuf, StageEntry), SnapshotError> {
    let unreadable = || SnapshotError::NotAnOid {
        what: "ls-tree",
        printed: String::from_utf8_lossy(record).into_owned(),
    };
    let tab = record
        .iter()
        .position(|byte| *byte == b'\t')
        .ok_or_else(unreadable)?;
    let (head, rest) = record.split_at(tab);
    let mut fields = head.split(|byte| *byte == b' ');
    let mode = fields.next().ok_or_else(unreadable)?;
    let _kind = fields.next().ok_or_else(unreadable)?;
    let oid = fields.next().ok_or_else(unreadable)?;
    let name = &rest[1..];
    let mode = std::str::from_utf8(mode)
        .ok()
        .and_then(|text| u32::from_str_radix(text, 8).ok())
        .ok_or_else(unreadable)?;
    let oid = std::str::from_utf8(oid)
        .ok()
        .and_then(|text| Oid::from_hex(text).ok())
        .ok_or_else(unreadable)?;
    let (stage, path) = if unmerged {
        let digit = name.first().copied().ok_or_else(unreadable)?;
        if !digit.is_ascii_digit() || name.get(1) != Some(&b'/') {
            return Err(unreadable());
        }
        (digit - b'0', strip_stage(name))
    } else {
        (0, name.to_vec())
    };
    if stage > 3 {
        return Err(unreadable());
    }
    Ok((path_of(path), StageEntry { mode, oid, stage }))
}

/// `<stage>/<path>` as the bytes a pathspec takes.
fn stage_prefixed(stage: u8, path: &Path) -> Vec<u8> {
    let mut name = vec![b'0' + stage, b'/'];
    name.extend_from_slice(path.as_os_str().as_encoded_bytes());
    name
}

/// `2/src/a.rs` → `src/a.rs`. Anything without a prefix is left alone: a
/// caller that asked for a stage tree and got a plain name has a bigger
/// problem than a mangled path, and mangling it would hide it.
pub(super) fn strip_stage(name: &[u8]) -> Vec<u8> {
    match name.get(1) {
        Some(b'/') if name[0].is_ascii_digit() => name[2..].to_vec(),
        _ => name.to_vec(),
    }
}
