//! The perf script's controls (`MWR_FRAME_CONTROL`, m7-power-tools/04)
//! and control D, the machine alone.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use super::{IDLE, TICK};
use crate::heavy::MEASURING;

/// Which run this is: `MWR_FRAME_CONTROL`, set by `scripts/m7-perf-key.sh`
/// (m7-power-tools/04); unset is the plain run. Each control takes one
/// suspect away from the slow keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Control {
    Plain,
    /// A (`a`): the idle keys follow a tick, not an [`IDLE`] pause.
    NoIdle,
    /// B (`b`): a thread spins on one core through the whole scroll, so the
    /// machine never sits idle.
    Busy,
    /// C (`c`): no idle prefetch. The loop is told a wake is waiting before
    /// it readies any strip.
    NoPrefetch,
}

impl Control {
    pub(super) fn from_env() -> Result<Self, String> {
        match std::env::var("MWR_FRAME_CONTROL").as_deref() {
            Err(std::env::VarError::NotPresent) | Ok("" | "plain") => Ok(Self::Plain),
            Ok("a") => Ok(Self::NoIdle),
            Ok("b") => Ok(Self::Busy),
            Ok("c") => Ok(Self::NoPrefetch),
            other => Err(format!(
                "MWR_FRAME_CONTROL: {other:?} is not plain, a, b or c"
            )),
        }
    }
}

/// A thread spinning on one core until dropped (control B).
pub(super) struct Spinner {
    pub(super) stop: Arc<AtomicBool>,
    pub(super) thread: Option<thread::JoinHandle<()>>,
}

impl Spinner {
    pub(super) fn start() -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let thread = thread::spawn(move || {
            while !flag.load(Ordering::Relaxed) {
                std::hint::spin_loop();
            }
        });
        Self {
            stop,
            thread: Some(thread),
        }
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            // The spinner only loads a flag, so it cannot panic, and a drop
            // has nowhere to report a panic anyway.
            drop(thread.join());
        }
    }
}

/// Control D (m7-power-tools/04): the machine alone, no Graft code at all.
/// Each lap sleeps [`IDLE`] the way the harness did before 04b's
/// [`KeyFeed`] (`recv_timeout` on a quiet channel), then times a fixed piece of CPU and memory work at once
/// (`after_idle`) and again straight after (`warm`), and times a wake
/// handed over from another thread after the same pause (`handoff`, the
/// real loop's shape: the input thread sends, the loop's `recv` returns).
/// With and without a core held busy (control B). `tick_oversleep`: the
/// same timed wait at the scroll's one-tick length. Recorded, not asserted.
#[test]
fn the_machine_alone_after_idle_and_warm() {
    const LAPS: usize = 8;
    let turn = MEASURING.lock().unwrap_or_else(PoisonError::into_inner);
    // 4 MiB: bigger than a core's L1/L2 share, like a frame's working set.
    let buffer: Vec<u64> = (0..(4 << 20) / 8).collect();
    let work = || {
        let started = Instant::now();
        let mut sum = 0u64;
        for pass in 0..4u64 {
            for value in &buffer {
                sum = sum.wrapping_mul(31).wrapping_add(value ^ pass);
            }
        }
        std::hint::black_box(sum);
        started.elapsed()
    };
    for busy in [false, true] {
        let spinner = busy.then(Spinner::start);
        let (mut oversleep, mut after_idle, mut warm, mut handoff) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for _ in 0..LAPS {
            oversleep.push(timed_wait(IDLE));
            after_idle.push(work());
            warm.push(work());
            handoff.push(handoff_after(IDLE));
        }
        // The same timed wait at the scroll's pace: one tick, 30 times.
        let tick_oversleep: Vec<Duration> = (0..30).map(|_| timed_wait(TICK)).collect();
        drop(spinner);
        let list = |laps: &[Duration]| {
            laps.iter()
                .map(|lap| lap.as_micros().to_string())
                .collect::<Vec<_>>()
                .join(",")
        };
        println!(
            "machine: busy={busy} oversleep_us={} tick_oversleep_us={} work_after_idle_us={} work_warm_us={} handoff_us={}",
            list(&oversleep),
            list(&tick_oversleep),
            list(&after_idle),
            list(&warm),
            list(&handoff),
        );
    }
    drop(turn);
}

/// Waits `wait` the way [`Paced`] does, on a channel nobody sends to, and
/// returns how late the thread got back.
fn timed_wait(wait: Duration) -> Duration {
    let (_quiet, never) = mpsc::channel::<()>();
    let due = Instant::now() + wait;
    match never.recv_timeout(wait) {
        Err(RecvTimeoutError::Timeout) => {}
        other => panic!("a quiet channel answered: {other:?}"),
    }
    Instant::now().saturating_duration_since(due)
}

/// Another thread sleeps `pause` then sends; returns send → `recv` back.
fn handoff_after(pause: Duration) -> Duration {
    let (post, inbox) = mpsc::channel::<Instant>();
    let sender = thread::spawn(move || {
        thread::sleep(pause);
        post.send(Instant::now())
    });
    let sent = inbox
        .recv()
        .unwrap_or_else(|why| panic!("the sender hung up: {why}"));
    let handoff = sent.elapsed();
    sender
        .join()
        .unwrap_or_else(|_| panic!("the sender panicked"))
        .unwrap_or_else(|why| panic!("the send failed: {why}"));
    handoff
}
