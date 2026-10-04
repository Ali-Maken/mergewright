//! The gates a fork's fate is weighed through (ADR-0059 §6), and the reads
//! they share: where a branch is, whether it is pushed, what it carries,
//! whether its base changes, and last the forecast of its replay.
//! [`Weigh::fate`] (in `forks.rs`) runs them in order.

use std::collections::HashMap;

use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::history_reads::CommitSummary;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, RunError, Stop};

use crate::rewrite::{TodoAction, squash_groups};

use super::super::chain::{self, Step};
use super::super::rewrite::holds_any;
use super::super::{ConflictForecast, ForkFate, PreviewError, WhyStays};
use super::{Candidate, Env, Rewritten, facts, ref_name};

/// What every fate is weighed with, read once for all the forks.
pub(super) struct Weigh {
    /// Each row's commit → its index in `rows`.
    pub(super) row_of: HashMap<Oid, usize>,
    /// The default branch, full name.
    default: Option<String>,
    /// Fork → its upstream, full names.
    upstreams: HashMap<String, String>,
    /// Every fork commit's summary.
    pub(super) info: HashMap<Oid, CommitSummary>,
    /// The line's commits before the edit, `onto..` its top on the first
    /// parents, oldest first.
    line: Vec<Oid>,
}

/// One fork as its fate is weighed.
pub(super) struct Weighed<'a> {
    pub(super) candidate: &'a Candidate,
    /// The other candidates on its own commits, moved with it if it follows.
    carried: Vec<&'a Candidate>,
    /// The row whose result would be its new base.
    pub(super) after_row: Option<usize>,
}

impl<'a> Weighed<'a> {
    /// `candidates[index]`, carrying `candidates[carried[..]]`.
    pub(super) fn of(
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
    pub(super) fn read(
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
    pub(super) fn under(&self, at: Oid) -> Option<&[Oid]> {
        let end = self.line.iter().position(|commit| *commit == at)?;
        Some(&self.line[..=end])
    }

    /// The row whose result is the fork's new base: `at`'s own, or the last
    /// row of the squash group it is in (git makes the group's commit
    /// there). `None` when `at` is not a row.
    pub(super) fn after_row(&self, rewritten: &Rewritten<'_>, at: Oid) -> Option<usize> {
        let row = *self.row_of.get(&at)?;
        let end = squash_groups(rewritten.his)
            .into_iter()
            .find(|group| group.kept == row || group.melded.contains(&row))
            .map_or(row, |group| {
                group.melded.iter().copied().fold(group.kept, usize::max)
            });
        Some(end)
    }

    /// Where a branch is that keeps it put: the default branch, or checked
    /// out in another worktree. No read.
    pub(super) fn placed(&self, branch: &Candidate) -> Option<WhyStays> {
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
    pub(super) fn upstream_holding(
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
    pub(super) fn carries(
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
    pub(super) fn base_change(
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
pub(super) fn remote_holding(
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

/// Whether a fork's own commits (`own`: commit, parent) replay clean onto
/// the result of `after_row`, as the chained forecast reaches it. A
/// forecast hole is never read as clean.
pub(super) fn replayed_fate(
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
