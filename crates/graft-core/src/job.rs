//! Identity for the background work `app` runs off the UI thread.
//!
//! The runner itself lives in `app` (ARCHITECTURE.md): only `app` can name
//! what a job actually does, because a job carries a `git::GitInvocation` or an
//! `ops` plan. What `core` owns is the *handle* — the id `Event::JobFinished`
//! and `Event::JobFailed` carry back, which is how `update` learns that a job
//! it asked for is over without `core` ever seeing the job's contents
//! (DESIGN.md §8.2).

use std::fmt;

/// Names one background job for as long as it runs.
///
/// Opaque on purpose: nothing outside the job runner may invent an id or read
/// meaning into its value. `app` keeps the map from id to "what this job was
/// producing"; everyone else only ever compares ids for equality.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JobId(u64);

impl JobId {
    /// Wrap a number the job runner has allocated.
    ///
    /// Allocation is the runner's business: ids must be unique for the life of
    /// the process, and only the runner can guarantee that.
    #[must_use]
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// The number behind the id, for logging and for the runner's own tables.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Renders as `job 7` — short enough for a status line.
impl fmt::Display for JobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "job {}", self.0)
    }
}
