//! The working-tree status read the app is waiting on, and the one line
//! that waits for it (review m6-amend-13, Minor 1).
//!
//! A write's answer and the status read it asks for are two jobs. Between
//! them, [`AppState::working_tree`] still holds the status from before the
//! write — after the last file's `Stage` it still calls that file unmerged.
//! Anything that *counts* on the status in that window counts stale
//! entries: `c` once refused with `1 file still conflicted` right under a
//! line saying `every file done ✓ · c finishes the merge`.
//!
//! So every read carries an id `update` chose ([`ask`]), the newest is
//! remembered in [`AppState::status_read`], and while it is out both the
//! line and `c` say [`CHECKING`] — the same words, so they cannot disagree.
//! When it lands, [`settle`] replaces those words with the verdict the
//! fresh status gives.

use graft_core::job::JobId;

use super::events::Command;
use super::state::{AppState, Status};

/// What the line says while the status read is out, and what `c` answers
/// meanwhile. Drawn as a `Busy` line, so it reads `checking the files…`
/// and never as a failure: nothing is wrong, the answer is on its way.
pub(crate) const CHECKING: &str = "checking the files";

/// Ask for the working tree's status under `id`, and remember it as the
/// read the status is now waiting for. Every `ReadWorkingTree` is built
/// here, so [`AppState::status_read`] is always the newest one asked.
pub(super) fn ask(state: &mut AppState, id: JobId) -> Command {
    state.status_read = Some(id);
    Command::ReadWorkingTree(id)
}

/// Whether a read is out, so the status may not have caught up with a
/// write yet.
#[must_use]
pub(crate) fn owed(state: &AppState) -> bool {
    state.status_read.is_some()
}

/// Read `id` ended — with a status, a failure, or no answer at all.
/// `true` when it was the read the status was waiting for; an older one
/// ending changes nothing, because the newer one is still coming and reads
/// the repository later.
pub(super) fn answered(state: &mut AppState, id: JobId) -> bool {
    if state.status_read != Some(id) {
        return false;
    }
    state.status_read = None;
    true
}

/// The owed read has landed and the status is fresh: a line still saying
/// [`CHECKING`] becomes the verdict — every file done and `c` offered, or
/// `c`'s own refusal when something is conflicted after all (an edit from
/// outside). Any other line was put up since, and stays.
///
/// The same for the resolver's notice row, where the words go while the
/// resolver is open after the last file (m6-conflicts/31): a notice still
/// saying [`CHECKING`] gives way — to the finish row when every file is
/// done (m6-conflicts/60: the row says `c` in its own words), or to `c`'s
/// refusal in the notice's words.
pub(super) fn settle(state: &mut AppState) {
    if notice_checking(state) {
        let verdict = super::banner::continue_refusal(state)
            .map(|reason| super::resolver::finish_refused_notice(&reason));
        if let Some(open) = state.resolver.as_mut() {
            open.notice = verdict;
        }
    }
    if !checking(state) {
        return;
    }
    state.status = match super::banner::continue_refusal(state) {
        None => super::resolver::every_file_done(state),
        Some(reason) => super::banner::continue_refused(reason),
    };
}

/// The owed read ended with no status to give: the line — or the
/// resolver's notice — stops saying it is checking. Nothing is claimed
/// either way — the next read decides.
///
/// A finish summary owed by the last file (m6-conflicts/60) is cancelled,
/// never spent: with no status to judge by, opening it could stand on a
/// stale conflict or a stale "all done" (rule 3). The caller puts the
/// failure's own words on the status line.
pub(super) fn abandon(state: &mut AppState) {
    if let Some(open) = state.resolver.as_mut() {
        open.finish_owed = false;
    }
    if notice_checking(state)
        && let Some(open) = state.resolver.as_mut()
    {
        open.notice = None;
    }
    if checking(state) {
        state.status = Status::Ready;
    }
}

fn checking(state: &AppState) -> bool {
    matches!(&state.status, Status::Busy(words) if words == CHECKING)
}

/// The open resolver's notice row says [`CHECKING`]. A notice cleared or
/// replaced meanwhile (a scroll, a take) is not resurrected: the next `c`
/// asks again.
fn notice_checking(state: &AppState) -> bool {
    state
        .resolver
        .as_ref()
        .and_then(|open| open.notice.as_ref())
        .is_some_and(|(words, problem)| words == CHECKING && !problem)
}
