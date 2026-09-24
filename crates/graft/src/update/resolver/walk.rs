//! `n`/`N`: the next or previous block, taken or not, then the next file
//! in the list — and where a take moves the cursor (m6-conflicts/15).
//!
//! Past the last block (for `N`, before the first) one rule decides, by
//! position only (keymap §2.6, the user's call of 2026-09-24):
//! 1. another file in the list not done → cross to it;
//! 2. else this file has an open block → wrap to it;
//! 3. else `no more conflicts · ⏎ marks <file> done`.
//!
//! Crossing first is what keeps `n` from circling a file that still has an
//! open block and never leaving it.

use graft_core::job::JobId;
use graft_git::conflict::{BlockState, ConflictFile};

use super::files::{RowState, listed_files, select_file};
use super::scrolled_to;
use super::take::pending_file;
use crate::update::events::Command;
use crate::update::state::{AppState, Status};

/// `n`/`N`: the next or previous block, then the three rules above.
pub(super) fn walk_blocks(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    forward: bool,
    next_id: &mut impl FnMut() -> JobId,
) {
    let (pending, card, cursor, path) = {
        let Some(open) = state.resolver.as_ref() else {
            return;
        };
        (
            pending_file(open),
            open.whole_file.is_some(),
            open.cursor_block,
            open.path.clone(),
        )
    };
    // With neither a document nor a card the opening read is still in
    // flight, and there is nothing yet to walk from.
    let file = match pending {
        Some(Ok(file)) => Some(file),
        // The document lacks a block a take in flight names: the same
        // words a take gets for it, and no walk over a document that
        // cannot be what is on disk.
        Some(Err(missing)) => {
            if let Some(open) = state.resolver.as_mut() {
                open.notice = Some((no_block(missing.0), true));
            }
            return;
        }
        None if card => None,
        None => return,
    };
    // A whole-file card has no blocks, so it is at its own end straight
    // away: rule 1 only (m6-05 review, Major 1).
    if let Some(block) = file
        .as_ref()
        .and_then(|file| next_block(file, cursor, forward))
    {
        move_cursor(state, block, None);
        return;
    }
    if let Some(index) = cross_target(state, forward) {
        cross(state, commands, index, forward, next_id);
        return;
    }
    if let Some((block, left)) = file.as_ref().and_then(|file| wrap_target(file, forward)) {
        move_cursor(state, block, Some(format!("wrapped · {left} still open")));
        return;
    }
    state.status = Status::Sticky {
        reason: "no more conflicts".to_owned(),
        // A card is resolved by its own `o`/`t`; it has no `⏎` to offer.
        keys: Some(if card {
            super::names::side_keys(state)
        } else {
            format!("⏎ marks {} done", path.display())
        }),
    };
}

/// Rule 2: this file's first (`N`: last) open block, and how many are
/// open, or `None` when none is.
fn wrap_target(file: &ConflictFile, forward: bool) -> Option<(usize, usize)> {
    let open = file
        .blocks()
        .into_iter()
        .filter(|at| at.state == BlockState::Open)
        .map(|at| at.index);
    let block = if forward { open.min() } else { open.max() }?;
    Some((block, file.open()))
}

/// The cursor onto `block`, scrolled into view, with `notice` under the
/// title (`None` clears it).
fn move_cursor(state: &mut AppState, block: usize, notice: Option<String>) {
    let top = scrolled_to(state, block);
    if let Some(open) = state.resolver.as_mut() {
        open.cursor_block = Some(block);
        open.top = top;
        open.notice = notice.map(|words| (words, false));
    }
}

/// Open row `index` of the list: `n` lands on its first block, `N` on its
/// last. A refusal (a take in flight, a box up) is `select_file`'s, in its
/// words, and leaves this file open.
fn cross(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    index: usize,
    forward: bool,
    next_id: &mut impl FnMut() -> JobId,
) {
    let was = state.resolver.as_ref().map(|open| open.path.clone());
    let Some(target) = listed_files(state).into_iter().nth(index) else {
        return;
    };
    let spent = select_file(state, commands, index, next_id);
    debug_assert!(spent, "index came from listed_files");
    let Some(open) = state.resolver.as_mut() else {
        return;
    };
    if Some(&open.path) == was.as_ref() {
        // Refused: the resolver is as it was, and the status says why.
        return;
    }
    open.land_last = !forward;
    open.notice = Some((format!("▸ {}", target.path.display()), false));
}

/// The words a take or a walk gets when the document has no block `index`.
pub(super) fn no_block(index: usize) -> String {
    format!("no block {index} · re-read the file")
}

/// The next block from `from` in `forward`'s direction, **taken or not**,
/// or `None` past the end. `from: None` starts outside the file, so it
/// answers the first (or last) block.
#[must_use]
pub fn next_block(file: &ConflictFile, from: Option<usize>, forward: bool) -> Option<usize> {
    let count = file.blocks().len();
    match (from, forward) {
        (_, _) if count == 0 => None,
        (None, true) => Some(0),
        (None, false) => Some(count - 1),
        (Some(cursor), true) => cursor.checked_add(1).filter(|next| *next < count),
        (Some(cursor), false) => cursor
            .checked_sub(1)
            .map(|previous| previous.min(count - 1)),
    }
}

/// Where the cursor goes after `taken` was taken: the next **open** block
/// after it, wrapping round to the file's start; `None` when none is open.
#[must_use]
pub fn after_take(file: &ConflictFile, taken: usize) -> Option<usize> {
    let open: Vec<usize> = file
        .blocks()
        .into_iter()
        .filter(|at| at.state == BlockState::Open)
        .map(|at| at.index)
        .collect();
    open.iter()
        .find(|index| **index > taken)
        .or_else(|| open.first())
        .copied()
}

/// The `listed_files` index `n` (or `N`) crosses to: the next row not done
/// after the open file's, in the list's order and wrapping — never the
/// open file itself.
pub(super) fn cross_target(state: &AppState, forward: bool) -> Option<usize> {
    let open = state.resolver.as_ref()?;
    let listed = listed_files(state);
    let count = listed.len();
    let at = listed.iter().position(|file| file.path == open.path)?;
    (1..count)
        .map(|step| {
            if forward {
                (at + step) % count
            } else {
                (at + count - step) % count
            }
        })
        .find(|index| listed[*index].state != RowState::Done)
}
