//! `⏎`: marking the open file resolved (`git add`), and what happens when
//! that stage lands — on to the next file not done, or, after the last,
//! staying on it with `every file done ✓ · c …` on the notice row (a stash
//! pop, with no `c`, closes to the graph).

use std::path::{Path, PathBuf};

use graft_core::job::JobId;

use super::files::{RowState, listed_files, reread_set};
use super::{reopen, write_refusal};
use crate::update::events::Command;
use crate::update::state::{AppState, Status};

/// `⏎`: mark the file resolved, which is `git add` on it — git's own way of
/// recording a resolution.
///
/// Refused **in words, without a job**, while any block is still open:
/// `graft_ops::conflict::mark_resolved_refusal` reads the document the user
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
    if let Some(refusal) = graft_ops::conflict::mark_resolved_refusal(&path, &file) {
        state.status = Status::Sticky {
            reason: refusal.explain(),
            keys: Some("n next".to_owned()),
        };
        return;
    }
    if let Some(words) = write_refusal(state) {
        state.status = Status::sticky(words);
        return;
    }
    crate::update::operate::launch(
        state,
        commands,
        graft_ops::operation::Operation::Stage(graft_ops::operation::Target::Paths(vec![path])),
        next_id,
    );
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
        close_after_last(state);
    } else {
        stay_after_last(state, commands, done, from, next_id);
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
fn next_not_done(state: &AppState, done: &Path) -> Option<PathBuf> {
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
        .find(|file| file.path != done && file.state != RowState::Done)
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
/// nothing is left to do here, and the resolver closes to the graph with
/// the words on the status line.
fn close_after_last(state: &mut AppState) {
    state.resolver = None;
    state.banner_focus = false;
    // The status the `Stage` asked for is still out and still calls this
    // file unmerged; `status_read::settle` gives the verdict when it lands
    // (review m6-amend-13, Minor 1).
    state.status = if crate::update::status_read::owed(state) {
        Status::Busy(crate::update::status_read::CHECKING.to_owned())
    } else {
        every_file_done(state)
    };
}

/// Every file is done and an operation waits on `c`: the resolver **stays**
/// (P8, m6-conflicts/31), on the file just staged, which its read now
/// answers as done and shows read-only (m6-conflicts/25). `c` works from
/// here, so the words go on its notice row, not the status line.
fn stay_after_last(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    done: PathBuf,
    from: super::OpenedFrom,
    next_id: &mut impl FnMut() -> JobId,
) {
    reopen(state, commands, done, from, next_id);
    // The stage changed what git calls unmerged: the list is read again.
    reread_set(state, commands, next_id);
    // The status the `Stage` asked for is still out and still calls this
    // file unmerged, so `c` would count it. The notice says what `c` says
    // meanwhile, and `status_read::settle` says the rest when it lands
    // (review m6-amend-13, Minor 1).
    let words = if crate::update::status_read::owed(state) {
        crate::update::status_read::CHECKING.to_owned()
    } else {
        every_file_done_words(state)
    };
    set_notice(state, (words, false));
}

/// `every file done ✓`, and the key that carries the operation on when
/// there is one to carry on and `c` is live where the user is: the banner
/// has the keyboard, or the resolver does (m6-conflicts/31). The resolver's
/// notice row says it in these words; the status line is [`every_file_done`].
pub(crate) fn every_file_done_words(state: &AppState) -> String {
    let (what, keys) = every_file_done_parts(state);
    crate::view::status::done_line(what, keys.as_deref())
}

/// [`every_file_done_words`] as the status line holds it: the tick is the
/// renderer's, so the keys stay apart (ticket 41).
pub(crate) fn every_file_done(state: &AppState) -> Status {
    let (what, keys) = every_file_done_parts(state);
    Status::Done {
        what: what.to_owned(),
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

/// `c`'s refusal as the resolver's notice row says it: the reason, and the
/// key that marks a file done **here** — `⏎` (§2.6), where the banner's
/// line names `space`, the key outside (§2.6a).
pub(crate) fn finish_refused_notice(reason: &str) -> (String, bool) {
    (format!("{reason} · ⏎ marks done"), false)
}

/// Put `notice` on the open resolver's notice row; nothing when it is
/// closed.
pub(crate) fn set_notice(state: &mut AppState, notice: (String, bool)) {
    if let Some(open) = state.resolver.as_mut() {
        open.notice = Some(notice);
    }
}
