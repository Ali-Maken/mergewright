//! The stash put-back preview (m6-conflicts/86): what `stash apply` or
//! `stash pop` would write, and whether it may clash.
//!
//! - **Files:** `stash^1`→`stash` (the tracked changes), then the paths of
//!   `stash^3` (the untracked files a `-u` stash carries).
//! - **In the way:** a file the index doesn't track, where the stash writes
//!   one, ignored files included (the status doesn't list those): read from
//!   the disk by `crate::in_the_way::blocking` in `plan`, and again at `y`
//!   (ADR-0049). Unsaved edits to a tracked path are found from the facts, before any
//!   spawn ([`in_the_way`]).
//! - **Staged part:** git's `--index` rule, checked first: when the stash's
//!   staged part no longer applies to the index (`apply --cached --check`
//!   on the same temporary copy), git refuses, and so does the preview
//!   (review 86, M1).
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

use std::path::{Path, PathBuf};

use mergewright_core::journal::IndexState;
use mergewright_core::oid::Oid;
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::{GitArg, GitInvocation, RunError, Stop};
use mergewright_git::version::GitVersion;

use crate::operation::{Refusal, RepoFacts};
use crate::snapshot::{self, SnapshotError};

use super::{ConflictForecast, PATHS_CAP, PreviewError, WorktreeEffect, forecast_one};

/// The oldest git whose `merge-tree` takes `--merge-base`.
const MERGE_BASE_MIN: GitVersion = GitVersion {
    major: 2,
    minor: 40,
    patch: 0,
};

/// `check`'s [`Refusal::TouchesChangedFiles`] for a stash put-back, in the
/// stash preview's words: among the paths the stash touched, those with
/// unsaved edits (worktree ≠ index), an untracked file standing where the
/// stash writes, and the rest of `touched` (the staged-only edits `check`
/// also refuses) named beside them (review 86, m4, m5). Staged-only edits
/// alone keep `check`'s own refusal.
pub(super) fn in_the_way(commit: Oid, facts: &RepoFacts, touched: Vec<PathBuf>) -> Refusal {
    let Some(entry) = facts.stashes.iter().find(|stash| stash.commit == commit) else {
        return Refusal::TouchesChangedFiles { paths: touched };
    };
    let now = |path: &PathBuf| facts.status.entry(path);
    let unsaved: Vec<PathBuf> = entry
        .paths
        .iter()
        .filter(|path| now(path).is_some_and(|now| now.is_unstaged() && !now.untracked))
        .cloned()
        .collect();
    let untracked: Vec<PathBuf> = entry
        .paths
        .iter()
        .filter(|path| now(path).is_some_and(|now| now.untracked))
        .cloned()
        .collect();
    if unsaved.is_empty() && untracked.is_empty() {
        return Refusal::TouchesChangedFiles { paths: touched };
    }
    let staged = touched
        .into_iter()
        .filter(|path| !unsaved.contains(path) && !untracked.contains(path))
        .collect();
    // Status never lists an ignored file; those are `in_the_way`'s.
    Refusal::StashInTheWay {
        unsaved,
        staged,
        untracked,
        ignored: Vec::new(),
    }
}

/// What putting `stash` back would do to the working tree, and whether it
/// may clash with the index as it stands.
///
/// # Errors
/// [`PreviewError::Stopped`] on a stop between spawns;
/// [`PreviewError::Refused`] with
/// [`Refusal::StashIndexNoLongerApplies`] when its staged part no longer
/// applies to the index (git's `--index` rule, [`forecast`]);
/// [`PreviewError::Read`] when the stash's parents or paths could not be
/// read. `index` is the stash's position in the list, for the words.
pub(super) fn forecast_stash(
    repo: &RepoHandle,
    git: &GitVersion,
    index: usize,
    stash: Oid,
    stop: &dyn Stop,
) -> Result<(WorktreeEffect, ConflictForecast), PreviewError> {
    stopped(stop)?;
    let parents = parents_of(repo, stash, stop)?;
    let (Some(&base), Some(&staged)) = (parents.first(), parents.get(1)) else {
        return Err(PreviewError::Read(RunError::Unparsable(format!(
            "{stash} has fewer than two parents; it is not a stash"
        ))));
    };
    let untracked_tree = parents.get(2).copied();

    stopped(stop)?;
    let (mut paths, mut more) =
        mergewright_git::history_reads::changed_paths(repo, base, stash, PATHS_CAP)?;
    let untracked = match untracked_tree {
        Some(tree) => {
            stopped(stop)?;
            tree_paths(repo, tree, stop)?
        }
        None => Vec::new(),
    };
    // A file on disk where the stash creates one was refused by `plan`
    // before this runs (`in_the_way`, ADR-0049).
    for path in untracked {
        if paths.len() >= PATHS_CAP {
            more = true;
            break;
        }
        if !paths.contains(&path) {
            paths.push(path);
        }
    }
    let worktree = if paths.is_empty() {
        // Nothing for the files: the stash's one change is staged (its
        // file was put back as committed before the stash). Name the staged
        // paths rather than an empty list (review 86, m1).
        stopped(stop)?;
        let (paths, more) = mergewright_git::history_reads::changed_paths(repo, base, staged, PATHS_CAP)?;
        WorktreeEffect::IndexOnly { paths, more }
    } else {
        WorktreeEffect::Updated { paths, more }
    };

    let conflicts = forecast(repo, git, index, [base, staged, stash], stop)?;
    Ok((worktree, conflicts))
}

/// git's own order for `stash apply --index`: first the **staged part**,
/// then the three-way merge.
///
/// 1. When the stash's index tree differs from both its base's tree and the
///    index tree now, git applies `diff-tree --binary stash^2^ stash^2` to
///    the index with `apply --cached`, and on failure refuses with
///    `conflicts in index` having written nothing. The same patch is
///    checked here (`apply --cached --check`) against the same temporary
///    copy of the index the forecast reads: failure is
///    [`Refusal::StashIndexNoLongerApplies`], never `Clean` (review 86, M1).
/// 2. `merge-tree --merge-base=<base> <index now> <stash>` (git 2.40+).
///
/// `Ok` with a hole on any read failure; `Err` only on a stop or that
/// refusal.
fn forecast(
    repo: &RepoHandle,
    git: &GitVersion,
    index: usize,
    [base, staged, stash]: [Oid; 3],
    stop: &dyn Stop,
) -> Result<ConflictForecast, PreviewError> {
    stopped(stop)?;
    // The copy lives in the git dir for the moment it takes, beside the
    // index it copies, and is removed on every path (`TempIndex`).
    let read = snapshot::index_state_then(repo, repo.git_dir(), stop, |state, copy| match state {
        IndexState::Merged(now) => staged_part_applies(repo, [base, staged], *now, copy, stop),
        // Not asked: an unmerged index is `NotChecked` below either way.
        IndexState::Unmerged { .. } => Ok(true),
    });
    let (tree, applies) = match read {
        Ok((IndexState::Merged(tree), applies)) => (tree, applies),
        Ok((IndexState::Unmerged { .. }, _)) => {
            return Ok(not_checked("the index has unresolved conflicts"));
        }
        Err(SnapshotError::Stopped) => return Err(PreviewError::Stopped),
        Err(why) => return Ok(not_checked(&why.to_string())),
    };
    match applies {
        Ok(true) => {}
        Ok(false) => {
            return Err(PreviewError::Refused(Refusal::StashIndexNoLongerApplies {
                index,
            }));
        }
        Err(PreviewError::Stopped) => return Err(PreviewError::Stopped),
        Err(why) => return Ok(not_checked(&why.to_string())),
    }
    if *git < MERGE_BASE_MIN {
        return Ok(not_checked("git 2.40+ needed to forecast a stash"));
    }
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
        Err(why) => Ok(not_checked(&why.to_string())),
    }
}

/// Whether the stash's staged part applies to the index `now` (its tree),
/// held at `copy`: git's `--index` rule, step 1 of [`forecast`]. `true`
/// without a spawn when git would not apply it at all — the staged tree is
/// the base's (nothing was staged) or is already the index's.
fn staged_part_applies(
    repo: &RepoHandle,
    [base, staged]: [Oid; 2],
    now: Oid,
    copy: &Path,
    stop: &dyn Stop,
) -> Result<bool, PreviewError> {
    stopped(stop)?;
    let staged_tree = tree_of(repo, staged, stop)?;
    stopped(stop)?;
    if staged_tree == now || staged_tree == tree_of(repo, base, stop)? {
        return Ok(true);
    }
    stopped(stop)?;
    let patch = GitInvocation::new(repo)
        .args([
            GitArg::Flag("diff-tree"),
            GitArg::Flag("--binary"),
            GitArg::Oid(base),
            GitArg::Oid(staged),
        ])
        .run(stop)?
        .stdout;
    stopped(stop)?;
    let checked = GitInvocation::new(repo)
        .env("GIT_INDEX_FILE", copy)
        .args([
            GitArg::Flag("apply"),
            GitArg::Flag("--cached"),
            GitArg::Flag("--check"),
        ])
        .stdin(patch)
        .run(stop);
    match checked {
        Ok(_) => Ok(true),
        // git's answer, not a failed read: the patch does not apply.
        Err(RunError::Exited { .. }) => Ok(false),
        Err(why) => Err(why.into()),
    }
}

/// `rev-parse --verify <commit>^{tree}`: the commit's tree.
fn tree_of(repo: &RepoHandle, commit: Oid, stop: &dyn Stop) -> Result<Oid, PreviewError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("rev-parse"),
            GitArg::Flag("--verify"),
            GitArg::Word(format!("{commit}^{{tree}}")),
        ])
        .run(stop)?;
    let printed = String::from_utf8_lossy(&output.stdout);
    Oid::from_hex(printed.trim()).map_err(|_| {
        PreviewError::Read(RunError::Unparsable(format!(
            "rev-parse printed {}, not a tree id",
            printed.trim()
        )))
    })
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

fn stopped(stop: &dyn Stop) -> Result<(), PreviewError> {
    if stop.should_stop() {
        return Err(PreviewError::Stopped);
    }
    Ok(())
}

fn not_checked(why: &str) -> ConflictForecast {
    ConflictForecast::NotChecked(why.to_owned())
}
