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
use graft_git::run::{GitInvocation, RunError, Stop};
use graft_git::version::GitVersion;

use crate::facts::{self, FactsError};
use crate::journal::{Journal, JournalError};
use crate::operation::{Invocation, Operation, Refusal, RemoteNames};
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
    let facts = facts::read_facts(ctx.repo, ctx.git, ctx.stop, op.is_remote())
        .map_err(ApplyError::Facts)?;
    op.check(&facts).map_err(ApplyError::Refused)?;
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
        None => {
            let mut unread = unread;
            unread.extend(cleanup);
            // A remote operation moved refs the entry does not record (a
            // fetch's tracking refs), so the graph is re-read regardless.
            let refresh = if entry.touches.refs.is_empty() && !op.is_remote() {
                Refresh::WorkingTree
            } else {
                Refresh::Everything
            };
            let applied = Applied { entry, refresh };
            if unread.is_empty() {
                Ok(applied)
            } else {
                // The operation itself succeeded; what failed is the record
                // of it. An `Err` the caller has to look at is the only way
                // that reaches the user, and `applied` is in it so the
                // refresh still happens.
                Err(ApplyError::AfterState {
                    applied: Box::new(applied),
                    unread,
                })
            }
        }
        // `finish` has already put the after-state failures, if any, into the
        // entry's `Aborted` reason: git's own failure is the headline, but
        // nothing is lost behind it.
        Some(failed) => Err(failed.into_error(entry)),
    }
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
        let mut git = GitInvocation::new(ctx.repo).args(invocation.args.iter().cloned());
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
    let open: Vec<(EntryId, Vec<String>, Option<String>)> = ctx
        .journal
        .unresolved()
        .into_iter()
        .map(|entry| {
            (
                entry.id.clone(),
                entry.refs_before.keys().cloned().collect(),
                entry.backup_ref.clone(),
            )
        })
        .collect();
    let mut recovered = Recovered::default();
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
        Operation::Stage(_)
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
        OperationKind::Undo => "undo",
        OperationKind::Redo => "redo",
    }
}
