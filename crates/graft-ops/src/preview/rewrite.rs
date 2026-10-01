//! The chain preview of a rewrite (`R`, `a` into an older commit, absorb;
//! m7/05, note R1/R3): a ghost for every commit written, one ref move per
//! branch of the line, bottom first, the chained forecast, the messages
//! Graft writes, and the branches whose upstream holds a rewritten commit.
//!
//! [`published`] is shared with a rebase (`r`).

use std::collections::{BTreeMap, HashMap};

use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::history_reads::CommitSummary;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, RunError, Stop};
use graft_git::version::GitVersion;

use crate::facts::FactsError;
use crate::rewrite::{
    BranchLine, RewriteParams, TodoAction, TodoLine, default_message, line_picks, messages_of,
    squash_groups,
};

use super::chain::{self, Step};
use super::{
    GhostCommit, HeadMove, Landing, LeftBehind, PreviewError, PreviewPlan, RefMove, Target,
    WorktreeEffect, blank, ghost_oid, worktree,
};

pub(super) fn rewrite(
    repo: &RepoHandle,
    git: &GitVersion,
    params: &RewriteParams,
    stop: &dyn Stop,
) -> Result<PreviewPlan, PreviewError> {
    let followers = line_picks(repo, &params.line, stop).map_err(facts)?;
    let rows: Vec<&TodoLine> = params
        .todo
        .iter()
        .chain(followers.iter().flat_map(|(_, picks)| picks))
        .collect();
    let commits: Vec<Oid> = rows.iter().map(|line| line.commit).collect();
    let info = chain::summaries(repo, &commits, stop)?;
    let steps: Vec<Step> = rows
        .iter()
        .map(|line| Step {
            commit: line.commit,
            parent: info
                .get(&line.commit)
                .and_then(|it| it.parents.first().copied()),
            replays: line.action != TodoAction::Drop,
        })
        .collect();
    let chained = chain::chain(repo, git, params.onto, &steps, stop)?;
    let messages = written_messages(repo, params, stop)?;

    let mut plan = blank();
    let his_tip = draw_his_rows(&mut plan, params, &info, &messages)?;
    plan.ref_moves.push(RefMove {
        name: params.head.name.clone(),
        from: Some(params.head.oid),
        to: Some(his_tip),
    });
    let mut below = his_tip;
    for ((name, picks), tip) in followers.iter().zip(params.line.dependents()) {
        for pick in picks {
            below = push_ghost(&mut plan, pick, &pick.subject, below, &info)?;
        }
        plan.ref_moves.push(RefMove {
            name: name.clone(),
            from: Some(tip.oid),
            to: Some(below),
        });
    }
    plan.head = Some(HeadMove::ToBranch(params.head.name.clone()));
    plan.worktree = landed_tree(repo, params, &chained.results)?;
    plan.conflicts = chained.forecast;
    plan.published = published(repo, &params.line, &commits, params.onto, stop)?;
    (plan.left_behind, plan.left_behind_more) =
        left_behind(repo, git, &params.line, &commits, stop)?;
    Ok(plan)
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
/// Graft writes carries it, in `plan.messages`, keyed by its ghost. Returns
/// his branch's new tip.
fn draw_his_rows(
    plan: &mut PreviewPlan,
    params: &RewriteParams,
    info: &HashMap<Oid, CommitSummary>,
    messages: &BTreeMap<usize, String>,
) -> Result<Target, PreviewError> {
    let mut tip = Target::Real(params.onto);
    for (row, line) in params.todo.iter().enumerate() {
        if !line.action.keeps() {
            continue;
        }
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
    Ok(tip)
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

/// Kept row → the whole message Graft writes for it: a reword's own, or a
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
fn holds_any(
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
