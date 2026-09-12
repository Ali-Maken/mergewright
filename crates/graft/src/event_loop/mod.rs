//! The loop that turns a terminal into an application.
//!
//! ```text
//! Event ──▶ update(state) ──▶ (new state, [Command]) ──▶ view(state)
//! ```
//!
//! Three things can wake it, and they all arrive down one channel as a
//! [`Wake`]: the user did something, a signal said stop, or reading the
//! terminal failed. Between wakes the loop blocks — an idle Graft uses no CPU,
//! because nothing polls and nothing ticks.
//!
//! [`run`] is the whole application: it sets up the terminal, installs the
//! panic hook, starts the input and signal threads, and hands control to
//! [`pump`], which is the part that can be tested without a terminal at all.

pub mod input;
pub mod signals;

use std::io::{self, Write};
use std::sync::mpsc::{self, Receiver, Sender};

use ratatui::Terminal;
use ratatui::backend::{Backend, CrosstermBackend};

use crate::crash;
use crate::terminal::{SessionOptions, TerminalSession};
use crate::update::{AppState, update};
use crate::view::view;

/// Begin a synchronized update: the terminal buffers what follows and paints
/// it in one go (DEC 2026, DESIGN.md §4.1).
const BEGIN_SYNCHRONIZED_UPDATE: &[u8] = b"\x1b[?2026h";

/// End a synchronized update, which is what makes the frame appear.
const END_SYNCHRONIZED_UPDATE: &[u8] = b"\x1b[?2026l";

/// Why the loop stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exit {
    /// The user asked to leave.
    Quit,
    /// This signal arrived. The caller re-raises it once the terminal is back,
    /// so the process dies the way it was told to.
    Signal(i32),
}

/// Something that wakes the loop.
#[derive(Debug)]
pub enum Wake {
    /// The user did something.
    Event(graft_core::event::Event),
    /// A signal arrived and Graft should stop. Carries the signal number so
    /// the exit status can be honest about which one it was.
    Shutdown(i32),
    /// Reading the terminal failed. Not recoverable — there is no input any
    /// more — and not ignorable, so it wakes the loop to be reported.
    InputFailed(io::Error),
}

/// Where the loop gets its wakes.
///
/// A seam, so [`pump`] can be driven from a test with a scripted list of wakes
/// instead of a terminal, a signal and two threads.
pub trait Wakes {
    /// Block until something happens. `None` once nothing can happen again,
    /// because every producer has gone.
    fn next_wake(&mut self) -> Option<Wake>;
}

/// The real source: one channel fed by the input thread and the signal thread.
pub struct ChannelWakes {
    receiver: Receiver<Wake>,
}

impl ChannelWakes {
    /// A channel, and the sender its producers clone.
    #[must_use]
    pub fn new() -> (Self, Sender<Wake>) {
        let (sender, receiver) = mpsc::channel();
        (Self { receiver }, sender)
    }
}

impl Wakes for ChannelWakes {
    fn next_wake(&mut self) -> Option<Wake> {
        // The only error `recv` can give is "all senders dropped", which is
        // not a failure: it means both threads have ended and no wake can ever
        // arrive again. That is exactly what `None` says.
        self.receiver.recv().ok()
    }
}

/// Run Graft until the user quits or a signal says stop.
///
/// The terminal is set up before the loop and restored after it, in that
/// order, whatever happens in between — including a panic, which the hook
/// handles first (`crate::crash`).
pub fn run(options: SessionOptions) -> io::Result<Exit> {
    let session = TerminalSession::on_stdout(options)?;
    crash::install(session.restorer());

    let (mut wakes, sender) = ChannelWakes::new();
    let signals = signals::watch(sender.clone())?;
    input::spawn_reader(sender)?;

    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let outcome = pump(&mut terminal, &mut wakes, AppState::default());
    // Dropped before the restore so that its own teardown (it shows the cursor
    // it hid) goes out while the session is still in charge of the terminal.
    drop(terminal);

    // Let the signal thread end. The input thread cannot be stopped — it is
    // blocked inside a terminal read that only returns when the user types —
    // so it is left to die with the process. It holds nothing that matters.
    signals.close();

    // Restore explicitly rather than leaving it to `Drop`, so a restore
    // failure can be returned instead of merely printed.
    let restored = session.restore();
    match (outcome, restored) {
        (Ok(exit), Ok(())) => Ok(exit),
        (Ok(_), Err(restore_error)) => Err(restore_error),
        (Err(loop_error), Ok(())) => Err(loop_error),
        (Err(loop_error), Err(restore_error)) => {
            // Both went wrong. The loop's failure is the one worth returning,
            // but a terminal left in raw mode must never be silent.
            let mut stderr = io::stderr();
            writeln!(
                stderr,
                "graft: could not restore the terminal: {restore_error}"
            )
            .and_then(|()| stderr.flush())
            // Nothing sensible is left to do if even stderr is gone.
            .unwrap_or(());
            Err(loop_error)
        }
    }
}

/// The loop itself: draw, wait, update, repeat.
///
/// Split out from [`run`] so it can be driven over any backend and any source
/// of wakes — which is what lets the frame bytes be asserted in a test.
pub fn pump<B, W>(
    terminal: &mut Terminal<B>,
    wakes: &mut W,
    mut state: AppState,
) -> io::Result<Exit>
where
    B: Backend<Error = io::Error> + Write,
    W: Wakes,
{
    // Draw before waiting: the first frame must be on screen while Graft sits
    // idle waiting for the user's first key.
    draw(terminal, &state)?;

    loop {
        let Some(wake) = wakes.next_wake() else {
            // Nothing can wake the loop again. Leave as if the user had quit
            // rather than spinning on a dead channel.
            return Ok(Exit::Quit);
        };

        let event = match wake {
            Wake::Event(event) => event,
            Wake::Shutdown(signal) => return Ok(Exit::Signal(signal)),
            Wake::InputFailed(error) => return Err(error),
        };

        let (next, commands) = update(&state, event);
        state = next;

        if let Some(command) = commands.into_iter().next() {
            // `Command` has no variants yet, so this arm is unreachable and the
            // compiler knows it. The moment ticket 10 adds the first variant
            // this stops compiling — which is the point: a command can never be
            // quietly dropped.
            match command {}
        }

        if state.should_quit {
            return Ok(Exit::Quit);
        }

        draw(terminal, &state)?;
    }
}

/// Draw one frame, wrapped in a synchronized update.
///
/// The begin/end pair goes out unconditionally (DESIGN.md §4.1): a terminal
/// that does not understand DEC 2026 ignores the sequence, and probing for it
/// would cost a round-trip on every frame to learn something that changes
/// nothing.
fn draw<B>(terminal: &mut Terminal<B>, state: &AppState) -> io::Result<()>
where
    B: Backend<Error = io::Error> + Write,
{
    terminal
        .backend_mut()
        .write_all(BEGIN_SYNCHRONIZED_UPDATE)?;

    let drawn = terminal.draw(|frame| view(state, frame)).map(|_| ());

    // The end marker goes out even when the draw failed. Leaving the terminal
    // inside a synchronized update would freeze the user's screen on whatever
    // was there before, with no way to tell that anything is wrong.
    let ended = terminal
        .backend_mut()
        .write_all(END_SYNCHRONIZED_UPDATE)
        .and_then(|()| Write::flush(terminal.backend_mut()));

    drawn.and(ended)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use graft_core::event::{Event, KeyCode, KeyPress};
    use ratatui::layout::Rect;
    use ratatui::{TerminalOptions, Viewport};

    use super::*;

    /// A terminal's output stream, kept in memory and readable while the
    /// `Terminal` still owns the backend.
    #[derive(Clone, Default)]
    struct Recorder(Arc<Mutex<Vec<u8>>>);

    impl Recorder {
        fn written(&self) -> String {
            let bytes = self.0.lock().expect("no test panics while holding this");
            String::from_utf8_lossy(&bytes).into_owned()
        }
    }

    impl Write for Recorder {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let mut bytes = self.0.lock().expect("no test panics while holding this");
            bytes.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    /// A scripted source of wakes: hands out what it was given, then reports
    /// that its producers are gone.
    struct ScriptedWakes {
        remaining: std::vec::IntoIter<Wake>,
    }

    impl ScriptedWakes {
        fn new(wakes: Vec<Wake>) -> Self {
            Self {
                remaining: wakes.into_iter(),
            }
        }
    }

    impl Wakes for ScriptedWakes {
        fn next_wake(&mut self) -> Option<Wake> {
            self.remaining.next()
        }
    }

    /// A terminal that writes its bytes into memory, plus the handle that
    /// reads them back. `with_options` and a fixed viewport are what let this
    /// exist with no tty to ask for a size.
    fn terminal(columns: u16, rows: u16) -> (Terminal<CrosstermBackend<Recorder>>, Recorder) {
        let recorder = Recorder::default();
        let terminal = Terminal::with_options(
            CrosstermBackend::new(recorder.clone()),
            TerminalOptions {
                viewport: Viewport::Fixed(Rect::new(0, 0, columns, rows)),
            },
        )
        .expect("a fixed viewport never asks the backend for its size");
        (terminal, recorder)
    }

    fn quit_key() -> Wake {
        Wake::Event(Event::Key(KeyPress::plain(KeyCode::Char('q'))))
    }

    #[test]
    fn the_quit_key_ends_the_loop() {
        let (mut terminal, _recorder) = terminal(20, 3);
        let mut wakes = ScriptedWakes::new(vec![quit_key()]);

        let exit = pump(&mut terminal, &mut wakes, AppState::default())
            .expect("writing to memory cannot fail");

        assert_eq!(exit, Exit::Quit);
    }

    #[test]
    fn a_signal_ends_the_loop_and_says_which_one() {
        let (mut terminal, _recorder) = terminal(20, 3);
        let mut wakes = ScriptedWakes::new(vec![Wake::Shutdown(signals::WATCHED[1])]);

        let exit = pump(&mut terminal, &mut wakes, AppState::default())
            .expect("writing to memory cannot fail");

        assert_eq!(exit, Exit::Signal(signals::WATCHED[1]));
    }

    #[test]
    fn a_signal_wins_over_a_key_that_arrived_first() {
        let (mut terminal, _recorder) = terminal(20, 3);
        let mut wakes = ScriptedWakes::new(vec![
            Wake::Event(Event::Resize {
                columns: 20,
                rows: 3,
            }),
            Wake::Shutdown(signals::WATCHED[0]),
            quit_key(),
        ]);

        let exit = pump(&mut terminal, &mut wakes, AppState::default())
            .expect("writing to memory cannot fail");

        assert_eq!(exit, Exit::Signal(signals::WATCHED[0]));
    }

    #[test]
    fn a_failed_read_stops_the_loop_and_is_reported() {
        let (mut terminal, _recorder) = terminal(20, 3);
        let failure = io::Error::new(io::ErrorKind::BrokenPipe, "the terminal went away");
        let mut wakes = ScriptedWakes::new(vec![Wake::InputFailed(failure)]);

        let error = pump(&mut terminal, &mut wakes, AppState::default())
            .expect_err("a failed read must not be swallowed");

        assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
    }

    #[test]
    fn losing_every_producer_ends_the_loop_rather_than_spinning() {
        let (mut terminal, _recorder) = terminal(20, 3);
        let mut wakes = ScriptedWakes::new(Vec::new());

        let exit = pump(&mut terminal, &mut wakes, AppState::default())
            .expect("writing to memory cannot fail");

        assert_eq!(exit, Exit::Quit);
    }

    #[test]
    fn every_frame_is_wrapped_in_a_synchronized_update() {
        let (mut terminal, recorder) = terminal(20, 3);
        // One frame before the wait, one after the resize, and none after the
        // quit — three wakes, two frames.
        let mut wakes = ScriptedWakes::new(vec![
            Wake::Event(Event::Resize {
                columns: 20,
                rows: 3,
            }),
            quit_key(),
        ]);

        pump(&mut terminal, &mut wakes, AppState::default())
            .expect("writing to memory cannot fail");

        let bytes = recorder.written();
        assert_eq!(bytes.matches("\u{1b}[?2026h").count(), 2);
        assert_eq!(bytes.matches("\u{1b}[?2026l").count(), 2);
        assert!(
            bytes.starts_with("\u{1b}[?2026h"),
            "the first bytes out must open a synchronized update: {bytes:?}"
        );
        assert!(
            bytes.ends_with("\u{1b}[?2026l"),
            "the last bytes out must close it: {bytes:?}"
        );
    }

    #[test]
    fn the_placeholder_reaches_the_terminal() {
        let (mut terminal, recorder) = terminal(30, 3);
        let mut wakes = ScriptedWakes::new(vec![quit_key()]);

        pump(&mut terminal, &mut wakes, AppState::default())
            .expect("writing to memory cannot fail");

        // The backend skips blank cells and moves the cursor instead, so the
        // line arrives as words rather than as one string.
        let bytes = recorder.written();
        for word in ["Graft", "press", "quit", "ready"] {
            assert!(bytes.contains(word), "{word:?} never reached the terminal");
        }
    }

    #[test]
    fn a_terminal_with_no_cells_draws_without_panicking() {
        // ADR-0012's degenerate pty, driven through the whole loop rather than
        // just through `view`.
        let (mut terminal, _recorder) = terminal(0, 0);
        let mut wakes = ScriptedWakes::new(vec![
            Wake::Event(Event::Resize {
                columns: 0,
                rows: 0,
            }),
            quit_key(),
        ]);

        let exit = pump(&mut terminal, &mut wakes, AppState::default())
            .expect("writing to memory cannot fail");

        assert_eq!(exit, Exit::Quit);
    }

    #[test]
    fn a_channel_with_no_senders_left_reports_that_it_is_finished() {
        let (mut wakes, sender) = ChannelWakes::new();
        drop(sender);

        assert!(wakes.next_wake().is_none());
    }
}
