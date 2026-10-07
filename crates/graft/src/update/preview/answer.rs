//! A preview's answers: planned, refused or failed, the click, confirm and close, and the box drawn from them.

use super::{
    AppState, Arc, BadgeOverlay, Command, HISTORY_CANNOT_STOP, HashMap, JobId, Loading, Operation,
    PUT_BACK_CANNOT_STOP, Planned, PreviewBox, PreviewFailure, PreviewState, PreviewSubject, Row,
    SliceError, Status, badges_for_preview, build_slice, choice_at, closing_box, enter,
    footer_keys, keeps_drops_here, planning_title, preview_box_rect, put_back_box, put_in_words,
    ready_box, sanitize_line, short, stash_box, stash_put_back, stash_put_back_op, take_title_of,
    with_cursor,
};
use crate::update::op_keys::OpKeys;

/// A click while the preview is open.
///
/// A click on one of the box's buttons is that choice, acted on at once.
/// Every other click — and the wheel, which never reaches here — is
/// ignored: the rows behind a modal picture are not selectable, and a
/// stray click must never dismiss a box the user is reading.
///
/// `true` whenever a preview was open.
pub fn preview_click(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    column: u16,
    row: u16,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    let Some(view) = preview_box(state) else {
        return false;
    };
    let Some(area) = preview_box_rect(state, &view) else {
        return true;
    };
    if let Some(choice) = choice_at(&view, area, column, row) {
        if let Some(preview) = state.preview.as_mut() {
            preview.cursor = choice;
        }
        enter(state, commands, next_id);
    }
    true
}

/// The open preview's plan failed: a refusal closes the box with a status
/// line (m6-conflicts/67) — except a stash put-back's, which keeps its box:
/// the refusal is about the list the box sits over, and `esc` is back on it
/// (m6-conflicts/86; the design note's "Refused: the box's refusal state").
/// A failed read shows in the box.
pub(super) fn plan_failed(state: &mut AppState, failure: PreviewFailure) {
    let Some(preview) = state.preview.as_mut() else {
        return;
    };
    match failure {
        PreviewFailure::Refused(refusal) if stash_put_back(&preview.subject).is_some() => {
            preview.plan = Loading::Failed(refusal.explain(&OpKeys(&state.keymap)));
            preview.refused = true;
            preview.cursor = 0;
        }
        PreviewFailure::Refused(refusal) => refused(state, &refusal),
        PreviewFailure::Failed(error) => {
            preview.plan = Loading::Failed(error);
            preview.cursor = 0;
        }
    }
}

/// The plan for an open preview landed (or failed).
///
/// Lays the slice out **once**, here, rather than per frame: `layout` is a
/// pure function over a few hundred commits and the result is what the
/// graph draws until the box closes.
pub fn preview_produced(
    state: &mut AppState,
    id: JobId,
    result: Result<Arc<Planned>, PreviewFailure>,
) {
    // The same seeds the live graph was laid out with, rebuilt from the
    // walk's own facts and the badges: the ghosts have to sit in the same
    // lanes and carry the same colours as the real rows around them, or the
    // before and after read as two graphs rather than one picture.
    let seeds = state
        .refs
        .as_deref()
        .map_or_else(graft_graph::Seeds::default, |refs| {
            crate::history::preview_seeds(
                state.history.as_ref().and_then(|it| it.walk.as_ref()),
                refs,
            )
        });
    // Which absolute history row the user's screen starts at, and which
    // their cursor is on. Read before the preview is borrowed, and read
    // once: a preview is modal, so neither can move while it is open.
    let viewport = state.history.as_ref().map(|it| (it.top, it.cursor));
    let Some(preview) = state.preview.as_mut() else {
        // The preview closed while the job was in flight. Nothing to do —
        // `close_preview` has already cancelled it.
        return;
    };
    if preview.job != id {
        // A stale answer, from a preview that has been closed and reopened.
        return;
    }
    let planned = match result {
        Ok(planned) => planned,
        Err(failure) => {
            plan_failed(state, failure);
            return;
        }
    };
    // The op the job completed (05b: a rebase's line and drops) becomes the
    // one `y` runs — `confirm_preview` moves the subject's op across. A
    // copy out of the shared answer; the key's own op is let go.
    if let (Some(runs), PreviewSubject::Operation { op, .. }) =
        (&planned.runs, &mut preview.subject)
    {
        op.clone_from(runs);
    }
    match build_slice(&preview.resident, &planned.anchors, &planned.plan) {
        Ok(slice) => {
            preview.ghost_index = slice
                .ghost_at
                .iter()
                .enumerate()
                .filter_map(|(index, &at)| Some((slice.commits.get(at)?.id, index)))
                .collect();
            preview.clipped = slice.clipped;
            preview.undrawable = None;
            let rows = graft_graph::layout(&slice.commits, seeds);
            let (top, cursor) = viewport.unwrap_or((0, 0));
            // `top` is always resident — it is the row the user is looking
            // at — so the fallback only fires when there is no history at
            // all, where `rows` is empty anyway.
            preview.view_start = row_of(preview, &rows, top).unwrap_or(0);
            preview.selected_row = row_of(preview, &rows, cursor);
            preview.rows = rows;
        }
        Err(SliceError::MissingParent(oid)) => {
            preview.rows = Vec::new();
            preview.view_start = 0;
            preview.selected_row = None;
            preview.ghost_index = HashMap::new();
            preview.clipped = 0;
            preview.undrawable = Some(oid);
        }
    }
    preview.plan = Loading::Ready(planned);
    preview.cursor = 0;
}

/// The preview's operation was refused: close the box, and say why on the
/// status line, in the words the write would use.
///
/// No box: the preview is modal, so the key the words name (`S`) would need
/// `esc` first (UX §1.3), and a picture of a write that cannot run is not
/// the result before you confirm. Focus goes back where the key was
/// pressed — nothing but the box moved it; the journal comes back if the
/// preview was opened from there, as `close_preview` does.
pub(super) fn refused(state: &mut AppState, refusal: &graft_ops::operation::Refusal) {
    let Some(preview) = state.preview.take() else {
        return;
    };
    if preview.return_to_journal {
        state.journal.shown = true;
    }
    crate::update::picker::preview_closed(state);
    state.status = match &preview.subject {
        PreviewSubject::Operation { op, .. } => {
            crate::update::operate::refusal_status(op, refusal, state.size.columns, &state.keymap)
        }
        // The time machine is planned by `plan_revert_to`, which the job
        // never maps to `Refused`; if it ever does, its sentence stands.
        PreviewSubject::RevertTo(_) => Status::sticky(refusal.explain(&OpKeys(&state.keymap))),
    };
}

/// Where the commit on absolute history row `row` ended up in the laid-out
/// slice, if the slice holds it at all.
///
/// By id rather than by arithmetic: the slice has the plan's ghosts woven
/// into it, so a resident row's index in `rows` is its index in the
/// snapshot plus however many ghosts were inserted above it.
pub(super) fn row_of(preview: &PreviewState, rows: &[Row], row: usize) -> Option<usize> {
    let id = preview
        .resident
        .get(row.checked_sub(preview.slice_top)?)?
        .id;
    rows.iter().position(|it| it.commit.id == id)
}

/// `Confirm`: run the very operation that was drawn.
///
/// The preview closes on the same event — the picture has been answered —
/// and the status line carries the operation's description from this
/// keystroke rather than from whenever the job thread gets going.
pub fn confirm_preview(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(preview) = state.preview.take() else {
        return;
    };
    let Some(description) = preview.planned().map(|it| it.plan.description.clone()) else {
        // `Confirm` is only offered over a plan that landed
        // (`PreviewState::acts`), so this cannot fire. If it ever did, the
        // honest answer is to leave the picture up: launching an operation
        // under a description this function had to invent is how the box
        // and the Command Log come to describe one act two ways
        // (review 08 Minor 2).
        state.preview = Some(preview);
        return;
    };
    // Answered: a picker hidden under it is done (m7-power-tools/10).
    crate::update::picker::preview_confirmed(state);
    match preview.subject {
        PreviewSubject::Operation { op, opened_by } => {
            // The same value, not a rebuilt one: `check` refuses `Moved` if
            // a tip has shifted since the picture was drawn, which is the
            // whole of hard choice 3. `opened_by` rides along so that
            // refusal can name the key to press again.
            //
            // The preview has closed on this key, so a refusal (a write
            // began while it was up; `preview_refusal` only runs at open)
            // is said on the row it gave back (m6-conflicts/78).
            //
            // A stash put-back runs as the list's key ran it before the
            // preview (m6-conflicts/86): one git write, which `Esc` may
            // stop, so `launch` — not the history ops' `launch_unstoppable`,
            // whose `Esc` words name a rebase's banner.
            //
            // A put-back confirmed over J closes J too, once it has
            // launched: it lands on the graph, on what came back
            // (m7-power-tools/24-a2). J was left shown under the box
            // (`return_to_journal` false), so nothing else would close it.
            // Refused, he stays on J with the refusal on its status row
            // (m7/30o Minor 7). Its `Esc` words are its own: a put-back
            // has no banner (`PUT_BACK_CANNOT_STOP`).
            let put_back = matches!(*op, Operation::Recover(_));
            let launched = if stash_put_back_op(&op).is_some() {
                crate::update::operate::launch(state, commands, *op, next_id)
            } else {
                crate::update::operate::launch_unstoppable(
                    state,
                    commands,
                    *op,
                    if put_back {
                        PUT_BACK_CANNOT_STOP
                    } else {
                        HISTORY_CANNOT_STOP
                    },
                    Some(opened_by),
                    next_id,
                )
            };
            if put_back && launched.is_ok() {
                crate::update::journal::close(state, commands);
            }
            crate::update::operate::say_if_refused(state, launched);
        }
        PreviewSubject::RevertTo(entry) => {
            let id = next_id();
            let claimed = crate::update::operate::claim_write(state, id);
            if claimed.is_err() {
                crate::update::operate::say_if_refused(state, claimed);
                return;
            }
            state.status = Status::Busy(description.clone());
            commands.extend(crate::update::remote::launched(
                state,
                id,
                description,
                None,
                crate::update::remote::Stoppable::No(HISTORY_CANNOT_STOP),
                // No key to press again: the time machine is aimed from a
                // journal row, not from a key over the graph.
                None,
            ));
            commands.push(Command::RevertTo(
                id,
                entry,
                false,
                std::sync::Arc::clone(&state.keymap),
            ));
        }
    }
}

/// Closes the preview without running anything.
///
/// A plan still in flight is cancelled: it is a read, but it can be several
/// `merge-tree` spawns, and nobody is waiting for the answer any more. The
/// journal browser reopens if it was what asked.
pub fn close_preview(state: &mut AppState, commands: &mut Vec<Command>) {
    let Some(preview) = state.preview.take() else {
        return;
    };
    // A put-in's words go with its picture (m7-power-tools/09).
    crate::update::amend_into::forget(state);
    if preview.planning() {
        commands.push(Command::CancelJob(preview.job));
    }
    if preview.return_to_journal {
        state.journal.shown = true;
    }
    // The branch picker comes back the same way (m7-power-tools/10).
    crate::update::picker::preview_closed(state);
}

/// The badges the graph draws while the preview is open, or `None` when
/// there is no plan to overlay.
#[must_use]
pub fn preview_badges(state: &AppState) -> Option<BadgeOverlay<'_>> {
    let preview = state.preview.as_ref()?;
    let planned = preview.planned()?;
    let badges = state.refs.as_deref()?;
    Some(badges_for_preview(badges, &planned.plan))
}

// --- the box ------------------------------------------------------------

/// Everything the Ghost Preview box draws, or `None` when none is open.
///
/// Pure over [`AppState`], and **every line comes from the
/// [`PreviewPlan`]**: the refs it moves, what it does to the working tree,
/// what the forecast says, what `u` would take back, and `plan.commands`
/// verbatim. No `git …` string is built here (`KNOWN_GAPS` row 229) — the
/// plan's own lines are the ones that run.
#[must_use]
pub fn preview_box(state: &AppState) -> Option<PreviewBox> {
    let preview = state.preview.as_ref()?;
    let view = match &preview.plan {
        Loading::Idle | Loading::Pending(_) => {
            let mut view = PreviewBox::new(sanitize_line(&planning_title(&preview.subject)));
            view.planning = true;
            view
        }
        Loading::Failed(words) if preview.refused => closing_box(words, &preview.subject),
        Loading::Failed(error) => {
            closing_box(&format!("preview failed: {error}"), &preview.subject)
        }
        Loading::Ready(planned) => {
            if let Some(missing) = preview.undrawable {
                closing_box(
                    &format!(
                        "preview cannot be drawn here: {} is not loaded · esc",
                        short(missing)
                    ),
                    &preview.subject,
                )
            } else if let Some((index, pop)) = stash_put_back(&preview.subject) {
                stash_box(&planned.plan, index, pop)
            } else if let Some(Operation::Recover(params)) = preview.subject.operation() {
                put_back_box(state, &planned.plan, params)
            } else {
                ready_box(
                    &planned.plan,
                    preview.subject.operation(),
                    keeps_drops_here(state).is_some(),
                    preview.clipped,
                    state.size.columns,
                )
            }
        }
    };
    let mut view = view;
    if let Some(title) = preview.subject.operation().and_then(take_title_of) {
        view.title = sanitize_line(&format!("Preview: {title}"));
    }
    // A put-back from J says what comes back, planning, ready or refused
    // (m7-power-tools/24-a2, J1), not `describe`'s journal words.
    if let Some(Operation::Recover(params)) = preview.subject.operation() {
        view.title = sanitize_line(&crate::update::recover::preview_title(params, |oid| {
            crate::update::recover::subject_in(state, oid)
        }));
    }
    let keep = keeps_drops_here(state).is_some();
    let view = put_in_words(footer_keys(view, preview, keep), state, preview);
    Some(with_cursor(view, preview.cursor))
}
