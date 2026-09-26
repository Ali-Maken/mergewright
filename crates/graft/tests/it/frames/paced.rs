//! The loop's wakes ([`Paced`]): the script's keys and the jobs' events, each
//! stamped as the loop takes it and as its frame's phases end.

use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use graft::event_loop::{Phase, Wake, Wakes, is_job_event};
use graft::history::HistoryUpdate;
use graft::jobs::Dispatch;
use graft::update::{AppEvent, Command, Progress};
use graft_core::event::{Event, KeyCode, KeyPress};

use super::PATIENCE;
use super::sink::Sink;
use crate::frame_trace::{Posted, SentLog};

/// The job pool, noting the id of every `ShowRows` the loop sends.
pub(super) struct Watching<'a, D> {
    pub(super) inner: &'a D,
    pub(super) asked: Mutex<Vec<u64>>,
}

impl<D: Dispatch> Dispatch for Watching<'_, D> {
    fn dispatch(&self, command: Command) {
        if let Command::ShowRows(_, request) = &command {
            self.asked
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(request.id);
        }
        self.inner.dispatch(command);
    }
}

impl<D> Watching<'_, D> {
    /// How many `ShowRows` the loop has sent, and the newest id.
    pub(super) fn asked_so_far(&self) -> (usize, Option<u64>) {
        let asked = self.asked.lock().unwrap_or_else(PoisonError::into_inner);
        (asked.len(), asked.last().copied())
    }

    pub(super) fn asked(&self) -> Vec<u64> {
        self.asked
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// What one wake was.
#[derive(Clone, Copy, Debug)]
pub(super) enum Kind {
    /// The key at this index of the script, sent by the input thread at
    /// this instant: where its latency starts.
    Key(usize, Instant),
    /// A window of rows answering this request.
    Window(u64),
    /// Any other job event.
    Other,
}

/// One wake handed to the loop.
pub(super) struct Handed {
    pub(super) kind: Kind,
    /// [`Wake::label`]: what it was, without its payload.
    pub(super) label: &'static str,
    /// Just before it was sent (the input thread's or a job's send).
    pub(super) sent: Instant,
    /// When the loop took it: its frame runs from here to the next stamp.
    pub(super) at: Instant,
    /// A key: the script's tick for it. Due → sent is the input thread's own
    /// wake from its sleep, before the send and outside the latency.
    pub(super) due: Option<Instant>,
    /// When each [`Phase`] of its frame ended, in `Phase` order.
    pub(super) marks: [Option<Instant>; 4],
    /// `ShowRows` sent before its frame, and by the end of it.
    pub(super) asked_before: usize,
    pub(super) asked_after: usize,
    /// A `ShowRows` was still unanswered when it was handed: the history
    /// job thread was busy while this frame ran.
    pub(super) rows_out: bool,
    /// Time inside the terminal's `write` before its frame, and by its end.
    pub(super) wrote_before: Duration,
    pub(super) wrote_after: Duration,
}

/// Where `phase` sits in [`Handed::marks`].
fn slot(phase: Phase) -> usize {
    match phase {
        Phase::Drained => 0,
        Phase::Updated => 1,
        Phase::CarriedOut => 2,
        Phase::Drawn => 3,
    }
}

/// Wakes for [`pump`]: one channel carrying the script's keys, sent by the
/// [`KeyFeed`] thread on a 60 Hz clock, and job events as they arrive, read
/// the way `ChannelWakes` reads the app's (`event_loop/mod.rs`: `recv` to
/// wait, `try_recv` to look). Every call means the loop has drawn the frame
/// for the wake before it, so each call is stamped.
pub(super) struct Paced<'a, D> {
    pub(super) events: &'a Receiver<Posted>,
    pub(super) keys: Vec<(KeyCode, Duration)>,
    /// Keys received so far: the index of the next one.
    pub(super) next_key: usize,
    /// When the input thread sent each key.
    pub(super) sent: SentLog,
    pub(super) watching: &'a Watching<'a, D>,
    pub(super) handed: Vec<Handed>,
    /// `replay[i]`: the event `handed[i]` carried, to replay through `update`.
    pub(super) replay: Vec<AppEvent>,
    /// `drawn[i]`: the frame after `handed[i - 1]` was on the terminal
    /// (`drawn[0]` is the first frame, before any wake).
    pub(super) drawn: Vec<Instant>,
    /// `written[i]`: bytes the terminal had been sent when `drawn[i]` was
    /// stamped; parsed into [`Counts`] after the scroll ([`Sink::count`]).
    pub(super) written: Vec<usize>,
    pub(super) sink: Sink,
    pub(super) failure: Option<String>,
    /// A wake `waiting` took off the channel to see it; handed out first.
    pub(super) seen: Option<Posted>,
    /// When the loop first looked for a wake since the last one was handed:
    /// its frame was on the terminal then, before any idle prefetch began.
    pub(super) looked: Option<Instant>,
    /// Per look: from the loop's first look to its `next_wake` — the idle
    /// prefetch after that frame.
    pub(super) prefetch: Vec<Duration>,
    /// Control C: say a wake is waiting before any strip is readied.
    pub(super) no_prefetch: bool,
    /// The newest window of rows handed to the loop.
    pub(super) answered: Option<u64>,
}

impl Handed {
    pub(super) fn kind_name(&self) -> String {
        match self.kind {
            Kind::Key(key, _) => format!("key {key}"),
            Kind::Window(request) => format!("window {request}"),
            Kind::Other => "other event".to_owned(),
        }
    }
}

impl<'a, D> Paced<'a, D> {
    pub(super) fn new(
        events: &'a Receiver<Posted>,
        keys: Vec<(KeyCode, Duration)>,
        watching: &'a Watching<'a, D>,
        sink: Sink,
        sent: SentLog,
    ) -> Self {
        Self {
            events,
            keys,
            next_key: 0,
            sent,
            watching,
            handed: Vec::new(),
            replay: Vec::new(),
            drawn: Vec::new(),
            written: Vec::new(),
            sink,
            failure: None,
            seen: None,
            looked: None,
            prefetch: Vec::new(),
            no_prefetch: false,
            answered: None,
        }
    }

    /// Stamp the frame just drawn, once: at the loop's first look for a wake.
    pub(super) fn stamp(&mut self) {
        if self.looked.is_none() {
            self.looked = Some(Instant::now());
            self.drawn.push(Instant::now());
            self.written.push(self.sink.sent());
        }
    }

    pub(super) fn hand(
        &mut self,
        kind: Kind,
        due: Option<Instant>,
        sent: Instant,
        event: AppEvent,
    ) -> Wake {
        let (asked, newest) = self.watching.asked_so_far();
        if let Kind::Window(id) = kind {
            self.answered = self.answered.max(Some(id));
        }
        let wake = Wake::Event(event);
        self.handed.push(Handed {
            kind,
            label: wake.label(),
            sent,
            at: Instant::now(),
            due,
            marks: [None; 4],
            asked_before: asked,
            asked_after: asked,
            rows_out: newest > self.answered,
            wrote_before: self.sink.write_spent(),
            wrote_after: Duration::ZERO,
        });
        if let Wake::Event(event) = &wake {
            self.replay.push(event.clone());
        }
        wake
    }

    pub(super) fn event(&mut self, event: AppEvent, sent: Instant) -> Wake {
        let kind = match &event {
            AppEvent::Progress(_, Progress::History(HistoryUpdate::Window(window))) => {
                Kind::Window(window.request)
            }
            _ => Kind::Other,
        };
        self.hand(kind, None, sent, event)
    }

    /// Hands `wake` to the loop: the next script key, or a job event.
    pub(super) fn take(&mut self, posted: Posted) -> Option<Wake> {
        let event = match posted.wake {
            Wake::Event(AppEvent::Core(Event::Key(press))) => press,
            Wake::Event(event) => return Some(self.event(event, posted.at)),
            other => {
                self.failure = Some(format!("a wake the harness never sends: {other:?}"));
                return None;
            }
        };
        let key = self.next_key;
        let (Some((code, _)), Some(sent)) = (self.keys.get(key), self.sent.get(key)) else {
            self.failure = Some(format!("key {key} arrived with no script entry or stamp"));
            return None;
        };
        if event != KeyPress::plain(*code) {
            self.failure = Some(format!("key {key} arrived as {event:?}, not {code:?}"));
            return None;
        }
        self.next_key += 1;
        let press = AppEvent::Core(Event::Key(event));
        Some(self.hand(Kind::Key(key, sent.at), Some(sent.due), sent.at, press))
    }
}

impl<D> Wakes for Paced<'_, D> {
    fn next_wake(&mut self) -> Option<Wake> {
        self.stamp();
        if let Some(looked) = self.looked.take() {
            self.prefetch.push(looked.elapsed());
        }
        if let Some(wake) = self.seen.take() {
            return self.take(wake);
        }
        if self.next_key < self.keys.len() {
            // As the app's loop waits: blocked in `recv` until a job or
            // the input thread sends.
            return match self.events.recv() {
                Ok(wake) => self.take(wake),
                Err(_hung_up) => {
                    self.failure =
                        Some("the job pool and the input thread hung up mid-scroll".to_owned());
                    None
                }
            };
        }
        // The script is over: wait only for rows still out.
        let newest_asked = self.watching.asked().iter().max().copied();
        let newest_answered = self
            .handed
            .iter()
            .filter_map(|handed| match handed.kind {
                Kind::Window(id) => Some(id),
                _ => None,
            })
            .max();
        if newest_asked.is_none() || newest_answered >= newest_asked {
            return None;
        }
        match self.events.recv_timeout(PATIENCE) {
            Ok(wake) => self.take(wake),
            Err(why) => {
                self.failure = Some(format!("rows still out after {PATIENCE:?}: {why}"));
                None
            }
        }
    }

    /// A key or an event has come: the loop stops readying strips, so a
    /// key's latency counts the one strip it waited behind.
    fn waiting(&mut self) -> bool {
        self.looked() || self.no_prefetch
    }

    /// As `ChannelWakes` answers. Asked before the frame is drawn, so it
    /// looks without `stamp`ing: the stamp marks a frame already drawn.
    /// Empty or hung up both answer `false` here; `next_wake` records a
    /// hang-up, as it does after `looked`.
    fn job_event_waiting(&mut self) -> bool {
        if self.seen.is_none()
            && let Ok(wake) = self.events.try_recv()
        {
            self.seen = Some(wake);
        }
        self.seen
            .as_ref()
            .is_some_and(|posted| is_job_event(&posted.wake))
    }

    fn mark(&mut self, phase: Phase) {
        let now = Instant::now();
        let after = (phase == Phase::Drawn)
            .then(|| (self.watching.asked_so_far().0, self.sink.write_spent()));
        if let Some(handed) = self.handed.last_mut() {
            handed.marks[slot(phase)] = Some(now);
            if let Some((asked, wrote)) = after {
                handed.asked_after = asked;
                handed.wrote_after = wrote;
            }
        }
    }
}

impl<D> Paced<'_, D> {
    pub(super) fn looked(&mut self) -> bool {
        self.stamp();
        if self.seen.is_some() {
            return true;
        }
        // The script is over: nothing left to measure, so nothing to ready.
        if self.next_key >= self.keys.len() {
            return true;
        }
        // As `ChannelWakes::waiting` looks (event_loop/mod.rs).
        match self.events.try_recv() {
            Ok(wake) => {
                self.seen = Some(wake);
                true
            }
            Err(TryRecvError::Empty) => false,
            // `next_wake` records the hang-up.
            Err(TryRecvError::Disconnected) => true,
        }
    }
}
