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

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use graft_core::journal::{EntryId, EntryStatus, JournalEntry, OperationKind, Touches};
use graft_core::oid::Oid;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitInvocation, RunError, Stop};
use graft_git::version::GitVersion;

use crate::facts::{self, FactsError};
use crate::journal::{Journal, JournalError};
use crate::operation::{Invocation, Operation, Refusal};
use crate::snapshot::{self, SnapshotError, SnapshotScope};

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
    /// The journal could not be written. The operation did not run: a write
    /// Graft cannot record is a write it must not make.
    #[error("{0}")]
    Journal(JournalError),
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
                let what = match error {
                    RunError::Exited {
                        stderr_first_line, ..
                    } if !stderr_first_line.is_empty() => stderr_first_line.clone(),
                    other => other.to_string(),
                };
                let mut line = format!("{}: {what}", entry.description);
                if *recoverable {
                    line.push_str(" · u restores the backup");
                }
                line
            }
            other => other.to_string(),
        }
    }
}

impl From<PrepareError> for ApplyError {
    fn from(why: PrepareError) -> Self {
        match why {
            PrepareError::Facts(why) => ApplyError::Facts(why),
            PrepareError::Snapshot(why) => ApplyError::Snapshot(why),
            PrepareError::Journal(why) => ApplyError::Journal(why),
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
                ApplyError::Journal(JournalError::Io(std::io::Error::other(
                    "the journal has no directory to keep scratch in",
                )))
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

    fn scratch(&self) -> &Path {
        &self.scratch
    }
}

/// Run one operation, journaled.
pub fn apply(ctx: &mut Context<'_>, op: &Operation) -> Result<Applied, ApplyError> {
    if ctx.stop.should_stop() {
        return Err(ApplyError::Stopped);
    }
    let facts = facts::read_facts(ctx.repo, ctx.git, ctx.stop).map_err(ApplyError::Facts)?;
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

    let mut failure = None;
    for invocation in &plan {
        let mut git = GitInvocation::new(ctx.repo).args(invocation.args.iter().cloned());
        if let Some(bytes) = &invocation.stdin {
            git = git.stdin(bytes.clone());
        }
        if let Err(why) = git.run(ctx.stop) {
            failure = Some(why);
            break;
        }
    }

    let status = match &failure {
        None => EntryStatus::Committed,
        Some(why) => EntryStatus::Aborted {
            reason: reason_for(why),
        },
    };
    finish(ctx, &mut pending, status, commands).map_err(ApplyError::Journal)?;

    let entry = pending.entry;
    match failure {
        None => {
            let refresh = if entry.touches.refs.is_empty() {
                Refresh::WorkingTree
            } else {
                Refresh::Everything
            };
            Ok(Applied { entry, refresh })
        }
        Some(error) => {
            let recoverable = entry.worktree_before.is_some();
            Err(ApplyError::Git {
                entry: Box::new(entry),
                error,
                recoverable,
            })
        }
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
/// Returns the ids it closed, so the caller can surface them.
pub fn recover_interrupted(ctx: &mut Context<'_>) -> Result<Vec<EntryId>, ApplyError> {
    let open: Vec<(EntryId, Vec<String>)> = ctx
        .journal
        .unresolved()
        .into_iter()
        .map(|entry| {
            (
                entry.id.clone(),
                entry.refs_before.keys().cloned().collect(),
            )
        })
        .collect();
    let mut closed = Vec::new();
    for (id, refs) in open {
        let now =
            facts::read_state_now(ctx.repo, ctx.scratch(), &refs, ctx.stop).map_err(
                |why| match why {
                    FactsError::Stopped => ApplyError::Stopped,
                    other => ApplyError::Facts(other),
                },
            );
        let now = match now {
            Ok(now) => Some(now),
            // The repository is unreadable, and the entry still has to be
            // closed: an entry left `Pending` for ever blocks `u` outright.
            // `None` is the honest record and the widest restore, and the
            // user is asked before it runs (`Drift::unknown`).
            Err(ApplyError::Stopped) => return Err(ApplyError::Stopped),
            Err(_unreadable) => None,
        };
        ctx.journal
            .mark_interrupted(&id, now.as_ref())
            .map_err(ApplyError::Journal)?;
        closed.push(id);
    }
    Ok(closed)
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
    let (backup, backup_ref) = if wanted {
        let label = format!("{}: {description}", kind_name(kind));
        let taken = snapshot::take(
            ctx.repo,
            ctx.scratch(),
            &id,
            &ctx.worktree,
            &label,
            &pins,
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

/// Read the after-state and write the outcome line.
///
/// Every after-field the repository will give up is recorded, on abort as
/// much as on success: `refs_after` for every key of `refs_before`,
/// `head_after`, and each tree the entry touches. A field that cannot be read
/// is recorded as `None` rather than propagated — the outcome line has to be
/// written either way, since an entry with no outcome reads as an operation
/// still in flight, which this one is not. `None` is not a swallowed error:
/// `drift` reports it as unknown, the inverse widens to the whole backup
/// tree, and the user is asked before that runs. It costs a confirm box,
/// never the work itself.
pub(crate) fn finish(
    ctx: &mut Context<'_>,
    pending: &mut Pending,
    status: EntryStatus,
    commands: Vec<String>,
) -> Result<(), JournalError> {
    let entry = &mut pending.entry;
    entry.status = status;
    entry.commands = commands;
    entry.head_after = facts::head_ref(ctx.repo).ok();
    entry.refs_after = facts::read_refs(ctx.repo, &entry.touches.refs).unwrap_or_default();
    entry.index_after = if entry.touches.index {
        snapshot::index_tree(ctx.repo, &ctx.scratch, ctx.stop).ok()
    } else {
        entry.index_before
    };
    entry.worktree_after = if entry.touches.worktree {
        snapshot::worktree_tree(ctx.repo, &ctx.scratch, ctx.stop).ok()
    } else {
        entry.worktree_before
    };

    // A worktree-touching operation's after-state needs a backup of its own,
    // or redo would have nothing to restore from. It hangs off the
    // before-commit, so one ref keeps both ends (and the pins) reachable.
    if let (true, Some(name), Some(before), Some(index_after), Some(worktree_after)) = (
        entry.touches.worktree,
        entry.backup_ref.as_deref(),
        pending.before_commit,
        entry.index_after,
        entry.worktree_after,
    ) {
        let label = format!("{} after: {}", kind_name(entry.kind), entry.description);
        let sealed = snapshot::commit_trees(
            ctx.repo,
            Some(index_after),
            worktree_after,
            &entry.id,
            &label,
            &[before],
            ctx.stop,
        )
        .and_then(|commit| {
            snapshot::move_backup_ref(ctx.repo, name, commit, ctx.stop).map(|()| commit)
        });
        // An after-commit that could not be made leaves the ref on the
        // before-commit: the restore `u` needs still works, only redo's
        // does not, and the entry says so by carrying no after tree that
        // any ref reaches.
        if sealed.is_err() {
            entry.worktree_after = None;
        }
    }

    ctx.journal.append_outcome(entry)
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
        | Operation::StashApply { .. } => Vec::new(),
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
        OperationKind::Undo => "undo",
        OperationKind::Redo => "redo",
    }
}
