//! The reader thread, and the two ways out of it: events and replies.

use std::fs::File;
use std::io::{self, Read};
use std::os::fd::{AsFd, AsRawFd, OwnedFd, RawFd};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use graft_core::event::Event;

use super::decode::{Decoded, Decoder};

/// How long an unfinished escape sequence waits for its remaining bytes
/// before what has arrived is read as keys.
///
/// This is what separates a lone `Esc` press from the first byte of an arrow
/// key or a reply: a terminal writes a sequence in one go, so its bytes land
/// within a millisecond or two of each other, while a person cannot press a
/// second key 50 ms after `Esc` and mean it as part of the same one. Long
/// enough for a reply split across two reads to be reassembled; short enough
/// that `Esc` still feels instant.
pub const SEQUENCE_TIMEOUT: Duration = Duration::from_millis(50);

/// Events decoded from the terminal, in the order their bytes arrived. An
/// `Err` is the reader's last word: the input failed or closed, and nothing
/// follows it.
pub type Inputs = Receiver<io::Result<Event>>;

/// The single owner of the terminal's input.
///
/// Dropping it does not stop the reader: that thread is blocked in `poll` on
/// the terminal and holds nothing that matters, so it ends with the process.
#[derive(Debug, Clone)]
pub struct InputMux {
    waiters: Arc<Mutex<Vec<Sender<Vec<u8>>>>>,
}

impl InputMux {
    /// Start reading this process's stdin. Call once, before anything writes
    /// a query to the terminal, and with raw mode already on.
    pub fn on_stdin() -> io::Result<(Self, Inputs)> {
        // `Stdin` reads through a buffer of its own, which would hold bytes
        // `poll` cannot see. A duplicate of the raw descriptor reads exactly
        // what the kernel has, and nothing more.
        Self::start(io::stdin().as_fd().try_clone_to_owned()?)
    }

    /// Start reading `input` on a thread of its own.
    pub fn start(input: OwnedFd) -> io::Result<(Self, Inputs)> {
        let mux = Self {
            waiters: Arc::new(Mutex::new(Vec::new())),
        };
        let (events, inputs) = mpsc::channel();
        let waiters = Arc::clone(&mux.waiters);
        thread::Builder::new()
            .name("graft-input".to_owned())
            .spawn(move || read_until_done(File::from(input), &waiters, &events))?;
        Ok((mux, inputs))
    }

    /// Start receiving terminal replies.
    ///
    /// Every waiter open when a reply arrives gets a copy; each one picks
    /// out the reply it asked for and ignores the rest. A reply that arrives
    /// while no waiter is open answers a question nobody is asking any more,
    /// so it is dropped — it is never a key, and never reaches the keymap.
    pub fn waiter(&self) -> ReplyWaiter {
        let (sender, receiver) = mpsc::channel();
        self.waiters
            .lock()
            // The list is only ever pushed to or filtered, so a panic while
            // it was held cannot have left it half-changed.
            .unwrap_or_else(PoisonError::into_inner)
            .push(sender);
        ReplyWaiter {
            receiver,
            pending: Vec::new(),
        }
    }
}

/// Terminal replies for one asker, as the bytes the terminal sent.
#[derive(Debug)]
pub struct ReplyWaiter {
    receiver: Receiver<Vec<u8>>,
    pending: Vec<u8>,
}

impl ReplyWaiter {
    /// Copy reply bytes into `buffer`, waiting no longer than `deadline`.
    ///
    /// `Ok(0)` means the deadline passed, or the reader has stopped — in
    /// which case its failure has already gone out on [`Inputs`], where the
    /// event loop reports it. Either way no more replies are coming.
    pub fn read(&mut self, buffer: &mut [u8], deadline: Instant) -> io::Result<usize> {
        if self.pending.is_empty() {
            let remaining = deadline.saturating_duration_since(Instant::now());
            match self.receiver.recv_timeout(remaining) {
                Ok(reply) => self.pending = reply,
                Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => return Ok(0),
            }
        }
        let count = buffer.len().min(self.pending.len());
        buffer[..count].copy_from_slice(&self.pending[..count]);
        self.pending.drain(..count);
        Ok(count)
    }
}

fn read_until_done(
    mut input: File,
    waiters: &Mutex<Vec<Sender<Vec<u8>>>>,
    events: &Sender<io::Result<Event>>,
) {
    let fd = input.as_raw_fd();
    let mut decoder = Decoder::new();
    let mut chunk = [0u8; 4096];
    let mut finished = Vec::new();
    let mut last_byte = Instant::now();
    loop {
        let wait = decoder
            .is_holding()
            .then(|| SEQUENCE_TIMEOUT.saturating_sub(last_byte.elapsed()));
        let failure = match wait_for_input(fd, wait) {
            // A signal (SIGWINCH, most often) cut the wait short. The wait is
            // recomputed from the clock, so retrying loses no time.
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => Some(error),
            Ok(false) => {
                decoder.flush(&mut finished);
                None
            }
            Ok(true) => match input.read(&mut chunk) {
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => Some(error),
                Ok(0) => {
                    decoder.flush(&mut finished);
                    Some(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "the terminal closed its input",
                    ))
                }
                Ok(count) => {
                    last_byte = Instant::now();
                    decoder.feed(&chunk[..count], &mut finished);
                    None
                }
            },
        };
        if !route(&mut finished, waiters, events) {
            // Nobody reads events any more: the loop has finished.
            return;
        }
        if let Some(error) = failure {
            // Sent, not retried: there is no input any more. A failed send
            // means the loop is gone too, and there is nobody left to tell.
            if events.send(Err(error)).is_err() {
                return;
            }
            break;
        }
    }
}

/// Send each decoded item where it goes. `false` once the event receiver
/// has gone.
fn route(
    decoded: &mut Vec<Decoded>,
    waiters: &Mutex<Vec<Sender<Vec<u8>>>>,
    events: &Sender<io::Result<Event>>,
) -> bool {
    for item in decoded.drain(..) {
        match item {
            Decoded::Event(event) => {
                if events.send(Ok(event)).is_err() {
                    return false;
                }
            }
            Decoded::Reply(bytes) => waiters
                .lock()
                // Only pushed to or filtered; see `InputMux::waiter`.
                .unwrap_or_else(PoisonError::into_inner)
                // A waiter whose receiver is gone has finished asking.
                .retain(|waiter| waiter.send(bytes.clone()).is_ok()),
        }
    }
    true
}

/// Wait for `fd` to have something to read, up to `timeout` (`None`: for as
/// long as it takes). `false` means the timeout won.
#[allow(
    unsafe_code,
    reason = "poll(2) has no safe wrapper in the standard library; ADR-0013 \
              covers this call and its soundness argument"
)]
fn wait_for_input(fd: RawFd, timeout: Option<Duration>) -> io::Result<bool> {
    // `poll` takes `int` milliseconds, -1 for no limit. Rounded up, so a wait
    // of a fraction of a millisecond does not spin; clamped, because nothing
    // here waits 24 days.
    let milliseconds = timeout.map_or(-1, |wait| {
        i32::try_from(wait.as_micros().div_ceil(1000)).unwrap_or(i32::MAX)
    });
    let mut watched = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: the pointer is to one live, fully initialised `pollfd`, the
    // count says one, and `poll` writes only `revents` back. ADR-0013.
    let ready = unsafe { libc::poll(&raw mut watched, 1, milliseconds) };
    match ready {
        0 => Ok(false),
        count if count > 0 => Ok(true),
        _ => Err(io::Error::last_os_error()),
    }
}

#[cfg(test)]
mod tests;
