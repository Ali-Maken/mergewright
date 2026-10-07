//! `o`/`t`/`b`: take one side of the block under the cursor — or, on a
//! whole-file card, resolve the path whole (`super::whole_file`).

use std::path::Path;

use graft_core::event::{KeyCode, KeyPress};
use graft_core::job::JobId;
use graft_git::conflict::{ConflictFile, NoSuchBlock, Take};
use graft_ops::operation::Side;

use super::settle::settle_key;
use super::walk::after_take;
use super::whole_file::{whole_file_both_refused, whole_file_take};
use super::{ResolverState, TAKE_QUEUE_CAP, scrolled_to};
use crate::update::events::{Command, TakeAt, TakeRequest};
use crate::update::state::{AppState, Status};

/// `o`/`t`/`b`: a whole-file card's own take when one is open, else the
/// ordinary per-block take.
pub(super) fn take_key(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    code: KeyCode,
    next_id: &mut impl FnMut() -> JobId,
) {
    match code {
        KeyCode::Char('o') => {
            if !whole_file_take(state, commands, Side::Ours, next_id) {
                take_block(state, commands, Take::Ours, next_id);
            }
        }
        KeyCode::Char('t') => {
            if !whole_file_take(state, commands, Side::Theirs, next_id) {
                take_block(state, commands, Take::Theirs, next_id);
            }
        }
        KeyCode::Char('b') => {
            if !whole_file_both_refused(state) {
                take_block(state, commands, Take::Both, next_id);
            }
        }
        _ => unreachable!("act only calls take_key for o/t/b"),
    }
}

/// `o`/`t`/`b`: take one side of the block under the cursor, and move the
/// cursor on to the next open block **at the keypress** (m6-conflicts/15).
///
/// While a take is in flight the new one is queued behind it rather than
/// refused, so takes typed faster than the disk all land, in order. Any
/// other job in flight — a read, a base load, a git write — still refuses.
fn take_block(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    take: Take,
    next_id: &mut impl FnMut() -> JobId,
) {
    if let Some(words) = take_refusal(state) {
        state.status = Status::sticky(words);
        return;
    }
    let Some(open) = state.resolver.as_ref() else {
        return;
    };
    let (Some(block), Some(pending)) = (open.cursor_block, pending_file(open)) else {
        state.status = Status::Sticky {
            reason: "no conflict under the cursor".to_owned(),
            keys: state
                .keymap
                .key_verb(&[graft_ui::keymap::ActionId("resolver.next_block")], "next"),
        };
        return;
    };
    // What the document will be once this take lands too. A block it
    // lacks means the cursor and the document disagree: said, never
    // swallowed, and nothing sent (rule 3).
    let pending = pending.and_then(|mut file| file.take(block, take).map(|()| file));
    let pending = match pending {
        Ok(file) => file,
        Err(missing) => {
            if let Some(open) = state.resolver.as_mut() {
                open.notice = Some((super::walk::no_block(missing.0), true));
            }
            return;
        }
    };
    take_side(state, commands, TakeAt::Block(block), take, next_id);
    advance(state, &pending, block);
}

/// `Ctrl+O`/`Ctrl+T`: take one side in **every** block still open, in one
/// write (m6-conflicts/16 D3). The cursor stays; the notice is set when the
/// write lands (`answers::taken::take_landed`). A done file was refused before
/// this, in `act`.
pub(super) fn take_all_key(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    take: Take,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(open) = state.resolver.as_ref() else {
        return;
    };
    if open.whole_file.is_some() {
        state.status = Status::sticky(format!(
            "this card is one choice · {}",
            super::names::side_keys(state)
        ));
        return;
    }
    if let Some(words) = take_refusal(state) {
        state.status = Status::sticky(words);
        return;
    }
    let Some(open) = state.resolver.as_ref() else {
        return;
    };
    let pending = match pending_file(open) {
        // Still reading: `take_refusal` answered that; nothing to take.
        None => return,
        Some(Ok(file)) => file,
        Some(Err(missing)) => {
            if let Some(open) = state.resolver.as_mut() {
                open.notice = Some((super::walk::no_block(missing.0), true));
            }
            return;
        }
    };
    if pending.open() == 0 {
        let words = format!("nothing open in {}", open.path.display());
        if let Some(open) = state.resolver.as_mut() {
            open.notice = Some((words, false));
        }
        return;
    }
    take_side(state, commands, TakeAt::Every, take, next_id);
}

/// Send a take whose checks have passed — or queue it behind the take in
/// flight. The one site a `TakeSide` leaves the keys from.
pub(super) fn take_side(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    at: TakeAt,
    take: Take,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(open) = state.resolver.as_mut() else {
        return;
    };
    if open.in_flight.is_some() {
        open.queued.push_back((at, take));
        return;
    }
    let Some(file) = open.file.clone() else {
        return;
    };
    let path = open.path.clone();
    let id = next_id();
    open.job = Some(id);
    open.reading = false;
    open.in_flight = Some((at, take));
    commands.push(Command::TakeSide(
        id,
        TakeRequest {
            path,
            file,
            at,
            take,
        },
    ));
}

/// Why a take may not go ahead now, or `None` when it may — queued behind
/// the take in flight, or sent. Only a take in flight queues: any other job
/// in flight keeps its own words.
pub(super) fn take_refusal(state: &AppState) -> Option<&'static str> {
    let open = state.resolver.as_ref()?;
    // An `O`/`T` is waiting for its Stage: nothing more joins the queue
    // behind it (m6-conflicts/17 D1 step 3).
    if open.settling.is_some() {
        return Some("still writing the last take");
    }
    if open.in_flight.is_some() {
        return (open.queued.len() >= TAKE_QUEUE_CAP).then_some("still writing the last take");
    }
    if let Some(words) = super::write_refusal(state) {
        return Some(words);
    }
    // The base load carries the document as it stood at `B` and replaces
    // the open one when it lands. A take in between would be written to
    // disk and then overwritten on screen by the pre-take copy — the
    // screen showing the block open while the file has it taken, and the
    // next take refused as an outside edit (m6-05 review, Major 2). So the
    // two are serial: the take waits.
    open.base_job.is_some().then_some("still loading the base")
}

/// The cursor after a take of `taken`: on to the next open block of
/// `pending`, scrolled into view — or, with none open, left where it is
/// and the notice says what is left to do.
fn advance(state: &mut AppState, pending: &ConflictFile, taken: usize) {
    let next = after_take(pending, taken);
    let top = next.map(|block| scrolled_to(state, block));
    let keys = std::sync::Arc::clone(&state.keymap);
    let Some(open) = state.resolver.as_mut() else {
        return;
    };
    match (next, top) {
        (Some(block), Some(top)) => {
            open.cursor_block = Some(block);
            open.top = top;
            open.notice = None;
        }
        _ => open.notice = Some((all_taken(&open.path, &keys), false)),
    }
}

/// The notice once every block is taken: `⏎` from his keymap (m8/10g),
/// left out when unbound.
pub(super) fn all_taken(path: &Path, keys: &graft_ui::keymap::Keymap) -> String {
    keys.after(
        "all blocks taken",
        graft_ui::keymap::ActionId("resolver.mark_done"),
        &format!("marks {} done", path.display()),
    )
}

/// The document as it will stand once every take sent or queued has
/// landed: `file`, then the take in flight, then the queue, in order — a
/// `TakeAt::Every` entry replayed as `take_all`.
/// `None` while no document is open; `Err` names a block the document
/// lacks.
pub(super) fn pending_file(resolver: &ResolverState) -> Option<Result<ConflictFile, NoSuchBlock>> {
    let mut file = (**resolver.file.as_ref()?).clone();
    for (at, take) in resolver.in_flight.iter().chain(resolver.queued.iter()) {
        match at {
            TakeAt::Block(block) => {
                if let Err(missing) = file.take(*block, *take) {
                    return Some(Err(missing));
                }
            }
            TakeAt::Every => {
                file.take_all(*take);
            }
        }
    }
    Some(Ok(file))
}

/// The keys that take a side: `o`/`t`/`b`, `Ctrl+O`/`Ctrl+T` (every open
/// block, m6-conflicts/16) and `O`/`T` (the whole file, then the Stage —
/// m6-conflicts/17, keymap §2.1's named exception). `true` when `key` was
/// one of them. `Ctrl+Alt+…` is not, and is swallowed by `act`.
pub(super) fn take_keys(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    let plain = !key.modifiers.control && !key.modifiers.alt;
    let bulk = key.modifiers.control && !key.modifiers.alt;
    match key.code {
        KeyCode::Char('o' | 't' | 'b') if plain => take_key(state, commands, key.code, next_id),
        KeyCode::Char('o') if bulk => take_all_key(state, commands, Take::Ours, next_id),
        KeyCode::Char('t') if bulk => take_all_key(state, commands, Take::Theirs, next_id),
        KeyCode::Char('O') if plain => settle_key(state, commands, Take::Ours, next_id),
        KeyCode::Char('T') if plain => settle_key(state, commands, Take::Theirs, next_id),
        _ => return false,
    }
    true
}
