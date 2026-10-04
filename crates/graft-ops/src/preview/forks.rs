//! A rewrite's forks (ADR-0059): the branches off the line that hold a
//! commit it replaces, which of them follow it — replayed inline in the same
//! `rebase -i`, at their fork point — and why each other one stays.
//!
//! A fate is fixed here, in the preview, and run as previewed: the
//! sections of the followers go into the operation `y` runs
//! (`plan_as_run`), and every one of their tips is pinned (`check`'s
//! `Moved`). A forecast hole is never read as clean.

use std::collections::{HashMap, HashSet};

use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::history_reads::CommitSummary;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, RunError, Stop};
use graft_git::version::GitVersion;

use crate::facts::FactsError;
use crate::operation::Tip;
use crate::rewrite::{ForkSection, OffLine, OwnCommits, TodoAction, TodoLine, squash_groups};

use super::chain::{self, Chained, Step};
use super::rewrite::holds_any;
use super::{ConflictForecast, Fork, ForkFate, LeftBehind, PreviewError, Target, WhyStays};

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

/// What every fate is weighed with, read once for all the forks.
struct Weigh {
    /// Each row's commit → its index in `rows`.
    row_of: HashMap<Oid, usize>,
    /// The default branch, full name.
    default: Option<String>,
    /// Fork → its upstream, full names.
    upstreams: HashMap<String, String>,
    /// Every fork commit's summary.
    info: HashMap<Oid, CommitSummary>,
    /// The line's commits before the edit, `onto..` its top on the first
    /// parents, oldest first.
    line: Vec<Oid>,
}

/// One fork as its fate is weighed.
struct Weighed<'a> {
    candidate: &'a Candidate,
    /// The other candidates on its own commits, moved with it if it follows.
    carried: Vec<&'a Candidate>,
    /// The row whose result would be its new base.
    after_row: Option<usize>,
}

impl<'a> Weighed<'a> {
    /// `candidates[index]`, carrying `candidates[carried[..]]`.
    fn of(
        candidates: &'a [Candidate],
        index: usize,
        carried: &[usize],
        after_row: Option<usize>,
    ) -> Self {
        Self {
            candidate: &candidates[index],
            carried: carried.iter().map(|other| &candidates[*other]).collect(),
            after_row,
        }
    }
}

impl Weigh {
    fn read(
        repo: &RepoHandle,
        rewritten: &Rewritten<'_>,
        candidates: &[Candidate],
        stop: &dyn Stop,
    ) -> Result<Self, PreviewError> {
        let row_of = rewritten
            .rows
            .iter()
            .enumerate()
            .map(|(index, row)| (row.commit, index))
            .collect();
        if candidates.is_empty() {
            return Ok(Self {
                row_of,
                default: None,
                upstreams: HashMap::new(),
                info: HashMap::new(),
                line: Vec::new(),
            });
        }
        let default = crate::rewrite::default_branch_of(repo, None).map_err(facts)?;
        let upstreams = upstreams(repo, candidates, stop)?;
        let commits: Vec<Oid> = candidates.iter().flat_map(Candidate::commits).collect();
        let info = chain::summaries(repo, &commits, stop)?;
        let line = match rewritten.line.last() {
            Some(top) => line_before(repo, rewritten.onto, top.oid, stop)?,
            None => Vec::new(),
        };
        Ok(Self {
            row_of,
            default,
            upstreams,
            info,
            line,
        })
    }

    /// The line's commits at or under `at`, before the edit, oldest first;
    /// `None` when `at` is not one of them.
    fn under(&self, at: Oid) -> Option<&[Oid]> {
        let end = self.line.iter().position(|commit| *commit == at)?;
        Some(&self.line[..=end])
    }

    /// The row whose result is the fork's new base: `at`'s own, or the last
    /// row of the squash group it is in (git makes the group's commit
    /// there). `None` when `at` is not a row.
    fn after_row(&self, rewritten: &Rewritten<'_>, at: Oid) -> Option<usize> {
        let row = *self.row_of.get(&at)?;
        let end = squash_groups(rewritten.his)
            .into_iter()
            .find(|group| group.kept == row || group.melded.contains(&row))
            .map_or(row, |group| {
                group.melded.iter().copied().fold(group.kept, usize::max)
            });
        Some(end)
    }

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

    /// Where a branch is that keeps it put: the default branch, or checked
    /// out in another worktree. No read.
    fn placed(&self, branch: &Candidate) -> Option<WhyStays> {
        let name = String::from_utf8_lossy(branch.off.branch.as_bytes());
        if self.default.as_deref() == Some(name.as_ref()) {
            return Some(WhyStays::DefaultBranch);
        }
        branch
            .off
            .elsewhere
            .as_ref()
            .map(|worktree| WhyStays::Elsewhere {
                worktree: worktree.clone(),
            })
    }

    /// `branch`'s upstream, when it holds a commit of `onto..branch`
    /// (ADR-0059 §6.2): `under`, the line's commits at or under its fork
    /// point, or `own`, its own commits. The other half of pushed — any
    /// remote-tracking ref holding one of `own` — is [`remote_holding`].
    fn upstream_holding(
        &self,
        env: &Env<'_>,
        branch: &Candidate,
        under: &[Oid],
        own: &[Oid],
    ) -> Result<Option<RefName>, PreviewError> {
        let name = String::from_utf8_lossy(branch.off.branch.as_bytes());
        let Some(upstream) = self.upstreams.get(name.as_ref()) else {
            return Ok(None);
        };
        let held: Vec<Oid> = under.iter().chain(own).copied().collect();
        if holds_any(env.repo, upstream, env.rewritten.onto, &held, env.stop)? {
            return Ok(Some(ref_name(upstream)?));
        }
        Ok(None)
    }

    /// The first branch the fork carries that fails one of its gates
    /// (ADR-0059 §6.1), as [`WhyStays::Carries`]. `own` is the fork's own
    /// commits, oldest first.
    fn carries(
        &self,
        env: &Env<'_>,
        weighed: &Weighed<'_>,
        under: &[Oid],
        own: &[Oid],
    ) -> Result<Option<WhyStays>, PreviewError> {
        for branch in &weighed.carried {
            // Its own commits are the fork's up to its tip. A
            // remote-tracking ref holding one of them holds the fork's
            // oldest, weighed after this as the fork's own.
            let why = if let Some(why) = self.placed(branch) {
                Some(why)
            } else {
                let end = own.iter().position(|commit| *commit == branch.off.tip);
                let theirs = end.map_or(own, |end| &own[..=end]);
                self.upstream_holding(env, branch, under, theirs)?
                    .map(|upstream| WhyStays::Pushed { upstream })
            };
            if let Some(why) = why {
                return Ok(Some(WhyStays::Carries {
                    branch: branch.off.branch.clone(),
                    why: Box::new(why),
                }));
            }
        }
        Ok(None)
    }

    /// Whether following would change what the fork is built on (ADR-0059
    /// §6.3): the line's commits at or under its fork point before the edit
    /// (`under`) against the commits in the rows up to `after_row` — the
    /// last row of the fork point's squash group, so every commit folded
    /// into it counts. A commit `a` wrote from the index is not the line's
    /// and does not count; nor does a dropped row.
    fn base_change(
        &self,
        rewritten: &Rewritten<'_>,
        under: &[Oid],
        after_row: usize,
    ) -> Option<WhyStays> {
        let after: Vec<Oid> = rewritten
            .rows
            .iter()
            .take(after_row + 1)
            .filter(|row| !matches!(row.action, TodoAction::Drop))
            .map(|row| row.commit)
            .filter(|commit| self.line.contains(commit))
            .collect();
        if let Some(lost) = under.iter().find(|commit| !after.contains(commit)) {
            return Some(WhyStays::WouldLose { commit: *lost });
        }
        after
            .iter()
            .find(|commit| !under.contains(commit))
            .map(|gained| WhyStays::WouldGain { commit: *gained })
    }
}

/// The line's commits before the edit, `onto..top` on the first parents,
/// oldest first: one `rev-list`.
fn line_before(
    repo: &RepoHandle,
    onto: Oid,
    top: Oid,
    stop: &dyn Stop,
) -> Result<Vec<Oid>, PreviewError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("rev-list"),
            GitArg::Flag("--first-parent"),
            GitArg::Flag("--reverse"),
            GitArg::Word(top.to_string()),
            GitArg::Word(format!("^{onto}")),
        ])
        .run(stop)?;
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| {
            Oid::from_hex(line.trim()).map_err(|_| {
                PreviewError::Read(RunError::Unparsable(format!("rev-list printed {line:?}")))
            })
        })
        .collect()
}

/// The first remote-tracking ref (by name, symbolic refs aside) that holds
/// `commit`, in full: one `for-each-ref --contains`.
fn remote_holding(
    repo: &RepoHandle,
    commit: Oid,
    stop: &dyn Stop,
) -> Result<Option<RefName>, PreviewError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("for-each-ref"),
            GitArg::Flag("--format=%(refname)%00%(symref)"),
            GitArg::Word(format!("--contains={commit}")),
            GitArg::Flag("refs/remotes"),
        ])
        .run(stop)?;
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split_once('\0'))
        .find(|(_, symref)| symref.is_empty())
        .map(|(name, _)| ref_name(name))
        .transpose()
}

/// Whether a fork's own commits (`own`: commit, parent) replay clean onto
/// the result of `after_row`, as the chained forecast reaches it. A
/// forecast hole is never read as clean.
fn replayed_fate(
    env: &Env<'_>,
    own: &[(Oid, Oid)],
    after_row: usize,
    at: Oid,
) -> Result<ForkFate, PreviewError> {
    let chained = env.rewritten.chained;
    let base = match &chained.forecast {
        ConflictForecast::StopsAt { step, .. } if *step <= after_row + 1 => {
            return Ok(ForkFate::Stays(WhyStays::StopsBelow));
        }
        ConflictForecast::NotChecked(why) => {
            return Ok(ForkFate::Stays(WhyStays::NotChecked(why.clone())));
        }
        _ => chained.results.get(after_row).copied().flatten(),
    };
    let (Some(base), Some(onto)) = (base, env.rewritten.tip_after.get(after_row).copied()) else {
        return Ok(ForkFate::Stays(WhyStays::NotChecked(
            "the rewrite's forecast did not reach it".to_owned(),
        )));
    };
    let steps: Vec<Step> = own
        .iter()
        .map(|(commit, parent)| Step {
            commit: *commit,
            parent: Some(*parent),
            replays: true,
        })
        .collect();
    let replayed = chain::chain(env.repo, env.git, base, &steps, env.stop)?;
    Ok(match replayed.forecast {
        ConflictForecast::Clean => ForkFate::Follows { onto },
        ConflictForecast::StopsAt { commit, paths, .. } => {
            ForkFate::Stays(WhyStays::WouldConflict { commit, paths })
        }
        ConflictForecast::NotChecked(why) => ForkFate::Stays(WhyStays::NotChecked(why)),
        // The chained replay never answers `Likely`; were it to, a
        // conflict forecast is no clean bill.
        ConflictForecast::Likely { paths } => ForkFate::Stays(WhyStays::WouldConflict {
            commit: own.first().map_or(at, |(commit, _)| *commit),
            paths,
        }),
    })
}

/// Each candidate's upstream, full names: one `for-each-ref`.
fn upstreams(
    repo: &RepoHandle,
    candidates: &[Candidate],
    stop: &dyn Stop,
) -> Result<HashMap<String, String>, PreviewError> {
    let mut args = vec![
        GitArg::Flag("for-each-ref"),
        GitArg::Flag("--format=%(refname)%00%(upstream)"),
    ];
    args.extend(
        candidates
            .iter()
            .map(|candidate| GitArg::Ref(candidate.off.branch.as_bytes().to_vec())),
    );
    let output = GitInvocation::new(repo).args(args).run(stop)?;
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split_once('\0'))
        .filter(|(_, upstream)| !upstream.is_empty())
        .map(|(name, upstream)| (name.to_owned(), upstream.to_owned()))
        .collect())
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
