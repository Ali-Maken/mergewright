//! The reader thread, and the two ways out of it: events and replies.
//! Fence answers are a third, split out to [`fence`].

mod fence;

use std::fs::File;
use std::io::{self, Read};
use std::os::fd::{AsFd, AsRawFd, OwnedFd, RawFd};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use graft_core::event::Event;

use super::decode::{Decoded, Decoder};
use super::gate::{Gate, Ready, drain};
use fence::{FenceSlot, to_fence_listener};

pub use fence::{FenceAnswers, FenceListener};

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
    fence: Arc<Mutex<FenceSlot>>,
    outlet: Arc<Mutex<Outlet>>,
    gate: Arc<Gate>,
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
        let (gate, wake) = Gate::new()?;
        let mux = Self {
            waiters: Arc::new(Mutex::new(Vec::new())),
            fence: Arc::new(Mutex::new(FenceSlot::default())),
            outlet: Arc::new(Mutex::new(Outlet {
                sender: Some(sender),
                held: Vec::new(),
            })),
            gate: Arc::new(gate),
        };
        let routes = Routes {
            waiters: Arc::clone(&mux.waiters),
            fence: Arc::clone(&mux.fence),
            outlet: Arc::clone(&mux.outlet),
        };
        let gate = Arc::clone(&mux.gate);
        thread::Builder::new()
            .name("graft-input".to_owned())
            .spawn(move || read_until_done(File::from(input), &routes, &gate, &wake))?;
        Ok((mux, inputs))
    }

    /// Take the reader off the terminal's input entirely, and wait until it
    /// is off (m3-working-tree-undo-and-stash/05).
    ///
    /// Stronger than [`InputMux::hold`], and for a different reason: `hold`
    /// stops events reaching an [`Inputs`] while the reader keeps draining
    /// the terminal, which is right between two runs of the loop and wrong
    /// when the terminal is being handed to another program. After this, the
    /// bytes a user types are still in the kernel's buffer when `$EDITOR`
    /// asks for them.
    ///
    /// Undone by [`InputMux::resume`], which the caller owes it — a mux left
    /// paused delivers no keys ever again.
    ///
    /// # Errors
    /// The reader could not be woken to be told. The caller must keep the
    /// terminal: the reader is still reading it.
    pub fn pause(&self) -> io::Result<()> {
        self.gate.pause()
    }

    /// Put the reader back on the terminal after [`InputMux::pause`].
    pub fn resume(&self) {
        self.gate.resume();
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

    /// Put back events a run took off its [`Inputs`] but never handled —
    /// the keys behind the one that ended its loop. They go ahead of
    /// anything held since (they were typed first), so the next
    /// [`InputMux::inputs`] sees the terminal's order. Call after
    /// [`InputMux::hold`], once the run has stopped taking from its `Inputs`.
    pub fn give_back(&self, events: impl IntoIterator<Item = Event>) {
        let mut outlet = Outlet::lock(&self.outlet);
        let since: Vec<_> = std::mem::take(&mut outlet.held);
        outlet.held.extend(events.into_iter().map(Ok));
        outlet.held.extend(since);
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

    /// Send every fence answer (`CSI 0 n`, ADR-0054) to `listener` instead
    /// of to the waiters, until the returned guard is dropped or the
    /// listener says `false`. Called from the reader thread, so it must not
    /// block: a channel send is the intended body.
    ///
    /// Before a listener is installed, a fence answer is a reply like any
    /// other — the startup probe reads its `CSI 5 n` answer through a
    /// waiter. A later call replaces the listener (the run that paces now).
    pub fn answer_fences(&self, listener: FenceListener) -> FenceAnswers {
        FenceSlot::install(&self.fence, listener)
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

/// Where the reader thread sends what it decoded.
struct Routes {
    waiters: Arc<Mutex<Vec<Sender<Vec<u8>>>>>,
    fence: Arc<Mutex<FenceSlot>>,
    outlet: Arc<Mutex<Outlet>>,
}

fn read_until_done(mut input: File, routes: &Routes, gate: &Gate, wake: &OwnedFd) {
    let fd = input.as_raw_fd();
    let wake_fd = wake.as_raw_fd();
    let mut decoder = Decoder::new();
    let mut chunk = [0u8; 4096];
    let mut finished = Vec::new();
    let mut last_byte = Instant::now();
    loop {
        // Before the wait, not after: a reader that parks here is not in
        // `poll` on the terminal either, so nothing it does can disturb the
        // program the terminal has been handed to.
        gate.wait_while_paused();
        let wait = decoder
            .is_holding()
            .then(|| SEQUENCE_TIMEOUT.saturating_sub(last_byte.elapsed()));
        let failure = match wait_for_input(fd, wake_fd, wait) {
            // A signal (SIGWINCH, most often) cut the wait short. The wait is
            // recomputed from the clock, so retrying loses no time.
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => Some(error),
            // The gate opened or shut. Round the loop: the check at the top
            // decides which, and nothing was read in the meantime.
            Ok(Ready::Wake) => continue,
            Ok(Ready::Timeout) => {
                decoder.flush(&mut finished);
                None
            }
            // Paused between the poll and here: the bytes `poll` saw belong
            // to whoever has the terminal now, so they stay in the buffer.
            Ok(Ready::Input) if gate.is_paused() => continue,
            Ok(Ready::Input) => match input.read(&mut chunk) {
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
        route(&mut finished, routes);
        if let Some(error) = failure {
            // Delivered, not retried: there is no input any more, and this
            // is the reader's last word.
            Outlet::lock(&routes.outlet).deliver(Err(error));
            break;
        }
    }
    // The reader's last act. `Gate::pause` waits for the reader to park; a
    // reader that has ended will never park, and without this the wait could
    // only time out — which it used to answer `Ok` to, not knowing the
    // difference between a reader that had gone and one that was slow
    // (m3/05 review, Major 6).
    gate.reader_finished();
}

/// Send each decoded item where it goes: events to the outlet, fence
/// answers to the fence listener when there is one, other replies to every
/// waiter.
fn route(decoded: &mut Vec<Decoded>, routes: &Routes) {
    for item in decoded.drain(..) {
        match item {
            Decoded::Event(event) => Outlet::lock(&routes.outlet).deliver(Ok(event)),
            Decoded::Reply(bytes) if to_fence_listener(&bytes, &routes.fence) => {}
            Decoded::Reply(bytes) => routes
                .waiters
                .lock()
                // Only pushed to or filtered; see `InputMux::waiter`.
                .unwrap_or_else(PoisonError::into_inner)
                // A waiter whose receiver is gone has finished asking.
                .retain(|waiter| waiter.send(bytes.clone()).is_ok()),
        }
    }
}

/// Wait until the terminal `fd` has something to read, the self-pipe `wake`
/// fires, or `timeout` runs out (`None`: for as long as it takes).
#[allow(
    unsafe_code,
    reason = "poll(2) has no safe wrapper in the standard library; ADR-0013 \
              covers this call and its soundness argument"
)]
fn wait_for_input(fd: RawFd, wake: RawFd, timeout: Option<Duration>) -> io::Result<Ready> {
    // `poll` takes `int` milliseconds, -1 for no limit. Rounded up, so a wait
    // of a fraction of a millisecond does not spin; clamped, because nothing
    // here waits 24 days.
    let milliseconds = timeout.map_or(-1, |wait| {
        i32::try_from(wait.as_micros().div_ceil(1000)).unwrap_or(i32::MAX)
    });
    let mut watched = [
        libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        },
        libc::pollfd {
            fd: wake,
            events: libc::POLLIN,
            revents: 0,
        },
    ];
    // SAFETY: the pointer is to a live, fully initialised array of two
    // `pollfd`s, the count says two, and `poll` writes only `revents` back.
    // ADR-0013.
    let ready = unsafe { libc::poll(watched.as_mut_ptr(), 2, milliseconds) };
    if ready < 0 {
        return Err(io::Error::last_os_error());
    }
    if ready == 0 {
        return Ok(Ready::Timeout);
    }
    if watched[1].revents != 0 {
        // The byte carries nothing; it is the wake itself that matters.
        drain(wake);
        return Ok(Ready::Wake);
    }
    Ok(Ready::Input)
}

#[cfg(test)]
mod tests;
