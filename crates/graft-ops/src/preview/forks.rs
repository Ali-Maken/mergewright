//! A rewrite's forks (ADR-0059): the branches off the line that hold a
//! commit it replaces, which of them follow it — replayed inline in the same
//! `rebase -i`, at their fork point — and why each other one stays.
//!
//! A fate is fixed here, in the preview, and run as previewed: the
//! sections of the followers go into the operation `y` runs
//! (`plan_as_run`), and every one of their tips is pinned (`check`'s
//! `Moved`). A forecast hole is never read as clean.

mod gates;

use std::collections::{HashMap, HashSet};

use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::history_reads::CommitSummary;
use graft_git::repo::RepoHandle;
use graft_git::run::{RunError, Stop};
use graft_git::version::GitVersion;

use crate::facts::FactsError;
use crate::operation::Tip;
use crate::rewrite::{ForkSection, OffLine, OwnCommits, TodoAction, TodoLine};

use super::chain::Chained;
use super::{Fork, ForkFate, LeftBehind, PreviewError, Target, WhyStays};
use gates::{Weigh, Weighed, remote_holding, replayed_fate};

/// The rewrite as the forks are measured against it.
pub(super) struct Rewritten<'a> {
    /// His rows ([`crate::rewrite::RewriteParams::todo`]): their squash
    /// groups say where a fork point's result is made.
    pub(super) his: &'a [TodoLine],
    /// Every row git runs, his then the dependents', in order.
    pub(super) rows: &'a [&'a TodoLine],
    /// Every branch of the line, with its tip before.
    pub(super) line: &'a [Tip],
    /// Where the first kept row is replayed: the line's commits are
    /// `onto..` its top.
    pub(super) onto: Oid,
    /// Row → what the line is after it: a ghost, or `onto`.
    pub(super) tip_after: &'a [Target],
    /// The line's chained forecast over `rows`.
    pub(super) chained: &'a Chained,
}

/// What [`forks`] found.
pub(super) struct Forks {
    /// The named forks, each with its fate, in the drawing order.
    pub(super) drawn: Vec<Fork>,
    /// Forks past the named ones.
    pub(super) more: usize,
    /// What the followers run as, in the todo's section order.
    pub(super) sections: Vec<ForkSection>,
    /// The branches the rewrite leaves on old commits: every one named off
    /// the line, but the followers and the branches on them.
    pub(super) left_behind: Vec<LeftBehind>,
    /// Each follower commit's summary, for its ghost.
    pub(super) info: HashMap<Oid, CommitSummary>,
}

/// One branch off the line, as it is weighed.
struct Candidate {
    off: OffLine,
    own: OwnCommits,
}

impl Candidate {
    /// Its own commits, oldest first; empty when a merge is among them.
    fn commits(&self) -> Vec<Oid> {
        match &self.own {
            OwnCommits::Line(own) => own.iter().map(|(commit, _)| *commit).collect(),
            OwnCommits::Merge(_) => Vec::new(),
        }
    }

    fn count(&self) -> usize {
        match &self.own {
            OwnCommits::Line(own) => own.len(),
            OwnCommits::Merge(_) => 0,
        }
    }
}

/// The forks of the rewrite and their fates (ADR-0059 §1, §2).
///
/// A branch off the line holding a replaced commit is a fork unless it is
/// a backup: no commits of its own, parked at a tip of the line — that one
/// stays, in [`Forks::left_behind`] only. A fork follows when its fork
/// point's row is reached clean and its own commits replay clean onto that
/// row's result, and it is not the default branch, not checked out
/// elsewhere, not pushed, carries no branch that is one of those (§6.1),
/// is built on the same commits after the edit as before (§6.3), and
/// shares no commit with a fork that follows.
/// Weighed longest first, so a fork holding another moves it as one of the
/// branches on it.
///
/// # Errors
/// [`PreviewError::Stopped`]; [`PreviewError::Facts`] or
/// [`PreviewError::Read`] when a read fails.
pub(super) fn forks(
    repo: &RepoHandle,
    git: &GitVersion,
    rewritten: &Rewritten<'_>,
    stop: &dyn Stop,
) -> Result<Forks, PreviewError> {
    let mut found = Forks {
        drawn: Vec::new(),
        more: 0,
        sections: Vec::new(),
        left_behind: Vec::new(),
        info: HashMap::new(),
    };
    let (Some(oldest), Some(top)) = (rewritten.rows.first(), rewritten.line.last()) else {
        return Ok(found);
    };
    let names = rewritten
        .line
        .iter()
        .map(|tip| ref_name(&tip.name))
        .collect::<Result<Vec<RefName>, PreviewError>>()?;
    let (off, more) =
        crate::rewrite::off_line(repo, oldest.commit, top.oid, &names, stop).map_err(facts)?;
    found.more = more;

    let candidates = candidates_of(repo, &off, rewritten.line, stop)?;
    let weigh = Weigh::read(repo, rewritten, &candidates, stop)?;
    let env = Env {
        repo,
        git,
        rewritten,
        stop,
    };
    let mut fates = Fates::default();
    for index in 0..candidates.len() {
        if fates.claimed.contains(&index) {
            continue;
        }
        let candidate = &candidates[index];
        // The branches it would carry are known before its fate: each is
        // gated as the fork itself is (ADR-0059 §6.1).
        let carried = fates.carried(&candidates, index);
        let after_row = weigh.after_row(rewritten, candidate.off.at);
        let weighed = Weighed::of(&candidates, index, &carried, after_row);
        let fate = weigh.fate(&env, &weighed, &fates.moving)?;
        if let (ForkFate::Follows { .. }, Some(after_row)) = (&fate, after_row) {
            let section = fates.follow(&candidates, index, &carried, after_row, &weigh.info)?;
            found.sections.push(section);
        }
        found.drawn.push(Fork {
            branch: candidate.off.branch.clone(),
            at: candidate.off.at,
            commits: candidate.count(),
            fate,
        });
    }
    // Follows first, then stays, each oldest fork point first.
    found.drawn.sort_by_key(|fork| {
        (
            !matches!(fork.fate, ForkFate::Follows { .. }),
            weigh.row_of.get(&fork.at).copied().unwrap_or(usize::MAX),
            fork.branch.as_bytes().to_vec(),
        )
    });
    found.left_behind = fates.left_behind(off);
    found.info = weigh.info;
    Ok(found)
}

/// The branches off the line weighed as forks, longest first (then by
/// name): every one but a backup — no commits of its own, parked at a tip
/// of the line.
fn candidates_of(
    repo: &RepoHandle,
    off: &[OffLine],
    line: &[Tip],
    stop: &dyn Stop,
) -> Result<Vec<Candidate>, PreviewError> {
    let line_tips: HashSet<Oid> = line.iter().map(|tip| tip.oid).collect();
    let mut candidates = Vec::new();
    for branch in off {
        let backup = branch.tip == branch.at && line_tips.contains(&branch.tip);
        if backup {
            continue;
        }
        let own = crate::rewrite::own_commits(repo, branch.at, branch.tip, stop).map_err(facts)?;
        candidates.push(Candidate {
            off: branch.clone(),
            own,
        });
    }
    candidates.sort_by(|a, b| {
        b.count()
            .cmp(&a.count())
            .then_with(|| a.off.branch.as_bytes().cmp(b.off.branch.as_bytes()))
    });
    Ok(candidates)
}

/// The reads a fate is weighed with.
struct Env<'a> {
    repo: &'a RepoHandle,
    git: &'a GitVersion,
    rewritten: &'a Rewritten<'a>,
    stop: &'a dyn Stop,
}

/// The fates fixed so far, as the candidates are weighed longest first.
#[derive(Default)]
struct Fates {
    /// Each follower commit -> the fork that moves it.
    moving: HashMap<Oid, RefName>,
    /// Candidates moved as a branch on a follower: not weighed again.
    claimed: HashSet<usize>,
    /// Every branch that moves, full name: the followers and those on them.
    follows: HashSet<Vec<u8>>,
}

impl Fates {
    /// Every branch named off the line but those that move.
    fn left_behind(&self, off: Vec<OffLine>) -> Vec<LeftBehind> {
        off.into_iter()
            .filter(|branch| !self.follows.contains(branch.branch.as_bytes()))
            .map(|branch| LeftBehind {
                branch: branch.branch,
                on: branch.at,
            })
            .collect()
    }

    /// The candidates `candidates[index]` carries: every other one not yet
    /// claimed whose tip is one of its own commits.
    fn carried(&self, candidates: &[Candidate], index: usize) -> Vec<usize> {
        let own = candidates[index].commits();
        candidates
            .iter()
            .enumerate()
            .filter(|(other, _)| *other != index && !self.claimed.contains(other))
            .filter(|(_, other)| own.contains(&other.off.tip))
            .map(|(other, _)| other)
            .collect()
    }

    /// `candidates[index]` follows from `after_row`: its commits claimed,
    /// the branches it carries (`carried`) moved with it. Returns its
    /// section.
    fn follow(
        &mut self,
        candidates: &[Candidate],
        index: usize,
        carried: &[usize],
        after_row: usize,
        info: &HashMap<Oid, CommitSummary>,
    ) -> Result<ForkSection, PreviewError> {
        let candidate = &candidates[index];
        let own = candidate.commits();
        let inside: Vec<(usize, Tip)> = carried
            .iter()
            .map(|other| (*other, tip_of(&candidates[*other].off)))
            .collect();
        for commit in &own {
            self.moving.insert(*commit, candidate.off.branch.clone());
        }
        self.follows
            .insert(candidate.off.branch.as_bytes().to_vec());
        let mut inside_tips = Vec::new();
        for (other, tip) in inside {
            self.claimed.insert(other);
            self.follows.insert(tip.name.as_bytes().to_vec());
            inside_tips.push(tip);
        }
        // Bottom up, as the picks run.
        inside_tips.sort_by_key(|tip| own.iter().position(|commit| *commit == tip.oid));
        let picks = own
            .iter()
            .map(|commit| {
                let summary = info.get(commit).ok_or_else(|| {
                    PreviewError::Read(RunError::Unparsable(format!(
                        "rev-list did not list {commit}"
                    )))
                })?;
                Ok(TodoLine {
                    action: TodoAction::Pick,
                    commit: *commit,
                    subject: summary.subject.clone(),
                })
            })
            .collect::<Result<Vec<TodoLine>, PreviewError>>()?;
        Ok(ForkSection {
            after_row,
            branch: tip_of(&candidate.off),
            picks,
            inside: inside_tips,
        })
    }
}

impl Weigh {
    /// A fork's fate. Every gate is a read with no forecast, and all run
    /// first: a fork that fails one is never forecast (ADR-0059 §6.4).
    fn fate(
        &self,
        env: &Env<'_>,
        weighed: &Weighed<'_>,
        moving: &HashMap<Oid, RefName>,
    ) -> Result<ForkFate, PreviewError> {
        let candidate = weighed.candidate;
        let off = &candidate.off;
        if let Some(why) = self.placed(candidate) {
            return Ok(ForkFate::Stays(why));
        }
        let own = match &candidate.own {
            OwnCommits::Merge(merge) => {
                return Ok(ForkFate::Stays(WhyStays::HasMerge { merge: *merge }));
            }
            OwnCommits::Line(own) => own,
        };
        let (Some(after_row), Some(under)) = (weighed.after_row, self.under(off.at)) else {
            return Ok(ForkFate::Stays(WhyStays::NotChecked(format!(
                "{} is not a commit the rewrite replays",
                crate::operation::short(off.at)
            ))));
        };
        let commits: Vec<Oid> = own.iter().map(|(commit, _)| *commit).collect();
        // Pushed (§6.2) is weighed in two halves around the branches it
        // carries: a remote-tracking ref of a carried branch holds the
        // fork's own commits too, and the words name that branch.
        if let Some(upstream) = self.upstream_holding(env, candidate, under, &commits)? {
            return Ok(ForkFate::Stays(WhyStays::Pushed { upstream }));
        }
        if let Some(why) = self.carries(env, weighed, under, &commits)? {
            return Ok(ForkFate::Stays(why));
        }
        // Own commits are one first-parent chain: a ref that holds any of
        // them holds the oldest.
        if let Some(oldest) = commits.first()
            && let Some(upstream) = remote_holding(env.repo, *oldest, env.stop)?
        {
            return Ok(ForkFate::Stays(WhyStays::Pushed { upstream }));
        }
        if let Some(why) = self.base_change(env.rewritten, under, after_row) {
            return Ok(ForkFate::Stays(why));
        }
        if let Some(on) = commits.iter().find_map(|commit| moving.get(commit)) {
            return Ok(ForkFate::Stays(WhyStays::Nested { on: on.clone() }));
        }
        replayed_fate(env, own, after_row, off.at)
    }
}

fn tip_of(off: &OffLine) -> Tip {
    Tip {
        name: String::from_utf8_lossy(off.branch.as_bytes()).into_owned(),
        oid: off.tip,
    }
}

fn ref_name(name: &str) -> Result<RefName, PreviewError> {
    RefName::new(name.as_bytes())
        .map_err(|_| PreviewError::Read(RunError::Unparsable(format!("the branch name {name:?}"))))
}

fn facts(why: FactsError) -> PreviewError {
    match why {
        FactsError::Stopped => PreviewError::Stopped,
        other => PreviewError::Facts(other),
    }
}
