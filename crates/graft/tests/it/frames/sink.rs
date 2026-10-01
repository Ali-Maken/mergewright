//! The in-memory terminal ([`Sink`]) and the parse of what it was sent.

use std::io::{self, Write};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use crate::drain_sink::DrainSink;
use crate::frame_trace::WriteClock;

/// The terminal: keeps every byte it is sent, and only copies them.
///
/// A write is a copy into a log, the way a write to a pty is a copy into the
/// kernel's buffer; parsing is the terminal's own work, which this test
/// leaves out. Frames note only how many bytes had been sent
/// ([`Sink::sent`]); [`Sink::count`] parses the log once, after the scroll,
/// so no timed window pays for the test's own bookkeeping (m1-the-graph/30f:
/// parsing each byte inside `write` added ~3.6 ms to every pixel page key).
#[derive(Clone, Default)]
pub(crate) struct Sink(Arc<Mutex<Log>>);

/// The bytes sent, in chunks so that growing the log never moves what is
/// already in it (a reallocation would copy megabytes inside one frame).
#[derive(Default)]
struct Log {
    chunks: Vec<Vec<u8>>,
    len: usize,
    /// Time inside `write`, read only when tracing.
    clock: WriteClock,
    /// [`Sink::draining`]: every write goes here and no byte is kept.
    drain: Option<Box<dyn Write + Send>>,
}

/// A new chunk's capacity, unless one write is bigger.
const CHUNK: usize = 4 << 20;

impl Sink {
    /// A sink that times its writes when `timed` (tracing is on).
    pub(super) fn timed(timed: bool) -> Self {
        Self(Arc::new(Mutex::new(Log {
            clock: WriteClock::new(timed),
            ..Log::default()
        })))
    }

    /// Writes go to `drain` (blocking while its pipe is full); no byte is kept.
    /// `sent` and `write_spent` mean what they mean for [`Sink::timed`]; the
    /// write time is always taken, and includes the time blocked.
    pub(crate) fn draining(drain: &DrainSink) -> Sink {
        Self(Arc::new(Mutex::new(Log {
            clock: WriteClock::new(true),
            drain: Some(Box::new(drain.writer())),
            ..Log::default()
        })))
    }

    /// Time spent inside `write` so far; zero unless timed.
    pub(super) fn write_spent(&self) -> Duration {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clock
            .spent()
    }

    /// Bytes sent so far.
    pub(crate) fn sent(&self) -> usize {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).len
    }

    /// Parses everything sent: the totals, and the running [`Counts`] at each
    /// of `marks` (byte offsets from [`Sink::sent`], in order).
    pub(super) fn count(&self, marks: &[usize]) -> (Written, Vec<Counts>) {
        let log = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        assert!(
            log.drain.is_none(),
            "Sink::count on a Sink::draining: it keeps no bytes (a harness bug)"
        );
        let mut written = Written::default();
        let mut at = Vec::with_capacity(marks.len());
        let mut marks = marks.iter().peekable();
        let mut offset = 0;
        let mut note = |written: &Written, offset: usize, at: &mut Vec<Counts>| {
            while marks.next_if(|mark| **mark <= offset).is_some() {
                at.push(written.counts);
            }
        };
        for &byte in log.chunks.iter().flatten() {
            note(&written, offset, &mut at);
            written.take(byte);
            offset += 1;
        }
        note(&written, offset, &mut at);
        (written, at)
    }
}

#[derive(Default)]
pub(super) struct Written {
    pub(super) bytes: usize,
    pub(super) kitty_commands: usize,
    counts: Counts,
    parse: Parse,
    control: Vec<u8>,
    /// The kind of the command being read, for its payload bytes.
    current: Option<CommandKind>,
}

/// Running totals of what was written, by kind.
#[derive(Clone, Copy, Default, Debug)]
pub(super) struct Counts {
    pub(super) bytes: usize,
    /// `a=t` commands: one per transmitted strip.
    pub(super) transmits: usize,
    /// Bytes of transmit commands, continuation chunks included.
    pub(super) transmit_bytes: usize,
    /// `a=p` commands.
    pub(super) placements: usize,
    /// `a=d,d=i`: placements removed.
    pub(super) deletes: usize,
    /// `a=d,d=I`: uploads freed.
    pub(super) frees: usize,
}

impl Counts {
    pub(super) fn minus(self, before: Counts) -> Counts {
        Counts {
            bytes: self.bytes - before.bytes,
            transmits: self.transmits - before.transmits,
            transmit_bytes: self.transmit_bytes - before.transmit_bytes,
            placements: self.placements - before.placements,
            deletes: self.deletes - before.deletes,
            frees: self.frees - before.frees,
        }
    }

    pub(super) fn plus(self, other: Counts) -> Counts {
        Counts {
            bytes: self.bytes + other.bytes,
            transmits: self.transmits + other.transmits,
            transmit_bytes: self.transmit_bytes + other.transmit_bytes,
            placements: self.placements + other.placements,
            deletes: self.deletes + other.deletes,
            frees: self.frees + other.frees,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum CommandKind {
    Transmit,
    Other,
}

/// Where the byte stream is, relative to `ESC _ G <control> [; payload] ESC \`.
#[derive(Default, Clone, Copy, PartialEq)]
enum Parse {
    #[default]
    Outside,
    Esc,
    EscUnderscore,
    Control,
    ControlEsc,
    Payload,
    PayloadEsc,
}

impl Written {
    fn take(&mut self, byte: u8) {
        self.bytes += 1;
        self.counts.bytes += 1;
        if matches!(self.current, Some(CommandKind::Transmit)) {
            self.counts.transmit_bytes += 1;
        }
        self.parse = match (self.parse, byte) {
            (Parse::Outside | Parse::Esc, 0x1b) => Parse::Esc,
            (Parse::Esc, b'_') => Parse::EscUnderscore,
            (Parse::EscUnderscore, b'G') => {
                self.kitty_commands += 1;
                self.control.clear();
                Parse::Control
            }
            (Parse::Outside | Parse::Esc | Parse::EscUnderscore, _) => Parse::Outside,
            (Parse::Control, b';') => {
                self.classify();
                Parse::Payload
            }
            (Parse::Control, 0x1b) => {
                self.classify();
                Parse::ControlEsc
            }
            (Parse::Control, _) => {
                self.control.push(byte);
                Parse::Control
            }
            (Parse::Payload, 0x1b) => Parse::PayloadEsc,
            (Parse::ControlEsc | Parse::PayloadEsc, b'\\') => {
                self.current = None;
                Parse::Outside
            }
            (Parse::Payload | Parse::ControlEsc | Parse::PayloadEsc, _) => Parse::Payload,
        };
    }

    /// The control keys are in: count the command by kind. A continuation
    /// chunk (`m=` only) belongs to the transmit before it.
    fn classify(&mut self) {
        let control = String::from_utf8_lossy(&self.control).into_owned();
        let keys: Vec<&str> = control.split(',').collect();
        let has = |key: &str| keys.contains(&key);
        if has("a=t") {
            self.counts.transmits += 1;
            self.current = Some(CommandKind::Transmit);
        } else if keys.iter().all(|key| key.starts_with("m=")) {
            self.current = Some(CommandKind::Transmit);
        } else if has("a=p") {
            self.counts.placements += 1;
            self.current = Some(CommandKind::Other);
        } else if has("d=i") {
            self.counts.deletes += 1;
            self.current = Some(CommandKind::Other);
        } else if has("d=I") {
            self.counts.frees += 1;
            self.current = Some(CommandKind::Other);
        } else {
            self.current = Some(CommandKind::Other);
        }
    }
}

impl Write for Sink {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut log = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let Log {
            chunks,
            len,
            clock,
            drain,
        } = &mut *log;
        if let Some(drain) = drain {
            clock.time(|| drain.write_all(buf))?;
            *len += buf.len();
            return Ok(buf.len());
        }
        clock.time(|| {
            let room = chunks
                .last()
                .map_or(0, |chunk| chunk.capacity() - chunk.len());
            if room < buf.len() {
                chunks.push(Vec::with_capacity(CHUNK.max(buf.len())));
            }
            if let Some(chunk) = chunks.last_mut() {
                chunk.extend_from_slice(buf);
            }
            *len += buf.len();
        });
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// The measurement change must not change what is counted: parsing the log
/// after the run gives, at every frame mark, exactly what parsing each write
/// as it arrived gave (the Sink before m1-the-graph/30f). Commands split
/// across writes, a continuation chunk, and a write bigger than a chunk.
#[test]
fn the_sink_counts_after_the_run_what_parsing_each_write_counted_during_it() {
    let transmit_payload = "A".repeat(CHUNK + 10);
    let stream: Vec<String> = vec![
        "plain text \x1b[31m red".to_owned(),
        format!("\x1b_Ga=t,f=32,i=1,m=1;{transmit_payload}\x1b\\"),
        "\x1b_Gm=0;QUJD\x1b\\".to_owned(),
        "\x1b_Ga=p,i=1,p=1\x1b\\\x1b_Ga=d,d=i,i=1\x1b\\".to_owned(),
        "\x1b_Ga=d,d=I,i=1\x1b\\\x1b_Gq=2\x1b\\ tail".to_owned(),
    ];
    let bytes: Vec<u8> = stream.concat().into_bytes();
    // Split everywhere interesting: inside ESC _ G, inside the control keys,
    // inside the payload, between ESC and '\'.
    let cuts = [0, 3, 13, 14, 15, 22, 40, CHUNK, CHUNK + 30, CHUNK + 33];
    let cuts: Vec<usize> = cuts
        .into_iter()
        .chain((CHUNK + 40..bytes.len()).step_by(7))
        .chain([bytes.len()])
        .collect();

    let mut sink = Sink::default();
    let mut during = Written::default();
    let mut expected = vec![during.counts];
    let mut marks = vec![0];
    for pair in cuts.windows(2) {
        let piece = &bytes[pair[0]..pair[1]];
        sink.write_all(piece).unwrap_or_else(|why| panic!("{why}"));
        for &byte in piece {
            during.take(byte);
        }
        expected.push(during.counts);
        marks.push(sink.sent());
    }

    let (after, counted) = sink.count(&marks);
    assert_eq!(sink.sent(), bytes.len(), "every byte kept");
    assert_eq!(after.bytes, during.bytes);
    assert_eq!(after.kitty_commands, during.kitty_commands);
    assert_eq!(after.kitty_commands, 6);
    let (transmits, placements, deletes, frees) = (1, 1, 1, 1);
    let last = counted.last().copied().unwrap_or_default();
    assert_eq!(
        (last.transmits, last.placements, last.deletes, last.frees),
        (transmits, placements, deletes, frees)
    );
    assert_eq!(format!("{counted:?}"), format!("{expected:?}"));
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::sync::PoisonError;

    use super::Sink;
    use crate::drain_sink::DrainSink;

    /// A draining sink forwards every byte to the drain, keeps none, and its
    /// `sent` is the drain's `written`.
    #[test]
    fn a_draining_sink_forwards_and_keeps_nothing() {
        let drain = DrainSink::new(64, 10_000_000);
        let mut sink = Sink::draining(&drain);
        let stream = format!("abc\x1b_Ga=t,i=3;{}\x1b\\ end", "A".repeat(1000));
        for piece in stream.as_bytes().chunks(37) {
            sink.write_all(piece).unwrap_or_else(|why| panic!("{why}"));
        }
        assert_eq!(sink.sent(), stream.len());
        assert_eq!(drain.written(), stream.len() as u64);
        assert_eq!(drain.kitty_transmits().len(), 1);
        let log = sink.0.lock().unwrap_or_else(PoisonError::into_inner);
        assert!(log.chunks.is_empty(), "no byte kept");
    }
}
