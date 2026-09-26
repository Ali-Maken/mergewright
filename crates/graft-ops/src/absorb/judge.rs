//! Judging one hunk: the commit its blamed lines name, and whether it
//! commutes down to that commit.

use std::collections::{BTreeMap, BTreeSet};

use graft_core::oid::Oid;
use graft_git::diff::Hunk;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, RunError, Stop};

use crate::facts::FactsError;
use crate::snapshot::TempIndex;

use super::read::asks_about;
use super::{StaysWhy, unparsable, with_index};

/// The commit a hunk goes to, or why it stays. The outer `Err` is a blame
/// that did not cover a line it was asked about.
pub(super) fn verdict(
    hunk: &Hunk,
    line_count: usize,
    blamed: &BTreeMap<u32, Oid>,
    range: &BTreeSet<Oid>,
) -> Result<Result<Oid, StaysWhy>, FactsError> {
    let count = u32::try_from(line_count).unwrap_or(u32::MAX);
    let lines = asks_about(hunk);
    if lines.iter().any(|line| !(1..=count).contains(line)) {
        // An insertion at the top or the bottom: one neighbour is missing.
        return Ok(Err(StaysWhy::NoCommitChangedIt));
    }
    let commits = lines
        .iter()
        .map(|line| {
            blamed
                .get(line)
                .copied()
                .ok_or_else(|| unparsable(format!("blame gave no commit for line {line}")))
        })
        .collect::<Result<BTreeSet<Oid>, FactsError>>()?;
    if commits.iter().all(|commit| !range.contains(commit)) {
        return Ok(Err(StaysWhy::BeforeYourBranch));
    }
    match commits.iter().next() {
        Some(only) if commits.len() == 1 => Ok(Ok(*only)),
        _ => Ok(Err(StaysWhy::TwoCommitsChangedIt)),
    }
}

/// `patch` applies on `target`'s own tree. A temporary index; the real one
/// is never read or written.
pub(super) fn commutes(
    repo: &RepoHandle,
    target: Oid,
    patch: &[u8],
    stop: &dyn Stop,
) -> Result<bool, FactsError> {
    let index = TempIndex::fresh(repo.git_dir())?;
    with_index(repo, &index)
        .args([GitArg::Flag("read-tree"), GitArg::Oid(target)])
        .run(stop)
        .map_err(FactsError::of_history)?;
    let checked = with_index(repo, &index)
        .args([
            GitArg::Flag("apply"),
            GitArg::Flag("--cached"),
            GitArg::Flag("--check"),
            GitArg::Flag("-"),
        ])
        .stdin(patch.to_vec())
        .run(stop);
    match checked {
        Ok(_) => Ok(true),
        // git's "patch does not apply".
        Err(RunError::Exited { status: 1, .. }) => Ok(false),
        Err(other) => Err(FactsError::of_history(other)),
    }
}
