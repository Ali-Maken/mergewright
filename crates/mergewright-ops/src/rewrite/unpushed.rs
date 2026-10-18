//! "Your commits" — the one range absorb places into and `a` accepts a
//! target from (06c ruling; 06e ruling, 2026-09-27: on every base).
//!
//! `HEAD`'s first-parent commits, walked down from `HEAD`. At each commit
//! the walk stops, checking in this order:
//! 1. a `refs/remotes/*` ref reaches it → [`RangeStop::Pushed`] (first: an
//!    upstream base is itself pushed, and "already pushed" is the true
//!    reason there);
//! 2. it is the base → [`RangeStop::Base`];
//! 3. a merge → [`RangeStop::Merge`];
//! 4. the first commit → [`RangeStop::Root`];
//! 5. only with no base below `HEAD`: [`ABSORB_FALLBACK_DEPTH`] already
//!    taken → [`RangeStop::Cap`].
//!
//! The walk's entry (06f): a base below `HEAD` → rules 1–4; no base at all
//! (no shared history, nothing to measure from) → the fallback (rules 1, 3,
//! 4, 5); a base that *is* `HEAD` → the fallback on the default or only
//! local branch (06c's premise), else nothing is its own:
//! [`RangeStop::NotItsOwn`] — unless `HEAD` itself is pushed, which rule 1
//! answers first, so a tracked branch pushed to its tip says "pushed".

use std::collections::BTreeSet;

use mergewright_core::oid::Oid;
use mergewright_core::ref_name::RefName;
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::{GitArg, GitInvocation, Stop};

use crate::facts::FactsError;
use crate::operation::Tip;

use super::reads::{base_of, default_branch_of, parents_of, parse_oid, unparsable};

/// How many of his unpushed commits absorb and `a` reach where no base
/// below `HEAD` bounds them (the default or only branch, or no base): git-absorb's rule, its last 10
/// commits (`docs/research/interactive-rebase.md` :135). Older ones he may
/// have forgotten; a pick he points at further back is `R`'s.
pub const ABSORB_FALLBACK_DEPTH: usize = 10;

/// His commits, the walk that found them, and what ended it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct YourCommits {
    /// Parent of the oldest commit taken; `HEAD` when none were taken.
    pub(crate) base: Oid,
    /// First-parent, newest first.
    pub(crate) commits: Vec<Oid>,
    /// What ended the walk.
    pub(crate) stop: RangeStop,
}

/// What ended [`your_commits`]'s walk down the first-parent chain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RangeStop {
    /// The next commit is on a remote-tracking ref (`refs/remotes/*`), and
    /// so is everything below it.
    Pushed,
    /// The next commit is the branch's base (where it left its upstream or
    /// the default branch).
    Base,
    /// The base is `HEAD` itself, and the branch is neither the default
    /// branch nor the only one: a branch at or behind `main`'s tip. Nothing
    /// was taken.
    NotItsOwn,
    /// The next commit is a merge (ADR-0047 §5 refuses merges in a range).
    Merge(Oid),
    /// The next commit is the repository's first: nothing to replay it
    /// onto.
    Root,
    /// [`ABSORB_FALLBACK_DEPTH`] commits taken (no base below `HEAD` only).
    Cap,
}

/// One commit the walk looks at: its parents, and the stop it is if a
/// remote does not reach it first.
struct Candidate {
    commit: Oid,
    parents: Vec<Oid>,
    ends: Option<RangeStop>,
}

/// His commits, for absorb (`plan`'s absorb and `absorb_plan_map`) and `a`
/// (`AmendInto`). See the module doc for the walk. `base_of` itself is
/// unchanged: `R` and others call it.
///
/// Reads: the base; the chain (`rev-list --first-parent --parents`); on
/// the fallback, whether the branch is the default or only one; then one
/// `rev-list <chain> --not --remotes` for which of the chain no remote
/// reaches — git's own walk, so a commit a remote ref reaches through a
/// merge's second parent counts as pushed. Not mergewright-git's
/// `contained_in`: one `for-each-ref --contains` per commit costs seconds
/// each on a repository with 50k refs (mergewright-git MAP, `unreached`).
pub(crate) fn your_commits(
    repo: &RepoHandle,
    head: &Tip,
    default_branch: Option<&RefName>,
    stop: &dyn Stop,
) -> Result<YourCommits, FactsError> {
    // 06f ruling, in order: a base below `HEAD` bounds the walk; the default
    // or only branch falls back; a base that *is* `HEAD` means the branch has
    // none of its own; no base at all (no shared history) falls back too.
    let base = base_of(repo, head, default_branch, stop)?;
    let candidates = match base {
        Some(base) if base != head.oid => down_to_base(repo, head.oid, base, stop)?,
        // No base: the same fallback as the default branch, one read fewer.
        None => capped(repo, head.oid, stop)?,
        Some(_) if falls_back(repo, head, default_branch, stop)? => capped(repo, head.oid, stop)?,
        Some(_) => {
            let parents = parents_of(repo, head.oid, stop)?;
            vec![Candidate {
                commit: head.oid,
                parents,
                ends: Some(RangeStop::NotItsOwn),
            }]
        }
    };
    let unpushed = not_on_a_remote(repo, &candidates, stop)?;
    let mut commits = Vec::new();
    let mut below = head.oid;
    for candidate in &candidates {
        let ended = if unpushed.contains(&candidate.commit) {
            candidate.ends
        } else {
            // Everything below a pushed commit is pushed too.
            Some(RangeStop::Pushed)
        };
        if let Some(stop) = ended {
            return Ok(YourCommits {
                base: below,
                commits,
                stop,
            });
        }
        commits.push(candidate.commit);
        below = candidate
            .parents
            .first()
            .copied()
            .ok_or_else(|| unparsable("a commit your_commits took has no parent".to_owned()))?;
    }
    Err(unparsable(
        "your_commits' walk ended without a stop".to_owned(),
    ))
}

/// The branch is the default branch (the user's override when set) or the
/// only local branch: the two places the fallback's premise holds.
fn falls_back(
    repo: &RepoHandle,
    head: &Tip,
    default_branch: Option<&RefName>,
    stop: &dyn Stop,
) -> Result<bool, FactsError> {
    if default_branch_of(repo, default_branch)?.as_deref() == Some(head.name.as_str()) {
        return Ok(true);
    }
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("for-each-ref"),
            GitArg::Flag("--count=2"),
            GitArg::Word("--format=%(refname)".to_owned()),
            GitArg::Word("refs/heads/".to_owned()),
        ])
        .run(stop)
        .map_err(FactsError::of_history)?;
    Ok(String::from_utf8_lossy(&output.stdout).lines().count() == 1)
}

/// `HEAD` down to the base: every first-parent commit above it, ending at
/// the first merge or the first commit, else at the base itself.
fn down_to_base(
    repo: &RepoHandle,
    head: Oid,
    base: Oid,
    stop: &dyn Stop,
) -> Result<Vec<Candidate>, FactsError> {
    let mut candidates = chain(
        repo,
        [GitArg::Oid(head), GitArg::Word(format!("^{base}"))],
        stop,
    )?
    .into_iter()
    .map(|(commit, parents)| Candidate {
        ends: shape_stop(commit, &parents),
        commit,
        parents,
    })
    .collect::<Vec<_>>();
    if let Some(end) = candidates.iter().position(|c| c.ends.is_some()) {
        candidates.truncate(end + 1);
        return Ok(candidates);
    }
    candidates.push(Candidate {
        commit: base,
        parents: parents_of(repo, base, stop)?,
        ends: Some(RangeStop::Base),
    });
    Ok(candidates)
}

/// The fallback: `HEAD` and the next [`ABSORB_FALLBACK_DEPTH`] first-parent
/// commits, ending at the first merge or the first commit, else at the
/// one past the cap.
fn capped(repo: &RepoHandle, head: Oid, stop: &dyn Stop) -> Result<Vec<Candidate>, FactsError> {
    let mut candidates = Vec::new();
    let found = chain(
        repo,
        [
            GitArg::Word(format!("--max-count={}", ABSORB_FALLBACK_DEPTH + 1)),
            GitArg::Oid(head),
        ],
        stop,
    )?;
    for (taken, (commit, parents)) in found.into_iter().enumerate() {
        let ends = shape_stop(commit, &parents)
            .or_else(|| (taken == ABSORB_FALLBACK_DEPTH).then_some(RangeStop::Cap));
        candidates.push(Candidate {
            commit,
            parents,
            ends,
        });
        if ends.is_some() {
            break;
        }
    }
    Ok(candidates)
}

/// A merge or the first commit ends the walk whatever the base.
fn shape_stop(commit: Oid, parents: &[Oid]) -> Option<RangeStop> {
    match parents {
        [] => Some(RangeStop::Root),
        [_] => None,
        _ => Some(RangeStop::Merge(commit)),
    }
}

/// `rev-list --first-parent --parents <revs>`: each commit, newest first,
/// with its parents.
fn chain<const N: usize>(
    repo: &RepoHandle,
    revs: [GitArg; N],
    stop: &dyn Stop,
) -> Result<Vec<(Oid, Vec<Oid>)>, FactsError> {
    let mut args = vec![
        GitArg::Flag("rev-list"),
        GitArg::Flag("--first-parent"),
        GitArg::Flag("--parents"),
    ];
    args.extend(revs);
    let output = GitInvocation::new(repo)
        .args(args)
        .run(stop)
        .map_err(FactsError::of_history)?;
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| {
            let mut ids = line.split_whitespace().map(parse_oid);
            let Some(commit) = ids.next() else {
                return Err(unparsable("an empty rev-list line".to_owned()));
            };
            Ok((commit?, ids.collect::<Result<Vec<Oid>, FactsError>>()?))
        })
        .collect()
}

/// Which of `candidates` (a first-parent run, newest first) no
/// remote-tracking ref reaches: `rev-list <newest> ^<each parent of the
/// oldest> --not --remotes`. Only the oldest may have more than one parent,
/// so the positive side is exactly the run.
fn not_on_a_remote(
    repo: &RepoHandle,
    candidates: &[Candidate],
    stop: &dyn Stop,
) -> Result<BTreeSet<Oid>, FactsError> {
    let (Some(newest), Some(oldest)) = (candidates.first(), candidates.last()) else {
        return Ok(BTreeSet::new());
    };
    let mut args = vec![GitArg::Flag("rev-list"), GitArg::Oid(newest.commit)];
    args.extend(
        oldest
            .parents
            .iter()
            .map(|parent| GitArg::Word(format!("^{parent}"))),
    );
    args.push(GitArg::Flag("--not"));
    args.push(GitArg::Flag("--remotes"));
    let output = GitInvocation::new(repo)
        .args(args)
        .run(stop)
        .map_err(FactsError::of_history)?;
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(parse_oid)
        .collect()
}
