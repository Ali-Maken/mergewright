//! `⏎`: marking the open file resolved (`git add`), and what happens when
//! that stage lands — on to the next file not done, or, after the last,
//! staying on it: a merge then opens its finish summary on its own, anything
//! else shows the finish row (a stash pop, with no `c`, closes to the graph).

use std::path::{Path, PathBuf};

use graft_core::event::KeyCode;
use graft_core::job::JobId;
use graft_git::state::InProgress;

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
    if let Err(busy) = crate::update::operate::launch(
        state,
        commands,
        graft_ops::operation::Operation::Stage(graft_ops::operation::Target::Paths(vec![path])),
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
/// nothing is left to do here, and the resolver closes to where it was
/// opened from with the words on the status line.
///
/// A clash Graft recorded (`AppState::clash`, m6-conflicts/72b) ends here
/// too (ADR-0045 §3): a **pop** or set-aside launches
/// `Operation::DropPutBack` — its answer says `every file done ·
/// stash@{0} dropped ✓ · u brings it back`; an **apply** runs nothing,
/// `every file done · stash@{0} kept ✓`. A shell pop has no record, and
/// closes as it always did.
fn close_after_last(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    state.resolver = None;
    state.banner_focus = false;
    if let Some(clash) = state.clash.as_ref().map(|clash| clash.clash.clone()) {
        if !crate::update::clash::drops(clash.why) {
            state.status = crate::update::clash::kept_status(clash.index);
            return;
        }
        let drop = graft_ops::operation::Operation::DropPutBack { stash: clash.stash };
        match crate::update::operate::launch(state, commands, drop, next_id) {
            Ok(()) => state.clash_drop = state.operating.map(|id| (id, clash.index)),
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
/// A **merge** then owes its finish summary (m6-conflicts/60, P11): the
/// flag goes on the state `reopen` just built — set before it, it would be
/// lost with the old one — and [`finish_if_owed`] opens the summary once
/// everything this stage started has landed. A refused `reopen` owes
/// nothing: its words are on the line, and `c` is the way on. A rebase,
/// cherry-pick or revert owes nothing either, since its `c` writes: the
/// finish row offers it instead.
///
/// The status line names the file and that every file is done, plus the
/// key to press wherever no summary is coming (`… · c continues the
/// rebase`).
fn stay_after_last(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    done: &Path,
    from: super::OpenedFrom,
    next_id: &mut impl FnMut() -> JobId,
) {
    let opened = reopen(state, commands, done.to_path_buf(), from, next_id);
    let merge =
        crate::update::banner::live(state).is_some_and(|detail| detail.kind == InProgress::Merge);
    let owes = opened && merge;
    if let Some(open) = state.resolver.as_mut() {
        open.finish_owed = owes;
    }
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
        state.status = last_done(state, done, owes);
    }
}

/// `<file> done ✓ · every file done`, and — where no summary is coming —
/// the key that carries the operation on (m6-conflicts/60).
fn last_done(state: &AppState, done: &Path, summary_coming: bool) -> Status {
    let (every, keys) = every_file_done_parts(state);
    let keys = match keys.filter(|_| !summary_coming) {
        Some(keys) => format!("{every} · {keys}"),
        None => every.to_owned(),
    };
    Status::Done {
        what: format!("{} done", done.display()),
        keys: Some(keys),
    }
}

/// A key that takes him somewhere else cancels the summary the last file
/// owes (m6-conflicts/60): it opens on the event, never later over what he
/// moved on to. A file switch also rebuilds the state, which drops it.
pub(super) fn cancel_owed_finish(state: &mut AppState, code: KeyCode, plain: bool) {
    if plain
        && matches!(
            code,
            KeyCode::Escape
                | KeyCode::Char('j' | 'k' | 'n' | 'N' | 'd' | 'e' | 'M')
                | KeyCode::Down
                | KeyCode::Up
        )
        && let Some(open) = state.resolver.as_mut()
    {
        open.finish_owed = false;
    }
}

/// Spend the owed open (m6-conflicts/60) at the first moment everything the
/// last stage started has landed: no take settling, in flight or queued, no
/// write out, the resolver's own read in, and the status read the stage
/// asked for answered. Then `c`'s one path opens the summary, with the same
/// refusals. A status that still shows a file conflicted drops the owed
/// open instead: `status_read::settle` has already put `c`'s refusal on the
/// notice row.
///
/// State-only, and called once at the tail of every answer
/// (`update::answer`), so it does not matter which of the stage, a take or
/// a read lands last.
pub(crate) fn finish_if_owed(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(open) = state.resolver.as_ref() else {
        return;
    };
    if !open.finish_owed
        || open.settling.is_some()
        || open.in_flight.is_some()
        || !open.queued.is_empty()
        || open.staging.is_some()
        || open.restarting.is_some()
        || open.job.is_some()
        || crate::update::status_read::owed(state)
    {
        return;
    }
    if let Some(open) = state.resolver.as_mut() {
        open.finish_owed = false;
    }
    if crate::update::banner::continue_refusal(state).is_some() {
        return;
    }
    crate::update::banner::continue_pressed(
        state,
        commands,
        crate::update::finish::ReturnTo::Resolver,
        next_id,
    );
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
