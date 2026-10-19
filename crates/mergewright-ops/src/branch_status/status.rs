//! The right pane's four answers about one branch, from its footing (R2)
//! and its route: pushed, merged, behind, who (ease-pass 01r, R3).

use mergewright_core::commit::Timestamp;
use mergewright_core::oid::Oid;
use mergewright_git::history_reads::{CommitSummary, commits_between, commits_from, count_between};
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::{RunError, Stop};

use super::footing::{Footing, footing};
use super::{Candidate, CandidateKind, measure, pick_base, walk};
use crate::operation::Tip;

/// The most branch-only commits `Who` reads; past it, the newest are kept.
const WHO_CAP: usize = 2_000;

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
/// "Merged" is proven by a merge on a line; anything unproven is `NotYet`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Merged {
    /// The trunk, and only the trunk: nothing to merge it into.
    MainLine,
    /// Taken in by a merge into this branch: a long-lived line holding it
    /// off its first-parent line, or an ordinary base that sat under it.
    Into(String),
    /// Not merged anywhere git can prove.
    NotYet {
        /// Its nearest base (`route[0]`); `None` with no base (an orphan).
        into: Option<String>,
    },
    /// No commits of its own: it sits on a long-lived branch's line (just
    /// made, left at an older commit, or fast-forwarded in).
    NothingToMerge,
}

/// How far the branch's base has moved on since the branch left it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Behind {
    /// No base to be behind: the trunk with no upstream, an orphan, or a
    /// merged branch nothing eligible lies under.
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

/// One author of a branch's own commits, as the newest of their commits
/// names them; lossy UTF-8 of what the repository stores.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Author {
    /// The author name.
    pub name: String,
    /// The author email, raw (not mapped).
    pub email: String,
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
        /// Up to two authors, most commits first (ties: most recent
        /// first), grouped by raw email (no mailmap here: the app joins
        /// addresses when it asks who is the user).
        first: Vec<Author>,
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
    let footing = footing(repo, branch, &measured, stop)?;
    let first = match footing {
        Footing::OnLine(line) => Some(line.clone()),
        Footing::Merged { .. } | Footing::Own => {
            pick_base(repo, branch, &measured, branch.kind, stop)?
        }
    };
    // Each base carries how far it is from the walker that chose it, so
    // `route[0]`'s is the branch's own: no second lookup.
    let route = walk(repo, branch, candidates, first, stop)?;
    let base = route.first();
    let behind = base.map_or(Behind::NoBase, |(base, apart)| {
        behind_by(&base.name, apart.base_only)
    });

    let (merged, own) = match footing {
        Footing::OnLine(_) => (Merged::NothingToMerge, OwnCommits::None),
        Footing::Merged { into, own_from } => (
            Merged::Into(into.name.clone()),
            own_from.map_or(OwnCommits::None, OwnCommits::Since),
        ),
        Footing::Own => (
            Merged::NotYet {
                into: base.map(|(base, _)| base.name.clone()),
            },
            // No base: an orphan, so all of it is its own.
            base.map_or(OwnCommits::All, |(base, _)| OwnCommits::Since(base.tip)),
        ),
    };
    let own = match own {
        OwnCommits::None => None,
        OwnCommits::Since(under) => Some(commits_between(repo, under, branch.tip, WHO_CAP, stop)?),
        OwnCommits::All => Some(commits_from(repo, branch.tip, WHO_CAP, stop)?),
    };
    let who = match own {
        None => Who::NoOwnCommits,
        Some(own) => who_of(&own.commits).ok_or_else(|| listed_nothing(branch))?,
    };

    Ok(BranchStatus {
        route: route.into_iter().map(|(base, _)| base).collect(),
        pushed,
        merged,
        behind,
        who,
    })
}

/// Which commits are the branch's own, for [`Who`].
enum OwnCommits {
    /// None: it sits on a long-lived line.
    None,
    /// Those this commit lacks.
    Since(Oid),
    /// Every commit it reaches: it shares no history with any base.
    All,
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

/// Authors of `commits`, grouped by email: most commits first, ties by the
/// newer last commit, then by email so the order never depends on input
/// order alone. `None` for no commits.
fn who_of(commits: &[CommitSummary]) -> Option<Who> {
    struct Tally<'a> {
        email: &'a [u8],
        name: &'a [u8],
        count: usize,
        newest: Timestamp,
    }
    let last = newest(commits)?;
    let mut authors: Vec<Tally<'_>> = Vec::new();
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
            None => authors.push(Tally {
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
    let first: Vec<Author> = authors
        .iter()
        .take(2)
        .map(|a| Author {
            name: String::from_utf8_lossy(a.name).into_owned(),
            email: String::from_utf8_lossy(a.email).into_owned(),
        })
        .collect();
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
