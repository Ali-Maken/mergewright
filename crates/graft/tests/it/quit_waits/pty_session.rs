//! The pty harness the hang-up tests drive (m8/13g, split out in m8/13l):
//! the real binary as a pty **session leader**, the pty its controlling
//! terminal, so closing the master is a real hang-up — the kernel sends
//! `SIGHUP` and every later read of the terminal fails.
//!
//! Every pid a test learns (Graft's, git's, the hook's) is held by a
//! [`Leftovers`] guard that kills it on drop, by pid, never by pattern.

use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{AsFd, AsRawFd, OwnedFd};
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
        let slave: OwnedFd = pty.slave;
        let master_fd = pty.master.as_raw_fd();
        let mut command = Command::new(env!("CARGO_BIN_EXE_graft"));
        command
            .arg(repo)
            .env("XDG_CONFIG_HOME", isolated.config())
            .env("HOME", isolated.home())
            .env("GIT_CEILING_DIRECTORIES", repo)
            .env("TERM", "xterm-256color")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("NO_COLOR")
            .env_remove("GRAFT_TRACE")
            .stdin(Stdio::from(slave.try_clone().expect("slave fd")))
            .stdout(Stdio::from(slave.try_clone().expect("slave fd")));
        match stderr {
            Stderr::Pty => command.stderr(Stdio::from(slave)),
            Stderr::File(path) => {
                command.stderr(Stdio::from(File::create(path).expect("a stderr file")))
            }
        };
        session_leader(&mut command, master_fd);
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

/// Start `command` in a new session with its stdin, the pty's slave, as its
/// controlling terminal — as a shell in a terminal window runs it, so
/// closing `master_fd` is the window closing.
fn session_leader(command: &mut Command, master_fd: std::os::fd::RawFd) {
    #[allow(
        unsafe_code,
        reason = "a controlling terminal is only set from pre_exec: close, setsid and \
                  TIOCSCTTY, each async-signal-safe, touching only the child"
    )]
    // SAFETY: the closure makes three async-signal-safe syscalls on the
    // child's own descriptors and allocates nothing.
    unsafe {
        command.pre_exec(move || {
            // The master is not close-on-exec: left open in Graft, it would
            // keep the terminal alive after the test closes it.
            if nix::libc::close(master_fd) < 0
                || nix::libc::setsid() < 0
                || nix::libc::ioctl(0, nix::libc::TIOCSCTTY.into(), 0) < 0
            {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
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
