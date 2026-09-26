//! The recording protocol (DESIGN.md §6.2): read, check, back up, journal,
//! run, journal again.
//!
//! The order is the whole point. The `Pending` line and the backup commit are
//! on disk **before** the first `git` write, so a crash between them leaves an
//! entry [`Journal::unresolved`] finds and a snapshot `u` can restore from.
//! The outcome line is written on success **and** on failure, and carries the
//! after-state either way: an entry with no recorded after-state is undoable
//! only by restoring everything, so a missing key is never a saving.
//!
//! [`apply`] and the [`Context`] it runs in live here; the rest are
//! submodules, their public items re-exported so every `apply::` path reads
//! as before: `record` (the before and after halves of the protocol), `run`
//! (running a plan), `error`, `restart` (`d` after `⏎`), `fold` (a step
//! inside a stopped operation), `resume`, `recover` (a relaunch) and `remote`.
//!
//! [`Journal::unresolved`]: crate::journal::Journal::unresolved

mod engine;
mod error;
mod finish_merge;
mod fold;
mod record;
mod recover;
mod remote;
mod restart;
mod resume;
mod run;
mod set_aside;

use std::path::{Path, PathBuf};

use graft_core::journal::{EntryId, EntryStatus, JournalEntry, PutBack, PutBackWhy, Touches};
use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::repo::RepoHandle;
use graft_git::run::{RunError, Stop};
use graft_git::state::{self, InProgress};
use graft_git::version::GitVersion;

use crate::clash::Clash;
use crate::facts::{self, FactsError};
use crate::journal::{Journal, JournalError};
use crate::operation::{Invocation, Operation, Refusal, RepoFacts};

pub use error::{AfterState, ApplyError, EditsAt};
pub use recover::{Recovered, recover_interrupted};
pub use remote::RemoteHooks;
pub use restart::{RestartPlan, RestartReadError, StageSource, restart_gate, stage_source};
pub(crate) use resume::live_entry;
pub use resume::{Resume, resume};

pub(crate) use error::{as_apply_error, reason_for};
pub(crate) use record::{Opening, Pending, PrepareError, begin, finish, kind_name};
pub(crate) use restart::{RestartPrep, prepare_restart, run_restart_prep};

use engine::Engine;
use finish_merge::ending_a_shell_merge;
use record::{pins_for, put_back_for};
use restart::abandon_pending;
use run::{PlanRun, run_plan, with_cleanup};

/// What the caller has to re-read after an operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refresh {
    /// Status and diffs only.
    WorkingTree,
    /// Refs moved, so the graph as well.
    Everything,
}

/// An operation that ran, and its journal entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Applied {
    /// The entry as it was written, outcome line included.
    ///
    /// One exception, and it is deliberate: a step taken while git already
    /// has an operation in progress (the resolver's "mark resolved") is part
    /// of that operation and writes no line of its own
    /// ([`inside_live_operation`]). Its record here is **not in the
    /// journal** — it says what ran, for the caller's status line, and its
    /// command line was folded into the live entry instead. Read
    /// `description` and `refresh` from it; do not look its `id` up.
    ///
    /// The exception has an exception: inside an operation **begun in the
    /// shell**, a step that can write the working tree (a whole-file take, a
    /// restart) is an ordinary entry, because there is no entry of Graft's to
    /// fold it into or chain its backup onto (`fold::folded_step`).
    pub entry: JournalEntry,
    /// What to re-read.
    pub refresh: Refresh,
    /// The chained backup commit taken **before** a folded step ran, when
    /// there was one ([`snapshot::chain_backup`]).
    ///
    /// `Some` only for the exception above: a step inside a live operation
    /// that writes the index or the working tree. It is not in the journal
    /// either — the entry's backup ref points at it, and `d` reads the
    /// conflict's stages back out of it.
    pub step_backup: Option<Oid>,
    /// What an undo or redo forced past drift replaced: the user's own edits
    /// since the operation, which its backup keeps and the next `U` (or `u`
    /// after a redo) puts back (m6-conflicts/77). Empty for every operation
    /// and for a clean undo.
    pub replaced: crate::undo::Replaced,
}

/// Everything one operation needs to run and be recorded.
pub struct Context<'a> {
    /// The repository being written.
    pub repo: &'a RepoHandle,
    /// The system git, read once (`check` needs it for feature minimums).
    pub git: GitVersion,
    /// This worktree's name, for the backup ref's namespace (ADR-0010).
    pub worktree: String,
    /// `<git-path graft>`: the journal, and the temporary index copies.
    pub scratch: PathBuf,
    /// The open journal.
    pub journal: &'a mut Journal,
    /// The clock, passed in so entry ids are testable.
    pub now_unix_ms: i64,
    /// The ULID's random half, likewise.
    pub random: [u8; 10],
    /// Cancellation.
    pub stop: &'a dyn Stop,
    /// Progress and credential prompts for a remote operation; `None` is
    /// headless (no prompt, no progress). [`Context::open`] leaves it `None`.
    pub remote: Option<RemoteHooks>,
    /// The user's `graph.default_branch`, when set: it wins over git's guess
    /// when a rebase finds the branches built on his (ADR-0001 rule 1,
    /// ADR-0047 §4) — the default branch never follows. [`Context::open`]
    /// leaves it `None`, git's guess; the app sets it.
    pub default_branch: Option<RefName>,
}

impl<'a> Context<'a> {
    /// Open a context over a repository and its already-open journal.
    pub fn open(
        repo: &'a RepoHandle,
        git: GitVersion,
        journal: &'a mut Journal,
        now_unix_ms: i64,
        random: [u8; 10],
        stop: &'a dyn Stop,
    ) -> Result<Self, ApplyError> {
        let scratch = journal
            .path()
            .parent()
            .ok_or_else(|| ApplyError::Journal {
                error: JournalError::Io(std::io::Error::other(
                    "the journal has no directory to keep scratch in",
                )),
                after: None,
                kept: None,
            })?
            .to_path_buf();
        let worktree = repo.worktree_name();
        Ok(Context {
            repo,
            git,
            worktree,
            scratch,
            journal,
            now_unix_ms,
            random,
            stop,
            remote: None,
            default_branch: None,
        })
    }

    /// The entry id for this context's one operation.
    ///
    /// A `Context` carries one clock reading and one random half, so it
    /// names one entry: open a fresh context per operation (and per undo,
    /// which is itself an operation).
    pub fn next_id(&self) -> EntryId {
        EntryId::new(self.now_unix_ms, self.random)
    }

    /// Move on to the next entry id within one call.
    ///
    /// Almost every call journals one entry and keeps one id. The time
    /// machine (§6.6) is the exception: it journals one `Undo` entry per
    /// step, and two entries may well be minted inside the same
    /// millisecond, so the clock alone cannot tell them apart. The random
    /// half is incremented as an 80-bit big-endian counter — ULID's own
    /// rule for entries that share a timestamp — which keeps the ids
    /// distinct *and* in the order they were written.
    pub(crate) fn advance(&mut self) {
        for byte in self.random.iter_mut().rev() {
            let (next, carried) = byte.overflowing_add(1);
            *byte = next;
            if !carried {
                return;
            }
        }
        // All ten bytes wrapped: astronomically unlikely, and the only
        // honest thing left is a different millisecond.
        self.now_unix_ms = self.now_unix_ms.saturating_add(1);
    }

    /// The directory the journal and the temporary index copies live in.
    pub(crate) fn scratch(&self) -> &Path {
        &self.scratch
    }
}

/// Run one operation, journaled.
pub fn apply(ctx: &mut Context<'_>, op: &Operation) -> Result<Applied, ApplyError> {
    if ctx.stop.should_stop() {
        return Err(ApplyError::Stopped);
    }
    let facts = facts::read_facts(ctx.repo, ctx.git, ctx.stop, &op.facts_scope())
        .map_err(ApplyError::Facts)?;
    op.check(&facts).map_err(ApplyError::Refused)?;
    // `check` could not see whether a restart's stages still exist, so this
    // finishes its gate — and it runs before the `Pending` line, because a
    // refusal must leave nothing behind.
    let prep = prepare_restart(ctx, op, &facts)?;
    if let Some(folded) = fold::folded_step(ctx, op, &facts, &prep)? {
        return Ok(folded);
    }
    guard_repush(ctx, op, &facts)?;
    guard_untracked(ctx, op, &facts)?;
    let description = ending_a_shell_merge(ctx, op)?.unwrap_or_else(|| op.describe_in(&facts));
    // A rebase or rewrite: read the line of branches and write the todo
    // before `touches`, which records every branch of the line (ADR-0047).
    let engine = engine::prepare(ctx, op, &facts)?;
    let touches = touches_of(op, &facts, engine.as_ref());
    let plan = op.plan(&facts);

    // ADR-0045 §4, step 1: a rebase or checkout over changed tracked files
    // makes `S` before the entry opens, so the Pending line can name it.
    let mut commands = Vec::new();
    let aside_words = set_aside::words(&description);
    let aside = set_aside::create(ctx, op, &facts, &aside_words, &mut commands)?;
    let mut pins = pins_for(op);
    pins.extend(aside);
    let opening = Opening {
        kind: op.kind(),
        description,
        touches,
        pins,
        reverses: None,
        put_back: put_back_for(op).or(aside.map(|stash| PutBack {
            stash,
            why: PutBackWhy::SetAside,
        })),
    };
    // Step 2.
    let mut pending = open_entry(ctx, opening, engine.as_ref())?;
    // Steps 3 and 4.
    if let Some(stash) = aside
        && let Err(error) = set_aside::put_aside(ctx, stash, &aside_words, &mut commands)
    {
        let left = engine.as_ref().and_then(Engine::remove_plan);
        return set_aside::abandon(ctx, pending, commands, stash, error, left);
    }

    // After the before-snapshot, never before it: the state `u` puts back is
    // the resolution the user is starting over from, not the stages this is
    // about to restore over it.
    match run_restart_prep(ctx, &prep) {
        Ok(prepared) => commands.extend(prepared),
        Err(error) => return abandon_pending(ctx, &mut pending, error),
    }
    // Step 5 for a set-aside; `closed` runs step 6.
    let run = if let Some(engine) = &engine {
        commands.push(engine.command_line());
        engine.run(ctx, &mut commands)?
    } else {
        commands.extend(plan.iter().map(Invocation::command_line));
        run_plan(ctx, &plan, op.remote_names().as_ref())
    };
    closed(ctx, op, pending, commands, run)
}

/// Step 2, the `Pending` line — after a rewrite's plan is saved, never
/// before it (ADR-0047 §1), and with the plan removed again when the line
/// cannot be written.
fn open_entry(
    ctx: &mut Context<'_>,
    opening: Opening,
    engine: Option<&Engine>,
) -> Result<Pending, ApplyError> {
    if let Some(engine) = engine {
        engine.save(ctx.repo)?;
    }
    begin(ctx, ctx.next_id(), opening).map_err(|why| {
        let why = ApplyError::from(why);
        match engine {
            Some(engine) => engine.abandon(why),
            None => why,
        }
    })
}

/// What the entry records: the operation's own refs, and for a rewrite every
/// branch of its line beside them (ADR-0047 §2).
fn touches_of(op: &Operation, facts: &RepoFacts, engine: Option<&Engine>) -> Touches {
    let mut touches = op.touches(facts);
    for name in engine.into_iter().flat_map(Engine::refs) {
        if !touches.refs.iter().any(|recorded| recorded == name) {
            touches.refs.push(name.to_owned());
        }
    }
    touches
}

/// git stopped — on a conflict, or a rewrite's `edit` row: the entry stays
/// `Pending` with no outcome line and `resume` closes it (hard choice 2).
fn held(
    ctx: &mut Context<'_>,
    mut pending: Pending,
    commands: Vec<String>,
    kind: InProgress,
    paths: Vec<PathBuf>,
) -> ApplyError {
    ctx.journal
        .hold_commands(&pending.entry.id, commands.clone());
    pending.entry.commands = commands;
    ApplyError::Conflicted {
        entry: Box::new(pending.entry),
        kind,
        paths,
    }
}

/// The plan has run: close the entry the way git left the repository —
/// stopped on a conflict (the entry stays `Pending`), a put-back that
/// clashed, done, or failed.
fn closed(
    ctx: &mut Context<'_>,
    op: &Operation,
    mut pending: Pending,
    commands: Vec<String>,
    run: PlanRun,
) -> Result<Applied, ApplyError> {
    let PlanRun {
        failure,
        cleanup,
        edit_stop,
    } = run;
    // Stopped, not failed.
    //
    // `cleanup` is dropped on this path and nothing is lost by it: it is
    // filled by `remote::run`, which `run_plan` reaches only when
    // `remote_names()` is `Some`, while `starts_in_progress()` is `Some`
    // only for merge, rebase, cherry-pick and revert — all local; and by
    // `Engine::run` only when git did not stop. So here it is always empty.
    if let Some((kind, paths)) = stopped_on_conflict(ctx, op, failure.is_some() || edit_stop)? {
        return Err(held(ctx, pending, commands, kind, paths));
    }
    // A rebase or checkout that set edits aside: step 6, whether the
    // operation went through or git refused it (ADR-0045 §4).
    if let Some(stash) = set_aside::stash_of(&pending.entry) {
        let ended = set_aside::Ended {
            failure: failure.map(|failed| failed.error),
            done: EntryStatus::Committed,
            unread: cleanup,
        };
        return set_aside::close(ctx, pending, commands, stash, ended);
    }
    // A put-back that failed: the index says whether it clashed, never the
    // exit code (ADR-0045 §7). Read before the outcome, which it decides.
    // Only a failure is read: a put-back that went through left nothing
    // unmerged, and `close_unreadable_put_back` needs git's failure to name.
    let (failure, clash) = match failure {
        None => (None, None),
        Some(failed) => match clashed_put_back(ctx, &pending.entry) {
            Ok(clash) => (Some(failed), clash),
            Err(error) => {
                return Err(close_unreadable_put_back(
                    ctx,
                    &mut pending,
                    commands,
                    failed,
                    &error,
                ));
            }
        },
    };
    let status = match (&failure, &clash) {
        // A clash is an outcome, not a failure: the stash's changes are in
        // the tree with the conflicts, git kept the stash, and the entry is
        // in force so `u` is the way back (ADR-0045 §2, §6).
        (None, _) | (Some(_), Some(_)) => EntryStatus::Committed,
        (Some(failed), None) => EntryStatus::Aborted {
            reason: with_cleanup(failed.reason(), &cleanup),
        },
    };
    let unread = match finish(ctx, &mut pending, status, commands) {
        Ok(unread) => unread,
        // The outcome line is what makes the entry undoable, so this is the
        // headline — but git's own failure goes with it rather than under it.
        Err(error) => {
            return Err(ApplyError::Journal {
                error,
                after: failure.map(|failed| failed.error),
                kept: None,
            });
        }
    };

    let entry = pending.entry;
    if let Some(clash) = clash {
        return clashed(entry, clash, unread, cleanup);
    }
    match failure {
        None => succeeded(entry, op, unread, cleanup),
        // `finish` has already put the after-state failures, if any, into the
        // entry's `Aborted` reason: git's own failure is the headline, but
        // nothing is lost behind it.
        Some(failed) => Err(failed.into_error(entry)),
    }
}

/// ADR-0038 §4's ancestry rule, enforced where the push actually runs.
///
/// A re-push force-writes the remote branch to a commit the user picked out
/// of their own history, and the rule is that the commit must be **on that
/// branch**. Deciding it needs a read (`merge-base --is-ancestor`), and
/// `check` is pure over `RepoFacts` and cannot spawn — so the decision lived
/// in `preview::push` alone, and anything reaching `apply` without drawing a
/// picture first would have forced any commit at all under the lease
/// (m5/07 review, M5). `apply` can spawn, so it asks the same question in
/// the same terms, before the `Pending` line and before git is told to push.
///
/// Nothing else is re-checked here: every other precondition is a fact
/// `check` already holds.
fn guard_repush(ctx: &Context<'_>, op: &Operation, facts: &RepoFacts) -> Result<(), ApplyError> {
    let Operation::Push(params) = op else {
        return Ok(());
    };
    let Some(source) = params.source_commit else {
        return Ok(());
    };
    // A branch the facts cannot resolve is refused the same way a commit off
    // the branch is: an unverified re-push is not one that runs.
    let on_branch = match facts.refs.get(&params.branch).copied().flatten() {
        Some(tip) => {
            crate::preview::is_ancestor(ctx.repo, source, tip, ctx.stop).map_err(|error| {
                match error {
                    RunError::Stopped => ApplyError::Stopped,
                    other => ApplyError::Facts(FactsError::Ancestry(other)),
                }
            })?
        }
        None => false,
    };
    if on_branch {
        return Ok(());
    }
    Err(ApplyError::Refused(Refusal::NotOnBranch {
        commit: source,
        branch: crate::operation::short_ref(&params.branch).to_owned(),
    }))
}

/// A checked-out branch put back is a `reset --hard`, which writes over an
/// untracked file at a path the target holds without a word. `check` cannot
/// see trees, so the preview's question is asked again here, before the
/// `Pending` line: facts can change between the picture and `y` (m7/23,
/// conductor ruling).
fn guard_untracked(ctx: &Context<'_>, op: &Operation, facts: &RepoFacts) -> Result<(), ApplyError> {
    let Operation::Recover(params) = op else {
        return Ok(());
    };
    let paths = crate::preview::untracked_in_the_way(ctx.repo, params, facts, ctx.stop).map_err(
        |error| match error {
            RunError::Stopped => ApplyError::Stopped,
            other => ApplyError::Snapshot(crate::snapshot::SnapshotError::Run(other)),
        },
    )?;
    if paths.is_empty() {
        Ok(())
    } else {
        Err(ApplyError::Refused(Refusal::UnsavedEditsTo { paths }))
    }
}

/// The plan ran through: what the caller re-reads, and what the record of it
/// could not keep.
fn succeeded(
    entry: JournalEntry,
    op: &Operation,
    mut unread: Vec<AfterState>,
    cleanup: Vec<AfterState>,
) -> Result<Applied, ApplyError> {
    unread.extend(cleanup);
    // A remote operation moved refs the entry does not record (a fetch's
    // tracking refs), so the graph is re-read regardless; a history
    // operation moved `HEAD` or a ref the graph draws, even when
    // `touches.refs` is empty (a checkout, m5/07).
    let refresh = if entry.touches.refs.is_empty() && !op.is_remote() && !op.is_history() {
        Refresh::WorkingTree
    } else {
        Refresh::Everything
    };
    let applied = Applied {
        entry,
        refresh,
        step_backup: None,
        replaced: crate::undo::Replaced::default(),
    };
    if unread.is_empty() {
        Ok(applied)
    } else {
        // The operation itself succeeded; what failed is the record of it.
        // An `Err` the caller has to look at is the only way that reaches
        // the user, and `applied` is in it so the refresh still happens.
        Err(ApplyError::AfterState {
            applied: Box::new(applied),
            unread,
        })
    }
}

/// Did a failed put-back clash? Called only when git failed. `Some` when
/// the entry carries a `put_back` (a pop or apply) and the status lists
/// unmerged paths (ADR-0045 §1). Not [`stopped_on_conflict`]: a clash is
/// not an operation in progress — there is nothing to `c`ontinue or `a`bort.
///
/// Nothing unmerged is `None`: a plain failure (`apply --index` refusing,
/// an untracked file in the way), closed `Aborted` as before.
fn clashed_put_back(ctx: &Context<'_>, entry: &JournalEntry) -> Result<Option<Clash>, ApplyError> {
    let Some(put_back) = entry.put_back.as_ref() else {
        return Ok(None);
    };
    // Never stopped: the put-back has already run, and a stop that cut it
    // short must not also cut short the read that says what it left
    // (`record::read_after`, m3/03 review M1).
    let status =
        graft_git::status::read(ctx.repo, &()).map_err(|error| ApplyError::Facts(error.into()))?;
    let paths: Vec<PathBuf> = status
        .entries
        .into_iter()
        .filter(|entry| entry.unmerged)
        .map(|entry| entry.path)
        .collect();
    if paths.is_empty() {
        return Ok(None);
    }
    Ok(Some(Clash {
        stash: put_back.stash,
        why: put_back.why,
        paths,
    }))
}

/// A put-back failed and the status could not be read to say whether it
/// clashed. The entry is closed `Aborted` all the same — an entry left open
/// reads as an operation still running — with both failures in its reason,
/// and git's failure is the error returned. The after-state `finish` reads
/// includes the index, clash or not, so `u` puts the tree back either way.
fn close_unreadable_put_back(
    ctx: &mut Context<'_>,
    pending: &mut Pending,
    commands: Vec<String>,
    failure: run::PlanFailure,
    unread: &ApplyError,
) -> ApplyError {
    let status = EntryStatus::Aborted {
        reason: format!(
            "{}; the status after could not be read: {unread}",
            failure.reason()
        ),
    };
    if let Err(error) = finish(ctx, pending, status, commands) {
        return ApplyError::Journal {
            error,
            after: Some(failure.error),
            kept: None,
        };
    }
    failure.into_error(pending.entry.clone())
}

/// A put-back that clashed, journaled `Committed`: the answer is
/// [`ApplyError::Clashed`], which carries the entry so the caller still
/// refreshes. An after-state field that could not be read is not lost
/// behind it: it goes to `AfterState` instead, whose `applied` holds the
/// same entry — and the clash is then re-found from the journal by
/// `live_clash`.
fn clashed(
    entry: JournalEntry,
    clash: Clash,
    mut unread: Vec<AfterState>,
    cleanup: Vec<AfterState>,
) -> Result<Applied, ApplyError> {
    unread.extend(cleanup);
    if unread.is_empty() {
        return Err(ApplyError::Clashed {
            entry: Box::new(entry),
            clash,
        });
    }
    Err(ApplyError::AfterState {
        applied: Box::new(Applied {
            entry,
            refresh: Refresh::WorkingTree,
            step_backup: None,
            replaced: crate::undo::Replaced::default(),
        }),
        unread,
    })
}

/// Did git stop on a conflict rather than fail?
///
/// A non-zero exit from an operation that opens its own in-progress state,
/// with that state live afterwards, is a stop: `MERGE_HEAD` or
/// `rebase-merge/` is there and the work is half done. Anything else
/// non-zero is a failure and closes the entry (m5/07, hard choice 2). A
/// rewrite that exits 0 on an `edit` row is a stop too (`stopped`), with
/// nothing unmerged.
fn stopped_on_conflict(
    ctx: &Context<'_>,
    op: &Operation,
    stopped: bool,
) -> Result<Option<(InProgress, Vec<PathBuf>)>, ApplyError> {
    if !stopped {
        return Ok(None);
    }
    let Some(started) = op.starts_in_progress() else {
        return Ok(None);
    };
    let live = state::in_progress(ctx.repo).map_err(|error| ApplyError::Facts(error.into()))?;
    if live != Some(started) {
        return Ok(None);
    }
    Ok(Some((started, resume::unmerged_paths(ctx, &())?)))
}
