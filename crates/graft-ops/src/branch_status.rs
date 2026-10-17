//! What a branch was built on, and the four answers the graph's right pane
//! gives about it: pushed? merged? behind its base? who worked on it?
//! (design note `ease-pass`, G12; hard-to-reverse choice 1).
//!
//! **Built on** is measured, not recorded: a branch's base is the candidate
//! it has the fewest commits apart from (fewest commits only on the branch),
//! ties going trunk, then integration, then by name. The **route** repeats
//! that from each base until it reaches the trunk.
//!
//! Reads only, every one through `count_between` / `commits_between` /
//! `commits_from`, so
//! each honours a [`Stop`] and fails with [`RunError`]. Branches are named
//! by full ref name (`refs/heads/main`); `graft-graph` turns names into lane
//! identities, not this crate.

use graft_core::oid::Oid;
use graft_git::history_reads::count_between;
use graft_git::repo::RepoHandle;
use graft_git::run::{RunError, Stop};

mod status;

pub use status::{Behind, BranchStatus, Merged, Pushed, Who, branch_status};

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

/// The base among `measured`: the candidate with the fewest branch-only
/// commits, ties by kind (trunk first), then by name. Never `branch` itself,
/// and never a candidate with no branch-only commits: one that already
/// holds every commit of the branch is not something it was built on.
/// Pure.
pub fn choose_base<'a>(
    branch: &str,
    measured: &'a [(Candidate, Apart)],
    _at_most: CandidateKind,
) -> Option<&'a Candidate> {
    measured
        .iter()
        .filter(|(candidate, apart)| candidate.name != branch && apart.branch_only != 0)
        .min_by(|(a, a_apart), (b, b_apart)| {
            (a_apart.branch_only, a.kind, &a.name).cmp(&(b_apart.branch_only, b.kind, &b.name))
        })
        .map(|(candidate, _)| candidate)
}

/// The bases `branch` was built on, nearest first, ending at the trunk when
/// the walk reaches it. Empty for the trunk itself, and for a branch every
/// candidate already holds.
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
    let first = measure(repo, branch, candidates, &[], stop)?;
    route_from(repo, branch, candidates, &first, stop)
}

/// The rest of [`route`], from the branch's own measurements.
fn route_from(
    repo: &RepoHandle,
    branch: &Candidate,
    candidates: &[Candidate],
    first: &[(Candidate, Apart)],
    stop: &dyn Stop,
) -> Result<Vec<Candidate>, RunError> {
    let mut route: Vec<Candidate> = Vec::new();
    let Some(mut base) = choose_base(&branch.name, first, branch.kind).cloned() else {
        return Ok(route);
    };
    loop {
        route.push(base.clone());
        if base.kind == CandidateKind::Trunk || route.len() == ROUTE_DEPTH {
            return Ok(route);
        }
        let walked: Vec<Oid> = std::iter::once(branch.tip)
            .chain(route.iter().map(|c| c.tip))
            .collect();
        let measured = measure(repo, &base, candidates, &walked, stop)?;
        match choose_base(&base.name, &measured, base.kind) {
            Some(next) => base = next.clone(),
            None => return Ok(route),
        }
    }
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
