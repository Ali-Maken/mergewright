//! The repository's commit-graph file: whether it has one, building one with
//! the system `git`, and whether the user declined Graft's offer to.
//!
//! Without a commit-graph, history sorts on every open (ADR-0008), so Graft
//! offers — once per repository, a decline remembered — to run
//! `git commit-graph write --reachable`. gix has no writer, and PROMPT.md §6.3
//! makes the user's own `git` the only thing Graft runs.
//!
//! **The write is never killed.** git writes through a `.lock` file and
//! renames it into place, so a finished write is atomic and an interrupted
//! one is git's to clean up. Killing the child (`SIGKILL`) would leave a
//! stale `.lock` behind and break the next write. If the user quits while it
//! runs, the job runner abandons the worker after its shutdown grace and the
//! `git` child finishes on its own.
//!
//! **Outside the undo journal, by name** (ADR-0008, "Landed"): a commit-graph
//! is a cache, not history; there is nothing to undo.
//!
//! **The decline lives in the common directory**, `<common_dir>/graft/
//! commit-graph-offer`, not under `git rev-parse --git-path` (which is per
//! worktree): the commit-graph belongs to the shared object store, so one
//! decline covers every worktree.

use std::fmt;
use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::repo::RepoHandle;
use crate::run::{GitArg, GitInvocation, RunError};

/// Whether the repository has a commit-graph the walk will use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommitGraphStatus {
    /// A commit-graph file covering this many commits.
    Present {
        /// Commits the file holds.
        commits: u32,
    },
    /// No commit-graph file (or `core.commitGraph` is off, which makes it the
    /// same thing to the walk).
    Absent,
}

/// What a finished `git commit-graph write` left behind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Written {
    /// Commits the new file holds, read back through gix.
    pub commits: u32,
    /// How long git took.
    pub elapsed: Duration,
}

/// Why reading or writing the commit-graph failed.
#[derive(Debug)]
pub enum CommitGraphError {
    /// No `git` could be started.
    GitNotFound(io::Error),
    /// git ran and exited non-zero. `stderr_first_line` is git's own words,
    /// raw: the app sanitises before drawing.
    Exited {
        /// The exit code.
        status: i32,
        /// The first non-empty line git wrote to stderr.
        stderr_first_line: String,
    },
    /// git was ended by a signal, not by itself.
    Signalled,
    /// Starting git or waiting on it failed for another reason.
    Io(io::Error),
    /// gix could not read the commit-graph file (on `status`, or reading it
    /// back after a write). A corrupt file is this, never `Absent`.
    Graph(String),
}

impl fmt::Display for CommitGraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommitGraphError::GitNotFound(error) => {
                write!(
                    f,
                    "commit-graph write failed: git could not be run: {error}"
                )
            }
            CommitGraphError::Exited {
                status,
                stderr_first_line,
            } => write!(
                f,
                "commit-graph write failed (exit {status}): {stderr_first_line}"
            ),
            CommitGraphError::Signalled => {
                write!(f, "commit-graph write failed: git was ended by a signal")
            }
            CommitGraphError::Io(error) => write!(f, "commit-graph write failed: {error}"),
            CommitGraphError::Graph(message) => write!(f, "commit-graph unreadable: {message}"),
        }
    }
}

impl std::error::Error for CommitGraphError {}

/// Whether `repo` has a commit-graph the walk will use.
///
/// Asks gix exactly as [`CommitTable::open`](crate::history::CommitTable::open)
/// does (`core.commitGraph` respected), so "absent" here means the walk sorts
/// without one.
///
/// # Errors
/// [`CommitGraphError::Graph`] when a file is there but cannot be read.
pub fn status(repo: &RepoHandle) -> Result<CommitGraphStatus, CommitGraphError> {
    match repo.gix().commit_graph_if_enabled() {
        Ok(Some(graph)) => Ok(CommitGraphStatus::Present {
            commits: graph.num_commits(),
        }),
        Ok(None) => Ok(CommitGraphStatus::Absent),
        Err(error) => Err(CommitGraphError::Graph(error.to_string())),
    }
}

/// Runs `git -C <common_dir> commit-graph write --reachable --no-progress`
/// through [`GitInvocation`] and waits for it, then reads the new file back.
///
/// Blocking: a job's work. Never kills the child (module doc) — run with
/// `&()`, the [`Stop`](crate::run::Stop) that never says stop.
///
/// # Errors
/// [`CommitGraphError`]: git missing, exiting non-zero or signalled; the file
/// unreadable afterwards; or a write that "succeeded" and left no file.
pub fn write(repo: &RepoHandle) -> Result<Written, CommitGraphError> {
    let started = Instant::now();
    let invocation = GitInvocation::in_directory(repo.common_dir()).args([
        GitArg::Flag("commit-graph"),
        GitArg::Flag("write"),
        GitArg::Flag("--reachable"),
        GitArg::Flag("--no-progress"),
    ]);
    invocation
        .run(&())
        .map_err(run_error_to_commit_graph_error)?;
    let elapsed = started.elapsed();

    match status(repo)? {
        CommitGraphStatus::Present { commits } => Ok(Written { commits, elapsed }),
        // git said it wrote one and gix sees none: `core.commitGraph` off, or
        // a git that writes somewhere gix does not look. Not a success.
        CommitGraphStatus::Absent => Err(CommitGraphError::Graph(
            "git wrote a commit-graph but it is not in use (is core.commitGraph off?)".to_owned(),
        )),
    }
}

/// Reads [`RunError`] into the shape `write`'s callers already know.
/// `RunError::Stopped` cannot occur here: `write` runs with `&()`, which
/// never asks for a stop.
fn run_error_to_commit_graph_error(error: RunError) -> CommitGraphError {
    match error {
        RunError::GitNotFound => CommitGraphError::GitNotFound(io::Error::new(
            io::ErrorKind::NotFound,
            "git is not installed, or not on the PATH",
        )),
        // `write` never calls `on_stderr_line`, so `Sink` can only mean a
        // future caller added one; treated the same as `Spawn` — both are
        // "talking to git failed" — rather than left unhandled.
        RunError::Spawn(source) | RunError::Sink(source) => CommitGraphError::Io(source),
        // `write` never parses git's output itself (it re-reads the graph
        // file through `status` instead), so this never fires in practice;
        // handled for exhaustiveness rather than left a `todo!`.
        RunError::Unparsable(reason) => CommitGraphError::Io(io::Error::other(reason)),
        RunError::Stopped => CommitGraphError::Io(io::Error::other(
            "write was stopped, which never asks to be",
        )),
        RunError::Exited {
            status,
            stderr_first_line,
            ..
        } => CommitGraphError::Exited {
            status,
            stderr_first_line,
        },
        RunError::Signalled { .. } => CommitGraphError::Signalled,
    }
}

/// The first non-empty line of git's stderr, lossily decoded, trimmed.
pub(crate) fn first_line(stderr: &[u8]) -> String {
    String::from_utf8_lossy(stderr)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("git gave no reason")
        .to_owned()
}

/// What the user said to Graft's offer to build a commit-graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Offer {
    /// Nothing recorded: offer.
    Undecided,
    /// "Never for this repository."
    Declined,
}

/// Where the decline is remembered: `<common_dir>/graft/commit-graph-offer`.
fn offer_file(repo: &RepoHandle) -> PathBuf {
    repo.common_dir().join("graft").join("commit-graph-offer")
}

/// Whether the user declined the offer for this repository (any worktree).
///
/// # Errors
/// Any error reading the file other than its absence.
pub fn offer_state(repo: &RepoHandle) -> Result<Offer, io::Error> {
    match std::fs::metadata(offer_file(repo)) {
        Ok(_) => Ok(Offer::Declined),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Offer::Undecided),
        Err(error) => Err(error),
    }
}

/// Remembers "never for this repository": writes `declined\n`, creating
/// `graft/` in the common directory.
///
/// # Errors
/// The directory or the file could not be written.
pub fn remember_declined(repo: &RepoHandle) -> Result<(), io::Error> {
    let file = offer_file(repo);
    if let Some(directory) = file.parent() {
        std::fs::create_dir_all(directory)?;
    }
    std::fs::write(file, "declined\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_non_empty_stderr_line_is_kept_trimmed() {
        assert_eq!(first_line(b"\n  fatal: no  \nmore\n"), "fatal: no");
        assert_eq!(first_line(b""), "git gave no reason");
    }

    #[test]
    fn an_exit_reads_as_git_said_it() {
        let error = CommitGraphError::Exited {
            status: 128,
            stderr_first_line: "fatal: nope".to_owned(),
        };
        assert_eq!(
            error.to_string(),
            "commit-graph write failed (exit 128): fatal: nope"
        );
    }
}
