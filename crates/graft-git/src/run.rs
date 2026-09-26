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
//!   can match) and `GIT_OPTIONAL_LOCKS=0`. That last one is a request, not
//!   a guarantee: only `git status` honours it, and porcelain `git diff`
//!   refreshes and rewrites `.git/index` regardless (verified on git 2.54).
//!   Staying out of the user's index is the caller's job — read with
//!   plumbing, which does not refresh. The user's config is kept on purpose:
//!   their diff settings must apply.
//! - **Pipes.** stdin is fed from its own thread and stdout and stderr are
//!   drained on two more, so a child blocked writing one pipe can never
//!   deadlock against the runner waiting on another.
//! - **Cancellation.** [`Stop`] is polled at least every 50 ms. When it says
//!   stop, the child is killed and reaped before [`run`](GitInvocation::run)
//!   returns [`RunError::Stopped`]. A stop already asked when `run` is
//!   called spawns nothing. There are no clock timeouts: stop is the only
//!   cancellation.
//! - **Output cap.** Past the cap the child is killed and what was read is
//!   returned, cut to the cap, with `truncated` set. A capped read is still a
//!   successful one.

use std::ffi::OsString;
use std::fmt;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use graft_core::oid::Oid;

use exec::{Ending, Event, Pipe, apply, collect_rest, drain, kill, os_arg, shell_quote};

use crate::repo::RepoHandle;

pub mod askpass;
mod exec;

/// The longest the runner goes without asking [`Stop`].
const POLL: Duration = Duration::from_millis(50);

/// The default output cap: 64 MiB.
const DEFAULT_CAP: usize = 64 * 1024 * 1024;

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
    /// A fixed flag with raw bytes glued on: `Glued("--path=", bytes)` is
    /// `--path=<bytes>`. For an option whose value is a pathname — not
    /// always UTF-8, and not something that can wait for the `--` that
    /// [`GitArg::Path`] lands after (`cat-file --filters --path=…`).
    Glued(&'static str, Vec<u8>),
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

/// What [`GitInvocation::run_raw`] returns: everything captured, whatever
/// git's exit code was.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RawOutput {
    /// Everything captured, whatever the exit code.
    pub output: GitOutput,
    /// git's own exit code. `None` only when the runner killed the child
    /// itself — a stop, or the output cap — before it could exit on its
    /// own; a real exit, including a non-zero one, is always `Some`.
    pub code: Option<i32>,
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
    /// The stderr sink an earlier clone of this invocation gave
    /// [`GitInvocation::on_stderr_line`] panicked on that earlier run, and
    /// is now unusable: calling a closure whose state might be half-mutated
    /// would be worse than refusing (`KNOWN_GAPS` row 455 — this used to
    /// surface as [`RunError::Spawn`], which reads as "git could not be
    /// started" for what is really an app-side callback bug).
    #[error("git's stderr callback is unusable after an earlier panic: {0}")]
    Sink(io::Error),
    /// git ran, exited zero, and printed something that does not match the
    /// shape the caller parses. Distinct from [`RunError::Spawn`]: git
    /// answered fine, this reader could not make sense of the answer
    /// (m5-history-and-ghost-preview/02 fix round — `Spawn`'s `Display`
    /// reads "could not run git", which is false for this case).
    #[error("git's output did not parse as expected: {0}")]
    Unparsable(String),
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
    /// Extra environment variables, added after the fixed ones in
    /// [`spawn`](Self::spawn). A later entry for the same key overrides an
    /// earlier one, as [`Command::env`] does.
    env: Vec<(OsString, OsString)>,
    /// Where stderr segments go as they arrive, if anywhere.
    stderr_sink: Option<StderrSink>,
}

/// The closure [`GitInvocation::on_stderr_line`] was given, shared so the
/// invocation stays `Clone` (clones share one sink) and `&self` can run it.
#[derive(Clone)]
pub(crate) struct StderrSink(Arc<Mutex<StderrFn>>);

/// What [`GitInvocation::on_stderr_line`] takes.
type StderrFn = Box<dyn FnMut(&[u8]) + Send>;

impl fmt::Debug for StderrSink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("StderrSink(…)")
    }
}

/// `git <args>` as a person would type it: the text behind
/// [`GitInvocation::command_line`], for an argument list that has not been
/// bound to a repository yet (an operation's plan, the Command Log). Arguments
/// are shell-quoted where needed and decoded lossily; never run this.
#[must_use]
pub fn command_line(args: &[GitArg]) -> String {
    let mut line = String::from("git");
    for argument in args {
        line.push(' ');
        let text = match argument {
            GitArg::Flag(flag) => (*flag).to_owned(),
            GitArg::Count(flag, n) => format!("{flag}{n}"),
            GitArg::Oid(oid) => oid.to_string(),
            GitArg::Ref(bytes) | GitArg::Path(bytes) => String::from_utf8_lossy(bytes).into_owned(),
            GitArg::Glued(flag, bytes) => {
                format!("{flag}{}", String::from_utf8_lossy(bytes))
            }
            GitArg::Separator => "--".to_owned(),
            GitArg::Word(word) => word.clone(),
        };
        line.push_str(&shell_quote(&text));
    }
    line
}

impl GitInvocation {
    /// A command with no arguments yet, run in `repo`'s working tree (its git
    /// directory when bare).
    #[must_use]
    pub fn new(repo: &RepoHandle) -> Self {
        let directory = repo.workdir().unwrap_or_else(|| repo.git_dir());
        Self::in_directory(directory)
    }

    /// A command with no arguments yet, run in `directory` directly — for git
    /// surfaces with no open [`RepoHandle`] yet (`git --version`, at
    /// startup) or that need a directory a handle does not give
    /// (`--git-dir` on the common directory, so a commit-graph write reaches
    /// every worktree's shared object store).
    pub(crate) fn in_directory(directory: &Path) -> Self {
        Self {
            directory: directory.to_path_buf(),
            args: Vec::new(),
            stdin: None,
            cap: DEFAULT_CAP,
            env: Vec::new(),
            stderr_sink: None,
        }
    }

    /// Hands git's stderr to `sink` as it arrives, one segment at a time: the
    /// bytes between `\r`/`\n` separators (a progress line git redraws in
    /// place is one segment per redraw), separators left out, empty segments
    /// skipped, order kept, a partial segment carried across pipe reads and
    /// flushed at EOF. [`GitOutput::stderr`] is still the whole stream, under
    /// the same cap; the sink sees exactly the kept bytes.
    ///
    /// The sink runs on the runner's stderr drain thread, not the caller's:
    /// it must be quick and must not block on the thread that called
    /// [`run`](Self::run). After a stop, `run` may return while the drain
    /// thread is still delivering the last segments.
    #[must_use]
    pub fn on_stderr_line(mut self, sink: StderrFn) -> Self {
        self.stderr_sink = Some(StderrSink(Arc::new(Mutex::new(sink))));
        self
    }

    /// Adds one variable to the child's environment, after the fixed ones
    /// (`GIT_TERMINAL_PROMPT`, `LC_ALL`, `GIT_OPTIONAL_LOCKS`): `GIT_INDEX_FILE`,
    /// `GIT_AUTHOR_*` and the like. A later call for the same key overrides an
    /// earlier one.
    #[must_use]
    pub fn env(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    /// Routes git's (and ssh's) credential prompts to `server`: sets
    /// `GIT_ASKPASS` and `SSH_ASKPASS` to its helper, `SSH_ASKPASS_REQUIRE`
    /// to `force` (ssh asks through it even with a terminal), and
    /// `GRAFT_ASKPASS_SOCKET` to its socket, so the helper knows it is in
    /// askpass mode and where to call. `GIT_TERMINAL_PROMPT=0` stays: git
    /// asks the helper, never the terminal. Only these four variables are
    /// added; no credential ever travels in the environment or the arguments,
    /// and the user's own credential helpers still run first.
    #[must_use]
    pub fn askpass(self, server: &askpass::AskPassServer) -> Self {
        self.env("GIT_ASKPASS", server.helper())
            .env("GRAFT_ASKPASS_SOCKET", server.socket_path())
            .env("SSH_ASKPASS", server.helper())
            .env("SSH_ASKPASS_REQUIRE", "force")
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
        // A stop already asked spawns nothing: git given even a moment can
        // finish a write the caller has already said it does not want
        // (m7-power-tools/27).
        if stop.should_stop() {
            return Err(RunError::Stopped);
        }
        let mut child = self.spawn()?;
        self.supervise(&mut child, stop)
    }

    /// Runs the command like [`run`](Self::run), but never turns a non-zero
    /// exit into an error and never discards stdout to keep only a
    /// [`RunError::Exited`]'s `stderr_first_line`. For a command whose exit
    /// code is not just success/failure — `git merge-tree`'s clean (0) vs
    /// conflicts (1) vs a real error (anything else), each of which writes
    /// stdout differently — the caller reads [`RawOutput::code`] and decides
    /// what it means. `git` missing, the run being stopped, or the child
    /// being killed by a signal it did not send itself are still errors:
    /// this only stops turning *git's own* exit code into one.
    ///
    /// # Errors
    /// [`RunError`]: git missing or unstartable, stopped, or killed by a
    /// signal Graft did not send. Never [`RunError::Exited`].
    pub fn run_raw(&self, stop: &dyn Stop) -> Result<RawOutput, RunError> {
        // As in `run`: a stop already asked spawns nothing.
        if stop.should_stop() {
            return Err(RunError::Stopped);
        }
        let mut child = self.spawn()?;
        self.supervise_raw(&mut child, stop)
    }

    /// The command as a person would type it, for footers and error messages
    /// only; never run. Arguments are shell-quoted where needed and decoded
    /// lossily, and the runner's own `-C <dir> --no-pager` is left out.
    #[must_use]
    pub fn command_line(&self) -> String {
        command_line(&self.args)
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
            .envs(
                self.env
                    .iter()
                    .map(|(key, value)| (key.as_os_str(), value.as_os_str())),
            )
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
        match self.outcome(child, stop)? {
            Ending::Exited(status, output) => self.judge(status, output),
            // Killed by the runner: a stdin write cut short by the kill is
            // expected, so the writer is left to finish on its own.
            Ending::Capped(output) => Ok(output),
        }
    }

    /// [`supervise`](Self::supervise)'s [`run_raw`](Self::run_raw) twin:
    /// the same outcome, without [`judge`](Self::judge) turning a non-zero
    /// exit into [`RunError::Exited`] and losing stdout along the way.
    fn supervise_raw(&self, child: &mut Child, stop: &dyn Stop) -> Result<RawOutput, RunError> {
        match self.outcome(child, stop)? {
            Ending::Exited(status, output) => match status.code() {
                Some(code) => Ok(RawOutput {
                    output,
                    code: Some(code),
                }),
                // No code at all means a signal, not an exit git chose —
                // `run_raw`'s contract is git's own exit code, so this stays
                // an error rather than becoming a fabricated `None` code
                // indistinguishable from the runner's own kill below.
                None => Err(RunError::Signalled {
                    command: self.command_line(),
                }),
            },
            Ending::Capped(output) => Ok(RawOutput { output, code: None }),
        }
    }

    /// Feeds, drains and waits on a spawned child, reporting how it ended
    /// without judging the result. On every return path the child has been
    /// reaped: no zombie outlives the call.
    fn outcome(&self, child: &mut Child, stop: &dyn Stop) -> Result<Ending, RunError> {
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
            drain(pipe, self.cap, events.clone(), self.stderr_sink.clone());
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
                Ok(Ending::Exited(status, output))
            }
            Ok(Ending::Capped(output)) => Ok(Ending::Capped(output)),
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

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;

    use graft_fixtures::Shape;

    use super::*;
    use crate::fixture::Fixture;

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
        // Stop on the very first poll, microseconds after the spawn: git has
        // not even finished starting, so this is a kill, not an exit a stop
        // arrived after. A later poll would race a small repository's `log`
        // to its end.
        let result = invocation.supervise(&mut child, &AtomicBool::new(true));
        assert!(matches!(result, Err(RunError::Stopped)), "{result:?}");
        assert!(
            child.try_wait().expect("try_wait").is_some(),
            "the child has exited and been reaped"
        );
    }

    /// A bare remote holding `commits` commits of one new file each (three
    /// objects apiece), and an empty repository to fetch it into.
    fn remote_with_objects(commits: usize) -> (Fixture, Fixture) {
        let work = Fixture::empty_repo();
        for n in 0..commits {
            std::fs::write(work.root().join(format!("f{n}")), format!("{n}\n"))
                .expect("a fixture file writes");
            work.git(&["add", "."]);
            work.git(&["commit", "--quiet", "-m", &format!("c{n}")]);
        }
        let bare = Fixture::bare_repo();
        let bare_path = bare.root().to_str().expect("UTF-8 fixture path");
        work.git(&["push", "--quiet", bare_path, "main"]);
        (bare, Fixture::empty_repo())
    }

    #[test]
    fn stderr_sink_sees_progress_and_capture_is_whole() {
        let (bare, into) = remote_with_objects(20);
        let seen = Arc::new(Mutex::new(Vec::<Vec<u8>>::new()));
        let sink_seen = Arc::clone(&seen);
        let output = GitInvocation::in_directory(into.root())
            .args([
                GitArg::Flag("fetch"),
                GitArg::Flag("--progress"),
                GitArg::Word(bare.root().to_str().expect("UTF-8").to_owned()),
                GitArg::Flag("main"),
            ])
            .on_stderr_line(Box::new(move |segment| {
                sink_seen
                    .lock()
                    .expect("the test's own lock")
                    .push(segment.to_vec());
            }))
            .run(&())
            .expect("the fetch succeeds");

        let seen = seen.lock().expect("the test's own lock").clone();
        assert!(
            seen.iter()
                .any(|segment| crate::remote::parse_progress(segment).is_some()),
            "at least one progress segment: {seen:?}"
        );
        let from_capture: Vec<&[u8]> = output
            .stderr
            .split(|&b| b == b'\r' || b == b'\n')
            .filter(|segment| !segment.is_empty())
            .collect();
        assert_eq!(
            from_capture,
            seen.iter().map(Vec::as_slice).collect::<Vec<_>>(),
            "the sink saw exactly the captured stderr, in order, minus separators"
        );
    }

    /// `KNOWN_GAPS` row 455: a stderr sink poisoned by an earlier run of the
    /// same (cloned) invocation must surface as `RunError::Sink`, not
    /// `RunError::Spawn` — the latter reads as "git could not be started",
    /// which is not what happened.
    #[test]
    fn poisoned_sink_is_not_spawn() {
        let repo = Fixture::with_two_commits();
        let invocation = GitInvocation::in_directory(repo.root())
            .args([
                GitArg::Flag("checkout"),
                GitArg::Word("nonesuch-branch".to_owned()),
            ])
            .on_stderr_line(Box::new(|_| panic!("the sink panics on purpose")));

        // git writes to stderr and exits non-zero; the sink panics on the
        // first segment it is handed, poisoning its mutex. However this
        // first run fails is not the point under test.
        let first = invocation.run(&());
        assert!(
            first.is_err(),
            "a panicking sink cannot let this run succeed"
        );

        // Same (cloned) invocation, same poisoned sink, a second run: the
        // failure now is the sink refusing to run again, not a fresh spawn
        // failure — `git` starts and runs exactly as before.
        let second = invocation.run(&());
        assert!(matches!(second, Err(RunError::Sink(_))), "{second:?}");
    }

    /// Counts the asks; never expects one.
    struct Counting(AtomicUsize);

    impl askpass::AskPass for Counting {
        fn ask(&self, _prompt: &str) -> Option<askpass::Secret> {
            self.0.fetch_add(1, Ordering::SeqCst);
            None
        }
    }

    fn askpass_server() -> (Arc<Counting>, askpass::AskPassServer) {
        let handler = Arc::new(Counting(AtomicUsize::new(0)));
        let server = askpass::AskPassServer::spawn(
            std::path::PathBuf::from("/opt/graft helper"),
            handler.clone(),
        )
        .expect("the askpass server starts");
        (handler, server)
    }

    /// Read back from the child itself: a git alias running `env` shows the
    /// environment git hands the programs it starts, askpass included.
    #[test]
    fn askpass_env_is_set() {
        let (_handler, server) = askpass_server();
        let repo = Fixture::empty_repo();
        let output = GitInvocation::in_directory(repo.root())
            .args([
                GitArg::Flag("-c"),
                GitArg::Flag("alias.showenv=!env"),
                GitArg::Flag("showenv"),
            ])
            .askpass(&server)
            .run(&())
            .expect("git runs the alias");
        let env = String::from_utf8(output.stdout).expect("UTF-8 env");
        let socket = server.socket_path().to_str().expect("UTF-8 socket path");
        for line in [
            "GIT_ASKPASS=/opt/graft helper".to_owned(),
            "SSH_ASKPASS=/opt/graft helper".to_owned(),
            "SSH_ASKPASS_REQUIRE=force".to_owned(),
            format!("GRAFT_ASKPASS_SOCKET={socket}"),
            "GIT_TERMINAL_PROMPT=0".to_owned(),
        ] {
            assert!(env.lines().any(|seen| seen == line), "{line} in:\n{env}");
        }
        server.close().expect("closes cleanly");
    }

    /// Answers every ask with one known password.
    struct Answers(&'static [u8]);

    impl askpass::AskPass for Answers {
        fn ask(&self, _prompt: &str) -> Option<askpass::Secret> {
            Some(askpass::Secret::new(self.0.to_vec()))
        }
    }

    /// After a real exchange over the socket, the password is nowhere the
    /// runner puts things: not the invocation's `Debug`, not its command
    /// line, not the environment git hands its children, not git's output,
    /// not the server's `Debug`. It went to the helper's `out` and nowhere
    /// else. (The full git → helper → app path is proved where the helper
    /// lives, in the app's askpass mode.)
    #[test]
    fn the_password_never_reaches_the_invocation() {
        const PASSWORD: &str = "hunter2-do-not-leak";
        let server = askpass::AskPassServer::spawn(
            std::path::PathBuf::from("/opt/graft"),
            Arc::new(Answers(PASSWORD.as_bytes())),
        )
        .expect("the askpass server starts");
        let mut helper_out = Vec::new();
        askpass::respond(server.socket_path(), "Password:", &mut helper_out)
            .expect("the exchange works");
        assert_eq!(helper_out, format!("{PASSWORD}\n").into_bytes());

        let repo = Fixture::empty_repo();
        let invocation = GitInvocation::in_directory(repo.root())
            .args([
                GitArg::Flag("-c"),
                GitArg::Flag("alias.showenv=!env"),
                GitArg::Flag("showenv"),
            ])
            .askpass(&server)
            .on_stderr_line(Box::new(|_| {}));
        let output = invocation.run(&()).expect("git runs the alias");

        for (place, text) in [
            ("invocation Debug", format!("{invocation:?}")),
            ("command line", invocation.command_line()),
            (
                "child env",
                String::from_utf8_lossy(&output.stdout).into_owned(),
            ),
            (
                "git stderr",
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ),
            ("server Debug", format!("{server:?}")),
        ] {
            assert!(
                !text.contains(PASSWORD),
                "the password leaked into the {place}"
            );
        }
        server.close().expect("closes cleanly");
    }

    /// A remote that needs no credentials never reaches the helper: askpass
    /// only adds a way to answer, it changes nothing else.
    #[test]
    fn askpass_leaves_a_credential_free_remote_alone() {
        let (handler, server) = askpass_server();
        let (bare, into) = remote_with_objects(1);
        let url = format!("file://{}", bare.root().to_str().expect("UTF-8"));
        let output = GitInvocation::in_directory(into.root())
            .args([GitArg::Flag("ls-remote"), GitArg::Word(url)])
            .askpass(&server)
            .run(&())
            .expect("ls-remote over file:// works");
        assert!(
            String::from_utf8_lossy(&output.stdout).contains("refs/heads/main"),
            "{output:?}"
        );
        assert_eq!(handler.0.load(Ordering::SeqCst), 0, "nothing was asked");
        server.close().expect("closes cleanly");
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
