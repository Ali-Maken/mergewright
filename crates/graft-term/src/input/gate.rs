//! Getting the reader thread off the terminal's input, and putting it back
//! (m3-working-tree-undo-and-stash/05).
//!
//! [`super::InputMux::hold`] is not enough for this. It stops events reaching
//! an [`super::Inputs`], but the reader thread goes on polling and **reading**
//! the terminal, so a program Graft hands the terminal to — `$EDITOR`, or the
//! shell after `Ctrl+Z` — would be racing Graft for every keystroke, and
//! losing about half of them.
//!
//! What is needed is for the reader to stop taking bytes out of the kernel's
//! buffer at all, so they are still there when the editor asks. Two pieces:
//!
//! 1. A **self-pipe**. The reader is asleep in `poll` with no deadline, and
//!    nothing else would wake it. `poll` watches the pipe's read end too, so
//!    a single byte written to the other end brings the reader back.
//! 2. A **flag checked before every read**, not after. Waking the reader is
//!    not the point — the point is that it looks at the flag and parks
//!    without touching the terminal, leaving whatever has been typed where
//!    the editor will find it.
//!
//! [`Gate::pause`] waits for the reader to say it has parked, so a caller
//! that gets a `Ok(())` back knows the terminal is nobody's. It waits with a
//! deadline: a reader thread that has already exited (the terminal closed)
//! will never answer, and hanging the editor on that would be worse than
//! going ahead.

use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

/// How long [`Gate::pause`] waits for the reader to park before going ahead
/// without it. Long enough for a thread that is running to answer many times
/// over; short enough that a thread that is gone does not hold up an editor.
const PARK_TIMEOUT: Duration = Duration::from_millis(250);

/// Shared between the reader thread and whoever wants the terminal.
#[derive(Debug)]
pub(super) struct Gate {
    state: Mutex<GateState>,
    changed: Condvar,
    /// The self-pipe's write end. A byte here wakes the reader out of `poll`.
    waker: OwnedFd,
}

#[derive(Debug, Default)]
struct GateState {
    /// The reader must not touch the terminal.
    paused: bool,
    /// The reader has seen `paused` and is waiting on the condvar.
    parked: bool,
}

/// What woke the reader up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Ready {
    /// The terminal has bytes to read.
    Input,
    /// The self-pipe fired: the gate opened or closed.
    Wake,
    /// Nothing happened before the deadline.
    Timeout,
}

impl Gate {
    /// A gate and the read end the reader thread polls.
    ///
    /// # Errors
    /// The pipe could not be made, or could not be put in non-blocking mode.
    pub(super) fn new() -> io::Result<(Self, OwnedFd)> {
        let (read, write) = pipe()?;
        let gate = Self {
            state: Mutex::new(GateState::default()),
            changed: Condvar::new(),
            waker: write,
        };
        Ok((gate, read))
    }

    /// Stop the reader touching the terminal, and wait until it has stopped.
    ///
    /// # Errors
    /// The reader could not be woken. The caller must not hand the terminal
    /// over: the reader is still on it.
    pub(super) fn pause(&self) -> io::Result<()> {
        self.lock().paused = true;
        self.wake()?;
        let mut state = self.lock();
        while !state.parked {
            let (next, timed_out) = self
                .changed
                .wait_timeout(state, PARK_TIMEOUT)
                .unwrap_or_else(PoisonError::into_inner);
            state = next;
            if timed_out.timed_out() {
                // The reader is not answering — it has most likely already
                // ended, because the terminal closed its input. Not an error
                // to report: there is then nothing left to race with.
                break;
            }
        }
        Ok(())
    }

    /// Let the reader back onto the terminal. Never fails: if the pipe write
    /// fails the reader is woken by the next keystroke instead, and the flag
    /// it re-reads is already false.
    pub(super) fn resume(&self) {
        self.lock().paused = false;
        self.changed.notify_all();
        if let Err(error) = self.wake() {
            // Deliberately not fatal, and not silent: the reader wakes on the
            // next byte anyway, so the only cost is that a key typed *before*
            // the first one after resume stays in the buffer a moment longer.
            eprintln!("graft: could not wake the input reader: {error}");
        }
    }

    /// The reader's side: park here for as long as the gate is shut.
    pub(super) fn wait_while_paused(&self) {
        let mut state = self.lock();
        if !state.paused {
            return;
        }
        state.parked = true;
        self.changed.notify_all();
        while state.paused {
            state = self
                .changed
                .wait(state)
                .unwrap_or_else(PoisonError::into_inner);
        }
        state.parked = false;
    }

    /// The reader's side: is the gate shut right now? Checked after `poll`
    /// says the terminal is readable and **before** the read, so a paused
    /// reader leaves those bytes for whoever has the terminal now.
    pub(super) fn is_paused(&self) -> bool {
        self.lock().paused
    }

    /// One byte down the pipe.
    #[allow(
        unsafe_code,
        reason = "writing to a raw fd has no safe standard-library wrapper \
                  that does not take ownership of the descriptor; ADR-0013 \
                  covers the raw-fd calls in this crate"
    )]
    fn wake(&self) -> io::Result<()> {
        let byte = b"\0";
        // SAFETY: `self.waker` is an `OwnedFd` alive for this call, the
        // pointer is to one live byte and the count says one. The pipe is
        // non-blocking, so a full pipe returns `EAGAIN` rather than hanging —
        // and a full pipe means a wake is already pending, which is the same
        // outcome.
        let written = unsafe { libc::write(self.waker.as_raw_fd(), byte.as_ptr().cast(), 1) };
        if written < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::WouldBlock {
                return Ok(());
            }
            return Err(error);
        }
        Ok(())
    }

    /// The state, adopting a poisoned lock: `GateState` is two booleans that
    /// are only ever set, never half-written, so a panic while it was held
    /// cannot have left it inconsistent.
    fn lock(&self) -> MutexGuard<'_, GateState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// A non-blocking, close-on-exec pipe: `(read, write)`.
///
/// `pipe(2)` then `fcntl(2)` rather than `pipe2(2)`, which macOS does not
/// have. Non-blocking on both ends so neither a wake nor a drain can stall
/// the thread doing it; close-on-exec so the editor Graft spawns does not
/// inherit a pipe it knows nothing about.
#[allow(
    unsafe_code,
    reason = "pipe(2) and fcntl(2) have no safe wrapper in the standard \
              library; ADR-0013 covers the raw-fd calls in this crate"
)]
fn pipe() -> io::Result<(OwnedFd, OwnedFd)> {
    let mut ends = [0 as RawFd; 2];
    // SAFETY: the pointer is to a live array of two `int`s, which is exactly
    // what `pipe` writes.
    if unsafe { libc::pipe(ends.as_mut_ptr()) } < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `pipe` succeeded, so both are fresh descriptors this thread is
    // the only owner of, and `OwnedFd` takes over closing them — including on
    // the early return below, because they are wrapped before anything else
    // can fail.
    let (read, write) = unsafe { (OwnedFd::from_raw_fd(ends[0]), OwnedFd::from_raw_fd(ends[1])) };
    for end in [&read, &write] {
        set_flags(end.as_raw_fd())?;
    }
    Ok((read, write))
}

/// `O_NONBLOCK` and `FD_CLOEXEC` on one end of the pipe.
#[allow(
    unsafe_code,
    reason = "fcntl(2) has no safe wrapper in the standard library; ADR-0013"
)]
fn set_flags(fd: RawFd) -> io::Result<()> {
    // SAFETY: `fd` is a live descriptor owned by the caller for the duration
    // of this call, and both `fcntl` commands only read or replace its flags.
    let ok = unsafe {
        libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK) >= 0
            && libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) >= 0
    };
    if ok {
        return Ok(());
    }
    Err(io::Error::last_os_error())
}

/// Throw away everything waiting on the wake pipe. The bytes carry nothing —
/// only the fact that somebody wrote one — so the count is not interesting
/// and neither is an `EAGAIN`, which just means another thread got there
/// first.
#[allow(
    unsafe_code,
    reason = "read(2) on a raw fd has no safe standard-library wrapper that \
              does not take ownership of the descriptor; ADR-0013"
)]
pub(super) fn drain(fd: RawFd) {
    let mut sink = [0u8; 64];
    loop {
        // SAFETY: `fd` is the pipe's read end, alive for this call, and the
        // pointer and length describe a live buffer of that size. The pipe is
        // non-blocking, so this returns rather than waiting.
        let count = unsafe { libc::read(fd, sink.as_mut_ptr().cast(), sink.len()) };
        // A short read means the pipe is empty now; `<= 0` is `EAGAIN` (empty
        // already) or a closed pipe, and neither is worth reporting — the
        // pipe carries no data, only the fact that somebody wrote to it.
        let Ok(count) = usize::try_from(count) else {
            return;
        };
        if count < sink.len() {
            return;
        }
    }
}
