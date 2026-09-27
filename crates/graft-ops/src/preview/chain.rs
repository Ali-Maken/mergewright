//! The chained replay forecast (m7/05, `KNOWN_GAPS` :548): a rebase or a
//! rewrite replayed the way git will, each step merged onto the result of
//! the one before.
//!
//! Step `k` is `git merge-tree --write-tree --merge-base=<commit^> <ours>
//! <commit>`, where `ours` is the result so far. A clean step's tree is made
//! a commit (`commit-tree`, no ref) to be the next `ours` — a commit, not a
//! bare tree, because that is the shape git 2.40's `merge-tree` was
//! confirmed with (`forecast_merge_accepts_a_commit_tree_result_as_ours`).
//! The first step that conflicts is the answer; nothing past it is checked.
//!
//! Objects are written (trees and commits with no ref, `KNOWN_GAPS` :505);
//! no ref, index or file is. A step whose parent already is the result so
//! far is git's fast-forward (`skip_unnecessary_picks`): the commit itself
//! is the result, and no spawn is spent on it.

use std::collections::HashMap;

use graft_core::oid::Oid;
use graft_git::history_reads::{self, CommitSummary, MergeStep};
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, RunError, Stop};
use graft_git::version::GitVersion;

use crate::operation::short;
use crate::snapshot::{self, SnapshotError};

use super::{ConflictForecast, FORECAST_CAP, PreviewError};

/// `merge-tree --merge-base`: git 2.40. Below it nothing is checked.
const MERGE_BASE_MIN: GitVersion = GitVersion {
    major: 2,
    minor: 40,
    patch: 0,
};

/// One row git counts: a commit it replays, or a `drop` it passes over.
#[derive(Clone, Copy, Debug)]
pub(super) struct Step {
    /// The commit on the row.
    pub(super) commit: Oid,
    /// Its first parent: the merge base of its replay. `None` for a root.
    pub(super) parent: Option<Oid>,
    /// `false` for a `drop`: counted, never replayed.
    pub(super) replays: bool,
}

/// The forecast, and the commit standing for the result after each step
/// that was reached (`None` past a stop, or when nothing was checked).
#[derive(Debug)]
pub(super) struct Chained {
    pub(super) forecast: ConflictForecast,
    pub(super) results: Vec<Option<Oid>>,
}

impl Chained {
    fn hole(steps: usize, why: impl Into<String>) -> Self {
        Self {
            forecast: ConflictForecast::NotChecked(why.into()),
            results: vec![None; steps],
        }
    }
}

/// Replay `steps` onto `onto`, in order. Git older than 2.40, more than
/// [`FORECAST_CAP`] steps, or a read that fails is
/// [`ConflictForecast::NotChecked`] — never a clean bill.
///
/// # Errors
/// [`PreviewError::Stopped`] only: `stop` is polled between spawns.
pub(super) fn chain(
    repo: &RepoHandle,
    git: &GitVersion,
    onto: Oid,
    steps: &[Step],
    stop: &dyn Stop,
) -> Result<Chained, PreviewError> {
    if *git < MERGE_BASE_MIN {
        return Ok(Chained::hole(
            steps.len(),
            "git 2.40+ needed for a rebase forecast",
        ));
    }
    if steps.len() > FORECAST_CAP {
        return Ok(Chained::hole(
            steps.len(),
            format!("more than {FORECAST_CAP} commits"),
        ));
    }
    let mut results = vec![None; steps.len()];
    let mut ours = onto;
    for (index, step) in steps.iter().enumerate() {
        polled(stop)?;
        if step.replays {
            let Some(base) = step.parent else {
                return Ok(Chained::hole(
                    steps.len(),
                    format!("{} has no parent to replay from", short(step.commit)),
                ));
            };
            match replay(repo, git, base, ours, step.commit, stop)? {
                Replayed::Onto(result) => ours = result,
                Replayed::Stops(paths) => {
                    return Ok(Chained {
                        forecast: ConflictForecast::StopsAt {
                            step: index + 1,
                            of: steps.len(),
                            commit: step.commit,
                            paths,
                        },
                        results,
                    });
                }
                Replayed::Hole(why) => return Ok(Chained::hole(steps.len(), why)),
            }
        }
        results[index] = Some(ours);
    }
    Ok(Chained {
        forecast: ConflictForecast::Clean,
        results,
    })
}

/// What one step came to.
enum Replayed {
    /// Clean: the commit standing for the result.
    Onto(Oid),
    /// These paths conflict.
    Stops(Vec<std::path::PathBuf>),
    /// Not checkable, and why.
    Hole(String),
}

/// One step: `commit` (whose parent is `base`) replayed onto `ours`.
fn replay(
    repo: &RepoHandle,
    git: &GitVersion,
    base: Oid,
    ours: Oid,
    commit: Oid,
    stop: &dyn Stop,
) -> Result<Replayed, PreviewError> {
    // git's own fast-forward: nothing below the commit changed.
    if base == ours {
        return Ok(Replayed::Onto(commit));
    }
    let tree = match history_reads::merge_tree_step(repo, git, base, ours, commit, stop) {
        Ok(MergeStep::Clean { tree }) => tree,
        Ok(MergeStep::Conflicts { paths }) => return Ok(Replayed::Stops(paths)),
        Ok(MergeStep::NotCheckable(why)) => return Ok(Replayed::Hole(why)),
        Err(RunError::Stopped) => return Err(PreviewError::Stopped),
        Err(why) => return Ok(Replayed::Hole(why.to_string())),
    };
    polled(stop)?;
    match snapshot::scratch_commit(repo, tree, &[ours], stop) {
        Ok(result) => Ok(Replayed::Onto(result)),
        Err(SnapshotError::Stopped) => Err(PreviewError::Stopped),
        Err(why) => Ok(Replayed::Hole(why.to_string())),
    }
}

/// Each commit's parents, author and subject, in one `rev-list --no-walk`
/// in [`history_reads::parse_rev_list`]'s shape.
///
/// # Errors
/// [`PreviewError::Stopped`]; [`PreviewError::Read`] when git fails or its
/// answer does not parse.
pub(super) fn summaries(
    repo: &RepoHandle,
    commits: &[Oid],
    stop: &dyn Stop,
) -> Result<HashMap<Oid, CommitSummary>, PreviewError> {
    if commits.is_empty() {
        return Ok(HashMap::new());
    }
    let mut args = vec![
        GitArg::Flag("rev-list"),
        GitArg::Flag("--no-walk=unsorted"),
        GitArg::Flag("--date=raw"),
        GitArg::Flag("--format=%H%x00%P%x00%an%x00%ae%x00%ad%x00%s"),
    ];
    args.extend(commits.iter().map(|&commit| GitArg::Oid(commit)));
    let output = GitInvocation::new(repo).args(args).run(stop)?;
    let parsed = history_reads::parse_rev_list(&output.stdout).map_err(|why| {
        PreviewError::Read(RunError::Unparsable(format!(
            "rev-list's output did not parse: {why}"
        )))
    })?;
    Ok(parsed
        .into_iter()
        .map(|commit| (commit.id, commit))
        .collect())
}

fn polled(stop: &dyn Stop) -> Result<(), PreviewError> {
    if stop.should_stop() {
        return Err(PreviewError::Stopped);
    }
    Ok(())
}
