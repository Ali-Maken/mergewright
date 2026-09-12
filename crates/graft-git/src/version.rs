//! The version of the `git` binary on this machine.
//!
//! Graft reads a repository with `gix` but *writes* it by running the user's
//! own `git` (PROMPT.md Technology), and the minimum that supports everything
//! Graft asks for is 2.38. An older git mostly works, so this is a warning and
//! not a refusal — but it has to be a warning the user actually sees, because
//! the failure it predicts arrives much later and looks like a Graft bug.
//!
//! This is deliberately the whole of the git-running surface for M0. The typed
//! `GitInvocation` builder every real operation goes through — arguments as
//! values, never a joined shell string — arrives with the first write.

use std::fmt;
use std::io;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

/// How long `git --version` may take before Graft gives up on it.
///
/// It should take milliseconds. The deadline exists because this runs during
/// startup: a `git` wedged on a stale network mount must leave Graft with a
/// version it cannot determine, not with a window that never appears.
const DEADLINE: Duration = Duration::from_secs(10);

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
/// # Errors
/// [`VersionError`] if git is not on the `PATH`, could not be run, did not
/// finish within [`DEADLINE`], exited non-zero, or printed something this
/// function cannot read as a version.
pub fn system_git() -> Result<SystemGit, VersionError> {
    let output = run_git_version()?;

    if !output.status.success() {
        return Err(VersionError::Failed {
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_version(&stdout).map(SystemGit::for_version)
}

/// Runs `git --version`, giving up at [`DEADLINE`].
fn run_git_version() -> Result<Output, VersionError> {
    let mut child = Command::new("git")
        // A typed argument, not a joined shell string (PROMPT.md Technology).
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // `--version` never asks for credentials, but an ambient askpass or a
        // terminal prompt would turn a version check into a hang.
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("LC_ALL", "C")
        .spawn()
        .map_err(|source| VersionError::Spawn { source })?;

    let started = Instant::now();
    loop {
        let exited = child
            .try_wait()
            .map_err(|source| VersionError::Spawn { source })?;
        if exited.is_some() {
            break;
        }
        if started.elapsed() >= DEADLINE {
            // Best effort: if the kill or the reap fails the child is beyond
            // our reach anyway, and the timeout is still the right answer.
            let killed = child.kill().and_then(|()| child.wait().map(drop));
            return Err(VersionError::Timeout {
                after: DEADLINE,
                kill: killed.err(),
            });
        }
        // Both pipes are read below. `git --version` prints one short line,
        // far less than a pipe buffer, so neither can fill while we wait.
        std::thread::sleep(Duration::from_millis(2));
    }

    child
        .wait_with_output()
        .map_err(|source| VersionError::Spawn { source })
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
#[derive(Debug)]
pub enum VersionError {
    /// `git` could not be started — most often, it is not installed — or
    /// could not be waited for once it was.
    Spawn {
        /// What the operating system said.
        source: io::Error,
    },
    /// `git --version` ran and failed.
    Failed {
        /// How it exited.
        status: String,
        /// What it wrote to stderr, trimmed.
        stderr: String,
    },
    /// `git --version` did not finish in time.
    Timeout {
        /// How long it was given.
        after: Duration,
        /// Why the wedged process could not be killed, if it could not be.
        kill: Option<io::Error>,
    },
    /// `git --version` printed something that is not a version.
    Unrecognised {
        /// What it printed, escaped for display.
        output: String,
    },
}

impl fmt::Display for VersionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VersionError::Spawn { source } => {
                write!(f, "cannot run git: {source}")
            }
            VersionError::Failed { status, stderr } => {
                write!(f, "`git --version` {status}: {stderr}")
            }
            VersionError::Timeout { after, kill } => {
                write!(f, "`git --version` did not finish within {after:?}")?;
                match kill {
                    Some(error) => write!(f, "; it could not be stopped: {error}"),
                    None => Ok(()),
                }
            }
            VersionError::Unrecognised { output } => {
                write!(f, "`git --version` printed \"{output}\", not a version")
            }
        }
    }
}

impl std::error::Error for VersionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            VersionError::Spawn { source } => Some(source),
            VersionError::Timeout { kill, .. } => kill.as_ref().map(|error| error as _),
            VersionError::Failed { .. } | VersionError::Unrecognised { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests;
