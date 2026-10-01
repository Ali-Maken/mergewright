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

use std::io::{self, Write};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, Weak};
use std::thread;
use std::time::{Duration, Instant};

/// How often the drain thread wakes to empty the pipe.
const DRAIN_TICK: Duration = Duration::from_millis(1);

/// The longest Kitty control part kept while it is read; real ones are under
/// 100 bytes. Past this the header is cut, never the count.
const CONTROL_CAP: usize = 256;

/// One Kitty transmit seen in the stream: where its `ESC _ G` began and the
/// image id it named (`i=`; 0 when it named none).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct KittyTransmit {
    pub(crate) offset: u64,
    pub(crate) image_id: u32,
}

/// The pipe and its drain. Cheap to clone: every clone is the same pipe.
#[derive(Clone)]
pub(crate) struct DrainSink(Arc<Shared>);

struct Shared {
    state: Mutex<State>,
    /// Signalled whenever the drain makes room.
    room: Condvar,
    capacity: usize,
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
            }),
            room: Condvar::new(),
            capacity: capacity.max(1),
        });
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
        state.buffered -= moved;
        let total = state.drained() + moved as u64;
        state.progress.push((total, now));
        state.last_activity = now;
        drop(state);
        shared.room.notify_all();
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
            ..
        } = &mut *state;
        scan.feed(*written, &buf[..taken], transmits);
        state.buffered += taken;
        state.written += taken as u64;
        state.last_activity = Instant::now();
        Ok(taken)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
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
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wait_quiet(sink: &DrainSink) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !sink.quiet_for(Duration::from_millis(20)) {
            assert!(Instant::now() < deadline, "the drain never went quiet");
            thread::sleep(Duration::from_millis(5));
        }
    }

    /// 1 KiB pipe at 40 KB/s, 20 KB written: the writer is held back to the
    /// rate (±10%), and `written`, `drained_at`, `quiet_for` agree.
    #[test]
    fn drains_at_its_rate() {
        const RATE: u64 = 40_000;
        const TOTAL: usize = 20_000;
        let sink = DrainSink::new(1024, RATE);
        let mut writer = sink.writer();
        assert!(sink.quiet_for(Duration::ZERO), "empty at the start");

        let started = Instant::now();
        let first = writer
            .write(&vec![b'x'; 4096])
            .unwrap_or_else(|why| panic!("{why}"));
        assert_eq!(first, 1024, "a write takes only the room there is");
        assert_eq!(sink.written(), 1024);
        assert!(!sink.quiet_for(Duration::ZERO), "full, not quiet");
        writer
            .write_all(&vec![b'x'; TOTAL - 1024])
            .unwrap_or_else(|why| panic!("{why}"));
        let wrote = started.elapsed().as_millis();
        assert_eq!(sink.written(), TOTAL as u64);
        // The writer returns once the last byte is in the pipe: TOTAL minus
        // one pipeful had to drain first.
        let at_rate = |bytes: usize| bytes as u128 * 1000 / u128::from(RATE);
        let within = |took: u128, expected: u128| took.abs_diff(expected) * 10 <= expected;
        let expected = at_rate(TOTAL - 1024);
        assert!(
            within(wrote, expected),
            "writer held {wrote} ms, rate says {expected} ms"
        );

        wait_quiet(&sink);
        let start = sink.drained_at(1).unwrap_or_else(|| panic!("first byte"));
        let end = sink
            .drained_at(TOTAL as u64)
            .unwrap_or_else(|| panic!("last byte drained"));
        assert!(
            sink.drained_at(TOTAL as u64 + 1).is_none(),
            "nothing past the end"
        );
        let drained = end.duration_since(start).as_millis();
        let expected = at_rate(TOTAL);
        assert!(
            within(drained, expected),
            "drained in {drained} ms, rate says {expected} ms"
        );
        assert!(sink.kitty_transmits().is_empty());
    }

    #[test]
    fn counts_kitty_transmits_across_writes() {
        let payload = "QUJD".repeat(5000);
        let stream = [
            format!("text\x1b[31m\x1b_Ga=T,f=32,i=7,m=1;{payload}\x1b\\"),
            format!("\x1b_Gm=1;{payload}\x1b\\\x1b_Gm=0;{payload}\x1b\\"),
            "\x1b_Ga=p,i=7,p=1\x1b\\\x1b_Ga=d,d=i,i=7\x1b\\".to_owned(),
            format!("\x1b_Gf=100,i=2147483657;{payload}\x1b\\"),
            format!("\x1b_Ga=t,i=9,q=2;{payload}\x1b\\ tail"),
        ]
        .concat()
        .into_bytes();
        let starts: Vec<u64> = stream
            .windows(3)
            .enumerate()
            .filter(|(_, w)| *w == b"\x1b_G")
            .map(|(at, _)| at as u64)
            .collect();
        // Fast drain; a pipe smaller than the payload so writes split.
        let sink = DrainSink::new(4096, 50_000_000);
        let mut writer = sink.writer();
        // Split inside the first header (after `ESC _`, inside `i=7`).
        let cuts = [0, 11, 22, 30, stream.len()];
        for pair in cuts.windows(2) {
            writer
                .write_all(&stream[pair[0]..pair[1]])
                .unwrap_or_else(|why| panic!("{why}"));
        }
        assert_eq!(sink.written(), stream.len() as u64);
        let found = sink.kitty_transmits();
        assert_eq!(
            found,
            vec![
                KittyTransmit {
                    offset: starts[0],
                    image_id: 7
                },
                KittyTransmit {
                    offset: starts[5],
                    image_id: 2_147_483_657
                },
                KittyTransmit {
                    offset: starts[6],
                    image_id: 9
                },
            ],
            "a=T, no-a first chunk, a=t; not m= continuations, a=p, a=d"
        );
        wait_quiet(&sink);
    }
}
