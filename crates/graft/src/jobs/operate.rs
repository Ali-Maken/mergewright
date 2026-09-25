//! [`Command::Operate`](crate::update::Command::Operate)'s job: the one place
//! `app` opens the undo journal, builds a [`graft_ops::apply::Context`] and
//! calls [`graft_ops::apply::apply`]. The seam rule (ARCHITECTURE.md,
//! `crates/graft/tests/it/seam_rule.rs`): `ops` decides what a write does,
//! `app` only wires the key to the call.

use std::path::PathBuf;
use std::sync::Arc;

use graft_git::remote::{RemoteFailure, RemoteProgress};
use graft_git::run::askpass::AskPass;
use graft_git::state::InProgress;
use graft_git::version::system_git;
use graft_ops::apply::{self, Applied, ApplyError, Context, Refresh, RemoteHooks};
use graft_ops::journal::{Journal, journal_path};
use graft_ops::operation::Operation;

use super::Cancel;
use crate::repos::RepoStore;
use crate::session::Clock;

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
    /// Git stopped part-way through a history operation and left its own
    /// state live — `MERGE_HEAD`, `rebase-merge/`, `CHERRY_PICK_HEAD`,
    /// `REVERT_HEAD`.
    ///
    /// **Not a failure, and not `Done`.** The journal entry stays `Pending`
    /// with no outcome line (`.scratch/m5-history-and-ghost-preview/as-built.md`
    /// §1), git's state is on disk, and the user is *in* the operation
    /// until it is finished or aborted. Reporting it as
    /// [`Operated::Failed`] would word a state the user must act on as
    /// something that went wrong, and would drop the two facts they need to
    /// act: which operation is live and which files are unmerged.
    ///
    /// Refreshing is always `Everything` — refs and the working tree have
    /// both moved — which is what puts the unmerged paths under `CONFLICTS`
    /// in the changes pane and `AppState::in_progress` behind the commit
    /// editor's refusal.
    Conflicted {
        /// What was being done, in the journal's own words.
        description: String,
        /// The state git left: what `graft_ops::apply::resume` runs, and
        /// what the commit editor's refusal already names.
        kind: InProgress,
        /// The unmerged paths, as `apply` read them out of `status`. Can be
        /// empty: a rebase can stop with nothing unmerged (an `edit` step,
        /// an empty commit).
        paths: Vec<PathBuf>,
        /// Git's own words when this came from a
        /// [`ApplyError::ResumeFailed`] — a `--continue` a hook refused,
        /// an empty pick — where the state is live but no unmerged path
        /// explains it. `None` when the operation itself stopped on a
        /// conflict.
        ///
        /// The two share a variant because they are one state to the user
        /// ("git stopped and its state is still here"), and the one fact
        /// that tells them apart is carried rather than dropped.
        stalled: Option<String>,
        /// The journal entry git is part-way through, still `Pending` —
        /// the live entry, named by the answer itself (gap row 505).
        entry: graft_core::journal::EntryId,
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
        /// The precondition that said no ([`ApplyError::Refused`]); `None`
        /// for every other failure. Carried as the value rather than as its
        /// sentence because the two history refusals need a *key hint*
        /// beside the reason — `m again` after a tip moved, `S stash …`
        /// over a dirty tree — and a hint cannot be recovered from a string
        /// (m5-history-and-ghost-preview/08, piece 8).
        refused: Option<graft_ops::operation::Refusal>,
    },
}

/// Where a remote operation's progress goes: one `brief` line at a time.
pub(super) type ProgressSink = Box<dyn Fn(String) + Send + Sync>;

/// What a remote operation needs from the app: where its progress goes, who
/// answers git's credential questions, and which binary git runs as the
/// askpass helper.
pub(super) struct Hooks {
    /// One `RemoteProgress::brief` line at a time.
    pub progress: ProgressSink,
    /// The user's prompt (`jobs::askpass::ChannelAskPass`).
    pub askpass: Arc<dyn AskPass>,
    /// Graft's own binary, or why it could not be named.
    pub helper: Result<PathBuf, String>,
}

/// The hooks a remote operation runs with. `None` for a local operation.
///
/// When Graft cannot name the binary git must run to ask, the operation
/// still runs, headless — no progress, no questions — and the reason goes
/// out as the one progress line it will get, so the status line says why
/// credentials cannot be asked rather than failing later with no
/// explanation.
fn remote_hooks(op: &Operation, hooks: Hooks) -> Option<RemoteHooks> {
    if !op.is_remote() {
        return None;
    }
    let Hooks {
        progress,
        askpass,
        helper,
    } = hooks;
    match helper {
        Ok(helper) => Some(RemoteHooks {
            progress: Box::new(move |seen: RemoteProgress| progress(seen.brief())),
            askpass,
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
        // moved (if any) and `HEAD` decide how wide the re-read has to be,
        // as `graft_ops::restore::refresh_after` decides it. Refs are not
        // `HEAD`: a checkout names none, and `git switch` moves `HEAD` before
        // a failing `post-checkout` hook fails it (m6-conflicts/80, review
        // m10). `HEAD` unread counts as moved.
        ApplyError::Git { entry, .. } => Some(
            if entry.touches.refs.is_empty()
                && entry.head_after.as_ref() == Some(&entry.head_before)
            {
                Refresh::WorkingTree
            } else {
                Refresh::Everything
            },
        ),
        // The journal failed, or the after-state could not be read
        // (`AfterState` reaches `Incomplete` first, but is named rather than
        // left to a wildcard so a new variant has to be thought about here).
        // Neither can say whether refs moved, so re-read everything rather
        // than keep a screen that may be describing a repository that has.
        // A failed fetch/pull/push may still have moved remote-tracking refs
        // (a pull fetches before its fast-forward fails): the graph too.
        //
        // `Conflicted` and `ResumeFailed` are neither of those and are not
        // failures at all — `run_operation` takes both into
        // [`Operated::Conflicted`] before this is reached. They are named
        // here rather than left to a wildcard so a third "git stopped and
        // its state is live" variant has to be thought about, and they share
        // this answer because it is the same one: git wrote refs and files
        // before it stopped.
        ApplyError::Journal { .. }
        | ApplyError::AfterState { .. }
        | ApplyError::Remote { .. }
        | ApplyError::Conflicted { .. }
        | ApplyError::ResumeFailed { .. }
        // Interim (m6-conflicts/72; 72b replaces it with the landing): a
        // stash put back that clashed reaches the app as a failure whose
        // status is `ApplyError::Clashed`'s own line, sticky —
        // `<description> left conflicts in N files` — and nothing else. The
        // refresh is what `Git` gave the same pop before 72 (it touches
        // `refs/stash`): everything.
        | ApplyError::Clashed { .. } => Some(Refresh::Everything),
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
        // `nothing_ran` is the pool's own failure to even open the
        // repository, never a precondition `graft_ops` weighed.
        refused: None,
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
    clock: Clock,
    cancel: &Cancel,
    hooks: Hooks,
) -> Operated {
    in_context(repos, clock, cancel, |ctx| {
        ctx.remote = remote_hooks(op, hooks);
        apply::apply(ctx, op)
    })
}

/// Carries the live merge/rebase/cherry-pick/revert forward, skips its step
/// or aborts it (`c`/`s`/`a` on the operation banner,
/// m5-history-and-ghost-preview/09).
///
/// No [`Hooks`]: there is no remote to report progress from, and `resume`
/// runs one `git <kind> --continue|--skip|--abort` rather than a plan with
/// steps to narrate.
///
/// **Writes no journal entry of its own.** The operation's entry has been
/// `Pending` since git stopped; `graft_ops::apply::resume` writes that
/// entry's outcome line, which is what closes the span
/// (`.scratch/m5-history-and-ghost-preview/as-built.md` §1).
pub(super) fn run_resume(
    repos: &RepoStore,
    what: apply::Resume,
    clock: Clock,
    cancel: &Cancel,
) -> Operated {
    in_context(repos, clock, cancel, |ctx| apply::resume(ctx, what))
}

/// The setup both of the above share: the repository, the system `git`, the
/// journal file and one [`Context`] — one per call, since a context carries
/// the single clock reading and random half that name its entry.
fn in_context(
    repos: &RepoStore,
    clock: Clock,
    cancel: &Cancel,
    run: impl FnOnce(&mut Context<'_>) -> Result<Applied, ApplyError>,
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
    // Through the `Clock`, not `SystemTime::now()`: a capture of a
    // repository the app writes mid-capture would otherwise draw a
    // different instant on every run (`crate::session::Clock`).
    let now_unix_ms = clock.now_millis();
    let mut random = [0u8; 10];
    if let Err(error) = getrandom::fill(&mut random) {
        return nothing_ran(&format!("could not read random bytes: {error}"));
    }
    let mut ctx = match Context::open(&handle, git, &mut journal, now_unix_ms, random, cancel) {
        Ok(ctx) => ctx,
        Err(error) => return nothing_ran(&error),
    };
    answer(run(&mut ctx))
}

/// One [`apply::apply`] result, as the caller reads it.
///
/// Split out of [`run_operation`] so the mapping is one readable match
/// rather than a tail on the end of the setup.
fn answer(result: Result<Applied, ApplyError>) -> Operated {
    match result {
        Ok(applied) => Operated::Done(applied),
        Err(ApplyError::AfterState { applied, unread }) => Operated::Incomplete {
            applied: *applied,
            error: unread
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; "),
        },
        // git stopped and left its state live. Not a failure: see
        // [`Operated::Conflicted`].
        Err(ApplyError::Conflicted { entry, kind, paths }) => Operated::Conflicted {
            description: entry.description.clone(),
            kind,
            paths,
            stalled: None,
            entry: entry.id,
        },
        Err(ApplyError::ResumeFailed { entry, kind, error }) => Operated::Conflicted {
            description: entry.description.clone(),
            kind,
            // Nothing is unmerged — that is what makes this case different,
            // and why git's own line is the only thing that can say what to
            // do about it.
            paths: Vec::new(),
            stalled: Some(error.to_string()),
            entry: entry.id,
        },
        Err(error) => {
            let refresh = refresh_after_failure(&error);
            let remote = match &error {
                ApplyError::Remote { failure, .. } => Some(failure.clone()),
                _ => None,
            };
            let refused = match &error {
                ApplyError::Refused(refusal) => Some(refusal.clone()),
                _ => None,
            };
            Operated::Failed {
                error: error.to_string(),
                refresh,
                remote,
                refused,
            }
        }
    }
}

#[cfg(test)]
mod tests;
