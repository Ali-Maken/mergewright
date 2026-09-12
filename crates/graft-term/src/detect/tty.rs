//! Talking to the real terminal: the one place in detection that does I/O.
//!
//! Everything above this file works against the [`TerminalHost`] trait, so the
//! decisions are tested against scripted terminals and only the three syscalls
//! here are untestable. That is the point of the split: this module holds no
//! logic worth testing, and the logic holds no syscalls that stop it being
//! tested.
//!
//! Unix only, deliberately: Windows is an explicit non-goal for v1
//! (PROMPT.md, Mission), and the platform-specific code is confined here so
//! that staying possible costs nothing today.

use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, RawFd};
use std::process::Command;
use std::time::Instant;

use super::{TerminalHost, WindowPixels};

/// The process's own terminal: queries out on stdout, replies in on stdin.
///
/// Requires raw mode — in cooked mode the terminal's replies sit in the line
/// buffer until the user presses Return, and the deadline expires first.
#[derive(Debug)]
pub struct TtyHost {
    input: io::Stdin,
    output: io::Stdout,
}

impl TtyHost {
    /// Wrap this process's stdin and stdout.
    pub fn new() -> Self {
        Self {
            input: io::stdin(),
            output: io::stdout(),
        }
    }
}

impl Default for TtyHost {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalHost for TtyHost {
    fn write_probe(&mut self, bytes: &[u8]) -> io::Result<()> {
        let mut output = self.output.lock();
        output.write_all(bytes)?;
        output.flush()
    }

    fn read(&mut self, buffer: &mut [u8], deadline: Instant) -> io::Result<usize> {
        let fd = self.input.as_raw_fd();
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Ok(0);
            }
            let milliseconds = remaining.as_millis().min(i32::MAX as u128) as i32;
            match wait_for_input(fd, milliseconds) {
                // A signal cut the wait short. Retrying against a *recomputed*
                // deadline is not a workaround for a failure: the caller asked
                // to wait until a point in time, and that point has not come.
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
                // The deadline arrived before the terminal did.
                Ok(false) => return Ok(0),
                Ok(true) => match self.input.lock().read(buffer) {
                    // A signal interrupted the read; the deadline is still the
                    // deadline, so recompute it and carry on rather than
                    // reporting a failure that did not happen.
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    result => return result,
                },
            }
        }
    }

    fn window_pixels(&mut self) -> Option<WindowPixels> {
        // SAFETY: `TIOCGWINSZ` writes one `winsize` through the pointer, which
        // points at a live local of exactly that type.
        let mut size: libc::winsize = unsafe { std::mem::zeroed() };
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

    fn tmux_version(&mut self) -> Option<String> {
        let output = Command::new("tmux").arg("-V").output().ok()?;
        if !output.status.success() {
            return None;
        }
        String::from_utf8(output.stdout).ok()
    }
}

/// Wait up to `timeout` milliseconds for `fd` to have something to read.
/// `false` means the timeout won.
fn wait_for_input(fd: RawFd, timeout: i32) -> io::Result<bool> {
    let mut watched = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: the pointer is to one live `pollfd`, and the count says one.
    let ready = unsafe { libc::poll(&raw mut watched, 1, timeout) };
    match ready {
        0 => Ok(false),
        count if count > 0 => Ok(true),
        _ => Err(io::Error::last_os_error()),
    }
}
