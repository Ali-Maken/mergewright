//! The slope guard's child process: one gated run's peaks, read back from a
//! fresh copy of the test binary (m1-the-graph/57).

use std::path::Path;
use std::time::Duration;

/// Set by [`gated_in_child`] for `heavy::child_gated_run`: the repository to
/// run the gated path on.
///
/// Used by `heavy.rs`'s slope guard; not visible from
/// `examples/budgets.rs`'s separate compilation of this file (as
/// [`Opened`] above), hence the `dead_code` allow on every item down to
/// [`parse_child_peak`].
#[allow(
    dead_code,
    reason = "used by heavy.rs's slope guard, not visible from examples/budgets.rs's separate compilation of this file"
)]
pub const CHILD_REPO_ENV: &str = "MWR_CHILD_REPO";

/// The child's one line of output carries this (cargo's own harness may
/// share the line with a `test <name> ... ` prefix, so it is not
/// necessarily at the start — see [`gated_in_child`]).
#[allow(
    dead_code,
    reason = "used by heavy.rs's slope guard, not visible from examples/budgets.rs's separate compilation of this file"
)]
pub const CHILD_MARK: &str = "child_peak";

/// One gated run's peaks, read back from a fresh process (m1-the-graph/57).
///
/// `getrusage`'s peak and `footprint`'s peak are both lifetime maxima of the
/// *process* they run in, not of the work inside it (see
/// `mergewright_fixtures::memory`'s module docs). A slope guard wants one
/// fixture's own peak, never a peak an earlier fixture already set in the
/// same test binary, so every fixture in a slope guard runs the gated path
/// in a process of its own and this is what that process reports back.
#[derive(Debug)]
#[allow(
    dead_code,
    reason = "used by heavy.rs's slope guard, not visible from examples/budgets.rs's separate compilation of this file"
)]
pub struct ChildPeak {
    /// The child's peak RSS (`getrusage`) over its whole gated run. The
    /// only field the slope guard asserts on.
    pub peak_rss: u64,
    /// The child's peak physical footprint over its whole gated run.
    #[allow(
        dead_code,
        reason = "reported by budgets --gate's own instrumentation, not yet by a heavy-tier assertion"
    )]
    pub footprint_peak: u64,
    /// The child's physical footprint the moment `G` landed.
    #[allow(
        dead_code,
        reason = "reported by budgets --gate's own instrumentation, not yet by a heavy-tier assertion"
    )]
    pub footprint_after_g: u64,
    /// The child's first-paint duration.
    #[allow(
        dead_code,
        reason = "carried for a future first-paint-in-a-child guard; not asserted here"
    )]
    pub first_paint: Duration,
}

/// Why [`gated_in_child`] could not read a [`ChildPeak`].
#[derive(Debug)]
#[allow(
    dead_code,
    reason = "used by heavy.rs's slope guard, not visible from examples/budgets.rs's separate compilation of this file"
)]
pub enum ChildError {
    /// The child process could not even be started, or this binary could
    /// not find its own path to re-exec.
    Spawn(std::io::Error),
    /// The child exited without success. Carries the tail of its stderr, so
    /// the failure is visible rather than turned into a default reading.
    Exit {
        status: std::process::ExitStatus,
        stderr_tail: String,
    },
    /// The child exited cleanly but printed no [`CHILD_MARK`] line, or the
    /// line it printed did not parse.
    NoReport,
}

impl std::fmt::Display for ChildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChildError::Spawn(why) => write!(f, "could not run the measuring child: {why}"),
            ChildError::Exit {
                status,
                stderr_tail,
            } => write!(f, "the measuring child exited {status}: {stderr_tail}"),
            ChildError::NoReport => {
                write!(f, "the measuring child printed no {CHILD_MARK:?} line")
            }
        }
    }
}

impl std::error::Error for ChildError {}

/// Runs the gated path (open → first paint → `G` → `g`) on `repo` in a
/// fresh copy of this test binary — `heavy::child_gated_run`, selected by
/// [`CHILD_REPO_ENV`] — so the peaks reported are that run's own, not
/// whatever this process's lifetime maximum already was.
#[allow(
    dead_code,
    reason = "used by heavy.rs's slope guard, not visible from examples/budgets.rs's separate compilation of this file"
)]
pub fn gated_in_child(repo: &Path) -> Result<ChildPeak, ChildError> {
    let exe = std::env::current_exe().map_err(ChildError::Spawn)?;
    let output = std::process::Command::new(exe)
        .args([
            "heavy::child_gated_run",
            "--exact",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD_REPO_ENV, repo)
        .output()
        .map_err(ChildError::Spawn)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stderr_tail: String = stderr.lines().rev().take(20).collect::<Vec<_>>().join("\n");
        return Err(ChildError::Exit {
            status: output.status,
            stderr_tail,
        });
    }
    // The mark is not necessarily at the start of the line: cargo's own test
    // harness prints `test heavy::child_gated_run ... ` with no newline
    // before the test body's own `println!` runs, so the two share a line.
    // `contains`, not `starts_with`, is what `result_line` (this file's
    // predecessor for the other budget tests) already did for the same
    // reason.
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .find(|line| line.contains(CHILD_MARK))
        .ok_or(ChildError::NoReport)?;
    parse_child_peak(line).ok_or(ChildError::NoReport)
}

/// Parses one `child_peak rss=<bytes> footprint_peak=<bytes>
/// footprint_after_g=<bytes> first_paint_ms=<n>` line. `None` for any
/// missing or unparseable field — [`gated_in_child`] turns that into
/// [`ChildError::NoReport`] rather than defaulting a field.
fn parse_child_peak(line: &str) -> Option<ChildPeak> {
    let field = |name: &str| -> Option<u64> {
        line.split_whitespace()
            .find_map(|pair| pair.strip_prefix(name)?.strip_prefix('='))
            .and_then(|value| value.parse().ok())
    };
    Some(ChildPeak {
        peak_rss: field("rss")?,
        footprint_peak: field("footprint_peak")?,
        footprint_after_g: field("footprint_after_g")?,
        first_paint: Duration::from_millis(field("first_paint_ms")?),
    })
}
