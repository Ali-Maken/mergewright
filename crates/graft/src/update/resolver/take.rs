//! `o`/`t`/`b`: take one side of the block under the cursor — or, on a
//! whole-file card, resolve the path whole (`super::whole_file`).

use graft_core::event::KeyCode;
use graft_core::job::JobId;
use graft_git::conflict::Take;
use graft_ops::operation::Side;

use super::whole_file::{whole_file_both_refused, whole_file_take};
use crate::update::events::{Command, TakeRequest};
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
                take_side(state, commands, Take::Ours, next_id);
            }
        }
        KeyCode::Char('t') => {
            if !whole_file_take(state, commands, Side::Theirs, next_id) {
                take_side(state, commands, Take::Theirs, next_id);
            }
        }
        KeyCode::Char('b') => {
            if !whole_file_both_refused(state) {
                take_side(state, commands, Take::Both, next_id);
            }
        }
        _ => unreachable!("act only calls take_key for o/t/b"),
    }
}

/// `o`/`t`/`b`: take one side of the block under the cursor.
pub(super) fn take_side(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    take: Take,
    next_id: &mut impl FnMut() -> JobId,
) {
    let (running, loading_base, cursor, file, path) = {
        let Some(open) = state.resolver.as_ref() else {
            return;
        };
        (
            open.busy_words(),
            open.base_job.is_some(),
            open.cursor_block,
            open.file.clone(),
            open.path.clone(),
        )
    };
    if let Some(words) = running {
        state.status = Status::sticky(words);
        return;
    }
    // The base load carries the document as it stood at `B` and replaces
    // the open one when it lands. A take in between would be written to
    // disk and then overwritten on screen by the pre-take copy — the
    // screen showing the block open while the file has it taken, and the
    // next take refused as an outside edit (m6-05 review, Major 2). So the
    // two are serial: the take waits for the base.
    if loading_base {
        state.status = Status::sticky("still loading the base");
        return;
    }
    let (Some(block), Some(file)) = (cursor, file) else {
        state.status = Status::Sticky {
            reason: "no conflict under the cursor".to_owned(),
            keys: Some("n next".to_owned()),
        };
        return;
    };
    let id = next_id();
    let Some(open) = state.resolver.as_mut() else {
        return;
    };
    open.job = Some(id);
    open.reading = false;
    open.notice = None;
    commands.push(Command::TakeSide(
        id,
        TakeRequest {
            path,
            file,
            block,
            take,
        },
    ));
}
