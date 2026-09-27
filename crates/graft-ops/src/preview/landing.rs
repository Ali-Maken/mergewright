//! The working tree: where an operation leaves it, and whether git may stop on a conflict.

#[allow(clippy::wildcard_imports)]
use super::*;

/// Where an operation leaves the working tree, named so that [`worktree`]
/// can answer the one question the `working tree:` line asks.
///
/// **The one rule: the line names the files that differ between the tree
/// `HEAD` has now and the tree it will have once this operation lands.**
/// Nothing else. Not "the two tips the operation mentions" — that pair
/// counts `HEAD`'s own commits, which are already on disk and do not move,
/// and it was wrong in every box that drew it (m5/32).
///
/// The landed tree usually cannot be diffed directly: a merge commit, a
/// rebased branch and a picked commit are all ghosts with no tree yet. It
/// does not need to be. `HEAD` keeps what it has and gains the **incoming
/// change**, so the paths that move on disk are exactly the paths that
/// change touches. The three variants differ only in where the incoming
/// change begins, which is a fact about the operation, not a second rule.
pub(super) enum Landing {
    /// `HEAD` ends at a commit that already exists, so the landed tree is
    /// real and is the pair: a fast-forward merge, a checkout, a `--hard`
    /// reset, a branch created and checked out, a branch from a stash.
    At {
        /// Where `HEAD` is now.
        from: Oid,
        /// The commit whose tree ends up on disk.
        to: Oid,
    },
    /// Another line of history joins `HEAD`'s: a merge that commits, a
    /// rebase onto an upstream. What comes in is everything that side
    /// gained since the two diverged, so the pair is the fork point against
    /// that side's tip.
    Joining {
        /// `HEAD`'s side — where the user is.
        ours: Oid,
        /// The side being brought in: the merge source, the rebase's
        /// `onto`.
        theirs: Oid,
    },
    /// Patches are replayed on top of `HEAD`: a cherry-pick, a revert.
    /// Only those patches come in, so each is taken against its own first
    /// parent and the paths are their union. A revert's patch is the
    /// reverse of the same pair, which touches the same files.
    Replaying(Vec<Replay>),
}

/// One commit a [`Landing::Replaying`] replays, and the parent its patch is
/// against. `parent` is `None` when it was not recorded — a hole, never an
/// empty list (see [`WorktreeEffect::NotShown`]).
pub(super) struct Replay {
    /// The commit's own first parent.
    pub(super) parent: Option<Oid>,
    /// The commit being replayed.
    pub(super) commit: Oid,
}

/// [`Landing`]'s one rule, run: the pair (or pairs) it names, diffed.
pub(super) fn worktree(
    repo: &RepoHandle,
    landing: &Landing,
) -> Result<WorktreeEffect, PreviewError> {
    match landing {
        Landing::At { from, to } => updated(repo, *from, *to),
        Landing::Joining { ours, theirs } => match history_reads::merge_base(repo, *ours, *theirs)?
        {
            Some(fork) => updated(repo, fork, *theirs),
            // Unrelated histories: no fork point, so "what that side gained
            // since we diverged" has no answer. A named hole, because
            // "nothing changes" is the one thing this does not mean.
            None => Ok(WorktreeEffect::NotShown {
                why: "the two histories have no commit in common".to_owned(),
            }),
        },
        Landing::Replaying(replays) => {
            let mut paths: Vec<PathBuf> = Vec::new();
            let mut more = false;
            for replay in replays {
                let Some(parent) = replay.parent else {
                    return Ok(WorktreeEffect::NotShown {
                        why: format!("{}'s parent was not recorded", short(replay.commit)),
                    });
                };
                let (changed, cut) =
                    history_reads::changed_paths(repo, parent, replay.commit, PATHS_CAP)?;
                more |= cut;
                for path in changed {
                    if !paths.contains(&path) {
                        paths.push(path);
                    }
                }
            }
            Ok(WorktreeEffect::Updated { paths, more })
        }
    }
}

/// The files that differ between two commits. The only place a pair is
/// turned into an effect; every caller reaches it through [`worktree`], so
/// the rule above has one implementation and no kind picks its own pair.
pub(super) fn updated(
    repo: &RepoHandle,
    from: Oid,
    to: Oid,
) -> Result<WorktreeEffect, PreviewError> {
    let (paths, more) = history_reads::changed_paths(repo, from, to, PATHS_CAP)?;
    Ok(WorktreeEffect::Updated { paths, more })
}

/// Tracked paths with staged or unstaged changes: what a `--hard` reset
/// would throw away. Untracked files are not touched by a reset and never
/// counted (hard choice 4).
pub(super) fn dirty_tracked(facts: &RepoFacts) -> Vec<PathBuf> {
    facts
        .status
        .entries
        .iter()
        .filter(|entry| entry.is_staged() || entry.is_unstaged())
        .map(|entry| entry.path.clone())
        .collect()
}

/// One `merge-tree`, turned into a forecast.
pub(super) fn forecast_one(
    repo: &RepoHandle,
    git: &GitVersion,
    base: Option<Oid>,
    ours: Oid,
    theirs: Oid,
    stop: &dyn Stop,
) -> Result<ConflictForecast, PreviewError> {
    Ok(
        match history_reads::forecast_merge(repo, git, base, ours, theirs, stop)? {
            Forecast::Clean => ConflictForecast::Clean,
            Forecast::Conflicts { paths } => ConflictForecast::Likely { paths },
            Forecast::NotCheckable(why) => ConflictForecast::NotChecked(why),
        },
    )
}
/// `git merge-base --is-ancestor`: exit 0 is yes, exit 1 is no, anything
/// else is a read that failed and is reported as one.
///
/// Shared with [`crate::apply`], which asks it again at confirm time: a rule
/// only the picture enforces is not enforced (m5/07 review, M5).
pub(crate) fn is_ancestor(
    repo: &RepoHandle,
    older: Oid,
    newer: Oid,
    stop: &dyn Stop,
) -> Result<bool, RunError> {
    let ran = GitInvocation::new(repo)
        .args([
            GitArg::Word("merge-base".to_owned()),
            GitArg::Flag("--is-ancestor"),
            GitArg::Oid(older),
            GitArg::Oid(newer),
        ])
        .run(stop);
    match ran {
        Ok(_) => Ok(true),
        Err(RunError::Exited { status: 1, .. }) => Ok(false),
        Err(other) => Err(other),
    }
}
