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
    outlet: Arc<Mutex<Outlet>>,
}

/// Where decoded events go: the current [`Inputs`], or a queue for the next
/// one while there is none.
///
/// A key is never dropped because its receiver went away — a wrapper that
/// runs the loop again (the gallery) gets it on the `Inputs` it opens next.
#[derive(Debug, Default)]
struct Outlet {
    sender: Option<Sender<io::Result<Event>>>,
    held: Vec<io::Result<Event>>,
}

impl Outlet {
    fn deliver(&mut self, item: io::Result<Event>) {
        let Some(sender) = self.sender.as_ref() else {
            self.held.push(item);
            return;
        };
        // Nobody reads that `Inputs` any more: keep the event for the next
        // one. `send` hands a refused item back, so nothing is lost.
        if let Err(mpsc::SendError(item)) = sender.send(item) {
            self.sender = None;
            self.held.push(item);
        }
    }

    fn lock(outlet: &Mutex<Self>) -> std::sync::MutexGuard<'_, Self> {
        // Every change under the lock is one field assignment or one push,
        // so a panic while it was held cannot have left it half-changed.
        outlet.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The one reader of this process's stdin (ticket m1-the-graph/35c). Stdin
/// has one owner; the second `on_stdin` hands out the same one.
static STDIN: Mutex<Option<InputMux>> = Mutex::new(None);

impl InputMux {
    /// Start reading this process's stdin, or, from the second call on, take
    /// over the reader the first call started. Call before anything writes a
    /// query to the terminal, and with raw mode already on.
    ///
    /// Returns a fresh [`Inputs`] each time (see [`InputMux::inputs`]): only
    /// one run at a time reads the terminal, so the previous run's `Inputs`
    /// ends and every key from here on goes to the new one.
    pub fn on_stdin() -> io::Result<(Self, Inputs)> {
        let mut stdin = STDIN.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(mux) = stdin.as_ref() {
            return Ok((mux.clone(), mux.inputs()));
        }
        // `Stdin` reads through a buffer of its own, which would hold bytes
        // `poll` cannot see. A duplicate of the raw descriptor reads exactly
        // what the kernel has, and nothing more.
        let (mux, inputs) = Self::start(io::stdin().as_fd().try_clone_to_owned()?)?;
        *stdin = Some(mux.clone());
        Ok((mux, inputs))
    }

    /// Start reading `input` on a thread of its own.
    pub fn start(input: OwnedFd) -> io::Result<(Self, Inputs)> {
        let (sender, inputs) = mpsc::channel();
        let mux = Self {
            waiters: Arc::new(Mutex::new(Vec::new())),
            outlet: Arc::new(Mutex::new(Outlet {
                sender: Some(sender),
                held: Vec::new(),
            })),
        };
        let waiters = Arc::clone(&mux.waiters);
        let outlet = Arc::clone(&mux.outlet);
        thread::Builder::new()
            .name("graft-input".to_owned())
            .spawn(move || read_until_done(File::from(input), &waiters, &outlet))?;
        Ok((mux, inputs))
    }

    /// Open a fresh [`Inputs`] and make it the one that gets every event from
    /// now on. The previous `Inputs` gets nothing more and ends (its sender
    /// is dropped here), and anything that arrived while no `Inputs` was
    /// listening — after [`InputMux::hold`], or after the previous receiver
    /// was dropped — comes first, in order.
    pub fn inputs(&self) -> Inputs {
        let (sender, inputs) = mpsc::channel();
        let mut outlet = Outlet::lock(&self.outlet);
        outlet.sender = Some(sender);
        // Through `deliver`, so the held events take the same path as live
        // ones (the receiver is in this function's hands, so none is refused).
        for item in std::mem::take(&mut outlet.held) {
            outlet.deliver(item);
        }
        inputs
    }

    /// End the current [`Inputs`] and keep every event from now on for the
    /// next [`InputMux::inputs`]. A run calls this the moment its loop is
    /// done, so a key typed while it tears down reaches the run that
    /// follows instead of a forwarder nobody listens to.
    pub fn hold(&self) {
        Outlet::lock(&self.outlet).sender = None;
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

fn read_until_done(mut input: File, waiters: &Mutex<Vec<Sender<Vec<u8>>>>, outlet: &Mutex<Outlet>) {
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
        route(&mut finished, waiters, outlet);
        if let Some(error) = failure {
            // Delivered, not retried: there is no input any more, and this
            // is the reader's last word.
            Outlet::lock(outlet).deliver(Err(error));
            break;
        }
    }
}

/// Send each decoded item where it goes.
fn route(
    decoded: &mut Vec<Decoded>,
    waiters: &Mutex<Vec<Sender<Vec<u8>>>>,
    outlet: &Mutex<Outlet>,
) {
    for item in decoded.drain(..) {
        match item {
            Decoded::Event(event) => Outlet::lock(outlet).deliver(Ok(event)),
            Decoded::Reply(bytes) => waiters
                .lock()
                // Only pushed to or filtered; see `InputMux::waiter`.
                .unwrap_or_else(PoisonError::into_inner)
                // A waiter whose receiver is gone has finished asking.
                .retain(|waiter| waiter.send(bytes.clone()).is_ok()),
        }
    }
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
