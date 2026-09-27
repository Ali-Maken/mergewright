//! The total-time guard every test pump loop carries (m7-power-tools/33).
//! Its own file so `budget_run` — also built into the `budgets` example,
//! which has no `app_harness` — can include it by path.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// The most one pump may take in total, whatever its events. `PATIENCE` is
/// per event, so a loop that keeps producing events never trips it: the
/// radar-context retry loop spun for 9 minutes (m7-power-tools/33). A test
/// guard, not a product retry cap. Longer than any `PATIENCE` so a slow but
/// settling pump never trips it.
pub const DEADLINE: Duration = Duration::from_secs(600);

/// How many of the latest events a [`Deadline`] failure quotes.
const LAST_EVENTS: usize = 10;

/// The total-time guard every pump loop carries: note each event as it is
/// taken, and past the limit it fails naming the wait and the last events.
pub struct Deadline {
    started: Instant,
    limit: Duration,
    last: VecDeque<String>,
}

impl Deadline {
    /// Starts the clock at `limit`.
    pub fn of(limit: Duration) -> Self {
        Self {
            started: Instant::now(),
            limit,
            last: VecDeque::with_capacity(LAST_EVENTS),
        }
    }

    /// Records `event`; `Err` naming `what` and the last events once the
    /// pump has run past its limit.
    pub fn note(&mut self, what: &str, event: &impl std::fmt::Debug) -> Result<(), String> {
        if self.last.len() == LAST_EVENTS {
            self.last.pop_front();
        }
        self.last.push_back(format!("{event:?}"));
        if self.started.elapsed() <= self.limit {
            return Ok(());
        }
        let last: Vec<&str> = self.last.iter().map(String::as_str).collect();
        Err(format!(
            "waiting for {what}: never settled within {:?}; last {} events:\n  {}",
            self.limit,
            last.len(),
            last.join("\n  ")
        ))
    }
}
