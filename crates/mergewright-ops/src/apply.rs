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

mod close;
mod engine;
mod error;
mod fetched;
mod finish_merge;
mod fold;
mod guard;
mod lost_edit;
mod pull;
mod record;
mod recover;
pub(crate) mod remote;
mod restart;
mod resume;
mod run;
mod set_aside;
mod stopped;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use mergewright_core::journal::{EntryId, JournalEntry, PutBack, PutBackWhy, Touches};
use mergewright_core::oid::Oid;
use mergewright_core::ref_name::RefName;
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::Stop;
use mergewright_git::version::GitVersion;

use crate::facts;
use crate::journal::{Journal, JournalError};
use crate::operation::{Invocation, Operation, RepoFacts};
use crate::snapshot::TempIndex;

pub use error::{AfterState, ApplyError, EditsAt, RawStepOf, askpass_words};
pub use recover::{Recovered, recover_interrupted};
pub use remote::RemoteHooks;
pub use restart::{RestartPlan, RestartReadError, StageSource, restart_gate, stage_source};
pub use resume::{Resume, resume};
pub(crate) use resume::{live_entry, live_entry_facts};
pub use set_aside::{came_back_from_index, came_back_words};
pub use stopped::StoppedAt;

pub(crate) use error::{as_apply_error, reason_for};
pub use record::kind_name;
pub(crate) use record::{Opening, Pending, PrepareError, begin, finish};
pub(crate) use restart::{RestartPrep, prepare_restart, run_restart_prep};

use close::closed;
use engine::Engine;
use finish_merge::ending_a_shell_merge;
use guard::{guard_in_the_way, guard_pick, guard_repush};
use pull::Ahead;
use record::{pins_for, put_back_for};
use restart::abandon_pending;

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
    /// (`inside_live_operation`). Its record here is **not in the
    /// journal** — it says what ran, for the caller's status line, and its
    /// command line was folded into the live entry instead. Read
    /// `description` and `refresh` from it; do not look its `id` up.
    ///
    /// The exception has an exception: inside an operation **begun in the
    /// shell**, a step that can write the working tree (a whole-file take, a
    /// restart) is an ordinary entry, because there is no entry of Mergewright's to
    /// fold it into or chain its backup onto (`fold::folded_step`).
    pub entry: JournalEntry,
    /// What to re-read.
    pub refresh: Refresh,
    /// The chained backup commit taken **before** a folded step ran, when
    /// there was one (`snapshot::chain_backup`).
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
    /// Files a set-aside put-back brought back from the index: new files he
    /// staged, then deleted from disk, which git's `stash apply --index`
    /// checks out again (m8-polish-and-ship/14,
    /// [`came_back_from_index`]). Nothing is lost; the landing line names
    /// them. Empty for every other operation.
    pub came_back: Vec<PathBuf>,
    /// Where a stop left git, when the user stopped this write and it
    /// finished anyway (m8-polish-and-ship/13c): `Some(Landed)` for a merge
    /// whose commit was made before the stop reached its `post-merge` hook.
    /// `None` for every write that was not stopped. A stop that paused or
    /// aborted the write is an error, read by
    /// [`ApplyError::stopped_at`].
    pub stopped_at: Option<StoppedAt>,
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
    /// Where this context's plan copies the index for its `Scratch` lines:
    /// `<scratch>/commit-index-<pid>-<n>`, named once per context so the
    /// Command Log and the run name the same file, and no other window's
    /// run can take it (review ease-pass-04, Major 1).
    scratch_index: PathBuf,
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
            .ok_or_else(|| {
                ApplyError::journal(JournalError::Io(std::io::Error::other(
                    "the journal has no directory to keep scratch in",
                )))
            })?
            .to_path_buf();
        let worktree = repo.worktree_name();
        let scratch_index = scratch.join(TempIndex::unique_name(run::SCRATCH_INDEX));
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
            scratch_index,
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

    /// This context's scratch index, for a plan's `Scratch` lines.
    pub(crate) fn scratch_index(&self) -> &Path {
        &self.scratch_index
    }
}

/// Run one operation, journaled.
pub fn apply(ctx: &mut Context<'_>, op: &Operation) -> Result<Applied, ApplyError> {
    if ctx.stop.should_stop() {
        return Err(ApplyError::Stopped);
    }
    let mut facts = facts::read_facts(ctx.repo, ctx.git, ctx.stop, &op.facts_scope())
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
    guard_pick(ctx, op)?;
    guard_in_the_way(ctx, op, &facts)?;
    let description = ending_a_shell_merge(ctx, op)?.unwrap_or_else(|| op.describe_in(&facts));
    // A rebase or rewrite: read the line of branches and write the todo
    // before `touches`, which records every branch of the line (ADR-0047).
    let engine = engine::prepare(ctx, op, &facts)?;
    let plan = op.plan(&facts);
    // A pull's fetch runs here, before `touches` and the set-aside, which
    // read what it brought (m7-power-tools/03b); a fetch's refs are listed
    // here, before anything is written (ADR-0062).
    let ahead = pull::fetch_first(ctx, op, &mut facts, &plan)?;
    let touches = touches_of(op, &facts, engine.as_ref());

    // ADR-0045 §4, step 1: a rebase or checkout over changed tracked files
    // makes `S` before the entry opens, so the Pending line can name it.
    let mut commands = Vec::new();
    // The fetch has run: its line goes first, as it ran first.
    let to_run = match &ahead {
        Ahead::Fetched { .. } => {
            commands.extend(plan.first().map(Invocation::command_line));
            plan.get(1..).unwrap_or_default()
        }
        Ahead::Nothing | Ahead::Fetch(_) | Ahead::Failed { .. } => &plan[..],
    };
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
    let mut pending = pull::open_after(ctx, &ahead, opening, engine.as_ref())?;
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
        commands.extend(to_run.iter().map(|line| run::logged_line(ctx, line)));
        pull::run_rest(ctx, op, ahead, to_run, &mut pending.entry)
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
