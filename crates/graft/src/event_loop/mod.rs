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
//! panic hook, asks the terminal what it can do, starts the input and signal
//! threads, and hands control to [`pump`], which is the part that can be
//! tested without a terminal at all.

pub mod input;
pub mod signals;

use std::io::{self, Write};
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};

use graft_core::capabilities::Supported;
use graft_term::detect::{self, TtyHost};
use ratatui::Terminal;
use ratatui::backend::{Backend, CrosstermBackend};

use crate::crash;
use crate::jobs::{self, Delivery, Dispatch, JobEvents, JobRunner, Shutdown};
use crate::session::{Session, Startup};
use crate::terminal::{SessionOptions, TerminalSession};
use crate::update::{AppEvent, AppState, Command, Quit, update};
use crate::view::view;

/// Begin a synchronized update: the terminal buffers what follows and paints
/// it in one go (DEC 2026, DESIGN.md §4.1).
const BEGIN_SYNCHRONIZED_UPDATE: &[u8] = b"\x1b[?2026h";

/// End a synchronized update, which is what makes the frame appear.
const END_SYNCHRONIZED_UPDATE: &[u8] = b"\x1b[?2026l";

/// Why the loop stopped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Exit {
    /// The user asked to leave.
    Quit,
    /// This signal arrived. The caller re-raises it once the terminal is back,
    /// so the process dies the way it was told to.
    Signal(i32),
    /// Graft could not go on, and this is the reason to print. Carried out of
    /// the loop rather than printed inside it: a message written into the
    /// alternate screen disappears with the alternate screen, which is how a
    /// program comes to "exit without saying anything".
    Fatal(String),
}

/// Something that wakes the loop.
#[derive(Debug)]
pub enum Wake {
    /// The user did something, or a job came back with something.
    Event(AppEvent),
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

/// A job's result is a wake like any other: the loop reads it from the same
/// channel as the keyboard, in the order it arrived.
impl JobEvents for Sender<Wake> {
    fn deliver(&self, event: AppEvent) -> Delivery {
        match self.send(Wake::Event(event)) {
            Ok(()) => Delivery::Received,
            Err(_returned) => Delivery::NobodyListening,
        }
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
pub fn run(startup: Startup) -> io::Result<Exit> {
    // Taken apart here: `run` consumes the startup facts, and the two halves
    // are used at opposite ends of the function.
    let Startup { repo_path, config } = startup;
    // Kitty keyboard is deliberately off here and pushed below instead:
    // whether the terminal speaks the protocol is something only the probe
    // knows, and the probe needs the raw mode this session turns on.
    let session = TerminalSession::on_stdout(SessionOptions::default())?;
    crash::install(session.restorer());

    // Before the input thread exists, and never at the same time as it: the
    // probe writes queries to the terminal and reads the replies off stdin,
    // and a reader running alongside it would eat them.
    let capabilities = detect::detect(&detect::Environment::from_process(), &mut TtyHost::new());
    if capabilities.kitty_keyboard == Supported::Yes {
        session.enable_kitty_keyboard()?;
    }
    let settled = Session::settle(&config, capabilities);

    let (mut wakes, sender) = ChannelWakes::new();
    let signals = signals::watch(sender.clone())?;
    let jobs = JobRunner::with_default_size(sender.clone())?;
    input::spawn_reader(sender)?;

    let state = opening_repository(&jobs, &repo_path);
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let outcome = pump(&mut terminal, &mut wakes, &jobs, &settled, state);
    // Dropped before the restore so that its own teardown (it shows the cursor
    // it hid) goes out while the session is still in charge of the terminal.
    drop(terminal);

    // Stop the pool before the terminal goes back, so that a job still writing
    // its result cannot outlive the screen it was writing for.
    let shutdown = jobs.shutdown(jobs::SHUTDOWN_GRACE);

    // Let the signal thread end. The input thread cannot be stopped — it is
    // blocked inside a terminal read that only returns when the user types —
    // so it is left to die with the process. It holds nothing that matters.
    signals.close();

    // Restore explicitly rather than leaving it to `Drop`, so a restore
    // failure can be returned instead of merely printed.
    let restored = session.restore();
    report_stalled_jobs(shutdown);
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

/// The state Graft starts in, with the first two jobs already on their way.
///
/// Both read the world — one opens the repository, one runs `git --version` —
/// so both are jobs: the first frame is drawn while they are still running,
/// and says so, rather than waiting on the disk.
fn opening_repository(jobs: &impl Dispatch, repo_path: &Path) -> AppState {
    jobs.dispatch(Command::OpenRepository(repo_path.to_path_buf()));
    jobs.dispatch(Command::CheckSystemGit);
    AppState {
        repo_path: repo_path.to_path_buf(),
        ..AppState::default()
    }
}

/// Say so when jobs had to be abandoned, now that the terminal is the user's
/// again.
///
/// Rare, and a bug when it happens: it means a job did not check its
/// cancellation token often enough. Printing it is the only way anyone finds
/// out, since by this point there is no screen left to put it on.
fn report_stalled_jobs(shutdown: Shutdown) {
    let Shutdown::Stalled { workers } = shutdown else {
        return;
    };

    let mut stderr = io::stderr();
    // Nothing sensible is left to do if even stderr is gone, and the exit is
    // otherwise a normal one.
    writeln!(
        stderr,
        "graft: {workers} background job(s) did not stop in time and were abandoned"
    )
    .and_then(|()| stderr.flush())
    .unwrap_or(());
}

/// The loop itself: draw, wait, update, repeat.
///
/// Split out from [`run`] so it can be driven over any backend, any source of
/// wakes and any job runner — which is what lets the frame bytes be asserted in
/// a test.
pub fn pump<B, W, D>(
    terminal: &mut Terminal<B>,
    wakes: &mut W,
    jobs: &D,
    session: &Session,
    mut state: AppState,
) -> io::Result<Exit>
where
    B: Backend<Error = io::Error> + Write,
    W: Wakes,
    D: Dispatch,
{
    // Draw before waiting: the first frame must be on screen while Graft sits
    // idle waiting for the user's first key.
    draw(terminal, session, &state)?;

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

        for command in commands {
            // Handed over and forgotten. `dispatch` starts the work and
            // returns: there is no handle here to join and no result to wait
            // for, because the result comes back as another `Event`
            // (ARCHITECTURE.md). This is the line that keeps the keyboard
            // responsive during a two-minute fetch.
            jobs.dispatch(command);
        }

        match &state.quit {
            Some(Quit::Requested) => return Ok(Exit::Quit),
            // Reported by the caller, once the terminal is the user's again.
            Some(Quit::Fatal(reason)) => return Ok(Exit::Fatal(reason.clone())),
            None => {}
        }

        draw(terminal, session, &state)?;
    }
}

/// Draw one frame, wrapped in a synchronized update.
///
/// The begin/end pair goes out unconditionally (DESIGN.md §4.1): a terminal
/// that does not understand DEC 2026 ignores the sequence, and probing for it
/// would cost a round-trip on every frame to learn something that changes
/// nothing.
fn draw<B>(terminal: &mut Terminal<B>, session: &Session, state: &AppState) -> io::Result<()>
where
    B: Backend<Error = io::Error> + Write,
{
    terminal
        .backend_mut()
        .write_all(BEGIN_SYNCHRONIZED_UPDATE)?;

    let drawn = terminal
        .draw(|frame| view(session, state, frame))
        .map(|_| ());

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
    use crate::session::for_tests::session;

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

    /// A job runner that runs nothing and remembers everything, so a test can
    /// see exactly what the loop handed over.
    #[derive(Default)]
    struct Recorded(Mutex<Vec<Command>>);

    impl Recorded {
        fn commands(&self) -> Vec<Command> {
            self.0
                .lock()
                .expect("no test panics while holding this")
                .clone()
        }
    }

    impl Dispatch for Recorded {
        fn dispatch(&self, command: Command) {
            self.0
                .lock()
                .expect("no test panics while holding this")
                .push(command);
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
        key(KeyPress::plain(KeyCode::Char('q')))
    }

    fn key(press: KeyPress) -> Wake {
        Wake::Event(AppEvent::Core(Event::Key(press)))
    }

    fn resized(columns: u16, rows: u16) -> Wake {
        Wake::Event(AppEvent::Core(Event::Resize { columns, rows }))
    }

    #[test]
    fn the_quit_key_ends_the_loop() {
        let (mut terminal, _recorder) = terminal(20, 3);
        let mut wakes = ScriptedWakes::new(vec![quit_key()]);

        let exit = pump(
            &mut terminal,
            &mut wakes,
            &Recorded::default(),
            &session(),
            AppState::default(),
        )
        .expect("writing to memory cannot fail");

        assert_eq!(exit, Exit::Quit);
    }

    #[test]
    fn a_signal_ends_the_loop_and_says_which_one() {
        let (mut terminal, _recorder) = terminal(20, 3);
        let mut wakes = ScriptedWakes::new(vec![Wake::Shutdown(signals::WATCHED[1])]);

        let exit = pump(
            &mut terminal,
            &mut wakes,
            &Recorded::default(),
            &session(),
            AppState::default(),
        )
        .expect("writing to memory cannot fail");

        assert_eq!(exit, Exit::Signal(signals::WATCHED[1]));
    }

    #[test]
    fn a_signal_wins_over_a_key_that_arrived_first() {
        let (mut terminal, _recorder) = terminal(20, 3);
        let mut wakes = ScriptedWakes::new(vec![
            resized(20, 3),
            Wake::Shutdown(signals::WATCHED[0]),
            quit_key(),
        ]);

        let exit = pump(
            &mut terminal,
            &mut wakes,
            &Recorded::default(),
            &session(),
            AppState::default(),
        )
        .expect("writing to memory cannot fail");

        assert_eq!(exit, Exit::Signal(signals::WATCHED[0]));
    }

    #[test]
    fn a_failed_read_stops_the_loop_and_is_reported() {
        let (mut terminal, _recorder) = terminal(20, 3);
        let failure = io::Error::new(io::ErrorKind::BrokenPipe, "the terminal went away");
        let mut wakes = ScriptedWakes::new(vec![Wake::InputFailed(failure)]);

        let error = pump(
            &mut terminal,
            &mut wakes,
            &Recorded::default(),
            &session(),
            AppState::default(),
        )
        .expect_err("a failed read must not be swallowed");

        assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
    }

    #[test]
    fn losing_every_producer_ends_the_loop_rather_than_spinning() {
        let (mut terminal, _recorder) = terminal(20, 3);
        let mut wakes = ScriptedWakes::new(Vec::new());

        let exit = pump(
            &mut terminal,
            &mut wakes,
            &Recorded::default(),
            &session(),
            AppState::default(),
        )
        .expect("writing to memory cannot fail");

        assert_eq!(exit, Exit::Quit);
    }

    #[test]
    fn every_frame_is_wrapped_in_a_synchronized_update() {
        let (mut terminal, recorder) = terminal(20, 3);
        // One frame before the wait, one after the resize, and none after the
        // quit — three wakes, two frames.
        let mut wakes = ScriptedWakes::new(vec![resized(20, 3), quit_key()]);

        pump(
            &mut terminal,
            &mut wakes,
            &Recorded::default(),
            &session(),
            AppState::default(),
        )
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
    fn the_status_screen_reaches_the_terminal() {
        let (mut terminal, recorder) = terminal(60, 12);
        let mut wakes = ScriptedWakes::new(vec![quit_key()]);

        pump(
            &mut terminal,
            &mut wakes,
            &Recorded::default(),
            &session(),
            AppState::default(),
        )
        .expect("writing to memory cannot fail");

        // The backend skips blank cells and moves the cursor instead, so the
        // screen arrives as words rather than as whole lines.
        let bytes = recorder.written();
        for word in ["Graft", "Repository", "opening", "Rendering", "quit"] {
            assert!(bytes.contains(word), "{word:?} never reached the terminal");
        }
    }

    #[test]
    fn a_terminal_with_no_cells_draws_without_panicking() {
        // ADR-0012's degenerate pty, driven through the whole loop rather than
        // just through `view`.
        let (mut terminal, _recorder) = terminal(0, 0);
        let mut wakes = ScriptedWakes::new(vec![resized(0, 0), quit_key()]);

        let exit = pump(
            &mut terminal,
            &mut wakes,
            &Recorded::default(),
            &session(),
            AppState::default(),
        )
        .expect("writing to memory cannot fail");

        assert_eq!(exit, Exit::Quit);
    }

    #[test]
    fn a_command_from_update_reaches_the_job_runner() {
        let (mut terminal, _recorder) = terminal(20, 3);
        let jobs = Recorded::default();
        let mut wakes =
            ScriptedWakes::new(vec![key(KeyPress::control(KeyCode::Char('r'))), quit_key()]);
        let state = AppState {
            repo_path: std::path::PathBuf::from("/code/api"),
            ..AppState::default()
        };

        pump(&mut terminal, &mut wakes, &jobs, &session(), state)
            .expect("writing to memory cannot fail");

        assert_eq!(
            jobs.commands(),
            vec![Command::OpenRepository(std::path::PathBuf::from(
                "/code/api"
            ))]
        );
    }

    #[test]
    fn a_job_result_is_just_another_wake() {
        let (mut terminal, recorder) = terminal(60, 14);
        let mut wakes = ScriptedWakes::new(vec![
            Wake::Event(AppEvent::Core(Event::JobFailed(
                graft_core::job::JobId::new(1),
                graft_core::job::JobFailure::Failed("not a git repository".to_owned()),
            ))),
            quit_key(),
        ]);

        pump(
            &mut terminal,
            &mut wakes,
            &Recorded::default(),
            &session(),
            AppState::default(),
        )
        .expect("writing to memory cannot fail");

        let bytes = recorder.written();
        for word in ["failed", "repository"] {
            assert!(bytes.contains(word), "{word:?} never reached the terminal");
        }
    }

    /// The rule the whole architecture rests on, checked rather than reviewed:
    /// nothing on the UI thread's path waits for a job. A result arrives as a
    /// wake or it does not arrive; the loop is never parked on one.
    #[test]
    fn the_ui_thread_never_waits_on_a_job() {
        let source = include_str!("mod.rs");
        let (code, _tests) = source
            .split_once("#[cfg(test)]")
            .expect("this module has a test section");

        for forbidden in [".join()", "recv_timeout", "wait_timeout", "thread::sleep"] {
            assert!(
                !code.contains(forbidden),
                "the loop must never wait on a job, but it mentions `{forbidden}`"
            );
        }
    }

    #[test]
    fn a_channel_with_no_senders_left_reports_that_it_is_finished() {
        let (mut wakes, sender) = ChannelWakes::new();
        drop(sender);

        assert!(wakes.next_wake().is_none());
    }
}
