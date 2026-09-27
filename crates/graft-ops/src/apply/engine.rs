//! Running a rewrite (ADR-0047): every [`Operation::Rewrite`] and every
//! [`Operation::Rebase`] is git's `rebase -i` on a todo Graft wrote, with the
//! branches built on his following on one line — for a rebase (`r`), the
//! line and the dropped commits its preview found and drew
//! (`preview::plan_as_run`, 05b).
//!
//! [`apply`](super::apply) drives it in three beats, each where the recording
//! protocol needs it:
//! 1. [`prepare`] — reads only: the line of branches, the todo, the command.
//!    Before `touches` is taken, since every branch of the line is recorded.
//! 2. [`Engine::save`] — the saved plan, **before** the `Pending` line, so a
//!    kill -9 mid-rewrite leaves a plan a relaunch can go on from.
//! 3. [`Engine::run`] — git runs; a stop keeps the plan, anything else goes
//!    home (the top of the line back to his branch) and removes it.
//!
//! [`resume`](super::resume) takes a stopped one on with the same helper, and
//! goes home and removes the plan in the same way ([`go_home`], [`remove`]).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use graft_core::journal::{EntryId, HeadRef};
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, RunError};
use graft_git::state::{self, InProgress};

use crate::facts::{self, FactsError};
use crate::operation::{
    Invocation, Operation, RebaseParams, Refusal, RepoFacts, Tip, rebase_invocation, short_ref,
    source_arg,
};
use crate::rewrite::saved::{self, CLEANUP, SavedPlan};
use crate::rewrite::{
    BranchLine, GitTodo, RewriteError, TodoAction, TodoLine, git_todo, line_of, line_picks,
    messages_of, rebase_rows, squash_groups,
};

use super::run::PlanRun;
use super::{AfterState, ApplyError, Context};

/// A rewrite ready to run: what git is handed, and where its plan is saved.
pub(super) struct Engine {
    /// `<git dir>/graft/rewrite/<entry-id>/`.
    dir: PathBuf,
    saved: SavedPlan,
    todo: GitTodo,
    invocation: Invocation,
    line: BranchLine,
}

impl Engine {
    /// Every branch of the line, full names, his first: what the entry
    /// records beside `HEAD` (ADR-0047 §2).
    pub(super) fn refs(&self) -> impl Iterator<Item = &str> {
        self.line.branches.iter().map(|tip| tip.name.as_str())
    }

    /// Remove the plan: the entry closed before git ran.
    pub(super) fn remove_plan(&self) -> Option<AfterState> {
        remove(&self.dir)
    }

    /// The command line the Command Log shows.
    pub(super) fn command_line(&self) -> String {
        self.invocation.command_line()
    }

    /// Write the plan: before the `Pending` line, never after it.
    ///
    /// # Errors
    /// [`ApplyError::Facts`] with [`FactsError::SavedPlan`]: nothing is
    /// journaled yet, so the rewrite does not run.
    pub(super) fn save(&self, repo: &RepoHandle) -> Result<(), ApplyError> {
        saved::save(repo, &self.dir, &self.saved, &self.todo)
            .map_err(|error| ApplyError::Facts(FactsError::SavedPlan(error)))
    }

    /// The plan is saved, but the entry never opened: remove it, and carry
    /// `why` — the reason it never opened — whether or not that worked.
    pub(super) fn abandon(&self, why: ApplyError) -> ApplyError {
        match saved::remove(&self.dir) {
            Ok(()) => why,
            // Both failures reach the caller: the one that stopped the
            // rewrite, and the files it left.
            Err(left) => ApplyError::Facts(FactsError::SavedPlan(std::io::Error::other(format!(
                "{why} · and the saved plan at {} could not be removed: {left}",
                self.dir.display()
            )))),
        }
    }

    /// Run git on the saved todo, and say how it ended.
    ///
    /// - git stopped — a conflict (non-zero) or an `edit` row (exit 0) with
    ///   `rebase-merge/` still there: the plan stays for `resume`, and an
    ///   `edit` stop is `edit_stop` so `apply` reads it as the stop it is.
    /// - Anything else: `HEAD` goes home when git ran on the top of the line,
    ///   and the plan is removed. Neither can fail the rewrite: each failure
    ///   is an [`AfterState`] in `cleanup`.
    ///
    /// The command lines run go on `commands`.
    ///
    /// # Errors
    /// [`ApplyError::Facts`] when git's state cannot be read after the run:
    /// whether it stopped is unknown, so the entry is left `Pending` for
    /// `recover_interrupted`, as `apply`'s own stop check does.
    pub(super) fn run(
        &self,
        ctx: &Context<'_>,
        commands: &mut Vec<String>,
    ) -> Result<PlanRun, ApplyError> {
        let mut git = GitInvocation::new(ctx.repo)
            .args(self.invocation.args.iter().cloned())
            .env("GIT_SEQUENCE_EDITOR", saved::sequence_editor(&self.dir))
            .env("GIT_EDITOR", saved::editor(&self.dir));
        for (key, value) in CLEANUP {
            git = git.env(key, value);
        }
        let ran = git.run(ctx.stop);
        if rebasing(ctx.repo)? {
            let edit_stop = ran.is_ok();
            return Ok(PlanRun {
                failure: ran.err().map(super::run::PlanFailure::local),
                cleanup: Vec::new(),
                edit_stop,
            });
        }
        let mut cleanup = Vec::new();
        cleanup.extend(go_home(ctx.repo, &self.saved, commands));
        cleanup.extend(remove(&self.dir));
        Ok(PlanRun {
            failure: ran.err().map(super::run::PlanFailure::local),
            cleanup,
            edit_stop: false,
        })
    }
}

/// `Some` for the two operations the engine runs, with everything read that
/// running them needs; `None` for every other operation.
///
/// A rebase (`r`) runs `params.line`, its rows `params.drops` marked
/// `drop`, once [`line_holds`]; a rewrite runs the line its plan found and
/// refused over.
///
/// # Errors
/// [`ApplyError::Refused`] (`Moved`) when the line read now is not the one
/// drawn, or a drop is not among the rows; the line's own refusals;
/// [`ApplyError::Facts`] when a read fails; [`ApplyError::Stopped`].
pub(super) fn prepare(
    ctx: &Context<'_>,
    op: &Operation,
    facts: &RepoFacts,
) -> Result<Option<Engine>, ApplyError> {
    let repo = ctx.repo;
    let (head, onto, line, rows, messages, invocation, index_base) = match op {
        Operation::Rebase(params) => {
            // `r` runs the line its preview drew (`preview::plan_as_run`),
            // never one found afresh: the picture and the act are one act
            // (hard choice 3). A line that changed since is refused.
            line_holds(ctx, params)?;
            let mut rows = rebase_rows(repo, params.onto_tip, params.branch.oid, ctx.stop)
                .map_err(facts_error)?;
            for dropped in &params.drops {
                let Some(row) = rows.iter_mut().find(|row| row.commit == *dropped) else {
                    // A drop with no row: the branch is not what the preview
                    // measured.
                    return Err(ApplyError::Refused(Refusal::Moved {
                        name: params.branch.name.clone(),
                    }));
                };
                row.action = TodoAction::Drop;
            }
            let line = params.line.clone();
            let top = top_of(&line).map(|top| short_ref(&top.name).to_owned());
            let invocation = rebase_invocation(source_arg(&params.onto), top.as_deref(), ctx.git);
            (
                params.branch.clone(),
                params.onto_tip,
                line,
                rows,
                BTreeMap::new(),
                invocation,
                None,
            )
        }
        Operation::Rewrite(params) => {
            let Some(invocation) = op.plan(facts).into_iter().next() else {
                return Ok(None);
            };
            (
                params.head.clone(),
                params.onto,
                params.line.clone(),
                params.todo.clone(),
                params.messages.clone(),
                invocation,
                params.index_base,
            )
        }
        _ => return Ok(None),
    };
    let followers = line_picks(repo, &line, ctx.stop).map_err(facts_error)?;
    let written = unwritten_group_messages(repo, &rows, &messages, ctx)?;
    let message_of = |row: usize| {
        rows.get(row)
            .and_then(|line| written.get(&line.commit))
            .cloned()
            .unwrap_or_default()
    };
    let todo = git_todo(&rows, &messages, &message_of, &head.name, &followers);
    Ok(Some(Engine {
        dir: saved::dir(repo, &ctx.next_id()),
        saved: SavedPlan::new(&head.name, &line, onto, index_base),
        todo,
        invocation,
        line,
    }))
}

/// The line of branches built on his, read again at `y`, is the one the
/// preview drew: the same names, in the same order. A branch built on his
/// since is refused `Moved` naming the first branch that differs, never
/// left behind; a fork, a worktree or an old git since is refused in `R`'s
/// words. The tips themselves are `check`'s (`Operation::tips`).
///
/// # Errors
/// [`ApplyError::Refused`]; [`ApplyError::Facts`] when a read fails;
/// [`ApplyError::Stopped`].
fn line_holds(ctx: &Context<'_>, params: &RebaseParams) -> Result<(), ApplyError> {
    let now =
        line_of(ctx.repo, &ctx.git, &params.branch, None, ctx.stop).map_err(|why| match why {
            RewriteError::Refused(refusal) => ApplyError::Refused(refusal),
            RewriteError::Facts(error) => facts_error(error),
            RewriteError::Stopped => ApplyError::Stopped,
        })?;
    let drawn = &params.line.branches;
    let differs = (0..drawn.len().max(now.branches.len())).find_map(|at| {
        match (drawn.get(at), now.branches.get(at)) {
            (Some(was), Some(is)) if was.name == is.name => None,
            (_, Some(is)) => Some(is.name.clone()),
            (Some(was), None) => Some(was.name.clone()),
            (None, None) => None,
        }
    });
    match differs {
        Some(name) => Err(ApplyError::Refused(Refusal::Moved { name })),
        None => Ok(()),
    }
}

/// The top of the line, when branches follow his.
fn top_of(line: &BranchLine) -> Option<&Tip> {
    match line.branches.as_slice() {
        [_, .., top] => Some(top),
        _ => None,
    }
}

/// The whole message of every row a squash group's default message is made
/// from, for a group holding a squash that has no message written — the
/// rows `git_todo` asks `message_of` for. Empty when every message was
/// written, as `plan` writes them.
fn unwritten_group_messages(
    repo: &RepoHandle,
    rows: &[TodoLine],
    messages: &BTreeMap<usize, String>,
    ctx: &Context<'_>,
) -> Result<BTreeMap<graft_core::oid::Oid, String>, ApplyError> {
    let wanted: Vec<graft_core::oid::Oid> = squash_groups(rows)
        .into_iter()
        .filter(|group| {
            !messages.contains_key(&group.kept)
                && group
                    .melded
                    .iter()
                    .any(|&row| rows.get(row).map(|line| line.action) == Some(TodoAction::Squash))
        })
        .flat_map(|group| std::iter::once(group.kept).chain(group.melded))
        .filter_map(|row| rows.get(row).map(|line| line.commit))
        .collect();
    messages_of(repo, &wanted, ctx.stop).map_err(facts_error)
}

/// `rebase-merge/` is there: git stopped mid-rewrite.
fn rebasing(repo: &RepoHandle) -> Result<bool, ApplyError> {
    state::in_progress(repo)
        .map(|live| live == Some(InProgress::Rebase))
        .map_err(|error| ApplyError::Facts(error.into()))
}

/// git is done with the rewrite, one way or the other: when it ran on the
/// top of the line, switch `HEAD` back to his branch (ADR-0047 §3). Inside
/// the same entry, so it is recorded with it.
///
/// Nothing to do when git ran on his branch, or when `HEAD` is on it
/// already (git refused before it switched to the top). A `HEAD` that cannot
/// be read is switched anyway: `switch` to the branch it is on is a no-op,
/// and to any other it is the way home.
///
/// Never stopped: the rewrite has run, and a stop that cut its second half
/// would leave `HEAD` on another branch for no reason (as
/// `record::read_after`).
pub(super) fn go_home(
    repo: &RepoHandle,
    plan: &SavedPlan,
    commands: &mut Vec<String>,
) -> Option<AfterState> {
    let top = plan.top();
    if top == plan.home {
        return None;
    }
    if matches!(facts::head_ref(repo), Ok(HeadRef::Branch(name)) if name == plan.home) {
        return None;
    }
    let home = short_ref(&plan.home).to_owned();
    let git = GitInvocation::new(repo).args([
        GitArg::Flag("switch"),
        GitArg::Ref(home.clone().into_bytes()),
    ]);
    commands.push(git.command_line());
    git.run(&())
        .err()
        .map(|error: RunError| AfterState::NotHome {
            home,
            top: short_ref(top).to_owned(),
            error,
        })
}

/// Remove a closed entry's plan. A failure is an [`AfterState`]: the entry
/// closes all the same.
pub(super) fn remove(dir: &Path) -> Option<AfterState> {
    saved::remove(dir).err().map(|error| AfterState::PlanLeft {
        dir: dir.to_path_buf(),
        error,
    })
}

/// The saved plan of an entry, and where it is; `None` when the entry has
/// none (every entry but a rewrite's, and a rewrite's from before -b).
///
/// # Errors
/// [`FactsError::SavedPlan`] when a plan is there but cannot be read.
pub(super) fn plan_of(
    repo: &RepoHandle,
    id: &EntryId,
) -> Result<Option<(PathBuf, SavedPlan)>, FactsError> {
    let dir = saved::dir(repo, id);
    Ok(saved::read(&dir)
        .map_err(FactsError::SavedPlan)?
        .map(|plan| (dir, plan)))
}

/// A read's failure as `apply`'s, a stop kept as a stop.
fn facts_error(error: FactsError) -> ApplyError {
    match error {
        FactsError::Stopped => ApplyError::Stopped,
        other => ApplyError::Facts(other),
    }
}
