//! A rebase's picture: its line of branches replayed, one ghost per commit, and the branches that follow.

#[allow(clippy::wildcard_imports)]
use super::*;

/// `r`: his branch and each branch of `params.line` built on it, replayed
/// onto `onto` as the engine runs it (05b) — one ghost per pick, his rows in
/// `params.drops` left out, one [`RefMove`] per branch bottom first, and the
/// chained forecast over the same todo.
///
/// Also answers his branch's landed commit, from the chain, for the
/// put-back forecast: `None` when the chain did not reach it (a stop, a
/// hole, past the cap) and on the one-branch shortcuts that replay nothing.
pub(in crate::preview) fn rebase(
    repo: &RepoHandle,
    git: &GitVersion,
    params: &RebaseParams,
    stop: &dyn Stop,
) -> Result<(PreviewPlan, Option<Oid>), PreviewError> {
    let onto = params.onto_tip;
    // Not `commits_between`: the ghosts must be the commits git will write,
    // and `git rebase` replays a narrower list than `onto..branch` — merges
    // and changes already on `onto` are dropped (m5/43). See
    // `replayed_commits`. The engine's rows (`rewrite::rebase_rows`) are
    // the same `rev-list`.
    let range = history_reads::replayed_commits(repo, onto, params.branch.oid, FORECAST_CAP, stop)?;
    let mut plan = blank();
    if range.commits.is_empty() && params.line.dependents().is_empty() {
        if params.branch.oid == onto {
            plan.undo = UndoNote::NotUndoable("nothing would be replayed".to_owned());
            return Ok((plan, None));
        }
        // Nothing to replay, but the branch still moves: git checks `onto`
        // out and leaves the branch there — a fast-forward when the branch
        // is behind, and a plain move when everything it had was a merge or
        // was already applied. Proved by running `git rebase` on both
        // shapes (m5/43). Drawing "nothing would be replayed" here would
        // hide a ref move and a rewritten tree.
        plan.ref_moves = vec![RefMove {
            name: params.branch.name.clone(),
            from: Some(params.branch.oid),
            to: Some(Target::Real(onto)),
        }];
        plan.worktree = worktree(
            repo,
            &Landing::At {
                from: params.branch.oid,
                to: onto,
            },
        )?;
        return Ok((plan, None));
    }
    let followers = followers_of(repo, &params.line, stop)?;
    line_ghosts(&mut plan, params, &range.commits, &followers);
    // The replayed branch's tree is a ghost. What lands on disk is what
    // `onto` gained since the two diverged: the branch's own commits are
    // replayed on top and their files are already here.
    plan.worktree = worktree(
        repo,
        &Landing::Joining {
            ours: params.branch.oid,
            theirs: onto,
        },
    )?;
    let steps = todo_steps(params, &range.commits, &followers);
    let landed = if range.capped {
        plan.conflicts = ConflictForecast::NotChecked(format!("more than {FORECAST_CAP} commits"));
        None
    } else {
        let chained = chain::chain(repo, git, onto, &steps, stop)?;
        plan.conflicts = chained.forecast;
        // His branch's landed commit: the result after his last row, or
        // `onto` when he has none. `None` past a stop or a hole.
        match range.commits.len() {
            0 => Some(onto),
            his => chained.results.get(his - 1).copied().flatten(),
        }
    };
    let rewritten: Vec<Oid> = steps.iter().map(|step| step.commit).collect();
    plan.published = rewrite::published(repo, &params.line, &rewritten, onto, stop)?;
    Ok((plan, landed))
}

/// One chain of ghosts on `onto`: his kept rows (`params.drops` left out),
/// his branch's [`RefMove`], then each follower's rows and its own move.
fn line_ghosts(
    plan: &mut PreviewPlan,
    params: &RebaseParams,
    his: &[history_reads::CommitSummary],
    followers: &Followers,
) {
    let mut below = Target::Real(params.onto_tip);
    let kept = his
        .iter()
        .filter(|commit| !params.drops.contains(&commit.id));
    for commit in kept {
        below = push_ghost(plan, commit, below);
    }
    plan.ref_moves.push(RefMove {
        name: params.branch.name.clone(),
        from: Some(params.branch.oid),
        to: Some(below),
    });
    let mut drawn = 0;
    for (name, from, end) in &followers.ends {
        for commit in &followers.commits[drawn..*end] {
            below = push_ghost(plan, commit, below);
        }
        drawn = *end;
        plan.ref_moves.push(RefMove {
            name: name.clone(),
            from: Some(*from),
            to: Some(below),
        });
    }
}

/// The todo git runs, as the chained forecast's steps: his rows (a dropped
/// one counted, never replayed), then each follower's picks.
fn todo_steps(
    params: &RebaseParams,
    his: &[history_reads::CommitSummary],
    followers: &Followers,
) -> Vec<Step> {
    let step = |commit: &history_reads::CommitSummary, replays: bool| Step {
        commit: commit.id,
        parent: commit.parents.first().copied(),
        replays,
    };
    his.iter()
        .map(|commit| step(commit, !params.drops.contains(&commit.id)))
        .chain(followers.commits.iter().map(|commit| step(commit, true)))
        .collect()
}

/// A ghost for `commit` on `below`, next in the chain; the target standing
/// for it.
fn push_ghost(
    plan: &mut PreviewPlan,
    commit: &history_reads::CommitSummary,
    below: Target,
) -> Target {
    let index = plan.ghosts.len();
    plan.ghosts.push(GhostCommit {
        oid: ghost_oid(&commit.subject, index),
        parents: vec![below],
        subject: commit.subject.clone(),
        author: commit.author.clone(),
        replays: Some(commit.id),
    });
    Target::Ghost(index)
}

/// Each follower's picks (the commits between the branch below it and its
/// tip), oldest first, all of them in one list, and where each branch of
/// the line ends in it: `(name, from, picks so far)`.
struct Followers {
    commits: Vec<history_reads::CommitSummary>,
    ends: Vec<(String, Oid, usize)>,
}

fn followers_of(
    repo: &RepoHandle,
    line: &BranchLine,
    stop: &dyn Stop,
) -> Result<Followers, PreviewError> {
    let picks = line_picks(repo, line, stop).map_err(|why| match why {
        FactsError::Stopped => PreviewError::Stopped,
        other => PreviewError::Facts(other),
    })?;
    let ids: Vec<Oid> = picks
        .iter()
        .flat_map(|(_, rows)| rows.iter().map(|row| row.commit))
        .collect();
    let mut info = chain::summaries(repo, &ids, stop)?;
    let mut commits = Vec::with_capacity(ids.len());
    let mut ends = Vec::with_capacity(picks.len());
    for ((name, rows), tip) in picks.iter().zip(line.dependents()) {
        for row in rows {
            let Some(commit) = info.remove(&row.commit) else {
                return Err(PreviewError::Read(RunError::Unparsable(format!(
                    "rev-list gave nothing for {}",
                    short(row.commit)
                ))));
            };
            commits.push(commit);
        }
        ends.push((name.clone(), tip.oid, commits.len()));
    }
    Ok(Followers { commits, ends })
}
