//! The stash put-back preview (m6-conflicts/86): what `stash apply` or
//! `stash pop` would write, and whether it may clash.
//!
//! - **Files:** `stash^1`→`stash` (the tracked changes), then the paths of
//!   `stash^3` (the untracked files a `-u` stash carries).
//! - **In the way:** a file where the stash would create an untracked one.
//!   git refuses to write over it (`already exists, no checkout`), ignored
//!   files included — which the status does not list, so the disk is read.
//!   Unsaved edits to a tracked path are found from the facts, before any
//!   spawn ([`in_the_way`]).
//! - **Forecast:** `git merge-tree --merge-base=<stash^1> <ours> <stash>`,
//!   the three-way merge `stash apply` does, with `ours` the **index tree
//!   now** — written by `write-tree` over a temporary copy of the index
//!   (`snapshot::index_state`) and made a commit (`scratch_commit`), since
//!   git 2.40's `merge-tree` was only confirmed to take commits (see
//!   `put_back`). No ref, index or file is written; the scratch commit is a
//!   loose object nothing references.
//!
//! A read that goes wrong while forecasting is `NotChecked(<why>)`, never a
//! clean bill; a read that goes wrong while listing the files fails the
//! preview (the box then says `preview failed:`), since a stash put-back
//! with no list of what it writes is not a picture of it.

use std::path::PathBuf;

use graft_core::journal::IndexState;
use graft_core::oid::Oid;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, RunError, Stop};
use graft_git::version::GitVersion;

use crate::operation::{Refusal, RepoFacts};
use crate::snapshot::{self, SnapshotError};

use super::{ConflictForecast, PATHS_CAP, PreviewError, WorktreeEffect, forecast_one};

/// The oldest git whose `merge-tree` takes `--merge-base`.
const MERGE_BASE_MIN: GitVersion = GitVersion {
    major: 2,
    minor: 40,
    patch: 0,
};

/// The paths of a stash put-back that `check` already knows are in the way:
/// among the paths the stash touched, those with unsaved edits (worktree ≠
/// index) or an untracked file standing where the stash writes. Staged-only
/// edits are not in the way — git merges over the index — so they are left
/// to `check`'s own refusal.
pub(super) fn in_the_way(commit: Oid, facts: &RepoFacts) -> Vec<PathBuf> {
    let Some(entry) = facts.stashes.iter().find(|stash| stash.commit == commit) else {
        return Vec::new();
    };
    entry
        .paths
        .iter()
        .filter(|path| {
            facts
                .status
                .entry(path)
                .is_some_and(|now| now.is_unstaged() || now.untracked)
        })
        .cloned()
        .collect()
}

/// What putting `stash` back would do to the working tree, and whether it
/// may clash with the index as it stands.
///
/// # Errors
/// [`PreviewError::Stopped`] on a stop between spawns;
/// [`PreviewError::Refused`] with [`Refusal::StashInTheWay`] when a file
/// stands where the stash creates an untracked one; [`PreviewError::Read`]
/// when the stash's parents or paths could not be read.
pub(super) fn forecast_stash(
    repo: &RepoHandle,
    git: &GitVersion,
    stash: Oid,
    stop: &dyn Stop,
) -> Result<(WorktreeEffect, ConflictForecast), PreviewError> {
    stopped(stop)?;
    let parents = parents_of(repo, stash, stop)?;
    let Some(&base) = parents.first() else {
        return Err(PreviewError::Read(RunError::Unparsable(format!(
            "{stash} has no parent; it is not a stash"
        ))));
    };
    let untracked_tree = parents.get(2).copied();

    stopped(stop)?;
    let (mut paths, mut more) =
        graft_git::history_reads::changed_paths(repo, base, stash, PATHS_CAP)?;
    let untracked = match untracked_tree {
        Some(tree) => {
            stopped(stop)?;
            tree_paths(repo, tree, stop)?
        }
        None => Vec::new(),
    };
    let occupied = occupied(repo, &untracked)?;
    if !occupied.is_empty() {
        return Err(PreviewError::Refused(Refusal::StashInTheWay {
            paths: occupied,
        }));
    }
    for path in untracked {
        if paths.len() >= PATHS_CAP {
            more = true;
            break;
        }
        if !paths.contains(&path) {
            paths.push(path);
        }
    }
    let worktree = WorktreeEffect::Updated { paths, more };

    if *git < MERGE_BASE_MIN {
        return Ok((
            worktree,
            not_checked("git 2.40+ needed to forecast a stash"),
        ));
    }
    let conflicts = forecast(repo, git, base, stash, stop)?;
    Ok((worktree, conflicts))
}

/// `merge-tree --merge-base=<base> <index now> <stash>`. `Ok` with a hole on
/// any read failure; `Err` only on a stop.
fn forecast(
    repo: &RepoHandle,
    git: &GitVersion,
    base: Oid,
    stash: Oid,
    stop: &dyn Stop,
) -> Result<ConflictForecast, PreviewError> {
    stopped(stop)?;
    // The copy lives in the git dir for the moment it takes, beside the
    // index it copies, and is removed on every path (`TempIndex`).
    let tree = match snapshot::index_state(repo, repo.git_dir(), stop) {
        Ok(IndexState::Merged(tree)) => tree,
        Ok(IndexState::Unmerged { .. }) => {
            return Ok(not_checked("the index has unresolved conflicts"));
        }
        Err(SnapshotError::Stopped) => return Err(PreviewError::Stopped),
        Err(why) => return Ok(not_checked(&why.to_string())),
    };
    stopped(stop)?;
    let ours = match snapshot::scratch_commit(repo, tree, &[], stop) {
        Ok(ours) => ours,
        Err(SnapshotError::Stopped) => return Err(PreviewError::Stopped),
        Err(why) => return Ok(not_checked(&why.to_string())),
    };
    stopped(stop)?;
    match forecast_one(repo, git, Some(base), ours, stash, stop) {
        Ok(forecast) => Ok(forecast),
        Err(PreviewError::Stopped) => Err(PreviewError::Stopped),
        Err(why) => Ok(not_checked(&why.explain())),
    }
}

/// `rev-list --no-walk --parents <stash>`: the stash's parents, first
/// first. A stash has two (`HEAD` then, the index then), or three with
/// untracked files.
fn parents_of(repo: &RepoHandle, stash: Oid, stop: &dyn Stop) -> Result<Vec<Oid>, PreviewError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("rev-list"),
            GitArg::Flag("--no-walk"),
            GitArg::Flag("--parents"),
            GitArg::Oid(stash),
        ])
        .run(stop)?;
    let line = String::from_utf8_lossy(&output.stdout);
    line.split_whitespace()
        .skip(1)
        .map(|hex| {
            Oid::from_hex(hex).map_err(|_| {
                PreviewError::Read(RunError::Unparsable(format!(
                    "rev-list --parents printed {hex}, not an id"
                )))
            })
        })
        .collect()
}

/// `ls-tree -r --name-only -z <commit>`: every file the untracked commit
/// holds.
fn tree_paths(
    repo: &RepoHandle,
    commit: Oid,
    stop: &dyn Stop,
) -> Result<Vec<PathBuf>, PreviewError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("ls-tree"),
            GitArg::Flag("-r"),
            GitArg::Flag("--name-only"),
            GitArg::Flag("-z"),
            GitArg::Oid(commit),
        ])
        .run(stop)?;
    Ok(output
        .stdout
        .split(|&byte| byte == 0)
        .filter(|path| !path.is_empty())
        .map(path_of)
        .collect())
}

/// Raw path bytes as a `PathBuf`, never decoded.
fn path_of(bytes: &[u8]) -> PathBuf {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        PathBuf::from(std::ffi::OsStr::from_bytes(bytes))
    }
    #[cfg(not(unix))]
    {
        PathBuf::from(String::from_utf8_lossy(bytes).into_owned())
    }
}

/// Which of `paths` something already stands at on disk (a file, a link or
/// a directory): the untracked files git would refuse to write over.
fn occupied(repo: &RepoHandle, paths: &[PathBuf]) -> Result<Vec<PathBuf>, PreviewError> {
    let Some(root) = repo.workdir() else {
        // A bare repository has no working tree to put a stash back into;
        // `check` refuses it first. Nothing stands in the way of nothing.
        return Ok(Vec::new());
    };
    let mut found = Vec::new();
    for path in paths {
        match std::fs::symlink_metadata(root.join(path)) {
            Ok(_) => found.push(path.clone()),
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => {}
            // A parent that is a file (`a` where the stash creates `a/b`)
            // is in the way too: git cannot make the directory.
            Err(why) if why.kind() == std::io::ErrorKind::NotADirectory => {
                found.push(path.clone());
            }
            Err(why) => {
                return Err(PreviewError::Read(RunError::Unparsable(format!(
                    "{} could not be read: {why}",
                    path.display()
                ))));
            }
        }
    }
    Ok(found)
}

fn stopped(stop: &dyn Stop) -> Result<(), PreviewError> {
    if stop.should_stop() {
        return Err(PreviewError::Stopped);
    }
    Ok(())
}

fn not_checked(why: &str) -> ConflictForecast {
    ConflictForecast::NotChecked(why.to_owned())
}
