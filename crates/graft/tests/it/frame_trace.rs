//! The frame gate and the frame trace the `frames` perf tests use
//! (m7-power-tools/04). Kept out of `frames` so both run in the fast tier:
//! `frames` itself is heavy-only.
//!
//! - [`Gate`]: the input-to-frame verdict over one scroll's keys. It asserts
//!   the p99 and prints the max, so one outlier among 420 keys is recorded,
//!   not a failure (`KNOWN_GAPS.md`: "the frame gate asserts on the max").
//! - [`FrameTrace`]: per-key phase lines, written only when
//!   `GRAFT_FRAME_TRACE=<file>` is set (`scripts/m7-perf-key.sh` sets it).
//! - [`WriteClock`]: time spent writing to the in-memory terminal, taken only
//!   while tracing.

// Only the heavy-tier `frames` writes a trace; the fast tier compiles this
// module for its two tests alone.
#![cfg_attr(not(feature = "heavy-tests"), allow(dead_code))]

use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// The environment variable naming the trace file.
pub const TRACE_VAR: &str = "GRAFT_FRAME_TRACE";

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
