//! Talking to the remote (m4-remotes/05): `f` fetch, `p` pull, `P` push
//! (`docs/design/02-keymap.md` §2.3), the push box, `Esc` cancelling any
//! running write, the live progress line and the failure lines.
//!
//! `app` only builds the [`Operation`] and says what happened; `ops` checks,
//! plans, journals and runs it. The words of every failure line are `ops`'
//! (`ApplyError::Remote`'s `explain`), so the status line and the journal
//! can never describe one failure two ways.
//!
//! Here: the remote state, the facts read, and the `f`/`p` keys.
//! `push_box` is `P` and its box; `after_push` is `u` on work a push already
//! sent and the re-push; `in_flight` is a running write's `Esc`, progress
//! and ending lines.

use std::sync::Arc;

use graft_core::job::JobId;
use graft_core::oid::Oid;
use graft_ops::operation::{FetchParams, Operation, PullParams, PushParams};

use super::detail::Loading;
use super::events::Command;
use super::state::{AppState, Status};
use crate::jobs::remote::RemoteFacts;

/// Below this many columns the progress brief drops its counts.
const NARROW_COLUMNS: u16 = 100;

/// A push git refused as not a fast-forward, and the tip it tried to send.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RejectedPush {
    /// The push that was refused: `P` offers the lease only for this same
    /// branch, remote and remote branch.
    pub params: PushParams,
    /// `HEAD`'s tip when git refused the push, read from current refs.
    /// `None` when the refs were not current then: with nothing to compare,
    /// `P` offers the plain box and git decides.
    pub sent: Option<Oid>,
}

/// The remote keys' own state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RemoteState {
    /// The push git last rejected as not a fast-forward. While it names the
    /// push `P` would build, the refs are current and `HEAD` is still the
    /// tip that was refused ([`RejectedPush::sent`]), `P` offers the lease
    /// box instead of the plain one (m6-conflicts/49). Cleared by a
    /// successful push and by a change of branch.
    pub last_rejected_push: Option<RejectedPush>,
    /// The remote facts read in flight. Held apart from
    /// [`AppState::remotes`] so a re-read does not blank the facts `p` and
    /// `P` decide from; an answer under any other id is stale and dropped.
    pub reading: Option<JobId>,
    /// The write `AppState::operating` names, in words: what `Esc` says it
    /// is cancelling, and — for a fetch, pull or push — the operation, so
    /// its answer can be worded and a rejection remembered. Every write
    /// sets it (`operate::launch`, `undo`'s launch); it is read only under
    /// the id it was set with.
    pub running: Option<Running>,
    /// The write `Esc` asked to stop. Its `JobCancelled` is the answer the
    /// user asked for, not a failure.
    pub cancelling: Option<JobId>,
    /// The upstream a `p` just refused to fast-forward onto — its full
    /// tracking ref and the tip it was at — so the `m` and `r` the refusal
    /// offers aim at *that* branch rather than at whatever row the cursor
    /// happens to be on (m5-history-and-ghost-preview/08, piece 9).
    ///
    /// Set only when the tip is known: without it there is nothing to pin
    /// an `Operation` to, and a key hint that cannot be honoured is worse
    /// than none. Cleared by dismissing the line and by any write
    /// launching, each of which makes the recorded tip a guess.
    ///
    /// The tip is **not** kept as it was first recorded. The pull's own
    /// fetch moves the tracking ref after the params were built, so
    /// [`forget_stale_divergence`] re-reads it from the refs at the tail of
    /// every `update` for as long as the line is showing (m5/32).
    pub diverged_upstream: Option<(String, Oid)>,
    /// The push whose undo-after-push box has already been answered
    /// (ADR-0038 §4).
    ///
    /// Keyed on the **push**, not on the entry `u` would take back. The box
    /// explains one fact — "this work is already on the remote" — and that
    /// fact belongs to the push. Keying it on the undo target instead would
    /// re-ask for every entry below the push, which is the nagging the
    /// design note's "asked once" rules out; a *new* push is a new fact and
    /// asks again. Set only when a choice was made, so `[ Cancel ]` asks
    /// again.
    pub undo_after_push_asked: Option<graft_core::journal::EntryId>,
}

/// One write in flight.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Running {
    /// `AppState::operating`'s id.
    pub id: JobId,
    /// `Operation::describe`, or `undo`/`redo`.
    pub description: String,
    /// The operation, when it talks to a remote.
    pub op: Option<Operation>,
    /// Whether `Esc` may stop this write.
    pub stoppable: Stoppable,
    /// The key that drew the Ghost Preview this write was confirmed from
    /// (`m`, `r`, `.`), so a `Moved` refusal at confirm time can say which
    /// key to press again. `None` for every write that is not a previewed
    /// history operation (m5-history-and-ghost-preview/08, piece 8).
    pub opened_by: Option<&'static str>,
}

/// Whether `Esc` may stop a running write.
///
/// Not every write can be stopped safely. Stopping one means killing the git
/// child it is inside (`graft_git::run`), and whether that is safe depends on
/// what the layer below does with the steps that already ran.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stoppable {
    /// `Esc` sends `Command::CancelJob`. An operation is one git write, so
    /// killing it leaves the repository either written or not.
    Yes,
    /// `Esc` refuses and says so, in these words.
    No(&'static str),
}

impl RemoteState {
    /// The running write, if `id` is it.
    fn running(&self, id: JobId) -> Option<&Running> {
        self.running.as_ref().filter(|running| running.id == id)
    }
}

/// `refs/heads/main` → `main`.
fn short_ref(full: &str) -> &str {
    full.rsplit_once('/').map_or(full, |(_, last)| last)
}

/// Note the write just launched under `id` (see [`RemoteState::running`]).
pub(super) fn launched(
    state: &mut AppState,
    id: JobId,
    description: String,
    op: Option<Operation>,
    stoppable: Stoppable,
    opened_by: Option<&'static str>,
) {
    state.remote.running = Some(Running {
        id,
        description,
        op,
        stoppable,
        opened_by,
    });
    // Whatever this write is, the tip the diverged line remembered is now a
    // guess: the write may be the merge or the rebase that answers it, or
    // something else that moves the same refs.
    state.remote.diverged_upstream = None;
}

/// The keys the diverged-pull line offers, and the mark that tells
/// [`forget_stale_divergence`] the line is still the one on screen.
pub const DIVERGED_KEYS: &str = "m merge it  r rebase onto it";

/// The remembered upstream outlives its words otherwise, and its tip goes
/// stale the moment the refs move.
///
/// `m` and `r` aim at [`RemoteState::diverged_upstream`] instead of at the
/// row under the cursor, which is right only while the line that explained
/// the divergence is on screen. `Esc` clears both; every other way the line
/// goes — a job's `Done`, a later refusal — used to leave the tip behind,
/// so the next `m` silently merged the upstream from wherever the cursor
/// happened to be (review 08 Minor 9). Run at the tail of every `update`.
///
/// While the line **is** showing, the tip is re-read from the refs the app
/// now holds. The record is written by [`remote_failed`] from the params
/// the `p` was built with — taken **before the pull's own fetch**, which is
/// the very thing that moved the tracking ref — so a recorded tip is stale
/// from the instant it is written, and the first `r` was refused with
/// `main moved since the preview` (m5/32, finding 2). Re-reading here makes
/// the tip a projection of the refs rather than a snapshot beside them:
/// `history_operation` already takes the current branch from those same
/// refs, so both halves of the pinned pair now come from one reading.
///
/// A tracking ref the refs do not list keeps what it had:
/// [`crate::refs::RefBadges::remotes`] leaves out a remote level a local
/// branch tracks at the same commit, and guessing would be worse than the
/// `Moved` refusal `tip_holds` already gives.
pub(super) fn forget_stale_divergence(state: &mut AppState) {
    let showing = matches!(
        &state.status,
        Status::Sticky { keys: Some(keys), .. } if keys == DIVERGED_KEYS
    );
    if !showing {
        state.remote.diverged_upstream = None;
        return;
    }
    let Some((tracking, tip)) = state.remote.diverged_upstream.as_mut() else {
        return;
    };
    if let Some(now) = state
        .refs
        .as_deref()
        .and_then(|refs| tracking_tip(refs, tracking))
    {
        *tip = now;
    }
}

/// Where the refs the app holds put the tracking ref named by its full name
/// (`refs/remotes/origin/main`); `None` when they carry no badge for it.
fn tracking_tip(refs: &crate::refs::RefBadges, tracking: &str) -> Option<Oid> {
    let short = short_tracking(tracking).as_bytes();
    refs.remotes()
        .find(|(_, name)| *name == short)
        .map(|(oid, _)| oid)
}

/// The key hint a running remote write's status line ends with. The footer
/// draws a line ending in it as it is, without the `…` it gives every other
/// busy line: the hint is not the part still in progress (design note, New
/// UI, "Footer while a remote write runs").
pub const ESC_HINT: &str = " · esc cancels";

/// What the status line says while `op` runs. A remote write says `Esc`
/// stops it: it is the one kind that can hang (design note, choice 4).
pub(super) fn busy_text(op: &Operation) -> String {
    if op.is_remote() {
        format!("{}{ESC_HINT}", op.describe())
    } else {
        op.describe()
    }
}

/// Ask for the remote facts again, keeping the last answer on screen until
/// the new one lands.
pub(super) fn request(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let id = next_id();
    state.remote.reading = Some(id);
    if !matches!(state.remotes, Loading::Ready(_)) {
        state.remotes = Loading::Pending(id);
    }
    commands.push(Command::ReadRemotes(id));
}

/// The launch read: asks once the repository is open, and never again on
/// its own — `Ctrl+R` and every write that refreshes everything ask through
/// [`request`], beside the refs read. Called at the tail of every `update`
/// (`super::settle`), like the journal's.
pub(super) fn request_at_launch(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    if state.repository.is_some() && state.remotes == Loading::Idle {
        request(state, commands, next_id);
    }
}

/// The remote facts arrived. A stale answer — not the read in flight — is
/// dropped. A branch change forgets the last rejected push: the lease it
/// would offer is about another branch.
pub(super) fn remotes_arrived(
    state: &mut AppState,
    id: JobId,
    result: Result<Arc<RemoteFacts>, String>,
) {
    if state.remote.reading != Some(id) {
        return;
    }
    state.remote.reading = None;
    match result {
        Ok(facts) => {
            if state
                .remote
                .last_rejected_push
                .as_ref()
                .is_some_and(|rejected| {
                    facts.head_branch.as_deref() != Some(&rejected.params.branch)
                })
            {
                state.remote.last_rejected_push = None;
            }
            state.remotes = Loading::Ready(facts);
        }
        // Kept, not shown: `p` and `P` say it when pressed, which is the
        // only moment it matters.
        Err(why) => state.remotes = Loading::Failed(why),
    }
}

/// `p`, `P` or `f` before the remote facts have landed; `P` also while a
/// re-read is in flight and the answer would be the lease.
const REMOTES_PENDING: &str = "still reading the remotes — try again in a moment";

/// The facts, or the words for why `p`/`P` cannot act yet.
fn facts(state: &AppState) -> Result<&Arc<RemoteFacts>, String> {
    match &state.remotes {
        Loading::Ready(facts) => Ok(facts),
        Loading::Failed(why) => Err(format!("remotes could not be read: {why}")),
        Loading::Idle | Loading::Pending(_) => Err(REMOTES_PENDING.to_owned()),
    }
}

/// `f`: fetch every remote, pruning. Refused in words while a write runs
/// (gap row 481): it used to answer the key with nothing at all.
pub fn fetch_key(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    if super::operate::operation_in_flight(state) {
        state.status = Status::Failed(wait_for_running(state));
        return true;
    }
    let op = Operation::Fetch(FetchParams {
        remote: None,
        prune: true,
    });
    super::operate::launch(state, commands, op, next_id);
    true
}

/// The refusal for a write key pressed while another write runs: `wait for
/// <what is running> to finish`, the Ghost Preview's own words for it.
fn wait_for_running(state: &AppState) -> String {
    let running = state
        .remote
        .running
        .as_ref()
        .map_or("the write", |it| it.description.as_str());
    format!("wait for {running} to finish")
}

/// `p`: fast-forward `HEAD`'s branch from its upstream. Refused in words,
/// with no command, when there is no branch or no upstream.
pub fn pull_key(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    if super::operate::operation_in_flight(state) {
        return true;
    }
    let params = match pull_params(state) {
        Ok(params) => params,
        Err(why) => {
            state.status = Status::Failed(why);
            return true;
        }
    };
    super::operate::launch(state, commands, Operation::Pull(params), next_id);
    true
}

/// What `p` would pull, or why it cannot.
fn pull_params(state: &AppState) -> Result<PullParams, String> {
    let facts = facts(state)?;
    let Some(branch) = &facts.head_branch else {
        return Err("HEAD is detached — check out a branch first".to_owned());
    };
    let Some(upstream) = &facts.head_upstream else {
        return Err(format!(
            "{} has no upstream — P pushes and sets one",
            short_branch(branch)
        ));
    };
    Ok(PullParams {
        branch: branch.clone(),
        upstream: upstream.clone(),
    })
}

/// `refs/heads/main` → `main`.
fn short_branch(name: &str) -> &str {
    name.strip_prefix("refs/heads/").unwrap_or(name)
}

/// `refs/remotes/origin/main` → `origin/main`.
pub(super) fn short_tracking(name: &str) -> &str {
    name.strip_prefix("refs/remotes/").unwrap_or(name)
}

/// Seven hex digits, as the design note writes them.
fn short_oid(oid: Oid) -> String {
    let mut hex = oid.to_string();
    hex.truncate(7);
    hex
}

mod after_push;
mod in_flight;
mod push_box;

pub(crate) use after_push::pushed_to;
pub use after_push::repush_lease;
pub(super) use after_push::{pushed_since, repush_chosen, undo_after_push, undo_after_push_box};
pub use in_flight::{cancel_operating, remote_failed, remote_progress};
pub(super) use in_flight::{cancelled, remote_done, stop_write};
pub use push_box::{push_box, push_key, push_params};

#[cfg(test)]
mod tests;
