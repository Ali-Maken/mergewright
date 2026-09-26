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
            // His `apply.whitespace` must not change the verdict: `error`
            // exits 128, which is not "does not apply" (review m7-06 M5).
            GitArg::Flag("--whitespace=nowarn"),
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

#[cfg(test)]
mod tests {
    use super::*;

    struct Never;

    impl Stop for Never {
        fn should_stop(&self) -> bool {
            false
        }
    }

    /// A scratch repository, removed on drop.
    struct Scratch(std::path::PathBuf);

    impl Drop for Scratch {
        fn drop(&mut self) {
            // Best effort: a left-over scratch directory is noise, not a
            // failure, and a panic in `Drop` would hide the real one.
            let _removed = std::fs::remove_dir_all(&self.0);
        }
    }

    /// git's empty tree: `read-tree` knows it without the object stored.
    const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

    #[test]
    fn an_apply_failure_other_than_does_not_apply_is_an_error() {
        // A broken patch is git's exit 128, not the "does not apply" exit
        // 1: read as a stay, a bug in the patch would hide as a verdict
        // (review m7-06 M8).
        let dir = std::env::temp_dir().join(format!("graft-judge-{}", std::process::id()));
        let scratch = Scratch(dir.clone());
        std::fs::create_dir_all(&scratch.0).expect("the scratch directory");
        let status = std::process::Command::new("git")
            .args(["init", "-q", "."])
            .current_dir(&dir)
            .status()
            .expect("git runs");
        assert!(status.success());
        let repo = RepoHandle::open(&dir).expect("it opens");
        let tree = Oid::from_hex(EMPTY_TREE).expect("an id");

        let judged = commutes(&repo, tree, b"not a patch\n", &Never);

        assert!(judged.is_err(), "{judged:?}");
    }
}
