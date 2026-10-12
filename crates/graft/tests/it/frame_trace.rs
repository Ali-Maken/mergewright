//! The frame gate and the frame trace the `frames` perf tests use
//! (m7-power-tools/04). Kept out of `frames` so both run in the fast tier:
//! `frames` itself is heavy-only.
//!
//! - [`Gate`]: the input-to-frame verdict over one scroll's keys. It asserts
//!   the p99 and prints the max, so one outlier among 420 keys is recorded,
//!   not a failure (`KNOWN_GAPS.md`: "the frame gate asserts on the max").
//! - [`FrameTrace`]: per-key phase lines, written only when
//!   `MWR_FRAME_TRACE=<file>` is set (`scripts/m7-perf-key.sh` sets it).
//! - [`WriteClock`]: time spent writing to the in-memory terminal, taken only
//!   while tracing.

// Only the heavy-tier `frames` writes a trace; the fast tier compiles this
// module for its two tests alone.
#![cfg_attr(
    not(feature = "heavy-tests"),
    allow(dead_code, reason = "only the heavy tier writes a trace")
)]

use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use graft::event_loop::Wake;
use graft::jobs::{Delivery, JobEvents};
use graft::update::AppEvent;
use graft_core::event::{Event, KeyCode, KeyPress};

/// The environment variable naming the trace file.
pub const TRACE_VAR: &str = "MWR_FRAME_TRACE";

/// One scroll's input-to-frame numbers and its verdict.
#[derive(Debug, PartialEq, Eq)]
pub struct Gate {
    pub p50: Duration,
    pub p99: Duration,
    pub max: Duration,
    /// Keys at or over the budget, by index: recorded, printed, not the
    /// verdict.
    pub over: Vec<usize>,
}

impl Gate {
    /// Ranks rounded down, over a few hundred samples.
    #[must_use]
    pub fn of(latencies: &[Duration], budget: Duration) -> Self {
        let mut sorted = latencies.to_vec();
        sorted.sort();
        let at = |percent: usize| {
            sorted
                .get(sorted.len().saturating_sub(1) * percent / 100)
                .copied()
                .unwrap_or_default()
        };
        Self {
            p50: at(50),
            p99: at(99),
            max: sorted.last().copied().unwrap_or_default(),
            over: latencies
                .iter()
                .enumerate()
                .filter(|(_, latency)| **latency >= budget)
                .map(|(key, _)| key)
                .collect(),
        }
    }

    /// PROMPT.md §6.3's budget holds when the p99 is within it.
    #[must_use]
    pub fn holds(&self, budget: Duration) -> bool {
        self.p99 <= budget
    }

    /// The line the perf script greps: p50, p99 and the max, every run.
    #[must_use]
    pub fn line(&self) -> String {
        format!(
            "p50_us={} p99_us={} max_us={} over_budget={}",
            self.p50.as_micros(),
            self.p99.as_micros(),
            self.max.as_micros(),
            self.over.len()
        )
    }
}

/// Where the trace lines go, when tracing is on.
pub struct FrameTrace {
    file: PathBuf,
}

impl FrameTrace {
    /// From the value of [`TRACE_VAR`]: `None` when unset or empty.
    #[must_use]
    pub fn from_var(value: Option<OsString>) -> Option<Self> {
        value
            .filter(|path| !path.is_empty())
            .map(|path| Self { file: path.into() })
    }

    /// Appends `lines`, one per line.
    ///
    /// # Errors
    /// When the file cannot be opened or written.
    pub fn append(&self, lines: &[String]) -> io::Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file)?;
        for line in lines {
            writeln!(file, "{line}")?;
        }
        file.flush()
    }
}

/// Time spent inside the in-memory terminal's `write`. Off, it never reads
/// the clock: the untraced scroll pays nothing for it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WriteClock {
    on: bool,
    spent: Duration,
    calls: u64,
}

impl WriteClock {
    #[must_use]
    pub fn new(on: bool) -> Self {
        Self {
            on,
            ..Self::default()
        }
    }

    /// Runs `write`, timing it only when on.
    pub fn time<T>(&mut self, write: impl FnOnce() -> T) -> T {
        if !self.on {
            return write();
        }
        let started = Instant::now();
        let out = write();
        self.spent += started.elapsed();
        self.calls += 1;
        out
    }

    #[must_use]
    pub fn spent(&self) -> Duration {
        self.spent
    }

    #[must_use]
    pub fn calls(&self) -> u64 {
        self.calls
    }
}

/// One wake on the harness's channel, stamped just before it was sent, so a
/// job event's time in the queue can be told from its time in the loop
/// (m7-power-tools/12, the startup trace). The app's own channel carries
/// the bare [`Wake`]; the stamp is the harness's only addition.
#[derive(Debug)]
pub struct Posted {
    /// Just before the send.
    pub at: Instant,
    pub wake: Wake,
}

/// The job pool's side of the harness channel: every result and progress
/// report goes in as the app's `impl JobEvents for Sender<Wake>` sends it
/// (`event_loop/mod.rs`), with its send time.
#[derive(Clone)]
pub struct Post(pub Sender<Posted>);

impl JobEvents for Post {
    fn deliver(&self, event: AppEvent) -> Delivery {
        let posted = Posted {
            at: Instant::now(),
            wake: Wake::Event(event),
        };
        match self.0.send(posted) {
            Ok(()) => Delivery::Received,
            Err(_returned) => Delivery::NobodyListening,
        }
    }
}

/// When the input thread sent one key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sent {
    /// The script's tick for it: the first key's start plus every pause
    /// since.
    pub due: Instant,
    /// Just before the send: where the key's latency starts.
    pub at: Instant,
}

/// The `frames` harness's keyboard (m7-power-tools/04b): a thread that
/// sleeps each of the script's pauses and sends the key into the loop's
/// wake channel, as the app's input forwarder does
/// (`event_loop/input.rs` `spawn_reader`: `wakes.send(Wake::Event(
/// AppEvent::Core(event)))`). The loop learns of the key from the send, in
/// `recv`, never from a timed wait of its own, so the OS's timer slack on a
/// long sleep lands before the send, where a real key's arrival is, not in
/// its latency.
///
/// The pauses are kept on the script's clock (each key due one pause after
/// the one before, however late the thread woke), so a loop that falls
/// behind still shows as keys queued in the channel.
pub struct KeyFeed {
    sent: SentLog,
    thread: JoinHandle<Result<(), String>>,
}

/// The input thread's stamps, readable while it runs.
#[derive(Clone)]
pub struct SentLog(Arc<Mutex<Vec<Sent>>>);

impl SentLog {
    /// Key `key`'s stamp; there once the loop has received it (the stamp
    /// is taken before the send).
    #[must_use]
    pub fn get(&self, key: usize) -> Option<Sent> {
        self.stamps().get(key).copied()
    }

    fn stamps(&self) -> MutexGuard<'_, Vec<Sent>> {
        // Poisoned only if the thread panicked mid-push, which a `Vec::push`
        // of a `Copy` value does not; `finish` reports a panic either way.
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl KeyFeed {
    /// Starts the thread. `keys[i].1` is the pause before key `i`; the
    /// first key's is ignored (it is due at once).
    ///
    /// # Errors
    /// When the thread cannot be spawned.
    pub fn start(keys: Vec<(KeyCode, Duration)>, wakes: Sender<Posted>) -> io::Result<Self> {
        let sent = Arc::new(Mutex::new(Vec::with_capacity(keys.len())));
        let stamps = Arc::clone(&sent);
        let thread = thread::Builder::new()
            .name("frames-input".to_owned())
            .spawn(move || {
                let mut due = Instant::now();
                for (index, (key, pause)) in keys.into_iter().enumerate() {
                    if index > 0 {
                        due += pause;
                    }
                    let wait = due.saturating_duration_since(Instant::now());
                    if !wait.is_zero() {
                        thread::sleep(wait);
                    }
                    let at = Instant::now();
                    stamps
                        .lock()
                        .map_err(|_poisoned| "the stamp list was poisoned".to_owned())?
                        .push(Sent { due, at });
                    let press = KeyPress::plain(key);
                    wakes
                        .send(Posted {
                            at,
                            wake: Wake::Event(AppEvent::Core(Event::Key(press))),
                        })
                        .map_err(|_gone| format!("the loop hung up before key {index}"))?;
                }
                Ok(())
            })?;
        Ok(Self {
            sent: SentLog(sent),
            thread,
        })
    }

    /// A handle on the stamps, for the loop's side to read as keys arrive.
    #[must_use]
    pub fn log(&self) -> SentLog {
        self.sent.clone()
    }

    /// Key `key`'s stamp; see [`SentLog::get`].
    #[must_use]
    pub fn sent(&self, key: usize) -> Option<Sent> {
        self.sent.get(key)
    }

    /// Waits for the thread to send its last key and returns every stamp.
    ///
    /// # Errors
    /// The thread's own failure (the loop hung up), or its panic.
    pub fn finish(self) -> Result<Vec<Sent>, String> {
        self.thread
            .join()
            .map_err(|_panic| "the input thread panicked".to_owned())??;
        Ok(self.sent.stamps().clone())
    }
}

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

#[test]
fn frames_gate_asserts_p99_and_prints_max() {
    let budget = ms(16);
    // 420 keys, one 30 ms outlier: the max says so, the verdict holds.
    let mut keys = vec![ms(5); 420];
    keys[37] = ms(30);
    let gate = Gate::of(&keys, budget);
    assert!(gate.holds(budget), "one outlier in 420 fails nothing");
    assert_eq!(gate.max, ms(30));
    assert_eq!(gate.over, vec![37]);
    assert_eq!(
        gate.line(),
        "p50_us=5000 p99_us=5000 max_us=30000 over_budget=1"
    );

    // More than 1% over: the p99 is over, and the verdict fails.
    for key in keys.iter_mut().take(10) {
        *key = ms(20);
    }
    let gate = Gate::of(&keys, budget);
    assert_eq!(gate.p99, ms(20));
    assert!(!gate.holds(budget));

    // Nothing measured: no panic, nothing over.
    assert!(Gate::of(&[], budget).holds(budget));
}

#[test]
fn trace_off_adds_no_work() {
    assert!(FrameTrace::from_var(None).is_none(), "unset: no trace file");
    assert!(
        FrameTrace::from_var(Some(OsString::new())).is_none(),
        "empty: no trace file"
    );
    let mut clock = WriteClock::new(false);
    let mut wrote = 0;
    for _ in 0..1000 {
        clock.time(|| wrote += 1);
    }
    assert_eq!(wrote, 1000, "every write still happens");
    assert_eq!(
        (clock.spent(), clock.calls()),
        (Duration::ZERO, 0),
        "off, the clock is never read"
    );

    let mut clock = WriteClock::new(true);
    clock.time(|| ());
    assert_eq!(clock.calls(), 1, "on, each write is timed");
}

/// m7-power-tools/04b: the loop learns of a key the way the app does, from
/// the input thread's send, so the harness's own timed wait is out of the
/// latency. A key sent after a 600 ms idle is back out of `recv` within
/// 1 ms of its send. Release asserts it (ADR-0021: the scripts run this
/// with `frames::`); debug prints it, since a parallel fast tier can hold
/// the receiving thread off a core for longer than that.
#[test]
fn a_key_sent_after_idle_is_received_within_1ms_of_its_send() {
    const IDLE: Duration = Duration::from_millis(600);
    let (wakes, inbox) = mpsc::channel();
    let feed = KeyFeed::start(
        vec![
            (KeyCode::Char('j'), Duration::ZERO),
            (KeyCode::Char('k'), IDLE),
        ],
        wakes,
    )
    .unwrap_or_else(|why| panic!("the input thread did not start: {why}"));

    let mut received = Vec::new();
    for key in ['j', 'k'] {
        let posted = inbox
            .recv()
            .unwrap_or_else(|why| panic!("the input thread hung up: {why}"));
        let at = Instant::now();
        match posted.wake {
            Wake::Event(AppEvent::Core(Event::Key(press))) => {
                assert_eq!(
                    press,
                    KeyPress::plain(KeyCode::Char(key)),
                    "in script order"
                );
            }
            other => panic!("not a key: {other:?}"),
        }
        received.push(at);
    }
    let second = feed.sent(1).expect("stamped before it was sent");
    let sent = feed
        .finish()
        .unwrap_or_else(|why| panic!("the input thread failed: {why}"));
    assert_eq!(sent.len(), 2, "one stamp per key");
    assert_eq!(sent[1], second);
    let first = sent[0];
    // The script: the second key is due one pause after the first.
    assert_eq!(second.due.duration_since(first.due), IDLE);
    assert!(second.at >= second.due, "never sent before it is due");
    let handoff = received[1].saturating_duration_since(second.at);
    println!(
        "frame_trace: key after {IDLE:?} idle: send late by {}us (the thread's own sleep, outside the latency), send -> recv {}us",
        second.at.duration_since(second.due).as_micros(),
        handoff.as_micros()
    );
    if !cfg!(debug_assertions) {
        assert!(
            handoff < Duration::from_millis(1),
            "send -> recv took {handoff:?} after {IDLE:?} idle"
        );
    }
}
