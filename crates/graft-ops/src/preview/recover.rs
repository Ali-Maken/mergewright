//! The put-back's picture (note J1, m7-power-tools/23): the `now` / `after`
//! ref lines as [`RefMove`]s, and the `your files:` line as a
//! [`FilesEffect`]. The app draws the box; this says what goes in it.

use std::collections::BTreeSet;
use std::path::PathBuf;

use graft_core::oid::Oid;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, RunError, Stop};

use crate::facts::FactsError;
use crate::operation::{FilesEffect, RecoverParams, RecoverTarget, Refusal, RepoFacts, branch_ref};
use crate::snapshot::{SnapshotError, tracked_paths_now};
use crate::undo::tree_diff;

use super::{Landing, PreviewError, PreviewPlan, RefMove, Target, WorktreeEffect, blank, worktree};

pub(super) fn recover(
    repo: &RepoHandle,
    params: &RecoverParams,
    facts: &RepoFacts,
    stop: &dyn Stop,
) -> Result<PreviewPlan, PreviewError> {
    let mut plan = blank();
    match &params.target {
        RecoverTarget::MoveRef { name, to, expected } => {
            let full = branch_ref(name);
            plan.ref_moves = vec![RefMove {
                name: full.clone(),
                from: Some(*expected),
                to: Some(Target::Real(*to)),
            }];
            let on_head = matches!(
                &facts.head,
                graft_core::journal::HeadRef::Branch(head) if *head == full
            );
            if on_head {
                let in_the_way = in_the_way(repo, params, facts, stop).map_err(preview_error)?;
                if !in_the_way.is_empty() {
                    return Err(PreviewError::Refused(Refusal::UnsavedEditsTo {
                        paths: in_the_way,
                    }));
                }
                let aside = set_aside_count(params, facts);
                (plan.worktree, plan.files) = reset_effect(repo, *expected, *to, aside, stop)?;
            } else {
                plan.files = Some(not_checked_out(name));
            }
        }
        RecoverTarget::Recreate { name, at } => {
            plan.ref_moves = vec![RefMove {
                name: branch_ref(name),
                from: None,
                to: Some(Target::Real(*at)),
            }];
            plan.files = Some(not_checked_out(name));
        }
        RecoverTarget::Files {
            backup_tree, paths, ..
        } => {
            let held = held_in(repo, *backup_tree, paths, stop)?;
            let (come_back, go) =
                split(paths.iter().map(|path| (path.clone(), held.contains(path))));
            plan.worktree = WorktreeEffect::Updated {
                paths: paths.clone(),
                more: false,
            };
            plan.files = Some(FilesEffect::Changes {
                come_back,
                go,
                set_aside: 0,
            });
        }
    }
    Ok(plan)
}

/// His files a checked-out `MoveRef`'s `reset --hard` would write over:
/// every path `to` adds or changes against `expected` (and every untracked
/// path `to` holds) that the index does not track and that stands on disk.
///
/// Decided from the disk, not from `git status` (review m7-22, C1): status
/// never lists an **ignored** file, yet `reset --hard` writes over it, and
/// the backup (`add -A`) cannot hold it, so `u` could not bring it back.
/// Tracked paths are the set-aside's (ADR-0045 §4), never in the way. The
/// stash preview's `occupied` is the same rule. Both the preview and
/// `apply` refuse over these (`Refusal::UnsavedEditsTo`). Empty for every
/// other target.
pub(crate) fn in_the_way(
    repo: &RepoHandle,
    params: &RecoverParams,
    facts: &RepoFacts,
    stop: &dyn Stop,
) -> Result<Vec<PathBuf>, FactsError> {
    let RecoverTarget::MoveRef { name, to, expected } = &params.target else {
        return Ok(Vec::new());
    };
    let on_head = matches!(
        &facts.head,
        graft_core::journal::HeadRef::Branch(head) if *head == branch_ref(name)
    );
    let Some(root) = repo.workdir().filter(|_| on_head) else {
        // Not checked out: an `update-ref` writes no file. A bare
        // repository has no files to write over.
        return Ok(Vec::new());
    };
    let mut written: BTreeSet<PathBuf> = tree_diff(repo, *expected, *to, stop, &mut Vec::new())?
        .into_iter()
        .filter_map(|(path, holds)| holds.map(|_| path))
        .collect();
    let untracked: Vec<PathBuf> = facts
        .status
        .entries
        .iter()
        .filter(|entry| entry.untracked)
        .map(|entry| entry.path.clone())
        .collect();
    if !untracked.is_empty() {
        written.extend(held_in(repo, *to, &untracked, stop).map_err(run_error)?);
    }
    if written.is_empty() {
        return Ok(Vec::new());
    }
    let tracked = tracked_paths_now(repo, stop, &mut Vec::new())?;
    let mut found = Vec::new();
    for path in written.into_iter().filter(|path| !tracked.contains(path)) {
        match std::fs::symlink_metadata(root.join(&path)) {
            Ok(_) => found.push(path),
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => {}
            // A file where `to` needs a directory (`a` for `a/b`) is in the
            // way too: git would remove it to make the directory.
            Err(why) if why.kind() == std::io::ErrorKind::NotADirectory => found.push(path),
            Err(source) => return Err(FactsError::OnDisk { path, source }),
        }
    }
    Ok(found)
}

fn run_error(why: RunError) -> FactsError {
    match why {
        RunError::Stopped => FactsError::Stopped,
        other => FactsError::Snapshot(SnapshotError::Run(other)),
    }
}

/// A checked-out branch put back is a `reset --hard` from `expected` to
/// `to`: the files that differ come back or go, and his edits are set
/// aside around it.
fn reset_effect(
    repo: &RepoHandle,
    expected: Oid,
    to: Oid,
    set_aside: usize,
    stop: &dyn Stop,
) -> Result<(WorktreeEffect, Option<FilesEffect>), PreviewError> {
    let changed = tree_diff(repo, expected, to, stop, &mut Vec::new()).map_err(preview_error)?;
    let (come_back, go) = split(
        changed
            .into_iter()
            .map(|(path, holds)| (path, holds.is_some())),
    );
    let landing = worktree(repo, &Landing::At { from: expected, to })?;
    Ok((
        landing,
        Some(FilesEffect::Changes {
            come_back,
            go,
            set_aside,
        }),
    ))
}

fn not_checked_out(name: &str) -> FilesEffect {
    FilesEffect::Unchanged {
        not_checked_out: format!("{name} isn't checked out"),
    }
}

/// How many changed files the put-back sets aside: the same count `apply`
/// acts on, never a second one.
fn set_aside_count(params: &RecoverParams, facts: &RepoFacts) -> usize {
    crate::operation::Operation::Recover(params.clone())
        .sets_aside(facts)
        .map_or(0, |aside| aside.files)
}

/// Present afterwards → comes back; absent → goes.
fn split(paths: impl Iterator<Item = (PathBuf, bool)>) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut come_back = Vec::new();
    let mut go = Vec::new();
    for (path, present) in paths {
        if present {
            come_back.push(path);
        } else {
            go.push(path);
        }
    }
    (come_back, go)
}

/// Which of `paths` the tree holds: `git ls-tree -r -z --name-only <tree>
/// -- <paths>`.
fn held_in(
    repo: &RepoHandle,
    tree: Oid,
    paths: &[PathBuf],
    stop: &dyn Stop,
) -> Result<BTreeSet<PathBuf>, RunError> {
    use std::os::unix::ffi::{OsStrExt, OsStringExt};
    let mut args = vec![
        GitArg::Flag("--literal-pathspecs"),
        GitArg::Flag("ls-tree"),
        GitArg::Flag("-r"),
        GitArg::Flag("-z"),
        GitArg::Flag("--name-only"),
        GitArg::Oid(tree),
        GitArg::Separator,
    ];
    args.extend(
        paths
            .iter()
            .map(|path| GitArg::Path(path.as_os_str().as_bytes().to_vec())),
    );
    let output = GitInvocation::new(repo).args(args).run(stop)?;
    Ok(output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|name| !name.is_empty())
        .map(|name| PathBuf::from(std::ffi::OsString::from_vec(name.to_vec())))
        .collect())
}

fn preview_error(why: FactsError) -> PreviewError {
    match why {
        FactsError::Stopped | FactsError::Snapshot(SnapshotError::Stopped) => PreviewError::Stopped,
        other => PreviewError::Facts(other),
    }
}
