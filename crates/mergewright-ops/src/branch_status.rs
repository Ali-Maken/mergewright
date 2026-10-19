//! What a branch was built on, and the four answers the graph's right pane
//! gives about it: pushed? merged? behind its base? who worked on it?
//! (design note `ease-pass`, G12; hard-to-reverse choice 1).
//!
//! Git does not record who was cut from whom, and between two ordinary
//! branches a commit count cannot tell a base from a sibling. So these
//! answers use only what git proves: a branch's **kind** (trunk,
//! integration, other), **containment**, and the **first-parent line**
//! (ease-pass 01r). "Merged" must be proven; "not merged" is the default.
//!
//! - **Built on** ([`choose_base`], R1): the nearest candidate (fewest
//!   commits only on the branch; ties by kind, then name) that is not above
//!   the walker's kind, and is the trunk, an integration branch, or an
//!   ordinary branch lying wholly under the walker (a dependent branch's
//!   base). One sharing no history is passed over.
//! - **Footing** (R2): a branch a long-lived line holds is either **on its
//!   line** (no commits of its own) or **merged into** it; failing that, one
//!   an ordinary base took in by a merge is merged into that; else its own.
//! - The **route** repeats the step from each base, to the trunk: an
//!   on-line branch steps to its line, any other by R1.
//!
//! Reads only, every one through `count_between` / `commits_between` /
//! `commits_from` / `compare_span` / `entry_into`, so each honours a
//! [`Stop`] and fails with [`RunError`]. Branches are named by full ref
//! name (`refs/heads/main`); `mergewright-graph` turns names into lane
//! identities, not this crate.

use mergewright_core::oid::Oid;
use mergewright_git::history_reads::{CompareSpan, compare_span, count_between};
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::{RunError, Stop};

mod footing;
mod status;

pub use status::{Author, Behind, BranchStatus, Merged, Pushed, Who, branch_status};

/// How far the route walks down before it gives up on reaching the trunk.
pub const ROUTE_DEPTH: usize = 8;

/// What sort of branch a candidate is; also the tie order when two
/// candidates are equally near (trunk first).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CandidateKind {
    /// The default branch (`main`).
    Trunk,
    /// An integration branch (ADR-0001's set: `develop`, `release/*`, …).
    Integration,
    /// Any other branch.
    Other,
}

/// A branch that could be another's base.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Candidate {
    /// Full ref name: `refs/heads/main`, as [`Tip::name`].
    pub name: String,
    /// The commit it points at.
    pub tip: Oid,
    /// What sort of branch it is.
    pub kind: CandidateKind,
}

/// How far a branch and a candidate base are apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Apart {
    /// Commits on the branch the candidate does not have.
    pub branch_only: usize,
    /// Commits on the candidate the branch does not have.
    pub base_only: usize,
}

/// A full ref name as people say it: `refs/heads/x` → `x`,
/// `refs/remotes/o/x` → `o/x`, `refs/tags/t` → `t`; anything else
/// unchanged.
pub fn short_name(full: &str) -> &str {
    ["refs/heads/", "refs/remotes/", "refs/tags/"]
        .iter()
        .find_map(|prefix| full.strip_prefix(prefix))
        .unwrap_or(full)
}

/// The base among `measured` (R1). Eligible: not `branch` itself; has
/// branch-only commits (one holding every commit of the branch is not
/// something it was built on); kind no higher than `at_most`; and the
/// trunk, an integration branch, or an ordinary branch with no commits the
/// branch lacks (all of it under the branch, CONTEXT's **dependent
/// branch**). An ordinary branch with commits of its own may be a sibling,
/// or cut from an older commit, and git cannot tell which.
///
/// Among the eligible: fewest branch-only commits, then kind (trunk
/// first), then name. Pure.
pub fn choose_base<'a>(
    branch: &str,
    measured: &'a [(Candidate, Apart)],
    at_most: CandidateKind,
) -> Option<&'a Candidate> {
    base_index(branch, measured, at_most)
        .and_then(|at| measured.get(at))
        .map(|(candidate, _)| candidate)
}

/// Where in `measured` [`choose_base`]'s answer is.
fn base_index(
    branch: &str,
    measured: &[(Candidate, Apart)],
    at_most: CandidateKind,
) -> Option<usize> {
    measured
        .iter()
        .enumerate()
        .filter(|(_, (candidate, apart))| {
            candidate.name != branch
                && apart.branch_only != 0
                && candidate.kind <= at_most
                && (candidate.kind != CandidateKind::Other || apart.base_only == 0)
        })
        .min_by(|(_, (a, a_apart)), (_, (b, b_apart))| {
            (a_apart.branch_only, a.kind, &a.name).cmp(&(b_apart.branch_only, b.kind, &b.name))
        })
        .map(|(at, _)| at)
}

/// [`choose_base`] against real history: a choice sharing no history with
/// `walker` (an orphan line such as `gh-pages`) is passed over and the next
/// is taken. A candidate with no commits the walker lacks lies under it, so
/// only the others are asked.
fn pick_base(
    repo: &RepoHandle,
    walker: &Candidate,
    measured: &[(Candidate, Apart)],
    at_most: CandidateKind,
    stop: &dyn Stop,
) -> Result<Option<(Candidate, Apart)>, RunError> {
    let mut left: Vec<(Candidate, Apart)> = measured.to_vec();
    while let Some(at) = base_index(&walker.name, &left, at_most) {
        let (base, apart) = left.swap_remove(at);
        let unrelated = apart.base_only != 0
            && compare_span(repo, base.tip, walker.tip, stop)? == CompareSpan::NoSharedHistory;
        if !unrelated {
            return Ok(Some((base, apart)));
        }
    }
    Ok(None)
}

/// The bases `branch` was built on, nearest first, ending at the trunk when
/// the walk reaches it. Each step: a walker on a long-lived line's
/// first-parent line (R2) steps to that line; any other steps by R1
/// ([`choose_base`], `at_most` the walker's kind). So an integration branch
/// is never built on an ordinary one. Empty for the trunk itself, for an
/// orphan, and for a branch nothing eligible lies under.
///
/// At most [`ROUTE_DEPTH`] long. A candidate at the tip of `branch` or of a
/// base already on the route is never measured again, so the walk cannot
/// loop or step back up through a base that has moved on.
///
/// `candidates` are the caller's to choose; a branch's own upstream should
/// not be among them, or it would read as the base.
///
/// # Errors
/// [`RunError`] from a read, [`RunError::Stopped`] included.
pub fn route(
    repo: &RepoHandle,
    branch: &Candidate,
    candidates: &[Candidate],
    stop: &dyn Stop,
) -> Result<Vec<Candidate>, RunError> {
    if stop.should_stop() {
        return Err(RunError::Stopped);
    }
    if branch.kind == CandidateKind::Trunk {
        return Ok(Vec::new());
    }
    let measured = measure(repo, branch, candidates, &[], stop)?;
    let first = step(repo, branch, &measured, stop)?;
    let route = walk(repo, branch, candidates, first, stop)?;
    Ok(route.into_iter().map(|(base, _)| base).collect())
}

/// One step of the route from `walker`, given its measurements: its
/// long-lived line when it sits on one, else R1.
fn step(
    repo: &RepoHandle,
    walker: &Candidate,
    measured: &[(Candidate, Apart)],
    stop: &dyn Stop,
) -> Result<Option<(Candidate, Apart)>, RunError> {
    match footing::held_by_long_lived(repo, walker, measured, stop)? {
        Some(held) if held.on_line(walker.tip) => Ok(Some(held.holder.clone())),
        _ => pick_base(repo, walker, measured, walker.kind, stop),
    }
}

/// The route from `branch`'s first base on, each base with how far it is
/// from the walker that chose it (`route[0]`'s from `branch`).
fn walk(
    repo: &RepoHandle,
    branch: &Candidate,
    candidates: &[Candidate],
    first: Option<(Candidate, Apart)>,
    stop: &dyn Stop,
) -> Result<Vec<(Candidate, Apart)>, RunError> {
    let mut route: Vec<(Candidate, Apart)> = Vec::new();
    let mut next = first;
    while let Some((base, apart)) = next {
        let walker = base.clone();
        route.push((base, apart));
        if walker.kind == CandidateKind::Trunk || route.len() == ROUTE_DEPTH {
            break;
        }
        let skip: Vec<Oid> = std::iter::once(branch.tip)
            .chain(route.iter().map(|(c, _)| c.tip))
            .collect();
        let measured = measure(repo, &walker, candidates, &skip, stop)?;
        next = step(repo, &walker, &measured, stop)?;
    }
    Ok(route)
}

/// `branch` against every candidate but itself and those at a tip in
/// `skip`: two counts each.
fn measure(
    repo: &RepoHandle,
    branch: &Candidate,
    candidates: &[Candidate],
    skip: &[Oid],
    stop: &dyn Stop,
) -> Result<Vec<(Candidate, Apart)>, RunError> {
    candidates
        .iter()
        .filter(|c| c.name != branch.name && !skip.contains(&c.tip))
        .map(|c| {
            let apart = if c.tip == branch.tip {
                // The same commit: nothing apart, no read needed.
                Apart {
                    branch_only: 0,
                    base_only: 0,
                }
            } else {
                Apart {
                    branch_only: count_between(repo, c.tip, branch.tip, stop)?,
                    base_only: count_between(repo, branch.tip, c.tip, stop)?,
                }
            };
            Ok((c.clone(), apart))
        })
        .collect()
}

#[cfg(test)]
mod tests;
