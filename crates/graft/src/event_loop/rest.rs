//! When the cursor has stopped (G-settle, m8-polish-and-ship/03).
//!
//! Per-row work — the detail pane's file list, its "contained in" query,
//! the evil-merge probe — is worth starting only for the row he stops on.
//! While he holds `j` it is thrown away every repeat. So the first move
//! follows at once (a single `j`, a click, a `G` are unchanged), and a move
//! that comes within [`BURST_GAP`] of the one before it is **deferred**:
//! `update::settle` records the follow as owed and starts nothing. Once the
//! moves have stopped for a quiet window the loop applies
//! `AppEvent::Rested`, which follows the cursor's row once.
//!
//! Rest is **time since the last motion**, not "a frame drawn with nothing
//! queued": a paced frame takes a few ms and key repeats come every
//! 15–90 ms, so the loop goes idle between every repeat. The quiet window is
//! `2 ×` the widest of the burst's last four gaps — so a held key's own
//! rhythm never reads as rest — clamped to [`QUIET_MIN`]..[`BURST_GAP`].
//! Why 150 ms: macOS and X repeat every 15–90 ms, and their initial repeat
//! delay is 225 ms or more, so 150 ms separates a burst from separate
//! presses. The 50 ms floor: keys applied together in one paced turn
//! measure a gap near 0.
//!
//! Like `Pacer`, `Rest` never reads the clock: the loop passes `now`.

use std::time::{Duration, Instant};

use graft_core::event::{Event, KeyPress, MouseKind};

use super::Wake;
use crate::update::AppEvent;

/// Motions closer together than this are one burst.
pub(crate) const BURST_GAP: Duration = Duration::from_millis(150);

/// The shortest quiet window that counts as rest.
pub(crate) const QUIET_MIN: Duration = Duration::from_millis(50);

/// How many of the burst's gaps the quiet window looks at.
const GAPS: usize = 4;

/// What moved the cursor, as far as ending a burst is concerned: the key
/// held (its repeats keep the burst going), or anything else.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mover {
    /// A key press or repeat.
    Key(KeyPress),
    /// A click, a job's answer (a jump's rows arriving, an undo landing).
    Other,
}

impl Mover {
    /// The mover a wake would be, read before the loop takes the wake.
    pub(crate) fn of(wake: &Wake) -> Mover {
        match wake {
            Wake::Event(AppEvent::Core(Event::Key(press))) => Mover::Key(*press),
            _ => Mover::Other,
        }
    }
}

/// The loop's memory of the current burst of motions.
#[derive(Debug, Default)]
pub(crate) struct Rest {
    /// When the last motion was applied; `None` before any, and after rest.
    last: Option<Instant>,
    /// The burst's last [`GAPS`] gaps between motions, newest last.
    gaps: Vec<Duration>,
    /// What made the burst's latest motion.
    key: Option<Mover>,
}

impl Rest {
    pub(crate) fn new() -> Rest {
        Rest::default()
    }

    /// Is a motion at `now` part of a burst — within [`BURST_GAP`] of the
    /// last one? The loop writes this into `AppState::defer_follow` before
    /// each update.
    pub(crate) fn in_burst(&self, now: Instant) -> bool {
        self.last
            .is_some_and(|last| now.saturating_duration_since(last) < BURST_GAP)
    }

    /// The wake just applied, made by `by`, changed what the pane follows.
    pub(crate) fn moved(&mut self, now: Instant, by: Mover) {
        if self.in_burst(now) {
            if let Some(last) = self.last {
                if self.gaps.len() == GAPS {
                    self.gaps.remove(0);
                }
                self.gaps.push(now.saturating_duration_since(last));
            }
        } else {
            self.gaps.clear();
        }
        self.last = Some(now);
        self.key = Some(by);
    }

    /// Does `wake` end the burst before it is applied? An input that is not
    /// the burst's own key — `⏎`, `Tab`, a click, `x` — acts on the
    /// cursor's commit, so the pane must follow it first. The held key's
    /// repeats, job answers, the wheel and hovering do not.
    pub(crate) fn rest_first(&self, wake: &Wake) -> bool {
        let Wake::Event(AppEvent::Core(event)) = wake else {
            return false;
        };
        match event {
            Event::Key(press) => self.key != Some(Mover::Key(*press)),
            Event::Mouse(mouse) => matches!(
                mouse.kind,
                MouseKind::Down(_) | MouseKind::Up(_) | MouseKind::Drag(_)
            ),
            Event::Paste(_) => true,
            _ => false,
        }
    }

    /// When the burst counts as rested: the last motion plus the quiet
    /// window. `None` before any motion.
    pub(crate) fn deadline(&self) -> Option<Instant> {
        let widest = self.gaps.iter().copied().max().unwrap_or_default();
        let quiet = (widest * 2).clamp(QUIET_MIN, BURST_GAP);
        self.last.map(|last| last + quiet)
    }

    /// The burst is over: the next motion is a leading edge.
    pub(crate) fn rested(&mut self) {
        *self = Rest::default();
    }
}

#[cfg(test)]
mod tests {
    use graft_core::event::{KeyCode, Modifiers, MouseButton, MouseInput};

    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn j() -> KeyPress {
        KeyPress::plain(KeyCode::Char('j'))
    }

    fn key(press: KeyPress) -> Wake {
        Wake::Event(AppEvent::Core(Event::Key(press)))
    }

    fn burst(start: Instant, count: u64, gap: Duration) -> Rest {
        let mut rest = Rest::new();
        for n in 0..count {
            let now = start + gap * u32::try_from(n).expect("small");
            rest.moved(now, Mover::Key(j()));
        }
        rest
    }

    #[test]
    fn a_lone_motion_is_not_in_a_burst() {
        let start = Instant::now();
        let rest = Rest::new();
        assert!(!rest.in_burst(start), "nothing moved yet");
        let rest = burst(start, 1, ms(0));
        assert!(
            !rest.in_burst(start + BURST_GAP),
            "150 ms later is a new press"
        );
        assert!(rest.in_burst(start + ms(149)));
    }

    #[test]
    fn sixty_motions_16_ms_apart_are_a_burst_from_the_second_resting_50_ms_after() {
        let start = Instant::now();
        let mut rest = Rest::new();
        for n in 0..60u32 {
            let now = start + ms(16) * n;
            assert_eq!(rest.in_burst(now), n > 0, "motion {n}");
            rest.moved(now, Mover::Key(j()));
        }
        let last = start + ms(16) * 59;
        assert_eq!(
            rest.deadline(),
            Some(last + ms(50)),
            "2 × 16 ms, floored at 50"
        );
    }

    #[test]
    fn gaps_of_90_ms_rest_150_ms_after_the_last() {
        let start = Instant::now();
        let rest = burst(start, 5, ms(90));
        assert_eq!(
            rest.deadline(),
            Some(start + ms(90) * 4 + ms(150)),
            "2 × 90 capped"
        );
    }

    #[test]
    fn the_quiet_window_reads_only_the_last_four_gaps() {
        let start = Instant::now();
        let mut rest = Rest::new();
        rest.moved(start, Mover::Key(j()));
        rest.moved(start + ms(100), Mover::Key(j()));
        let mut now = start + ms(100);
        for _ in 0..4 {
            now += ms(20);
            rest.moved(now, Mover::Key(j()));
        }
        assert_eq!(
            rest.deadline(),
            Some(now + ms(50)),
            "the 100 ms gap aged out"
        );
    }

    #[test]
    fn another_input_rests_first_the_bursts_own_key_and_job_wakes_do_not() {
        let rest = burst(Instant::now(), 3, ms(16));
        assert!(!rest.rest_first(&key(j())), "the held key's repeat");
        assert!(rest.rest_first(&key(KeyPress::plain(KeyCode::Enter))));
        assert!(rest.rest_first(&key(KeyPress::plain(KeyCode::Tab))));
        assert!(rest.rest_first(&key(KeyPress {
            code: KeyCode::Char('j'),
            modifiers: Modifiers::CONTROL,
        })));
        let click = MouseInput {
            kind: MouseKind::Down(MouseButton::Left),
            column: 1,
            row: 1,
            modifiers: Modifiers::default(),
        };
        assert!(rest.rest_first(&Wake::Event(AppEvent::Core(Event::Mouse(click)))));
        let wheel = MouseInput {
            kind: MouseKind::ScrollDown,
            ..click
        };
        assert!(!rest.rest_first(&Wake::Event(AppEvent::Core(Event::Mouse(wheel)))));
        let job = Wake::Event(AppEvent::Core(Event::JobFinished(
            graft_core::job::JobId::new(7),
        )));
        assert!(!rest.rest_first(&job), "a job's answer");
        assert!(!rest.rest_first(&Wake::Fence));
    }

    #[test]
    fn rested_makes_the_next_motion_a_leading_edge() {
        let start = Instant::now();
        let mut rest = burst(start, 3, ms(16));
        assert!(rest.in_burst(start + ms(40)));
        rest.rested();
        assert!(!rest.in_burst(start + ms(40)));
        assert_eq!(rest.deadline(), None);
    }
}
