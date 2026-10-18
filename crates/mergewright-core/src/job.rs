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

/// Why a job produced no result.
///
/// The message is already rendered as text, and that is deliberate: `core`
/// cannot name `git`'s or `ops`' error types (ARCHITECTURE.md), and the only
/// thing the application does with a failed job is tell the user about it. The
/// job runner renders the error at the boundary where the concrete type is
/// still in scope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JobFailure {
    /// The job ran and reported that it could not do what it was asked — no
    /// repository at that path, a `git` command that exited non-zero. Expected,
    /// and the user's to act on.
    Failed(String),
    /// The job panicked: a bug in Mergewright, caught at the worker boundary so that
    /// one broken job cannot take the whole application down with it.
    Panicked {
        /// What the panic said.
        message: String,
        /// Where the crash report went, when there was a crash handler to
        /// write one — either a path or why it could not be written. `None`
        /// when no handler was installed, which is the case in tests.
        report: Option<String>,
    },
}

/// Reads as a status line: `not a git repository`, or
/// `panicked: index out of bounds — report: /…/crash-20260912-124000.log`.
///
/// Hand-written rather than derived: a job failure is a value that travels
/// through an event, never a `Result`'s error, so it is not an
/// `std::error::Error` and `thiserror` would make it one.
impl fmt::Display for JobFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JobFailure::Failed(message) => f.write_str(message),
            JobFailure::Panicked {
                message,
                report: None,
            } => write!(f, "panicked: {message}"),
            JobFailure::Panicked {
                message,
                report: Some(report),
            } => write!(f, "panicked: {message} — report: {report}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_id_reads_as_a_short_label() {
        assert_eq!(JobId::new(7).to_string(), "job 7");
    }

    #[test]
    fn a_failure_reads_as_what_went_wrong() {
        assert_eq!(
            JobFailure::Failed("not a git repository".to_owned()).to_string(),
            "not a git repository"
        );
    }

    #[test]
    fn a_panic_says_so_and_names_its_report() {
        let without = JobFailure::Panicked {
            message: "lane 3 vanished".to_owned(),
            report: None,
        };
        let with = JobFailure::Panicked {
            message: "lane 3 vanished".to_owned(),
            report: Some("/state/graft/crash-1.log".to_owned()),
        };

        assert_eq!(without.to_string(), "panicked: lane 3 vanished");
        assert_eq!(
            with.to_string(),
            "panicked: lane 3 vanished — report: /state/graft/crash-1.log"
        );
    }
}
