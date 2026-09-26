//! "Your commits" on a branch with no base of its own — the default branch,
//! or the only branch, with no upstream — where the usual base comes out as
//! `HEAD` itself (planner ruling, 06c Question 2). Absorb and `a` both ask
//! [`unpushed_range`], so the commits one may change are the other's.

use std::collections::BTreeSet;

use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, Stop};

use crate::facts::FactsError;
use crate::operation::Tip;

use super::reads::{base_of, parse_oid, unparsable};

/// How many of his unpushed commits absorb and `a` reach on a branch with
/// no base of its own: git-absorb's rule, its last 10 commits
/// (`docs/research/interactive-rebase.md` :135). Older ones he may have
/// forgotten; a pick he points at further back is `R`'s.
pub const ABSORB_FALLBACK_DEPTH: usize = 10;

/// His unpushed commits, the walk that found them, and what ended it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct UnpushedRange {
    /// The parent of the oldest commit taken; `HEAD` when none was.
    pub(crate) base: Oid,
    /// The commits taken, newest first.
    pub(crate) commits: Vec<Oid>,
    /// What ended the walk.
    pub(crate) stop: RangeStop,
}

/// What ended [`unpushed_range`]'s walk down the first-parent chain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RangeStop {
    /// [`ABSORB_FALLBACK_DEPTH`] commits taken.
    Cap,
    /// The next commit is on a remote-tracking ref (`refs/remotes/*`).
    Pushed,
    /// The next commit is a merge (ADR-0047 §5 refuses merges in a range).
    Merge(Oid),
    /// The next commit is the repository's first: nothing to replay it
    /// onto.
    Root,
}

/// Which commits are his, for absorb and `a`.
pub(crate) enum YourCommits {
    /// The branch has a base of its own (its upstream, or the default
    /// branch it left): the commits above it, whatever a remote holds.
    Above(Oid),
    /// The base came out as `HEAD` itself, or there is none: his unpushed
    /// commits, at most [`ABSORB_FALLBACK_DEPTH`].
    Unpushed(UnpushedRange),
}

/// [`base_of`] when it gives a base below `HEAD`; else [`unpushed_range`].
/// `base_of` itself is unchanged: `R` and others call it.
pub(crate) fn your_commits(
    repo: &RepoHandle,
    head: &Tip,
    default_branch: Option<&RefName>,
    stop: &dyn Stop,
) -> Result<YourCommits, FactsError> {
    match base_of(repo, head, default_branch, stop)? {
        Some(base) if base != head.oid => Ok(YourCommits::Above(base)),
        _ => Ok(YourCommits::Unpushed(unpushed_range(repo, head, stop)?)),
    }
}

/// Walk the first-parent chain down from `head`, taking commits while all
/// hold: fewer than [`ABSORB_FALLBACK_DEPTH`] taken; not a merge; not the
/// first commit; not reachable from any remote-tracking ref.
///
/// Two reads: the chain (`rev-list --first-parent --parents`), then which of
/// its merge-free prefix no remote-tracking ref reaches (`rev-list <prefix>
/// --not --remotes`, git's own walk: a commit a remote ref reaches through a
/// merge's second parent counts as pushed). Not graft-git's `contained_in`:
/// one `for-each-ref --contains` per commit costs seconds each on a
/// repository with 50k refs (graft-git MAP, `unreached`).
pub(crate) fn unpushed_range(
    repo: &RepoHandle,
    head: &Tip,
    stop: &dyn Stop,
) -> Result<UnpushedRange, FactsError> {
    let chain = first_parent_chain(repo, head.oid, stop)?;
    // The merge-free, root-free prefix, at most the cap, and what ends it.
    let mut prefix: Vec<(Oid, Oid)> = Vec::new();
    let mut ended = RangeStop::Cap;
    for (commit, parents) in &chain {
        if prefix.len() == ABSORB_FALLBACK_DEPTH {
            ended = RangeStop::Cap;
            break;
        }
        match parents.as_slice() {
            [] => {
                ended = RangeStop::Root;
                break;
            }
            [parent] => prefix.push((*commit, *parent)),
            _ => {
                ended = RangeStop::Merge(*commit);
                break;
            }
        }
    }
    let unpushed = not_on_a_remote(repo, &prefix, stop)?;
    let mut commits = Vec::new();
    let mut base = head.oid;
    for (commit, parent) in &prefix {
        if !unpushed.contains(commit) {
            // Everything below a pushed commit is pushed too.
            ended = RangeStop::Pushed;
            break;
        }
        commits.push(*commit);
        base = *parent;
    }
    Ok(UnpushedRange {
        base,
        commits,
        stop: ended,
    })
}

/// `head` and its first-parent ancestors, newest first, each with its
/// parents: one more than the cap, so a walk that takes the cap still sees
/// the next.
fn first_parent_chain(
    repo: &RepoHandle,
    head: Oid,
    stop: &dyn Stop,
) -> Result<Vec<(Oid, Vec<Oid>)>, FactsError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("rev-list"),
            GitArg::Flag("--first-parent"),
            GitArg::Flag("--parents"),
            GitArg::Word(format!("--max-count={}", ABSORB_FALLBACK_DEPTH + 1)),
            GitArg::Oid(head),
        ])
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

/// Which of `prefix` (a merge-free first-parent run, newest first, each
/// with its parent) no remote-tracking ref reaches.
fn not_on_a_remote(
    repo: &RepoHandle,
    prefix: &[(Oid, Oid)],
    stop: &dyn Stop,
) -> Result<BTreeSet<Oid>, FactsError> {
    let (Some((newest, _)), Some((_, below))) = (prefix.first(), prefix.last()) else {
        return Ok(BTreeSet::new());
    };
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("rev-list"),
            GitArg::Oid(*newest),
            GitArg::Word(format!("^{below}")),
            GitArg::Flag("--not"),
            GitArg::Flag("--remotes"),
        ])
        .run(stop)
        .map_err(FactsError::of_history)?;
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(parse_oid)
        .collect()
}
