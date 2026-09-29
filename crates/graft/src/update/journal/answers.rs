//! The journal's answers: an event, a landing outside Graft, lost commits, a recover.

use super::*;

/// `Product::Journal` landed. Stale answers are dropped by id, the same way
/// every other `Loading` field drops them.
pub(in crate::update) fn journal_event(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
    product: &Product,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    let Product::Journal(result) = product else {
        return false;
    };
    if state.journal.reading != Some(id) {
        return true;
    }
    // Read before the assignment below: the launch read is the one that
    // finds `entries` anything but `Ready`. See `update::relaunch::journal_read_landed`.
    let first_read = !matches!(state.journal.entries, Loading::Ready(_));
    state.journal.reading = None;
    if let Ok(read) = result {
        // Only a successful read knows what is live. A failed one is left
        // holding the last answer rather than silently claiming "nothing is
        // live", which would put the interrupted notice back on a merge that
        // is standing right there.
        state.live_entries.clone_from(&read.live);
        // Merged, not replaced: see `JournalState::unreadable`.
        for line in &read.unreadable {
            if !state.journal.unreadable.contains(line) {
                state.journal.unreadable.push(line.clone());
            }
        }
        // Merged too: only the read that closed an entry knows where its
        // edits were (`AppState::interrupted_edits`).
        for edits in &read.interrupted_edits {
            if !state.interrupted_edits.iter().any(|(id, _)| *id == edits.0) {
                state.interrupted_edits.push(edits.clone());
            }
        }
    }
    state.journal.entries = match result {
        Ok(read) => Loading::Ready(Arc::clone(read)),
        Err(why) => Loading::Failed(why.clone()),
    };
    // Stage 1: the `●` rows. A shorter journal must not leave the cursor
    // past its end; `rebuild` keeps it on its row or falls to the newest.
    rebuild(state);
    if first_read {
        crate::update::relaunch::journal_read_landed(state, commands, next_id);
    }
    // Whether a stash put back is still clashing is read out of the journal
    // (ADR-0045 §2, m6-conflicts/72b).
    crate::update::clash::refresh_and_land(state, commands, next_id);
    true
}

/// Stage 2 landed: git's reflog records. The `$` rows join the list; the
/// cursor keeps its row. Stale answers are dropped by id.
pub(crate) fn outside_landed(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
    result: &Result<Arc<Vec<OutsideRecord>>, String>,
    next_id: &mut impl FnMut() -> JobId,
) {
    if state.journal.outside != Loading::Pending(id) {
        return;
    }
    match result {
        Ok(records) => {
            state.journal.outside = Loading::Ready(Arc::clone(records));
            // Stage 3 looks for what nothing holds among these records' old
            // values too, so it waits for them.
            let id = next_id();
            state.journal.lost = Loading::Pending(id);
            commands.push(Command::FindLost {
                id,
                outside: Arc::clone(records),
            });
        }
        Err(why) => {
            state.journal.outside = Loading::Failed(why.clone());
            // No scan over records never read: the title says why there
            // are no lost rows rather than claiming there are none.
            state.journal.lost = Loading::Failed(format!("couldn't read git's own records: {why}"));
        }
    }
    rebuild(state);
}

/// Stage 3 landed: the lost commits and edits. The `✗` rows join the list
/// under their events, and the preselect runs (ruling 2026-09-26). A failed
/// scan is said in the title (`LostScan::Failed`), never dropped.
pub(crate) fn lost_landed(
    state: &mut AppState,
    id: JobId,
    result: &Result<Arc<LostFound>, String>,
) {
    if state.journal.lost != Loading::Pending(id) {
        return;
    }
    state.journal.lost = match result {
        Ok(found) => Loading::Ready(Arc::clone(found)),
        Err(why) => Loading::Failed(why.clone()),
    };
    rebuild(state);
}

/// `recover_for`'s answer to a `⏎` on a `✗`, `$` or lost-edit row.
///
/// `Run` opens the put-back's preview **with J left shown** under it, so
/// `Esc` is J on the same row (V3); `y` confirms (ADR-0044 §1).
/// `OfferBranch` opens nothing: the status says why and what `b` makes, and
/// `b` on the row prefills the name. A refusal (`Err`, already worded) is
/// the status line's.
pub(crate) fn recover_answered(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
    result: &Result<Recover, String>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(row) = state
        .journal
        .recovering
        .take_if(|(asked, _)| *asked == id)
        .map(|(_, row)| row)
    else {
        return;
    };
    match result {
        Ok(Recover::Run(params)) => {
            // A refusal to open left its own sentence on the status line,
            // and J stays where it is.
            let _opened: bool = crate::update::preview::open_preview(
                state,
                commands,
                crate::update::preview::PreviewSubject::Operation {
                    op: Box::new(Operation::Recover(params.clone())),
                    opened_by: "\u{23ce}",
                },
                false,
                next_id,
            );
        }
        Ok(Recover::OfferBranch { at, name, why }) => {
            state.status = Status::Sticky {
                reason: why.clone(),
                keys: Some(format!("b makes {name} at {}", crate::update::place::short(*at))),
            };
            state.journal.offered = Some((row, name.clone()));
        }
        Err(why) => state.status = Status::sticky(why.clone()),
    }
}
