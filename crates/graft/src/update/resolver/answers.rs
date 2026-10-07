//! The resolver's jobs answering: the opening read, a take, `$EDITOR` or a
//! mergetool handing the terminal back, and a write that did not land
//! (`⏎`'s stage landing is `resolved.rs`). Each one
//! is matched against the id the resolver is waiting on, so a withdrawn
//! question's answer is dropped.

use std::sync::Arc;

use graft_core::job::JobId;
use graft_git::conflict::{BlockState, ConflictFile};

use super::done::{done_arrived, done_unreadable};
use super::files::remember;
use super::settle::settle_landed;
use super::take::all_taken;
use super::{WholeFileShape, next_block, scrolled_to, side_word};
use crate::update::events::{Command, TakeAt, TakeRequest};
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
        Ok(Opening::Done(..)) if resolved_outside => closed_resolved_outside(state),
        Ok(Opening::Done(done, start_over)) => done_arrived(state, &done, start_over),
        // The done route's own failure keeps the resolver open on the path
        // (m6-conflicts/25): the list it was clicked in stays.
        Err(failure) => match failure.done.clone() {
            Some(start_over) => done_unreadable(state, &failure.words, start_over),
            None => open_failed(state, &failure),
        },
    }
}

/// The document arrived.
pub(super) fn conflict_arrived(state: &mut AppState, opened: &crate::jobs::conflict::OpenedFile) {
    // A conflicted path is not done, whatever this resolver showed of it
    // before (a `d` on a done file brings its conflict back).
    let keys = Arc::clone(&state.keymap);
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
            // The job asks ops for every `file: None` open; a missing answer
            // is a job bug, and hides `d` rather than offer a key ops might
            // refuse.
            start_over: opened.start_over.clone().unwrap_or_else(|| {
                crate::jobs::conflict::StartOver::Cannot {
                    why: "could not check: not asked".to_owned(),
                }
            }),
        });
        open.cursor_block = None;
        open.top = 0;
        open.base_shown = false;
        let landing = landing_notice(open);
        // A file there but unreadable says so as a problem, over any
        // landing words (`KNOWN_GAPS` :572).
        open.notice = opened
            .unreadable
            .clone()
            .map(|words| (words, true))
            .or(landing);
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
    let landing = landing_notice(open);
    open.notice = empty.then(|| (no_markers(&keys), false)).or(landing);
    remember(open);
}

/// A document with no blocks: `⏎` (his key, m8/10g) marks it resolved.
fn no_markers(keys: &graft_ui::keymap::Keymap) -> String {
    keys.after(
        "no conflict markers",
        graft_ui::keymap::ActionId("resolver.mark_done"),
        "marks it resolved",
    )
}

/// What of the notice set before a read survives its landing: the words a
/// finished file left (`gone.txt done ✓ · ▸ notes.txt`), which are not a
/// problem. A problem's words were about the document the read replaces —
/// but the takes a failed take dropped are the user's own, and their count
/// outlives them (review m6-amend-15 M2).
pub(super) fn landing_notice(open: &mut super::ResolverState) -> Option<(String, bool)> {
    let kept = open.notice.take().filter(|(_, problem)| !problem);
    let dropped = std::mem::take(&mut open.dropped);
    if dropped == 0 {
        return kept;
    }
    let count = format!("{dropped} queued takes dropped");
    Some(match kept {
        Some((words, _)) => (format!("{words} · {count}"), false),
        None => (count, false),
    })
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
    // Spelled for where the close leaves him, read after it (m8/10j ruling
    // 4): a key with no edit or mergetool row there is left out.
    let place = crate::update::edge::context_of(state);
    state.status = Status::Sticky {
        reason: failure.words.clone(),
        keys: crate::update::banner::edit_or_mergetool(place, &state.keymap),
    };
}

/// The open path was resolved outside the app: closed, and said so.
fn closed_resolved_outside(state: &mut AppState) {
    let Some(open) = state.resolver.take() else {
        return;
    };
    state.status = Status::done(format!("{} resolved outside", open.path.display()));
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
    // A write that did not run leaves nothing to finish on (m6-conflicts/60).
    open.finish_owed = false;
    if open.staging == Some(id) {
        open.staging = None;
        open.whole_side = None;
    }
    if open.restarting == Some(id) {
        open.restarting = None;
    }
}

pub(super) mod taken;

pub use taken::taken;
