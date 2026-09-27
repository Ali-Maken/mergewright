//! git's reflogs as records: what moved `HEAD` and each local branch, when,
//! and git's own words for it (m7-power-tools/22, ADR-0046).
//!
//! Read through `gix`, no spawn. Only `HEAD` and `refs/heads/*`: a
//! remote-tracking ref's log is a fetch's business and a fetch loses nothing
//! (`docs/research/reflog.md` §4). A deleted branch's log goes with it — git
//! removes `logs/refs/heads/<b>` on `branch -D` — so what a deleted branch
//! did survives only where `HEAD` was on it.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use gix::bstr::BStr;
use graft_core::oid::Oid;

use crate::history::ReflogError;
use crate::repo::RepoHandle;
use crate::shallow::ShallowList;

/// One line of a reflog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReflogRecord {
    /// The log's ref: `HEAD` or `refs/heads/<b>`.
    pub refname: String,
    /// The value before; `None` for git's zero id (the ref was created).
    pub old: Option<Oid>,
    /// The value after; `None` for git's zero id.
    pub new: Option<Oid>,
    /// When, in seconds since the epoch (git's committer clock).
    pub at: i64,
    /// git's message, as written: `reset: moving to HEAD~1`.
    pub subject: String,
}

/// The newest `cap` records of `HEAD`'s log and every local branch's log,
/// newest first.
///
/// Newest first is by time, and within one log by the log's own order, so
/// records git wrote in the same second keep the order it wrote them in.
/// Across logs, a tie in time puts `HEAD`'s record first, then the branches'
/// by name. A ref with no log — a missing file, `core.logAllRefUpdates` off —
/// has no records; that is not an error.
///
/// # Errors
/// [`ReflogError`] if a log exists but cannot be read.
pub fn records(repo: &RepoHandle, cap: usize) -> Result<Vec<ReflogRecord>, ReflogError> {
    if cap == 0 {
        return Ok(Vec::new());
    }
    let gix_repo = repo.gix();
    let mut logs = Vec::new();
    match gix_repo.find_reference("HEAD") {
        Ok(head) => logs.push(newest_of(&head, cap)?),
        Err(gix::reference::find::existing::Error::NotFound { .. }) => {}
        Err(source) => return Err(ReflogError::Unreadable(source.to_string())),
    }
    let platform = gix_repo
        .references()
        .map_err(|source| ReflogError::Unreadable(source.to_string()))?;
    let branches = platform
        .local_branches()
        .map_err(|source| ReflogError::Unreadable(source.to_string()))?;
    let mut named = Vec::new();
    for branch in branches {
        let branch = branch.map_err(|source| ReflogError::Unreadable(source.to_string()))?;
        named.push(newest_of(&branch, cap)?);
    }
    named.sort_by(|left, right| left.name.cmp(&right.name));
    logs.extend(named);
    Ok(merge_newest_first(logs, cap))
}

/// The first-parent distance from `from` back to `to`: `Some(n)` when `to`
/// is `from`'s `n`-th first parent, `n <= limit`; `None` otherwise.
///
/// What a `reset: moving to …` record's "N commits back" is measured by
/// (`to` = the new value, `from` = the old), and a fast-forward's "N
/// commits" the other way round. Reads commit headers through `gix`, at most
/// `limit` of them. Stops at a shallow boundary: a commit listed in
/// `<common_dir>/shallow` has no parents (git's rule, m7/32b).
///
/// # Errors
/// [`ReflogError::Shallow`] if the shallow list cannot be read;
/// [`ReflogError::Unreadable`] if a commit on the way cannot be.
pub fn first_parent_distance(
    repo: &RepoHandle,
    from: Oid,
    to: Oid,
    limit: usize,
) -> Result<Option<usize>, ReflogError> {
    let shallow = ShallowList::read(repo)?;
    let gix_repo = repo.gix();
    let mut at = from;
    for distance in 0..=limit {
        if at == to {
            return Ok(Some(distance));
        }
        if distance == limit || shallow.contains(&at) {
            break;
        }
        let id = gix::hash::oid::try_from_bytes(at.as_bytes())
            .map_err(|source| ReflogError::Unreadable(source.to_string()))?;
        let commit = match gix_repo.find_commit(id) {
            Ok(commit) => commit,
            // A pruned commit on the way: the distance cannot be measured,
            // which is an answer ("not within reach"), not a failed read.
            Err(gix::object::find::existing::with_conversion::Error::Find(
                gix::object::find::existing::Error::NotFound { .. },
            )) => return Ok(None),
            Err(source) => return Err(ReflogError::Unreadable(source.to_string())),
        };
        let Some(parent) = commit.parent_ids().next() else {
            return Ok(None);
        };
        at = Oid::from_bytes(parent.as_bytes())
            .map_err(|source| ReflogError::Unreadable(source.to_string()))?;
    }
    Ok(None)
}

/// One log's newest records, newest first.
struct Log {
    name: String,
    records: Vec<ReflogRecord>,
}

fn newest_of(reference: &gix::Reference<'_>, cap: usize) -> Result<Log, ReflogError> {
    let name = String::from_utf8_lossy(reference.name().as_bstr()).into_owned();
    let mut platform = reference.log_iter();
    let Some(lines) = platform
        .rev()
        .map_err(|source| ReflogError::Unreadable(source.to_string()))?
    else {
        return Ok(Log {
            name,
            records: Vec::new(),
        });
    };
    let mut records = Vec::new();
    for line in lines.take(cap) {
        let line = line.map_err(|source| ReflogError::Unreadable(source.to_string()))?;
        records.push(ReflogRecord {
            refname: name.clone(),
            old: oid_or_zero(line.previous_oid.as_bytes())?,
            new: oid_or_zero(line.new_oid.as_bytes())?,
            at: line.signature.time.seconds,
            subject: String::from_utf8_lossy(BStr::new(&line.message)).into_owned(),
        });
    }
    Ok(Log { name, records })
}

fn oid_or_zero(bytes: &[u8]) -> Result<Option<Oid>, ReflogError> {
    if bytes.iter().all(|byte| *byte == 0) {
        return Ok(None);
    }
    Oid::from_bytes(bytes)
        .map(Some)
        .map_err(|source| ReflogError::Unreadable(source.to_string()))
}

/// A k-way merge of logs that are each newest first: always the log whose
/// next record is newest, the earlier log on a tie. Each log's own order is
/// kept even where its clock went backwards.
fn merge_newest_first(logs: Vec<Log>, cap: usize) -> Vec<ReflogRecord> {
    let mut heads: Vec<std::vec::IntoIter<ReflogRecord>> = logs
        .into_iter()
        .map(|log| log.records.into_iter())
        .collect();
    let mut pending: Vec<Option<ReflogRecord>> = heads.iter_mut().map(Iterator::next).collect();
    let mut heap = BinaryHeap::new();
    for (index, record) in pending.iter().enumerate() {
        if let Some(record) = record {
            heap.push((record.at, Reverse(index)));
        }
    }
    let mut merged = Vec::new();
    while merged.len() < cap {
        let Some((_, Reverse(index))) = heap.pop() else {
            break;
        };
        if let Some(record) = pending[index].take() {
            merged.push(record);
        }
        pending[index] = heads[index].next();
        if let Some(record) = &pending[index] {
            heap.push((record.at, Reverse(index)));
        }
    }
    merged
}

#[cfg(test)]
mod tests;
