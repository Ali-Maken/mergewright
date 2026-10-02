//! The frame trace the shipped binary writes when asked (m8-polish-and-ship
//! G-trace): `GRAFT_PERF_TRACE=<file>` makes every drawn frame append one
//! line of numbers saying where its time went, so a run on the user's own
//! repository, in his own terminal, can be read without guessing.
//!
//! **Numbers only.** A line never holds a path, a ref name, a sha or a
//! subject: the file is something he sends, and the repository is his
//! employer's. The header's two words (the tier, `$TERM_PROGRAM`) are the
//! only text, and `$TERM_PROGRAM` is cut to `[A-Za-z0-9._-]`.
//!
//! ```text
//! header tier=unicode columns=187 rows=50 history_rows=17694 refs=1323 term=ghostty
//! frame at_us=812345 wakes=3 motions=2 queued=0 update_us=41 commands=1 draw_us=903 bytes=5120 write_us=88 fence_us=- input_age_us=1032
//! ```
//!
//! A header line goes out before the first frame and again whenever one of
//! its numbers changes (the history grows while it loads; a resize); the
//! summariser (`scripts/m8-trace-summary.sh`) reads the last. `-` is "not
//! measured": `fence_us` on a frame no fence answer released (unfenced, the
//! first frame, a timeout), `input_age_us` on a frame no input drew.
//!
//! Off, the loop pays one `Option` check per hook ([`Tracer::off`]) and the
//! terminal's writer is the plain `Stdout` (`event_loop::startup`); on, the
//! writer is wrapped in [`Metered`], which counts the bytes and the time
//! spent in the writes that reach the terminal.
//!
//! The test-only `GRAFT_FRAME_TRACE` (`tests/it/frame_trace.rs`) is a
//! different thing — per-key phases inside the perf harness — and is left as
//! it is.

use std::collections::VecDeque;
use std::ffi::OsString;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use graft_core::capabilities::RenderingTier;
use graft_core::event::{Event, MouseKind};

use crate::event_loop::Wake;
use crate::update::{AppEvent, AppState};

/// The variable that turns the trace on. Renamed in G-rename
/// (`TRIB_PERF_TRACE`).
pub(crate) const VAR: &str = "GRAFT_PERF_TRACE";

/// One drawn frame, as the trace line says it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct FrameRecord {
    /// When the frame finished writing, in µs since the trace opened. Not in
    /// the design note's struct: frames/s and the tail after the last input
    /// cannot be had without it.
    pub at_us: u64,
    /// Wakes taken since the frame before: keys, wheel, job answers, all.
    pub wakes: u16,
    /// Of those, the keys and wheel scrolls applied — the inputs a held key
    /// or a flicked wheel sends. Not split by what the key does: the trace
    /// is for scroll runs, where every key is a motion.
    pub motions: u16,
    /// Keys and mouse events already sent to the loop but not yet applied,
    /// counted when the frame's first wake was taken.
    pub queued: u16,
    /// Time inside `update` over the frame's wakes.
    pub update_us: u32,
    /// Commands `update` returned over the frame's wakes.
    pub commands: u16,
    /// Building the frame — view, diff, image bytes — less `write_us`.
    pub draw_us: u32,
    /// Bytes handed to the terminal since the frame before (with the
    /// metered writer; 0 without it).
    pub bytes: u32,
    /// Time inside the terminal writer's `write`/`flush` — blocked on the
    /// pty when the terminal reads slower than Graft writes.
    pub write_us: u32,
    /// How long the terminal took to answer the fence before this frame
    /// (G-pace, ADR-0054): the previous frame's write to its `CSI 0 n`, i.e.
    /// how long the terminal took to read it. The last answer since the
    /// frame before; `None` when none came (unfenced, or drawn on a timeout).
    pub fence_us: Option<u32>,
    /// The oldest applied input's age when its frame finished writing,
    /// from when the input thread forwarded it.
    pub input_age_us: Option<u32>,
}

/// What the header line says. Repeated whenever it changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Header {
    pub tier: RenderingTier,
    pub columns: u16,
    pub rows: u16,
    pub history_rows: usize,
    pub refs: usize,
    /// `$TERM_PROGRAM`, already cut by [`term_word`].
    pub term: String,
}

/// The trace file.
pub(crate) struct FrameTrace {
    /// A `BufWriter<File>` in the shipped binary; anything in a test.
    out: Box<dyn Write>,
}

impl FrameTrace {
    /// The trace `GRAFT_PERF_TRACE` names, created (truncated) now.
    /// `Ok(None)` when the variable is unset or empty; an `Err` when the
    /// file cannot be created, which the loop shows and runs on without.
    pub(crate) fn from_env() -> io::Result<Option<FrameTrace>> {
        Self::from_var(std::env::var_os(VAR))
    }

    /// [`Self::from_env`] with the variable's value handed in, so a test
    /// never has to set the process's environment.
    pub(crate) fn from_var(value: Option<OsString>) -> io::Result<Option<FrameTrace>> {
        match value {
            None => Ok(None),
            Some(path) if path.is_empty() => Ok(None),
            Some(path) => {
                let file = File::create(path)?;
                Ok(Some(FrameTrace::writing_to(Box::new(BufWriter::new(file)))))
            }
        }
    }

    /// A trace into `out`.
    pub(crate) fn writing_to(out: Box<dyn Write>) -> FrameTrace {
        FrameTrace { out }
    }

    /// Write the header line.
    pub(crate) fn header(&mut self, header: &Header) -> io::Result<()> {
        let tier = match header.tier {
            RenderingTier::Pixel => "pixel",
            RenderingTier::Unicode => "unicode",
            RenderingTier::Ascii => "ascii",
        };
        writeln!(
            self.out,
            "header tier={tier} columns={} rows={} history_rows={} refs={} term={}",
            header.columns, header.rows, header.history_rows, header.refs, header.term
        )?;
        self.out.flush()
    }

    /// Write one frame's line. Flushed at once: a trace cut short by a
    /// crash still holds every frame before it, and a failing disk is
    /// reported on the frame that hit it, not at exit when nobody can see.
    pub(crate) fn record(&mut self, r: &FrameRecord) -> io::Result<()> {
        writeln!(
            self.out,
            "frame at_us={} wakes={} motions={} queued={} update_us={} commands={} \
             draw_us={} bytes={} write_us={} fence_us={} input_age_us={}",
            r.at_us,
            r.wakes,
            r.motions,
            r.queued,
            r.update_us,
            r.commands,
            r.draw_us,
            r.bytes,
            r.write_us,
            Maybe(r.fence_us),
            Maybe(r.input_age_us),
        )?;
        self.out.flush()
    }
}

/// `-` for a number not measured.
struct Maybe(Option<u32>);

impl std::fmt::Display for Maybe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Some(n) => write!(f, "{n}"),
            None => f.write_str("-"),
        }
    }
}

/// `$TERM_PROGRAM` as one safe word: `[A-Za-z0-9._-]` kept, anything else
/// `_`, at most 32 characters; `-` when unset or empty.
pub(crate) fn term_word(value: Option<&str>) -> String {
    let word: String = value
        .unwrap_or_default()
        .chars()
        .take(32)
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if word.is_empty() {
        "-".to_owned()
    } else {
        word
    }
}

/// Bytes and time spent in the terminal writer, shared between [`Metered`]
/// (which adds) and the [`Tracer`] (which takes).
#[derive(Clone, Debug, Default)]
pub(crate) struct Meter(Arc<Counts>);

#[derive(Debug, Default)]
struct Counts {
    bytes: AtomicU64,
    nanos: AtomicU64,
}

impl Meter {
    fn add(&self, bytes: usize, spent: Duration) {
        let bytes = u64::try_from(bytes).unwrap_or(u64::MAX);
        let nanos = u64::try_from(spent.as_nanos()).unwrap_or(u64::MAX);
        self.0.bytes.fetch_add(bytes, Ordering::Relaxed);
        self.0.nanos.fetch_add(nanos, Ordering::Relaxed);
    }

    /// What was written since the last take, and how long it took.
    fn take(&self) -> (u64, Duration) {
        let bytes = self.0.bytes.swap(0, Ordering::Relaxed);
        let nanos = self.0.nanos.swap(0, Ordering::Relaxed);
        (bytes, Duration::from_nanos(nanos))
    }
}

/// A writer that tells a [`Meter`] what passed through it and how long the
/// inner writer took. Put it *under* a buffer (`LineWriter<Metered<Stdout>>`)
/// so it times the writes that reach the terminal, not every small
/// `queue!` — timing those would cost more than they do.
pub(crate) struct Metered<W> {
    inner: W,
    meter: Meter,
}

impl<W> Metered<W> {
    pub(crate) fn new(inner: W, meter: Meter) -> Self {
        Self { inner, meter }
    }
}

impl<W: Write> Write for Metered<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let began = Instant::now();
        let written = self.inner.write(buf);
        let spent = began.elapsed();
        self.meter
            .add(written.as_ref().map_or(0, |&written| written), spent);
        written
    }

    fn flush(&mut self) -> io::Result<()> {
        let began = Instant::now();
        let flushed = self.inner.flush();
        self.meter.add(0, began.elapsed());
        flushed
    }
}

/// The terminal's writer: `Stdout` as it always was, or — tracing — the same
/// 1 KiB line buffer `Stdout` keeps, over a [`Metered`] `Stdout`, so the
/// writes reach the terminal in the same pieces and each one is timed.
pub(crate) enum TerminalOut {
    Plain(io::Stdout),
    Metered(io::LineWriter<Metered<io::Stdout>>),
}

impl TerminalOut {
    /// `Metered` when there is a meter to report to.
    pub(crate) fn for_meter(meter: Option<Meter>) -> TerminalOut {
        match meter {
            None => TerminalOut::Plain(io::stdout()),
            Some(meter) => {
                TerminalOut::Metered(io::LineWriter::new(Metered::new(io::stdout(), meter)))
            }
        }
    }
}

impl Write for TerminalOut {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            TerminalOut::Plain(out) => out.write(buf),
            TerminalOut::Metered(out) => out.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            TerminalOut::Plain(out) => out.flush(),
            TerminalOut::Metered(out) => out.flush(),
        }
    }
}

/// The loop's side of the trace: off, or the trace plus the frame being
/// counted. Every hook is one `Option` check when off.
#[derive(Default)]
pub(crate) struct Tracer {
    on: Option<Box<Active>>,
    /// Why tracing stopped (or never started), until the loop has put it on
    /// the status row. Said once.
    unreported: Option<String>,
}

/// The parts of a trace the rest of startup wires in.
pub(crate) struct Opened {
    pub tracer: Tracer,
    /// For the terminal writer: `Some` only when tracing.
    pub meter: Option<Meter>,
    /// For the input forwarder: one `Instant` per key or mouse event, sent
    /// just before the event itself. `Some` only when tracing.
    pub stamps: Option<Sender<Instant>>,
}

struct Active {
    trace: FrameTrace,
    opened: Instant,
    term: String,
    meter: Option<Meter>,
    stamps: Option<Receiver<Instant>>,
    /// Stamps taken off the channel, not yet matched to their event.
    pending: VecDeque<Instant>,
    header: Option<Header>,
    frame: Counting,
}

/// The frame being counted.
#[derive(Default)]
struct Counting {
    wakes: u16,
    motions: u16,
    queued: u16,
    update: Duration,
    commands: u16,
    oldest_input: Option<Instant>,
    fence: Option<Duration>,
}

impl Tracer {
    /// No trace: every hook does nothing.
    pub(crate) fn off() -> Tracer {
        Tracer::default()
    }

    /// What the process's environment asks for: [`FrameTrace::from_env`]
    /// opened under `$TERM_PROGRAM`'s [`term_word`]. A `$TERM_PROGRAM` that
    /// is not UTF-8 is read lossily — it only labels the header.
    pub(crate) fn from_env() -> Opened {
        let term = std::env::var_os("TERM_PROGRAM");
        let term = term.as_deref().map(|term| term.to_string_lossy());
        Tracer::open(FrameTrace::from_env(), term_word(term.as_deref()))
    }

    /// The tracer for what [`FrameTrace::from_env`] answered: on with a
    /// meter and a stamp channel, off, or off with the reason waiting for
    /// the status row.
    pub(crate) fn open(opened: io::Result<Option<FrameTrace>>, term: String) -> Opened {
        match opened {
            Ok(Some(trace)) => {
                let meter = Meter::default();
                let (stamps, received) = mpsc::channel();
                Opened {
                    tracer: Tracer::on(trace, term, Some(meter.clone()), Some(received)),
                    meter: Some(meter),
                    stamps: Some(stamps),
                }
            }
            Ok(None) => Opened {
                tracer: Tracer::off(),
                meter: None,
                stamps: None,
            },
            Err(error) => Opened {
                tracer: Tracer {
                    on: None,
                    unreported: Some(error.to_string()),
                },
                meter: None,
                stamps: None,
            },
        }
    }

    /// Tracing into `trace`, with what the writer and the input thread
    /// report when they are wired (tests leave either `None`).
    pub(crate) fn on(
        trace: FrameTrace,
        term: String,
        meter: Option<Meter>,
        stamps: Option<Receiver<Instant>>,
    ) -> Tracer {
        Tracer {
            on: Some(Box::new(Active {
                trace,
                opened: Instant::now(),
                term,
                meter,
                stamps,
                pending: VecDeque::new(),
                header: None,
                frame: Counting::default(),
            })),
            unreported: None,
        }
    }

    /// Now, when tracing; `None` (and no clock read) when not.
    pub(crate) fn clock(&self) -> Option<Instant> {
        self.on.as_ref().map(|_| Instant::now())
    }

    /// The loop took `wake`.
    pub(crate) fn took(&mut self, wake: &Wake) {
        let Some(active) = &mut self.on else { return };
        let first = active.frame.wakes == 0;
        active.frame.wakes = active.frame.wakes.saturating_add(1);
        // Drained whatever the wake: a frame that starts on a job answer
        // still has keys waiting behind it (review m8-01 Major 1).
        if let Some(stamps) = &active.stamps {
            active.pending.extend(stamps.try_iter());
        }
        let (stamped, motion) = match wake {
            Wake::Event(AppEvent::Core(Event::Key(_))) => (true, true),
            Wake::Event(AppEvent::Core(Event::Mouse(mouse))) => (
                true,
                matches!(
                    mouse.kind,
                    MouseKind::ScrollUp
                        | MouseKind::ScrollDown
                        | MouseKind::ScrollLeft
                        | MouseKind::ScrollRight
                ),
            ),
            _ => (false, false),
        };
        if motion {
            active.frame.motions = active.frame.motions.saturating_add(1);
        }
        if stamped && let Some(sent) = active.pending.pop_front() {
            active.frame.oldest_input.get_or_insert(sent);
        }
        if first {
            active.frame.queued = u16::try_from(active.pending.len()).unwrap_or(u16::MAX);
        }
    }

    /// One `update` ran from `began` and returned `commands` commands.
    pub(crate) fn updated(&mut self, began: Option<Instant>, commands: usize) {
        let (Some(active), Some(began)) = (&mut self.on, began) else {
            return;
        };
        active.frame.update += began.elapsed();
        let commands = u16::try_from(commands).unwrap_or(u16::MAX);
        active.frame.commands = active.frame.commands.saturating_add(commands);
    }

    /// The terminal answered a fence; `waited` is how long that fence
    /// waited (`None`: no fence was waiting for it). Not a wake in the
    /// frame's count: it carries no input and no job.
    pub(crate) fn fenced(&mut self, waited: Option<Duration>) {
        let (Some(active), Some(waited)) = (&mut self.on, waited) else {
            return;
        };
        active.frame.fence = Some(waited);
    }

    /// A frame began drawing at `began` and is written. Writes its line (and
    /// a header first when the header changed). A write that fails stops the
    /// trace for the run; [`Self::unreported`] then says why, once.
    pub(crate) fn drew(&mut self, began: Option<Instant>, state: &AppState) {
        let (Some(active), Some(began)) = (&mut self.on, began) else {
            return;
        };
        let now = Instant::now();
        let (bytes, write) = active
            .meter
            .as_ref()
            .map_or((0, Duration::ZERO), Meter::take);
        let frame = std::mem::take(&mut active.frame);
        let record = FrameRecord {
            at_us: u64::try_from(now.duration_since(active.opened).as_micros()).unwrap_or(u64::MAX),
            wakes: frame.wakes,
            motions: frame.motions,
            queued: frame.queued,
            update_us: micros(frame.update),
            commands: frame.commands,
            draw_us: micros(now.duration_since(began).saturating_sub(write)),
            bytes: u32::try_from(bytes).unwrap_or(u32::MAX),
            write_us: micros(write),
            fence_us: frame.fence.map(micros),
            input_age_us: frame
                .oldest_input
                .map(|sent| micros(now.duration_since(sent))),
        };
        let header = Header {
            tier: state.tier(),
            columns: state.size.columns,
            rows: state.size.rows,
            history_rows: state.history.as_ref().map_or(0, |history| history.rows),
            refs: state.refs.as_ref().map_or(0, |refs| refs.count()),
            term: active.term.clone(),
        };
        let written = if active.header.as_ref() == Some(&header) {
            Ok(())
        } else {
            let written = active.trace.header(&header);
            active.header = Some(header);
            written
        };
        if let Err(error) = written.and_then(|()| active.trace.record(&record)) {
            self.on = None;
            self.unreported = Some(error.to_string());
        }
    }

    /// Why the trace stopped or never started, the first time it is asked;
    /// `None` after that and whenever nothing went wrong.
    pub(crate) fn unreported(&mut self) -> Option<String> {
        self.unreported.take()
    }
}

/// `duration` in whole µs. A frame longer than `u32::MAX` µs (71 minutes)
/// reads as `u32::MAX`: the field is a count of µs, and the cap is the
/// honest largest one it can hold.
fn micros(duration: Duration) -> u32 {
    u32::try_from(duration.as_micros()).unwrap_or(u32::MAX)
}

#[cfg(test)]
pub(crate) mod tests;
