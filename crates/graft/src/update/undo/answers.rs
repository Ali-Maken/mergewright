//! The answers an undo or a redo comes back with: an operation ended, the
//! launch, what was undone. Moved out of `undo.rs` (m8/21b-5, move-only).

use graft_core::job::JobId;

use super::super::events::Command;
use super::super::state::{AppState, Status};
use super::consequence::{applied_is_redo, done_with, landing};
use super::revert::drift_box;
use super::{banner_keys, in_progress_refusal, live_refusal, redo_clause, refresh};
use crate::jobs::undo::Undone;

/// A `--continue`/`--skip`/`--abort` answered `Done`: the operation is
/// over, on the answer's own word, **in the same update** (gap row 569).
///
/// Before this the only writer of `in_progress` was the working-tree
/// re-read the answer schedules, so for as long as that read took `u` said
/// *"finish or abort the rebase first"* about a rebase that had finished.
/// The re-read still runs and still has the last word: [`reread_disagrees`]
/// says so if it finds the operation standing. A rebase that stops again is
/// `Operated::Conflicted`, never `Done`, so disagreement is a real surprise.
pub fn operation_ended(state: &mut AppState) {
    state.in_progress = None;
    state.banner_focus = false;
    // The one clearer of a stop (m6-conflicts/12), and of a launch landing
    // still owed to an operation that is now over.
    state.stopped = None;
    state.live_landing_owed = false;
    state.ended_by_answer = true;
    // What the finish summary remembered belongs to the operation that
    // just ended (m6-conflicts/19): the next merge starts clean.
    state.choices.clear();
    state.finish_draft = None;
    // And its picks (ease-pass 05).
    state.picks.clear();
}

/// The working-tree read after [`operation_ended`]: spends the mark, and
/// when git still reports an operation, the words for it. `None` when the
/// read agrees, or when no answer was waiting on it.
pub(in crate::update) fn reread_disagrees(
    state: &mut AppState,
    found: Option<&graft_git::state::InProgressDetail>,
) -> Option<Status> {
    if !std::mem::take(&mut state.ended_by_answer) {
        return None;
    }
    found.map(|detail| {
        Status::sticky(format!(
            "git says a {} is still in progress \u{b7} Ctrl+R",
            detail.kind.name()
        ))
    })
}

/// Pays [`AppState::undo_landing_owed`]: called on the **newest**
/// working-tree read only, after it has replaced `in_progress` — an older
/// read looked at the repository before the undo wrote (ticket 41).
///
/// When the undo put an operation back (a finish taken back leaves git
/// mid-merge), the banner takes the keyboard, exactly as a launch into that
/// state lands (keymap §2.6a), and the `Done` line names the key that
/// finishes it again: `undo: finish merge of feature ✓ · c finishes the
/// merge`. Not under a modal or an overlay that hides the banner (the Map,
/// the journal browser, the stash list): the keyboard is not taken from
/// under them, and the landing is dropped rather than kept, because a later
/// read is no longer the undo's answer. Spends the flag in every case.
pub(in crate::update) fn land_after_undo(state: &mut AppState) {
    if !std::mem::take(&mut state.undo_landing_owed) {
        return;
    }
    let Some(kind) = state.in_progress.as_ref().map(|detail| detail.kind) else {
        return;
    };
    if !super::super::banner::banner_focusable(kind)
        || state.modal_owns_keys()
        || state.graph_overlay().is_some()
    {
        return;
    }
    state.banner_focus = true;
    // A key pressed since the answer cleared the line; the focus still
    // lands, and there is no line to add keys to.
    if let Status::Done { keys, .. } = &mut state.status {
        *keys = super::super::banner::continue_words(kind);
    }
}

/// The same key again, past the drift guard: the user has read what moved
/// and chosen `[ Undo ]`.
pub(in crate::update) fn forced(
    state: &mut AppState,
    redo: bool,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    launch(state, redo, true, next_id).into_iter().collect()
}

/// The undo or redo job, or `None` with the refusal's words on the status
/// line.
pub(super) fn launch(
    state: &mut AppState,
    redo: bool,
    force: bool,
    next_id: &mut impl FnMut() -> JobId,
) -> Option<Command> {
    // A write is running: said, as every refused launch says it (R3). It
    // used to return nothing and say nothing, leaving `u` looking dead.
    if let Some(busy) = super::super::operate::in_flight(state) {
        state.status = Status::Failed(busy.words());
        return None;
    }
    // Checked here rather than in `undo_key` alone, so the drift box's
    // second, forcing `Enter` cannot slip past it either.
    if let Some(refusal) = in_progress_refusal(state) {
        state.status = refusal;
        return None;
    }
    let id = next_id();
    let claimed = super::super::operate::claim_write(state, id);
    if claimed.is_err() {
        // Unreachable: `in_flight` returned above.
        super::super::operate::say_if_refused(state, claimed);
        return None;
    }
    let words = if redo { "redo" } else { "undo" };
    // `Esc` stops it. `graft_ops::restore::execute` now checks its stop
    // token between steps and rolls back the ones that already ran
    // (m5-history-and-ghost-preview/06), so a cancelled undo leaves the
    // repository where it started rather than half-applied — which is what
    // `UNDO_CANNOT_STOP` used to be the honest answer to.
    state.status = Status::Busy(format!(
        "{words}{}",
        super::super::remote::esc_hint(&state.keymap)
    ));
    let unstoppable = super::super::remote::launched(
        state,
        id,
        words.to_owned(),
        None,
        super::super::remote::Stoppable::Yes,
        None,
    );
    debug_assert_eq!(unstoppable, None, "an undo can be stopped: no mark");
    Some(if redo {
        Command::Redo(id, force, std::sync::Arc::clone(&state.keymap))
    } else {
        Command::Undo(id, force, std::sync::Arc::clone(&state.keymap))
    })
}

/// `u` on the commit just made gives its message back to the next `c` (Q9
/// a); a redo never takes a commit back, and one re-creating it takes the
/// words back while unedited (08s m1). Complete or not, the same (m2).
fn commit_words(state: &mut AppState, applied: &graft_ops::apply::Applied) {
    if applied_is_redo(applied) {
        crate::update::commit_editor::redo_landed(state, &applied.entry);
    } else {
        crate::update::commit_editor::undo_landed(state, &applied.entry);
    }
}

/// An undo or redo ran to the end: the status names it, and the reads it
/// asks for are started.
fn undid(
    state: &mut AppState,
    applied: &graft_ops::apply::Applied,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    // The entry's own description already reads `undo: stage src/a.rs`
    // (`InversePlan::description`), so it is shown as it stands rather than
    // prefixed into `undid: undo: …`. `u` passes over a push (design note,
    // choice 1), so a commit taken back here may still be on the remote:
    // say so, or the graph's `origin/main` badge reads as a bug.
    let still_there = super::super::remote::pushed_since(state, &applied.entry);
    let what = format!("{}{still_there}", applied.entry.description);
    // A forced run over drift names what it replaced and the key that
    // brings it back (m6-conflicts/77); a clean one adds nothing.
    state.status = landing(what, &applied.replaced, applied_is_redo(applied), 1);
    commit_words(state, applied);
    // Taking back a finish or an abort leaves git mid-operation again, and
    // only the read this asks for can say so: it pays the landing
    // (`land_after_undo`, ticket 41). With a merge begun in the shell
    // already live (a whole-file step undone inside it, m6-conflicts/57)
    // there is no landing to owe: the banner keeps whatever focus it had.
    state.undo_landing_owed = state.in_progress.is_none();
    let commands = refresh(state, applied.refresh, next_id);
    // …and the cursor lands on what came back, as a finished operation's
    // does (m8-polish-and-ship/13, ruling 2): `HEAD` when it moved, else
    // the first branch the entry moved, else where it was.
    if let Some(on) = super::lands_on::of(&applied.entry) {
        super::super::banner::finished_lands_on(state, &commands, on);
    }
    for at in super::lands_on::came_back(&applied.entry) {
        state
            .row_tags
            .set(at, graft_ui::graph_screen::RowTag::PutBack);
    }
    commands
}

/// An undo or redo job answered. Stale answers — a job that is not the one
/// `AppState::operating` waits for — are dropped, exactly as
/// `update::operate::operated` drops them.
pub fn undone(
    state: &mut AppState,
    id: JobId,
    outcome: Undone,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    if state.operating != Some(id) {
        return Vec::new();
    }
    state.operating = None;
    match outcome {
        Undone::Done(applied) => undid(state, &applied, next_id),
        Undone::Incomplete { applied, error } => {
            state.status = Status::sticky(format!(
                "{} \u{2014} could not confirm it fully: {error}",
                applied.entry.description
            ));
            // It ran: a commit it took back gives its words back all the
            // same (08s m2).
            commit_words(state, &applied);
            refresh(state, applied.refresh, next_id)
        }
        // Not a failure: the journal simply has nothing in force to take
        // back. Cleared by the next key, like any other acknowledgement.
        // With a redo waiting, the line names it: after `u y` over drift
        // the edit is one `U` away, and a bare "nothing to undo" read as a
        // dead end (m6-conflicts/77, usability review Q4).
        Undone::Nothing { reason, redo } => {
            let clause = redo.and_then(|waiting| redo_clause(&waiting, &state.keymap));
            state.status = done_with(reason, clause);
            Vec::new()
        }
        // Ops' gate: the entry is older than the merge git is standing in
        // (m6-conflicts/57). A refusal, so `Sticky`, in `u`'s own words;
        // the banner is named only where there is one (a bisect has none).
        Undone::OperationLive { kind } => {
            let keys = super::super::banner::banner_focusable(kind)
                .then(|| banner_keys(state))
                .flatten();
            state.status = live_refusal(kind, keys);
            Vec::new()
        }
        // `Esc`, answered. A clean cancel is an acknowledgement — `Done`,
        // cleared by the next key, because nothing is wrong (CLAUDE.md: a
        // status field must never read as an error when nothing is). A
        // part-way cancel left something in force and is `Sticky` until
        // it is read.
        Undone::Cancelled {
            clean,
            why,
            refresh: what,
        } => {
            state.status = if clean {
                Status::done(why)
            } else {
                Status::sticky(why)
            };
            refresh(state, what, next_id)
        }
        Undone::Drift {
            description,
            drift,
            redo,
        } => {
            // The answer ends its own `Busy`: the box is what the screen
            // waits on now, and `undo · esc cancels` under it would name a
            // job that is over (m6-conflicts/69, Decided 4).
            state.status = Status::Ready;
            state.confirm = Some(drift_box(&description, &drift, redo));
            Vec::new()
        }
        Undone::Failed {
            error,
            refresh: what,
        } => {
            // Sticky, not `Failed`: an undo that ran part-way asks for a
            // refresh of its own, and that refresh finishing would wipe the
            // reason off the screen before it could be read (m3/05 review,
            // Major 7).
            state.status = Status::sticky(error);
            match what {
                Some(what) => refresh(state, what, next_id),
                None => Vec::new(),
            }
        }
    }
}
