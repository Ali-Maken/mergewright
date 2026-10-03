//! The line of branches built on his (ADR-0047 §4): which branches follow
//! a rewrite, and the refusals that stop one before anything runs.

use std::path::PathBuf;

use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, Stop};
use graft_git::version::GitVersion;

use crate::facts::FactsError;
use crate::operation::{Refusal, Tip};
use crate::preview::{LEFT_BEHIND_CAP, LeftBehind};

use super::reads::{default_branch_of, parse_oid, ref_name, unparsable};
use super::{BranchLine, Dependents, RewriteError};

/// The oldest git whose `rebase -i` knows `update-ref`: a rewrite with
/// branches following needs it (ADR-0047 §4).
pub(super) const UPDATE_REF_GIT: GitVersion = GitVersion {
    major: 2,
    minor: 38,
    patch: 0,
};

/// His branch and the branches built on it, or the refusal that stops the
/// rewrite: a fork, one checked out in another worktree, or an old git.
pub(crate) fn line_of(
    repo: &RepoHandle,
    git: &GitVersion,
    head: &Tip,
    default_branch: Option<&RefName>,
    stop: &dyn Stop,
) -> Result<BranchLine, RewriteError> {
    let line = match dependents(repo, git, head, default_branch, stop)? {
        Dependents::Line(line) => line,
        Dependents::Fork { a, b } => {
            return Err(RewriteError::Refused(Refusal::ForkedDependents { a, b }));
        }
    };
    if line.branches.len() > 1 {
        if let Some((branch, worktree)) = checked_out_elsewhere(repo, line.dependents(), stop)? {
            return Err(RewriteError::Refused(Refusal::DependentElsewhere {
                branch,
                worktree,
            }));
        }
        if *git < UPDATE_REF_GIT {
            return Err(RewriteError::Refused(Refusal::GitTooOld { need: "2.38" }));
        }
    }
    Ok(line)
}

/// The branches built on `branch`, bottom first after it.
///
/// A local branch B is one when B's first-parent chain reaches `branch`'s
/// tip with no merge between the two, B is not the default branch, and B
/// has at least one commit of its own (a second name for the same commit —
/// a backup made before a rewrite — is not built on it). They must form one
/// line, each built on the one below it; two that are not are a
/// [`Dependents::Fork`], and Graft does not guess which one follows.
///
/// A branch checked out in another worktree is still in the line: it is
/// built on his, and leaving it behind is the duplicated history the chain
/// exists to prevent. [`plan`](super::plan) refuses it in words instead.
///
/// `default_branch` is the user's `graph.default_branch` when set; it wins
/// over git's guess.
///
/// # Errors
/// [`FactsError`] when a read fails, [`FactsError::Stopped`] when `stop`
/// asks.
pub fn dependents(
    repo: &RepoHandle,
    _git: &GitVersion,
    branch: &Tip,
    default_branch: Option<&RefName>,
    stop: &dyn Stop,
) -> Result<Dependents, FactsError> {
    if stop.should_stop() {
        return Err(FactsError::Stopped);
    }
    let default = default_branch_of(repo, default_branch)?;
    let mut found: Vec<(Tip, Vec<Oid>)> = Vec::new();
    for head in local_branches(repo, Some(branch.oid), stop)? {
        if head.name == branch.name
            || Some(&head.name) == default.as_ref()
            || head.oid == branch.oid
        {
            continue;
        }
        if let Some(chain) = chain_above(repo, branch.oid, head.oid, stop)? {
            found.push((
                Tip {
                    name: head.name,
                    oid: head.oid,
                },
                chain,
            ));
        }
    }
    found.sort_by(|(a, a_chain), (b, b_chain)| {
        a_chain
            .len()
            .cmp(&b_chain.len())
            .then_with(|| a.name.cmp(&b.name))
    });
    for pair in found.windows(2) {
        let [(lower, _), (upper, upper_chain)] = pair else {
            continue;
        };
        if lower.oid != upper.oid && !upper_chain.contains(&lower.oid) {
            return Ok(Dependents::Fork {
                a: ref_name(&lower.name)?,
                b: ref_name(&upper.name)?,
            });
        }
    }
    let mut branches = vec![branch.clone()];
    branches.extend(found.into_iter().map(|(tip, _)| tip));
    Ok(Dependents::Line(BranchLine { branches }))
}

/// The local branches a rewrite of the line leaves on old commits (m7/37):
/// every one holding `oldest_replaced` that is not in `line`, by name, the
/// first [`LEFT_BEHIND_CAP`] with the old commit each stays on, and how many
/// more there are.
///
/// The replaced commits are one straight chain (a merge in range is
/// refused), so a branch holding any of them holds the oldest: one
/// `for-each-ref --contains`. Where each stays is where it meets `old_top`,
/// the line's top before the rewrite: one `merge-base` per named branch. A
/// mid-chain fork meets it at the fork; a branch parked on a replaced
/// commit (a backup at his tip) at that commit.
///
/// `_git` is unused: the reads are the same on every git Graft supports.
///
/// # Errors
/// [`FactsError`] when a read fails, [`FactsError::Stopped`] when `stop`
/// asks.
pub(crate) fn left_behind(
    repo: &RepoHandle,
    _git: &GitVersion,
    oldest_replaced: Oid,
    old_top: Oid,
    line: &[RefName],
    stop: &dyn Stop,
) -> Result<(Vec<LeftBehind>, usize), FactsError> {
    let (named, more) = off_line(repo, oldest_replaced, old_top, line, stop)?;
    Ok((
        named
            .into_iter()
            .map(|off| LeftBehind {
                branch: off.branch,
                on: off.at,
            })
            .collect(),
        more,
    ))
}

/// A local branch off the line that holds a commit the rewrite replaces:
/// what [`left_behind`] names, with what a fork's fate is read from
/// (ADR-0059 §1).
#[derive(Clone, Debug)]
pub(crate) struct OffLine {
    /// Full name.
    pub(crate) branch: RefName,
    /// Its tip now.
    pub(crate) tip: Oid,
    /// Where it meets the line's old top: its fork point.
    pub(crate) at: Oid,
    /// The worktree it is checked out in, when that is not this one.
    pub(crate) elsewhere: Option<PathBuf>,
}

/// [`left_behind`]'s reads, whole: the first [`LEFT_BEHIND_CAP`] branches
/// off `line` holding `oldest_replaced`, in `for-each-ref` order, and how
/// many more there are.
///
/// # Errors
/// [`FactsError`] when a read fails, [`FactsError::Stopped`] when `stop`
/// asks.
pub(crate) fn off_line(
    repo: &RepoHandle,
    oldest_replaced: Oid,
    old_top: Oid,
    line: &[RefName],
    stop: &dyn Stop,
) -> Result<(Vec<OffLine>, usize), FactsError> {
    let off_line: Vec<LocalBranch> = local_branches(repo, Some(oldest_replaced), stop)?
        .into_iter()
        .filter(|local| {
            !line
                .iter()
                .any(|name| name.as_bytes() == local.name.as_bytes())
        })
        .collect();
    let more = off_line.len().saturating_sub(LEFT_BEHIND_CAP);
    let here = repo.workdir().map(canonical);
    let mut named = Vec::with_capacity(off_line.len().min(LEFT_BEHIND_CAP));
    for local in off_line.into_iter().take(LEFT_BEHIND_CAP) {
        named.push(OffLine {
            branch: ref_name(&local.name)?,
            tip: local.oid,
            at: merge_base(repo, local.oid, old_top, stop)?,
            elsewhere: local
                .worktree
                .filter(|worktree| here.as_ref() != Some(&canonical(worktree))),
        });
    }
    Ok((named, more))
}

/// A fork's own commits, `at..tip`, each with its first parent, oldest
/// first — or the first merge among them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum OwnCommits {
    /// No merge: `(commit, parent)`, oldest first.
    Line(Vec<(Oid, Oid)>),
    /// A merge lies in `at..tip`.
    Merge(Oid),
}

/// One `rev-list --reverse --parents at..tip`.
///
/// # Errors
/// [`FactsError`] when the read fails or does not parse;
/// [`FactsError::Stopped`].
pub(crate) fn own_commits(
    repo: &RepoHandle,
    at: Oid,
    tip: Oid,
    stop: &dyn Stop,
) -> Result<OwnCommits, FactsError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("rev-list"),
            GitArg::Flag("--reverse"),
            GitArg::Flag("--parents"),
            GitArg::Oid(tip),
            GitArg::Word(format!("^{at}")),
        ])
        .run(stop)
        .map_err(FactsError::of_history)?;
    let text = String::from_utf8_lossy(&output.stdout);
    let mut own = Vec::new();
    for line in text.lines() {
        let ids = line
            .split_whitespace()
            .map(parse_oid)
            .collect::<Result<Vec<Oid>, FactsError>>()?;
        match ids.as_slice() {
            [commit, parent] => own.push((*commit, *parent)),
            [commit, _, _, ..] => return Ok(OwnCommits::Merge(*commit)),
            _ => return Err(unparsable(format!("rev-list --parents printed {line:?}"))),
        }
    }
    Ok(OwnCommits::Line(own))
}

/// Where `a` and `b` meet. Both hold the oldest replaced commit, so there
/// is always one.
fn merge_base(repo: &RepoHandle, a: Oid, b: Oid, stop: &dyn Stop) -> Result<Oid, FactsError> {
    let output = GitInvocation::new(repo)
        .args([GitArg::Flag("merge-base"), GitArg::Oid(a), GitArg::Oid(b)])
        .run(stop)
        .map_err(FactsError::of_history)?;
    let text = String::from_utf8_lossy(&output.stdout);
    parse_oid(text.trim())
}

/// The commits from `tip` (excluded) up `above`'s first-parent chain, newest
/// first — `None` unless that chain reaches `tip` with no merge on the way.
fn chain_above(
    repo: &RepoHandle,
    tip: Oid,
    above: Oid,
    stop: &dyn Stop,
) -> Result<Option<Vec<Oid>>, FactsError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("rev-list"),
            GitArg::Flag("--first-parent"),
            GitArg::Flag("--parents"),
            GitArg::Oid(above),
            GitArg::Word(format!("^{tip}")),
        ])
        .run(stop)
        .map_err(FactsError::of_history)?;
    let text = String::from_utf8_lossy(&output.stdout);
    let mut chain = Vec::new();
    let mut last_parent = None;
    for line in text.lines() {
        let ids = line
            .split_whitespace()
            .map(parse_oid)
            .collect::<Result<Vec<Oid>, FactsError>>()?;
        let [commit, parent] = ids.as_slice() else {
            // A merge, or a root: not a straight line down to `tip`.
            return Ok(None);
        };
        chain.push(*commit);
        last_parent = Some(*parent);
    }
    Ok((last_parent == Some(tip)).then_some(chain))
}

/// A local branch as `for-each-ref` lists it.
struct LocalBranch {
    name: String,
    oid: Oid,
    /// The worktree it is checked out in, if any.
    worktree: Option<PathBuf>,
}

/// Every local branch, or only those containing `contains`.
fn local_branches(
    repo: &RepoHandle,
    contains: Option<Oid>,
    stop: &dyn Stop,
) -> Result<Vec<LocalBranch>, FactsError> {
    let mut args = vec![
        GitArg::Flag("for-each-ref"),
        GitArg::Word("--format=%(refname)%00%(objectname)%00%(worktreepath)".to_owned()),
    ];
    if let Some(oid) = contains {
        args.push(GitArg::Flag("--contains"));
        args.push(GitArg::Oid(oid));
    }
    args.push(GitArg::Word("refs/heads/".to_owned()));
    let output = GitInvocation::new(repo)
        .args(args)
        .run(stop)
        .map_err(FactsError::of_history)?;
    let text = String::from_utf8(output.stdout).map_err(|error| {
        FactsError::NonUtf8Ref(String::from_utf8_lossy(error.as_bytes()).into_owned())
    })?;
    text.lines()
        .map(|line| {
            let mut fields = line.split('\0');
            let (Some(name), Some(oid), worktree) = (fields.next(), fields.next(), fields.next())
            else {
                return Err(unparsable(format!("for-each-ref line {line:?}")));
            };
            Ok(LocalBranch {
                name: name.to_owned(),
                oid: parse_oid(oid)?,
                worktree: worktree.filter(|path| !path.is_empty()).map(PathBuf::from),
            })
        })
        .collect()
}

/// The first of `branches` checked out in a worktree other than this one,
/// and where.
fn checked_out_elsewhere(
    repo: &RepoHandle,
    branches: &[Tip],
    stop: &dyn Stop,
) -> Result<Option<(RefName, PathBuf)>, FactsError> {
    let here = repo.workdir().map(canonical);
    for local in local_branches(repo, None, stop)? {
        let Some(worktree) = local.worktree else {
            continue;
        };
        if !branches.iter().any(|tip| tip.name == local.name) {
            continue;
        }
        if here.as_ref() != Some(&canonical(&worktree)) {
            return Ok(Some((ref_name(&local.name)?, worktree)));
        }
    }
    Ok(None)
}

/// A path with symlinks resolved, for comparing two spellings of one
/// worktree (`/tmp` and `/private/tmp` on macOS). One that cannot be
/// resolved is compared as written.
fn canonical(path: &std::path::Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}
