//! A terminal that drains at a fixed rate ([`DrainSink`]): a bounded pipe the
//! writer blocks on once it is full, emptied by a thread at
//! `bytes_per_second`. It models a terminal busy parsing, so a frame bigger
//! than the terminal can swallow holds the loop back as a real pty does
//! (m7-power-tools/42). It keeps counts and offsets, never the bytes: the
//! in-memory sink's 3.5 GB peak was the bytes it kept
//! (`docs/perf/scroll-lag-big-repo.txt:27`).
//!
//! Kitty transmits are found as the bytes pass ([`DrainSink::kitty_transmits`]),
//! from the command headers only, across write boundaries.
//!
//! It can answer the loop's fences (ADR-0054) as a terminal does
//! ([`DrainSink::answering_fences`]): each `CSI 5 n` is answered once the
//! drain has passed its last byte, never when it is written.

use std::collections::VecDeque;
use std::io::{self, Write};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, Weak};
use std::thread;
use std::time::{Duration, Instant};

/// How often the drain thread wakes to empty the pipe. It moves at most a
/// pipeful per tick, so a rate above `capacity / DRAIN_TICK` (16 MB/s for a
/// 16 KiB pipe, less where a 1 ms sleep oversleeps) drains no faster.
const DRAIN_TICK: Duration = Duration::from_millis(1);

/// The rate of a terminal that never falls behind: every byte is drained as
/// it is written, with no tick (m8 G-pace J1 "free").
pub(crate) const FREE: u64 = u64::MAX;

/// The longest Kitty control part kept while it is read; real ones are under
/// 100 bytes. Past this the header is cut, never the count.
const CONTROL_CAP: usize = 256;

/// One Kitty transmit seen in the stream: where its `ESC _ G` began, the
/// image id it named (`i=`; 0 when it named none), the image's width and
/// height in pixels (`s=`, `v=`) and whether its data went zlib-deflated (`o=z`). Read from
/// the header bytes only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct KittyTransmit {
    pub(crate) offset: u64,
    pub(crate) image_id: u32,
    pub(crate) width_px: Option<u32>,
    pub(crate) height_px: Option<u32>,
    pub(crate) deflated: bool,
}

/// The pipe and its drain. Cheap to clone: every clone is the same pipe.
#[derive(Clone)]
pub(crate) struct DrainSink(Arc<Shared>);

/// What the terminal does when the drain reads a fence: sends the loop its
/// answer.
pub(crate) type Answer = Box<dyn FnMut() + Send>;

struct Shared {
    state: Mutex<State>,
    /// Set by [`DrainSink::answering_fences`]; called by the drain thread,
    /// never while `state` is held.
    answer: Mutex<Option<Answer>>,
    /// Signalled whenever the drain makes room.
    room: Condvar,
    capacity: usize,
    /// Rate [`FREE`]: the writer drains its own bytes; there is no thread.
    free: bool,
}

struct State {
    /// Bytes in the pipe, not yet drained.
    buffered: usize,
    /// Bytes accepted so far.
    written: u64,
    /// `(drained so far, when)`, one entry per drain step that moved bytes.
    progress: Vec<(u64, Instant)>,
    /// When a byte last moved, in or out.
    last_activity: Instant,
    scan: Scan,
    transmits: Vec<KittyTransmit>,
    fence: FenceScan,
    /// Offsets one past each fence's last byte, not yet drained past.
    fences: VecDeque<u64>,
}

impl State {
    fn drained(&self) -> u64 {
        self.progress.last().map_or(0, |(total, _)| *total)
    }
}

/// Lock, taking the data on poison: every write to it is a counter update
/// that cannot leave it half-done, so a panicked holder leaves it whole.
fn lock(shared: &Shared) -> MutexGuard<'_, State> {
    shared.state.lock().unwrap_or_else(PoisonError::into_inner)
}

impl DrainSink {
    /// A pipe of `capacity` bytes drained at `bytes_per_second`.
    pub(crate) fn new(capacity: usize, bytes_per_second: u64) -> DrainSink {
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                buffered: 0,
                written: 0,
                progress: Vec::new(),
                last_activity: Instant::now(),
                scan: Scan::default(),
                transmits: Vec::new(),
                fence: FenceScan::default(),
                fences: VecDeque::new(),
            }),
            answer: Mutex::new(None),
            room: Condvar::new(),
            capacity: capacity.max(1),
            free: bytes_per_second == FREE,
        });
        if shared.free {
            return DrainSink(shared);
        }
        let weak = Arc::downgrade(&shared);
        // The thread holds only a `Weak`: it stops once every handle is gone.
        let spawned = thread::Builder::new()
            .name("drain-sink".to_owned())
            .spawn(move || drain(&weak, bytes_per_second));
        if let Err(why) = spawned {
            panic!("DrainSink: the drain thread did not start: {why}");
        }
        DrainSink(shared)
    }

    /// A writer into the pipe; blocks while the pipe is full.
    pub(crate) fn writer(&self) -> impl Write + Send + 'static {
        PipeWriter(Arc::clone(&self.0))
    }

    /// Bytes accepted so far.
    pub(crate) fn written(&self) -> u64 {
        lock(&self.0).written
    }

    /// When the drain passed `offset` (had drained at least that many
    /// bytes); `None` if it has not yet.
    pub(crate) fn drained_at(&self, offset: u64) -> Option<Instant> {
        let state = lock(&self.0);
        if offset == 0 {
            return state.progress.first().map(|(_, at)| *at);
        }
        let at = state.progress.partition_point(|(total, _)| *total < offset);
        state.progress.get(at).map(|(_, when)| *when)
    }

    /// The pipe is empty and no byte has moved for `quiet`.
    pub(crate) fn quiet_for(&self, quiet: Duration) -> bool {
        let state = lock(&self.0);
        state.buffered == 0 && state.last_activity.elapsed() >= quiet
    }

    /// Answer every fence (`CSI 5 n`) from now on: `answer` runs on the
    /// drain thread once per fence, when the drain passes its last byte.
    pub(crate) fn answering_fences(&self, answer: impl FnMut() + Send + 'static) {
        *self.0.answer.lock().unwrap_or_else(PoisonError::into_inner) = Some(Box::new(answer));
    }

    /// Fences written so far.
    pub(crate) fn fences_written(&self) -> u64 {
        lock(&self.0).fence.seen
    }

    /// Every Kitty transmit seen so far, in stream order.
    pub(crate) fn kitty_transmits(&self) -> Vec<KittyTransmit> {
        lock(&self.0).transmits.clone()
    }
}

/// The drain thread: every [`DRAIN_TICK`], empties what the elapsed time at
/// `rate` allows. Credit does not build up while the pipe is empty.
fn drain(shared: &Weak<Shared>, rate: u64) {
    // Credit in byte-nanoseconds: whole bytes move, the rest carries over.
    const NANOS: u128 = 1_000_000_000;
    let mut last = Instant::now();
    let mut credit: u128 = 0;
    loop {
        thread::sleep(DRAIN_TICK);
        let Some(shared) = shared.upgrade() else {
            return;
        };
        let now = Instant::now();
        let mut state = lock(&shared);
        if state.buffered == 0 {
            credit = 0;
            last = now;
            continue;
        }
        credit += now.duration_since(last).as_nanos() * u128::from(rate);
        last = now;
        let whole = usize::try_from(credit / NANOS).unwrap_or(usize::MAX);
        let moved = whole.min(state.buffered);
        if moved == 0 {
            continue;
        }
        credit -= moved as u128 * NANOS;
        let read = drained(&mut state, moved, now);
        drop(state);
        shared.room.notify_all();
        answer(&shared, read);
    }
}

/// `moved` more bytes drained at `now`: returns how many fences that read.
fn drained(state: &mut State, moved: usize, now: Instant) -> usize {
    state.buffered -= moved;
    let total = state.drained() + moved as u64;
    state.progress.push((total, now));
    state.last_activity = now;
    let read = state.fences.partition_point(|&end| end <= total);
    state.fences.drain(..read);
    read
}

/// The terminal answers `read` fences. Never called while `state` is held.
fn answer(shared: &Shared, read: usize) {
    if read == 0 {
        return;
    }
    if let Some(answer) = shared
        .answer
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_mut()
    {
        for _ in 0..read {
            answer();
        }
    }
}

struct PipeWriter(Arc<Shared>);

impl Write for PipeWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let shared = &*self.0;
        let mut state = lock(shared);
        while state.buffered >= shared.capacity {
            state = shared
                .room
                .wait(state)
                .unwrap_or_else(PoisonError::into_inner);
        }
        let taken = buf.len().min(shared.capacity - state.buffered);
        let State {
            written,
            scan,
            transmits,
            fence,
            fences,
            ..
        } = &mut *state;
        scan.feed(*written, &buf[..taken], transmits);
        fence.feed(*written, &buf[..taken], fences);
        state.buffered += taken;
        state.written += taken as u64;
        state.last_activity = Instant::now();
        if shared.free {
            let read = drained(&mut state, taken, Instant::now());
            drop(state);
            answer(shared, read);
        }
        Ok(taken)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// The loop's fence (`mergewright_term::write_fence`).
const FENCE: &[u8] = b"\x1b[5n";

/// Finds fences as the bytes pass, across write boundaries. A fence's bytes
/// never occur inside a Kitty payload (base64) or text (escaped), so a plain
/// match is exact.
#[derive(Default)]
pub(crate) struct FenceScan {
    /// How much of [`FENCE`] the last bytes matched.
    matched: usize,
    seen: u64,
}

impl FenceScan {
    /// Pushes onto `out` the offset one past each fence's last byte;
    /// `base` is the offset of `bytes[0]`.
    pub(crate) fn feed(&mut self, base: u64, bytes: &[u8], out: &mut VecDeque<u64>) {
        for (index, &byte) in bytes.iter().enumerate() {
            // A miss restarts the match, at this byte if it is an `ESC`.
            self.matched = if byte == FENCE[self.matched] {
                self.matched + 1
            } else {
                usize::from(byte == FENCE[0])
            };
            if self.matched == FENCE.len() {
                self.matched = 0;
                self.seen += 1;
                out.push_back(base + index as u64 + 1);
            }
        }
    }
}

/// Where the stream is, relative to `ESC _ G <control> [; payload] ESC \`.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum Parse {
    #[default]
    Outside,
    Esc,
    EscUnderscore,
    Control,
    Payload,
    PayloadEsc,
}

/// The Kitty header scanner: keeps the control part of the command being
/// read, never a payload byte.
#[derive(Default)]
struct Scan {
    parse: Parse,
    /// Offset of the `ESC` that may begin a command.
    start: u64,
    control: Vec<u8>,
    /// The last transmit said `m=1`: a following command with no `a=` is
    /// its continuation chunk, not a new transmit.
    chunking: bool,
}

impl Scan {
    fn feed(&mut self, base: u64, bytes: &[u8], out: &mut Vec<KittyTransmit>) {
        for (index, &byte) in bytes.iter().enumerate() {
            let offset = base + index as u64;
            self.parse = match (self.parse, byte) {
                (Parse::Outside | Parse::Esc | Parse::EscUnderscore, 0x1b) => {
                    self.start = offset;
                    Parse::Esc
                }
                (Parse::Esc, b'_') => Parse::EscUnderscore,
                (Parse::EscUnderscore, b'G') => {
                    self.control.clear();
                    Parse::Control
                }
                // Not a Kitty command, or the end of one (`ESC \`).
                (Parse::Outside | Parse::Esc | Parse::EscUnderscore, _)
                | (Parse::PayloadEsc, b'\\') => Parse::Outside,
                (Parse::Control, b';') => {
                    self.classify(out);
                    Parse::Payload
                }
                (Parse::Control, 0x1b) => {
                    self.classify(out);
                    Parse::PayloadEsc
                }
                (Parse::Control, _) => {
                    if self.control.len() < CONTROL_CAP {
                        self.control.push(byte);
                    }
                    Parse::Control
                }
                (Parse::Payload | Parse::PayloadEsc, 0x1b) => Parse::PayloadEsc,
                (Parse::Payload | Parse::PayloadEsc, _) => Parse::Payload,
            };
        }
    }

    /// The control keys are in: a transmit is `a=t`, `a=T`, or no `a=` at
    /// all (Kitty's default action is `t`) unless it continues an `m=1`
    /// chunk.
    fn classify(&mut self, out: &mut Vec<KittyTransmit>) {
        let control = String::from_utf8_lossy(&self.control).into_owned();
        let value = |key: &str| {
            control
                .split(',')
                .find_map(|pair| pair.strip_prefix(key)?.strip_prefix('='))
        };
        let more = value("m") == Some("1");
        let transmit = match value("a") {
            Some("t" | "T") => true,
            Some(_) => {
                self.chunking = false;
                return;
            }
            None => !self.chunking,
        };
        self.chunking = more;
        if transmit {
            out.push(KittyTransmit {
                offset: self.start,
                image_id: value("i").and_then(|id| id.parse().ok()).unwrap_or(0),
                width_px: value("s").and_then(|width| width.parse().ok()),
                height_px: value("v").and_then(|height| height.parse().ok()),
                deflated: value("o") == Some("z"),
            });
        }
    }
}

#[cfg(test)]
mod tests;
