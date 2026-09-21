//! Talking to the remote (m4-remotes/05): `f` fetch, `p` pull, `P` push
//! (`docs/design/02-keymap.md` §2.3), the push box, `Esc` cancelling any
//! running write, the live progress line and the failure lines.
//!
//! `app` only builds the [`Operation`] and says what happened; `ops` checks,
//! plans, journals and runs it. The words of every failure line are `ops`'
//! (`ApplyError::Remote`'s `explain`), so the status line and the journal
//! can never describe one failure two ways.

use std::sync::Arc;

use graft_core::job::JobId;
use graft_core::journal::{EntryStatus, JournalEntry, OperationKind};
use graft_core::oid::Oid;
use graft_git::remote::RemoteFailure;
use graft_ops::operation::{FetchParams, Lease, Operation, PullParams, PushParams};
use graft_term::sanitize_line;
use graft_ui::confirm::ConfirmView;

use super::confirm::{ConfirmThen, PendingConfirm};
use super::detail::Loading;
use super::events::Command;
use super::state::{AppState, Status};
use crate::jobs::remote::RemoteFacts;
use crate::refs::Upstream;

/// Below this many columns the progress brief drops its counts.
const NARROW_COLUMNS: u16 = 100;

/// The remote keys' own state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RemoteState {
    /// The push git last rejected as not a fast-forward. While it names the
    /// branch `P` would push, `P` offers the lease box instead of the plain
    /// one. Cleared by a successful push and by a change of branch.
    pub last_rejected_push: Option<PushParams>,
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
    /// than none. Cleared by dismissing the line, by any write launching,
    /// and by a refresh — each of which makes the recorded tip a guess.
    pub diverged_upstream: Option<(String, Oid)>,
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

/// The remembered upstream tip outlives its words otherwise.
///
/// `m` and `r` aim at [`RemoteState::diverged_upstream`] instead of at the
/// row under the cursor, which is right only while the line that explained
/// the divergence is on screen. `Esc` clears both; every other way the line
/// goes — a job's `Done`, a later refusal — used to leave the tip behind,
/// so the next `m` silently merged the upstream from wherever the cursor
/// happened to be (review 08 Minor 9). Run at the tail of every `update`.
pub(super) fn forget_stale_divergence(state: &mut AppState) {
    let showing = matches!(
        &state.status,
        Status::Sticky { keys: Some(keys), .. } if keys == DIVERGED_KEYS
    );
    if !showing {
        state.remote.diverged_upstream = None;
    }
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
                .is_some_and(|rejected| facts.head_branch.as_deref() != Some(&rejected.branch))
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

/// The facts, or the words for why `p`/`P` cannot act yet.
fn facts(state: &AppState) -> Result<&Arc<RemoteFacts>, String> {
    match &state.remotes {
        Loading::Ready(facts) => Ok(facts),
        Loading::Failed(why) => Err(format!("remotes could not be read: {why}")),
        Loading::Idle | Loading::Pending(_) => {
            Err("still reading the remotes — try again in a moment".to_owned())
        }
    }
}

/// `f`: fetch every remote, pruning. A no-op while a write runs.
pub fn fetch_key(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    if super::operate::operation_in_flight(state) {
        return true;
    }
    let op = Operation::Fetch(FetchParams {
        remote: None,
        prune: true,
    });
    super::operate::launch(state, commands, op, next_id);
    true
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

/// `P`: open the push box — the lease variant after a rejection. The box's
/// `Enter` dispatches; nothing is pushed from here.
pub fn push_key(state: &mut AppState) -> bool {
    if super::operate::operation_in_flight(state) {
        return true;
    }
    match push_params(state) {
        Ok(params) => {
            let ahead = head_ahead(state);
            let local_tip = state.refs.as_deref().and_then(crate::refs::RefBadges::head);
            let view = push_box(&params, ahead, local_tip);
            state.confirm = Some(PendingConfirm {
                view,
                then: ConfirmThen::Operate(Operation::Push(params)),
            });
        }
        Err(why) => state.status = Status::Failed(why),
    }
    true
}

/// What `P` would push, or why it cannot. Pure: the table the tests drive.
///
/// The upstream's remote and branch when there is one; otherwise `origin`
/// (or the only remote) under the same name, setting the upstream. After a
/// non-fast-forward rejection of this same push, the lease variant, guarded
/// by the tracking tip the user last fetched.
///
/// # Errors
/// The status line's words: no remote, detached `HEAD`, several remotes and
/// none to choose by, nothing to lease against, or facts not read yet.
pub fn push_params(state: &AppState) -> Result<PushParams, String> {
    let facts = facts(state)?;
    if facts.remotes.is_empty() {
        return Err("no remote — add one in the shell: git remote add origin <url>".to_owned());
    }
    let Some(branch) = &facts.head_branch else {
        return Err("HEAD is detached — check out a branch first".to_owned());
    };
    let mut params = if let Some(upstream) = &facts.head_upstream {
        PushParams {
            branch: branch.clone(),
            remote: upstream.remote.clone(),
            remote_branch: upstream.remote_branch.clone(),
            set_upstream: false,
            lease: None,
            // `P` pushes the branch as it stands. Re-pushing an *old* tip
            // after `u` took a push back is a different offer with its own
            // confirm (ADR-0038 §4, the M5 note's "`u` after a push"), and
            // it is not built yet — so this is `None`, never a quiet reuse
            // of whatever commit happens to be selected.
            source_commit: None,
        }
    } else {
        let remote = match facts.remotes.iter().find(|remote| remote.name == "origin") {
            Some(origin) => origin,
            None if facts.remotes.len() == 1 => &facts.remotes[0],
            None => {
                return Err(format!(
                    "several remotes and no upstream — set one: git push -u <remote> {}",
                    short_branch(branch)
                ));
            }
        };
        PushParams {
            branch: branch.clone(),
            remote: remote.name.clone(),
            remote_branch: branch.clone(),
            set_upstream: true,
            lease: None,
            // A first push of a branch with no upstream: the branch tip is
            // the only thing there is to push. See the arm above.
            source_commit: None,
        }
    };
    let rejected = state
        .remote
        .last_rejected_push
        .as_ref()
        .is_some_and(|last| {
            last.branch == params.branch
                && last.remote == params.remote
                && last.remote_branch == params.remote_branch
        });
    if rejected {
        let Some(expected) = facts
            .head_upstream
            .as_ref()
            .and_then(|upstream| upstream.tracking_tip)
        else {
            return Err("f to fetch first — nothing to lease against".to_owned());
        };
        params.lease = Some(Lease { expected });
    }
    Ok(params)
}

/// How many commits `HEAD`'s branch has that its upstream lacks, when the
/// refs say (`Upstream::Apart`); `None` when they do not.
fn head_ahead(state: &AppState) -> Option<usize> {
    let refs = state.refs.as_deref()?;
    refs.branches()
        .find(|&(_, _, current, _)| current)
        .and_then(|(_, _, _, upstream)| match upstream {
            Upstream::Apart(counts) => Some(counts.ahead),
            Upstream::None | Upstream::InSync { .. } => None,
        })
}

/// The push box, in the design note's words (New UI, "Push box"). Pure.
///
/// `ahead` is how many commits go, when known; `local_tip` is what the
/// lease variant says replaces the remote's tip.
#[must_use]
pub fn push_box(params: &PushParams, ahead: Option<usize>, local_tip: Option<Oid>) -> ConfirmView {
    let local = short_branch(&params.branch);
    let remote_side = format!("{}/{}", params.remote, short_branch(&params.remote_branch));
    let command = Operation::Push(params.clone())
        .preview()
        .map_or_else(String::new, |lines| lines.join("\n"));
    match params.lease {
        None => {
            let mut consequence = Vec::new();
            if let Some(count) = ahead {
                consequence.push(if count == 1 {
                    "1 commit".to_owned()
                } else {
                    format!("{count} commits")
                });
            }
            if params.set_upstream {
                consequence.push("sets upstream".to_owned());
            }
            consequence.push(format!(
                "can't be undone from here — {} keeps what you send",
                params.remote
            ));
            ConfirmView::new(
                sanitize_line(&format!("Push {local} → {remote_side}")),
                vec![sanitize_line(&consequence.join(" · "))],
                sanitize_line(&command),
            )
            .with_labels("Cancel", "Push")
        }
        Some(Lease { expected }) => {
            let theirs = short_oid(expected);
            let ours = local_tip.map_or_else(|| local.to_owned(), short_oid);
            ConfirmView::new(
                sanitize_line(&format!("Force-push {local} → {remote_side}?")),
                vec![
                    sanitize_line(&format!(
                        "{remote_side} {theirs} → {ours} · refused if {} moved since your fetch (lease)",
                        params.remote
                    )),
                    sanitize_line(&format!("rewrites history for everyone who has {theirs}")),
                ],
                sanitize_line(&command),
            )
            .with_labels("Cancel", "Force push")
        }
    }
}

/// `Esc` while a write runs: ask it to stop. Not while a text field, the
/// confirm box or the menu is open — their `Esc` is their own. A second
/// `Esc` while the first is being honoured does nothing more.
pub fn cancel_operating(state: &mut AppState, commands: &mut Vec<Command>) -> bool {
    if state.operating.is_none()
        || state.text_field_open()
        || state.confirm.is_some()
        || state.context_menu.is_some()
    {
        return false;
    }
    stop_write(state, commands)
}

/// Ask the running write to stop, and say so — [`cancel_operating`]'s
/// work, without its guard, for the one modal that owns a running write:
/// the commit editor, whose `Esc` while its commit runs means this.
/// `false` when no write runs.
pub(super) fn stop_write(state: &mut AppState, commands: &mut Vec<Command>) -> bool {
    let Some(id) = state.operating else {
        return false;
    };
    // A write that cannot be stopped safely keeps running, and the line says
    // so rather than leaving `Esc` looking ignored. `Busy` is drawn with a
    // trailing `…`, which is right here: it really is still going.
    if let Stoppable::No(why) = stoppable(state, id) {
        state.status = Status::Busy(format!("{} {why}", running_words(state, id)));
        return true;
    }
    if state.remote.cancelling == Some(id) {
        return true;
    }
    state.remote.cancelling = Some(id);
    commands.push(Command::CancelJob(id));
    // `Busy` is drawn with a trailing `…`, so the words stop at the verb.
    state.status = Status::Busy(format!("{} · cancelling", running_words(state, id)));
    true
}

/// Whether `Esc` may stop the write running under `id`. A write nothing
/// recorded is treated as stoppable: that is what every write before this
/// field existed was, and killing one git child is the safe half.
fn stoppable(state: &AppState, id: JobId) -> Stoppable {
    state
        .remote
        .running(id)
        .map_or(Stoppable::Yes, |running| running.stoppable)
}

/// What the running write is called, for the cancel lines.
fn running_words(state: &AppState, id: JobId) -> String {
    state.remote.running(id).map_or_else(
        || "the write".to_owned(),
        |running| running.description.clone(),
    )
}

/// A job ended cancelled. When it is the write `Esc` stopped, that is the
/// answer the user asked for: `operating` clears, the line says so (not
/// sticky), and everything is re-read — a cancelled fetch may have moved
/// some tracking refs, a cancelled commit may have run its hook. `None`
/// when it is not that write, for the caller's own handling.
pub(super) fn cancelled(
    state: &mut AppState,
    id: JobId,
    next_id: &mut impl FnMut() -> JobId,
) -> Option<Vec<Command>> {
    if state.operating != Some(id) || state.remote.cancelling != Some(id) {
        return None;
    }
    state.operating = None;
    state.remote.cancelling = None;
    state.status = Status::Done(format!("{} cancelled", running_words(state, id)));
    Some(super::operate::refresh_scope(
        state,
        graft_ops::apply::Refresh::Everything,
        next_id,
    ))
}

/// A progress line from the running fetch, pull or push. Dropped for any
/// other job, and once `Esc` has asked it to stop.
pub fn remote_progress(state: &mut AppState, id: JobId, brief: &str) {
    if state.operating != Some(id) || state.remote.cancelling == Some(id) {
        return;
    }
    let brief = if state.size.columns < NARROW_COLUMNS {
        narrow(brief)
    } else {
        brief.to_owned()
    };
    state.status = Status::Busy(format!("{} · {brief}{ESC_HINT}", running_words(state, id)));
}

/// `receiving objects 45%` → `receiving 45%`: the phase's first word and the
/// percent, when there is one.
fn narrow(brief: &str) -> String {
    let mut words = brief.split_whitespace();
    let first = words.next().unwrap_or_default();
    match words.last().filter(|last| last.ends_with('%')) {
        Some(percent) => format!("{first} {percent}"),
        None => brief.to_owned(),
    }
}

/// The line a finished remote write leaves: `fetched`, `pulled origin/main`,
/// `pushed main → origin/main`. A push that landed forgets the rejection.
pub(super) fn remote_done(state: &mut AppState, op: &Operation) -> Option<String> {
    match op {
        Operation::Fetch(_) => Some("fetched".to_owned()),
        Operation::Pull(params) => Some(format!(
            "pulled {}",
            short_tracking(&params.upstream.tracking_ref)
        )),
        Operation::Push(params) => {
            state.remote.last_rejected_push = None;
            Some(format!(
                "pushed {} → {}/{}",
                short_branch(&params.branch),
                params.remote,
                short_branch(&params.remote_branch)
            ))
        }
        _ => None,
    }
}

/// The status a failed remote write leaves. `explained` is `ops`' failure
/// line (`ApplyError::Remote`'s `explain`), worded `reason` or `reason ·
/// keys` (the design note's "Failure line" convention, `docs/design/02-…`
/// isn't the source here — `ops::apply::remote::explain`'s own doc comment
/// is). Split so the footer can keep the keys whole and cut the reason
/// first at 80 columns (`hints::fit_status_line`, ADR-0037 §4) — parsed
/// here rather than in `graft-ops`, which is another ticket's crate. A
/// non-fast-forward push is remembered, so the next `P` offers the lease. A
/// cancel is the user's own doing: `Done`, not sticky.
pub fn remote_failed(
    state: &mut AppState,
    op: &Operation,
    failure: &RemoteFailure,
    explained: &str,
) -> Status {
    let sticky = || {
        let (reason, keys) = split_reason_and_keys(explained);
        Status::Sticky { reason, keys }
    };
    match (failure, op) {
        // Not reached through the job pool: `jobs::pool::run_job` turns any
        // product returned after a cancel into `Event::JobCancelled`, which
        // `cancelled` answers, so a cancelled remote write never arrives
        // here. Kept because `RemoteFailure` can carry it from `ops` on any
        // other route, and dropping the arm would make that a silent
        // wrong-worded failure line (m4-remotes/05 review, Minor 4).
        (RemoteFailure::Cancelled, _) => Status::Done(format!("{} cancelled", op.describe())),
        (RemoteFailure::NonFastForward, Operation::Push(params)) => {
            state.remote.last_rejected_push = Some(params.clone());
            sticky()
        }
        // The pull that cannot fast-forward. `graft_ops`' own line sends the
        // user to the shell (`git pull --rebase`); M5 gave them both verbs
        // inside Graft, so the line names the two keys instead and the
        // upstream is remembered for them to aim at.
        (RemoteFailure::NotFastForward, Operation::Pull(params)) => {
            let Some(tip) = params.upstream.tracking_tip else {
                // Nothing to pin `m` or `r` to. `ops`' own sentence is the
                // honest answer, including its shell fallback.
                return sticky();
            };
            state.remote.diverged_upstream = Some((params.upstream.tracking_ref.clone(), tip));
            Status::Sticky {
                reason: format!(
                    "{} and {} have diverged",
                    short_ref(&params.branch),
                    short_tracking(&params.upstream.tracking_ref)
                ),
                keys: Some(DIVERGED_KEYS.to_owned()),
            }
        }
        _ => sticky(),
    }
}

/// `"push rejected: X has moved · p pull first · P force with lease"` →
/// `("push rejected: X has moved", Some("p pull first · P force with
/// lease"))`. A line with no ` · ` (most of them) has no keys to keep
/// whole; the reason is cut like any other footer text.
fn split_reason_and_keys(explained: &str) -> (String, Option<String>) {
    match explained.split_once(" · ") {
        Some((reason, keys)) => (reason.to_owned(), Some(keys.to_owned())),
        None => (explained.to_owned(), None),
    }
}

/// `undo`'s line after it took back an entry that an in-force push newer
/// than it already sent: ` · origin/main still has it`. Empty otherwise.
/// Read from the journal the browser holds — `ops` skipped the push, so the
/// user is owed the reason the remote still differs.
pub(super) fn pushed_since(state: &AppState, undone: &JournalEntry) -> String {
    let Some(target) = &undone.reverses else {
        return String::new();
    };
    let Loading::Ready(read) = &state.journal.entries else {
        return String::new();
    };
    read.entries
        .iter()
        .filter(|entry| {
            entry.kind == OperationKind::Push
                && entry.status == EntryStatus::Committed
                && entry.id > *target
        })
        .max_by(|a, b| a.id.cmp(&b.id))
        .map_or_else(String::new, |push| {
            format!(" · {} still has it", pushed_to(push))
        })
}

/// `origin/main` for a push entry: its recorded tracking ref, else the
/// remote side of its description (`push main → origin/main (set upstream)`).
pub(crate) fn pushed_to(push: &JournalEntry) -> String {
    if let Some(tracking) = push.touches.refs.first() {
        return short_tracking(tracking).to_owned();
    }
    push.description
        .split_once(" → ")
        .map_or("the remote", |(_, rest)| {
            rest.split(" (").next().unwrap_or(rest)
        })
        .to_owned()
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

#[cfg(test)]
mod tests;
