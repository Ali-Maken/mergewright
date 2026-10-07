//! The paced loop (G-pace, ADR-0054, DESIGN.md §4.7): every frame bracketed
//! and fenced, never more than one unanswered frame in the pipe, the 16 ms
//! cap on a terminal that never answers, and a turn's wheel events drawn by
//! one frame.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use graft_core::event::MouseKind;
use graft_term::{SYNC_BEGIN, SYNC_END};

use super::*;

const FENCE: &str = "\u{1b}[5n";

fn fenced(mut session: Session) -> Session {
    session.capabilities.answers_dsr = true;
    session
}

/// Splits a stream into frames: each runs from `SYNC_BEGIN` to the byte
/// before the next one.
fn frames_of(stream: &str) -> Vec<&str> {
    let begin = String::from_utf8_lossy(SYNC_BEGIN).into_owned();
    stream
        .match_indices(&begin)
        .map(|(at, _)| at)
        .chain(std::iter::once(stream.len()))
        .collect::<Vec<_>>()
        .windows(2)
        .map(|pair| &stream[pair[0]..pair[1]])
        .collect()
}

#[test]
fn a_fenced_pixel_frame_is_bracketed_with_its_images_inside_then_fenced() {
    let (session, state) = at_pixel_tier();
    let mut session = fenced(session);
    let (mut terminal, recorder) = terminal(PIXEL_COLUMNS, PIXEL_ROWS);
    let mut graphics = Graphics::new(NoReplies, &session.capabilities);
    let exit = pump_with(
        Screen {
            terminal: &mut terminal,
            graphics: &mut graphics,
        },
        &mut ScriptedWakes::new(vec![]),
        &Recorded::default(),
        &mut session,
        state,
        Outside::default(),
        &mut crate::trace::Tracer::off(),
    )
    .expect("memory");
    assert_eq!(exit, Exit::Quit);

    let written = recorder.written();
    let frames = frames_of(&written);
    assert_eq!(frames.len(), 1, "one frame, then the script ended");
    let frame = frames[0];
    let end = String::from_utf8_lossy(SYNC_END).into_owned();
    assert!(frame.starts_with(&*String::from_utf8_lossy(SYNC_BEGIN)));
    let closed = frame.find(&end).expect("the frame is closed");
    let tail = &frame[closed..];
    assert!(
        tail.starts_with(&format!("{end}{FENCE}")),
        "SYNC_END, then the fence: {tail:?}"
    );
    let images = frame.find("\u{1b}_G").expect("the strips go out");
    assert!(images < closed, "every image command inside the pair");
    assert_eq!(frame.matches(FENCE).count(), 1);
}

#[test]
fn a_silent_terminal_gets_no_fence_and_frames_at_least_16_ms_apart() {
    let (mut terminal, recorder) = terminal(60, 30);
    let mut wakes = Stamped::new(ScriptedWakes::spaced(
        (0..5).map(|_| j()).chain([quit_key()]).collect(),
    ));
    let mut session = session();
    assert!(!session.capabilities.answers_dsr);
    let (exit, state) = super::super::running::pump_keeping(
        &mut terminal,
        &mut wakes,
        &Recorded::default(),
        &mut session,
        on_history(),
    );
    assert_eq!(exit.expect("memory"), Exit::Quit);
    let written = recorder.written();
    assert!(!written.contains(FENCE), "no fence on a silent terminal");
    assert_eq!(cursor(&state), 5);
    assert_eq!(wakes.drawn.len(), 6, "the first frame, then one per key");
    for pair in wakes.drawn.windows(2) {
        assert!(
            pair[1] - pair[0] >= Duration::from_millis(16),
            "capped: {:?} apart",
            pair[1] - pair[0]
        );
    }
}

/// [`ScriptedWakes`], stamping each frame the loop draws.
struct Stamped {
    inner: ScriptedWakes,
    drawn: Vec<Instant>,
}

impl Stamped {
    fn new(inner: ScriptedWakes) -> Self {
        Self {
            inner,
            drawn: Vec::new(),
        }
    }
}

impl Wakes for Stamped {
    fn next_wake(&mut self) -> Option<Wake> {
        self.inner.next_wake()
    }
    fn next_wake_by(&mut self, deadline: Instant) -> Waited {
        self.inner.next_wake_by(deadline)
    }
    fn waiting(&mut self) -> bool {
        self.inner.waiting()
    }
    fn joining_waiting(&mut self) -> bool {
        self.inner.joining_waiting()
    }
    fn mark(&mut self, phase: super::super::Phase) {
        if phase == super::super::Phase::Drawn {
            self.drawn.push(self.inner.now());
        }
        self.inner.mark(phase);
    }
    fn now(&self) -> Instant {
        self.inner.now()
    }
}

/// A held key against a terminal that answers a fence only when the loop
/// waits on it: keys keep coming (one per wait, and one to start each
/// turn), and every third wait the terminal answers. Watches the output
/// for fences written and keeps the most ever unanswered.
struct OnDemand {
    keys: usize,
    recorder: Recorder,
    answered: usize,
    waits: usize,
    most_unanswered: usize,
    /// Keys still to come at each frame drawn.
    keys_at_draws: Vec<usize>,
}

impl OnDemand {
    fn unanswered(&mut self) -> usize {
        let written = self.recorder.written().matches(FENCE).count();
        let unanswered = written - self.answered;
        self.most_unanswered = self.most_unanswered.max(unanswered);
        unanswered
    }

    /// `j` held, then `k` for the last [`BACK`]: where the cursor ends
    /// shows every key was applied, in order, though 300 run past the 255
    /// commits.
    fn key(&mut self) -> Option<Wake> {
        self.keys = self.keys.checked_sub(1)?;
        Some(if self.keys < BACK {
            key(KeyPress::plain(KeyCode::Char('k')))
        } else {
            j()
        })
    }
}

impl Wakes for OnDemand {
    fn next_wake(&mut self) -> Option<Wake> {
        self.unanswered();
        if let Some(key) = self.key() {
            return Some(key);
        }
        // The held key is released: the terminal answers what it owes.
        (self.unanswered() > 0).then(|| {
            self.answered += 1;
            Wake::Fence
        })
    }

    fn next_wake_by(&mut self, _deadline: Instant) -> Waited {
        self.waits += 1;
        if self.unanswered() > 0 && (self.waits.is_multiple_of(3) || self.keys == 0) {
            self.answered += 1;
            return Waited::Woke(Wake::Fence);
        }
        self.key().map_or(Waited::TimedOut, Waited::Woke)
    }

    fn waiting(&mut self) -> bool {
        self.unanswered();
        false
    }

    /// Keys arrive one at a time, after the turn has looked.
    fn joining_waiting(&mut self) -> bool {
        self.unanswered();
        false
    }

    fn mark(&mut self, phase: super::super::Phase) {
        if phase == super::super::Phase::Drawn {
            self.keys_at_draws.push(self.keys);
        }
    }
}

/// Of the held keys, how many at the end are `k`.
const BACK: usize = 100;

#[test]
fn three_hundred_held_keys_never_put_two_frames_in_the_pipe() {
    let (mut terminal, recorder) = terminal(60, 30);
    let mut wakes = OnDemand {
        keys: 300,
        recorder: recorder.clone(),
        answered: 0,
        waits: 0,
        most_unanswered: 0,
        keys_at_draws: Vec::new(),
    };
    let state = AppState {
        history: Some(crate::view::tests::history_of(255, 0)),
        ..on_history()
    };
    let (exit, state) = super::super::running::pump_keeping(
        &mut terminal,
        &mut wakes,
        &Recorded::default(),
        &mut fenced(session()),
        state,
    );
    assert_eq!(exit.expect("memory"), Exit::Quit);
    assert_eq!(wakes.most_unanswered, 1, "one frame in flight, never two");
    assert_eq!(cursor(&state), 200 - BACK, "every key applied, in order");
    let written = recorder.written();
    let frames = frames_of(&written);
    assert!(
        frames.len() < 300 / 2,
        "keys held behind a fence share frames: {} frames",
        frames.len()
    );
    assert!(
        frames.iter().all(|frame| frame.ends_with(FENCE)),
        "every frame fenced"
    );
    assert_eq!(
        wakes.keys_at_draws.last(),
        Some(&0),
        "a frame was drawn after the last key"
    );
}

#[test]
fn ten_wheel_events_in_one_turn_scroll_ten_steps_in_one_frame() {
    let wheel = || mouse(MouseKind::ScrollDown);
    let state = AppState {
        history: Some(crate::view::tests::history_of(255, 0)),
        ..on_history()
    };
    let (frames, state, _) = run_from(state, (0..10).map(|_| wheel()).collect());
    assert_eq!(frames, 2, "the frame before the wait, then one for all ten");
    let history = state.history.as_ref().expect("history stays");
    assert_eq!(history.top, 10 * 3, "ten steps of the unchanged 3 lines");
    assert_eq!(history.cursor, 0, "the wheel leaves the cursor");
}

/// What [`Line`] saw, in order: a wake it handed out, or a phase the loop
/// marked, each at the line's clock (ms since the run began).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Seen {
    Key(u64),
    Fence(u64),
    Drained,
    Updated,
    CarriedOut(u64),
    Drawn(u64),
}

/// A fenced terminal on a timeline of its own (as `settle::Timed`): keys
/// arrive at their instants; each fence the loop writes is answered
/// `answer_after` the frame was drawn, or never when `None`; and a wait
/// past a deadline moves the clock there instead of sleeping. Answers come
/// in order, ahead of a key due at the same instant.
struct Line {
    keys: VecDeque<(Instant, Wake)>,
    answers: VecDeque<Instant>,
    answer_after: Option<Duration>,
    recorder: Recorder,
    fences_written: usize,
    start: Instant,
    now: Instant,
    seen: Vec<Seen>,
}

impl Line {
    fn new(recorder: Recorder, answer_after: Option<Duration>, keys: Vec<(u64, Wake)>) -> Self {
        let start = Instant::now();
        Self {
            keys: keys
                .into_iter()
                .map(|(at, wake)| (start + Duration::from_millis(at), wake))
                .collect(),
            answers: VecDeque::new(),
            answer_after,
            recorder,
            fences_written: 0,
            start,
            now: start,
            seen: Vec::new(),
        }
    }

    fn ms(&self) -> u64 {
        u64::try_from((self.now - self.start).as_millis()).expect("a short run")
    }

    /// When the next wake is due, and whether it is a fence answer.
    fn next_due(&self) -> Option<(Instant, bool)> {
        let answer = self.answers.front().map(|at| (*at, true));
        let key = self.keys.front().map(|(at, _)| (*at, false));
        match (answer, key) {
            (Some(answer), Some(key)) => Some(if answer.0 <= key.0 { answer } else { key }),
            (one, other) => one.or(other),
        }
    }

    fn hand_out(&mut self) -> Option<Wake> {
        let (at, fence) = self.next_due()?;
        self.now = self.now.max(at);
        if fence {
            self.answers.pop_front();
            self.seen.push(Seen::Fence(self.ms()));
            return Some(Wake::Fence);
        }
        self.seen.push(Seen::Key(self.ms()));
        self.keys.pop_front().map(|(_, wake)| wake)
    }

    /// The times of every frame drawn.
    fn drawn(&self) -> Vec<u64> {
        self.seen
            .iter()
            .filter_map(|seen| match seen {
                Seen::Drawn(at) => Some(*at),
                _ => None,
            })
            .collect()
    }
}

impl Wakes for Line {
    fn next_wake(&mut self) -> Option<Wake> {
        self.hand_out()
    }

    fn next_wake_by(&mut self, deadline: Instant) -> Waited {
        if self.next_due().is_some_and(|(at, _)| at <= deadline) {
            return self.hand_out().map_or(Waited::Ended, Waited::Woke);
        }
        self.now = self.now.max(deadline);
        Waited::TimedOut
    }

    fn waiting(&mut self) -> bool {
        self.next_due().is_some_and(|(at, _)| at <= self.now)
    }

    fn joining_waiting(&mut self) -> bool {
        match self.next_due() {
            Some((at, false)) if at <= self.now => {
                self.keys.front().is_some_and(|(_, wake)| joins_frame(wake))
            }
            _ => false,
        }
    }

    fn mark(&mut self, phase: super::super::Phase) {
        use super::super::Phase;
        let at = self.ms();
        self.seen.push(match phase {
            Phase::Drained => Seen::Drained,
            Phase::Updated => Seen::Updated,
            Phase::CarriedOut => Seen::CarriedOut(at),
            Phase::Drawn => {
                let written = self.recorder.written().matches(FENCE).count();
                for _ in self.fences_written..written {
                    if let Some(after) = self.answer_after {
                        self.answers.push_back(self.now + after);
                    }
                }
                self.fences_written = written;
                Seen::Drawn(at)
            }
        });
    }

    fn now(&self) -> Instant {
        self.now
    }
}

fn ctrl_z() -> Wake {
    key(KeyPress {
        code: KeyCode::Char('z'),
        modifiers: graft_core::event::Modifiers::CONTROL,
    })
}

/// `Ctrl+Z` hands the terminal over (here refused in words: no terminal to
/// hand, but the loop's settle runs all the same), with the first frame's
/// fence owed and `j` arriving behind the key. Returns what the line saw
/// and where the cursor ended.
fn hand_over_with_a_fence_owed(answer_after: Option<Duration>) -> (Vec<Seen>, usize) {
    let (mut terminal, recorder) = no_tty_terminal(60, 30);
    let mut line = Line::new(recorder, answer_after, vec![(1, ctrl_z()), (2, j())]);
    let state = AppState {
        suspend_on_ctrl_z: true,
        ..on_history()
    };
    let (exit, state) = super::super::running::pump_keeping(
        &mut terminal,
        &mut line,
        &Recorded::default(),
        &mut fenced(session()),
        state,
    );
    assert_eq!(exit.expect("memory"), Exit::Quit);
    (line.seen, cursor(&state))
}

/// Review m8-02 Major 2: an answer still owed when `$EDITOR` takes the
/// terminal would be read by the editor as typing. The loop takes it
/// first; the `j` that came meanwhile waits, and is applied after.
#[test]
fn a_handover_takes_the_owed_fence_answer_first_and_holds_what_came_meanwhile() {
    let (seen, cursor) = hand_over_with_a_fence_owed(Some(Duration::from_millis(100)));
    assert_eq!(
        seen[..9],
        [
            Seen::Drawn(0),
            Seen::Key(1), // Ctrl+Z
            Seen::Drained,
            Seen::Updated,
            Seen::Key(2), // j, held back
            Seen::Fence(100),
            Seen::CarriedOut(100), // the handover, after the answer
            Seen::Drawn(100),      // at once: the screen was someone else's
            Seen::Drained,         // j, only now
        ],
        "{seen:?}"
    );
    assert_eq!(cursor, 1, "the held-back j was applied");
}

/// The same with a terminal that never answers: the handover goes ahead
/// once the fence has timed out, not before and not never.
#[test]
fn a_handover_with_no_answer_coming_goes_ahead_at_the_fence_timeout() {
    let (seen, cursor) = hand_over_with_a_fence_owed(None);
    let timeout = u64::try_from(super::super::pace::FENCE_TIMEOUT.as_millis()).expect("ms");
    assert_eq!(
        seen[..7],
        [
            Seen::Drawn(0),
            Seen::Key(1),
            Seen::Drained,
            Seen::Updated,
            Seen::Key(2),
            Seen::CarriedOut(timeout),
            Seen::Drawn(timeout),
        ],
        "{seen:?}"
    );
    assert_eq!(cursor, 1);
}

/// `j` every 100 ms for `until` ms against a fenced terminal answering
/// `answer_after` each frame. Returns the frames written and when.
fn held_j_against(answer_after: Option<Duration>, until: u64) -> (Vec<String>, Vec<u64>) {
    let (mut terminal, recorder) = terminal(60, 30);
    let keys = (1..=until / 100).map(|n| (n * 100, j())).collect();
    let mut line = Line::new(recorder.clone(), answer_after, keys);
    let state = AppState {
        history: Some(crate::view::tests::history_of(255, 0)),
        ..on_history()
    };
    let (exit, _) = super::super::running::pump_keeping(
        &mut terminal,
        &mut line,
        &Recorded::default(),
        &mut fenced(session()),
        state,
    );
    assert_eq!(exit.expect("memory"), Exit::Quit);
    let written = recorder.written();
    let frames = frames_of(&written).into_iter().map(str::to_owned).collect();
    (frames, line.drawn())
}

/// Review m8-02 Major 2: the timeout and the give-up, through the loop. A
/// fenced terminal that never answers gets a frame per timeout, three
/// timeouts, then unfenced frames at the 16 ms cap.
#[test]
fn a_fenced_terminal_that_never_answers_is_drawn_per_timeout_then_unfenced() {
    let (frames, drawn) = held_j_against(None, 2400);
    assert_eq!(drawn[..4], [0, 500, 1000, 1500], "{drawn:?}");
    assert!(
        frames[..4].iter().all(|frame| frame.ends_with(FENCE)),
        "fenced until the third timeout, whose frame still carries one"
    );
    assert!(frames.len() > 4, "{drawn:?}");
    assert!(
        frames[4..].iter().all(|frame| !frame.contains(FENCE)),
        "no fence once given up"
    );
    assert!(
        drawn[4..].windows(2).all(|pair| pair[1] - pair[0] >= 16),
        "capped: {drawn:?}"
    );
}

/// Review m8-02 Major 1, through the loop: a terminal answering every
/// fence 600 ms late stays fenced for the run, a frame per timeout.
#[test]
fn a_terminal_answering_every_fence_600_ms_late_stays_fenced() {
    let (frames, drawn) = held_j_against(Some(Duration::from_millis(600)), 4000);
    assert!(frames.len() > 6, "{drawn:?}");
    assert!(
        frames.iter().all(|frame| frame.ends_with(FENCE)),
        "every frame fenced: {drawn:?}"
    );
    assert!(
        drawn.windows(2).all(|pair| pair[1] - pair[0] >= 100),
        "never the 16 ms cap: {drawn:?}"
    );
}
