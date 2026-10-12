//! A remote write while it runs and as it ends: `Esc` stopping it, the
//! live progress line, and the done and failure lines.

use graft_core::job::JobId;
use graft_git::remote::RemoteFailure;
use graft_ops::operation::Operation;

use super::{
    NARROW_COLUMNS, RejectedPush, Stoppable, diverged_keys, esc_hint, overwrite_keys, short_branch,
    short_ref, short_tracking,
};
use crate::refs::RefBadges;
use crate::update::events::Command;
use crate::update::state::{AppState, RefsRead, Status};

/// `Esc` while a write runs: ask it to stop. Not while a text field, the
/// confirm box or the menu is open — their `Esc` is their own. A second
/// `Esc` while the first is being honoured does nothing more.
pub fn cancel_operating(state: &mut AppState, commands: &mut Vec<Command>) -> bool {
    if !esc_stops_write(state) {
        return false;
    }
    stop_write(state, commands)
}

/// Whether `Esc` now goes to the running write ([`cancel_operating`])
/// before any overlay sees it. The overlays' bars ask this too, so a bar
/// never offers `esc close` while `Esc` would stop the write instead
/// (m8/59 Major A): one predicate, so the key and its words can't drift.
#[must_use]
pub fn esc_stops_write(state: &AppState) -> bool {
    state.operating.is_some()
        && !state.text_field_open()
        && state.confirm.is_none()
        && state.context_menu.is_none()
}

/// Ask the running write to stop, and say so — [`cancel_operating`]'s
/// work, without its guard, for the one modal that owns a running write:
/// the commit editor, whose `Esc` while its commit runs means this.
/// `false` when no write runs.
pub(in crate::update) fn stop_write(state: &mut AppState, commands: &mut Vec<Command>) -> bool {
    let Some(id) = state.operating else {
        return false;
    };
    // Graft waiting to quit on this write (`q y`, m8/13b): `Esc` drops the
    // wait and nothing more; the write runs on, and the next `Esc` is the
    // write's own.
    if crate::update::quit::drop_wait(state) {
        return true;
    }
    // A write that cannot be stopped safely keeps running, and the line says
    // so rather than leaving `Esc` looking ignored. `Busy` is drawn with a
    // trailing `…`, which is right here: it really is still going.
    if let Stoppable::No(why) = stoppable(state, id) {
        state.status = Status::Busy(format!(
            "{} {}",
            running_words(state, id),
            why.words(&state.keymap)
        ));
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
pub(in crate::update) fn cancelled(
    state: &mut AppState,
    id: JobId,
    next_id: &mut impl FnMut() -> JobId,
) -> Option<Vec<Command>> {
    if state.operating != Some(id) || state.remote.cancelling != Some(id) {
        return None;
    }
    state.operating = None;
    state.remote.cancelling = None;
    state.status = Status::done(format!("{} cancelled", running_words(state, id)));
    Some(crate::update::operate::refresh_scope(
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
    state.status = Status::Busy(format!(
        "{} · {brief}{}",
        running_words(state, id),
        esc_hint(&state.keymap)
    ));
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
pub(in crate::update) fn remote_done(state: &mut AppState, op: &Operation) -> Option<String> {
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
        // A remote write a stop ended (21c B+1): `jobs::operate::outcome`
        // answers it although it was cancelled, so its words arrive here
        // rather than `cancelled`'s bare `<desc> cancelled`. `ops`' line is
        // that same sentence, then what the fetch moved before it stopped
        // (ADR-0062) — `Done`, as any cancel: he asked for it.
        (RemoteFailure::Cancelled, _) => Status::done(explained),
        (RemoteFailure::NonFastForward, Operation::Push(params)) => {
            // The tip refused, only from refs that answer the last read: a
            // tip read from refs before a write would make the next `P`
            // compare with the wrong commit (m6-conflicts/49).
            let sent = matches!(state.refs_read, RefsRead::Current(_))
                .then(|| state.refs.as_deref().and_then(RefBadges::head))
                .flatten();
            state.remote.last_rejected_push = Some(RejectedPush {
                params: params.clone(),
                sent,
                leaves: None,
            });
            sticky()
        }
        // The pull that cannot fast-forward. `graft_ops`' own line says
        // "merge or rebase" with no keys; Graft has both verbs, so the line
        // names the two keys and the upstream is remembered for them to aim
        // at.
        (RemoteFailure::NotFastForward, Operation::Pull(params)) => {
            let Some(tip) = params.upstream.tracking_tip else {
                // Nothing to pin `m` or `r` to. `ops`' own sentence is the
                // honest answer: diverged, merge or rebase.
                return sticky();
            };
            state.remote.diverged_upstream = Some((params.upstream.tracking_ref.clone(), tip));
            Status::Sticky {
                reason: format!(
                    "{} and {} have diverged",
                    short_ref(&params.branch),
                    short_tracking(&params.upstream.tracking_ref)
                ),
                keys: diverged_keys(&state.keymap),
            }
        }
        // `ops` names his edits and no keys; the keys that get him through
        // are the app's (m6-conflicts/85 D).
        (RemoteFailure::WouldOverwrite { .. }, Operation::Pull(_)) => Status::Sticky {
            reason: explained.to_owned(),
            keys: overwrite_keys(&state.keymap),
        },
        _ => sticky(),
    }
}

/// `"push rejected: X has commits you don't · p pull, then m or r · P force with lease"` →
/// `("push rejected: X has commits you don't", Some("p pull, then m or r · P force with
/// lease"))`. A line with no ` · ` (most of them) has no keys to keep
/// whole; the reason is cut like any other footer text.
fn split_reason_and_keys(explained: &str) -> (String, Option<String>) {
    match explained.split_once(" · ") {
        Some((reason, keys)) => (reason.to_owned(), Some(keys.to_owned())),
        None => (explained.to_owned(), None),
    }
}
