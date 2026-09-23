//! The resolver's jobs answering: the opening read, a take, `$EDITOR` or a
//! mergetool handing the terminal back, and `⏎`'s stage landing. Each one
//! is matched against the id the resolver is waiting on, so a withdrawn
//! question's answer is dropped.

use std::sync::Arc;

use graft_core::job::JobId;
use graft_git::conflict::{BlockState, ConflictFile};

use super::walk::remaining_conflicted_file;
use super::{WholeFileShape, next_open_block, reopen, scrolled_to, side_word};
use crate::update::events::Command;
use crate::update::state::{AppState, Status};

/// The opening read answered.
pub fn conflict_opened(
    state: &mut AppState,
    id: JobId,
    result: Result<Arc<crate::jobs::conflict::OpenedFile>, crate::jobs::conflict::OpenFailure>,
) {
    let Some(open) = state.resolver.as_ref() else {
        return;
    };
    if open.job != Some(id) {
        return;
    }
    match result {
        Ok(opened) => conflict_arrived(state, &opened),
        Err(failure) => open_failed(state, &failure),
    }
}

/// The document arrived.
pub(super) fn conflict_arrived(state: &mut AppState, opened: &crate::jobs::conflict::OpenedFile) {
    let Some(file) = opened.file.clone() else {
        // A whole-file shape (`UD`, `DU`, `AU`, `UA`, `DD`) or a binary
        // `UU`/`AA`: there are no blocks to take sides, so the card offers
        // `o`/`t` over the file itself instead (m6-conflicts/05).
        let Some(open) = state.resolver.as_mut() else {
            return;
        };
        open.job = None;
        open.base_job = None;
        open.how = Some(opened.how);
        open.file = None;
        open.whole_file = Some(WholeFileShape {
            how: opened.how,
            binary: opened.binary,
            on_disk: opened.on_disk.clone(),
        });
        open.cursor_block = None;
        open.top = 0;
        open.base_shown = false;
        open.notice = None;
        return;
    };
    let cursor_block = next_open_block(&file, None, true);
    let empty = file.blocks().is_empty();
    let Some(open) = state.resolver.as_mut() else {
        return;
    };
    open.job = None;
    // A fresh read replaces the document any base load was loading into.
    open.base_job = None;
    open.how = Some(opened.how);
    open.file = Some(Arc::new(file));
    open.whole_file = None;
    open.cursor_block = cursor_block;
    open.top = 0;
    // A fresh read carries no attached bases (`attach_base` runs only from
    // `B`, never from `open_conflict`), and a re-read after `e`/`M` must not
    // leave a stale `true` claiming blocks it has not actually loaded one
    // into (scope 2, "a re-read clears them and `base_shown`").
    open.base_shown = false;
    open.notice = empty.then(|| {
        (
            "no conflict markers · ⏎ marks it resolved".to_owned(),
            false,
        )
    });
}

/// The opening read failed, or refused.
pub(super) fn open_failed(state: &mut AppState, failure: &crate::jobs::conflict::OpenFailure) {
    let Some(open) = state.resolver.as_ref() else {
        return;
    };
    // Already had a document, and git no longer calls the path unmerged:
    // the user resolved it in `$EDITOR` or a mergetool while the resolver
    // was waiting for them. That is a finished job, not a failure.
    let resolved_outside = open.file.is_some() && failure.not_unmerged;
    let path = open.path.clone();
    state.resolver = None;
    state.status = if resolved_outside {
        Status::Done(format!("{} resolved outside", path.display()))
    } else {
        Status::Sticky {
            reason: failure.words.clone(),
            keys: Some("e edit  M mergetool".to_owned()),
        }
    };
}

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
        Ok(file) => take_landed(state, &file),
        Err(failure) => {
            if let Some(open) = state.resolver.as_mut() {
                open.job = None;
                open.notice = Some((failure.words.clone(), true));
            }
            if failure.reread {
                // Something else wrote the file between the read and the
                // take, so what is on screen is not what is on disk. The
                // document is thrown away and read again rather than
                // written over: an edit made outside Graft is never
                // silently undone (design note, hard choice 2).
                let next = next_id();
                if let Some(open) = state.resolver.as_mut() {
                    open.job = Some(next);
                    open.reading = true;
                }
                commands.push(Command::OpenConflict(next, path));
            }
        }
    }
}

/// The document came back with one more block taken.
pub(super) fn take_landed(state: &mut AppState, file: &Arc<ConflictFile>) {
    let blocks = file.blocks();
    let open_left = file.open();
    let total = blocks.len();
    let cursor = state.resolver.as_ref().and_then(|open| open.cursor_block);
    let side = cursor
        .and_then(|block| blocks.get(block))
        .and_then(|at| match at.state {
            BlockState::Took(take) => Some(side_word(take)),
            BlockState::Open => None,
        });
    let Some(open) = state.resolver.as_mut() else {
        return;
    };
    let path = open.path.clone();
    open.job = None;
    open.file = Some(Arc::clone(file));
    open.notice = None;
    // Only now, with the new document in place, so the layout the cursor
    // block is found in is the one the next frame draws.
    if let Some(block) = cursor {
        let top = scrolled_to(state, block);
        if let Some(open) = state.resolver.as_mut() {
            open.top = top;
        }
    }
    if let Some(side) = side {
        state.status = Status::Done(format!(
            "took {side} in {} · {open_left} of {total} open",
            path.display()
        ));
    }
}

/// `$EDITOR` or a mergetool gave the terminal back: **re-read the open
/// file**.
///
/// Both handovers, not one. `M` has always scheduled a `ReadWorkingTree`
/// (`update::mod::mergetool_returned`) and `e` has always scheduled nothing
/// at all, so what the user saw after the two differed for no reason the
/// user could see. Here they are the same: the document on screen is not
/// what is on disk any more, whichever program wrote it, and the file may
/// not even be unmerged now.
pub fn handed_back(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(open) = state.resolver.as_ref() else {
        return;
    };
    let path = open.path.clone();
    let id = next_id();
    if let Some(open) = state.resolver.as_mut() {
        open.job = Some(id);
        open.reading = true;
        // A base load still in flight carries the document from before the
        // hand-over; the re-read owns the document now, so its answer is
        // withdrawn rather than left to land over the fresh one (m6-05
        // review, Major 2).
        open.base_job = None;
        open.notice = None;
    }
    commands.push(Command::OpenConflict(id, path));
    // The `CONFLICTS` list behind the resolver is as stale as the document
    // was. `M` already did this; `e` never did (m6-conflicts/04).
    commands.push(Command::ReadWorkingTree);
}

/// The `Stage` that `⏎` launched landed: on to the next conflicted file, or
/// out of the resolver altogether.
pub fn resolved_advance(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(open) = state.resolver.as_ref() else {
        return;
    };
    if open.staging != Some(id) {
        return;
    }
    let done = open.path.clone();
    let from = open.opened_from;
    // The working tree behind this has not been re-read yet — the refresh
    // the write asked for is still in flight — so the file just staged is
    // still in the list and is skipped by name.
    if let Some(next) = remaining_conflicted_file(state, &done) {
        reopen(state, commands, next, from, next_id);
        return;
    }
    state.resolver = None;
    // The last file closes **to the banner** whichever way the resolver
    // was opened (design note, m6-conflicts §resolver keys): the line below
    // offers `c continue`, and `c` continues only with the banner focused —
    // on the changes pane it opens the commit editor (m6-conflicts/07 F4).
    // `Esc` still returns to where it was opened from; only the finished
    // close differs.
    state.banner_focus = crate::update::banner::banner_view(state).is_some();
    state.status = if state.banner_focus {
        Status::Done("all conflicts resolved · c continue".to_owned())
    } else {
        // A conflicted `git stash pop` has no operation and so no banner
        // (`KNOWN_GAPS.md` row 229): there is nothing to continue, and a
        // line offering `c` would name a key that does nothing.
        Status::Done("all conflicts resolved".to_owned())
    };
}
