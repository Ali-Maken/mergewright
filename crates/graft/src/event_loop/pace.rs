//! At most one unanswered frame in the pipe (DESIGN.md §4.7, ADR-0054).
//!
//! Every frame ends with a fence (`CSI 5 n`); the terminal answers `CSI 0 n`
//! once it has read every byte before it. [`Pacer`] says when the loop may
//! write the next frame: once the last fence is answered, or once it has
//! waited [`FENCE_TIMEOUT`] for it. Three timeouts in a row and the terminal
//! is not answering fences at all: fencing stops for the run, and frames are
//! capped at one per [`FRAME_CAP`] instead — the same cap a terminal that
//! never answered the startup DSR gets from the start.
//!
//! The pacer only keeps time; the loop asks it. Wakes that arrive while a
//! frame may not be drawn are applied to state and drawn together by the
//! frame the answer releases.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// How long a fence may go unanswered before the next frame is drawn anyway:
/// the SSH probe budget (DESIGN.md §4.4), the slowest answer Graft waits for
/// anywhere else.
pub(crate) const FENCE_TIMEOUT: Duration = Duration::from_millis(500);

/// Unfenced, frames are at least this far apart: one 60 Hz display frame.
pub(crate) const FRAME_CAP: Duration = Duration::from_millis(16);

/// Timeouts in a row that stop fencing for the run.
const MISSES_TO_GIVE_UP: u8 = 3;

/// When the next frame may be written.
#[derive(Debug)]
pub(crate) struct Pacer {
    /// When each fence still unanswered was written, oldest first. A
    /// terminal answers in order, so an answer is always the oldest one's.
    /// More than one only after a timeout drew over an unanswered fence.
    sent_at: VecDeque<Instant>,
    /// Frames drawn over an unanswered fence since the terminal last caught
    /// up (every fence answered).
    misses: u8,
    /// Frames end with a fence. False from the start for a terminal that
    /// did not answer the startup DSR, and for good after
    /// [`MISSES_TO_GIVE_UP`] timeouts.
    fenced: bool,
    last_drawn: Option<Instant>,
    fence_timeout: Duration,
}

impl Pacer {
    /// `fenced`: the terminal answered the startup DSR
    /// (`Capabilities::answers_dsr`).
    pub(crate) fn new(fenced: bool, fence_timeout: Duration) -> Pacer {
        Pacer {
            sent_at: VecDeque::new(),
            misses: 0,
            fenced,
            last_drawn: None,
            fence_timeout,
        }
    }

    /// Whether the frame about to be drawn ends with a fence.
    pub(crate) fn fenced(&self) -> bool {
        self.fenced
    }

    /// May a frame be written at `now`?
    pub(crate) fn may_draw(&self, now: Instant) -> bool {
        self.deadline().is_none_or(|deadline| now >= deadline)
    }

    /// A frame was written at `now` — with its fence when [`Self::fenced`]
    /// said so. Drawing over an unanswered fence is a timeout; the third in
    /// a row stops fencing (the fence of the frame that gave up is written
    /// but never waited for).
    pub(crate) fn drew(&mut self, now: Instant) {
        self.last_drawn = Some(now);
        if !self.fenced {
            return;
        }
        if !self.sent_at.is_empty() {
            self.misses = self.misses.saturating_add(1);
            if self.misses >= MISSES_TO_GIVE_UP {
                self.fenced = false;
                self.sent_at.clear();
                return;
            }
        }
        self.sent_at.push_back(now);
    }

    /// The terminal answered a fence at `now`. Returns how long that fence
    /// waited for its answer; `None` when no fence was waiting — unfenced,
    /// given up (an answer then never re-enables fencing), or after
    /// [`Self::forget`].
    pub(crate) fn answered(&mut self, now: Instant) -> Option<Duration> {
        if !self.fenced {
            return None;
        }
        let sent = self.sent_at.pop_front()?;
        if self.sent_at.is_empty() {
            // Caught up: every frame written has been read.
            self.misses = 0;
        }
        Some(now.saturating_duration_since(sent))
    }

    /// When a frame that may not be drawn now may be: the newest fence's
    /// timeout, or the cap after the last frame when unfenced. `None` when a
    /// frame may be drawn whenever there is one.
    pub(crate) fn deadline(&self) -> Option<Instant> {
        if self.fenced {
            self.owed()
        } else {
            self.last_drawn.map(|drawn| drawn + FRAME_CAP)
        }
    }

    /// While any fence is unanswered: when the newest one times out.
    pub(crate) fn owed(&self) -> Option<Instant> {
        if self.fenced {
            self.sent_at.back().map(|sent| *sent + self.fence_timeout)
        } else {
            None
        }
    }

    /// The terminal was handed to another program and taken back: whatever
    /// answers were owed may have gone to that program, so none is waited
    /// for. One that does arrive later finds nothing waiting and is ignored.
    pub(crate) fn forget(&mut self) {
        self.sent_at.clear();
        self.misses = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn fenced() -> (Pacer, Instant) {
        (Pacer::new(true, FENCE_TIMEOUT), Instant::now())
    }

    #[test]
    fn the_first_frame_may_always_be_drawn() {
        let now = Instant::now();
        assert!(Pacer::new(true, FENCE_TIMEOUT).may_draw(now));
        assert!(Pacer::new(false, FENCE_TIMEOUT).may_draw(now));
    }

    #[test]
    fn an_answered_fence_lets_the_next_frame_draw_at_once() {
        let (mut pacer, start) = fenced();
        pacer.drew(start);
        assert_eq!(pacer.answered(start + ms(3)), Some(ms(3)));
        assert!(pacer.may_draw(start + ms(3)));
        assert_eq!(pacer.deadline(), None);
    }

    #[test]
    fn an_unanswered_fence_holds_the_next_frame_until_its_timeout() {
        let (mut pacer, start) = fenced();
        pacer.drew(start);
        assert!(!pacer.may_draw(start + ms(1)));
        assert!(!pacer.may_draw(start + ms(499)));
        assert_eq!(pacer.deadline(), Some(start + ms(500)));
        assert!(pacer.may_draw(start + ms(500)));
    }

    #[test]
    fn three_timeouts_in_a_row_stop_fencing_and_cap_frames_at_16_ms() {
        let (mut pacer, start) = fenced();
        pacer.drew(start);
        pacer.drew(start + ms(500)); // miss 1
        assert!(pacer.fenced());
        pacer.drew(start + ms(1000)); // miss 2
        assert!(pacer.fenced());
        let last = start + ms(1500);
        pacer.drew(last); // miss 3: gives up
        assert!(!pacer.fenced());
        assert!(!pacer.may_draw(last + ms(15)), "capped within 16 ms");
        assert_eq!(pacer.deadline(), Some(last + ms(16)));
        assert!(pacer.may_draw(last + ms(16)));
    }

    #[test]
    fn an_answer_after_giving_up_does_not_fence_again() {
        let (mut pacer, start) = fenced();
        for frame in 0..4 {
            pacer.drew(start + ms(500) * frame);
        }
        assert!(!pacer.fenced());
        assert_eq!(pacer.answered(start + ms(2000)), None);
        assert!(!pacer.fenced());
        let last = start + ms(1500);
        assert_eq!(pacer.deadline(), Some(last + FRAME_CAP));
    }

    #[test]
    fn an_answer_in_between_resets_the_run_of_timeouts() {
        let (mut pacer, start) = fenced();
        pacer.drew(start);
        pacer.drew(start + ms(500)); // miss 1, two owed
        pacer.drew(start + ms(1000)); // miss 2, three owed
        for owed in 0..3 {
            pacer.answered(start + ms(1001 + owed));
        }
        pacer.drew(start + ms(1100));
        pacer.drew(start + ms(1600)); // a miss, but the first of a new run
        assert!(pacer.fenced());
    }

    #[test]
    fn a_late_answer_is_the_oldest_fences_not_the_newest() {
        let (mut pacer, start) = fenced();
        pacer.drew(start);
        pacer.drew(start + ms(500)); // timed out over the first fence
        assert_eq!(pacer.answered(start + ms(600)), Some(ms(600)));
        assert!(
            !pacer.may_draw(start + ms(600)),
            "the second frame's fence is still owed"
        );
        assert_eq!(pacer.answered(start + ms(610)), Some(ms(110)));
        assert!(pacer.may_draw(start + ms(610)));
    }

    #[test]
    fn a_silent_terminal_is_capped_from_the_start_and_never_fenced() {
        let mut pacer = Pacer::new(false, FENCE_TIMEOUT);
        let start = Instant::now();
        assert!(!pacer.fenced());
        pacer.drew(start);
        assert!(!pacer.may_draw(start + ms(15)));
        assert!(pacer.may_draw(start + ms(16)));
        assert_eq!(pacer.answered(start + ms(1)), None);
    }

    #[test]
    fn forgetting_after_a_handover_waits_for_nothing() {
        let (mut pacer, start) = fenced();
        pacer.drew(start);
        pacer.forget();
        assert!(pacer.may_draw(start + ms(1)));
        assert_eq!(pacer.answered(start + ms(2)), None);
        assert!(pacer.fenced());
    }
}
