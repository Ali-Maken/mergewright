//! The paced loop (G-pace, ADR-0054, DESIGN.md §4.7): every frame bracketed
//! and fenced, never more than one unanswered frame in the pipe, the 16 ms
//! cap on a terminal that never answers, and a turn's wheel events drawn by
//! one frame.

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
        &mut terminal,
        &mut ScriptedWakes::new(vec![]),
        &Recorded::default(),
        &mut session,
        state,
        &mut graphics,
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
            self.drawn.push(Instant::now());
        }
        self.inner.mark(phase);
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
