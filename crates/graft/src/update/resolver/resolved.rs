//! `⏎`: marking the open file resolved (`git add`), and what happens when
//! that stage lands — on to the next file not done, or out to the banner
//! with `every file done ✓`.

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
