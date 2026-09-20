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
//! [`Journal::unresolved`]: crate::journal::Journal::unresolved

mod remote;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use graft_core::journal::{EntryId, EntryStatus, HeadRef, JournalEntry, OperationKind, Touches};
use graft_core::oid::Oid;
use graft_git::remote::RemoteFailure;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, RunError, Stop};
use graft_git::state::{self, InProgress};
use graft_git::version::GitVersion;

use crate::facts::{self, FactsError};
use crate::journal::{Journal, JournalError};
use crate::operation::{Invocation, Operation, Refusal, RemoteNames, RepoFacts};
use crate::snapshot::{self, SnapshotError, SnapshotScope};
use crate::undo::RepoStateNow;

pub use remote::RemoteHooks;

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
    pub entry: JournalEntry,
    /// What to re-read.
    pub refresh: Refresh,
}

/// Why an operation did not run, or did not finish.
#[derive(Debug, thiserror::Error)]
pub enum ApplyError {
    /// A precondition said no. Nothing was written and nothing journaled.
    #[error("{}", .0.explain())]
    Refused(Refusal),
    /// The repository could not be read.
    #[error("{0}")]
    Facts(FactsError),
    /// The backup could not be taken. The operation did not run.
    #[error("{0}")]
    Snapshot(SnapshotError),
    /// The journal could not be written. Before the operation ran, that means
    /// it did not run: a write Graft cannot record is a write it must not
    /// make. After it ran, the outcome line is missing — and `after` carries
    /// git's own failure, which would otherwise be lost behind this one
    /// (m3/03 review, Minor 1).
    #[error("{}", journal_failed(.error, .after.as_ref()))]
    Journal {
        /// What the journal said.
        error: JournalError,
        /// What git said, when the operation had already failed.
        after: Option<RunError>,
    },
    /// The operation ran and is journaled, but part of what it left behind
    /// could not be read, or could not be kept reachable. The write happened,
    /// so the caller still refreshes — `applied` carries it — and `u` still
    /// works, more widely than it should.
    #[error("{}: {}", .applied.entry.description, joined(.unread))]
    AfterState {
        /// What ran. Boxed for the same reason as `Git`'s entry.
        applied: Box<Applied>,
        /// What could not be read or sealed, in the order it was tried.
        unread: Vec<AfterState>,
    },
    /// `git` failed part-way. The entry is journaled `Aborted`.
    #[error("{}: {error}", .entry.description)]
    Git {
        /// The entry, with whatever after-state could be read. Boxed: an
        /// entry is much larger than every other variant.
        entry: Box<JournalEntry>,
        /// What `git` said.
        error: RunError,
        /// A backup exists, so `u` can put the repository back.
        recoverable: bool,
    },
    /// The caller asked to stop.
    #[error("stopped")]
    Stopped,
    /// A fetch, pull or push failed. The entry is journaled `Aborted` with
    /// the failure's short text; `failure` is what the whole of git's stderr
    /// said (a stop is `Cancelled`).
    #[error("{}", remote::explain(.entry, .failure, .stderr_tail, .names))]
    Remote {
        /// The entry, with whatever after-state could be read.
        entry: Box<JournalEntry>,
        /// Why, as far as git said.
        failure: RemoteFailure,
        /// git's last few stderr segments, for a detail view.
        stderr_tail: Vec<String>,
        /// The branch and remote the failure line names, from the
        /// operation's params (`Operation::remote_names`).
        names: Box<RemoteNames>,
    },
    /// A history operation stopped on a conflict: git exited non-zero and
    /// left its own in-progress state behind (`MERGE_HEAD`, `rebase-merge/`
    /// …). The entry stays `Pending` — one entry from start to finish, M5
    /// design note hard choice 2 — and [`resume`] closes it. No outcome line
    /// is written here. `paths` are the unmerged paths the status lists
    /// (m5/07).
    #[error("{}: {} unmerged · resolve, then c continues, a aborts", entry.description, paths.len())]
    Conflicted {
        /// The entry, still `Pending`.
        entry: Box<JournalEntry>,
        /// The state git left: what the banner shows and what `resume` runs.
        kind: InProgress,
        /// The unmerged paths.
        paths: Vec<PathBuf>,
    },
    /// [`resume`] ran `--continue`, `--skip` or `--abort`, git refused, and
    /// its in-progress state is still there with no unmerged path to explain
    /// it (a hook said no, an empty pick). The entry stays `Pending`, the
    /// banner stays up; git's line says what to do (m5/07).
    #[error("{}: {} · the {} is still in progress", entry.description, reason_for(error), kind.name())]
    ResumeFailed {
        /// The entry, still `Pending`.
        entry: Box<JournalEntry>,
        /// The state still live.
        kind: InProgress,
        /// What git said.
        error: RunError,
    },
}

/// How [`resume`] takes a stopped history operation on (m5/07).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resume {
    /// `--continue`: every unmerged path must be staged first.
    Continue,
    /// `--skip`: drop the commit being replayed (rebase, cherry-pick,
    /// revert). A merge has nothing to skip.
    Skip,
    /// `--abort`: back to where the operation started.
    Abort,
}

/// A field of an entry's after-state that could not be read, or could not be
/// kept reachable.
///
/// The outcome line is written either way — an entry with no outcome reads as
/// an operation still in flight, which this one is not — so the failure has
/// nowhere else to go but back to the caller. It is never dropped: what it
/// costs the user is a wider `u` (an unreadable tree) or an `u` that refuses
/// (unreadable refs, which `generic_steps` reads as `Incomplete`), and they
/// are owed the reason at the moment it happens rather than a fortnight
/// later.
#[derive(Debug, thiserror::Error)]
pub enum AfterState {
    /// `HEAD` could not be read after the operation.
    #[error("HEAD after the operation: {0}")]
    Head(FactsError),
    /// The refs the operation touched could not be read after it.
    #[error("the refs after the operation: {0}")]
    Refs(FactsError),
    /// The index could not be written as a tree after the operation.
    #[error("the index after the operation: {0}")]
    Index(SnapshotError),
    /// The working tree could not be written as a tree after the operation.
    #[error("the working tree after the operation: {0}")]
    Worktree(SnapshotError),
    /// The after-state was read but could not be hung off the backup ref, so
    /// it is not recorded: `git gc` would prune it and leave the entry naming
    /// objects that are gone.
    #[error("the after-state could not be kept reachable: {0}")]
    Unsealed(SnapshotError),
    /// The askpass server a remote invocation ran with could not be shut
    /// down cleanly: its socket or directory may be left in the temp dir.
    /// The operation itself is unaffected.
    #[error("the credential prompt's socket could not be cleaned up: {0}")]
    AskPass(std::io::Error),
    /// The entry names a backup ref that no longer exists, so the after-state
    /// read by `resume` has nothing to hang off and is not sealed: something
    /// deleted `refs/graft/backup/…` while the operation was stopped. `u`
    /// of this entry has no backup to restore from either (m5/07).
    #[error("the backup ref {name} is gone; the after-state is not kept reachable")]
    BackupGone {
        /// The ref the entry named.
        name: String,
    },
}

/// The journal failing, with git's own failure when there was one.
fn journal_failed(error: &JournalError, after: Option<&RunError>) -> String {
    match after {
        Some(ran) => format!("{error} (git had already failed: {})", reason_for(ran)),
        None => error.to_string(),
    }
}

/// Every after-state failure on one line.
fn joined(unread: &[AfterState]) -> String {
    unread
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

impl ApplyError {
    /// One line for the status line.
    pub fn explain(&self) -> String {
        match self {
            ApplyError::Git {
                entry,
                error,
                recoverable,
            } => {
                let mut line = format!("{}: {}", entry.description, reason_for(error));
                if *recoverable {
                    line.push_str(" · u restores the backup");
                }
                line
            }
            ApplyError::Remote {
                entry,
                failure,
                stderr_tail,
                names,
            } => remote::explain(entry, failure, stderr_tail, names),
            other => other.to_string(),
        }
    }
}

impl From<PrepareError> for ApplyError {
    fn from(why: PrepareError) -> Self {
        match why {
            PrepareError::Facts(why) => ApplyError::Facts(why),
            PrepareError::Snapshot(why) => ApplyError::Snapshot(why),
            PrepareError::Journal(why) => ApplyError::Journal {
                error: why,
                after: None,
            },
            PrepareError::Stopped => ApplyError::Stopped,
        }
    }
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
    if let Some(live) = facts.in_progress {
        return inside_live_operation(ctx, op, &facts, live);
    }
    guard_repush(ctx, op, &facts)?;
    let touches = op.touches(&facts);
    let plan = op.plan(&facts);
    let commands: Vec<String> = plan.iter().map(Invocation::command_line).collect();

    let mut pending = begin(
        ctx,
        ctx.next_id(),
        op.kind(),
        op.describe(),
        touches,
        pins_for(op),
        None,
    )?;

    let PlanRun { failure, cleanup } = run_plan(ctx, &plan, op.remote_names().as_ref());
    // Stopped, not failed: the entry stays `Pending` with no outcome line
    // and `resume` closes it (hard choice 2).
    //
    // `cleanup` is dropped on this path and nothing is lost by it: it is
    // filled only by `remote::run`, which `run_plan` reaches only when
    // `remote_names()` is `Some`, while `starts_in_progress()` is `Some`
    // only for merge, rebase, cherry-pick and revert — all local. The two
    // are disjoint by construction, so `cleanup` here is always empty.
    if let Some((kind, paths)) = stopped_on_conflict(ctx, op, failure.is_some())? {
        ctx.journal
            .hold_commands(&pending.entry.id, commands.clone());
        pending.entry.commands = commands;
        return Err(ApplyError::Conflicted {
            entry: Box::new(pending.entry),
            kind,
            paths,
        });
    }
    let status = match &failure {
        None => EntryStatus::Committed,
        Some(failed) => EntryStatus::Aborted {
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
            });
        }
    };

    let entry = pending.entry;
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

/// Run a step that belongs to an operation git already has in progress.
///
/// `check` admits exactly `Stage` and `Unstage` while a merge, rebase,
/// cherry-pick or revert is open — the resolver's "mark resolved". Such a
/// step is **part of that operation, not an operation of its own**: DESIGN
/// §6.4 ("a mid-flight operation … is not a journal entry yet … the undo
/// journal picks up once there's a finished operation to reverse") and M5
/// hard choice 2 (one entry, `Pending` from the stop to the finish). So it
/// runs, its command line joins the live entry, and it writes **no journal
/// line of its own**. `u` after the finish takes the whole span back.
///
/// Journaling it as an ordinary entry is what killed `u` outright (m5/07
/// review, C1): `write-tree` has no tree for an unmerged index, so the
/// entry's `index_before` was `None` and its inverse `NoBackup` — and undo
/// is linear, so the merge behind it became unreachable. The rule is "an
/// operation is in progress", not "the index cannot be snapshotted": the
/// last file a user resolves leaves the index clean, and that step must fold
/// into the span like every one before it.
///
/// Nothing is at risk for want of a backup. Every step this admits writes
/// only the index (`add`, `apply --cached`, `restore --staged`), and the
/// span's own entry holds the pre-operation snapshot.
fn inside_live_operation(
    ctx: &mut Context<'_>,
    op: &Operation,
    facts: &RepoFacts,
    live: InProgress,
) -> Result<Applied, ApplyError> {
    let plan = op.plan(facts);
    let commands: Vec<String> = plan.iter().map(Invocation::command_line).collect();
    let PlanRun { failure, .. } = run_plan(ctx, &plan, op.remote_names().as_ref());
    fold_commands(ctx, live, commands.clone())?;

    let mut record = JournalEntry {
        id: ctx.next_id(),
        at_unix_ms: ctx.now_unix_ms,
        kind: op.kind(),
        description: op.describe(),
        status: EntryStatus::Committed,
        touches: op.touches(facts),
        head_before: facts.head.clone(),
        head_after: Some(facts.head.clone()),
        refs_before: BTreeMap::default(),
        refs_after: BTreeMap::default(),
        index_before: None,
        index_after: None,
        worktree_before: None,
        worktree_after: None,
        backup_ref: None,
        pins: Vec::new(),
        commands,
        reverses: None,
    };
    match failure {
        None => Ok(Applied {
            entry: record,
            refresh: Refresh::WorkingTree,
        }),
        Some(failed) => {
            record.status = EntryStatus::Aborted {
                reason: failed.reason(),
            };
            Err(failed.into_error(record))
        }
    }
}

/// Record a folded step on the live entry: its command line after the ones
/// already held, so the Command Log shows the span in the order it happened.
///
/// A `Pending` entry's commands live in memory until its outcome line is
/// written, so a caller that opens a fresh journal per job sees only the
/// step it just ran (the same limitation the stop itself has — the plan's
/// own lines do not reach disk either until `resume` closes the entry).
fn fold_commands(
    ctx: &mut Context<'_>,
    live: InProgress,
    commands: Vec<String>,
) -> Result<(), ApplyError> {
    let Some((id, mut held)) = live_entry(ctx.repo, ctx.journal.entries(), live, ctx.stop)?
        .map(|entry| (entry.id.clone(), entry.commands.clone()))
    else {
        // An operation begun outside Graft has no entry of Graft's to record
        // the step on. Nothing is dropped that was ever ours: git's own state
        // is the whole record of that operation, and the step is reported to
        // the caller on the returned `Applied`.
        return Ok(());
    };
    held.extend(commands);
    ctx.journal.hold_commands(&id, held);
    Ok(())
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
    let applied = Applied { entry, refresh };
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

/// Did git stop on a conflict rather than fail?
///
/// A non-zero exit from an operation that opens its own in-progress state,
/// with that state live afterwards, is a stop: `MERGE_HEAD` or
/// `rebase-merge/` is there and the work is half done. Anything else
/// non-zero is a failure and closes the entry (m5/07, hard choice 2).
fn stopped_on_conflict(
    ctx: &Context<'_>,
    op: &Operation,
    failed: bool,
) -> Result<Option<(InProgress, Vec<PathBuf>)>, ApplyError> {
    if !failed {
        return Ok(None);
    }
    let Some(started) = op.starts_in_progress() else {
        return Ok(None);
    };
    let live = state::in_progress(ctx.repo).map_err(|error| ApplyError::Facts(error.into()))?;
    if live != Some(started) {
        return Ok(None);
    }
    Ok(Some((started, unmerged_paths(ctx)?)))
}

/// What running a plan came to.
struct PlanRun {
    /// The invocation that failed, if one did.
    failure: Option<PlanFailure>,
    /// Askpass servers that would not shut down cleanly. Never fatal.
    cleanup: Vec<AfterState>,
}

/// The invocation that failed.
struct PlanFailure {
    error: RunError,
    /// For a remote operation: what went wrong in its terms.
    remote: Option<RemoteFailed>,
}

/// A remote operation's failure: what the whole of git's stderr said, its
/// last few segments, and the operation's names for the failure line.
struct RemoteFailed {
    failure: RemoteFailure,
    stderr_tail: Vec<String>,
    names: Box<RemoteNames>,
}

impl PlanFailure {
    /// The entry's `Aborted` reason.
    fn reason(&self) -> String {
        match &self.remote {
            Some(failed) => remote::short(&failed.failure),
            None => reason_for(&self.error),
        }
    }

    /// The error `apply` returns, the entry journaled.
    fn into_error(self, entry: JournalEntry) -> ApplyError {
        if let Some(RemoteFailed {
            failure,
            stderr_tail,
            names,
        }) = self.remote
        {
            return ApplyError::Remote {
                entry: Box::new(entry),
                failure,
                stderr_tail,
                names,
            };
        }
        let recoverable = entry.worktree_before.is_some();
        ApplyError::Git {
            entry: Box::new(entry),
            error: self.error,
            recoverable,
        }
    }
}

/// An `Aborted` reason with any askpass cleanup failure after it.
fn with_cleanup(reason: String, cleanup: &[AfterState]) -> String {
    if cleanup.is_empty() {
        reason
    } else {
        format!("{reason}; {}", joined(cleanup))
    }
}

/// Run the plan's invocations in order, stopping at the first that fails.
///
/// The order is the operation's own: group 02 puts the step that can fail
/// first, so a failure leaves the least behind it. A remote operation's
/// invocations go through `remote::run`, which keeps their stderr; a local
/// operation's run exactly as before. `names` is `Some` exactly for a remote
/// operation (`Operation::remote_names`).
fn run_plan(ctx: &Context<'_>, plan: &[Invocation], names: Option<&RemoteNames>) -> PlanRun {
    let mut cleanup = Vec::new();
    for invocation in plan {
        // No plan step ever wants an editor: every message travels on stdin
        // or in `-m`. A step that would still open one (`merge` writing its
        // message, `cherry-pick` without `--no-edit`) gets `true` and
        // carries on with git's own text (M5 design note, G5 plan).
        let mut git = GitInvocation::new(ctx.repo)
            .args(invocation.args.iter().cloned())
            .env("GIT_EDITOR", "true");
        if let Some(bytes) = &invocation.stdin {
            git = git.stdin(bytes.clone());
        }
        let failure = if let Some(names) = names {
            remote::run(ctx, git, invocation, &mut cleanup)
                .err()
                .map(|failed| PlanFailure {
                    remote: Some(RemoteFailed {
                        failure: remote::failure_of(&failed.error, &failed.segments),
                        stderr_tail: remote::tail(&failed.segments),
                        names: Box::new(names.clone()),
                    }),
                    error: failed.error,
                })
        } else {
            git.run(ctx.stop).err().map(|error| PlanFailure {
                error,
                remote: None,
            })
        };
        if failure.is_some() {
            return PlanRun { failure, cleanup };
        }
    }
    PlanRun {
        failure: None,
        cleanup,
    }
}

/// Close every entry left `Pending` by a previous run as `interrupted`,
/// recording the repository as it stands now (DESIGN.md §6.2 step 3).
///
/// Run this at launch, before the user can touch anything: the state read
/// here is recorded as the interrupted operation's after-state, so the later
/// it runs, the more of the user's own work it attributes to that operation.
/// When the repository cannot be read at all, the entry is closed with no
/// after-state — `u` then offers to restore the whole tree, and says so.
///
/// Returns what it closed **and** what it could not read, so the caller can
/// surface both: an entry closed with no after-state is one whose `u` will
/// offer to overwrite the whole working tree, and the reason for that belongs
/// on screen, not on the floor (m3/03 review, M1).
pub fn recover_interrupted(ctx: &mut Context<'_>) -> Result<Recovered, ApplyError> {
    // A conflict that survived a relaunch: git's own state says the
    // operation is still live, so its entry stays `Pending` and the banner
    // takes over (m5/07, hard choice 2). Only the newest matching entry is
    // live — an older one of the same kind was left behind by something
    // else and closes as interrupted like any other.
    let live = state::in_progress(ctx.repo).map_err(|error| ApplyError::Facts(error.into()))?;
    let live_id = match live {
        Some(kind) => live_entry(ctx.repo, ctx.journal.entries(), kind, ctx.stop)?
            .map(|entry| entry.id.clone()),
        None => None,
    };
    let open: Vec<(EntryId, Vec<String>, Option<String>)> = ctx
        .journal
        .unresolved()
        .into_iter()
        .filter(|entry| Some(&entry.id) != live_id.as_ref())
        .map(|entry| {
            (
                entry.id.clone(),
                entry.refs_before.keys().cloned().collect(),
                entry.backup_ref.clone(),
            )
        })
        .collect();
    let mut recovered = Recovered {
        live: live_id.into_iter().collect(),
        ..Recovered::default()
    };
    for (id, refs, backup_ref) in open {
        let read = facts::read_state_now(ctx.repo, ctx.scratch(), &refs, ctx.stop);
        // The repository being unreadable must not leave the entry `Pending`
        // for ever, which blocks `u` outright. `None` is the honest record
        // and the widest restore, and the user is asked before it runs
        // (`Drift::unknown`) — and told why, here.
        let now = match read {
            Ok(now) => match seal_recovered(ctx, &id, backup_ref.as_deref(), &now) {
                Ok(()) => Some(now),
                // The trees were read but nothing keeps them alive, so
                // recording them would name objects `git gc` will prune.
                Err(why) => {
                    recovered.unreadable.push((id.clone(), why));
                    None
                }
            },
            Err(FactsError::Stopped) => return Err(ApplyError::Stopped),
            Err(unreadable) => {
                recovered.unreadable.push((id.clone(), unreadable));
                None
            }
        };
        ctx.journal
            .mark_interrupted(&id, now.as_ref())
            .map_err(|error| ApplyError::Journal { error, after: None })?;
        recovered.closed.push(id);
    }
    Ok(recovered)
}

/// What [`recover_interrupted`] did at launch.
#[derive(Debug, Default)]
pub struct Recovered {
    /// The entries closed as `interrupted`, in journal order.
    pub closed: Vec<EntryId>,
    /// The ones whose after-state could not be read or kept reachable, and
    /// why. Each is closed with no after-state, so its `u` restores the whole
    /// tree after asking.
    pub unreadable: Vec<(EntryId, FactsError)>,
    /// The entry left `Pending` on purpose: a history operation still
    /// stopped on a conflict by git's own account (`MERGE_HEAD`,
    /// `rebase-merge/` …). The banner offers `resume`; nothing was closed
    /// (m5/07). At most one.
    pub live: Vec<EntryId>,
}

/// The newest `Pending` entry that is genuinely *this* live operation of
/// Graft's: the entry `resume` closes and recovery leaves alone (m5/07).
///
/// Kind alone was not enough (m5/07 review, M7). A Graft merge entry left
/// `Pending` by a crash *before* git ran, plus a `git merge` since started
/// in the shell, matched on kind — and `resume(Continue)` would have
/// finished someone else's merge and closed Graft's entry `Committed` with
/// a before-state from a different act. Two facts git leaves behind settle
/// it, both read here rather than assumed:
///
/// - **Where the operation started.** A rebase detaches `HEAD`, so its
///   branch is `rebase-merge/head-name`; a merge, cherry-pick or revert
///   leaves `HEAD` where it was. Either way the entry's `head_before` must
///   equal it. A `rebase-apply` directory with no `head-name` is a `git am`,
///   which matches no entry of Graft's at all (this also closes the group's
///   own Minor 11).
/// - **The branch has not moved.** A stopped merge, cherry-pick or revert
///   has written no commit, so the branch is still where the entry recorded
///   it. A rebase's own starting tip is in `rebase-merge/orig-head`, which
///   `graft_git::state` does not expose, so that leg rests on `head-name`
///   alone.
///
/// `ORIG_HEAD` is deliberately not used, although the review named it:
/// verified against git 2.54, a merge and a rebase set it but a conflicted
/// **cherry-pick and revert do not**, so requiring it would refuse two real
/// resumes to catch one impostor.
///
/// A fact the entry never recorded is not evidence against it: the
/// comparison is skipped rather than failed, which keeps a real conflict
/// resumable.
fn live_entry<'a>(
    repo: &RepoHandle,
    entries: &'a [JournalEntry],
    kind: InProgress,
    stop: &dyn Stop,
) -> Result<Option<&'a JournalEntry>, ApplyError> {
    if stop.should_stop() {
        return Err(ApplyError::Stopped);
    }
    let detail = state::in_progress_detail(repo).map_err(|error| match error {
        state::StateError::Io(io) => ApplyError::Facts(FactsError::State(io)),
    })?;
    let started_on = match kind {
        InProgress::Rebase => match detail.as_ref().and_then(|found| found.head_name.as_deref()) {
            Some(name) => HeadRef::Branch(name.to_owned()),
            // A `git am`, or a rebase whose state file is unreadable:
            // nothing here can be shown to be Graft's.
            None => return Ok(None),
        },
        _ => facts::head_ref(repo).map_err(ApplyError::Facts)?,
    };
    let tip_now = match (&started_on, kind) {
        (
            HeadRef::Branch(name),
            InProgress::Merge | InProgress::CherryPick | InProgress::Revert,
        ) => facts::read_refs(repo, std::slice::from_ref(name))
            .map_err(ApplyError::Facts)?
            .get(name)
            .copied()
            .flatten()
            .map(|oid| (name.clone(), oid)),
        _ => None,
    };
    Ok(entries.iter().rev().find(|entry| {
        entry.status == EntryStatus::Pending
            && starts(entry.kind) == Some(kind)
            && entry.head_before == started_on
            && match &tip_now {
                Some((name, now)) => match entry.refs_before.get(name) {
                    Some(Some(recorded)) => recorded == now,
                    // Not recorded, or recorded absent: nothing to compare.
                    _ => true,
                },
                None => true,
            }
    }))
}

/// The in-progress state an operation of this kind opens when it stops.
fn starts(kind: OperationKind) -> Option<InProgress> {
    match kind {
        OperationKind::Merge => Some(InProgress::Merge),
        OperationKind::Rebase => Some(InProgress::Rebase),
        OperationKind::CherryPick => Some(InProgress::CherryPick),
        OperationKind::Revert => Some(InProgress::Revert),
        _ => None,
    }
}

/// The unmerged paths the status lists now.
fn unmerged_paths(ctx: &Context<'_>) -> Result<Vec<PathBuf>, ApplyError> {
    let status = graft_git::status::read(ctx.repo, ctx.stop)
        .map_err(|error| ApplyError::Facts(error.into()))?;
    Ok(status
        .entries
        .into_iter()
        .filter(|entry| entry.unmerged)
        .map(|entry| entry.path)
        .collect())
}

/// Take a stopped history operation on: `--continue`, `--skip` or `--abort`
/// on the newest `Pending` entry whose kind matches git's live state, with
/// `GIT_EDITOR=true` so the resolved commit's message is never edited
/// (m5/07, design note G5).
///
/// `Continue` with unmerged paths still in the index is refused before git
/// runs — git would refuse too, but with a line about `git add`. A rebase
/// may stop again on its next commit: that is [`ApplyError::Conflicted`]
/// again, on the same entry, and the entry stays `Pending`. On success the
/// after-state is read now and the outcome written `Committed`; `Abort`
/// writes `Aborted("aborted")` with the after-state read just as fresh — it
/// should equal the before-state, and when it does not (an edit made
/// mid-conflict) `in_force` treats it honestly and the backup keeps the
/// pre-operation tree. The resume command joins the entry's `commands`
/// after the plan's own (kept in memory since the stop; a relaunch loses
/// them, since only the outcome line records commands).
///
/// # Errors
/// [`Refusal::NotInProgress`] when git has no operation in progress or no
/// `Pending` entry of Graft's matches it (one begun in the shell finishes
/// there); [`Refusal::CannotSkip`] for `Skip` on a merge;
/// [`Refusal::Unmerged`] for `Continue` over unmerged paths; the executor's
/// own errors otherwise.
pub fn resume(ctx: &mut Context<'_>, how: Resume) -> Result<Applied, ApplyError> {
    if ctx.stop.should_stop() {
        return Err(ApplyError::Stopped);
    }
    let (kind, entry) = resume_target(ctx, how)?;

    let args = [
        GitArg::Flag(kind.name()),
        GitArg::Flag(match how {
            Resume::Continue => "--continue",
            Resume::Skip => "--skip",
            Resume::Abort => "--abort",
        }),
    ];
    let mut commands = entry.commands.clone();
    commands.push(graft_git::run::command_line(&args));
    let ran = GitInvocation::new(ctx.repo)
        .args(args)
        .env("GIT_EDITOR", "true")
        .run(ctx.stop);

    let failure = match ran {
        Ok(_) => None,
        Err(RunError::Stopped) => return Err(ApplyError::Stopped),
        Err(error) => {
            // Still live: git stopped again (a rebase's next conflict) or
            // refused to go on. Either way the entry is not finished.
            let live =
                state::in_progress(ctx.repo).map_err(|error| ApplyError::Facts(error.into()))?;
            if live == Some(kind) {
                return Err(still_pending(ctx, kind, entry, commands, error)?);
            }
            Some(error)
        }
    };
    close_resume(ctx, entry, how, commands, failure)
}

/// Which entry a resume closes, and the checks that run before git does.
///
/// git's own state names the kind; the journal's newest `Pending` entry of
/// that kind is the one Graft opened. An operation begun in the shell has no
/// entry here and is finished there ([`Refusal::NotInProgress`]).
fn resume_target(ctx: &Context<'_>, how: Resume) -> Result<(InProgress, JournalEntry), ApplyError> {
    let live = state::in_progress(ctx.repo).map_err(|error| ApplyError::Facts(error.into()))?;
    let Some(kind) = live else {
        return Err(ApplyError::Refused(Refusal::NotInProgress));
    };
    let Some(entry) = live_entry(ctx.repo, ctx.journal.entries(), kind, ctx.stop)?.cloned() else {
        return Err(ApplyError::Refused(Refusal::NotInProgress));
    };
    if how == Resume::Skip && kind == InProgress::Merge {
        return Err(ApplyError::Refused(Refusal::CannotSkip(kind)));
    }
    if how == Resume::Continue {
        let paths = unmerged_paths(ctx)?;
        if !paths.is_empty() {
            return Err(ApplyError::Refused(Refusal::Unmerged { paths }));
        }
    }
    Ok((kind, entry))
}

/// git would not finish and its state is still live: the entry stays
/// `Pending` and the banner stays up. Unmerged paths mean it stopped again
/// on a conflict; none mean it refused (a hook, an empty pick).
fn still_pending(
    ctx: &mut Context<'_>,
    kind: InProgress,
    mut entry: JournalEntry,
    commands: Vec<String>,
    error: RunError,
) -> Result<ApplyError, ApplyError> {
    ctx.journal.hold_commands(&entry.id, commands.clone());
    let paths = unmerged_paths(ctx)?;
    entry.commands = commands;
    Ok(if paths.is_empty() {
        ApplyError::ResumeFailed {
            entry: Box::new(entry),
            kind,
            error,
        }
    } else {
        ApplyError::Conflicted {
            entry: Box::new(entry),
            kind,
            paths,
        }
    })
}

/// git is done with the operation, one way or the other: write the outcome
/// line the stop deferred.
fn close_resume(
    ctx: &mut Context<'_>,
    entry: JournalEntry,
    how: Resume,
    commands: Vec<String>,
    failure: Option<RunError>,
) -> Result<Applied, ApplyError> {
    // The backup ref still points at the before-commit: `seal_after` moves
    // it only when the outcome is written, which is now.
    let before_commit = match &entry.backup_ref {
        Some(name) => facts::read_refs(ctx.repo, std::slice::from_ref(name))
            .map_err(ApplyError::Facts)?
            .get(name)
            .copied()
            .flatten(),
        None => None,
    };
    let backup_gone = entry
        .backup_ref
        .clone()
        .filter(|_| before_commit.is_none())
        .map(|name| AfterState::BackupGone { name });
    let mut pending = Pending {
        entry,
        before_commit,
    };
    let status = match (how, &failure) {
        (_, Some(error)) => EntryStatus::Aborted {
            reason: reason_for(error),
        },
        (Resume::Abort, None) => EntryStatus::Aborted {
            reason: "aborted".to_owned(),
        },
        (Resume::Continue | Resume::Skip, None) => EntryStatus::Committed,
    };
    let mut unread = match finish(ctx, &mut pending, status, commands) {
        Ok(unread) => unread,
        Err(error) => {
            return Err(ApplyError::Journal {
                error,
                after: failure,
            });
        }
    };
    unread.extend(backup_gone);
    let entry = pending.entry;
    if let Some(error) = failure {
        let recoverable = entry.worktree_before.is_some();
        return Err(ApplyError::Git {
            entry: Box::new(entry),
            error,
            recoverable,
        });
    }
    let applied = Applied {
        entry,
        refresh: Refresh::Everything,
    };
    if unread.is_empty() {
        Ok(applied)
    } else {
        Err(ApplyError::AfterState {
            applied: Box::new(applied),
            unread,
        })
    }
}

/// Hang a recovered entry's after-trees off its backup ref before they are
/// recorded, so `git gc` cannot prune what the entry is about to name.
///
/// Nothing to do when the entry took no backup: such an entry records no tree
/// (`mark_interrupted` copies its before-fields, which are `None`), and the
/// refs it names are live refs.
fn seal_recovered(
    ctx: &Context<'_>,
    id: &EntryId,
    backup_ref: Option<&str>,
    now: &RepoStateNow,
) -> Result<(), FactsError> {
    let Some(name) = backup_ref else {
        return Ok(());
    };
    let at = facts::read_refs(ctx.repo, std::slice::from_ref(&name.to_owned()))?
        .get(name)
        .copied()
        .flatten();
    let mut parents: Vec<Oid> = at.into_iter().collect();
    for anchor in anchors(Some(&now.head), &now.refs) {
        if !parents.contains(&anchor) {
            parents.push(anchor);
        }
    }
    let label = format!("interrupted, recovered: {id}");
    let commit = snapshot::commit_trees(
        ctx.repo,
        now.index_tree,
        Some(now.worktree_tree),
        id,
        &label,
        &parents,
        &(),
    )?;
    snapshot::move_backup_ref(ctx.repo, name, commit, &())?;
    Ok(())
}

/// An entry whose `Pending` line is written and whose backup, if any, is
/// taken. The before-commit is kept so the after-snapshot can hang off it.
pub(crate) struct Pending {
    pub(crate) entry: JournalEntry,
    pub(crate) before_commit: Option<Oid>,
}

/// Why an entry could not be opened. Both `apply` and `restore` map it into
/// their own error.
#[derive(Debug, thiserror::Error)]
pub(crate) enum PrepareError {
    #[error("{0}")]
    Facts(FactsError),
    #[error("{0}")]
    Snapshot(SnapshotError),
    #[error("{0}")]
    Journal(JournalError),
    #[error("stopped")]
    Stopped,
}

/// Read the before-state, take the backup, write the `Pending` line.
///
/// A backup is taken when the work touches the index or the working tree
/// (ADR-0009) **or** when it has pins: a pin is a commit the inverse needs
/// alive, and the backup commit's parents are the only thing keeping it that
/// way once `git gc` runs. `snapshot::scope` stays the one knob hard choice 3
/// names; pins are an executor concern and sit beside it.
pub(crate) fn begin(
    ctx: &mut Context<'_>,
    id: EntryId,
    kind: OperationKind,
    description: String,
    touches: Touches,
    pins: Vec<Oid>,
    reverses: Option<EntryId>,
) -> Result<Pending, PrepareError> {
    if ctx.stop.should_stop() {
        return Err(PrepareError::Stopped);
    }
    let head_before = facts::head_ref(ctx.repo).map_err(PrepareError::Facts)?;
    let refs_before = facts::read_refs(ctx.repo, &touches.refs).map_err(PrepareError::Facts)?;

    let wanted = snapshot::scope(&touches) == SnapshotScope::Full || !pins.is_empty();
    // The snapshot's parents are the pins **and** every commit the entry's
    // before-fields name: the pre-amend commit, the branch tip a commit is
    // about to move off. Both are otherwise reachable only through a reflog,
    // which `gc.reflogExpireUnreachable` drops after 30 days, and after that
    // undoing an amend would lose the commit outright (m3/03 review, C1).
    // Every name here is a branch or `refs/stash`, so every value is a commit
    // git will take as a parent.
    let mut parents = pins.clone();
    for anchor in anchors(Some(&head_before), &refs_before) {
        if !parents.contains(&anchor) {
            parents.push(anchor);
        }
    }
    let (backup, backup_ref) = if wanted {
        let label = format!("{}: {description}", kind_name(kind));
        let taken = snapshot::take(
            ctx.repo,
            ctx.scratch(),
            &id,
            &ctx.worktree,
            &label,
            &parents,
            ctx.stop,
        )
        .map_err(PrepareError::Snapshot)?;
        let name = snapshot::backup_ref_name(&ctx.worktree, &id);
        (Some(taken), Some(name))
    } else {
        (None, None)
    };

    let entry = JournalEntry {
        id,
        at_unix_ms: ctx.now_unix_ms,
        kind,
        description,
        status: EntryStatus::Pending,
        touches,
        head_before,
        head_after: None,
        refs_before,
        refs_after: BTreeMap::default(),
        index_before: backup.and_then(|taken| taken.index_tree),
        index_after: None,
        worktree_before: backup.map(|taken| taken.worktree_tree),
        worktree_after: None,
        backup_ref,
        pins,
        commands: Vec::new(),
        reverses,
    };
    ctx.journal
        .append_pending(&entry)
        .map_err(PrepareError::Journal)?;
    Ok(Pending {
        entry,
        before_commit: backup.map(|taken| taken.commit),
    })
}

/// Read the after-state, keep it reachable, write the outcome line.
///
/// Every after-field the repository will give up is recorded, on abort as
/// much as on success: `refs_after` for every key of `refs_before`,
/// `head_after`, and each tree the entry touches. The outcome line is written
/// either way, since an entry with no outcome reads as an operation still in
/// flight, which this one is not — but a field that could not be read is
/// **returned**, never dropped. A missing tree costs the user a wider `u`
/// behind a confirm box; missing refs cost them an `u` that refuses
/// (`generic_steps` reads them as `Incomplete`), and either way they are told.
///
/// Then the after-state is sealed: an after-commit over whatever trees were
/// readable, parented on the before-commit and on every commit the
/// after-fields name, with the backup ref moved to it. **Whenever there is a
/// backup ref**, not only for worktree operations — an index-only operation's
/// `index_after` is a loose tree no commit holds, and `git gc` prunes it two
/// weeks later, long after the entry promised the operation could be undone
/// (m3/03 review, C1).
pub(crate) fn finish(
    ctx: &mut Context<'_>,
    pending: &mut Pending,
    status: EntryStatus,
    commands: Vec<String>,
) -> Result<Vec<AfterState>, JournalError> {
    let entry = &mut pending.entry;
    entry.commands = commands;
    let mut unread = read_after(ctx, entry);
    if let Some(before) = pending.before_commit {
        unread.extend(seal_after(ctx, entry, before));
    }

    entry.status = match status {
        EntryStatus::Aborted { reason } if !unread.is_empty() => EntryStatus::Aborted {
            reason: format!("{reason}; {}", joined(&unread)),
        },
        other => other,
    };
    ctx.journal.append_outcome(entry)?;
    Ok(unread)
}

/// Read every after-field the repository will give up, and say which it
/// would not.
///
/// The operation has already run, so the reads never stop: a stop that
/// cancelled it mid-plan (Esc during a slow pop) must not also cancel the
/// record of what it left behind, which would record a perfectly readable
/// repository as unknown and force the widest restore (m3/03 review, M1).
fn read_after(ctx: &Context<'_>, entry: &mut JournalEntry) -> Vec<AfterState> {
    let never = &();
    let mut unread = Vec::new();
    match facts::head_ref(ctx.repo) {
        Ok(head) => entry.head_after = Some(head),
        Err(why) => unread.push(AfterState::Head(why)),
    }
    match facts::read_refs(ctx.repo, &entry.touches.refs) {
        Ok(refs) => entry.refs_after = refs,
        Err(why) => unread.push(AfterState::Refs(why)),
    }
    if entry.touches.index {
        match snapshot::index_tree(ctx.repo, ctx.scratch(), never) {
            Ok(tree) => entry.index_after = Some(tree),
            // An unmerged index has no tree at all: a conflicted pop is the
            // shape this exists for, and the working tree still lands.
            Err(why) => unread.push(AfterState::Index(why)),
        }
    } else {
        entry.index_after = entry.index_before;
    }
    if entry.touches.worktree {
        match snapshot::worktree_tree(ctx.repo, ctx.scratch(), never) {
            Ok(tree) => entry.worktree_after = Some(tree),
            Err(why) => unread.push(AfterState::Worktree(why)),
        }
    } else {
        entry.worktree_after = entry.worktree_before;
    }
    unread
}

/// Hang the after-state off the entry's backup ref.
///
/// The commit's parents are the before-commit — which carries the
/// before-trees and the pins — and every commit the after-fields name, so one
/// ref keeps everything the entry says reachable. A failure drops the
/// freshly-read trees from the entry: a tree nothing reaches is a tree
/// `git gc` prunes, and an entry naming one is a promise that expires. The
/// before-fields stay, because the ref still holds the before-commit.
fn seal_after(ctx: &Context<'_>, entry: &mut JournalEntry, before: Oid) -> Option<AfterState> {
    let name = entry.backup_ref.clone()?;
    let mut parents = vec![before];
    for anchor in anchors(entry.head_after.as_ref(), &entry.refs_after) {
        if !parents.contains(&anchor) {
            parents.push(anchor);
        }
    }
    let label = format!("{} after: {}", kind_name(entry.kind), entry.description);
    let sealed = snapshot::commit_trees(
        ctx.repo,
        entry.index_after,
        entry.worktree_after,
        &entry.id,
        &label,
        &parents,
        &(),
    )
    .and_then(|commit| snapshot::move_backup_ref(ctx.repo, &name, commit, &()));
    match sealed {
        Ok(()) => None,
        Err(why) => {
            if entry.touches.index {
                entry.index_after = None;
            }
            if entry.touches.worktree {
                entry.worktree_after = None;
            }
            Some(AfterState::Unsealed(why))
        }
    }
}

/// Every commit an entry's `HEAD`/refs fields name.
///
/// These are ids the journal records, so keeping them reachable is the backup
/// ref's job: once the operation (or its undo) moves the ref off them, git's
/// reflog is all that holds them, and git expires that.
fn anchors(head: Option<&HeadRef>, refs: &BTreeMap<String, Option<Oid>>) -> Vec<Oid> {
    let mut found = Vec::new();
    if let Some(HeadRef::Detached(id)) = head {
        found.push(*id);
    }
    for id in refs.values().flatten() {
        if !found.contains(id) {
            found.push(*id);
        }
    }
    found
}

/// The commits an operation is about to make unreachable, and which its
/// inverse therefore needs pinned (rule 2 of the ops-model contract).
fn pins_for(op: &Operation) -> Vec<Oid> {
    match op {
        Operation::StashPop { commit, .. } | Operation::StashDrop { commit, .. } => vec![*commit],
        // The stash `stash branch` drops: undo stores it back (m5/07).
        Operation::BranchFromStash(params) => vec![params.commit],
        Operation::Merge(_)
        | Operation::Rebase(_)
        | Operation::CherryPick(_)
        | Operation::Revert(_)
        | Operation::Reset(_)
        | Operation::Checkout(_)
        | Operation::BranchCreate(_)
        | Operation::BranchDelete(_)
        | Operation::TagCreate(_)
        | Operation::TagDelete(_)
        | Operation::Stage(_)
        | Operation::Unstage(_)
        | Operation::Discard(_)
        | Operation::Commit(_)
        | Operation::Amend(_)
        | Operation::StashPush(_)
        | Operation::StashApply { .. }
        | Operation::Fetch(_)
        | Operation::Pull(_)
        | Operation::Push(_) => Vec::new(),
    }
}

/// One line for an `Aborted` entry's reason.
pub(crate) fn reason_for(why: &RunError) -> String {
    match why {
        RunError::Exited {
            stderr_first_line, ..
        } if !stderr_first_line.is_empty() => stderr_first_line.clone(),
        other => other.to_string(),
    }
}

/// The kind as it reads in a backup commit's message.
pub(crate) fn kind_name(kind: OperationKind) -> &'static str {
    match kind {
        OperationKind::Stage => "stage",
        OperationKind::Unstage => "unstage",
        OperationKind::Discard => "discard",
        OperationKind::Commit => "commit",
        OperationKind::Amend => "amend",
        OperationKind::StashPush => "stash push",
        OperationKind::StashApply => "stash apply",
        OperationKind::StashPop => "stash pop",
        OperationKind::StashDrop => "stash drop",
        OperationKind::Fetch => "fetch",
        OperationKind::Pull => "pull",
        OperationKind::Push => "push",
        OperationKind::Merge => "merge",
        OperationKind::Rebase => "rebase",
        OperationKind::CherryPick => "cherry-pick",
        OperationKind::Revert => "revert",
        OperationKind::Reset => "reset",
        OperationKind::Checkout => "checkout",
        OperationKind::BranchCreate => "branch",
        OperationKind::BranchDelete => "delete branch",
        OperationKind::TagCreate => "tag",
        OperationKind::TagDelete => "delete tag",
        OperationKind::BranchFromStash => "branch from stash",
        OperationKind::RevertTo => "revert to",
        OperationKind::Undo => "undo",
        OperationKind::Redo => "redo",
    }
}
