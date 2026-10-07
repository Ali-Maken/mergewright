//! The quit wait's stop offer on the loop's clock (m8/13b review fix, M1
//! item 2): the loop counts from its first sight of the wait, wakes itself
//! at `STOP_OFFER_AFTER` with no key pressed, draws the offer, and never
//! wakes for it again. `y` on the stop box ends the loop with the seconds
//! waited. A virtual timeline: waiting past a deadline moves time there.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use super::super::{Phase as LoopPhase, Waited};
use super::*;
use crate::update::quit::STOP_OFFER_AFTER;
use crate::update::quit::tests::{plain, waiting};

/// Wakes at their own instants on a virtual clock; records every deadline
/// the loop waited toward.
struct Clocked {
    script: VecDeque<(Instant, Wake)>,
    now: Instant,
    start: Instant,
    deadlines: Vec<Instant>,
    /// When each frame was drawn.
    drawn: Vec<Instant>,
}

impl Clocked {
    fn new(steps: Vec<(Duration, Wake)>) -> Self {
        let start = Instant::now();
        Self {
            script: steps
                .into_iter()
                .map(|(at, wake)| (start + at, wake))
                .collect(),
            now: start,
            start,
            deadlines: Vec::new(),
            drawn: Vec::new(),
        }
    }
}

impl Wakes for Clocked {
    fn next_wake(&mut self) -> Option<Wake> {
        let (at, wake) = self.script.pop_front()?;
        self.now = self.now.max(at);
        Some(wake)
    }

    fn next_wake_by(&mut self, deadline: Instant) -> Waited {
        self.deadlines.push(deadline);
        match self.script.front() {
            Some((at, _)) if *at <= deadline => {
                self.next_wake().map_or(Waited::Ended, Waited::Woke)
            }
            _ => {
                self.now = self.now.max(deadline);
                Waited::TimedOut
            }
        }
    }

    fn waiting(&mut self) -> bool {
        self.script.front().is_none_or(|(at, _)| *at <= self.now)
    }

    fn joining_waiting(&mut self) -> bool {
        self.script
            .front()
            .is_some_and(|(at, wake)| *at <= self.now && joins_frame(wake))
    }

    fn mark(&mut self, phase: LoopPhase) {
        if phase == LoopPhase::Drawn {
            self.drawn.push(self.now);
        }
    }

    fn now(&self) -> Instant {
        self.now
    }
}

#[test]
fn the_loop_offers_the_stop_at_ten_seconds_and_y_quits_with_the_seconds_waited() {
    let (state, id) = waiting();
    let what = state
        .remote
        .running
        .as_ref()
        .expect("the merge runs")
        .description
        .clone();
    let mut wakes = Clocked::new(vec![
        // Before the offer `q` changes nothing.
        (Duration::from_secs(2), key(plain('q'))),
        (Duration::from_secs(15), key(plain('q'))),
        (Duration::from_secs(16), key(plain('y'))),
    ]);
    let jobs = Recorded::default();
    let (mut terminal, recorder) = terminal(120, 40);
    let (exit, end) = super::super::running::pump_keeping(
        &mut terminal,
        &mut wakes,
        &jobs,
        &mut session(),
        state,
    );

    let offer = wakes.start + STOP_OFFER_AFTER;
    assert!(
        wakes.deadlines.contains(&offer),
        "the loop waited toward since + 10 s: {:?}",
        wakes.deadlines
    );
    assert!(
        wakes
            .deadlines
            .iter()
            .filter(|at| **at > offer)
            .all(|at| *at - offer < Duration::from_secs(1)),
        "nothing after the offer waits toward a later offer: {:?}",
        wakes.deadlines
    );
    assert!(
        wakes.drawn.contains(&offer),
        "a frame drawn at the offer, no key pressed: {:?}",
        wakes.drawn
    );
    drop(recorder);
    assert!(
        jobs.commands().contains(&Command::ForceStop(id)),
        "{:?}",
        jobs.commands()
    );
    assert_eq!(end.quit_when_done, None);
    match exit.expect("writing to memory cannot fail") {
        Exit::StoppedHalfWay { what: named, after } => {
            assert_eq!(named, what);
            assert_eq!(
                after,
                Duration::from_secs(16),
                "counted from the loop's first sight"
            );
        }
        other => panic!("expected StoppedHalfWay, got {other:?}"),
    }
}

/// Nothing pressed after the first 2 s: the loop alone turns the line into
/// the offer at 10 s, draws it, and asks for nothing.
#[test]
fn the_offer_line_is_drawn_from_the_clock_alone() {
    let (state, _) = waiting();
    let mut wakes = Clocked::new(vec![(Duration::from_secs(2), key(plain('q')))]);
    let jobs = Recorded::default();
    let (mut terminal, _recorder) = terminal(120, 40);
    let (exit, end) = super::super::running::pump_keeping(
        &mut terminal,
        &mut wakes,
        &jobs,
        &mut session(),
        state,
    );
    exit.expect("writing to memory cannot fail");
    let line = format!(
        "{} \u{b7} still running after 10 s \u{b7} q stop it now \u{b7} esc stays",
        end.remote
            .running
            .as_ref()
            .expect("the merge runs")
            .description
    );
    assert_eq!(end.shown_status(), &crate::update::Status::Waiting(line));
    assert!(wakes.drawn.contains(&(wakes.start + STOP_OFFER_AFTER)));
    assert!(jobs.commands().is_empty(), "{:?}", jobs.commands());
}
