//! The version of the `git` binary on this machine.
//!
//! Graft reads a repository with `gix` but *writes* it by running the user's
//! own `git` (PROMPT.md Technology), and the minimum that supports everything
//! Graft asks for is 2.38. An older git mostly works, so this is a warning and
//! not a refusal — but it has to be a warning the user actually sees, because
//! the failure it predicts arrives much later and looks like a Graft bug.
//!
//! Runs through [`GitInvocation`] like every other git spawn from M3 on
//! (m2-details-and-diffs/01): arguments as values, never a joined shell
//! string.

use std::fmt;
use std::io;
use std::path::Path;
use std::time::{Duration, Instant};

use crate::run::{GitArg, GitInvocation, RunError, Stop};

/// How long `git --version` may take before Graft gives up on it.
///
/// It should take milliseconds. The deadline exists because this runs during
/// startup: a `git` wedged on a stale network mount must leave Graft with a
/// version it cannot determine, not with a window that never appears.
const DEADLINE: Duration = Duration::from_secs(10);

/// A [`Stop`] that says stop once [`DEADLINE`] has passed, checked on
/// [`GitInvocation::run`]'s own poll (at least every 50 ms) — no separate
/// timer thread.
struct Deadline {
    started: Instant,
    limit: Duration,
}

impl Deadline {
    fn starting_now(limit: Duration) -> Self {
        Self {
            started: Instant::now(),
            limit,
        }
    }
}

impl Stop for Deadline {
    fn should_stop(&self) -> bool {
        self.started.elapsed() >= self.limit
    }
}

/// A `major.minor.patch` git version.
///
/// Ordered the way versions are compared, so `2.37.0 < 2.38.0 < 2.39.5`.
/// Anything git appends beyond the third component — `2.39.5.windows.1`, or
/// Apple's `(Apple Git-154)` — is dropped: it identifies a build, not a
/// feature set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct GitVersion {
    /// The first component: `2` for every git in use.
    pub major: u32,
    /// The second component, which is where git's features actually land.
    pub minor: u32,
    /// The third component, or `0` when git printed only two.
    pub patch: u32,
}

impl GitVersion {
    /// The oldest git Graft supports (PROMPT.md Technology).
    pub const MINIMUM: GitVersion = GitVersion {
        major: 2,
        minor: 38,
        patch: 0,
    };
}

impl fmt::Display for GitVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let GitVersion {
            major,
            minor,
            patch,
        } = self;
        write!(f, "{major}.{minor}.{patch}")
    }
}

/// What the system `git` turned out to be.
///
/// An enum rather than a version plus a `warning: Option<…>`, so that the
/// ordinary case has no warning field to render at all: a status line shows
/// `git 2.39.5`, never `git 2.39.5, warning: none` (CLAUDE.md).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemGit {
    /// New enough for everything Graft runs.
    Supported(GitVersion),
    /// Older than [`GitVersion::MINIMUM`]. Usable, but some operations may
    /// fail in ways that are not Graft's doing.
    TooOld {
        /// What is installed.
        found: GitVersion,
        /// What Graft needs.
        minimum: GitVersion,
    },
}

impl SystemGit {
    /// Classifies a version against the minimum.
    #[must_use]
    pub fn for_version(found: GitVersion) -> Self {
        if found >= GitVersion::MINIMUM {
            SystemGit::Supported(found)
        } else {
            SystemGit::TooOld {
                found,
                minimum: GitVersion::MINIMUM,
            }
        }
    }

    /// The installed version, whether or not it is new enough.
    #[must_use]
    pub fn version(&self) -> GitVersion {
        match self {
            SystemGit::Supported(found) | SystemGit::TooOld { found, .. } => *found,
        }
    }
}

/// Runs `git --version` and classifies what it printed.
///
/// Goes through [`GitInvocation`], in the current directory: there is no
/// open repository yet at startup, and `--version` reads none. Its own
/// [`Deadline`] stands in for [`Stop`] — `GitInvocation` has no built-in
/// timeout, only cancellation, and a wedged `git` here must not hang
/// startup.
///
/// # Errors
/// [`VersionError`] if git is not on the `PATH`, could not be run, did not
/// finish within [`DEADLINE`], exited non-zero, or printed something this
/// function cannot read as a version.
pub fn system_git() -> Result<SystemGit, VersionError> {
    let deadline = Deadline::starting_now(DEADLINE);
    let invocation = GitInvocation::in_directory(Path::new(".")).arg(GitArg::Flag("--version"));
    let output = invocation
        .run(&deadline)
        .map_err(run_error_to_version_error)?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_version(&stdout).map(SystemGit::for_version)
}

/// Reads [`RunError`] into the shape `system_git`'s callers already know.
/// `--version` never asks for credentials or reads config, so the only
/// failures `GitInvocation` can produce here are git missing, unstartable,
/// stopped (the deadline) or exiting non-zero — never `Signalled`, since
/// nothing but the deadline ever kills it and the deadline reads as
/// `Stopped`.
fn run_error_to_version_error(error: RunError) -> VersionError {
    match error {
        RunError::GitNotFound => VersionError::Spawn {
            source: io::Error::new(
                io::ErrorKind::NotFound,
                "git is not installed, or not on the PATH",
            ),
        },
        // `system_git` never calls `on_stderr_line`, so `Sink` can only
        // mean a future caller added one; treated the same as `Spawn` —
        // both are "talking to git failed" — rather than left unhandled.
        RunError::Spawn(source) | RunError::Sink(source) => VersionError::Spawn { source },
        RunError::Stopped => VersionError::Timeout {
            after: DEADLINE,
            // `GitInvocation::run` already killed and reaped the child
            // before returning `Stopped`; a kill that failed instead
            // surfaces as `RunError::Spawn`, above.
            kill: None,
        },
        RunError::Exited {
            status,
            stderr_first_line,
            ..
        } => VersionError::Failed {
            status: format!("exit code {status}"),
            stderr: stderr_first_line,
        },
        RunError::Signalled { .. } => VersionError::Failed {
            status: "killed by a signal".to_owned(),
            stderr: String::new(),
        },
    }
}

/// Reads the version out of `git --version`'s output.
///
/// The shape is `git version <version>[ <build details>]`: `2.39.5`,
/// `2.39.5.windows.1`, `2.50.1 (Apple Git-155)`. Two components are enough —
/// git printed `2.38` exactly once — and anything past the third is build
/// detail.
fn parse_version(stdout: &str) -> Result<GitVersion, VersionError> {
    let unrecognised = || VersionError::Unrecognised {
        output: stdout.trim().escape_debug().to_string(),
    };

    let rest = stdout
        .trim()
        .strip_prefix("git version ")
        .ok_or_else(unrecognised)?;
    let number = rest.split_whitespace().next().ok_or_else(unrecognised)?;

    let mut components = number.split('.');
    let mut required = || -> Result<u32, VersionError> {
        components
            .next()
            .ok_or_else(unrecognised)?
            .parse()
            .map_err(|_| unrecognised())
    };
    let major = required()?;
    let minor = required()?;
    // Git has shipped `2.39.GIT` from its own source tree: a third component
    // that is not a number is a build marker, not a patch level.
    let patch = components
        .next()
        .and_then(|component| component.parse().ok())
        .unwrap_or(0);

    Ok(GitVersion {
        major,
        minor,
        patch,
    })
}

/// Why the system git's version could not be established.
#[derive(Debug, thiserror::Error)]
pub enum VersionError {
    /// `git` could not be started — most often, it is not installed — or
    /// could not be waited for once it was.
    #[error("cannot run git: {source}")]
    Spawn {
        /// What the operating system said.
        source: io::Error,
    },
    /// `git --version` ran and failed.
    #[error("`git --version` {status}: {stderr}")]
    Failed {
        /// How it exited.
        status: String,
        /// What it wrote to stderr, trimmed.
        stderr: String,
    },
    /// `git --version` did not finish in time.
    #[error("`git --version` did not finish within {after:?}{}", unstoppable(.kill.as_ref()))]
    Timeout {
        /// How long it was given.
        after: Duration,
        /// Why the wedged process could not be killed, if it could not be.
        #[source]
        kill: Option<io::Error>,
    },
    /// `git --version` printed something that is not a version.
    #[error("`git --version` printed \"{output}\", not a version")]
    Unrecognised {
        /// What it printed, escaped for display.
        output: String,
    },
}

/// The second half of a timeout message, when there is one: a wedged `git`
/// that could not even be killed is a different situation from one that was.
fn unstoppable(kill: Option<&io::Error>) -> String {
    match kill {
        Some(error) => format!("; it could not be stopped: {error}"),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests;
