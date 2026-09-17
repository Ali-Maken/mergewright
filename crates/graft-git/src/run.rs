//! The typed runner for the system `git` binary (PROMPT.md Technology: "typed
//! command builder, never joined shell strings").
//!
//! Every git process Graft starts for a read the `gix` path lacks (ADR-0011's
//! named gaps: diffs, `--contains`), and every write from M3 on, goes through
//! [`GitInvocation`]. It builds the argument vector from [`GitArg`]s — a path
//! or ref name is handed to git as its raw bytes, never re-parsed by a shell —
//! and it owns the process for its whole life:
//!
//! - **Environment.** The user's own, plus `GIT_TERMINAL_PROMPT=0` (never
//!   block on a credential prompt nobody can see), `LC_ALL=C` (messages Graft
//!   can match) and `GIT_OPTIONAL_LOCKS=0` (a read never takes the index lock
//!   from under the user's own git). The user's config is kept on purpose:
//!   their diff settings must apply.
//! - **Pipes.** stdin is fed from its own thread and stdout and stderr are
//!   drained on two more, so a child blocked writing one pipe can never
//!   deadlock against the runner waiting on another.
//! - **Cancellation.** [`Stop`] is polled at least every 50 ms. When it says
//!   stop, the child is killed and reaped before [`run`](GitInvocation::run)
//!   returns [`RunError::Stopped`]. There are no clock timeouts: stop is the
//!   only cancellation.
//! - **Output cap.** Past the cap the child is killed and what was read is
//!   returned, cut to the cap, with `truncated` set. A capped read is still a
//!   successful one.

use std::ffi::OsString;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStderr, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::Duration;

use graft_core::oid::Oid;

use crate::repo::RepoHandle;

/// The longest the runner goes without asking [`Stop`].
const POLL: Duration = Duration::from_millis(50);

/// The default output cap: 64 MiB.
const DEFAULT_CAP: usize = 64 * 1024 * 1024;

/// How much one read from a pipe takes at a time.
const READ_CHUNK: usize = 64 * 1024;

/// One argument to git, typed by what it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GitArg {
    /// A fixed flag or subcommand: `Flag("--numstat")`, `Flag("diff-tree")`.
    Flag(&'static str),
    /// A flag with a number glued on: `Count("-U", 3)` is `-U3`.
    Count(&'static str, u32),
    /// A full object id, in hex.
    Oid(Oid),
    /// A ref name, as its raw bytes.
    Ref(Vec<u8>),
    /// A path, as its raw bytes. Always lands after `--`: if the caller has
    /// not added a [`GitArg::Separator`] before the first path, the runner
    /// adds one, so a path can never be read as an option.
    Path(Vec<u8>),
    /// `--`, the end of options.
    Separator,
    /// An argument the caller built, such as a `--format=` string. Never a
    /// path or a ref: those have their own variants.
    Word(String),
}

/// Whether a running command should be abandoned. Polled from the thread that
/// called [`GitInvocation::run`].
pub trait Stop: Sync {
    /// `true` once the caller no longer wants the result.
    fn should_stop(&self) -> bool;
}

impl Stop for AtomicBool {
    fn should_stop(&self) -> bool {
        self.load(Ordering::Relaxed)
    }
}

/// Never stops.
impl Stop for () {
    fn should_stop(&self) -> bool {
        false
    }
}

/// What a command printed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GitOutput {
    /// Standard output, at most the cap.
    pub stdout: Vec<u8>,
    /// Standard error, at most the cap.
    pub stderr: Vec<u8>,
    /// The output reached the cap: the child was killed and the rest is
    /// missing.
    pub truncated: bool,
}

/// Why a command did not give a result.
#[derive(Debug, thiserror::Error)]
pub enum RunError {
    /// There is no `git` on the `PATH`.
    #[error("git is not installed, or not on the PATH")]
    GitNotFound,
    /// git could not be started, or talking to it failed.
    #[error("could not run git: {0}")]
    Spawn(io::Error),
    /// The caller's [`Stop`] asked; the child was killed and reaped.
    #[error("git was stopped")]
    Stopped,
    /// git exited non-zero.
    #[error("`{command}` failed ({status}): {stderr_first_line}")]
    Exited {
        /// The exit code.
        status: i32,
        /// The first non-empty line git wrote to stderr.
        stderr_first_line: String,
        /// [`GitInvocation::command_line`], for the message.
        command: String,
    },
    /// git was killed by a signal Graft did not send.
    #[error("`{command}` was killed by a signal")]
    Signalled {
        /// [`GitInvocation::command_line`], for the message.
        command: String,
    },
}

/// A git command, built from typed arguments and run against one repository.
#[derive(Clone, Debug)]
pub struct GitInvocation {
    /// Where `-C` points: the working tree, or the git directory when bare.
    directory: std::path::PathBuf,
    args: Vec<GitArg>,
    stdin: Option<Vec<u8>>,
    cap: usize,
}

impl GitInvocation {
    /// A command with no arguments yet, run in `repo`'s working tree (its git
    /// directory when bare).
    #[must_use]
    pub fn new(repo: &RepoHandle) -> Self {
        let directory = repo.workdir().unwrap_or_else(|| repo.git_dir());
        Self::in_directory(directory)
    }

    fn in_directory(directory: &Path) -> Self {
        Self {
            directory: directory.to_path_buf(),
            args: Vec::new(),
            stdin: None,
            cap: DEFAULT_CAP,
        }
    }

    /// Adds one argument. The first [`GitArg::Path`] gets a `--` before it if
    /// the caller has not added one.
    #[must_use]
    pub fn arg(mut self, a: GitArg) -> Self {
        if matches!(a, GitArg::Path(_)) && !self.args.contains(&GitArg::Separator) {
            self.args.push(GitArg::Separator);
        }
        self.args.push(a);
        self
    }

    /// Adds arguments in order, as [`arg`](Self::arg) does.
    #[must_use]
    pub fn args(self, a: impl IntoIterator<Item = GitArg>) -> Self {
        a.into_iter().fold(self, Self::arg)
    }

    /// Bytes to feed git's stdin. Without this, stdin is closed.
    #[must_use]
    pub fn stdin(mut self, bytes: Vec<u8>) -> Self {
        self.stdin = Some(bytes);
        self
    }

    /// The most bytes kept from each of stdout and stderr (default 64 MiB).
    #[must_use]
    pub fn output_cap(mut self, bytes: usize) -> Self {
        self.cap = bytes;
        self
    }

    /// Runs the command to the end, until `stop` says so, or until the output
    /// reaches the cap.
    ///
    /// # Errors
    /// [`RunError`]: git missing or unstartable, stopped, exiting non-zero or
    /// killed by a signal.
    pub fn run(&self, stop: &dyn Stop) -> Result<GitOutput, RunError> {
        let mut child = self.spawn()?;
        self.supervise(&mut child, stop)
    }

    /// The command as a person would type it, for footers and error messages
    /// only; never run. Arguments are shell-quoted where needed and decoded
    /// lossily, and the runner's own `-C <dir> --no-pager` is left out.
    #[must_use]
    pub fn command_line(&self) -> String {
        let mut line = String::from("git");
        for argument in &self.args {
            line.push(' ');
            let text = match argument {
                GitArg::Flag(flag) => (*flag).to_owned(),
                GitArg::Count(flag, n) => format!("{flag}{n}"),
                GitArg::Oid(oid) => oid.to_string(),
                GitArg::Ref(bytes) | GitArg::Path(bytes) => {
                    String::from_utf8_lossy(bytes).into_owned()
                }
                GitArg::Separator => "--".to_owned(),
                GitArg::Word(word) => word.clone(),
            };
            line.push_str(&shell_quote(&text));
        }
        line
    }

    fn spawn(&self) -> Result<Child, RunError> {
        let mut command = Command::new("git");
        command
            .arg("-C")
            .arg(&self.directory)
            .arg("--no-pager")
            .args(self.args.iter().map(os_arg))
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("LC_ALL", "C")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .stdin(if self.stdin.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command.spawn().map_err(|error| match error.kind() {
            io::ErrorKind::NotFound => RunError::GitNotFound,
            _ => RunError::Spawn(error),
        })
    }

    /// Feeds, drains and waits on a spawned child. On every return path the
    /// child has been reaped: no zombie outlives the call.
    fn supervise(&self, child: &mut Child, stop: &dyn Stop) -> Result<GitOutput, RunError> {
        let (events, received) = mpsc::channel();
        let stdin = self
            .stdin
            .clone()
            .zip(child.stdin.take())
            .map(|(bytes, mut pipe)| {
                thread::spawn(move || {
                    let written = pipe.write_all(&bytes);
                    drop(pipe); // EOF, so git stops waiting for more
                    written
                })
            });
        let stdout = child.stdout.take().map(Pipe::Out);
        let stderr = child.stderr.take().map(Pipe::Err);
        let mut open = 0;
        for pipe in [stdout, stderr].into_iter().flatten() {
            open += 1;
            drain(pipe, self.cap, events.clone());
        }
        drop(events);

        match Self::wait(child, stop, &received, open) {
            Ok(Ending::Exited(status, output)) => {
                if let Some(writer) = stdin {
                    let written = writer.join().map_err(|_| {
                        RunError::Spawn(io::Error::other("the stdin thread panicked"))
                    })?;
                    match written {
                        Ok(()) => {}
                        // git exited without reading all of its input; its
                        // exit status below says whether that was a failure.
                        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => {}
                        Err(error) => return Err(RunError::Spawn(error)),
                    }
                }
                self.judge(status, output)
            }
            // Killed by the runner: a stdin write cut short by the kill is
            // expected, so the writer is left to finish on its own.
            Ok(Ending::Capped(output)) => Ok(output),
            Err(error) => Err(error),
        }
    }

    /// Waits for the child to exit, killing it on stop or at the cap.
    fn wait(
        child: &mut Child,
        stop: &dyn Stop,
        received: &Receiver<Event>,
        mut open: usize,
    ) -> Result<Ending, RunError> {
        let mut output = GitOutput::default();
        loop {
            if stop.should_stop() {
                kill(child)?;
                return Err(RunError::Stopped);
            }
            if output.truncated {
                kill(child)?;
                // The capped reader has already reported; the other pipe
                // reaches EOF now its writer is dead.
                collect_rest(received, &mut output, open);
                return Ok(Ending::Capped(output));
            }
            if open == 0 {
                match child.try_wait().map_err(RunError::Spawn)? {
                    Some(status) => return Ok(Ending::Exited(status, output)),
                    // Both pipes closed and git still running: it is exiting.
                    None => thread::sleep(Duration::from_millis(1)),
                }
                continue;
            }
            match received.recv_timeout(POLL) {
                Ok(event) => {
                    open -= 1;
                    apply(event, &mut output)?;
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    // Every reader sends exactly once before it drops its
                    // sender, so this means a reader thread panicked.
                    kill(child)?;
                    return Err(RunError::Spawn(io::Error::other(
                        "a git output reader panicked",
                    )));
                }
            }
        }
    }

    fn judge(&self, status: ExitStatus, output: GitOutput) -> Result<GitOutput, RunError> {
        if status.success() {
            return Ok(output);
        }
        Err(match status.code() {
            Some(code) => RunError::Exited {
                status: code,
                stderr_first_line: crate::commit_graph::first_line(&output.stderr),
                command: self.command_line(),
            },
            None => RunError::Signalled {
                command: self.command_line(),
            },
        })
    }
}

enum Ending {
    Exited(ExitStatus, GitOutput),
    Capped(GitOutput),
}

enum Pipe {
    Out(ChildStdout),
    Err(ChildStderr),
}

/// What a reader thread reports, once, when it stops reading.
struct Event {
    stdout: bool,
    read: io::Result<(Vec<u8>, bool)>,
}

/// Reads one pipe to EOF or to `cap` on its own thread, then reports.
fn drain(pipe: Pipe, cap: usize, events: Sender<Event>) {
    thread::spawn(move || {
        let (stdout, read) = match pipe {
            Pipe::Out(mut out) => (true, read_capped(&mut out, cap)),
            Pipe::Err(mut err) => (false, read_capped(&mut err, cap)),
        };
        if let Err(unwanted) = events.send(Event { stdout, read }) {
            // The receiver is gone only when `run` has already returned
            // (stopped or failed) and no longer wants this output.
            drop(unwanted);
        }
    });
}

/// Reads up to `cap` bytes; `true` alongside if there was more.
fn read_capped(pipe: &mut impl Read, cap: usize) -> io::Result<(Vec<u8>, bool)> {
    let mut kept = Vec::new();
    let mut chunk = vec![0; READ_CHUNK];
    loop {
        let n = match pipe.read(&mut chunk) {
            Ok(0) => return Ok((kept, false)),
            Ok(n) => n,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        if kept.len() + n > cap {
            kept.extend_from_slice(&chunk[..cap - kept.len()]);
            return Ok((kept, true));
        }
        kept.extend_from_slice(&chunk[..n]);
    }
}

fn apply(event: Event, output: &mut GitOutput) -> Result<(), RunError> {
    let (bytes, capped) = event.read.map_err(RunError::Spawn)?;
    if event.stdout {
        output.stdout = bytes;
    } else {
        output.stderr = bytes;
    }
    output.truncated |= capped;
    Ok(())
}

/// After a kill for the cap: gathers the other pipe, if it reports promptly.
/// A grandchild git started (an external diff tool) can hold a pipe open
/// past the kill; its output is then simply not collected.
fn collect_rest(received: &Receiver<Event>, output: &mut GitOutput, mut open: usize) {
    while open > 0 {
        match received.recv_timeout(POLL) {
            Ok(event) => {
                open -= 1;
                if let Err(error) = apply(event, output) {
                    // The capped result is still good; the other pipe's read
                    // failing after the kill is a consequence of the kill.
                    drop(error);
                }
            }
            Err(_) => return,
        }
    }
}

/// Kills and reaps the child. A child that already exited is only reaped.
fn kill(child: &mut Child) -> Result<(), RunError> {
    if let Err(error) = child.kill()
        && child.try_wait().map_err(RunError::Spawn)?.is_none()
    {
        return Err(RunError::Spawn(error));
    }
    child.wait().map(drop).map_err(RunError::Spawn)
}

fn os_arg(argument: &GitArg) -> OsString {
    match argument {
        GitArg::Flag(flag) => OsString::from(flag),
        GitArg::Count(flag, n) => OsString::from(format!("{flag}{n}")),
        GitArg::Oid(oid) => OsString::from(oid.to_string()),
        GitArg::Ref(bytes) | GitArg::Path(bytes) => os_bytes(bytes),
        GitArg::Separator => OsString::from("--"),
        GitArg::Word(word) => OsString::from(word),
    }
}

#[cfg(unix)]
fn os_bytes(bytes: &[u8]) -> OsString {
    use std::os::unix::ffi::OsStringExt;
    OsString::from_vec(bytes.to_vec())
}

#[cfg(not(unix))]
fn os_bytes(bytes: &[u8]) -> OsString {
    // Windows arguments are UTF-16; git for Windows reads paths as UTF-8.
    OsString::from(String::from_utf8_lossy(bytes).into_owned())
}

/// Quotes `text` for a POSIX shell when it holds anything beyond the plainly
/// safe characters.
fn shell_quote(text: &str) -> String {
    let plain = !text.is_empty()
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_./=:@%+,^~".contains(&b));
    if plain {
        text.to_owned()
    } else {
        format!("'{}'", text.replace('\'', r"'\''"))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;

    use graft_fixtures::Shape;

    use super::*;

    /// Says stop from its second poll on.
    struct SecondPoll(AtomicUsize);

    impl Stop for SecondPoll {
        fn should_stop(&self) -> bool {
            self.0.fetch_add(1, Ordering::Relaxed) >= 1
        }
    }

    #[test]
    fn a_stopped_command_is_killed_and_reaped() {
        let invocation =
            GitInvocation::in_directory(&graft_fixtures::borrow(Shape::Linear)).args([
                GitArg::Flag("log"),
                GitArg::Flag("-p"),
                GitArg::Flag("--all"),
                GitArg::Flag("--no-color"),
            ]);
        let mut child = invocation.spawn().expect("git starts");
        let stop = SecondPoll(AtomicUsize::new(0));
        let result = invocation.supervise(&mut child, &stop);
        assert!(matches!(result, Err(RunError::Stopped)), "{result:?}");
        assert!(
            child.try_wait().expect("try_wait").is_some(),
            "the child has exited and been reaped"
        );
    }

    #[test]
    fn command_line_quotes_what_a_shell_would_split() {
        let line = GitInvocation::in_directory(Path::new("."))
            .args([
                GitArg::Flag("log"),
                GitArg::Word("--format=%H %s".to_owned()),
                GitArg::Path(b"it's.txt".to_vec()),
            ])
            .command_line();
        assert_eq!(line, r"git log '--format=%H %s' -- 'it'\''s.txt'");
    }
}
