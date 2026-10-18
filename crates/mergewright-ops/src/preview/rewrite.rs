//! The chain preview of a rewrite (`R`, `a` into an older commit, absorb;
//! m7/05, note R1/R3): a ghost for every commit written, one ref move per
//! branch of the line, bottom first, the chained forecast, the messages
//! Mergewright writes, the branches whose upstream holds a rewritten commit, and
//! the forks that follow or stay (ADR-0059, `forks`).
//!
//! [`published`] is shared with a rebase (`r`).

use std::collections::{BTreeMap, HashMap};

use mergewright_core::oid::Oid;
use mergewright_core::ref_name::RefName;
use mergewright_git::history_reads::CommitSummary;
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::{GitArg, GitInvocation, RunError, Stop};
use mergewright_git::version::GitVersion;

use crate::facts::FactsError;
use crate::rewrite::{
    BranchLine, ForkSection, RewriteParams, TodoAction, TodoLine, default_message, line_picks,
    messages_of, squash_groups,
};

use super::chain::{self, Step};
use super::forks::{self, Rewritten};
use super::{
    GhostCommit, HeadMove, Landing, LeftBehind, PreviewError, PreviewPlan, RefMove, Target,
    WorktreeEffect, blank, ghost_oid, worktree,
};

/// The rewrite's picture, and the sections its forks that follow run as
/// (ADR-0059): `plan_as_run` puts those in the operation `y` runs. Any
/// sections `params` already carries are not read: the fates are weighed
/// afresh.
pub(super) fn rewrite(
    repo: &RepoHandle,
    git: &GitVersion,
    params: &RewriteParams,
    stop: &dyn Stop,
) -> Result<(PreviewPlan, Vec<ForkSection>), PreviewError> {
    let followers = line_picks(repo, &params.line, stop).map_err(facts)?;
    let rows: Vec<&TodoLine> = params
        .todo
        .iter()
        .chain(followers.iter().flat_map(|(_, picks)| picks))
        .collect();
    let commits: Vec<Oid> = rows.iter().map(|line| line.commit).collect();
    let info = chain::summaries(repo, &commits, stop)?;
    let chained = chain::chain(repo, git, params.onto, &steps_of(&rows, &info), stop)?;
    let messages = written_messages(repo, params, stop)?;

    let mut plan = blank();
    // Row → what the line is after it, for the forks' bases.
    let mut tip_after = draw_his_rows(&mut plan, params, &info, &messages)?;
    let his_tip = tip_after
        .last()
        .copied()
        .unwrap_or(Target::Real(params.onto));
    plan.ref_moves.push(RefMove {
        name: params.head.name.clone(),
        from: Some(params.head.oid),
        to: Some(his_tip),
    });
    let mut below = his_tip;
    for ((name, picks), tip) in followers.iter().zip(params.line.dependents()) {
        for pick in picks {
            below = push_ghost(&mut plan, pick, &pick.subject, below, &info)?;
            tip_after.push(below);
        }
        plan.ref_moves.push(RefMove {
            name: name.clone(),
            from: Some(tip.oid),
            to: Some(below),
        });
    }
    let rewritten = Rewritten {
        his: &params.todo,
        rows: &rows,
        line: &params.line.branches,
        onto: params.onto,
        tip_after: &tip_after,
        chained: &chained,
    };
    let sections = draw_forks(repo, git, &mut plan, &rewritten, stop)?;
    plan.head = Some(HeadMove::ToBranch(params.head.name.clone()));
    plan.worktree = landed_tree(repo, params, &chained.results)?;
    plan.conflicts = chained.forecast;
    plan.published = published(repo, &params.line, &commits, params.onto, stop)?;
    Ok((plan, sections))
}

/// Each row as the chained forecast replays it.
fn steps_of(rows: &[&TodoLine], info: &HashMap<Oid, CommitSummary>) -> Vec<Step> {
    rows.iter()
        .map(|line| Step {
            commit: line.commit,
            parent: info
                .get(&line.commit)
                .and_then(|it| it.parents.first().copied()),
            replays: line.action != TodoAction::Drop,
        })
        .collect()
}

/// The rewrite's forks weighed (ADR-0059) and drawn: each follower's ghosts
/// and ref moves, every fate, and the branches left behind. Returns the
/// followers' sections.
fn draw_forks(
    repo: &RepoHandle,
    git: &GitVersion,
    plan: &mut PreviewPlan,
    rewritten: &Rewritten<'_>,
    stop: &dyn Stop,
) -> Result<Vec<ForkSection>, PreviewError> {
    let found = forks::forks(repo, git, rewritten, stop)?;
    for section in &found.sections {
        draw_fork(plan, section, rewritten.tip_after, &found.info)?;
    }
    plan.left_behind = found.left_behind;
    plan.left_behind_more = found.more;
    plan.forks = found.drawn;
    plan.forks_more = found.more;
    Ok(found.sections)
}

/// A fork that follows: a ghost per pick on the result of its row, and a
/// ref move for it and each branch on it.
fn draw_fork(
    plan: &mut PreviewPlan,
    section: &ForkSection,
    tip_after: &[Target],
    info: &HashMap<Oid, CommitSummary>,
) -> Result<(), PreviewError> {
    let Some(mut below) = tip_after.get(section.after_row).copied() else {
        return Err(PreviewError::Read(RunError::Unparsable(format!(
            "{}'s fork point is past the rewrite's rows",
            section.branch.name
        ))));
    };
    for pick in &section.picks {
        below = push_ghost(plan, pick, &pick.subject, below, info)?;
        for inside in section.inside.iter().filter(|tip| tip.oid == pick.commit) {
            plan.ref_moves.push(RefMove {
                name: inside.name.clone(),
                from: Some(inside.oid),
                to: Some(below),
            });
        }
    }
    plan.ref_moves.push(RefMove {
        name: section.branch.name.clone(),
        from: Some(section.branch.oid),
        to: Some(below),
    });
    Ok(())
}

/// What lands on disk: `HEAD` against his branch's new tip, when the chain
/// reached it (git comes home to his branch). A stop before it leaves the
/// files unknown, and says so.
fn landed_tree(
    repo: &RepoHandle,
    params: &RewriteParams,
    results: &[Option<Oid>],
) -> Result<WorktreeEffect, PreviewError> {
    let landed = match params.todo.len().checked_sub(1) {
        None => Some(params.onto),
        Some(last) => results.get(last).copied().flatten(),
    };
    match landed {
        Some(to) => worktree(
            repo,
            &Landing::At {
                from: params.head.oid,
                to,
            },
        ),
        None => Ok(WorktreeEffect::NotShown {
            why: "the rewrite may stop first".to_owned(),
        }),
    }
}

/// His rows as ghosts, oldest first: a kept row is a ghost, a melding row
/// joins the ghost below it, a dropped row is none. A row whose message
/// Mergewright writes carries it, in `plan.messages`, keyed by its ghost. Returns
/// his branch's tip after each row: the last is its new tip.
fn draw_his_rows(
    plan: &mut PreviewPlan,
    params: &RewriteParams,
    info: &HashMap<Oid, CommitSummary>,
    messages: &BTreeMap<usize, String>,
) -> Result<Vec<Target>, PreviewError> {
    let mut tip = Target::Real(params.onto);
    let mut after = Vec::with_capacity(params.todo.len());
    for (row, line) in params.todo.iter().enumerate() {
        if line.action.keeps() {
            let subject = messages
                .get(&row)
                .and_then(|message| message.lines().next())
                .unwrap_or(&line.subject)
                .to_owned();
            tip = push_ghost(plan, line, &subject, tip, info)?;
            if let Some(message) = messages.get(&row) {
                plan.messages.push((tip, message.clone()));
            }
        }
        after.push(tip);
    }
    Ok(after)
}

/// One ghost for `line`, on `below`; returns it.
fn push_ghost(
    plan: &mut PreviewPlan,
    line: &TodoLine,
    subject: &str,
    below: Target,
    info: &HashMap<Oid, CommitSummary>,
) -> Result<Target, PreviewError> {
    let Some(summary) = info.get(&line.commit) else {
        return Err(PreviewError::Read(RunError::Unparsable(format!(
            "rev-list did not list {}",
            line.commit
        ))));
    };
    let at = plan.ghosts.len();
    plan.ghosts.push(GhostCommit {
        oid: ghost_oid(subject, at),
        parents: vec![below],
        subject: subject.to_owned(),
        author: summary.author.clone(),
        replays: Some(line.commit),
    });
    Ok(Target::Ghost(at))
}

/// Kept row → the whole message Mergewright writes for it: a reword's own, or a
/// squash group's (its own if written, else [`default_message`]). A group
/// of fixups alone keeps git's message, and is not here.
fn written_messages(
    repo: &RepoHandle,
    params: &RewriteParams,
    stop: &dyn Stop,
) -> Result<BTreeMap<usize, String>, PreviewError> {
    let todo = &params.todo;
    let mut written: BTreeMap<usize, String> = params
        .messages
        .iter()
        .filter(|(row, _)| todo.get(**row).is_some_and(|line| line.action.keeps()))
        .map(|(row, message)| (*row, message.clone()))
        .collect();
    let groups: Vec<_> = squash_groups(todo)
        .into_iter()
        .filter(|group| {
            !written.contains_key(&group.kept)
                && group
                    .melded
                    .iter()
                    .any(|&row| todo[row].action == TodoAction::Squash)
        })
        .collect();
    if groups.is_empty() {
        return Ok(written);
    }
    let commits: Vec<Oid> = todo.iter().map(|line| line.commit).collect();
    let full = messages_of(repo, &commits, stop).map_err(facts)?;
    let message_of = |row: usize| {
        todo.get(row)
            .and_then(|line| full.get(&line.commit))
            .cloned()
            .unwrap_or_default()
    };
    for group in groups {
        written.insert(group.kept, default_message(&group, &message_of, todo));
    }
    Ok(written)
}

/// Branches of `line` whose upstream holds any of `rewritten`: pushing each
/// afterwards needs force. One `for-each-ref`, then one `rev-list
/// <upstream> ^<onto>` per branch that has an upstream.
///
/// # Errors
/// [`PreviewError::Stopped`]; [`PreviewError::Read`] when git fails.
pub(super) fn published(
    repo: &RepoHandle,
    line: &BranchLine,
    rewritten: &[Oid],
    onto: Oid,
    stop: &dyn Stop,
) -> Result<Vec<RefName>, PreviewError> {
    let mut args = vec![
        GitArg::Flag("for-each-ref"),
        GitArg::Flag("--format=%(refname)%00%(upstream)"),
    ];
    args.extend(
        line.branches
            .iter()
            .map(|tip| GitArg::Ref(tip.name.as_bytes().to_vec())),
    );
    let output = GitInvocation::new(repo).args(args).run(stop)?;
    let upstreams: HashMap<String, String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split_once('\0'))
        .filter(|(_, upstream)| !upstream.is_empty())
        .map(|(name, upstream)| (name.to_owned(), upstream.to_owned()))
        .collect();
    let mut named = Vec::new();
    for tip in &line.branches {
        let Some(upstream) = upstreams.get(&tip.name) else {
            continue;
        };
        if holds_any(repo, upstream, onto, rewritten, stop)? {
            let name = RefName::new(tip.name.as_bytes()).map_err(|_| {
                PreviewError::Read(RunError::Unparsable(format!(
                    "the branch name {:?}",
                    tip.name
                )))
            })?;
            named.push(name);
        }
    }
    Ok(named)
}

/// The branches off `line` left on the old commits: those holding the
/// oldest of `replaced` (oldest first, as the todo runs), and where each
/// stays (`rewrite::left_behind`). Nothing when nothing is replaced.
///
/// # Errors
/// [`PreviewError::Stopped`]; [`PreviewError::Facts`] when a read fails.
pub(super) fn left_behind(
    repo: &RepoHandle,
    git: &GitVersion,
    line: &BranchLine,
    replaced: &[Oid],
    stop: &dyn Stop,
) -> Result<(Vec<LeftBehind>, usize), PreviewError> {
    let (Some(oldest), Some(top)) = (replaced.first(), line.top()) else {
        return Ok((Vec::new(), 0));
    };
    let names = line
        .branches
        .iter()
        .map(|tip| {
            RefName::new(tip.name.as_bytes()).map_err(|_| {
                PreviewError::Read(RunError::Unparsable(format!(
                    "the branch name {:?}",
                    tip.name
                )))
            })
        })
        .collect::<Result<Vec<RefName>, PreviewError>>()?;
    crate::rewrite::left_behind(repo, git, *oldest, top.oid, &names, stop).map_err(facts)
}

/// Whether `upstream` reaches any of `rewritten` (none of which `onto`
/// reaches).
pub(super) fn holds_any(
    repo: &RepoHandle,
    upstream: &str,
    onto: Oid,
    rewritten: &[Oid],
    stop: &dyn Stop,
) -> Result<bool, PreviewError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("rev-list"),
            // An upstream configured but never fetched names no ref: it
            // holds nothing, rather than failing the preview.
            GitArg::Flag("--ignore-missing"),
            GitArg::Ref(upstream.as_bytes().to_vec()),
            GitArg::Word(format!("^{onto}")),
        ])
        .run(stop)?;
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let oid = Oid::from_hex(line.trim()).map_err(|_| {
            PreviewError::Read(RunError::Unparsable(format!("rev-list printed {line:?}")))
        })?;
        if rewritten.contains(&oid) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn facts(why: FactsError) -> PreviewError {
    match why {
        FactsError::Stopped => PreviewError::Stopped,
        other => PreviewError::Facts(other),
    }
}
