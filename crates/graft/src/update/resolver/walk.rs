//! `n`/`N`: the next or previous open block, then the next conflicted file.

use std::path::{Path, PathBuf};

use graft_core::job::JobId;
use graft_git::conflict::{BlockState, ConflictFile};

use super::{reopen, scrolled_to};
use crate::update::events::Command;
use crate::update::state::{AppState, Status};

/// `n`/`N`: the next or previous **open** block, then the next conflicted
/// file once this one has none left in that direction.
pub(super) fn walk_blocks(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    forward: bool,
    next_id: &mut impl FnMut() -> JobId,
) {
    let (file, card, cursor, path, from) = {
        let Some(open) = state.resolver.as_ref() else {
            return;
        };
        (
            open.file.clone(),
            open.whole_file.is_some(),
            open.cursor_block,
            open.path.clone(),
            open.opened_from,
        )
    };
    // A whole-file card has no blocks to walk, so it is at its own end
    // straight away and `n` crosses to the next file (m6-05 review, Major
    // 1). With neither a document nor a card the opening read is still in
    // flight, and there is nothing yet to walk from.
    if file.is_none() && !card {
        return;
    }
    if let Some(block) = file
        .as_deref()
        .and_then(|file| next_open_block(file, cursor, forward))
    {
        let top = scrolled_to(state, block);
        let Some(open) = state.resolver.as_mut() else {
            return;
        };
        open.cursor_block = Some(block);
        open.top = top;
        open.notice = None;
        return;
    }
    // Nothing open left this way in this file. Forward, that means the next
    // conflicted file; backward it means staying put, since a walk that
    // only ever goes down has no previous file to go back to.
    if forward && let Some(next) = next_conflicted_file(state, &path) {
        reopen(state, commands, next, from, next_id);
        return;
    }
    state.status = Status::Sticky {
        reason: "no more conflicts".to_owned(),
        // A card is resolved by its own `o`/`t`; it has no `⏎` to offer.
        keys: Some(if card {
            "o ours  t theirs".to_owned()
        } else {
            format!("⏎ marks {} resolved", path.display())
        }),
    };
}

/// The next **open** block from `from`, in `forward`'s direction, or `None`
/// when there is none that way. `from: None` starts outside the file, so it
/// answers the first (or last) open block.
#[must_use]
pub fn next_open_block(file: &ConflictFile, from: Option<usize>, forward: bool) -> Option<usize> {
    let blocks = file.blocks();
    let mut open = blocks
        .iter()
        .filter(|at| at.state == BlockState::Open)
        .map(|at| at.index);
    match (from, forward) {
        (None, true) => open.next(),
        (None, false) => open.next_back(),
        (Some(cursor), true) => open.find(|index| *index > cursor),
        (Some(cursor), false) => open.rfind(|index| *index < cursor),
    }
}

/// The conflicted path after `after` in the working tree's own order, or
/// `None` when that was the last one.
pub(super) fn next_conflicted_file(state: &AppState, after: &Path) -> Option<PathBuf> {
    crate::update::banner::unmerged_paths(state)
        .into_iter()
        .skip_while(|path| path != after)
        .nth(1)
}

/// A conflicted path other than `done` that is still left: the next one
/// after it, or — `done` being the last row — the first one before it.
///
/// Unlike `n`'s [`next_conflicted_file`], which stops at the end of the
/// list, "none left" here must mean none: opened on the last `CONFLICTS`
/// row, `⏎` closing with `all conflicts resolved · c continue` would send
/// the user to a continue git refuses, with earlier rows still unmerged.
///
/// **Stale by one stage at most**: the list is the working tree as last
/// read, and a path staged by an *earlier* `⏎` whose refresh has not yet
/// landed would still be offered. Its read then answers "not conflicted".
pub(super) fn remaining_conflicted_file(state: &AppState, done: &Path) -> Option<PathBuf> {
    next_conflicted_file(state, done).or_else(|| {
        crate::update::banner::unmerged_paths(state)
            .into_iter()
            .find(|path| path != done)
    })
}
