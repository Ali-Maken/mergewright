//! [`Command::Operate`](crate::update::Command::Operate)'s job: the one place
//! `app` opens the undo journal, builds a [`graft_ops::apply::Context`] and
//! calls [`graft_ops::apply::apply`]. The seam rule (ARCHITECTURE.md,
//! `crates/graft/tests/it/seam_rule.rs`): `ops` decides what a write does,
//! `app` only wires the key to the call.

use std::sync::Arc;

use graft_git::remote::{RemoteFailure, RemoteProgress};
use graft_git::run::askpass::{AskPass, Secret};
use graft_git::version::system_git;
use graft_ops::apply::{self, Applied, ApplyError, Context, Refresh, RemoteHooks};
use graft_ops::journal::{Journal, journal_path};
use graft_ops::operation::Operation;

use super::Cancel;
use crate::repos::RepoStore;

/// What running one operation came back with.
///
/// Not a plain `Result`: [`ApplyError::AfterState`] means the write *did*
/// happen — journaled, on disk — and only part of its after-state could not
/// be read back. Collapsing that into an ordinary `Err(String)` would lose
/// the [`Applied`] value the caller needs to refresh with, which is exactly
/// the "operation ran, caller still refreshes" case
/// `ApplyError::AfterState`'s own doc comment calls out. `update::operate`
/// matches on this instead of a `Result` so that case cannot be dropped on
/// the floor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operated {
    /// Ran and journaled cleanly.
    Done(Applied),
    /// Ran and journaled, but part of the after-state could not be read or
    /// kept reachable. The write happened: refresh, and show `error`.
    Incomplete {
        /// What ran.
        applied: Applied,
        /// What could not be read, as one line.
        error: String,
    },
    /// Nothing was written, or nothing could be recorded — the one case
    /// with no [`Applied`] to refresh from.
    Failed {
        /// What went wrong, as one line.
        error: String,
        /// What the caller must re-read anyway, or `None` when nothing can
        /// have been written.
        ///
        /// A failure is not the same as "the screen is still true".
        /// [`ApplyError::Git`] means git ran **part-way** and the entry is
        /// journaled `Aborted` — a whole-file discard is up to three
        /// invocations, so the first can have run when the second failed.
        /// The file list, the diff bytes, the header counts and the journal
        /// are all stale after that, and the next `Space` would cut a patch
        /// from bytes that no longer describe the repository
        /// (m3/04 review M1).
        refresh: Option<Refresh>,
        /// Why a fetch, pull or push failed, as git's stderr said it
        /// ([`ApplyError::Remote`]); `None` for every other failure. `app`
        /// reads it to decide what the next `P` offers — a rejected push
        /// turns the next box into the lease variant (m4-remotes/05).
        remote: Option<RemoteFailure>,
    },
}

/// The credential answer until the in-app prompt lands (m4-remotes/06):
/// every question is cancelled, so git reports an authentication failure
/// rather than hanging or asking on a terminal Graft has taken over.
struct NoAskPass;

impl AskPass for NoAskPass {
    fn ask(&self, _prompt: &str) -> Option<Secret> {
        None
    }
}

/// Where a remote operation's progress goes: one `brief` line at a time.
pub(super) type ProgressSink = Box<dyn Fn(String) + Send + Sync>;

/// The hooks a remote operation runs with: progress to `progress`, and
/// Graft's own binary as the askpass helper. `None` for a local operation.
///
/// When Graft cannot name its own binary, the operation still runs, headless
/// — no progress, no questions — and the reason goes out as the one progress
/// line it will get, so the status line says why credentials cannot be asked
/// rather than failing later with no explanation.
fn remote_hooks(op: &Operation, progress: ProgressSink) -> Option<RemoteHooks> {
    if !op.is_remote() {
        return None;
    }
    match std::env::current_exe() {
        Ok(helper) => Some(RemoteHooks {
            progress: Box::new(move |seen: RemoteProgress| progress(seen.brief())),
            askpass: Arc::new(NoAskPass),
            helper,
        }),
        Err(error) => {
            progress(format!(
                "no askpass helper: {error} \u{2014} credentials cannot be asked"
            ));
            None
        }
    }
}

/// What a failed [`apply::apply`] leaves stale on screen.
///
/// `None` only where `ApplyError`'s own contract says nothing was written.
fn refresh_after_failure(error: &ApplyError) -> Option<Refresh> {
    match error {
        // "Nothing was written and nothing journaled" — the precondition, the
        // repository read and the backup all happen before the first write.
        ApplyError::Refused(_) | ApplyError::Facts(_) | ApplyError::Snapshot(_) => None,
        // git failed part-way. Whatever it managed is on disk; the refs it
        // moved (if any) decide how wide the re-read has to be, the same way
        // a successful `Applied` decides it.
        ApplyError::Git { entry, .. } => Some(if entry.touches.refs.is_empty() {
            Refresh::WorkingTree
        } else {
            Refresh::Everything
        }),
        // The journal failed, or the after-state could not be read
        // (`AfterState` reaches `Incomplete` first, but is named rather than
        // left to a wildcard so a new variant has to be thought about here).
        // Neither can say whether refs moved, so re-read everything rather
        // than keep a screen that may be describing a repository that has.
        // A failed fetch/pull/push may still have moved remote-tracking refs
        // (a pull fetches before its fast-forward fails): the graph too.
        ApplyError::Journal { .. } | ApplyError::AfterState { .. } | ApplyError::Remote { .. } => {
            Some(Refresh::Everything)
        }
        // Cancelled between steps, which can be after some of them ran.
        ApplyError::Stopped => Some(Refresh::WorkingTree),
    }
}

/// A failure before the first write: there is nothing on disk to re-read.
fn nothing_ran(error: &impl std::fmt::Display) -> Operated {
    Operated::Failed {
        error: error.to_string(),
        refresh: None,
        remote: None,
    }
}

/// Runs one operation against `repos`'s repository, journaled.
///
/// Opens the system `git`, the journal file and a fresh [`Context`] every
/// call — one context per operation, since a context carries the one clock
/// reading and random half that name its entry. A fetch, pull or push gets
/// [`RemoteHooks`]: its progress goes to `progress`, one brief at a time.
pub(super) fn run_operation(
    repos: &RepoStore,
    op: &Operation,
    cancel: &Cancel,
    progress: ProgressSink,
) -> Operated {
    let repo = match repos.shared() {
        Ok(repo) => repo,
        Err(error) => return nothing_ran(&error),
    };
    let handle = repo.handle();
    let git = match system_git() {
        Ok(found) => found.version(),
        Err(error) => return nothing_ran(&error),
    };
    let path = match journal_path(&handle) {
        Ok(path) => path,
        Err(error) => return nothing_ran(&error),
    };
    let mut journal = match Journal::open(&path) {
        Ok(journal) => journal,
        Err(error) => return nothing_ran(&error),
    };
    let now_unix_ms = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_millis()),
    )
    .unwrap_or(i64::MAX);
    let mut random = [0u8; 10];
    if let Err(error) = getrandom::fill(&mut random) {
        return nothing_ran(&format!("could not read random bytes: {error}"));
    }
    let mut ctx = match Context::open(&handle, git, &mut journal, now_unix_ms, random, cancel) {
        Ok(ctx) => ctx,
        Err(error) => return nothing_ran(&error),
    };
    ctx.remote = remote_hooks(op, progress);
    match apply::apply(&mut ctx, op) {
        Ok(applied) => Operated::Done(applied),
        Err(ApplyError::AfterState { applied, unread }) => Operated::Incomplete {
            applied: *applied,
            error: unread
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; "),
        },
        Err(error) => {
            let refresh = refresh_after_failure(&error);
            let remote = match &error {
                ApplyError::Remote { failure, .. } => Some(failure.clone()),
                _ => None,
            };
            Operated::Failed {
                error: error.to_string(),
                refresh,
                remote,
            }
        }
    }
}
