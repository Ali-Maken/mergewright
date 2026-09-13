//! Re-asking the terminal while Graft runs (DESIGN.md §4.5).
//!
//! A resize re-measures the cell size; focus regain re-reads the background.
//! Both wait for a reply, and the UI thread never waits for anything, so the
//! asking happens on a worker of its own: the resize and input threads call
//! [`Redetector::ask`], the worker writes the query and reads the reply, and
//! anything that actually changed comes back to the loop as a
//! [`Wake::Terminal`] — the event the pixel renderer takes its cell size from.
//!
//! One rule keeps the query bytes from corrupting a frame: **every write to
//! stdout that must stay whole holds [`hold_output`]**. crossterm writes a
//! single escape sequence in several pieces, so a query written between two
//! of them would land inside the sequence. The loop holds the lock for a whole
//! frame; the worker holds it only while it writes, never while it waits.

use std::io;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use graft_core::capabilities::{CellSize, ThemeHint};
use graft_term::detect::{self, TerminalHost, WindowPixels};

use super::Wake;
use crate::session::TerminalChange;

/// Guards the order of bytes on stdout, not any data.
static OUTPUT: Mutex<()> = Mutex::new(());

/// Hold stdout for one whole write: a frame, or a query.
pub(crate) fn hold_output() -> MutexGuard<'static, ()> {
    // Nothing is behind the lock but ordering, so a thread that panicked while
    // holding it left no half-updated value to protect.
    OUTPUT.lock().unwrap_or_else(PoisonError::into_inner)
}

/// What to re-ask.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Query {
    /// `CSI 16 t`, after a resize.
    CellSize,
    /// `OSC 11`, after focus regain.
    Theme,
}

/// The handle producers use to ask; cheap to clone, never blocks.
#[derive(Clone, Debug)]
pub struct Redetector {
    requests: Sender<Query>,
}

impl Redetector {
    /// Start the worker, knowing the values detection settled on at startup.
    /// `budget` is the same deadline the startup probe had (DESIGN.md §4.4).
    pub fn spawn<H>(
        host: H,
        cell_size: CellSize,
        theme: ThemeHint,
        budget: Duration,
        wakes: Sender<Wake>,
    ) -> io::Result<Self>
    where
        H: TerminalHost + Send + 'static,
    {
        let (requests, receiver) = mpsc::channel();
        let mut worker = Worker {
            host: LockedOutput(host),
            cell_size,
            theme,
            budget,
            wakes,
        };
        thread::Builder::new()
            .name("graft-redetect".to_owned())
            .spawn(move || worker.serve(&receiver))?;
        Ok(Self { requests })
    }

    /// Queue a re-query. Returns at once.
    pub fn ask(&self, query: Query) {
        // Fails only once the worker has stopped, and it stops only when the
        // loop has gone or the terminal failed (already reported as a wake):
        // either way there is nobody left to answer.
        self.requests.send(query).unwrap_or(());
    }
}

/// The worker: the values in force, and what it needs to re-ask them.
struct Worker<H> {
    host: LockedOutput<H>,
    cell_size: CellSize,
    theme: ThemeHint,
    budget: Duration,
    wakes: Sender<Wake>,
}

impl<H: TerminalHost> Worker<H> {
    fn serve(&mut self, requests: &Receiver<Query>) {
        while let Ok(first) = requests.recv() {
            // Dragging a window edge sends a burst of resizes. Everything
            // queued by now is answered by one round per kind.
            let mut cell_size = first == Query::CellSize;
            let mut theme = first == Query::Theme;
            for query in requests.try_iter() {
                match query {
                    Query::CellSize => cell_size = true,
                    Query::Theme => theme = true,
                }
            }
            if cell_size && !self.remeasure() {
                return;
            }
            if theme && !self.retheme() {
                return;
            }
        }
    }

    /// Re-measure; `false` when the worker should stop.
    fn remeasure(&mut self) -> bool {
        match detect::remeasure_cell_size(&mut self.host, self.cell_size, self.budget) {
            Ok(size) if size == self.cell_size => true,
            Ok(size) => {
                self.cell_size = size;
                self.send(Wake::Terminal(TerminalChange::CellSize(size)))
            }
            Err(error) => {
                self.send(Wake::TerminalFailed(error));
                false
            }
        }
    }

    /// Re-read the background; `false` when the worker should stop.
    fn retheme(&mut self) -> bool {
        match detect::requery_theme(&mut self.host, self.theme, self.budget) {
            Ok(hint) if hint == self.theme => true,
            Ok(hint) => {
                self.theme = hint;
                self.send(Wake::Terminal(TerminalChange::Theme(hint)))
            }
            Err(error) => {
                self.send(Wake::TerminalFailed(error));
                false
            }
        }
    }

    /// `false` once the loop has gone.
    fn send(&self, wake: Wake) -> bool {
        self.wakes.send(wake).is_ok()
    }
}

/// A host whose query writes wait for, and hold, [`hold_output`].
struct LockedOutput<H>(H);

impl<H: TerminalHost> TerminalHost for LockedOutput<H> {
    fn write_probe(&mut self, bytes: &[u8]) -> io::Result<()> {
        let _whole = hold_output();
        self.0.write_probe(bytes)
    }

    fn read(&mut self, buffer: &mut [u8], deadline: Instant) -> io::Result<usize> {
        self.0.read(buffer, deadline)
    }

    fn window_pixels(&mut self) -> Option<WindowPixels> {
        self.0.window_pixels()
    }

    fn tmux_version(&mut self) -> Option<String> {
        self.0.tmux_version()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use graft_core::capabilities::{CellSizeSource, Rgb, Theme, ThemeSource};

    use super::*;

    /// Long enough never to fire on a healthy machine; the fake answers at
    /// once, so a test only waits this long when it is already failing.
    const PATIENCE: Duration = Duration::from_secs(5);

    /// Replies one scripted answer per write; unscripted writes get silence.
    struct Scripted {
        answers: VecDeque<Vec<u8>>,
        pending: Vec<u8>,
        broken: bool,
    }

    impl Scripted {
        fn answering(answers: &[&[u8]]) -> Self {
            Self {
                answers: answers.iter().map(|answer| answer.to_vec()).collect(),
                pending: Vec::new(),
                broken: false,
            }
        }
    }

    impl TerminalHost for Scripted {
        fn write_probe(&mut self, _bytes: &[u8]) -> io::Result<()> {
            if self.broken {
                return Err(io::Error::other("scripted write failure"));
            }
            self.pending = self.answers.pop_front().unwrap_or_default();
            Ok(())
        }

        fn read(&mut self, buffer: &mut [u8], _deadline: Instant) -> io::Result<usize> {
            let taken = buffer.len().min(self.pending.len());
            buffer[..taken].copy_from_slice(&self.pending[..taken]);
            self.pending.drain(..taken);
            Ok(taken)
        }

        fn window_pixels(&mut self) -> Option<WindowPixels> {
            None
        }

        fn tmux_version(&mut self) -> Option<String> {
            None
        }
    }

    fn startup_size() -> CellSize {
        CellSize {
            width: 9,
            height: 18,
            source: CellSizeSource::Reported,
        }
    }

    fn startup_theme() -> ThemeHint {
        ThemeHint {
            theme: Theme::Dark,
            source: ThemeSource::Background(Rgb {
                red: 0,
                green: 0,
                blue: 0,
            }),
        }
    }

    fn spawn(host: Scripted) -> (Redetector, Receiver<Wake>) {
        let (sender, receiver) = mpsc::channel();
        let redetector = Redetector::spawn(
            host,
            startup_size(),
            startup_theme(),
            Duration::from_millis(200),
            sender,
        )
        .expect("a thread can be spawned");
        (redetector, receiver)
    }

    fn next(wakes: &Receiver<Wake>) -> Wake {
        wakes.recv_timeout(PATIENCE).expect("a wake arrives")
    }

    #[test]
    fn a_resize_that_is_answered_reports_the_new_size() {
        let (redetector, wakes) = spawn(Scripted::answering(&[b"\x1b[6;40;20t\x1b[?62;22c"]));

        redetector.ask(Query::CellSize);

        assert!(matches!(
            next(&wakes),
            Wake::Terminal(TerminalChange::CellSize(CellSize {
                width: 20,
                height: 40,
                source: CellSizeSource::Reported,
            }))
        ));
    }

    #[test]
    fn an_unchanged_answer_wakes_nobody() {
        // Round one: the silent resize degrades the size to assumed — a
        // change. Round two is silent again — no change, no wake — so the
        // next wake is the theme's.
        let (redetector, wakes) = spawn(Scripted::answering(&[
            b"",
            b"",
            b"\x1b]11;rgb:ffff/ffff/ffff\x1b\\\x1b[?62;22c",
        ]));

        redetector.ask(Query::CellSize);
        assert!(matches!(
            next(&wakes),
            Wake::Terminal(TerminalChange::CellSize(CellSize {
                width: 9,
                height: 18,
                source: CellSizeSource::Assumed,
            }))
        ));
        redetector.ask(Query::CellSize);
        redetector.ask(Query::Theme);

        assert!(matches!(
            next(&wakes),
            Wake::Terminal(TerminalChange::Theme(ThemeHint {
                theme: Theme::Light,
                ..
            }))
        ));
    }

    #[test]
    fn a_terminal_that_cannot_be_written_to_is_reported() {
        let mut host = Scripted::answering(&[]);
        host.broken = true;
        let (redetector, wakes) = spawn(host);

        redetector.ask(Query::Theme);

        assert!(matches!(next(&wakes), Wake::TerminalFailed(_)));
    }
}
