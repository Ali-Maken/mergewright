//! The practice tour, inside the repository Graft built for it (ADR-0056 §3,
//! ticket m8-polish-and-ship/19b).
//!
//! The ticks come from the practice repository's own journal, read by a job
//! (`Command::TourProgress`, `graft_ops::tour::tour_progress`): once at
//! arrival, then whenever a write ends — never per frame, and never from
//! what the card says. All eight ticked leaves the tour; so does
//! `tour.leave` (`Esc`) when nothing else on screen takes it. Both are a
//! [`Quit::LeaveTour`] the loop hands to `event_loop::places::drive`, which
//! removes the practice repository and goes back.

use graft_core::job::JobId;
use graft_ui::keymap::{ActionId, KeyChord};

use super::events::Command;
use super::state::{AppState, Quit, Status};

/// The keymap row that leaves the tour (§2.18).
pub(crate) const LEAVE: ActionId = ActionId("tour.leave");

/// The tour's state in the practice repository.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct TourState {
    /// Each task, in `graft_ops::tour::TourTask::ALL` order: done.
    pub(crate) done: [bool; 8],
    /// The progress read out, by id; any other answer is stale.
    pub(crate) asked: Option<JobId>,
    /// A read is owed: at arrival, and after every write that ends.
    pub(crate) wanted: bool,
}

impl TourState {
    /// Just arrived: nothing ticked yet, the first read owed.
    pub(crate) fn arrived() -> Self {
        Self {
            wanted: true,
            ..Self::default()
        }
    }

    /// The first task not done, from 0; `None` when all are.
    pub(crate) fn current(&self) -> Option<usize> {
        self.done.iter().position(|done| !done)
    }
}

/// Rows the card takes above the graph rows: the `tour` band of
/// `graft_ui`'s own carve (`geometry::graph_carve`) — all three while the
/// tour is on and the graph keeps at least one row; else none. Read by
/// `geometry::rows_above_graph` and `AppState::graph_rows`, so `j`/`k` and
/// every click row agree with what is drawn.
#[must_use]
pub(crate) fn card_rows(state: &AppState) -> u16 {
    super::geometry::graph_carve(state).tour.height
}

/// The tail of every `update`: a write that ended (`operating` let go)
/// owes a read; a read owed with none out is asked now.
pub(super) fn request(before: &AppState, next: &mut AppState, commands: &mut Vec<Command>) {
    let ended = before.operating.is_some() && next.operating != before.operating;
    let Some(tour) = next.tour.as_mut() else {
        return;
    };
    if ended {
        tour.wanted = true;
    }
    if tour.wanted && tour.asked.is_none() {
        let id = next.local_jobs.next_id();
        tour.asked = Some(id);
        tour.wanted = false;
        commands.push(Command::TourProgress(id));
    }
}

/// The progress read answered. A stale id is dropped; a failure is said
/// and the ticks kept; all eight leave the tour, finished.
pub(super) fn answered(next: &mut AppState, id: JobId, done: Result<[bool; 8], String>) {
    let Some(tour) = next.tour.as_mut() else {
        return;
    };
    if tour.asked != Some(id) {
        return;
    }
    tour.asked = None;
    match done {
        Ok(done) => {
            tour.done = done;
            if done.iter().all(|done| *done) {
                next.quit = Some(Quit::LeaveTour { finished: true });
            }
        }
        Err(why) => next.status = Status::Failed(format!("tour: {why}")),
    }
}

/// The progress read ended with no answer (failed or cancelled as a job):
/// nothing is out any more, so the next write asks again.
pub(super) fn job_ended(next: &mut AppState, id: JobId) {
    if let Some(tour) = next.tour.as_mut()
        && tour.asked == Some(id)
    {
        tour.asked = None;
    }
}

/// The last link of the graph's `Esc` ladder (`keys::escape::escape_key`):
/// in the tour, with `Esc` still `tour.leave`'s, leave. `true` when it did.
pub(super) fn esc_leaves(next: &mut AppState) -> bool {
    let esc = KeyChord::plain(graft_core::event::KeyCode::Escape);
    if next.tour.is_none() || !next.keymap.chords(&LEAVE).contains(&esc) {
        return false;
    }
    next.quit = Some(Quit::LeaveTour { finished: false });
    true
}

/// `tour.leave` moved off `Esc` by his `[keys]`, pressed on the graph with
/// nothing open (`edge::key`): no graph row takes the chord, so it leaves.
/// `true` when it did.
pub(super) fn moved_leave_key(next: &mut AppState, chord: KeyChord) -> bool {
    use graft_ui::keymap::ContextId;
    if next.tour.is_none()
        || next.keymap.acting(ContextId::Tour, chord) != Some(LEAVE)
        || next.keymap.acting(ContextId::Graph, chord).is_some()
    {
        return false;
    }
    next.quit = Some(Quit::LeaveTour { finished: false });
    true
}

/// The palette's `Practice tour` row: from his repository, leave for a
/// practice one, coming back to the commit under the cursor; inside the
/// tour, say so and how to leave.
pub(crate) fn start_from_palette(state: &mut AppState) {
    if state.tour.is_some() {
        let leave = state
            .keymap
            .key_verb(&[LEAVE], "leaves")
            .map_or_else(String::new, |keys| format!(" \u{b7} {keys}"));
        state.status = Status::Waiting(format!("tour: you're in it{leave}"));
        return;
    }
    let cursor = state
        .history
        .as_ref()
        .and_then(|history| history.selected())
        .map(|row| row.commit.id);
    state.quit = Some(Quit::Practise { cursor });
}

/// After every event while an arrival's landing looked for a commit
/// (`return_to`): once the walk has answered — the cursor on it, or no find
/// still out — settle. Not there: `HEAD`, and a word saying so (ticket
/// m8/19 Amendment §A4).
pub(super) fn check_return(next: &mut AppState, commands: &mut Vec<Command>) {
    let Some(sought) = next.return_to else {
        return;
    };
    if next.land_on_head {
        // The landing has not been spent yet: the reads are still out.
        return;
    }
    let height = next.graph_rows();
    let head = next.refs.as_ref().and_then(|refs| refs.head());
    let awaiting = next.awaiting;
    let Some(history) = next.history.as_mut() else {
        return;
    };
    if history.seeking == Some(sought) {
        return;
    }
    next.return_to = None;
    if history.selected().map(|row| row.commit.id) == Some(sought) {
        return;
    }
    if let Some(head) = head {
        let request = history.seek(head, height);
        if let (Some(request), Some(walk)) = (request, awaiting) {
            commands.push(Command::ShowRows(walk, request));
        }
    }
    next.status = Status::sticky("the commit you were on is gone \u{b7} on HEAD");
}

#[cfg(test)]
mod tests;
