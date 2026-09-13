//! Talking to the real terminal: the one place in detection that does I/O.
//!
//! Everything above this file works against the [`TerminalHost`] trait, so the
//! decisions are tested against scripted terminals and only the three syscalls
//! here are untestable.
//! Reading replies goes through the input mux, which owns stdin for the
//! whole run (`crate::input`). That is the point of the split: this module holds no
//! logic worth testing, and the logic holds no syscalls that stop it being
//! tested.
//!
//! Unix only, deliberately: Windows is an explicit non-goal for v1
//! (PROMPT.md, Mission), and the platform-specific code is confined here so
//! that staying possible costs nothing today.

use std::io::{self, Write};
use std::os::fd::AsRawFd;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::{TerminalHost, WindowPixels};
use crate::input::ReplyWaiter;

/// How long `tmux -V` gets before Graft stops waiting for it.
///
/// The same argument that bounds `git --version` in `graft-git`: this runs at
/// startup, before there is a window to show anything in, so a `tmux` that
/// wedges — a stale network mount on `PATH`, a shadowing wrapper script —
/// must cost Graft a known version, never a window that never appears. (The
/// shape is deliberately duplicated rather than shared: `term` and `git` are
/// siblings and neither may depend on the other, ARCHITECTURE.md.)
///
/// It is the local probe budget, not a number of its own, because it is added
/// to the probe budget: everything between launch and the first frame is
/// bounded by DESIGN.md §4.4's total, and a deadline in seconds would dwarf
/// the rounds it sits beside. `tmux` is a local binary printing one short
/// line, so 200 ms is already twenty times what a healthy one needs.
const TMUX_VERSION_DEADLINE: Duration = super::LOCAL_PROBE_BUDGET;

/// How often the wait above checks whether tmux has finished.
const TMUX_VERSION_POLL: Duration = Duration::from_millis(2);

/// The process's own terminal: queries out on stdout, replies in through the
/// input mux ([`crate::input::InputMux`]), which owns stdin.
///
/// Requires raw mode — in cooked mode the terminal's replies sit in the line
/// buffer until the user presses Return, and the deadline expires first.
#[derive(Debug)]
pub struct TtyHost {
    replies: Option<ReplyWaiter>,
    output: io::Stdout,
}

impl TtyHost {
    /// Write to this process's stdout and read replies from `replies`.
    pub fn new(replies: ReplyWaiter) -> Self {
        Self {
            replies: Some(replies),
            output: io::stdout(),
        }
    }

    /// A host that is never read: for a run where detection cannot probe
    /// (stdout is not a terminal), so no input mux is started to steal the
    /// user's typing from whatever else owns stdin. Every read reports that
    /// nothing arrived.
    pub fn unread() -> Self {
        Self {
            replies: None,
            output: io::stdout(),
        }
    }
}

impl TerminalHost for TtyHost {
    fn write_probe(&mut self, bytes: &[u8]) -> io::Result<()> {
        let mut output = self.output.lock();
        output.write_all(bytes)?;
        output.flush()
    }

    fn read(&mut self, buffer: &mut [u8], deadline: Instant) -> io::Result<usize> {
        match &mut self.replies {
            Some(replies) => replies.read(buffer, deadline),
            None => Ok(0),
        }
    }

    #[allow(
        unsafe_code,
        reason = "TIOCGWINSZ has no safe wrapper in the standard library; \
                  ADR-0013 covers this call and its soundness argument"
    )]
    fn window_pixels(&mut self) -> Option<WindowPixels> {
        // Built field by field rather than zeroed through `mem::zeroed`:
        // every field is a `u16` with a valid all-zeroes value, so the
        // initialisation needs no `unsafe` at all (ADR-0013).
        let mut size = libc::winsize {
            ws_row: 0,
            ws_col: 0,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        // SAFETY: `TIOCGWINSZ` writes one `winsize` through the pointer, which
        // points at a live, fully initialised local of exactly that type, and
        // the fd is this process's own stdout. ADR-0013.
        let queried =
            unsafe { libc::ioctl(self.output.as_raw_fd(), libc::TIOCGWINSZ, &raw mut size) };
        if queried != 0 {
            // Not a terminal, or a terminal that will not say: the caller's
            // next fallback answers, so there is nothing to report here.
            return None;
        }
        Some(WindowPixels {
            columns: size.ws_col,
            rows: size.ws_row,
            width: size.ws_xpixel,
            height: size.ws_ypixel,
        })
    }

    /// `None` covers every way this can come to nothing — tmux is not on the
    /// `PATH`, it failed, it took longer than [`TMUX_VERSION_DEADLINE`], or it
    /// printed bytes that are not text. The caller turns all of them into
    /// `TmuxStatus::VersionUnknown`, which is shown on the diagnostics screen
    /// and treated as too old, so nothing is lost by not distinguishing them
    /// here.
    fn tmux_version(&mut self) -> Option<String> {
        let mut child = Command::new("tmux")
            .arg("-V")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .ok()?;

        let started = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                // It cannot be waited for, so its output will never arrive.
                // Kill it anyway rather than leave a child nobody reaps; the
                // same best-effort argument as the deadline arm below.
                Err(_) => {
                    let _ = child.kill().and_then(|()| child.wait().map(drop));
                    return None;
                }
                Ok(None) => {}
            }
            if started.elapsed() >= TMUX_VERSION_DEADLINE {
                // Best effort, and deliberately unreported: the answer is
                // already "no version", and a process that cannot be killed
                // is beyond this program's reach with nothing useful to say
                // about it.
                let _ = child.kill().and_then(|()| child.wait().map(drop));
                return None;
            }
            // One short line of output, far less than a pipe buffer, so
            // neither pipe can fill while this waits.
            std::thread::sleep(TMUX_VERSION_POLL);
        }

        let output = child.wait_with_output().ok()?;
        if !output.status.success() {
            return None;
        }
        String::from_utf8(output.stdout).ok()
    }
}
