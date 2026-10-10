//! A take answering: the write landed (words, the choice recorded, the
//! next block or the file settled) or did not.

use super::{
    AppState, Arc, BlockState, Command, ConflictFile, JobId, Status, TakeAt, TakeRequest,
    all_taken, remember, scrolled_to, settle_landed, side_word,
};

/// A take answered.
pub fn taken(
    state: &mut AppState,
    id: JobId,
    result: Result<Arc<ConflictFile>, crate::jobs::conflict::TakeFailure>,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(open) = state.resolver.as_ref() else {
        return;
    };
    if open.job != Some(id) {
        return;
    }
    let path = open.path.clone();
    match result {
        Ok(file) => take_landed(state, &file, commands, next_id),
        Err(failure) => {
            let mut words = failure.words.clone();
            let mut dropped = 0;
            if let Some(open) = state.resolver.as_mut() {
                open.job = None;
                // The queue was chosen against a document that did not
                // land: every take behind the failed one is dropped, and
                // the line says how many.
                let settling = open.settling.is_some();
                dropped = open.drop_takes();
                if dropped > 0 {
                    words = format!("{words} · {dropped} queued takes dropped");
                }
                // An `O`/`T` was waiting on this: nothing is staged, and
                // the line says so (m6-conflicts/17 D3).
                if settling {
                    words.push_str(" · nothing staged");
                }
                open.notice = Some((words, true));
            }
            if failure.reread {
                // Something else wrote the file between the read and the
                // take, so what is on screen is not what is on disk. The
                // document is thrown away and read again rather than
                // written over: an edit made outside Tributary is never
                // silently undone (design note, hard choice 2).
                let next = next_id();
                if let Some(open) = state.resolver.as_mut() {
                    open.job = Some(next);
                    open.reading = true;
                    open.dropped = dropped;
                }
                commands.push(Command::OpenConflict(next, path));
            }
        }
    }
}

/// What landed is recorded for the finish summary's words, against the
/// document the request was applied to — still `open.file` when
/// `take_landed` calls this (m6-conflicts/19 scope 3). `O`/`T` land here
/// too, as `Every`.
pub(super) fn record_landed(state: &mut AppState, file: &ConflictFile) {
    if let Some((path, was, Some((at, take)))) = state
        .resolver
        .as_ref()
        .map(|open| (open.path.clone(), open.file.clone(), open.in_flight))
    {
        crate::update::finish::record_take(state, &path, was.as_deref(), file, at, take);
    }
}

/// What the landing take says: for an `Every` entry its side and how many
/// blocks it took — the blocks the request's document had open, read at
/// landing rather than carried (m6-16 D2): `take_landed` sends each request
/// against the document that landed before it, so that document is the one
/// this write was applied to. For a `Block` entry, the side that block now
/// shows.
pub(super) fn landed_words(
    state: &AppState,
    blocks: &[graft_git::conflict::BlockAt],
) -> (Option<(String, usize)>, Option<String>) {
    let landed = state.resolver.as_ref().and_then(|open| open.in_flight);
    match landed {
        Some((TakeAt::Every, take)) => {
            let before = state
                .resolver
                .as_ref()
                .and_then(|open| open.file.as_ref())
                .map_or(0, |was| was.open());
            (Some((side_word(state, take), before)), None)
        }
        Some((TakeAt::Block(block), _)) => (
            None,
            blocks.get(block).and_then(|at| match at.state {
                BlockState::Took(take) => Some(side_word(state, take)),
                BlockState::Open => None,
            }),
        ),
        None => (None, None),
    }
}

/// The document came back with one more block taken: the next queued take
/// is sent against it in the same update, so the queue is never waiting
/// with nothing in flight. The cursor stays wherever the keys put it.
pub(in crate::update::resolver) fn take_landed(
    state: &mut AppState,
    file: &Arc<ConflictFile>,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let blocks = file.blocks();
    let open_left = file.open();
    let total = blocks.len();
    let (bulk, side) = landed_words(state, &blocks);
    let was_bulk = bulk.is_some();
    record_landed(state, file);
    let keys = Arc::clone(&state.keymap);
    let Some(open) = state.resolver.as_mut() else {
        return;
    };
    let path = open.path.clone();
    open.job = None;
    open.in_flight = None;
    open.file = Some(Arc::clone(file));
    remember(open);
    if let Some((at, take)) = open.queued.pop_front() {
        let id = next_id();
        open.job = Some(id);
        open.reading = false;
        open.in_flight = Some((at, take));
        commands.push(Command::TakeSide(
            id,
            TakeRequest {
                path: path.clone(),
                file: Arc::clone(file),
                at,
                take,
            },
        ));
    }
    let settled = open_left == 0 && open.in_flight.is_none();
    open.notice = landed_notice(&keys, bulk, settled, &path);
    // The new document may draw in a different number of rows: the cursor
    // block, wherever the keys put it, is kept on screen — moved as little
    // as it must, never back to the block that landed.
    if let Some(block) = open.cursor_block {
        let top = scrolled_to(state, block);
        if let Some(open) = state.resolver.as_mut() {
            open.top = top;
        }
    }
    if let Some(side) = side {
        state.status = Status::done(format!(
            "took {side} in {} · {open_left} of {total} open",
            path.display()
        ));
    } else if was_bulk && matches!(state.status, Status::Done { .. }) {
        // An earlier block's `k of m open` must not outlive this landing
        // (review m6-amend-16 M1): the notice carries the bulk take's words,
        // so the footer goes back to the hints rather than a stale count.
        state.status = Status::Ready;
    }
    // The take wrote the file outside any operation, so no refresh runs:
    // the changes pane under the resolver would keep the pre-take diff,
    // markers and all (m6-conflicts/85 B).
    commands.extend(state.detail.file_written(&path, next_id));
    settle_landed(state, open_left, commands, next_id);
}

/// The notice a landed take leaves: a bulk take's count, and once nothing
/// is left open or in flight, `⏎` (his key, m8/10g) to mark the file done.
fn landed_notice(
    keys: &graft_ui::keymap::Keymap,
    bulk: Option<(String, usize)>,
    settled: bool,
    path: &std::path::Path,
) -> Option<(String, bool)> {
    match bulk {
        Some((side, n)) => {
            let blocks = if n == 1 { "block" } else { "blocks" };
            let mut words = format!("took {side} in {n} {blocks}");
            if settled {
                words = keys.after(
                    &words,
                    graft_ui::keymap::ActionId("resolver.mark_done"),
                    &format!("marks {} done", path.display()),
                );
            }
            Some((words, false))
        }
        None => settled.then(|| (all_taken(path, keys), false)),
    }
}
