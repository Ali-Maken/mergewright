//! The right pane's four answers about one branch, read from the route's
//! measurements: pushed, merged, behind, who.

use std::collections::HashMap;

use graft_core::commit::Timestamp;
use graft_core::oid::Oid;
use graft_git::history_reads::{CommitSummary, commits_between, commits_from, count_between};
use graft_git::repo::RepoHandle;
use graft_git::run::{RunError, Stop};

use super::{Apart, Candidate, CandidateKind, measure, route_from};
use crate::operation::Tip;

/// The most branch-only commits `Who` reads; past it, the newest are kept.
const WHO_CAP: usize = 2_000;

/// The most commits the "stacked on top" check reads from one container.
/// A container it cannot decide within this is not counted as one.
const STACK_CAP: usize = 10_000;

/// The right pane's answers about one branch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BranchStatus {
    /// What it was built on, nearest first (see [`route`]).
    pub route: Vec<Candidate>,
    /// Against its upstream.
    pub pushed: Pushed,
    /// Whether another branch has taken it in.
    pub merged: Merged,
    /// How far its base has moved on since it left.
    pub behind: Behind,
    /// Who made its commits.
    pub who: Who,
}

/// The branch against its upstream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pushed {
    /// No upstream set.
    NoUpstream,
    /// Same commit as the upstream.
    InSync,
    /// Commits here the upstream lacks.
    ToPush(usize),
    /// Commits on the upstream not here.
    ToPull(usize),
    /// Both ways.
    Both {
        /// Commits here the upstream lacks.
        push: usize,
        /// Commits on the upstream not here.
        pull: usize,
    },
}

/// Whether the branch has been merged, and where. Names are full ref names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Merged {
    /// The trunk: nothing to merge it into.
    MainLine,
    /// Every commit of it is in this branch, by a merge (not because this
    /// branch was made on top of it).
    Into(String),
    /// Not merged yet; `into` is where it would go: its base, or the trunk.
    NotYet {
        /// Its nearest base, or the trunk when it has none.
        into: Option<String>,
    },
    /// No commits of its own: it sits on a long-lived branch's line.
    NothingToMerge,
}

/// How far the branch's base has moved on since the branch left it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Behind {
    /// No base to be behind: the trunk with no upstream, or a branch every
    /// candidate already holds.
    NoBase,
    /// The base has nothing new.
    UpToDate {
        /// The base's full ref name.
        base: String,
    },
    /// The base has `count` commits the branch lacks.
    Behind {
        /// The base's full ref name.
        base: String,
        /// Commits on the base since the branch left it.
        count: usize,
    },
}

/// Who made the branch's own commits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Who {
    /// The trunk: everyone works on it, so no names.
    Everyone {
        /// When its tip was authored.
        last: Timestamp,
    },
    /// The authors of the branch's own commits.
    Authors {
        /// Up to two author names, most commits first (ties: most recent
        /// first); raw bytes, as the repository stores them.
        first: Vec<Vec<u8>>,
        /// How many more authors there are.
        others: usize,
        /// When the newest of its own commits was authored.
        last: Timestamp,
    },
    /// It has no commits of its own: it sits on a long-lived branch's line.
    NoOwnCommits,
}

/// [`BranchStatus`] for `branch`, measured against `candidates` and its
/// `upstream` (the remote-tracking ref it pulls from, if any).
///
/// # Errors
/// [`RunError`] from a read, [`RunError::Stopped`] included.
pub fn branch_status(
    repo: &RepoHandle,
    branch: &Candidate,
    candidates: &[Candidate],
    upstream: Option<&Tip>,
    stop: &dyn Stop,
) -> Result<BranchStatus, RunError> {
    if stop.should_stop() {
        return Err(RunError::Stopped);
    }
    let counts = match upstream {
        Some(up) => Some(UpstreamCounts {
            upstream: up,
            push: count_between(repo, up.oid, branch.tip, stop)?,
            pull: count_between(repo, branch.tip, up.oid, stop)?,
        }),
        None => None,
    };
    let pushed = counts
        .as_ref()
        .map_or(Pushed::NoUpstream, UpstreamCounts::pushed);

    if branch.kind == CandidateKind::Trunk {
        return trunk_status(repo, branch, pushed, counts.as_ref(), stop);
    }

    let measured = measure(repo, branch, candidates, &[], stop)?;
    let route = route_from(repo, branch, candidates, &measured, stop)?;
    // `route[0]` was chosen from `measured`, so it is always found there.
    let base = route
        .first()
        .and_then(|base| measured.iter().find(|(c, _)| c.name == base.name));
    let behind = match base {
        None => Behind::NoBase,
        Some((base, apart)) => behind_by(&base.name, apart.base_only),
    };

    let merged = match nearest_container(repo, branch, &measured, stop)? {
        Some(container) => Merged::Into(container.name.clone()),
        None => match route
            .first()
            .or_else(|| candidates.iter().find(|c| c.kind == CandidateKind::Trunk))
        {
            Some(into) => Merged::NotYet {
                into: Some(into.name.clone()),
            },
            // No base, no trunk among the candidates, nothing holding it:
            // nothing lies under this branch, so it is the main line there is.
            None => Merged::MainLine,
        },
    };

    let own = match route.first() {
        Some(base) => commits_between(repo, base.tip, branch.tip, WHO_CAP, stop)?,
        None => commits_from(repo, branch.tip, WHO_CAP, stop)?,
    };
    let who = who_of(&own.commits).ok_or_else(|| listed_nothing(branch))?;

    Ok(BranchStatus {
        route,
        pushed,
        merged,
        behind,
        who,
    })
}

/// A branch against its upstream, both ways.
struct UpstreamCounts<'a> {
    upstream: &'a Tip,
    /// Commits on the branch the upstream lacks.
    push: usize,
    /// Commits on the upstream the branch lacks.
    pull: usize,
}

impl UpstreamCounts<'_> {
    fn pushed(&self) -> Pushed {
        match (self.push, self.pull) {
            (0, 0) => Pushed::InSync,
            (push, 0) => Pushed::ToPush(push),
            (0, pull) => Pushed::ToPull(pull),
            (push, pull) => Pushed::Both { push, pull },
        }
    }
}

/// `count` commits on `base` the branch lacks.
fn behind_by(base: &str, count: usize) -> Behind {
    match count {
        0 => Behind::UpToDate {
            base: base.to_owned(),
        },
        count => Behind::Behind {
            base: base.to_owned(),
            count,
        },
    }
}

/// The trunk: no route, nothing to merge into, behind only its upstream,
/// worked on by everyone.
fn trunk_status(
    repo: &RepoHandle,
    trunk: &Candidate,
    pushed: Pushed,
    counts: Option<&UpstreamCounts<'_>>,
    stop: &dyn Stop,
) -> Result<BranchStatus, RunError> {
    let behind = counts.map_or(Behind::NoBase, |c| behind_by(&c.upstream.name, c.pull));
    let tip = commits_from(repo, trunk.tip, 1, stop)?;
    let who = Who::Everyone {
        last: newest(&tip.commits).ok_or_else(|| listed_nothing(trunk))?,
    };
    Ok(BranchStatus {
        route: Vec::new(),
        pushed,
        merged: Merged::MainLine,
        behind,
        who,
    })
}

/// The nearest branch that took `branch` in: holds every commit of it, is
/// not at the same commit, and was not simply made on top of it. Nearest =
/// fewest commits of its own the branch lacks; ties by kind, then name.
fn nearest_container<'a>(
    repo: &RepoHandle,
    branch: &Candidate,
    measured: &'a [(Candidate, Apart)],
    stop: &dyn Stop,
) -> Result<Option<&'a Candidate>, RunError> {
    let mut holders: Vec<&(Candidate, Apart)> = measured
        .iter()
        .filter(|(c, apart)| c.name != branch.name && apart.branch_only == 0 && c.tip != branch.tip)
        .collect();
    holders.sort_by(|(a, a_apart), (b, b_apart)| {
        (a_apart.base_only, a.kind, &a.name).cmp(&(b_apart.base_only, b.kind, &b.name))
    });
    for (holder, _) in holders {
        if !stacked_on(repo, branch.tip, holder.tip, stop)? {
            return Ok(Some(holder));
        }
    }
    Ok(None)
}

/// Whether `tip` lies on `holder`'s first-parent line: `holder` was made on
/// top of the branch rather than merging it. Undecidable within
/// [`STACK_CAP`] counts as stacked, so the holder is not called a merge.
fn stacked_on(repo: &RepoHandle, tip: Oid, holder: Oid, stop: &dyn Stop) -> Result<bool, RunError> {
    let range = commits_between(repo, tip, holder, STACK_CAP, stop)?;
    if range.capped {
        return Ok(true);
    }
    let by_id: HashMap<Oid, &CommitSummary> = range.commits.iter().map(|c| (c.id, c)).collect();
    let mut at = holder;
    while let Some(commit) = by_id.get(&at) {
        match commit.parents.first() {
            Some(&parent) if parent == tip => return Ok(true),
            Some(&parent) => at = parent,
            None => return Ok(false),
        }
    }
    Ok(false)
}

/// Authors of `commits`, grouped by email: most commits first, ties by the
/// newer last commit, then by email so the order never depends on input
/// order alone. `None` for no commits.
fn who_of(commits: &[CommitSummary]) -> Option<Who> {
    struct Author<'a> {
        email: &'a [u8],
        name: &'a [u8],
        count: usize,
        newest: Timestamp,
    }
    let last = newest(commits)?;
    let mut authors: Vec<Author<'_>> = Vec::new();
    for commit in commits {
        let who = &commit.author;
        match authors.iter_mut().find(|a| a.email == who.email.as_slice()) {
            Some(author) => {
                author.count += 1;
                if who.when.seconds > author.newest.seconds {
                    author.newest = who.when;
                    author.name = &who.name;
                }
            }
            None => authors.push(Author {
                email: &who.email,
                name: &who.name,
                count: 1,
                newest: who.when,
            }),
        }
    }
    authors.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then(b.newest.seconds.cmp(&a.newest.seconds))
            .then(a.email.cmp(b.email))
    });
    let first: Vec<Vec<u8>> = authors.iter().take(2).map(|a| a.name.to_vec()).collect();
    Some(Who::Authors {
        others: authors.len() - first.len(),
        first,
        last,
    })
}

/// A range the counts promised commits in came back empty. The reads are
/// by commit id, not by ref, so this is git contradicting itself.
fn listed_nothing(branch: &Candidate) -> RunError {
    RunError::Unparsable(format!(
        "rev-list listed no commits for {}, where a count found some",
        branch.name
    ))
}

/// When the newest of `commits` was authored.
fn newest(commits: &[CommitSummary]) -> Option<Timestamp> {
    commits
        .iter()
        .map(|c| c.author.when)
        .max_by_key(|when| when.seconds)
}
