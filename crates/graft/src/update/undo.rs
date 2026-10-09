//! `u`, `Ctrl+Z` and `U`: the keys the milestone's promise rests on
//! (m3-working-tree-undo-and-stash/06, ADR-0002).
//!
//! Pure, like every other `update` module: the key becomes a
//! [`Command::Undo`] and the job does the rest. What lives here is the part
//! that decides — when a key is a no-op, what the drift guard asks, and what
//! the status line says afterwards.
//!
//! The drift rail (DESIGN.md §6.7) is the one place undo stops to ask. `ops`
//! compares the three things the entry recorded — refs, the index and the
//! working tree — against what is there now, and refuses if any moved. That
//! refusal reaches here as [`Undone::Drift`] and becomes the confirm box; a
//! second, deliberate `Enter` on `[ Undo ]` re-runs the same key with
//! `force`.

use graft_core::job::JobId;
use graft_ops::apply::Refresh;
use graft_ui::keymap::{ActionId, Keymap};

use super::events::Command;
use super::state::{AppState, Status};

mod answers;
mod consequence;
mod lands_on;
mod notice;
mod revert;

use answers::launch;
pub(super) use answers::{forced, land_after_undo, reread_disagrees};
pub use answers::{operation_ended, undone};
use notice::UNREADABLE;
pub use notice::interrupted_notice;
pub(crate) use notice::interrupted_notice_in;
// Only the tests reach these through `undo`; the children use them.
#[cfg(test)]
pub(crate) use notice::notice_line;
#[cfg(test)]
use revert::drift_box;
pub(super) use revert::{revert_to_forced, reverted_to};
#[cfg(test)]
use {super::confirm::ConfirmThen, crate::jobs::undo::Undone};

/// What a `u`/`U` launch did (m7-power-tools/30o R3). Every undo path —
/// the graph's key, J's key, `⏎` on J's `← u`/`← U` row — reads this one
/// answer, so none infers it from how many commands came back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UndoStart {
    /// `Command::Undo`/`Redo` pushed, `operating` set.
    Launched,
    /// The undo-after-push box is up; nothing runs until its choice.
    Asked,
    /// Nothing runs; the words are on `AppState::status` (always words).
    Refused,
}

/// `u`/`Ctrl+Z` (`redo` false) and `U` (`redo` true): every launch goes
/// through here.
///
/// Refused in words while a write — including another undo — is already
/// running (`InFlight::words()`, the one wording every refused launch
/// uses, m6-conflicts/78), and while git is part-way through an operation
/// ([`in_progress_refusal`]).
pub(crate) fn start_undo(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    redo: bool,
    next_id: &mut impl FnMut() -> JobId,
) -> UndoStart {
    // `u` after a push explains itself before it runs — and only for `u`:
    // `U` puts back something the user has already taken back, so the
    // remote is where it was when they last saw it (ADR-0038 §4).
    if !redo && let Some(after) = super::remote::undo_after_push(state) {
        state.confirm = Some(super::remote::undo_after_push_box(&after));
        return UndoStart::Asked;
    }
    match launch(state, redo, false, next_id) {
        Some(command) => {
            commands.push(command);
            UndoStart::Launched
        }
        None => UndoStart::Refused,
    }
}

/// [`start_undo`] for a caller that only wants the commands.
pub fn undo_key(
    state: &mut AppState,
    redo: bool,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    let mut commands = Vec::new();
    // The outcome is on the state for every answer (`operating`, the box,
    // or the status words), which is all such a caller reads.
    let _started: UndoStart = start_undo(state, &mut commands, redo, next_id);
    commands
}

/// `[ Undo locally ]` and `[ Re-push old tip ]` on the undo-after-push box.
///
/// `chosen` is the cursor's index, so the two acting choices stay one box
/// rather than two: `1` undoes, `2` opens the lease box. `0` is `Cancel`
/// and never arrives here — which is exactly why cancelling asks again.
///
/// `repush` is `None` when the box could not build one (the remote read
/// failed or is still in flight): the box then drew two choices, so `2`
/// cannot be the cursor's index and there is nothing to open.
pub(super) fn after_push_chosen(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    chosen: usize,
    push: graft_core::journal::EntryId,
    repush: Option<graft_ops::operation::PushParams>,
    next_id: &mut impl FnMut() -> JobId,
) {
    // Answered, so this push explains itself once. Set for both choices: a
    // user who reached the lease box and backed out of it has still read
    // what the box had to say.
    state.remote.undo_after_push_asked = Some(push);
    // A choice that launches closes J when the box was opened from it, as
    // `u` there lands (R3): the result is on the graph. `Cancel` never
    // arrives here, so J stays under a cancelled box.
    match (chosen, repush) {
        (1, _) => {
            if let Some(command) = launch(state, false, false, next_id) {
                commands.push(command);
                close_journal(state, commands);
            }
        }
        (2, Some(params)) => {
            super::remote::repush_chosen(state, params);
            // The lease box stands where the graph does; its push lands
            // there too.
            close_journal(state, commands);
        }
        (other, params) => {
            // Impossible by construction — the box only ever offers two or
            // three choices, `chosen` is its own cursor index, and `2` never
            // arrives without a `repush` (the docs above). `debug_assert!`
            // catches it in every dev and test build; release must not
            // silently do nothing if it ever does fire (CLAUDE.md rule 3),
            // so it also leaves a status line naming the mismatch — costs
            // nothing on the path that never runs, and is seen the one time
            // it might (review M8).
            debug_assert!(
                false,
                "the undo-after-push box offers {} choices; the cursor was on {other}",
                if params.is_some() { "three" } else { "two" }
            );
            state.status = Status::Sticky {
                reason: format!("undo-after-push: no choice {other} on this box"),
                keys: None,
            };
        }
    }
}

/// J closes, when it is shown. Guarded: `journal::close` also cancels J's
/// stage reads, and with J hidden under a preview (`enter_graft`'s time
/// machine) those belong to the preview's way back.
fn close_journal(state: &mut AppState, commands: &mut Vec<Command>) {
    if state.journal.shown {
        super::journal::close(state, commands);
    }
}

/// Why `u`/`U` will not run while git is part-way through an operation, or
/// `None` when nothing is.
///
/// A merge, rebase, cherry-pick or revert that has stopped is a state with
/// exactly two ends — finish it or abort it — and an undo on top of one
/// would be a second history write over an unfinished first. Refused **in
/// words**, with no job and nothing dispatched, and the words name where
/// the two ends live (`docs/design/02-keymap.md` §2.6a).
///
/// A bisect is deliberately not covered: it draws no banner (`banner::live`
/// answers `None` for one, and bisect banners are M7's), so `⇥ banner`
/// would point at nothing.
///
/// **One exception** (keymap §2.6a, m6-conflicts/57): a merge begun in the
/// shell ([`shell_merge_known`]) answers `None`, so `u`/`U` are sent. What a
/// step there may undo is ops' decision, not this one's: its gate lets
/// through only an entry whose after-state is that same merge, and answers
/// anything older with `NotInvertible::OperationLive` — worded by
/// [`undone`] in the same sentence as this refusal.
pub(super) fn in_progress_refusal(state: &AppState) -> Option<Status> {
    let kind = super::banner::live(state)?.kind;
    if shell_merge_known(state) {
        return None;
    }
    Some(live_refusal(kind, banner_keys(state)))
}

/// The keys that reach the banner from where he is, as every "finish or
/// abort" line names them, the shorter way: `⇧⇥ banner` from the graph
/// with the pane open, `⇥ ⇥ banner` from the changes pane's file list
/// (m8/20b R5, 20d) — [`super::banner::way_keys`] over
/// [`super::commit_editor::refusal_way`]; `None` when neither
/// `app.next_pane` nor `app.prev_pane` is bound.
fn banner_keys(state: &AppState) -> Option<String> {
    super::banner::way_keys(super::commit_editor::refusal_way(state), &state.keymap)
}

/// `U redoes <waiting>` after a `nothing to undo`; `None` when redo is
/// unbound (the line is then `nothing to undo` alone).
pub(crate) fn redo_clause(waiting: &str, keys: &Keymap) -> Option<String> {
    keys.key_verb(&[ActionId("app.redo")], &format!("redoes {waiting}"))
}

/// `J journal`, the key a stopped revert names: the journal shows each
/// step that ran. `None` when `graph.journal` is unbound.
pub(crate) fn journal_keys(keys: &Keymap) -> Option<String> {
    keys.key_verb(&[ActionId("graph.journal")], "journal")
}

/// The relaunch notice's warning when the journal could not be read:
/// `…, so u restores the whole tree`, the clause dropped with undo unbound.
pub(crate) fn unreadable_words(keys: &Keymap) -> String {
    match keys.key_verb(&[ActionId("app.undo")], "restores the whole tree") {
        Some(clause) => format!("{UNREADABLE}, so {clause}"),
        None => UNREADABLE.to_owned(),
    }
}

/// A merge is live, **and it is known that Graft did not begin it**: the
/// launch's journal read has answered and named no entry of Graft's live
/// (`commit_editor::live_operation`'s `InShell`).
///
/// Until that read answers, `live_entries` is empty for a merge Graft began
/// too, so `live_operation` alone would call it the shell's. That is fine
/// for the commit editor's words (git's own command works either way) but
/// not for `u`, and not for the abort box, which promises `u` puts the merge
/// back: both treat "not read yet" as "begun in Graft" (decision 1).
pub(super) fn shell_merge_known(state: &AppState) -> bool {
    use super::commit_editor::Begun;
    matches!(
        super::commit_editor::live_operation(state),
        Some((graft_git::state::InProgress::Merge, Begun::InShell))
    ) && matches!(state.journal.entries, super::detail::Loading::Ready(_))
}

/// The one wording of "an operation is live, so not now"
/// (m6-conflicts/57): `u`/`U`'s refusal, the preview's, and ops' own
/// `NotInvertible::OperationLive` answer all build their status here, so the
/// three cannot drift (`operation_live_words_match_ops` pins it to ops'
/// text). `keys` is `None` where the banner is hidden or absent.
pub(super) fn live_refusal(kind: graft_git::state::InProgress, keys: Option<String>) -> Status {
    Status::Sticky {
        reason: live_refusal_words(kind),
        keys,
    }
}

/// [`live_refusal`]'s words alone, for a refusal said inside a modal
/// rather than on the status line (the stash-branch name prompt).
pub(super) fn live_refusal_words(kind: graft_git::state::InProgress) -> String {
    format!("finish or abort the {} first", kind.name())
}

/// Re-read what the restore changed — `update::operate`'s own two scopes,
/// so an undo and the operation it reverses cannot refresh differently.
fn refresh(
    state: &mut AppState,
    what: Refresh,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    // `refresh_scope` re-reads the journal itself: an undo journals an
    // entry of its own, exactly as the operation it reverses did.
    super::operate::refresh_scope(state, what, next_id)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod box_answers;
