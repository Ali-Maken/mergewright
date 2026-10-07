//! The quit wait's clock (m8/13b review fix, keymap §2.2 "a stuck write").
//! `update` holds no time, so the loop counts: from the turn it first sees
//! a wait (`quit::offer_pending`) to [`STOP_OFFER_AFTER`] later, when it
//! calls `quit::waited_long`; and, when the stop box's `y` quits, how long
//! the write had been waited on, for the shell's line.

use std::time::{Duration, Instant};

use graft_core::job::JobId;

use crate::update::AppState;
use crate::update::quit::{STOP_OFFER_AFTER, offer_pending};

/// When the loop first saw the wait it is counting, and for which write.
#[derive(Debug, Default)]
pub(super) struct QuitClock {
    since: Option<(JobId, Instant)>,
}

impl QuitClock {
    /// When `quit::waited_long` is due, or `None` while nothing is counted
    /// toward an offer. Starts the count the first turn a wait names its
    /// write, keeps it while the same write is named, and starts again for
    /// another. Once the offer is made the count is kept (`after` reads it)
    /// but nothing is due; with no wait at all it is dropped.
    pub(super) fn due(&mut self, state: &AppState, now: Instant) -> Option<Instant> {
        let Some(job) = offer_pending(state) else {
            if state.quit_when_done.is_none() {
                self.since = None;
            }
            return None;
        };
        let since = if let Some((counted, since)) = self.since
            && counted == job
        {
            since
        } else {
            self.since = Some((job, now));
            now
        };
        Some(since + STOP_OFFER_AFTER)
    }

    /// How long the write has been waited on, at `now`. `None` when no wait
    /// was counted — the stop box opens only after the offer, which only
    /// this clock makes, so the loop never asks without one.
    pub(super) fn after(&self, now: Instant) -> Option<Duration> {
        self.since
            .map(|(_, since)| now.saturating_duration_since(since))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::update::Status;
    use crate::update::quit::{QuitWait, waited_long};

    fn waiting_on(job: u64) -> AppState {
        let id = JobId::new(job);
        let mut state = AppState {
            operating: Some(id),
            ..AppState::default()
        };
        state.quit_when_done = Some(Box::new(QuitWait {
            job: id,
            line: Status::Waiting("a merge · quitting when it finishes · esc stays".to_owned()),
            long: false,
        }));
        state
    }

    /// The count starts at first sight and holds across later turns.
    #[test]
    fn the_first_sight_holds() {
        let state = waiting_on(1);
        let mut clock = QuitClock::default();
        let start = Instant::now();
        assert_eq!(clock.due(&state, start), Some(start + STOP_OFFER_AFTER));
        let later = start + Duration::from_secs(3);
        assert_eq!(clock.due(&state, later), Some(start + STOP_OFFER_AFTER));
        assert_eq!(clock.after(later), Some(Duration::from_secs(3)));
    }

    /// Nothing is due with no wait, nor once the offer is made; the count
    /// survives the offer (the stop box's `y` reads it) but not the wait.
    #[test]
    fn nothing_due_without_a_wait_or_after_the_offer() {
        let mut clock = QuitClock::default();
        let start = Instant::now();
        assert_eq!(clock.due(&AppState::default(), start), None);
        let mut state = waiting_on(1);
        clock.due(&state, start);
        waited_long(&mut state);
        let at = start + STOP_OFFER_AFTER;
        assert_eq!(clock.due(&state, at), None, "nothing wakes after the offer");
        assert_eq!(clock.after(at), Some(STOP_OFFER_AFTER));
        state.quit_when_done = None;
        assert_eq!(clock.due(&state, at), None);
        assert_eq!(clock.after(at), None, "the wait gone, the count with it");
    }

    /// A wait for another write starts its own count.
    #[test]
    fn another_write_starts_again() {
        let mut clock = QuitClock::default();
        let start = Instant::now();
        clock.due(&waiting_on(1), start);
        let later = start + Duration::from_secs(5);
        assert_eq!(
            clock.due(&waiting_on(2), later),
            Some(later + STOP_OFFER_AFTER)
        );
    }
}
