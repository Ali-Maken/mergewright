//! The pty harness the hang-up tests drive (m8/13g, split out in m8/13l):
//! the real binary as a pty **session leader**, the pty its controlling
//! terminal, so closing the master is a real hang-up — the kernel sends
//! `SIGHUP` and every later read of the terminal fails.
//!
//! Every pid a test learns (Graft's, git's, the hook's) is held by a
//! [`Leftovers`] guard that kills it on drop, by pid, never by pattern.

use std::ffi::OsString;
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{AsFd, OwnedFd};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use nix::poll::{PollFd, PollFlags, PollTimeout, poll};
use nix::pty::{Winsize, openpty};

use super::stuck::alive;
use crate::pty::{Isolated, Screen};

/// How long a wait for the screen may take.
pub(super) const PATIENCE: Duration = Duration::from_secs(20);

/// Pids to kill when the test ends, whatever it did: a regression must not
/// leave `sleep 600` (or git, or Graft) behind.
#[derive(Default)]
pub(super) struct Leftovers(Vec<u32>);

impl Leftovers {
    pub(super) fn hold(&mut self, pid: u32) {
        self.0.push(pid);
    }
}

impl Drop for Leftovers {
    fn drop(&mut self) {
        for pid in &self.0 {
            if alive(*pid) {
                // A pid that ended between the check and the kill makes
                // `kill` fail, which is what we wanted; only a `kill` that
                // could not run at all is worth a word.
                if let Err(error) = Command::new("kill").args(["-9", &pid.to_string()]).status() {
                    eprintln!("could not kill leftover {pid}: {error}");
                }
            }
        }
    }
}

/// Where Graft's stderr goes.
pub(super) enum Stderr {
    /// On the pty, as a shell runs it: dead once the terminal closes.
    Pty,
    /// A file, so a test can read the exit's lines after the hang-up.
    File(PathBuf),
}

/// `graft` as the session leader of a fresh pty.
pub(super) struct Session {
    master: Option<File>,
    child: Child,
    output: Arc<Mutex<Vec<u8>>>,
    reading: Arc<AtomicBool>,
    reader: Option<JoinHandle<()>>,
    size: (u16, u16),
}

impl Session {
    pub(super) fn spawn(
        isolated: &Isolated,
        repo: &Path,
        size: (u16, u16),
        stderr: &Stderr,
    ) -> Self {
        std::fs::create_dir_all(isolated.config()).expect("config home");
        std::fs::create_dir_all(isolated.home()).expect("home");
        let pty = openpty(
            Some(&Winsize {
                ws_row: size.1,
                ws_col: size.0,
                ws_xpixel: 0,
                ws_ypixel: 0,
            }),
            None,
        )
        .expect("a pty");
        // Close-on-exec, so neither the re-exec'd test binary nor Graft
        // holds the master: left open there, it would keep the terminal
        // alive after the test closes it.
        rustix::io::fcntl_setfd(&pty.master, rustix::io::FdFlags::CLOEXEC)
            .expect("the master is close-on-exec");
        let mut command = session_command(
            Path::new(env!("CARGO_BIN_EXE_trib")),
            &[repo.as_os_str().to_owned()],
            pty.slave,
        );
        // git's dates for what the app writes itself — a merge over his edit
        // sets it aside first, and `git stash create` stamps `S` — pinned to
        // the instant the captures of `preview::forked` are drawn at, so
        // `S`'s id and age are the same every run (19d-a3, ticket §R7.2).
        // The wall clock gave a new id per run, drawn `future`.
        let git_date = format!("@{} +0000", crate::preview::FORK_NOW);
        crate::pty::as_the_test_terminal(&mut command)
            .env("XDG_CONFIG_HOME", isolated.config())
            .env("HOME", isolated.home())
            .env("GIT_CEILING_DIRECTORIES", repo)
            .env("GIT_AUTHOR_DATE", &git_date)
            .env("GIT_COMMITTER_DATE", &git_date)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("TRIB_TRACE");
        if let Stderr::File(path) = stderr {
            command
                .stderr(Stdio::from(File::create(path).expect("a stderr file")))
                .env(KEEP_STDERR_VAR, "1");
        }
        let child = command.spawn().expect("graft starts");
        let master = File::from(pty.master);
        let (output, reading, reader) = read_into_memory(&master);
        Self {
            master: Some(master),
            child,
            output,
            reading,
            reader: Some(reader),
            size,
        }
    }
}

/// The program [`session_exec_entry`] execs; its arguments are in
/// `<this>_ARG_<n>`, their count in `<this>_ARGC`. Never in libtest's argv,
/// which libtest would read as filters.
pub(crate) const SESSION_EXEC_VAR: &str = "TRIB_IT_SESSION_EXEC";

/// Set when the caller gave the child its own stderr (a file): fd 2 is then
/// kept rather than pointed at the pty.
const KEEP_STDERR_VAR: &str = "TRIB_IT_SESSION_EXEC_KEEP_STDERR";

fn argc_var() -> String {
    format!("{SESSION_EXEC_VAR}_ARGC")
}

fn arg_var(n: usize) -> String {
    format!("{SESSION_EXEC_VAR}_ARG_{n}")
}

/// `program args` started as a shell in a terminal window starts it: in a
/// new session, with `slave` as its controlling terminal and on fds 0, 1
/// and 2, so closing the master is the window closing.
///
/// Only the process itself can take a controlling terminal, after `setsid`.
/// Rather than doing that between fork and exec (`pre_exec`, which is
/// `unsafe`), this re-execs the test binary into [`session_exec_entry`]: a
/// fresh process that does it with safe calls, then execs `program` under
/// its own pid, so the session leader is `program` itself.
///
/// stdin is `slave`. stdout is null, so libtest's own lines never reach the
/// pty. stderr is inherited, so a failure before the exec shows in the
/// test's output. The entry points both at the pty just before the exec.
pub(crate) fn session_command(program: &Path, args: &[OsString], slave: OwnedFd) -> Command {
    let test_binary = std::env::current_exe().expect("the test binary");
    // `it::quit_waits::pty_session` → `quit_waits::pty_session`: libtest's
    // names leave out the crate.
    let module = module_path!()
        .split_once("::")
        .map_or(module_path!(), |(_crate, path)| path);
    let mut command = Command::new(test_binary);
    command
        .args([
            "--exact",
            &format!("{module}::session_exec_entry"),
            "--test-threads=1",
            "--nocapture",
        ])
        .env(SESSION_EXEC_VAR, program)
        .env(argc_var(), args.len().to_string())
        .env_remove(KEEP_STDERR_VAR)
        .stdin(Stdio::from(slave))
        .stdout(Stdio::null());
    for (n, arg) in args.iter().enumerate() {
        command.env(arg_var(n), arg);
    }
    command
}

/// The re-exec target of [`session_command`]. In a normal test run
/// [`SESSION_EXEC_VAR`] is unset and it returns at once. When it is set, the
/// entry makes this process a session leader, takes fd 0 (the pty's slave)
/// as its controlling terminal, points fds 1 and 2 at it, and execs the
/// program, so it never returns.
#[test]
fn session_exec_entry() {
    let Some(program) = std::env::var_os(SESSION_EXEC_VAR) else {
        return;
    };
    let count: usize = std::env::var(argc_var())
        .expect("the argument count")
        .parse()
        .expect("a number");
    let args: Vec<OsString> = (0..count)
        .map(|n| std::env::var_os(arg_var(n)).expect("each argument"))
        .collect();
    let keep_stderr = std::env::var_os(KEEP_STDERR_VAR).is_some();

    rustix::process::setsid().expect("a new session");
    let tty = std::io::stdin();
    rustix::process::ioctl_tiocsctty(&tty).expect("the pty becomes the controlling terminal");
    rustix::stdio::dup2_stdout(&tty).expect("stdout on the pty");
    if !keep_stderr {
        rustix::stdio::dup2_stderr(&tty).expect("stderr on the pty");
    }

    let mut command = Command::new(&program);
    command
        .args(&args)
        .env_remove(SESSION_EXEC_VAR)
        .env_remove(argc_var())
        .env_remove(KEEP_STDERR_VAR);
    for n in 0..count {
        command.env_remove(arg_var(n));
    }
    let error = command.exec();
    panic!("could not exec {}: {error}", program.to_string_lossy());
}

type Reader = (Arc<Mutex<Vec<u8>>>, Arc<AtomicBool>, JoinHandle<()>);

/// Everything Graft writes to the pty, gathered on a thread until the
/// flag is cleared or the pty closes.
fn read_into_memory(master: &File) -> Reader {
    let output = Arc::new(Mutex::new(Vec::new()));
    let reading = Arc::new(AtomicBool::new(true));
    let mut source = master.try_clone().expect("master fd");
    let sink = Arc::clone(&output);
    let still = Arc::clone(&reading);
    // Polled, not blocked, so it lets go of its copy of the master when
    // told: `hang_up` must close the last one.
    let reader = std::thread::spawn(move || {
        let mut buffer = [0_u8; 8192];
        while still.load(Ordering::Acquire) {
            let mut fds = [PollFd::new(source.as_fd(), PollFlags::POLLIN)];
            match poll(&mut fds, PollTimeout::from(20_u16)) {
                Ok(0) => {}
                Ok(_) => match source.read(&mut buffer) {
                    Ok(read @ 1..) => sink
                        .lock()
                        .expect("output")
                        .extend_from_slice(&buffer[..read]),
                    // EOF or EIO: the last slave closed; Graft has gone.
                    Ok(0) | Err(_) => return,
                },
                Err(error) => panic!("polling the pty: {error}"),
            }
        }
    });
    (output, reading, reader)
}

impl Session {
    pub(super) fn pid(&self) -> u32 {
        self.child.id()
    }

    pub(super) fn send(&mut self, bytes: &[u8]) {
        let master = self.master.as_mut().expect("the terminal is open");
        master.write_all(bytes).expect("typed");
        master.flush().expect("flushed");
    }

    pub(super) fn wait_for(&self, what: &str, done: impl Fn(&Screen) -> bool) {
        let start = Instant::now();
        loop {
            let screen = Screen::replay(
                &self.output.lock().expect("output"),
                self.size.0,
                self.size.1,
            );
            if done(&screen) {
                return;
            }
            assert!(
                start.elapsed() < PATIENCE,
                "waited {PATIENCE:?} for {what}; the screen:\n{}",
                screen.text()
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Close the terminal: every master fd this side holds.
    pub(super) fn hang_up(&mut self) {
        self.reading.store(false, Ordering::Release);
        if let Some(reader) = self.reader.take() {
            reader.join().expect("the reader ends");
        }
        drop(self.master.take());
    }

    /// `kill -HUP` Graft's own pid: the signal again.
    pub(super) fn hang_up_again(&self) {
        let status = Command::new("kill")
            .args(["-HUP", &self.pid().to_string()])
            .status()
            .expect("kill runs");
        assert!(status.success(), "SIGHUP not delivered");
    }

    /// Wait up to `bound` for Graft to exit; `None` past it.
    pub(super) fn exit_within(&mut self, bound: Duration) -> Option<(ExitStatus, Duration)> {
        let start = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait().expect("waitable") {
                return Some((status, start.elapsed()));
            }
            if start.elapsed() > bound {
                return None;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            if let Err(error) = self.child.kill() {
                eprintln!("could not stop graft: {error}");
            }
            if let Err(error) = self.child.wait() {
                eprintln!("could not reap graft: {error}");
            }
        }
    }
}

/// `pre-merge-commit`: git's pid, its own, then `body`.
pub(super) fn hook(repo: &Path, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    let hook = repo.join(".git/hooks/pre-merge-commit");
    std::fs::create_dir_all(hook.parent().expect("a hooks dir")).expect("writable");
    std::fs::write(
        &hook,
        format!("#!/bin/sh\necho $PPID > .git/git.pid\necho $$ > .git/hook.pid\n{body}\n"),
    )
    .expect("writable");
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).expect("chmod");
}

/// Wait until `file` holds `needle`: the exit has reached that line.
pub(super) fn wait_for_line(file: &Path, needle: &str) -> String {
    let began = Instant::now();
    loop {
        let text = std::fs::read_to_string(file).unwrap_or_default();
        if text.contains(needle) {
            return text;
        }
        assert!(
            began.elapsed() < PATIENCE,
            "never saw {needle:?}; stderr so far:\n{text}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Graft's crash reports in its isolated state directory: none, or the
/// panic is named.
pub(super) fn assert_no_crash(isolated: &Isolated) {
    let dir = isolated.home().join(".local/state/graft");
    let reports: Vec<String> = std::fs::read_dir(&dir)
        .map(|entries| {
            entries
                .map(|entry| {
                    let path = entry.expect("a readable entry").path();
                    std::fs::read_to_string(&path).expect("a readable report")
                })
                .collect()
        })
        .unwrap_or_default();
    assert!(
        reports.is_empty(),
        "graft panicked:\n{}",
        reports.join("\n")
    );
}
