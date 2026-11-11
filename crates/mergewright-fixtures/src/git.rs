//! One hermetic `git`, run to completion or killed.
//!
//! Every `git` the corpus runs goes through here, so the rules PROMPT.md §7.1
//! sets for a test that shells out are set in exactly one place: no global or
//! system config, no ambient repository, no credential prompt, no network, a
//! fixed identity and clock, and a deadline so a wedged child fails the build
//! instead of hanging the suite.
//!
//! No pipes, in either direction. A `fast-import` stream is written to a file
//! and handed over as the child's stdin; the child's stdout and stderr go to
//! files too, read after it exits. That removes the classic deadlock — one
//! side blocked on a full pipe buffer while the other waits for it — which
//! matters here because the deadline is enforced by polling `try_wait`, so
//! nothing is draining a pipe while the child runs.

use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

/// How long any one `git` may take before the build gives up on it.
///
/// PROMPT.md §7.1's 30 seconds. Every command here touches a handful of tiny
/// objects, so reaching this means something is wrong, not slow.
const DEADLINE: Duration = Duration::from_secs(30);

/// How often the deadline is checked while a child runs.
///
/// There is no wait-with-timeout in std, so the choice is this or a reaper
/// thread per command. At a few hundred commands a build, the wasted wakeups
/// are not worth a thread apiece.
const POLL: Duration = Duration::from_millis(2);

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
        /// How it exited. Kept whole rather than rendered, because a caller
        /// that knows a particular exit code is an answer — `symbolic-ref -q`
        /// returning 1 for a detached `HEAD` — needs to read the code.
        status: ExitStatus,
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
///
/// Public so that a test which has to shell out to `git` gets the same
/// isolation and the same deadline as the generator, rather than growing its
/// own half of the rules (PROMPT.md §7.1).
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

/// What a `git` child reads on its standard input.
pub(crate) enum Input<'a> {
    /// Nothing: `/dev/null`.
    Nothing,
    /// A file already on disk, as `fast-import`'s stream is.
    File(&'a Path),
    /// Bytes this process holds, staged through the same scratch directory
    /// the output goes to — a pipe would have to be fed while the deadline
    /// loop waits, and the two do not combine.
    Bytes(&'a [u8]),
}

fn run_with_stdin(
    cwd: &Path,
    args: &[&str],
    stdin_file: Option<&Path>,
) -> Result<Vec<u8>, GitError> {
    run_built(hermetic, cwd, args, &Input::from(stdin_file))
}

impl<'a> From<Option<&'a Path>> for Input<'a> {
    fn from(path: Option<&'a Path>) -> Self {
        match path {
            Some(path) => Input::File(path),
            None => Input::Nothing,
        }
    }
}

/// Runs `git <args>` in `cwd` under [`isolated`] — the machine's config
/// switched off, its clock and identity left alone — with the same 30 s
/// deadline every other command here gets.
///
/// For a test whose commits must be dated now: pass `-c user.name=…` and
/// `-c user.email=…` in `args`, because a nulled global config has no
/// identity in it.
///
/// # Errors
/// [`GitError`] if `git` cannot be started, outlives the deadline, or exits
/// non-zero.
pub fn run_isolated(cwd: &Path, args: &[&str], stdin: Option<&[u8]>) -> Result<Vec<u8>, GitError> {
    let input = match stdin {
        Some(bytes) => Input::Bytes(bytes),
        None => Input::Nothing,
    };
    run_built(isolated, cwd, args, &input)
}

/// Runs `git <args>` in `cwd` as [`hermetic`] does, but stamped `date`
/// instead of [`FIXED_DATE`].
///
/// For a test that builds its own history and needs the commits to come out
/// **in a fixed order**. `git` dates at one-second resolution, so commits
/// made in a burst tie; the graph breaks a date tie with the commit id
/// (DESIGN.md §3.2), and a wall clock puts a different id on every run. A
/// date per commit fixes both the ids and the order. `date` is anything git
/// accepts, `@1609459260 +0000` included.
///
/// # Errors
/// [`GitError`] if `git` cannot be started, outlives the deadline, or exits
/// non-zero.
pub fn run_at(cwd: &Path, args: &[&str], date: &str) -> Result<Vec<u8>, GitError> {
    run_built(
        |cwd, args| {
            let mut command = hermetic(cwd, args);
            command
                .env("GIT_AUTHOR_DATE", date)
                .env("GIT_COMMITTER_DATE", date);
            command
        },
        cwd,
        args,
        &Input::Nothing,
    )
}

/// Runs the command `build` makes, to completion or the deadline.
///
/// `pub(crate)` so the remote fixtures can add their own environment (a
/// `HOME` of their own, an askpass) and still get the deadline and the
/// file-backed output.
pub(crate) fn run_built(
    build: impl FnOnce(&Path, &[&str]) -> Command,
    cwd: &Path,
    args: &[&str],
    stdin: &Input<'_>,
) -> Result<Vec<u8>, GitError> {
    let printed = args.join(" ");
    let fail = |source: io::Error| GitError::Io {
        argv: printed.clone(),
        cwd: cwd.to_path_buf(),
        source,
    };

    // The child writes to files, not to pipes. A pipe would have to be
    // drained while the child runs, and this waits on a deadline instead of
    // blocking on a read — the two do not combine without a reader thread,
    // and a full pipe would deadlock until the deadline killed it.
    let sink = tempfile::Builder::new()
        .prefix("git-output-")
        .tempdir()
        .map_err(&fail)?;
    let out_path = sink.path().join("stdout");
    let err_path = sink.path().join("stderr");

    let stdin = match stdin {
        Input::Nothing => Stdio::null(),
        Input::File(path) => Stdio::from(File::open(path).map_err(&fail)?),
        Input::Bytes(bytes) => {
            let staged = sink.path().join("stdin");
            std::fs::write(&staged, bytes).map_err(&fail)?;
            Stdio::from(File::open(&staged).map_err(&fail)?)
        }
    };

    let mut child = build(cwd, args)
        .stdin(stdin)
        .stdout(Stdio::from(File::create(&out_path).map_err(&fail)?))
        .stderr(Stdio::from(File::create(&err_path).map_err(&fail)?))
        .spawn()
        .map_err(|source| GitError::Spawn {
            argv: printed.clone(),
            cwd: cwd.to_path_buf(),
            source,
        })?;

    let started = Instant::now();
    let status = loop {
        match child.try_wait().map_err(&fail)? {
            Some(status) => break status,
            None if started.elapsed() >= DEADLINE => {
                child.kill().map_err(&fail)?;
                child.wait().map_err(&fail)?;
                return Err(GitError::Timeout {
                    argv: printed,
                    cwd: cwd.to_path_buf(),
                });
            }
            None => std::thread::sleep(POLL),
        }
    };

    let stdout = std::fs::read(&out_path).map_err(&fail)?;
    if status.success() {
        Ok(stdout)
    } else {
        let stderr = std::fs::read(&err_path).map_err(&fail)?;
        Err(GitError::Failed {
            argv: printed,
            cwd: cwd.to_path_buf(),
            status,
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
        })
    }
}

/// A `git` command with the whole outside world switched off: the machine's
/// config, and its clock and identity too.
pub(crate) fn hermetic(cwd: &Path, args: &[&str]) -> Command {
    let mut command = isolated(cwd, args);
    command
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

/// A `git` command that cannot see the machine's configuration and starts
/// no background maintenance — and nothing more.
///
/// The isolation half of [`hermetic`], without its fixed clock and identity.
/// A test that needs commits dated *now* (the acceptance walk, whose graph
/// reads "now" for every row) must not inherit the corpus's 2021, but it must
/// still be immune to a global `commit.gpgsign`, `core.hooksPath` or
/// `status.showUntrackedFiles` — the difference between a suite that passes
/// everywhere and one that passes on the machine that wrote it
/// (PROMPT.md §7.1; m3-07 review, Major 1). Set your own identity with
/// `-c user.name=…` in `args`.
///
/// The returned [`Command`] is unspawned, so the caller can add pipes, a
/// stdin or more environment before running it.
#[must_use]
pub fn isolated(cwd: &Path, args: &[&str]) -> Command {
    let mut command = Command::new("git");
    command
        // No git left working in the background. A commit, merge, rebase or
        // fetch ends by starting `git maintenance run --auto --detach`, and
        // git 2.54's geometric repack can then delete the loose objects and
        // their fan-out folders while the fixture's next command writes into
        // one: `unable to create temporary file`, now and then, under load
        // (pre-publish-cleanup/02). `-c` reaches the child git starts too.
        .args(["-c", "maintenance.auto=false"])
        .args(args)
        .current_dir(cwd)
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
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES");
    command
}

/// The name on every object plain `git` writes here.
pub const IDENTITY_NAME: &str = "Maker Fixture";
/// The address on every object plain `git` writes here.
pub const IDENTITY_EMAIL: &str = "fixture@example.invalid";
/// `IDENTITY_NAME <IDENTITY_EMAIL>`, as `fast-import` and reflog lines spell
/// it. Pinned to the two above by `tests::identity_is_name_and_email`.
pub const IDENTITY: &str = "Maker Fixture <fixture@example.invalid>";
/// The clock every object plain `git` writes here is stamped with.
///
/// Deliberately later than the newest `fast-import` timestamp, so a stash
/// made after the history was imported does not appear to predate it.
pub const FIXED_DATE: &str = "2021-01-01T00:00:00+00:00";

/// `git init` on the terms every fixture in this crate is built on.
///
/// One spelling, shared by the named corpus and by the heavy tier's synthetic
/// repository. `--object-format=sha1` is stated rather than inherited: a
/// fixture records object ids, and a machine configured for sha256 by default
/// would silently produce different ones.
pub fn init_repo(dir: &Path) -> Result<(), GitError> {
    run(
        dir,
        &["init", "-q", "-b", "main", "--object-format=sha1", "."],
    )?;
    Ok(())
}

/// `git --version`, trimmed — part of the corpus digest, because the objects
/// plain `git` writes (a stash, above all) are its output, not ours.
pub fn version() -> Result<String, GitError> {
    let out = run(Path::new("."), &["--version"])?;
    Ok(String::from_utf8_lossy(&out).trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::{IDENTITY, IDENTITY_EMAIL, IDENTITY_NAME};

    #[test]
    fn identity_is_name_and_email() {
        assert_eq!(IDENTITY, format!("{IDENTITY_NAME} <{IDENTITY_EMAIL}>"));
        assert_eq!(crate::stream::IDENTITY, IDENTITY);
    }
}
