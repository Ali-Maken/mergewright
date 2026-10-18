//! `⏎`: marking the open file resolved (`git add`), and what happens when
//! that stage lands — on to the next file not done, or, after the last,
//! staying on it: a merge then opens its finish summary on its own, anything
//! else shows the finish row (a stash pop, with no `c`, closes to the graph).

use crate::update::op_keys::OpKeys;
use std::path::{Path, PathBuf};

use mergewright_core::job::JobId;

use super::files::{RowState, listed_files, reread_set};
use super::{reopen, write_refusal};
use crate::update::events::Command;
use crate::update::state::{AppState, Status};

/// `⏎`: mark the file resolved, which is `git add` on it — git's own way of
/// recording a resolution. In a merge, nothing is staged: `⏎` on a solved
/// file goes to the next open clash (ease-pass/05b).
///
/// Refused **in words, without a job**, while any block is still open:
/// `mergewright_ops::conflict::mark_resolved_refusal` reads the document the user
/// is looking at, so the answer arrives in the same keystroke rather than
/// as a `git add` that runs and comes back refused.
pub(super) fn mark_resolved(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let (file, path) = {
        let Some(open) = state.resolver.as_ref() else {
            return;
        };
        (open.file.clone(), open.path.clone())
    };
    let Some(file) = file else {
        return;
    };
    if let Some(refusal) = mergewright_ops::conflict::mark_resolved_refusal(&path, &file) {
        state.status = Status::Sticky {
            reason: refusal.explain(&OpKeys(&state.keymap)),
            keys: state
                .keymap
                .key_verb(&[mergewright_ui::keymap::ActionId("resolver.next_block")], "next"),
        };
        return;
    }
    if let Some(words) = write_refusal(state) {
        state.status = Status::sticky(words);
        return;
    }
    // A merge stages nothing until the review's `y` (ease-pass/05b): a
    // solved file is solved on disk already, so `⏎` only goes on.
    if super::review::in_a_merge(state) {
        super::walk::solved_next(state, commands, next_id);
        return;
    }
    if let Err(busy) = crate::update::operate::launch(
        state,
        commands,
        mergewright_ops::operation::Operation::Stage(mergewright_ops::operation::Target::Paths(vec![path])),
        next_id,
    ) {
        // Unreachable: `write_refusal` returned above. Worded as it is, on
        // the resolver's notice row, and `staging` left alone — the id in
        // `operating` is the other write's, not this one's.
        state.status = Status::sticky(busy.words());
        return;
    }
    let operating = state.operating;
    if let Some(open) = state.resolver.as_mut() {
        open.staging = operating;
        open.whole_side = None;
    }
}

/// The `Stage` that `⏎` launched landed: on to the next file not done, in
/// the list's order, or — none left — stay on this one (a stash pop
/// closes).
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
    let whole = open.whole_side;
    // Staged: the index holds the resolution now, and git's conflict stages
    // for the path are gone, so its picks are over (ease-pass 05). Only
    // here, on the stage landing — a refused or failed one seals nothing.
    state.picks.seal(&done);
    // The write has landed. Cleared here, not left to the fresh resolver
    // `reopen` builds: a refused `reopen` keeps this one, and a stale id
    // would refuse every later switch as "a write is already running".
    if let Some(open) = state.resolver.as_mut() {
        open.staging = None;
        open.whole_side = None;
    }
    if let Some(side) = whole {
        crate::update::finish::record_whole(state, &done, side);
    }
    if let Some(next) = next_not_done(state, &done) {
        to_next(state, commands, &done, next, from, next_id);
    } else if crate::update::banner::banner_view(state).is_none() {
        close_after_last(state, commands, next_id);
    } else {
        stay_after_last(state, commands, &done, from, next_id);
    }
}

/// The next file not done after `done`, in the list's order, wrapping.
///
/// The working tree behind this has not been re-read yet — the refresh the
/// write asked for is still in flight — so the file just staged is still
/// unmerged in it and is skipped by name.
///
/// **Stale by one stage at most**: a path staged by an *earlier* `⏎` whose
/// refresh has not yet landed would still be offered. Its read then answers
/// "not conflicted".
///
/// In a merge a solved file is as far as it goes before the review's `y`
/// (ease-pass/05b), so it is skipped too: on to a clash still open.
fn next_not_done(state: &AppState, done: &Path) -> Option<PathBuf> {
    let merge = super::review::in_a_merge(state);
    let listed = listed_files(state);
    let at = listed
        .iter()
        .position(|file| file.path == done)
        .unwrap_or(0);
    listed
        .iter()
        .cycle()
        .skip(at + 1)
        .take(listed.len())
        .find(|file| {
            file.path != done
                && file.state != RowState::Done
                && !(merge && file.state == RowState::AllTaken)
        })
        .map(|file| file.path.clone())
}

/// On to `next`, saying which file was just done.
fn to_next(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    done: &Path,
    next: PathBuf,
    from: super::OpenedFrom,
    next_id: &mut impl FnMut() -> JobId,
) {
    let words = format!("{} done ✓ · ▸ {}", done.display(), next.display());
    reopen(state, commands, next, from, next_id);
    if let Some(open) = state.resolver.as_mut() {
        open.notice = Some((words, false));
    }
    // The stage changed what git calls unmerged: the list is read again.
    reread_set(state, commands, next_id);
}

/// A conflicted `git stash pop` has no operation, so no banner and no `c`:
/// nothing is left to do here, and the resolver closes to where it was
/// opened from with the words on the status line.
///
/// A clash Mergewright recorded (`AppState::clash`, m6-conflicts/72b) ends here
/// too (ADR-0045 §3): a **pop** or set-aside launches
/// `Operation::DropPutBack` — its answer says `✓ every file done ·
/// stash@{0} dropped  u brings it back` (a set-aside's: `✓ every file
/// done · your edits are back`); an **apply** runs nothing,
/// `✓ every file done · stash@{0} kept`. A shell pop has no record, and
/// closes as it always did.
pub(crate) fn close_after_last(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    state.resolver = None;
    state.banner_focus = false;
    // The stash clash is over with its last file; no pick outlives it.
    state.picks.clear();
    if let Some(clash) = state.clash.as_ref().map(|clash| clash.clash.clone()) {
        if !crate::update::clash::drops(clash.why) {
            state.status = crate::update::clash::kept_status(clash.index);
            return;
        }
        let drop = mergewright_ops::operation::Operation::DropPutBack {
            stash: clash.stash,
            why: clash.why,
        };
        match crate::update::operate::launch(state, commands, drop, next_id) {
            Ok(()) => state.clash_drop = state.operating.map(|id| (id, clash.index, clash.why)),
            // Unreachable: the stage that brought us here has just answered,
            // and nothing else launches in the same update. Worded, never
            // dropped: the stash stays and says why.
            Err(busy) => state.status = Status::sticky(busy.words()),
        }
        return;
    }
    // The status the `Stage` asked for is still out and still calls this
    // file unmerged; `status_read::settle` gives the verdict when it lands
    // (review m6-amend-13, Minor 1).
    state.status = if crate::update::status_read::owed(state) {
        Status::Busy(crate::update::status_read::CHECKING.to_owned())
    } else {
        every_file_done(state)
    };
}

/// Every file is done and an operation waits on `c`. The resolver stays,
/// reopened on the file just staged, which its read now answers as done
/// and shows read-only (m6-conflicts/25).
///
/// Nothing opens on its own (m6-conflicts/60's P11, dropped by
/// ease-pass/05b): a merge's review is `y`; a rebase, cherry-pick or
/// revert's `c` writes, and the finish row offers it.
///
/// The status line names the file and that every file is done, plus the
/// key to press (`… · c continues the rebase`).
fn stay_after_last(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    done: &Path,
    from: super::OpenedFrom,
    next_id: &mut impl FnMut() -> JobId,
) {
    // Only a merge whose last open file was a whole-file card gets here
    // at all — every other merge file stays unmerged until the review.
    let opened = reopen(state, commands, done.to_path_buf(), from, next_id);
    // The stage changed what git calls unmerged: the list is read again.
    reread_set(state, commands, next_id);
    // The status the `Stage` asked for is still out and still calls this
    // file unmerged, so `c` would count it. The notice says what `c` says
    // meanwhile; `status_read::settle` clears it when the read lands, and
    // the finish row (or the summary) takes the row (review m6-amend-13,
    // Minor 1).
    let checking = crate::update::status_read::owed(state)
        .then(|| (crate::update::status_read::CHECKING.to_owned(), false));
    if opened && let Some(open) = state.resolver.as_mut() {
        open.notice = checking;
    }
    if opened {
        state.status = last_done(state, done);
    }
}

/// `✓ <file> done  every file done`, and the key that carries the
/// operation on (m6-conflicts/60). A merge says
/// `✓ <file> done  every clash is solved · y reviews the merge`.
fn last_done(state: &AppState, done: &Path) -> Status {
    let (every, keys) = every_file_done_parts(state);
    // A merge's solved files wait, unstaged, for the review (ease-pass/05b).
    let keys = if super::review::in_a_merge(state) {
        super::review::reviews_the_merge("every clash is solved", &state.keymap)
    } else {
        match keys {
            Some(keys) => format!("{every} · {keys}"),
            None => every.to_owned(),
        }
    };
    Status::Done {
        what: format!("{} done", done.display()),
        also: None,
        keys: Some(keys),
    }
}

/// `every file done`, and the key that carries the operation on when there
/// is one to carry on and `c` is live where the user is: the banner has the
/// keyboard, or the resolver does (m6-conflicts/31). The tick is the
/// renderer's, so the keys stay apart (ticket 41). In the resolver the
/// finish row says it instead (m6-conflicts/60).
pub(crate) fn every_file_done(state: &AppState) -> Status {
    let (what, keys) = every_file_done_parts(state);
    Status::Done {
        what: what.to_owned(),
        also: None,
        keys,
    }
}

const EVERY_FILE_DONE: &str = "every file done";

fn every_file_done_parts(state: &AppState) -> (&'static str, Option<String>) {
    let c_live = crate::update::banner::banner_view(state).is_some()
        && (state.banner_focus || state.resolver.is_some());
    // A conflicted `git stash pop` has no operation and so no banner
    // (`KNOWN_GAPS.md` row 229): there is nothing to continue, and a line
    // offering `c` would name a key that does nothing.
    let keys = state
        .in_progress
        .as_ref()
        .filter(|_| c_live)
        .and_then(|detail| crate::update::banner::continue_words(detail.kind));
    (EVERY_FILE_DONE, keys)
}

/// Put `notice` on the open resolver's notice row; nothing when it is
/// closed.
pub(crate) fn set_notice(state: &mut AppState, notice: (String, bool)) {
    if let Some(open) = state.resolver.as_mut() {
        open.notice = Some(notice);
    }
}
