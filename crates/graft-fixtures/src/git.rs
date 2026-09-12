//! One hermetic `git`, run to completion or killed.
//!
//! Every `git` the corpus runs goes through here, so the rules PROMPT.md §7.1
//! sets for a test that shells out are set in exactly one place: no global or
//! system config, no ambient repository, no credential prompt, no network, a
//! fixed identity and clock, and a deadline so a wedged child fails the build
//! instead of hanging the suite.
//!
//! Nothing here reads stdin from a pipe. A `fast-import` stream is written to
//! a file first and handed over as the child's stdin, which removes the
//! classic deadlock — parent blocked writing a stream larger than the pipe
//! buffer while the child blocks writing to a stderr nobody is draining.

use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// How long any one `git` may take before the build gives up on it.
///
/// PROMPT.md §7.1's 30 seconds. Every command here touches a handful of tiny
/// objects, so reaching this means something is wrong, not slow.
const DEADLINE: Duration = Duration::from_secs(30);

/// Why a `git` command did not produce an answer.
#[derive(Debug, thiserror::Error)]
pub enum GitError {
    /// The child could not be started at all — usually no `git` on `PATH`.
    #[error("`git {argv}` could not be started in {}: {source}", cwd.display())]
    Spawn {
        /// The arguments, space-joined, as they would be typed.
        argv: String,
        /// The directory the command was to run in.
        cwd: PathBuf,
        /// What the operating system said.
        source: io::Error,
    },
    /// The child was still running after [`DEADLINE`] and was killed.
    #[error("`git {argv}` in {} did not finish within {DEADLINE:?}", cwd.display())]
    Timeout {
        /// The arguments, space-joined, as they would be typed.
        argv: String,
        /// The directory the command ran in.
        cwd: PathBuf,
    },
    /// The child ran and exited non-zero.
    #[error("`git {argv}` in {} failed ({status}):\n{stderr}", cwd.display())]
    Failed {
        /// The arguments, space-joined, as they would be typed.
        argv: String,
        /// The directory the command ran in.
        cwd: PathBuf,
        /// The exit status, rendered.
        status: String,
        /// Everything the child wrote to stderr.
        stderr: String,
    },
    /// The child's state or output could not be read, or its stdin file could
    /// not be opened.
    #[error("`git {argv}` in {}: {source}", cwd.display())]
    Io {
        /// The arguments, space-joined, as they would be typed.
        argv: String,
        /// The directory the command ran in.
        cwd: PathBuf,
        /// What the operating system said.
        source: io::Error,
    },
}

/// Runs `git <args>` in `cwd` and returns its stdout.
pub fn run(cwd: &Path, args: &[&str]) -> Result<Vec<u8>, GitError> {
    run_with_stdin(cwd, args, None)
}

/// Runs `git <args>` in `cwd` with `stdin_file` as its standard input.
///
/// Used for `fast-import`: the stream is a file on disk, never a pipe this
/// process has to keep feeding.
pub fn run_with_stdin_file(
    cwd: &Path,
    args: &[&str],
    stdin_file: &Path,
) -> Result<Vec<u8>, GitError> {
    run_with_stdin(cwd, args, Some(stdin_file))
}

fn run_with_stdin(
    cwd: &Path,
    args: &[&str],
    stdin_file: Option<&Path>,
) -> Result<Vec<u8>, GitError> {
    let printed = args.join(" ");
    let fail = |source: io::Error| GitError::Io {
        argv: printed.clone(),
        cwd: cwd.to_path_buf(),
        source,
    };

    let stdin = match stdin_file {
        Some(path) => Stdio::from(File::open(path).map_err(&fail)?),
        None => Stdio::null(),
    };

    let mut child = hermetic(cwd, args)
        .stdin(stdin)
        .spawn()
        .map_err(|source| GitError::Spawn {
            argv: printed.clone(),
            cwd: cwd.to_path_buf(),
            source,
        })?;

    let started = Instant::now();
    loop {
        match child.try_wait().map_err(&fail)? {
            Some(_) => break,
            None if started.elapsed() >= DEADLINE => {
                child.kill().map_err(&fail)?;
                child.wait().map_err(&fail)?;
                return Err(GitError::Timeout {
                    argv: printed,
                    cwd: cwd.to_path_buf(),
                });
            }
            // Both pipes are drained below. Every command here writes far
            // less than a pipe buffer, so neither can fill while we wait.
            None => std::thread::sleep(Duration::from_millis(2)),
        }
    }

    let output = child.wait_with_output().map_err(&fail)?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(GitError::Failed {
            argv: printed,
            cwd: cwd.to_path_buf(),
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

/// A `git` command with the whole outside world switched off.
fn hermetic(cwd: &Path, args: &[&str]) -> Command {
    let mut command = Command::new("git");
    command
        .args(args)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // No prompt, no global or system config, no ambient repository: the
        // corpus must come out the same on every machine (PROMPT.md §7.1).
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .env("GIT_ATTR_NOSYSTEM", "1")
        .env("LC_ALL", "C")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
        // A fixed identity and clock. Commits made by `fast-import` carry
        // their own timestamps from the stream; these cover the few objects
        // plain `git` creates — the stash commits, above all — so their ids
        // are stable too.
        .env("GIT_AUTHOR_NAME", IDENTITY_NAME)
        .env("GIT_AUTHOR_EMAIL", IDENTITY_EMAIL)
        .env("GIT_AUTHOR_DATE", FIXED_DATE)
        .env("GIT_COMMITTER_NAME", IDENTITY_NAME)
        .env("GIT_COMMITTER_EMAIL", IDENTITY_EMAIL)
        .env("GIT_COMMITTER_DATE", FIXED_DATE);
    command
}

/// The name on every object plain `git` writes here.
pub const IDENTITY_NAME: &str = "Graft Fixture";
/// The address on every object plain `git` writes here.
pub const IDENTITY_EMAIL: &str = "fixture@example.invalid";
/// The clock every object plain `git` writes here is stamped with.
///
/// Deliberately later than the newest `fast-import` timestamp, so a stash
/// made after the history was imported does not appear to predate it.
pub const FIXED_DATE: &str = "2021-01-01T00:00:00+00:00";

/// `git --version`, trimmed — part of the corpus digest, because the objects
/// plain `git` writes (a stash, above all) are its output, not ours.
pub fn version() -> Result<String, GitError> {
    let out = run(Path::new("."), &["--version"])?;
    Ok(String::from_utf8_lossy(&out).trim().to_owned())
}
