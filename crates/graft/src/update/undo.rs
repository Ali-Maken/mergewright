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
use graft_term::sanitize_line;

use super::confirm::{ConfirmThen, PendingConfirm};
use super::events::Command;
use super::state::{AppState, Status};
use crate::jobs::undo::{RevertedTo, Undone};

/// `u`/`Ctrl+Z` (`redo` false) and `U` (`redo` true).
///
/// Nothing while a write — including another undo — is already running:
/// one at a time, and the status line is already saying what that one is
/// (`update::operate::operation_in_flight`).
pub fn undo_key(
    state: &mut AppState,
    redo: bool,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    // `u` after a push explains itself before it runs — and only for `u`:
    // `U` puts back something the user has already taken back, so the
    // remote is where it was when they last saw it (ADR-0038 §4).
    if !redo && let Some(after) = super::remote::undo_after_push(state) {
        state.confirm = Some(super::remote::undo_after_push_box(&after));
        return Vec::new();
    }
    launch(state, redo, false, next_id)
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
    match (chosen, repush) {
        (1, _) => commands.extend(launch(state, false, false, next_id)),
        (2, Some(params)) => super::remote::repush_chosen(state, params),
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
    Some(live_refusal(kind, Some(BANNER_KEYS.to_owned())))
}

/// The key that reaches the banner, as every "finish or abort" line names it.
const BANNER_KEYS: &str = "\u{21e5} banner";

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
}

/// The working-tree read after [`operation_ended`]: spends the mark, and
/// when git still reports an operation, the words for it. `None` when the
/// read agrees, or when no answer was waiting on it.
pub(super) fn reread_disagrees(
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
pub(super) fn land_after_undo(state: &mut AppState) {
    if !std::mem::take(&mut state.undo_landing_owed) {
        return;
    }
    let Some(kind) = state.in_progress.as_ref().map(|detail| detail.kind) else {
        return;
    };
    if !super::banner::banner_focusable(kind)
        || state.modal_owns_keys()
        || state.graph_overlay().is_some()
    {
        return;
    }
    state.banner_focus = true;
    // A key pressed since the answer cleared the line; the focus still
    // lands, and there is no line to add keys to.
    if let Status::Done { keys, .. } = &mut state.status {
        *keys = super::banner::continue_words(kind);
    }
}

/// The same key again, past the drift guard: the user has read what moved
/// and chosen `[ Undo ]`.
pub(super) fn forced(
    state: &mut AppState,
    redo: bool,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    launch(state, redo, true, next_id)
}

fn launch(
    state: &mut AppState,
    redo: bool,
    force: bool,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    if super::operate::operation_in_flight(state) {
        return Vec::new();
    }
    // Checked here rather than in `undo_key` alone, so the drift box's
    // second, forcing `Enter` cannot slip past it either.
    if let Some(refusal) = in_progress_refusal(state) {
        state.status = refusal;
        return Vec::new();
    }
    let id = next_id();
    let claimed = super::operate::claim_write(state, id);
    if claimed.is_err() {
        // Unreachable: `operation_in_flight` returned above.
        super::operate::say_if_refused(state, claimed);
        return Vec::new();
    }
    let words = if redo { "redo" } else { "undo" };
    // `Esc` stops it. `graft_ops::restore::execute` now checks its stop
    // token between steps and rolls back the ones that already ran
    // (m5-history-and-ghost-preview/06), so a cancelled undo leaves the
    // repository where it started rather than half-applied — which is what
    // `UNDO_CANNOT_STOP` used to be the honest answer to.
    state.status = Status::Busy(format!("{words}{}", super::remote::ESC_HINT));
    super::remote::launched(
        state,
        id,
        words.to_owned(),
        None,
        super::remote::Stoppable::Yes,
        None,
    );
    vec![if redo {
        Command::Redo(id, force)
    } else {
        Command::Undo(id, force)
    }]
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
    let still_there = super::remote::pushed_since(state, &applied.entry);
    let what = format!("{}{still_there}", applied.entry.description);
    // A forced run over drift names what it replaced and the key that
    // brings it back (m6-conflicts/77); a clean one adds nothing.
    let keys = replaced_keys(&applied.replaced, applied_is_redo(applied), 1);
    state.status = done_with(what, keys);
    // Taking back a finish or an abort leaves git mid-operation again, and
    // only the read this asks for can say so: it pays the landing
    // (`land_after_undo`, ticket 41). With a merge begun in the shell
    // already live (a whole-file step undone inside it, m6-conflicts/57)
    // there is no landing to owe: the banner keeps whatever focus it had.
    state.undo_landing_owed = state.in_progress.is_none();
    refresh(state, applied.refresh, next_id)
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
            refresh(state, applied.refresh, next_id)
        }
        // Not a failure: the journal simply has nothing in force to take
        // back. Cleared by the next key, like any other acknowledgement.
        // With a redo waiting, the line names it: after `u y` over drift
        // the edit is one `U` away, and a bare "nothing to undo" read as a
        // dead end (m6-conflicts/77, usability review Q4).
        Undone::Nothing { reason, redo } => {
            state.status = done_with(reason, redo.map(|waiting| format!("U redoes {waiting}")));
            Vec::new()
        }
        // Ops' gate: the entry is older than the merge git is standing in
        // (m6-conflicts/57). A refusal, so `Sticky`, in `u`'s own words;
        // the banner is named only where there is one (a bisect has none).
        Undone::OperationLive { kind } => {
            let keys = super::banner::banner_focusable(kind).then(|| BANNER_KEYS.to_owned());
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

/// The time machine again, past the drift guard: the user has read what
/// moved and chosen `[ Revert ]`.
///
/// Unstoppable for the same reason the first attempt was: a jump is several
/// restores, and `Esc` between two of them would leave the repository
/// somewhere nobody asked for.
pub(super) fn revert_to_forced(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    entry: graft_core::journal::EntryId,
    what: &str,
    next_id: &mut impl FnMut() -> JobId,
) {
    if super::operate::operation_in_flight(state) {
        return;
    }
    let id = next_id();
    let claimed = super::operate::claim_write(state, id);
    if claimed.is_err() {
        // Unreachable: `operation_in_flight` returned above.
        super::operate::say_if_refused(state, claimed);
        return;
    }
    // The entry's own description, not its id: `01J…` on the busy line is
    // a fact about the journal file, not about what the user asked for
    // (review 08 Minor 2).
    let description = format!("revert to before \u{201c}{what}\u{201d}");
    state.status = Status::Busy(description.clone());
    super::remote::launched(
        state,
        id,
        description,
        None,
        super::remote::Stoppable::No(super::preview::HISTORY_CANNOT_STOP),
        None,
    );
    commands.push(Command::RevertTo(id, entry, true));
}

/// The journal's time machine answered (DESIGN §6.6,
/// m5-history-and-ghost-preview/08).
///
/// A jump is several restores, and the three answers say so plainly: how
/// many steps it took, how far it got when it stopped, or what moved
/// underneath before it started. **`Everything` is re-read whenever a step
/// ran** — a jump moves refs, `HEAD`, the index and the working tree, and no
/// narrower scope covers all four.
pub(super) fn reverted_to(
    state: &mut AppState,
    id: JobId,
    outcome: RevertedTo,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    if state.operating != Some(id) {
        return Vec::new();
    }
    state.operating = None;
    match outcome {
        RevertedTo::Done {
            description,
            total,
            steps,
            replaced,
            presses,
        } => {
            let what = format!(
                "reverted to before \u{201c}{description}\u{201d} ({total} {})",
                if total == 1 { "step" } else { "steps" }
            );
            state.status = done_with(what, replaced_keys(&replaced, false, presses));
            // `steps` is what ran, kept for the record rather than for the
            // refresh: `Everything` is wider than any of their own scopes.
            debug_assert_eq!(steps.len(), total, "every planned step journaled");
            refresh(state, Refresh::Everything, next_id)
        }
        RevertedTo::Drift {
            description,
            total,
            drift,
            target,
        } => {
            // As for `undone`'s drift box: the answer ends its own `Busy`.
            state.status = Status::Ready;
            state.confirm = Some(revert_drift_box(&description, &drift, total, target));
            Vec::new()
        }
        RevertedTo::Refused { description, error } => {
            // Nothing ran, so nothing is to be refreshed and nothing is
            // half-done: a refusal, worded as one (review 08 Minor 4).
            state.status = Status::Sticky {
                reason: format!("cannot revert to before \u{201c}{description}\u{201d}: {error}"),
                keys: None,
            };
            Vec::new()
        }
        RevertedTo::Failed {
            description: _,
            done,
            total,
            error,
        } => {
            // Sticky, and worded as a state: some steps really did run, and
            // each of them is its own journal entry the browser can show.
            state.status = Status::Sticky {
                reason: format!(
                    "revert stopped after {done} of {total} {}: {error}",
                    if total == 1 { "step" } else { "steps" }
                ),
                keys: Some("J journal".to_owned()),
            };
            if done > 0 {
                refresh(state, Refresh::Everything, next_id)
            } else {
                Vec::new()
            }
        }
    }
}

/// The time machine's own drift box. Same rail, same words, but `[ Revert ]`
/// re-runs the jump rather than one `u`.
///
/// `total` is the jump's step count. The box only ever comes from the
/// jump's first step (a later step's drift stops it part-way), and `U`
/// walks the jump back from its last step, so bringing back what step 0
/// replaces takes all `total` presses.
fn revert_drift_box(
    description: &str,
    drift: &graft_ops::undo::Drift,
    total: usize,
    target: graft_core::journal::EntryId,
) -> PendingConfirm {
    PendingConfirm {
        view: graft_ui::confirm::ConfirmView::new(
            sanitize_line(&format!("Revert to before \u{201c}{description}\u{201d}?")),
            with_consequence(
                wrapped(&format!("Since then, {}.", drift.explain())),
                drift,
                false,
                total,
            ),
            sanitize_line("revert restores the recorded state, step by step"),
        )
        .with_labels("Stop", "Revert"),
        then: ConfirmThen::RevertTo {
            entry: target,
            description: description.to_owned(),
        },
    }
}

/// The drift guard's box: what the operation was, what has moved since, and
/// the safe answer under the cursor.
///
/// The consequence line is `drift.explain()` verbatim — `ops` names which of
/// the three comparisons failed, and naming them again here would let the two
/// wordings drift apart.
fn drift_box(description: &str, drift: &graft_ops::undo::Drift, redo: bool) -> PendingConfirm {
    let verb = if redo { "Redo" } else { "Undo" };
    PendingConfirm {
        view: graft_ui::confirm::ConfirmView::new(
            sanitize_line(&format!("{verb} \u{201c}{description}\u{201d}?")),
            with_consequence(
                wrapped(&format!("Since this {description}, {}.", drift.explain())),
                drift,
                redo,
                1,
            ),
            // No single git command to show: a restore is a plan of several,
            // and the journal browser's `Space` is where they are listed.
            sanitize_line(&format!(
                "{} restores the recorded state",
                verb.to_lowercase()
            )),
        )
        .with_labels("Stop", verb),
        then: ConfirmThen::Undo { redo },
    }
}

/// The drift line's wrapped lines with the consequence line under them,
/// fitted into what graft-ui has left, each line sanitized for the box.
fn with_consequence(
    mut message: Vec<String>,
    drift: &graft_ops::undo::Drift,
    redo: bool,
    presses: usize,
) -> Vec<graft_term::SanitizedText> {
    let room = CONFIRM_LINES.saturating_sub(message.len());
    message.extend(wrapped(&drift_consequence(
        drift,
        redo,
        presses,
        CONFIRM_COLUMNS,
        room,
    )));
    message.iter().map(|line| sanitize_line(line)).collect()
}

/// The widest line the confirm box draws without re-wrapping it: the box
/// is 64 columns at its narrowest (`graft_ui::confirm::confirm_area`) and
/// wraps its message 4 columns inside that. A `…` where the drift's own
/// words should be is the one thing this box must not do.
const CONFIRM_COLUMNS: u16 = 60;

/// The most message lines graft-ui draws before it cuts the rest with a
/// `…` (`graft-ui/src/confirm.rs`, `MAX_MESSAGE_LINES`).
const CONFIRM_LINES: usize = 4;

/// The drift box's consequence line (m6-conflicts/77, Decided 6): what the
/// run replaces, that a backup keeps it, and the key that brings it back —
/// fitted to `width` in at most `lines` wrapped lines, so graft-ui's cap
/// never cuts it.
///
/// `presses` is how many `U`s the way back takes: 1 for `u`, the jump's
/// step count for the time machine. A redo names **no** key: `u` after a
/// redo takes back the operation at the root (`graft_ops::journal::
/// next_undo` steps over `Undo` and `Redo` entries), which writes that
/// operation's before-state, not the edit the redo replaced. The backup
/// keeps it, and the line says only that.
///
/// To fit, the one path is cut from the left (`…/fix.txt`) until the
/// sentence fits; if the drift line above left a single line, the sentence
/// also drops `a backup keeps it, and ` and keeps the key. When even that
/// cannot fit the short form is returned whole and graft-ui cuts it.
pub(crate) fn drift_consequence(
    drift: &graft_ops::undo::Drift,
    redo: bool,
    presses: usize,
    width: u16,
    lines: usize,
) -> String {
    let verb = if redo { "Redo" } else { "Undo" };
    let key = (!redo).then(|| way_back_key(presses));
    let replaced = &drift.replaced;
    if replaced.is_empty() {
        // Nothing the run writes differs from what the entry left: the
        // user's edits (if any) are on other paths and stay put.
        return if drift.worktree || drift.index.is_some() {
            format!("{verb} leaves your edits since as they are.")
        } else {
            match key {
                Some(key) => format!("A backup is taken first, and {key} brings this state back."),
                None => "A backup is taken first.".to_owned(),
            }
        };
    }
    let (named, it) = named_replaced(replaced);
    let sentence = |what: &str, long: bool| match (&key, long) {
        (Some(key), true) => {
            format!("{verb} replaces {what}; a backup keeps {it}, and {key} brings {it} back.")
        }
        (Some(key), false) => format!("{verb} replaces {what}; {key} brings {it} back."),
        (None, _) => format!("{verb} replaces {what}; a backup keeps {it}."),
    };
    let width = usize::from(width);
    let fits = |text: &str| {
        let wrapped = wrapped_at(text, width);
        wrapped.len() <= lines && wrapped.iter().all(|line| line.chars().count() <= width)
    };
    let shown = match &named {
        Named::Path(path) => left_cuts(path),
        Named::Whole(what) => vec![what.clone()],
    };
    let described = |shown: &str| match &named {
        Named::Path(_) => format!("your edit to {shown}"),
        Named::Whole(_) => shown.to_owned(),
    };
    for long in [true, false] {
        if let Some(found) = shown
            .iter()
            .map(|shown| sentence(&described(shown), long))
            .find(|text| fits(text))
        {
            return found;
        }
    }
    sentence(&described(shown.last().map_or("", String::as_str)), false)
}

/// The status line's way back after a forced run (m6-conflicts/77):
/// `U brings back your edit to fix.txt`, or `None` when nothing was
/// replaced. A redo names the backup and no key (see [`drift_consequence`]).
pub(crate) fn replaced_keys(
    replaced: &graft_ops::undo::Replaced,
    redo: bool,
    presses: usize,
) -> Option<String> {
    if replaced.is_empty() {
        return None;
    }
    let what = match named_replaced(replaced).0 {
        Named::Path(path) => format!("your edit to {path}"),
        Named::Whole(what) => what,
    };
    Some(if redo {
        format!("a backup keeps {what}")
    } else {
        format!("{} brings back {what}", way_back_key(presses))
    })
}

/// What a [`graft_ops::undo::Replaced`] is called: one path by name,
/// anything wider by what it is. With the pronoun that goes with it.
enum Named {
    /// The one path, as text.
    Path(String),
    /// `the whole working tree` or `your edits to 3 files`.
    Whole(String),
}

fn named_replaced(replaced: &graft_ops::undo::Replaced) -> (Named, &'static str) {
    match replaced.paths.as_slice() {
        _ if replaced.whole_tree => (Named::Whole("the whole working tree".to_owned()), "it"),
        [one] => (Named::Path(one.to_string_lossy().into_owned()), "it"),
        many => (
            Named::Whole(format!("your edits to {} files", many.len())),
            "them",
        ),
    }
}

/// `Done`, naming `keys` when there are any.
fn done_with(what: String, keys: Option<String>) -> Status {
    match keys {
        Some(keys) => Status::done_with_keys(what, keys),
        None => Status::done(what),
    }
}

/// Whether a finished `Undone::Done` was `U`'s: its entry is a `Redo`.
fn applied_is_redo(applied: &graft_ops::apply::Applied) -> bool {
    applied.entry.kind == graft_core::journal::OperationKind::Redo
}

/// `U`, or `U (3 times)` when the way back is several redos.
fn way_back_key(presses: usize) -> String {
    if presses > 1 {
        format!("U ({presses} times)")
    } else {
        "U".to_owned()
    }
}

/// `path` whole, then cut from the left at each `/` (`…/src/fix.txt`,
/// `…/fix.txt`), then the file name itself cut a character at a time —
/// longest first, so the first that fits keeps the most.
fn left_cuts(path: &str) -> Vec<String> {
    let mut cuts = vec![path.to_owned()];
    cuts.extend(
        path.match_indices('/')
            .map(|(at, _)| format!("\u{2026}{}", &path[at..])),
    );
    let name = path.rsplit('/').next().unwrap_or(path);
    let chars: Vec<char> = name.chars().collect();
    cuts.extend((1..chars.len()).map(|from| {
        let kept: String = chars[from..].iter().collect();
        format!("\u{2026}{kept}")
    }));
    cuts
}

/// `text` broken at spaces into lines of at most [`CONFIRM_COLUMNS`]. A
/// single word longer than that is left whole — the box clips it, and
/// cutting a path in half would be worse than a clipped one.
fn wrapped(text: &str) -> Vec<String> {
    wrapped_at(text, usize::from(CONFIRM_COLUMNS))
}

/// [`wrapped`] at any width.
fn wrapped_at(text: &str, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        match lines.last_mut() {
            Some(line) if line.chars().count() + 1 + word.chars().count() <= width => {
                line.push(' ');
                line.push_str(word);
            }
            _ => lines.push(word.to_owned()),
        }
    }
    lines
}

/// The footer's interrupted-entry line, until `J` has been pressed once.
///
/// Graft was killed — or the machine was — with an operation half-run. The
/// launch read closes such an entry and seals its backup, so `u` can still
/// take it back; the user is told it is there rather than left to find it.
/// An entry whose after-state could not be read says so too: its `u`
/// restores the whole working tree, which is not a thing to discover by
/// pressing the key.
///
/// Read off the journal itself, not off what a read happened to close. The
/// journal is re-read after every write and on `Ctrl+R`, and the second read
/// closes nothing — deriving the notice from "what this read recovered" made
/// it vanish at the first staged file, which is the one moment the user most
/// needs to know an operation did not finish (m3/06 review, Major 1).
/// An entry a later entry has taken back is not announced.
#[must_use]
pub fn interrupted_notice(state: &AppState) -> Option<String> {
    if state.journal.notice_seen {
        return None;
    }
    let super::detail::Loading::Ready(read) = &state.journal.entries else {
        return None;
    };
    // Announced unless a later entry has taken it back. Deliberately *not*
    // `in_force`: that answers "can `u` reach it", which is false for an
    // interrupted entry with no backup — and that is the one the user most
    // needs to be told about, not the one to keep quiet about.
    let undone: std::collections::HashSet<&graft_core::journal::EntryId> = read
        .entries
        .iter()
        .filter_map(|entry| entry.reverses.as_ref())
        .collect();
    // A live entry is `Pending` with no outcome, exactly like a torn one, so
    // `is_interrupted` says yes to both. Only git's own state tells them
    // apart, and the last journal read asked it: an operation standing right
    // there, with a banner two rows above saying so, was never interrupted
    // (m5/09 seam A).
    let mut interrupted = read
        .entries
        .iter()
        .filter(|entry| {
            entry.is_interrupted()
                && !undone.contains(&entry.id)
                && !state.live_entries.contains(&entry.id)
        })
        .map(|entry| entry.description.as_str());
    let first = interrupted.next()?;
    let more = interrupted.count();
    let rest = if more == 0 {
        String::new()
    } else {
        format!(" (and {more} more)")
    };
    let widest = if state.journal.unreadable.is_empty() {
        String::new()
    } else {
        " \u{b7} its state could not be read, so u restores the whole tree".to_owned()
    };
    Some(format!(
        "an operation was interrupted: {first}{rest}{widest} \u{b7} J to inspect"
    ))
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
mod tests {
    use std::sync::Arc;

    use graft_core::event::{KeyCode, KeyPress};
    use graft_core::journal::{
        EntryId, EntryStatus, HeadRef, IndexState, JournalEntry, OperationKind, Touches,
    };
    use graft_ops::apply::Applied;

    use super::*;
    use crate::jobs::undo::JournalRead;
    use crate::update::detail::Loading;
    use crate::update::journal::JournalState;

    fn ids() -> impl FnMut() -> JobId {
        let mut next = 0;
        move || {
            next += 1;
            JobId::new(next)
        }
    }

    /// An `Applied` naming `description`, with `refresh` as its scope: the
    /// two fields `undone` reads.
    fn applied(description: &str, refresh: Refresh) -> Applied {
        Applied {
            entry: JournalEntry {
                id: EntryId::new(1, [0; 10]),
                at_unix_ms: 1,
                kind: OperationKind::Undo,
                description: description.to_owned(),
                status: EntryStatus::Committed,
                touches: Touches::default(),
                head_before: HeadRef::Branch("refs/heads/main".to_owned()),
                head_after: None,
                refs_before: std::collections::BTreeMap::new(),
                refs_after: std::collections::BTreeMap::new(),
                index_before: None,
                index_after: None,
                worktree_before: None,
                worktree_after: None,
                backup_ref: None,
                pins: Vec::new(),
                commands: Vec::new(),
                reverses: None,
                merge_before: None,
                merge_after: None,
            },
            refresh,
            step_backup: None,
            replaced: graft_ops::undo::Replaced::default(),
        }
    }

    /// Two different merged trees: index drift that still explains itself as
    /// "the index changed", the wording the m3 captures pin.
    fn index_drift() -> (IndexState, IndexState) {
        let oid = |byte: u8| graft_core::oid::Oid::from_bytes(&[byte; 20]).expect("20 bytes");
        (IndexState::Merged(oid(1)), IndexState::Merged(oid(2)))
    }

    fn drift() -> graft_ops::undo::Drift {
        graft_ops::undo::Drift {
            index: Some(index_drift()),
            ..graft_ops::undo::Drift::default()
        }
    }

    #[test]
    fn u_asks_for_an_undo_and_says_so() {
        let mut state = AppState::default();
        let commands = undo_key(&mut state, false, &mut ids());
        assert_eq!(commands, vec![Command::Undo(JobId::new(1), false)]);
        assert_eq!(state.operating, Some(JobId::new(1)));
        // The busy line offers the cancel, because the cancel is now real
        // (m5-history-and-ghost-preview/06): `restore::execute` rolls back
        // the steps it had applied. `update::remote::tests::
        // esc_cancels_an_undo_in_flight` is the other half.
        assert_eq!(
            state.status,
            Status::Busy(format!("undo{}", super::super::remote::ESC_HINT))
        );
    }

    #[test]
    fn capital_u_asks_for_a_redo() {
        let mut state = AppState::default();
        let commands = undo_key(&mut state, true, &mut ids());
        assert_eq!(commands, vec![Command::Redo(JobId::new(1), false)]);
    }

    /// Piece 6. A stopped merge, rebase, cherry-pick or revert has exactly
    /// two ends, and `u` is neither. Refused **in words**, with no job — and
    /// the words name where the two ends are.
    #[test]
    fn u_refused_during_an_operation() {
        for (kind, word) in [
            (graft_git::state::InProgress::Merge, "merge"),
            (graft_git::state::InProgress::Rebase, "rebase"),
            (graft_git::state::InProgress::CherryPick, "cherry-pick"),
            (graft_git::state::InProgress::Revert, "revert"),
        ] {
            let mut state = AppState {
                in_progress: Some(graft_git::state::InProgressDetail {
                    kind,
                    step: None,
                    head_name: None,
                    onto: None,
                    theirs: Ok(None),
                }),
                ..AppState::default()
            };
            for redo in [false, true] {
                let commands = undo_key(&mut state, redo, &mut ids());
                assert!(commands.is_empty(), "{word}: {commands:?}");
                assert_eq!(state.operating, None, "{word}: a job was claimed");
                assert_eq!(
                    state.status,
                    Status::Sticky {
                        reason: format!("finish or abort the {word} first"),
                        keys: Some("\u{21e5} banner".to_owned()),
                    },
                    "{word}"
                );
            }
        }
    }

    /// The drift box's second, forcing `Enter` is the same key again, so it
    /// meets the same refusal: `[ Undo ]` must not be a way around a live
    /// operation.
    #[test]
    fn forcing_past_the_drift_box_is_refused_during_an_operation_too() {
        let mut state = AppState {
            confirm: Some(drift_box("stage src/a.rs", &drift(), false)),
            in_progress: Some(graft_git::state::InProgressDetail {
                kind: graft_git::state::InProgress::Rebase,
                step: None,
                head_name: None,
                onto: None,
                theirs: Ok(None),
            }),
            ..AppState::default()
        };
        state
            .confirm
            .as_mut()
            .expect("the box is open")
            .view
            .toggle();
        let mut commands = Vec::new();
        assert!(super::super::confirm::confirm_key(
            &mut state,
            &mut commands,
            KeyPress::plain(KeyCode::Enter),
            &mut ids(),
        ));
        assert!(commands.is_empty(), "{commands:?}");
        assert_eq!(state.operating, None);
    }

    /// A stopped merge, and the journal as the app last read it: `Some`
    /// holds the entries git is still inside (`JournalRead::live`), `None`
    /// is the launch read still out.
    fn merging(journal: Option<Vec<EntryId>>) -> AppState {
        let (entries, live_entries) = match journal {
            Some(live) => (
                Loading::Ready(Arc::new(JournalRead {
                    entries: Vec::new(),
                    unreadable: Vec::new(),
                    live: live.clone(),
                })),
                live,
            ),
            None => (Loading::Pending(JobId::new(40)), Vec::new()),
        };
        AppState {
            in_progress: Some(graft_git::state::InProgressDetail {
                kind: graft_git::state::InProgress::Merge,
                step: None,
                head_name: None,
                onto: None,
                theirs: Ok(None),
            }),
            live_entries,
            journal: JournalState {
                entries,
                ..JournalState::default()
            },
            ..AppState::default()
        }
    }

    /// Keymap §2.6a's one exception (m6-conflicts/57): in a merge **begun
    /// in the shell** — the journal read, and no entry of Graft's live —
    /// `u` and `U` are sent, and ops' gate answers.
    #[test]
    fn u_in_a_merge_begun_in_the_shell_is_sent() {
        for redo in [false, true] {
            let mut state = merging(Some(Vec::new()));
            let commands = undo_key(&mut state, redo, &mut ids());
            let sent = if redo {
                Command::Redo(JobId::new(1), false)
            } else {
                Command::Undo(JobId::new(1), false)
            };
            assert_eq!(commands, vec![sent], "redo {redo}: {:?}", state.status);
        }
    }

    /// A merge Graft began folds its steps into its own live entry, so `a`
    /// is its way back: `u` refuses as before.
    #[test]
    fn u_still_refuses_in_a_merge_begun_in_graft() {
        let mut state = merging(Some(vec![EntryId::new(7, [7; 10])]));
        for redo in [false, true] {
            assert!(undo_key(&mut state, redo, &mut ids()).is_empty());
            assert_eq!(
                state.status,
                live_refusal(
                    graft_git::state::InProgress::Merge,
                    Some("\u{21e5} banner".to_owned())
                )
            );
        }
    }

    /// Until the launch's journal read answers, `live_entries` is empty for
    /// a merge Graft began too, so "begun in the shell" is not known yet:
    /// refused as before (decision 1).
    #[test]
    fn u_refuses_while_the_launch_journal_read_is_out() {
        let mut state = merging(None);
        for redo in [false, true] {
            assert!(undo_key(&mut state, redo, &mut ids()).is_empty());
            assert_eq!(state.operating, None);
            assert_eq!(
                state.status,
                live_refusal(
                    graft_git::state::InProgress::Merge,
                    Some("\u{21e5} banner".to_owned())
                )
            );
        }
    }

    /// The app's refusal and ops' own answer are the same sentence for
    /// every kind, so `u` reads the same whichever side refused it.
    #[test]
    fn operation_live_words_match_ops() {
        use graft_git::state::InProgress;
        use graft_ops::undo::{NotInvertible, UndoError};
        for kind in [
            InProgress::Merge,
            InProgress::Rebase,
            InProgress::CherryPick,
            InProgress::Revert,
            InProgress::Bisect,
        ] {
            let ops = UndoError::Nothing(NotInvertible::OperationLive { kind }).explain();
            let Status::Sticky { reason, .. } = live_refusal(kind, None) else {
                panic!("a refusal is sticky");
            };
            assert_eq!(reason, ops, "{kind:?}");
        }
    }

    /// Ops' gate refused (an entry older than the merge): a refusal in the
    /// app's words, sticky, pointing at the banner — not the plain `Done`
    /// that "nothing to undo" gets.
    #[test]
    fn an_operation_live_answer_is_a_refusal() {
        let mut state = AppState {
            operating: Some(JobId::new(2)),
            ..merging(Some(Vec::new()))
        };
        let commands = undone(
            &mut state,
            JobId::new(2),
            Undone::OperationLive {
                kind: graft_git::state::InProgress::Merge,
            },
            &mut ids(),
        );
        assert!(commands.is_empty(), "nothing ran: {commands:?}");
        assert_eq!(state.operating, None);
        assert_eq!(
            state.status,
            Status::Sticky {
                reason: "finish or abort the merge first".to_owned(),
                keys: Some("\u{21e5} banner".to_owned()),
            }
        );
    }

    #[test]
    fn u_while_a_write_is_running_asks_for_nothing() {
        let mut state = AppState {
            operating: Some(JobId::new(4)),
            ..AppState::default()
        };
        assert!(undo_key(&mut state, false, &mut ids()).is_empty());
        // The running write keeps the slot: a second `u` must not steal it.
        assert_eq!(state.operating, Some(JobId::new(4)));
    }

    #[test]
    fn a_finished_undo_says_what_it_undid_and_re_reads_the_working_tree() {
        let mut state = AppState {
            operating: Some(JobId::new(2)),
            ..AppState::default()
        };
        let commands = undone(
            &mut state,
            JobId::new(2),
            Undone::Done(applied("undo: stage src/a.rs", Refresh::WorkingTree)),
            &mut ids(),
        );
        assert_eq!(state.status, Status::done("undo: stage src/a.rs"));
        assert!(state.operating.is_none());
        assert!(
            commands
                .iter()
                .any(|command| matches!(command, Command::ReadWorkingTree(_))),
            "{commands:?}"
        );
        assert!(
            commands
                .iter()
                .any(|command| matches!(command, Command::ReadJournal(_))),
            "the journal has a new entry of its own: {commands:?}"
        );
    }

    #[test]
    fn an_undo_that_moved_refs_re_reads_everything() {
        let mut state = AppState {
            operating: Some(JobId::new(2)),
            ..AppState::default()
        };
        let commands = undone(
            &mut state,
            JobId::new(2),
            Undone::Done(applied("undo: commit \"fix\"", Refresh::Everything)),
            &mut ids(),
        );
        assert!(commands.contains(&Command::OpenRepository), "{commands:?}");
        assert!(
            commands
                .iter()
                .any(|command| matches!(command, Command::ReadRefs(_))),
            "{commands:?}"
        );
    }

    #[test]
    fn an_answer_for_another_job_is_dropped() {
        let mut state = AppState {
            operating: Some(JobId::new(9)),
            ..AppState::default()
        };
        let commands = undone(
            &mut state,
            JobId::new(2),
            Undone::Done(applied("undo: stage a", Refresh::Everything)),
            &mut ids(),
        );
        assert!(commands.is_empty());
        assert_eq!(state.operating, Some(JobId::new(9)));
        assert_eq!(state.status, Status::Ready);
    }

    #[test]
    fn nothing_to_undo_is_an_answer_not_a_failure() {
        let mut state = AppState {
            operating: Some(JobId::new(2)),
            ..AppState::default()
        };
        let commands = undone(
            &mut state,
            JobId::new(2),
            Undone::Nothing {
                reason: "nothing to undo".to_owned(),
                redo: None,
            },
            &mut ids(),
        );
        assert_eq!(state.status, Status::done("nothing to undo"));
        assert!(commands.is_empty());
    }

    /// m6-conflicts/77: with a redo waiting, "nothing to undo" names the
    /// key that goes the other way instead of ending on a dead end.
    #[test]
    fn nothing_to_undo_names_the_redo_waiting() {
        let mut state = AppState {
            operating: Some(JobId::new(2)),
            ..AppState::default()
        };
        undone(
            &mut state,
            JobId::new(2),
            Undone::Nothing {
                reason: "nothing to undo".to_owned(),
                redo: Some("checkout fix".to_owned()),
            },
            &mut ids(),
        );
        assert_eq!(
            state.status,
            Status::done_with_keys("nothing to undo", "U redoes checkout fix")
        );
    }

    #[test]
    fn a_failed_undo_stays_on_screen_past_its_own_refresh() {
        let mut state = AppState {
            operating: Some(JobId::new(2)),
            ..AppState::default()
        };
        let commands = undone(
            &mut state,
            JobId::new(2),
            Undone::Failed {
                error: "refs/heads/main is not where it was".to_owned(),
                refresh: None,
            },
            &mut ids(),
        );
        assert_eq!(
            state.status,
            Status::sticky("refs/heads/main is not where it was".to_owned())
        );
        assert!(commands.is_empty(), "nothing ran: {commands:?}");
    }

    #[test]
    fn drift_opens_a_box_that_quotes_what_moved_and_answers_nothing_yet() {
        let mut state = AppState {
            operating: Some(JobId::new(2)),
            ..AppState::default()
        };
        let commands = undone(
            &mut state,
            JobId::new(2),
            Undone::Drift {
                description: "stage src/a.rs".to_owned(),
                drift: drift(),
                redo: false,
            },
            &mut ids(),
        );
        assert!(commands.is_empty(), "nothing runs until the user says so");
        let pending = state.confirm.as_ref().expect("the drift box opened");
        assert_eq!(pending.view.selected(), 0, "the safe choice first");
        assert_eq!(pending.view.choices[0].as_str(), "Stop");
        assert_eq!(pending.view.choices[1].as_str(), "Undo");
        assert!(
            pending
                .view
                .message
                .iter()
                .any(|line| line.as_str().contains(&drift().explain())),
            "the box says what ops said: {:?}",
            pending.view.message
        );
        assert_eq!(pending.then, ConfirmThen::Undo { redo: false });
    }

    #[test]
    fn a_long_drift_explanation_is_wrapped_rather_than_clipped() {
        // The box is 64 columns at most, and a `…` where the drift's own
        // words should be is the one thing it must not do.
        let drift = graft_ops::undo::Drift {
            index: Some(index_drift()),
            worktree: true,
            unknown: vec!["HEAD", "the working tree"],
            ..drift()
        };
        let mut state = AppState::default();
        let id = JobId::new(1);
        state.operating = Some(id);
        undone(
            &mut state,
            id,
            Undone::Drift {
                description: "commit \"Add the OAuth provider and its tests\"".to_owned(),
                drift,
                redo: false,
            },
            &mut ids(),
        );
        let view = &state.confirm.as_ref().expect("the guard opened").view;
        assert!(view.message.len() > 2, "{:?}", view.message);
        for line in &view.message {
            assert!(line.chars().count() <= 62, "{line}");
        }
        let joined = view
            .message
            .iter()
            .map(graft_term::SanitizedText::as_str)
            .collect::<Vec<_>>()
            .join(" ");
        assert!(joined.contains("the working tree"), "{joined}");
    }

    #[test]
    fn enter_on_stop_changes_nothing_and_enter_on_undo_forces() {
        let box_open = |cursor_on_undo: bool| {
            let mut view = drift_box("stage src/a.rs", &drift(), false);
            if cursor_on_undo {
                view.view.toggle();
            }
            AppState {
                confirm: Some(view),
                ..AppState::default()
            }
        };

        let mut stopped = box_open(false);
        let mut commands = Vec::new();
        assert!(super::super::confirm::confirm_key(
            &mut stopped,
            &mut commands,
            KeyPress::plain(KeyCode::Enter),
            &mut ids(),
        ));
        assert!(commands.is_empty(), "Stop does nothing: {commands:?}");
        assert!(stopped.operating.is_none());

        let mut forced = box_open(true);
        let mut commands = Vec::new();
        assert!(super::super::confirm::confirm_key(
            &mut forced,
            &mut commands,
            KeyPress::plain(KeyCode::Enter),
            &mut ids(),
        ));
        assert_eq!(commands, vec![Command::Undo(JobId::new(1), true)]);
    }

    #[test]
    fn a_redo_drift_box_re_runs_the_redo() {
        let mut state = AppState {
            confirm: Some(drift_box("undo: commit \"fix\"", &drift(), true)),
            ..AppState::default()
        };
        state
            .confirm
            .as_mut()
            .expect("the box is open")
            .view
            .toggle();
        let mut commands = Vec::new();
        assert!(super::super::confirm::confirm_key(
            &mut state,
            &mut commands,
            KeyPress::plain(KeyCode::Enter),
            &mut ids(),
        ));
        assert_eq!(commands, vec![Command::Redo(JobId::new(1), true)]);
    }

    /// A journal holding one entry per description, each one torn the way
    /// recovery leaves it: `Aborted { reason: "interrupted" }`, which is what
    /// the view is actually handed (`Pending` never survives a read).
    fn read(interrupted: &[&str], unreadable: &[&str]) -> JournalState {
        let entries = interrupted
            .iter()
            .enumerate()
            .map(|(index, description)| torn_entry(index, description))
            .collect();
        JournalState {
            entries: Loading::Ready(Arc::new(JournalRead {
                entries,
                unreadable: Vec::new(),
                live: Vec::new(),
            })),
            unreadable: unreadable.iter().map(|&s| s.to_owned()).collect(),
            ..JournalState::default()
        }
    }

    /// One entry a previous run left open and recovery then closed.
    fn torn_entry(index: usize, description: &str) -> graft_core::journal::JournalEntry {
        use graft_core::journal::{EntryId, EntryStatus, HeadRef, OperationKind, Touches};
        let oid = |byte: u8| graft_core::oid::Oid::from_bytes(&[byte; 20]).expect("20 bytes");
        let number = i64::try_from(index).expect("a handful of entries") + 1;
        graft_core::journal::JournalEntry {
            id: EntryId::new(number, [0; 10]),
            at_unix_ms: number,
            kind: OperationKind::Commit,
            description: description.to_owned(),
            status: EntryStatus::Aborted {
                reason: graft_core::journal::INTERRUPTED.to_owned(),
            },
            touches: Touches {
                refs: Vec::new(),
                index: false,
                worktree: true,
            },
            head_before: HeadRef::Branch("refs/heads/main".to_owned()),
            head_after: None,
            refs_before: std::collections::BTreeMap::new(),
            refs_after: std::collections::BTreeMap::new(),
            index_before: None,
            index_after: None,
            // A backup of the tree it changed, so `in_force` counts it: an
            // entry `u` can no longer reach is not one to announce.
            worktree_before: Some(oid(3)),
            worktree_after: None,
            backup_ref: Some("refs/graft/backup/main/x".to_owned()),
            pins: Vec::new(),
            commands: Vec::new(),
            reverses: None,
            merge_before: None,
            merge_after: None,
        }
    }

    #[test]
    fn the_notice_names_the_interrupted_entry_until_it_has_been_seen() {
        let mut state = AppState {
            journal: read(&["commit \"fix\""], &[]),
            ..AppState::default()
        };
        let notice = interrupted_notice(&state).expect("there is one to report");
        assert!(notice.contains("commit \"fix\""), "{notice}");
        assert!(notice.contains("J to inspect"), "{notice}");

        state.journal.notice_seen = true;
        assert_eq!(interrupted_notice(&state), None);
    }

    #[test]
    fn a_clean_journal_has_no_notice() {
        let state = AppState {
            journal: read(&[], &[]),
            ..AppState::default()
        };
        assert_eq!(interrupted_notice(&state), None);
    }

    #[test]
    fn an_unreadable_after_state_says_undo_would_restore_everything() {
        let state = AppState {
            journal: read(&["stash pop"], &["stash pop: could not read HEAD"]),
            ..AppState::default()
        };
        let notice = interrupted_notice(&state).expect("there is one to report");
        assert!(notice.contains("restores the whole tree"), "{notice}");
    }

    /// Review 08 Minor 4. A revert whose plan could not be made at all
    /// used to say `revert stopped after 0 of 0 steps`, which describes
    /// something that ran and gave up. Nothing ran.
    #[test]
    fn a_revert_that_never_started_is_worded_as_a_refusal() {
        let mut state = AppState::default();
        let running = JobId::new(7);
        state.operating = Some(running);
        let mut next = 0;
        let mut next_id = || {
            next += 1;
            JobId::new(next)
        };

        let commands = reverted_to(
            &mut state,
            running,
            crate::jobs::undo::RevertedTo::Refused {
                description: "commit one".to_owned(),
                error: "there is nothing to revert".to_owned(),
            },
            &mut next_id,
        );

        assert_eq!(
            state.status,
            Status::sticky(
                "cannot revert to before \u{201c}commit one\u{201d}: there is nothing to revert"
            )
        );
        assert!(commands.is_empty(), "nothing ran, so nothing is re-read");
    }

    /// Review 08 Minor 2. The drift box's `[ Revert ]` used to launch under
    /// `revert to before "01J..."` — the entry's ULID, because the
    /// description was not carried through `ConfirmThen::RevertTo`. A ULID
    /// is a fact about the journal file, not about what the user asked for.
    #[test]
    fn the_time_machines_busy_line_names_the_entry_not_its_id() {
        let target = EntryId::new(1, [0; 10]);
        let mut state = AppState::default();
        let mut commands = Vec::new();
        let mut next = 0;
        let mut next_id = || {
            next += 1;
            JobId::new(next)
        };
        // `reverted_to` drops an outcome that is not the job in flight.
        let running = JobId::new(99);
        state.operating = Some(running);
        reverted_to(
            &mut state,
            running,
            crate::jobs::undo::RevertedTo::Drift {
                description: "commit one".to_owned(),
                total: 1,
                drift: drift(),
                target,
            },
            &mut next_id,
        );
        let confirm = state.confirm.as_mut().expect("the drift box is open");
        // Index 0 is `Stop`; `[ Revert ]` is the acting choice.
        confirm.view.right();

        assert!(crate::update::confirm::confirm_key(
            &mut state,
            &mut commands,
            KeyPress::plain(KeyCode::Enter),
            &mut next_id,
        ));

        assert_eq!(
            state.status,
            Status::Busy("revert to before \u{201c}commit one\u{201d}".to_owned()),
            "the entry's own words, not its ULID"
        );
        assert!(
            matches!(commands.as_slice(), [Command::RevertTo(_, _, true)]),
            "{commands:?}"
        );
    }

    // ── ticket 41: the landing after an undo that puts an operation back ──

    const FINISH_UNDONE: &str = "undo: finish merge of feature";

    /// `u` on a finish, answered `Done` with nothing live yet: the read the
    /// answer asked for, and the state waiting on it. A working tree has
    /// been read before, as it has in any running app: the first read is
    /// the launch's landing (`relaunch`), not the undo's.
    fn finish_undone(state: AppState) -> (AppState, JobId) {
        let mut state = AppState {
            operating: Some(JobId::new(2)),
            working_tree: Some(Arc::new(graft_git::status::WorkingTreeStatus {
                entries: Vec::new(),
            })),
            ..state
        };
        let mut next = {
            let mut raw = 100;
            move || {
                raw += 1;
                JobId::new(raw)
            }
        };
        let commands = undone(
            &mut state,
            JobId::new(2),
            Undone::Done(applied(FINISH_UNDONE, Refresh::WorkingTree)),
            &mut next,
        );
        let read = commands
            .iter()
            .find_map(|command| match command {
                Command::ReadWorkingTree(id) => Some(*id),
                _ => None,
            })
            .expect("the answer asks for the working tree");
        (state, read)
    }

    /// Read `id` lands, reporting `kind` live (or nothing).
    fn read_lands(
        state: &AppState,
        id: JobId,
        kind: Option<graft_git::state::InProgress>,
    ) -> AppState {
        let read = crate::working_tree::WorkingTree {
            status: Arc::new(graft_git::status::WorkingTreeStatus {
                entries: Vec::new(),
            }),
            in_progress: kind.map(|kind| graft_git::state::InProgressDetail {
                kind,
                step: None,
                head_name: None,
                onto: None,
                theirs: Ok(None),
            }),
            mergetool: None,
        };
        crate::update::update(
            state,
            crate::update::AppEvent::Produced(
                id,
                crate::update::Product::WorkingTree(Ok(Box::new(read))),
            ),
        )
        .0
    }

    fn done_line_of(status: &Status) -> String {
        match status {
            Status::Done { what, keys } => crate::view::status::done_line(what, keys.as_deref()),
            other => panic!("not a done line: {other:?}"),
        }
    }

    #[test]
    fn an_undo_that_puts_a_merge_back_lands_on_the_banner() {
        let (state, read) = finish_undone(AppState::default());
        assert!(state.in_progress.is_none(), "nothing live when it answers");
        let state = read_lands(&state, read, Some(graft_git::state::InProgress::Merge));
        assert!(state.banner_focus, "the banner has the keyboard");
        assert!(!state.undo_landing_owed, "the landing is spent");
        assert_eq!(
            state.status,
            Status::done_with_keys(FINISH_UNDONE, "c finishes the merge")
        );
        assert_eq!(
            done_line_of(&state.status),
            "undo: finish merge of feature ✓ · c finishes the merge"
        );
    }

    #[test]
    fn an_undo_with_nothing_put_back_leaves_focus_alone() {
        let (state, read) = finish_undone(AppState::default());
        assert!(state.undo_landing_owed, "owed while the read is out");
        let focus = (state.graph_has_focus(), state.banner_focus);
        let state = read_lands(&state, read, None);
        assert_eq!((state.graph_has_focus(), state.banner_focus), focus);
        assert_eq!(state.status, Status::done(FINISH_UNDONE));
        assert!(!state.undo_landing_owed, "spent all the same");
    }

    #[test]
    fn a_stale_read_does_not_spend_the_landing() {
        // A read asked before the undo's own refresh: older, so it read the
        // repository before the undo wrote.
        let older = JobId::new(50);
        let (state, read) = finish_undone(AppState {
            status_read: Some(older),
            ..AppState::default()
        });
        assert_ne!(read, older);
        let state = read_lands(&state, older, None);
        assert!(!state.banner_focus, "no focus on a stale read");
        assert!(state.undo_landing_owed, "the stale read did not spend it");
        let state = read_lands(&state, read, Some(graft_git::state::InProgress::Merge));
        assert!(state.banner_focus, "the undo's own read lands it");
        assert!(!state.undo_landing_owed);
    }

    /// The Map, the journal browser and the stash list replace the graph
    /// screen and its banner, and `u` works under the journal browser, where
    /// the key that ran the undo was pressed; any of the three can also be
    /// opened while the undo runs.
    #[test]
    fn under_a_modal_the_landing_is_dropped() {
        let (mut state, read) = finish_undone(AppState::default());
        state.journal.shown = true;
        assert_eq!(
            state.graph_overlay(),
            Some(crate::update::GraphOverlay::Journal)
        );
        let state = read_lands(&state, read, Some(graft_git::state::InProgress::Merge));
        assert!(!state.banner_focus, "no focus under the overlay");
        assert_eq!(state.status, Status::done(FINISH_UNDONE), "no keys added");
        assert!(!state.undo_landing_owed, "dropped, not kept for later");
    }

    #[test]
    fn a_rebase_put_back_says_continues() {
        let (state, read) = finish_undone(AppState::default());
        let state = read_lands(&state, read, Some(graft_git::state::InProgress::Rebase));
        assert!(state.banner_focus);
        let Status::Done { keys, .. } = &state.status else {
            panic!("{:?}", state.status);
        };
        assert_eq!(keys.as_deref(), Some("c continues the rebase"));
        let Status::Done {
            keys: every_file_done_keys,
            ..
        } = crate::update::resolver::every_file_done(&state)
        else {
            panic!("every_file_done is a done line");
        };
        assert_eq!(
            *keys, every_file_done_keys,
            "the same words as every file done"
        );
    }
}

#[cfg(test)]
mod box_answers;
