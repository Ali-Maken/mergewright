//! `O`/`T`: settle a whole file in one key (m6-conflicts/17). The press
//! ([`settle_key`]) takes every open block and marks `settling`; the
//! landing ([`settle_landed`]) sends the `Stage` once nothing is in flight.

use mergewright_core::event::KeyCode;
use mergewright_core::job::JobId;
use mergewright_git::conflict::Take;

use super::take::{pending_file, take_key, take_refusal, take_side};
use crate::update::events::{Command, TakeAt};
use crate::update::state::{AppState, Status};

/// `O`/`T`: settle the whole file in one key (m6-conflicts/17 D1) —
/// every open block to `take`, then the `Stage`, then on to the next file
/// (`resolved::resolved_advance`). Not `take_all_key`: that stops at
/// `nothing open`, where this stages. First check that applies answers:
/// a done file was refused in `act`; a card is its own `o`/`t`; then
/// [`take_refusal`]; then take every open block, stage now, or wait for the
/// take in flight.
pub(super) fn settle_key(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    take: Take,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(open) = state.resolver.as_ref() else {
        return;
    };
    if open.whole_file.is_some() {
        let code = match take {
            Take::Ours => KeyCode::Char('o'),
            _ => KeyCode::Char('t'),
        };
        take_key(state, commands, code, next_id);
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
        // Still reading: `take_refusal` answered that.
        None => return,
        Some(Ok(file)) => file,
        Some(Err(missing)) => {
            if let Some(open) = state.resolver.as_mut() {
                open.notice = Some((super::walk::no_block(missing.0), true));
            }
            return;
        }
    };
    let in_flight = open.in_flight.is_some();
    if pending.open() > 0 {
        take_side(state, commands, TakeAt::Every, take, next_id);
    } else if !in_flight {
        // Nothing to take: exactly `⏎`, refusals and words included.
        super::resolved::mark_resolved(state, commands, next_id);
        return;
    }
    // The Stage goes out when the last take lands (`answers::taken::take_landed`).
    if let Some(open) = state.resolver.as_mut() {
        open.settling = Some(take);
    }
}

/// An `O`/`T` was waiting on the take that just landed: with nothing left
/// in flight, send its `Stage` — the same one `⏎` sends, so the advance to
/// the next file is `resolved_advance`'s (m6-conflicts/17 D2).
pub(super) fn settle_landed(
    state: &mut AppState,
    open_left: usize,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(open) = state.resolver.as_mut() else {
        return;
    };
    if open.in_flight.is_some() {
        return;
    }
    let Some(_) = open.settling.take() else {
        return;
    };
    // `open_left > 0` cannot happen — every take ahead of the Stage was in
    // `pending_file` when `O`/`T` saw nothing open past them. Should it,
    // nothing is staged: the blocks still open are on screen.
    if open_left == 0 {
        super::resolved::mark_resolved(state, commands, next_id);
    }
}
