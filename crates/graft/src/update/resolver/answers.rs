//! The resolver's jobs answering: the opening read, a take, `$EDITOR` or a
//! mergetool handing the terminal back, and `⏎`'s stage landing. Each one
//! is matched against the id the resolver is waiting on, so a withdrawn
//! question's answer is dropped.

use std::sync::Arc;

use graft_core::job::JobId;
use graft_git::conflict::{BlockState, ConflictFile};

use super::done::{done_arrived, done_unreadable};
use super::files::{RowState, listed_files, remember, reread_set};
use super::take::all_taken;
use super::{WholeFileShape, next_block, reopen, scrolled_to, side_word};
use crate::update::events::{Command, TakeRequest};
use crate::update::state::{AppState, Status};

/// The opening read answered.
pub fn conflict_opened(
    state: &mut AppState,
    id: JobId,
    result: Result<crate::jobs::conflict::Opening, crate::jobs::conflict::OpenFailure>,
) {
    use crate::jobs::conflict::Opening;
    let Some(open) = state.resolver.as_ref() else {
        return;
    };
    if open.job != Some(id) {
        return;
    }
    // A conflicted document on screen, and git no longer calls its path
    // unmerged: `$EDITOR` or a mergetool resolved and staged it while the
    // resolver waited. The route (m6-conflicts/25) answers that with the
    // done side; it means what `not_unmerged` meant before the route
    // existed, so it keeps its words and closes. A done file re-read, or a
    // done row opened from the list, has no unmerged document here.
    let resolved_outside = open.file.is_some() && open.done.is_none();
    match result {
        Ok(Opening::Unmerged(opened)) => conflict_arrived(state, &opened),
        Ok(Opening::Done(_)) if resolved_outside => closed_resolved_outside(state),
        Ok(Opening::Done(done)) => done_arrived(state, &done),
        // The done route's own failure keeps the resolver open on the path
        // (m6-conflicts/25): the list it was clicked in stays.
        Err(failure) if failure.done => done_unreadable(state, &failure.words),
        Err(failure) => open_failed(state, &failure),
    }
}

/// The document arrived.
pub(super) fn conflict_arrived(state: &mut AppState, opened: &crate::jobs::conflict::OpenedFile) {
    // A conflicted path is not done, whatever this resolver showed of it
    // before (a `d` on a done file brings its conflict back).
    if let Some(open) = state.resolver.as_mut() {
        open.done = None;
    }
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
        open.notice = landing_notice(open.notice.take());
        remember(open);
        return;
    };
    let land_last = state.resolver.as_ref().is_some_and(|open| open.land_last);
    // A fresh read has every block open (a take lives only in memory until
    // written, and the file on disk has it collapsed), so its first block
    // is its first open one. `N` crossed here → its last.
    let cursor_block = next_block(&file, None, !land_last);
    let empty = file.blocks().is_empty();
    let Some(open) = state.resolver.as_mut() else {
        return;
    };
    open.land_last = false;
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
    let landing = landing_notice(open.notice.take());
    open.notice = empty
        .then(|| {
            (
                "no conflict markers · ⏎ marks it resolved".to_owned(),
                false,
            )
        })
        .or(landing);
    remember(open);
}

/// What of the notice set before a read survives its landing: the words a
/// finished file left (`gone.txt done ✓ · ▸ notes.txt`), which are not a
/// problem. A problem's words were about the document the read replaces.
fn landing_notice(before: Option<(String, bool)>) -> Option<(String, bool)> {
    before.filter(|(_, problem)| !problem)
}

/// The opening read failed, or refused.
pub(super) fn open_failed(state: &mut AppState, failure: &crate::jobs::conflict::OpenFailure) {
    let Some(open) = state.resolver.as_ref() else {
        return;
    };
    // Already had a document, and git no longer calls the path unmerged:
    // the user resolved it in `$EDITOR` or a mergetool while the resolver
    // was waiting for them. That is a finished job, not a failure.
    // (Since m6-conflicts/25 this is reached only in the race where the path
    // is staged between the job's status read and its open.)
    if open.file.is_some() && failure.not_unmerged {
        closed_resolved_outside(state);
        return;
    }
    state.resolver = None;
    state.status = Status::Sticky {
        reason: failure.words.clone(),
        keys: Some("e edit  M mergetool".to_owned()),
    };
}

/// The open path was resolved outside the app: closed, and said so.
fn closed_resolved_outside(state: &mut AppState) {
    let Some(open) = state.resolver.take() else {
        return;
    };
    state.status = Status::Done(format!("{} resolved outside", open.path.display()));
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
        Ok(file) => take_landed(state, &file, commands, next_id),
        Err(failure) => {
            let mut words = failure.words.clone();
            if let Some(open) = state.resolver.as_mut() {
                open.job = None;
                // The queue was chosen against a document that did not
                // land: every take behind the failed one is dropped, and
                // the line says how many.
                let dropped = open.drop_takes();
                if dropped > 0 {
                    words = format!("{words} · {dropped} queued takes dropped");
                }
                open.notice = Some((words, true));
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

/// The document came back with one more block taken: the next queued take
/// is sent against it in the same update, so the queue is never waiting
/// with nothing in flight. The cursor stays wherever the keys put it.
pub(super) fn take_landed(
    state: &mut AppState,
    file: &Arc<ConflictFile>,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let blocks = file.blocks();
    let open_left = file.open();
    let total = blocks.len();
    let landed = state
        .resolver
        .as_ref()
        .and_then(|open| open.in_flight)
        .map(|(block, _)| block);
    let side = landed
        .and_then(|block| blocks.get(block))
        .and_then(|at| match at.state {
            BlockState::Took(take) => Some(side_word(state, take)),
            BlockState::Open => None,
        });
    let Some(open) = state.resolver.as_mut() else {
        return;
    };
    let path = open.path.clone();
    open.job = None;
    open.in_flight = None;
    open.file = Some(Arc::clone(file));
    remember(open);
    if let Some((block, take)) = open.queued.pop_front() {
        let id = next_id();
        open.job = Some(id);
        open.reading = false;
        open.in_flight = Some((block, take));
        commands.push(Command::TakeSide(
            id,
            TakeRequest {
                path: path.clone(),
                file: Arc::clone(file),
                block,
                take,
            },
        ));
    }
    open.notice = (open_left == 0 && open.in_flight.is_none()).then(|| (all_taken(&path), false));
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
        // The re-read replaces the document every take in flight or queued
        // was chosen against; their answers are withdrawn with it, and the
        // line says how many queued ones went.
        let dropped = open.drop_takes();
        // A base load still in flight carries the document from before the
        // hand-over; the re-read owns the document now, so its answer is
        // withdrawn rather than left to land over the fresh one (m6-05
        // review, Major 2).
        open.base_job = None;
        open.notice = (dropped > 0).then(|| (format!("{dropped} queued takes dropped"), false));
    }
    commands.push(Command::OpenConflict(id, path));
    // The `CONFLICTS` list behind the resolver is as stale as the document
    // was. `M` already did this; `e` never did (m6-conflicts/04).
    let tree = next_id();
    commands.push(crate::update::status_read::ask(state, tree));
}

/// The `Stage` that `⏎` launched landed: on to the next file not done, in
/// the list's order, or out of the resolver altogether.
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
    // The write has landed. Cleared here, not left to the fresh resolver
    // `reopen` builds: a refused `reopen` keeps this one, and a stale id
    // would refuse every later switch as "a write is already running".
    if let Some(open) = state.resolver.as_mut() {
        open.staging = None;
    }
    // The working tree behind this has not been re-read yet — the refresh
    // the write asked for is still in flight — so the file just staged is
    // still unmerged in it and is skipped by name.
    //
    // **Stale by one stage at most**: a path staged by an *earlier* `⏎`
    // whose refresh has not yet landed would still be offered. Its read
    // then answers "not conflicted".
    let listed = listed_files(state);
    let at = listed
        .iter()
        .position(|file| file.path == done)
        .unwrap_or(0);
    let next = listed
        .iter()
        .cycle()
        .skip(at + 1)
        .take(listed.len())
        .find(|file| file.path != done && file.state != RowState::Done)
        .map(|file| file.path.clone());
    if let Some(next) = next {
        reopen(state, commands, next.clone(), from, next_id);
        if let Some(open) = state.resolver.as_mut() {
            open.notice = Some((
                format!("{} done ✓ · ▸ {}", done.display(), next.display()),
                false,
            ));
        }
        // The stage changed what git calls unmerged: the list is read again.
        reread_set(state, commands, next_id);
        return;
    }
    state.resolver = None;
    // The last file closes **to the banner** whichever way the resolver
    // was opened (design note, m6-conflicts §resolver keys): the line below
    // offers `c`, and `c` acts only with the banner focused — on the
    // changes pane it opens the commit editor (m6-conflicts/07 F4). `Esc`
    // still returns to where it was opened from; only the finished close
    // differs. Ticket 19 Part B keeps the resolver open instead.
    state.banner_focus = crate::update::banner::banner_view(state).is_some();
    // The status the `Stage` asked for is still out and still calls this
    // file unmerged, so `c` would count it. The line says what `c` says
    // meanwhile, and `status_read::settle` says the rest when it lands
    // (review m6-amend-13, Minor 1).
    state.status = if crate::update::status_read::owed(state) {
        Status::Busy(crate::update::status_read::CHECKING.to_owned())
    } else {
        every_file_done(state)
    };
}

/// A write the resolver launched (`⏎`'s stage, a whole-file `o`/`t`, `d`'s
/// restart) came back **without** running — refused or failed. Its id is
/// released, and the resolver stays where it is: a held id would refuse
/// every later file switch as "a write is already running" (m6-conflicts/25:
/// `d` on a done file of a shape `checkout --merge` cannot restore is
/// refused by `graft_ops`, not before the box).
pub fn write_not_landed(state: &mut AppState, id: JobId) {
    let Some(open) = state.resolver.as_mut() else {
        return;
    };
    if open.staging == Some(id) {
        open.staging = None;
    }
    if open.restarting == Some(id) {
        open.restarting = None;
    }
}

/// `every file done ✓`, and the key that carries the operation on when
/// there is one to carry on and the banner has the keyboard.
pub(crate) fn every_file_done(state: &AppState) -> Status {
    let what = state
        .in_progress
        .as_ref()
        .filter(|_| state.banner_focus)
        .and_then(|detail| match detail.kind {
            graft_git::state::InProgress::Merge => Some("c finishes the merge"),
            graft_git::state::InProgress::Rebase => Some("c continues the rebase"),
            graft_git::state::InProgress::CherryPick => Some("c continues the cherry-pick"),
            graft_git::state::InProgress::Revert => Some("c continues the revert"),
            graft_git::state::InProgress::Bisect => None,
        });
    Status::Done(match what {
        Some(keys) => format!("every file done ✓ · {keys}"),
        // A conflicted `git stash pop` has no operation and so no banner
        // (`KNOWN_GAPS.md` row 229): there is nothing to continue, and a
        // line offering `c` would name a key that does nothing.
        None => "every file done ✓".to_owned(),
    })
}
