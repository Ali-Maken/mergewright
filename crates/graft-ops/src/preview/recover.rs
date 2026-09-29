//! The put-back's picture (note J1, m7-power-tools/23): the `now` / `after`
//! ref lines as [`RefMove`]s, and the `your files:` line as a
//! [`FilesEffect`]. The app draws the box; this says what goes in it.

use std::collections::BTreeSet;
use std::path::PathBuf;

use graft_core::oid::Oid;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, RunError, Stop};

use crate::facts::FactsError;
use crate::operation::{FilesEffect, RecoverParams, RecoverTarget, RepoFacts, branch_ref};
use crate::snapshot::SnapshotError;
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
            // His files in the reset's way were refused by `plan` before
            // this runs (`in_the_way`, ADR-0049).
            if on_head {
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
                replaced: changed_now(paths, facts),
            });
        }
    }
    Ok(plan)
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
            replaced: Vec::new(),
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

/// Which of `paths` he has changed against `HEAD` — staged, unstaged or
/// untracked — in `paths`' order: what a file put-back writes over, and its
/// entry's before-snapshot keeps (m7/30o R1). An ignored file there never
/// gets here: `in_the_way` refused it first.
fn changed_now(paths: &[PathBuf], facts: &RepoFacts) -> Vec<PathBuf> {
    paths
        .iter()
        .filter(|path| {
            facts.status.entries.iter().any(|entry| {
                entry.path == **path
                    && (entry.is_staged() || entry.is_unstaged() || entry.untracked)
            })
        })
        .cloned()
        .collect()
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
