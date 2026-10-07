//! A write's answer: `operated` and the line each landing leaves.

use super::{
    AppState, Command, InProgress, JobId, Landing, Operated, Operation, PathBuf, Refresh, Status,
    operation_failed, refresh_scope, stalled_line,
};

use graft_ui::keymap::ActionId;

pub(crate) mod staging_not_kept;

pub(crate) use staging_not_kept::staging_not_kept_status;

/// `operated`'s `Incomplete` answer: the write ran, only part of its
/// after-state could not be read back. It lands and tags as a `Done` does
/// (m8-polish-and-ship/13, ruling 3) — the write happened, so he should see
/// its result — and keeps its own warning on the status row, sticky, over
/// whatever line the landing would have left (a put-in's `Done` words
/// included: they are spent here, so no later landing picks them up,
/// review m7-09-to-21c Minor 1).
pub(super) fn incomplete_answered(
    state: &mut AppState,
    launched_op: Option<&Operation>,
    applied: &graft_ops::apply::Applied,
    error: &str,
    lands: Option<Landing>,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    let warning = Status::sticky(format!(
        "{} \u{2014} could not confirm it fully: {error}",
        applied.entry.description
    ));
    let mut commands = landed(state, applied, lands, next_id);
    crate::update::amend_into::landed(state, launched_op, &mut commands, next_id);
    crate::update::recover::tag_rows(state, launched_op);
    state.status = warning;
    commands
}

/// The line a clean write leaves: a clash's drop words the end of the clash;
/// a remote op its own words; a switch to a branch `on <b> ✓ · u goes back`
/// (10b); else the entry's description — then, after the tick, `· your
/// edits are back` when the write set edits aside and they came back clean
/// (m6-conflicts/74).
pub(super) fn done_status(
    state: &mut AppState,
    remote_op: Option<&Operation>,
    launched_op: Option<&Operation>,
    clash_drop: Option<(Option<usize>, graft_core::journal::PutBackWhy)>,
    applied: &graft_ops::apply::Applied,
) -> Status {
    if let Some((index, why)) = clash_drop {
        return crate::update::clash::dropped_status(index, why);
    }
    if pulled_nothing(launched_op, &applied.entry) {
        return Status::done(UP_TO_DATE);
    }
    // `a` at a rewrite's `edit` stop (07b): the stop is still live and `c`
    // is the next key.
    if matches!(launched_op, Some(Operation::Amend(_)))
        && crate::update::commit_editor::at_edit_stop(state)
    {
        return Status::Done {
            what: "amended the stopped commit".to_owned(),
            also: None,
            keys: state
                .keymap
                .key_verb(&[ActionId("operation.continue")], "continues"),
        };
    }
    if let Some(on) = launched_op.and_then(crate::update::switch::landed_words) {
        return Status::Done {
            what: on,
            also: crate::update::clash::edits_back(applied),
            keys: Some(crate::update::switch::GOES_BACK.to_owned()),
        };
    }
    // A put-back from the journal names what came back (m7-power-tools/24).
    if let Some(Operation::Recover(params)) = launched_op {
        return crate::update::recover::done_status(params, |oid| {
            crate::update::recover::subject_in(state, oid)
        });
    }
    // A one-commit cherry-pick (`o`, or slot 2's `⏎`) lands in `o`'s words
    // (m7-power-tools/11; the journal keeps git's `cherry-pick …`).
    if let Some(Operation::CherryPick(params)) = launched_op
        && params.commits.len() == 1
    {
        let onto = crate::update::preview::short_name_of(&params.head.name);
        return Status::Done {
            what: format!("took \"{}\" onto {onto}", params.subject),
            also: crate::update::clash::edits_back(applied),
            keys: state.keymap.key_verb(&[ActionId("app.undo")], "undoes"),
        };
    }
    Status::Done {
        what: done_what(state, remote_op, applied),
        also: crate::update::clash::edits_back(applied),
        keys: None,
    }
}

/// What a `Done` puts before its tick: a remote op's own words, else the
/// entry's description.
pub(super) fn done_what(
    state: &mut AppState,
    remote_op: Option<&Operation>,
    applied: &graft_ops::apply::Applied,
) -> String {
    remote_op
        .and_then(|op| crate::update::remote::remote_done(state, op))
        .unwrap_or_else(|| applied.entry.description.clone())
}

/// What a write that ran leaves behind whatever its line says: the editor
/// closed, the scope re-read, and the cursor where the launch said it lands
/// ([`landing_of`]).
pub(super) fn landed(
    state: &mut AppState,
    applied: &graft_ops::apply::Applied,
    lands: Option<Landing>,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    // The write the editor was open for landed, so the editor and
    // the draft behind it are about a commit that now exists
    // (m3-working-tree-undo-and-stash/05). A failure leaves both
    // alone: the message is what the user has to fix.
    crate::update::commit_editor::committed(state);
    let mut commands = refresh_scope(state, applied.refresh, next_id);
    // A write that moved `HEAD` lands the cursor on its new row once
    // this refresh's refs and walk are in, exactly as a `--continue`
    // does (UX §3.1, m7-power-tools/03); a put-back on what it restored
    // (m7-power-tools/24).
    match lands {
        Some(Landing::Changes(path)) => {
            crate::update::banner::lands_on_changes(state, &mut commands, path, next_id);
        }
        Some(on) => crate::update::banner::finished_lands_on(state, &commands, on),
        None => {}
    }
    commands
}

/// The fields of [`Operated::Conflicted`], so [`stopped`] can only be
/// handed a stop — the type carries what a silent `else` used to.
pub(super) struct ConflictedAnswer {
    description: String,
    kind: InProgress,
    paths: Vec<PathBuf>,
    stalled: Option<String>,
    entry: graft_core::journal::EntryId,
    rewrite: Option<Result<crate::update::banner::RewriteStop, String>>,
}

/// `operated`'s `Conflicted` answer: git stopped and its state is still
/// there — the stop recorded, the banner focused, everything re-read, and
/// the resolver opened on the first conflicted file when there is one.
pub(super) fn stopped(
    state: &mut AppState,
    answer: ConflictedAnswer,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    let ConflictedAnswer {
        description,
        kind,
        paths,
        stalled,
        entry,
        rewrite,
    } = answer;
    // The answer names the live entry; the journal re-read the
    // refresh asks for will say the same (gap row 505).
    state.live_entries = vec![entry];
    // A state, not a `Status` (m6-conflicts/12): git stopped and its
    // state is still there, so the stop has to outlive the refresh
    // it triggers, every line a later key puts on the status slot,
    // and an `Esc` that dismisses one of them. The footer words it
    // each frame (`banner::stop_reason`) beside the banner's keys —
    // never `esc dismiss`, which read as the way on and dropped the
    // banner's focus (`.scratch/m6-conflicts/scout-stuck-after-merge.md`).
    let stop = crate::update::state::Stop {
        origin: crate::update::relaunch::LiveOrigin::ThisRun,
        again: false,
        kind,
        paths_at_stop: paths.len(),
        rewrite,
    };
    // `Ready` also clears the launch's `Busy`, as the old line did.
    // Why git would not continue (`stalled`) is a real problem, not
    // the stop's state: it stays on the status line, dismissable. A
    // rewrite whose progress could not be read says so (07b).
    state.status = match stalled.as_deref() {
        Some(said) => Status::sticky(stalled_line(&description, kind, said)),
        None => crate::update::banner::progress_unread(&stop).map_or(Status::Ready, Status::sticky),
    };
    // …and the banner takes the keyboard, so `c`/`s`/`a` are under
    // the user's fingers rather than something to go looking for
    // (m5-history-and-ghost-preview/09) — except at a rewrite's `edit`
    // stop, which hands it to the graph (07b).
    state.banner_focus = crate::update::banner::stop_takes_focus(kind, &stop);
    state.stopped = Some(stop);
    // The editor and its draft are left alone, as on a failure: no
    // commit was made, so clearing a message the user typed would be
    // throwing away their words over someone else's conflict.
    // Refs moved and the working tree moved. `Everything` is also
    // the only scope that re-reads `AppState::in_progress`, which is
    // what the commit editor's refusal names.
    let mut commands = refresh_scope(state, Refresh::Everything, next_id);
    // …and, with anything unmerged, lands in the resolver on the
    // first conflicted file (m6-conflicts/18); at an `edit` stop, on
    // `HEAD`'s row (07b).
    crate::update::banner::land_in_resolver(state, &mut commands, &paths, next_id);
    crate::update::banner::edit_stop_lands_on_head(state, &commands);
    commands
}

/// The operation job `id` launched, when it launched one.
pub(super) fn launched_op(state: &AppState, id: JobId) -> Option<Operation> {
    state
        .remote
        .running
        .as_ref()
        .filter(|running| running.id == id)
        .and_then(|running| running.op.clone())
}

/// Where write `id`'s answer lands, spent: what its launch said
/// ([`landing_of`]), when the launch was this write's.
pub(super) fn owed_landing(state: &mut AppState, id: JobId) -> Option<Landing> {
    state
        .lands
        .take()
        .and_then(|(launched, on)| (launched == id).then_some(on))
}

/// `operated`'s `StagingNotKept` answer: it ran and was read back; only the
/// staging did not come back (30b). Everything a `Done` does, with its own
/// sticky line.
fn staging_not_kept_answered(
    state: &mut AppState,
    launched_op: Option<&Operation>,
    remote_op: Option<&Operation>,
    applied: &graft_ops::apply::Applied,
    paths: &[PathBuf],
    lands: Option<Landing>,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    let what = launched_op
        .and_then(crate::update::switch::landed_words)
        .unwrap_or_else(|| done_what(state, remote_op, applied));
    let status = staging_not_kept_status(
        &what,
        paths,
        &applied.came_back,
        state.size.columns,
        &state.keymap,
    );
    let mut commands = landed(state, applied, lands, next_id);
    crate::update::recover::tag_rows(state, launched_op);
    // A put-in landed all the same: its commits are tagged and its
    // words spent (review m7-09-to-21c, Minor 1). Its `Done` line
    // gives way to the staging failure, which must be seen.
    crate::update::amend_into::landed(state, launched_op, &mut commands, next_id);
    state.status = status;
    commands
}

/// The drop a stash clash's close launched (m6-conflicts/72b), when job `id`
/// is it: its `Done` is worded as the end of the clash.
fn clash_drop_of(
    state: &mut AppState,
    id: JobId,
) -> Option<(Option<usize>, graft_core::journal::PutBackWhy)> {
    state
        .clash_drop
        .take_if(|(drop, _, _)| *drop == id)
        .map(|(_, index, why)| (index, why))
}

/// `operated`'s `Done` answer: the line `words`, then the landing, a
/// put-in's commits tagged and recovered rows marked. A pull that brought
/// nothing lands nowhere (m8-polish-and-ship/13, ruling 1): the cursor
/// stays where he left it, and `done_status` says why nothing moved.
fn done_answered(
    state: &mut AppState,
    words: Status,
    launched_op: Option<&Operation>,
    applied: &graft_ops::apply::Applied,
    lands: Option<Landing>,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    let lands = lands.filter(|_| !pulled_nothing(launched_op, &applied.entry));
    state.status = words;
    let mut commands = landed(state, applied, lands, next_id);
    crate::update::amend_into::landed(state, launched_op, &mut commands, next_id);
    crate::update::recover::tag_rows(state, launched_op);
    commands
}

/// What a pull that brought nothing says before its tick.
pub(super) const UP_TO_DATE: &str = "up to date";

/// A pull whose entry reads the same before and after on every field it
/// recorded — `HEAD`, the refs, the index, the working tree. A pull never
/// records its tracking refs (`graft_ops::apply::pull::fetch_first`), so a
/// fetch that brought commits nobody merged still counts as nothing. An
/// entry with no `head_after` has not said `HEAD` stayed: it lands as ever.
fn pulled_nothing(
    launched_op: Option<&Operation>,
    entry: &graft_core::journal::JournalEntry,
) -> bool {
    matches!(launched_op, Some(Operation::Pull(_)))
        && entry.head_after.as_ref() == Some(&entry.head_before)
        && entry.refs_before == entry.refs_after
        && entry.index_before == entry.index_after
        && entry.worktree_before == entry.worktree_after
}

/// An operate job answered. Stale answers (a job that is not the one
/// `AppState::operating` is waiting for) are dropped. `Operated::Incomplete`
/// still refreshes — the write happened even though part of its after-state
/// could not be read (`ApplyError::AfterState`'s contract,
/// `.scratch/m3-working-tree-undo-and-stash/as-built.md`).
pub fn operated(
    state: &mut AppState,
    id: JobId,
    outcome: Operated,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    if state.operating != Some(id) {
        return Vec::new();
    }
    let done = matches!(outcome, Operated::Done(_));
    let commands = operated_answer(state, id, outcome, next_id);
    // Last: a `q y` waiting on this write quits only once the answer is in
    // place, and only on `Done` (m8/13b).
    crate::update::quit::write_ended(state, id, done);
    commands
}

/// [`operated`]'s answer, for the write `AppState::operating` waits on.
fn operated_answer(
    state: &mut AppState,
    id: JobId,
    outcome: Operated,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    // The rewrite editor's `y` (m7-power-tools/08, ruling 6): a refusal
    // brings the editor back with the words, any other answer ends it.
    // After the guard above: only the write it launched is its answer.
    crate::update::rewrite_editor::write_answered(state, &outcome);
    state.operating = None;
    let lands = owed_landing(state, id);
    // A write that finished before `Esc`'s stop reached it answers here as
    // usual; the cancel it outran is over.
    state.remote.cancelling = None;
    let launched_op = launched_op(state, id);
    let remote_op = launched_op.clone().filter(Operation::is_remote);
    let clash_drop = clash_drop_of(state, id);
    let (launched, remote) = (launched_op.as_ref(), remote_op.as_ref());
    match outcome {
        Operated::Done(applied) => {
            let words = done_status(state, remote, launched, clash_drop, &applied);
            done_answered(state, words, launched, &applied, lands, next_id)
        }
        // It ran and was read back; only the staging did not come back
        // (30b). Everything a `Done` does, with its own sticky line.
        Operated::StagingNotKept { applied, paths } => staging_not_kept_answered(
            state,
            launched_op.as_ref(),
            remote_op.as_ref(),
            &applied,
            &paths,
            lands,
            next_id,
        ),
        Operated::Incomplete { applied, error } => incomplete_answered(
            state,
            launched_op.as_ref(),
            &applied,
            &error,
            lands,
            next_id,
        ),
        Operated::Conflicted {
            description,
            kind,
            paths,
            stalled,
            entry,
            rewrite,
        } => {
            let answer = ConflictedAnswer {
                description,
                kind,
                paths,
                stalled,
                entry,
                rewrite,
            };
            stopped(state, answer, next_id)
        }
        // A stash put back clashed: the resolver on the first clashed file,
        // no banner (m6-conflicts/72b).
        Operated::Clashed { entry, clash } => {
            crate::update::clash::clashed(state, launched_op.as_ref(), entry, &clash, next_id)
        }
        failed @ Operated::Failed { .. } => {
            operation_failed(state, launched_op.as_ref(), failed, next_id)
        }
    }
}
