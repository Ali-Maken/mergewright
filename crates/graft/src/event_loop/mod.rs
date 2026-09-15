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

mod graphics;
pub mod input;
pub mod redetect;
mod row_clear;
pub mod signals;

use std::io::{self, IsTerminal, Write};
use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};

use graft_core::capabilities::Supported;
use graft_core::event::{Event, KeyCode, KeyPress};
use graft_git::history::Order;
use graft_graph::raster::cache::Flush;
use graft_term::Transport;
use graft_term::detect::{self, TtyHost};
use graft_term::input::InputMux;
use ratatui::Terminal;
use ratatui::backend::{Backend, CrosstermBackend, TestBackend};

use crate::crash;
use crate::history::Phase;
use crate::jobs::{self, Delivery, Dispatch, JobEvents, JobRunner, Shutdown};
use crate::repos::RepoStore;
use crate::session::{Session, Startup, TerminalChange};
use crate::terminal::{SessionOptions, TerminalSession};
use crate::update::{AppEvent, AppState, Command, Quit, Size, update};
use crate::view::{draws_pixels, view, view_with_graphics};
use graphics::{Graphics, GraphicsReplies, NoReplies};
use redetect::Redetector;
use row_clear::RowClear;

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
    /// The user pressed one of `Harness::hand_back`'s keys; the caller
    /// asked for it back and acts on it now that the terminal is restored.
    HandedBack(graft_core::event::KeyCode),
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
    /// The terminal re-answered a question and the answer changed
    /// (DESIGN.md §4.5). Applied to the session, not to `update`: it changes
    /// what Graft draws with, not what it shows.
    Terminal(TerminalChange),
    /// Writing a re-query to the terminal, or reading its reply, failed. As
    /// unrecoverable as [`Wake::InputFailed`]: the next frame would fail too.
    TerminalFailed(io::Error),
}

/// Where the loop gets its wakes.
///
/// A seam, so [`pump`] can be driven from a test with a scripted list of wakes
/// instead of a terminal, a signal and two threads.
pub trait Wakes {
    /// Block until something happens. `None` once nothing can happen again,
    /// because every producer has gone.
    fn next_wake(&mut self) -> Option<Wake>;

    /// Is a wake already here? Never blocks. The loop asks between pieces of
    /// idle work (the strips a page key would show) and stops as soon as the
    /// answer is `true`; a source that has ended says `true` too, so the loop
    /// goes straight to finding that out.
    fn waiting(&mut self) -> bool;
}

/// The real source: one channel fed by the input thread and the signal thread.
pub struct ChannelWakes {
    receiver: Receiver<Wake>,
    /// A wake `waiting` took off the channel to see it; handed out first.
    seen: Option<Wake>,
}

impl ChannelWakes {
    /// A channel, and the sender its producers clone.
    #[must_use]
    pub fn new() -> (Self, Sender<Wake>) {
        let (sender, receiver) = mpsc::channel();
        (
            Self {
                receiver,
                seen: None,
            },
            sender,
        )
    }

    /// The terminal input the loop never got to, in order: what the input
    /// forwarder sent after the wake that ended the loop. Everything else
    /// still queued (job results, signals, terminal changes) is for a loop
    /// that has finished, and is dropped with it.
    ///
    /// Only complete once the forwarder has stopped (`run` joins it first).
    #[must_use]
    pub fn unhandled_input(self) -> Vec<Event> {
        self.seen
            .into_iter()
            .chain(self.receiver.try_iter())
            .filter_map(|wake| match wake {
                Wake::Event(AppEvent::Core(event)) => Some(event),
                Wake::Event(_)
                | Wake::Shutdown(_)
                | Wake::InputFailed(_)
                | Wake::Terminal(_)
                | Wake::TerminalFailed(_) => None,
            })
            .collect()
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
        if let Some(seen) = self.seen.take() {
            return Some(seen);
        }
        // The only error `recv` can give is "all senders dropped", which is
        // not a failure: it means both threads have ended and no wake can ever
        // arrive again. That is exactly what `None` says.
        self.receiver.recv().ok()
    }

    fn waiting(&mut self) -> bool {
        if self.seen.is_some() {
            return true;
        }
        match self.receiver.try_recv() {
            Ok(wake) => {
                self.seen = Some(wake);
                true
            }
            Err(TryRecvError::Empty) => false,
            // Every producer has gone: `next_wake` says so at once.
            Err(TryRecvError::Disconnected) => true,
        }
    }
}

/// Start the re-query worker (DESIGN.md §4.5), knowing what startup settled.
///
/// Only a terminal detection was allowed to ask may be asked again:
/// `TERM=dumb` gets no query bytes mid-session either, so it gets `None`.
fn start_redetector(
    environment: &detect::Environment,
    mux: &InputMux,
    settled: &Session,
    wakes: &Sender<Wake>,
) -> io::Result<Option<Redetector>> {
    if !environment.is_probeable() {
        return Ok(None);
    }
    Redetector::spawn(
        TtyHost::new(mux.waiter()),
        settled.capabilities.cell_size,
        settled.capabilities.theme,
        environment.probe_budget(),
        wakes.clone(),
    )
    .map(Some)
}

/// Run Graft until the user quits or a signal says stop.
///
/// The terminal is set up before the loop and restored after it, in that
/// order, whatever happens in between — including a panic, which the hook
/// handles first (`crate::crash`).
pub fn run(startup: Startup) -> io::Result<Exit> {
    // Taken apart here: `run` consumes the startup facts, and the two halves
    // are used at opposite ends of the function.
    let Startup {
        repo_path,
        config,
        harness,
    } = startup;
    // One repository, opened once and shared by every job that follows
    // (`crate::repos`). Opened *before* the terminal is touched: a path that
    // is not a repository is a plain line on a plain terminal, never an
    // alternate screen flashed for the length of the capability probe.
    // `graph.default_branch` is ADR-0001's rule 1, so it goes where the
    // branches are resolved.
    let repos = Arc::new(RepoStore::new(
        repo_path.clone(),
        config.config.graph.default_branch.clone(),
    ));
    if let Err(error) = repos.shared() {
        return Ok(Exit::Fatal(error.to_string()));
    }

    // A stdout that is not a tty — piped, redirected — never gets the
    // alternate screen or a single escape byte (ticket m1-the-graph/19): no
    // probe can tell the other end of a pipe apart from a terminal that
    // simply has not answered yet, so the only safe thing to draw there is
    // plain text (DESIGN.md §4.2 treats an unprobeable terminal as ASCII for
    // the same reason).
    if !io::stdout().is_terminal() {
        return run_headless(&repo_path, &config, repos);
    }

    // Kitty keyboard is deliberately off here and pushed below instead:
    // whether the terminal speaks the protocol is something only the probe
    // knows, and the probe needs the raw mode this session turns on.
    let session = TerminalSession::on_stdout(SessionOptions::default())?;
    crash::install(session.restorer());

    // The mux owns stdin for the process (ticket m1-the-graph/20); this run
    // gets its own `Inputs` from it, and a wrapper that runs again (the
    // gallery) gets the next one (ticket 35c). It starts before the probe,
    // so the probe's replies reach detection through a waiter while any key
    // typed meanwhile is queued, in order, for the loop.
    let (mux, inputs) = InputMux::on_stdin()?;
    let environment = detect::Environment::from_process();
    let capabilities = detect::detect(&environment, &mut TtyHost::new(mux.waiter()));
    if capabilities.kitty_keyboard == Supported::Yes {
        session.enable_kitty_keyboard()?;
    }
    let mut settled = Session::settle(&config, capabilities);

    let (mut wakes, sender) = ChannelWakes::new();
    let signals = signals::watch(sender.clone())?;
    let jobs = JobRunner::with_default_size(sender.clone(), repos)?;
    let redetector = start_redetector(&environment, &mux, &settled, &sender)?;
    let resizes = signals::watch_resizes(sender.clone(), redetector.clone())?;
    let forwarder = input::spawn_reader(inputs, sender, redetector)?;

    // Resizes clear row by row: a whole-screen clear frees Kitty image data
    // in Ghostty, and every resize would retransmit the viewport (ADR-0012).
    let mut terminal = Terminal::new(RowClear(CrosstermBackend::new(io::stdout())))?;
    // The first frame and the first row request are both sized from this. A
    // terminal sends no resize at startup, so without it the graph would ask
    // for, and draw, a single row until the window changed size.
    let size = terminal.size()?;
    let mut state = opening_repository(
        &jobs,
        &repo_path,
        crate::update::Size {
            columns: size.width,
            rows: size.height,
        },
    );
    // Its own waiter: every waiter gets a copy of each reply, so the
    // redetector's rounds and the images' error replies never steal from
    // each other (and each ignores the other's kind).
    state.settled_tier = Some(settled.rendering.tier);
    state.harness = harness;
    let mut graphics =
        Graphics::new(mux.waiter()).with_transport(Transport::for_tmux(&settled.capabilities.tmux));
    let outcome = pump_with(
        &mut terminal,
        &mut wakes,
        &jobs,
        &mut settled,
        state,
        &mut graphics,
    );
    let outcome = hand_back_input(&mux, forwarder, wakes, outcome);
    // Dropped before the restore so that its own teardown (it shows the cursor
    // it hid) goes out while the session is still in charge of the terminal.
    drop(terminal);

    // Stop the pool before the terminal goes back, so that a job still writing
    // its result cannot outlive the screen it was writing for.
    let shutdown = jobs.shutdown(jobs::SHUTDOWN_GRACE);

    // Let the signal threads end. The mux's reader is the process's, not
    // this run's (it is blocked waiting on the terminal until the user
    // types), so it is left to die with the process. It holds nothing that
    // matters.
    signals.close();
    resizes.close();

    // Restore explicitly rather than leaving it to `Drop`, so a restore
    // failure can be returned instead of merely printed.
    let restored = session.restore();
    report_stalled_jobs(shutdown);
    finish(outcome, restored)
}

/// This run is done reading (ticket m1-the-graph/35c). A key typed from here
/// on is kept for the next `on_stdin` (a wrapper's next run), and so is any
/// key already forwarded past the loop when it stopped: `]q` typed in one go
/// must step once and then quit, not step and lose the `q`. Joining the
/// forwarder first makes the hand-back complete — it ends once `hold` drops
/// its `Inputs` sender, after forwarding all it had.
fn hand_back_input(
    mux: &InputMux,
    forwarder: input::Forwarder,
    wakes: ChannelWakes,
    outcome: io::Result<Exit>,
) -> io::Result<Exit> {
    mux.hold();
    let outcome = outcome.and_then(|exit| forwarder.finish().map(|()| exit));
    mux.give_back(wakes.unhandled_input());
    outcome
}

/// The loop's outcome, unless putting the terminal back failed: a terminal
/// left in raw mode is never reported silently.
fn finish(outcome: io::Result<Exit>, restored: io::Result<()>) -> io::Result<Exit> {
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

/// Runs Graft to one finished frame with no terminal at all, for a stdout
/// that [`run`] found is not a tty (ticket m1-the-graph/19).
///
/// The same `update`/`Command`/job machinery as the interactive loop — jobs
/// are dispatched and awaited over the same channel `pump` uses — but there
/// is nothing to draw between wakes: nobody is watching a screen that
/// blinks, so the loop simply runs the walk to its end and prints one plain
/// frame, in the tier `Session::settle` chose (ASCII, since detection never
/// probes a terminal that cannot be probeable — `graft_term::detect`).
fn run_headless(
    repo_path: &Path,
    config: &crate::config::Loaded,
    repos: Arc<RepoStore>,
) -> io::Result<Exit> {
    // Never probed (stdout is not a terminal), so no mux is started to read
    // a stdin that may belong to the user's shell.
    let capabilities = detect::detect(&detect::Environment::from_process(), &mut TtyHost::unread());
    let session = Session::settle(config, capabilities);

    let (mut wakes, sender) = ChannelWakes::new();
    let jobs = JobRunner::with_default_size(sender, repos)?;

    // No real terminal to size against; wide and tall enough that the
    // fixtures this ships with are not truncated mid-row.
    let size = Size {
        columns: 200,
        rows: 200,
    };
    let mut state = opening_repository(&jobs, repo_path, size);

    while !headless_settled(&state) {
        let Some(wake) = wakes.next_wake() else {
            break;
        };
        let event = match wake {
            Wake::Event(event) => event,
            Wake::Shutdown(signal) => return Ok(Exit::Signal(signal)),
            Wake::InputFailed(error) | Wake::TerminalFailed(error) => return Err(error),
            // Headless runs start no redetector, so nothing sends this; and
            // plain text has no cell size or theme to change if it did.
            Wake::Terminal(_) => continue,
        };

        let (next, commands) = update(&state, event);
        state = next;
        for command in commands {
            jobs.dispatch(command);
        }
        if let Some(Quit::Fatal(reason)) = &state.quit {
            return Ok(Exit::Fatal(reason.clone()));
        }
    }

    print_headless_frame(&session, &state, size)?;
    Ok(Exit::Quit)
}

/// Whether the headless loop has seen enough to print: history exists and
/// its walk has left `Streaming`, one way or another.
///
/// `None` (before `Task::ReadHistory`'s `Started` event has been handled) is
/// not settled — counting it as settled printed an "opening…" frame with no
/// rows before the repository ever opened (M1 review, M4). This cannot spin
/// forever: a repository that fails to open sets `Quit::Fatal` before this is
/// checked again, and `run_headless`'s wake loop itself ends (breaking out
/// rather than looping) once its wake source is exhausted.
fn headless_settled(state: &AppState) -> bool {
    state
        .history
        .as_ref()
        .is_some_and(|history| !matches!(history.phase, Phase::Streaming))
}

/// Renders one frame into a backend nothing writes escape bytes to, and
/// prints it as plain text: exactly the picture a real terminal would show,
/// with none of the bytes that only make sense to one.
fn print_headless_frame(session: &Session, state: &AppState, size: Size) -> io::Result<()> {
    let mut terminal = Terminal::new(TestBackend::new(size.columns, size.rows))
        .unwrap_or_else(|error: std::convert::Infallible| match error {});
    terminal
        .draw(|frame| view(session, state, frame))
        .unwrap_or_else(|error: std::convert::Infallible| match error {});

    let buffer = terminal.backend().buffer();
    let mut text = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            text.push_str(buffer[(x, y)].symbol());
        }
        text.push('\n');
    }

    let mut stdout = io::stdout().lock();
    stdout.write_all(text.as_bytes())?;
    stdout.flush()
}

/// The state Graft starts in, with the first two jobs already on their way.
///
/// Both read the world — one opens the repository, one runs `git --version` —
/// so both are jobs: the first frame is drawn while they are still running,
/// and says so, rather than waiting on the disk.
fn opening_repository(
    jobs: &impl Dispatch,
    repo_path: &Path,
    size: crate::update::Size,
) -> AppState {
    jobs.dispatch(Command::OpenRepository);
    jobs.dispatch(Command::CheckSystemGit);
    // The third is the one with work in it: reading history. Asked for in
    // topological order, which ADR-0008 downgrades to commit date when the
    // repository has no commit-graph file.
    jobs.dispatch(Command::ReadHistory(Order::Topological));
    // Badges: ahead/behind walks commits, so it is a job too.
    jobs.dispatch(Command::ReadRefs);
    // The uncommitted-changes row: a status read, so it is a job too.
    jobs.dispatch(Command::ReadWorkingTree);
    AppState {
        repo_path: repo_path.to_path_buf(),
        size,
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
    session: &mut Session,
    state: AppState,
) -> io::Result<Exit>
where
    B: Backend<Error = io::Error> + Write,
    W: Wakes,
    D: Dispatch,
{
    pump_with(
        terminal,
        wakes,
        jobs,
        session,
        state,
        &mut Graphics::new(NoReplies),
    )
}

/// [`pump`], with the pixel tier's images and the replies about them; every
/// upload is freed however the loop ends.
fn pump_with<B, W, D, R>(
    terminal: &mut Terminal<B>,
    wakes: &mut W,
    jobs: &D,
    session: &mut Session,
    state: AppState,
    graphics: &mut Graphics<R>,
) -> io::Result<Exit>
where
    B: Backend<Error = io::Error> + Write,
    W: Wakes,
    D: Dispatch,
    R: GraphicsReplies,
{
    let exit = frames(terminal, wakes, jobs, session, state, graphics);
    let released = release(terminal, graphics);
    // When the loop itself failed, its error is the one to report; a free
    // that fails too failed writing to the same broken terminal.
    exit.and_then(|exit| released.map(|()| exit))
}

/// The loop's body: draw, wait, update, repeat.
fn frames<B, W, D, R>(
    terminal: &mut Terminal<B>,
    wakes: &mut W,
    jobs: &D,
    session: &mut Session,
    mut state: AppState,
    graphics: &mut Graphics<R>,
) -> io::Result<Exit>
where
    B: Backend<Error = io::Error> + Write,
    W: Wakes,
    D: Dispatch,
    R: GraphicsReplies,
{
    // Draw before waiting: the first frame must be on screen while Graft sits
    // idle waiting for the user's first key.
    draw(terminal, session, &state, graphics)?;

    loop {
        // Idle: nothing is waiting, so get the next page ready.
        prefetch(session, &state, graphics, wakes);
        let Some(wake) = wakes.next_wake() else {
            // Nothing can wake the loop again. Leave as if the user had quit
            // rather than spinning on a dead channel.
            return Ok(Exit::Quit);
        };
        // Replies to the frames already drawn: an `ENOENT` here makes the
        // next frame transmit that strip again.
        let replied = graphics.drain()?;

        let event = match wake {
            Wake::Event(event) => event,
            Wake::Shutdown(signal) => return Ok(Exit::Signal(signal)),
            Wake::InputFailed(error) | Wake::TerminalFailed(error) => return Err(error),
            Wake::Terminal(change) => {
                let changed = session.apply(change);
                if changed {
                    graphics.flush(match change {
                        TerminalChange::CellSize(_) => Flush::CellSize,
                        TerminalChange::Theme(_) => Flush::Theme,
                    });
                }
                if changed || replied {
                    draw(terminal, session, &state, graphics)?;
                }
                continue;
            }
        };

        let (next, commands) = update(&state, event);
        state = next;

        for command in commands {
            // Output, not work: the clipboard request goes to the terminal
            // now, and a failed write ends the loop like any other.
            let command = match command {
                Command::CopyToClipboard(text) => {
                    write_clipboard(terminal, session, &text)?;
                    continue;
                }
                command => command,
            };
            // Handed over and forgotten. `dispatch` starts the work and
            // returns: there is no handle here to join and no result to wait
            // for, because the result comes back as another `Event`
            // (ARCHITECTURE.md). This is the line that keeps the keyboard
            // responsive during a two-minute fetch.
            jobs.dispatch(command);
        }

        match &state.quit {
            Some(Quit::Requested) => return Ok(Exit::Quit),
            Some(Quit::HandedBack(key)) => return Ok(Exit::HandedBack(*key)),
            // Reported by the caller, once the terminal is the user's again.
            Some(Quit::Fatal(reason)) => return Ok(Exit::Fatal(reason.clone())),
            None => {}
        }

        draw(terminal, session, &state, graphics)?;
    }
}

/// While nothing is waiting, rasterise the strips a page key would show:
/// `PageDown`'s and `PageUp`'s next screens, as `update` would draw them
/// (their cursor moves, so their thread and fold do too, and only strips
/// keyed exactly like that frame's are hits). Asks `wakes` before every
/// strip, so a key that arrives waits behind one strip at most; it never
/// uploads anything. Returns the strips rasterised.
fn prefetch<W, R>(
    session: &Session,
    state: &AppState,
    graphics: &mut Graphics<R>,
    wakes: &mut W,
) -> usize
where
    W: Wakes,
    R: GraphicsReplies,
{
    // The search prompt takes page keys as text: nothing to ready there.
    if !draws_pixels(session, state) || state.search_prompt.is_some() {
        return 0;
    }
    let mut rasterised = 0;
    for page in [KeyCode::PageDown, KeyCode::PageUp] {
        // A look before each page's what-if too: its `update` clones the
        // state (every ref badge with it), work a key must not queue behind.
        if wakes.waiting() {
            break;
        }
        // What the screen would be after `page`; never shown. Its commands
        // (rows to fetch) are dropped: the real key asks for them again.
        let (paged, _unsent) = update(state, AppEvent::Core(Event::Key(KeyPress::plain(page))));
        rasterised += graphics.prefetch(session, &paged, &mut || wakes.waiting());
    }
    rasterised
}

/// Ask the terminal to put `text` on its clipboard (OSC 52), wrapped for
/// tmux the way the image commands are, as one whole write.
fn write_clipboard<B>(terminal: &mut Terminal<B>, session: &Session, text: &str) -> io::Result<()>
where
    B: Backend<Error = io::Error> + Write,
{
    let mut bytes = Vec::new();
    Transport::for_tmux(&session.capabilities.tmux)
        .write(&graft_term::clipboard::osc52_copy(text), &mut bytes);
    let _whole = redetect::hold_output();
    let backend = terminal.backend_mut();
    backend
        .write_all(&bytes)
        .and_then(|()| Write::flush(backend))
}

/// Delete every image Graft uploaded, as one whole write.
fn release<B, R>(terminal: &mut Terminal<B>, graphics: &mut Graphics<R>) -> io::Result<()>
where
    B: Backend<Error = io::Error> + Write,
    R: GraphicsReplies,
{
    let mut bytes = Vec::new();
    graphics.release(&mut bytes);
    if bytes.is_empty() {
        return Ok(());
    }
    let _whole = redetect::hold_output();
    let backend = terminal.backend_mut();
    backend
        .write_all(&bytes)
        .and_then(|()| Write::flush(backend))
}

/// Draw one frame, wrapped in a synchronized update.
///
/// The begin/end pair goes out unconditionally (DESIGN.md §4.1): a terminal
/// that does not understand DEC 2026 ignores the sequence, and probing for it
/// would cost a round-trip on every frame to learn something that changes
/// nothing.
///
/// At the pixel tier the images go out after the cells, inside the same
/// update, so text and graph appear together; never after a clear.
fn draw<B, R>(
    terminal: &mut Terminal<B>,
    session: &Session,
    state: &AppState,
    graphics: &mut Graphics<R>,
) -> io::Result<()>
where
    B: Backend<Error = io::Error> + Write,
    R: GraphicsReplies,
{
    // A whole frame, so a re-query cannot land inside one of its sequences.
    let _frame = redetect::hold_output();
    terminal
        .backend_mut()
        .write_all(BEGIN_SYNCHRONIZED_UPDATE)?;

    let mut graph = None;
    let drawn = terminal
        .draw(|frame| graph = view_with_graphics(session, state, graphics.status(), frame))
        .map(|_| ());
    let placed = drawn.as_ref().map_or(Ok(()), |()| {
        let mut images = Vec::new();
        graphics.frame(session, state, graph, &mut images);
        terminal.backend_mut().write_all(&images)
    });

    // The end marker goes out even when the draw failed. Leaving the terminal
    // inside a synchronized update would freeze the user's screen on whatever
    // was there before, with no way to tell that anything is wrong.
    let ended = terminal
        .backend_mut()
        .write_all(END_SYNCHRONIZED_UPDATE)
        .and_then(|()| Write::flush(terminal.backend_mut()));

    drawn.and(placed).and(ended)
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
    ///
    /// Every scripted wake is already queued, so `waiting` says `true` —
    /// unless `idle` says how many times it answers `false` first.
    struct ScriptedWakes {
        remaining: std::vec::IntoIter<Wake>,
        /// `idle[i]`: `waiting` calls that find nothing before wake `i`.
        idle: std::collections::VecDeque<usize>,
        asked: usize,
    }

    impl ScriptedWakes {
        fn new(wakes: Vec<Wake>) -> Self {
            Self::idle(wakes, Vec::new())
        }

        fn idle(wakes: Vec<Wake>, idle: Vec<usize>) -> Self {
            Self {
                remaining: wakes.into_iter(),
                idle: idle.into(),
                asked: 0,
            }
        }
    }

    impl Wakes for ScriptedWakes {
        fn next_wake(&mut self) -> Option<Wake> {
            self.idle.pop_front();
            self.remaining.next()
        }

        fn waiting(&mut self) -> bool {
            self.asked += 1;
            match self.idle.front_mut() {
                Some(left) if *left > 0 => {
                    *left -= 1;
                    false
                }
                _ => true,
            }
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

    /// Replies handed out one batch per wake.
    struct ScriptedReplies(std::collections::VecDeque<Vec<u8>>);

    impl GraphicsReplies for ScriptedReplies {
        fn take(&mut self) -> io::Result<Vec<u8>> {
            Ok(self.0.pop_front().unwrap_or_default())
        }
    }

    const PIXEL_COLUMNS: u16 = 80;
    const PIXEL_ROWS: u16 = 24;

    /// A Kitty-protocol terminal at the pixel tier, showing 100 commits
    /// requested in whole strips.
    fn at_pixel_tier() -> (Session, AppState) {
        use graft_core::capabilities::{PixelProtocol, RenderingTier};
        let mut session = session();
        session.rendering.tier = RenderingTier::Pixel;
        session.capabilities.pixel_protocol = Some(PixelProtocol::Kitty);
        let mut history = crate::view::tests::history_of(100, 0);
        history.strip_rows = Some(graft_graph::raster::cache::STRIP_ROWS);
        let state = AppState {
            history: Some(history),
            settled_tier: Some(RenderingTier::Pixel),
            size: Size {
                columns: PIXEL_COLUMNS,
                rows: PIXEL_ROWS,
            },
            ..AppState::default()
        };
        (session, state)
    }

    /// Runs the loop at the pixel tier through `wakes`, answering with
    /// `replies`; returns every frame (split on the end of each synchronized
    /// update) and whatever was written after the last one.
    fn pixel_run(wakes: Vec<Wake>, replies: Vec<Vec<u8>>) -> (Vec<String>, String) {
        let (mut terminal, recorder) = terminal(PIXEL_COLUMNS, PIXEL_ROWS);
        let (mut session, state) = at_pixel_tier();
        let mut graphics = Graphics::new(ScriptedReplies(replies.into()));
        let exit = pump_with(
            &mut terminal,
            &mut ScriptedWakes::new(wakes),
            &Recorded::default(),
            &mut session,
            state,
            &mut graphics,
        )
        .expect("writing to memory cannot fail");
        assert_eq!(exit, Exit::Quit);
        let written = recorder.written();
        let end = String::from_utf8_lossy(END_SYNCHRONIZED_UPDATE).into_owned();
        let mut frames: Vec<String> = written.split(&end).map(str::to_owned).collect();
        let tail = frames.pop().expect("split yields at least one piece");
        (frames, tail)
    }

    /// Image ids a stream transmits, in order.
    fn transmitted(stream: &str) -> Vec<u32> {
        stream
            .split("a=t,f=32,t=d,i=")
            .skip(1)
            .map(|rest| {
                rest.split(',')
                    .next()
                    .and_then(|id| id.parse().ok())
                    .expect("an id follows i=")
            })
            .collect()
    }

    /// Strip-cache flushes over a pixel-tier run through `wakes`, and every
    /// frame it drew.
    fn pixel_flushes(wakes: Vec<Wake>) -> (u64, Vec<String>) {
        let (mut terminal, recorder) = terminal(PIXEL_COLUMNS, PIXEL_ROWS);
        let (mut session, state) = at_pixel_tier();
        let mut graphics = Graphics::new(NoReplies);
        pump_with(
            &mut terminal,
            &mut ScriptedWakes::new(wakes),
            &Recorded::default(),
            &mut session,
            state,
            &mut graphics,
        )
        .expect("writing to memory cannot fail");
        let end = String::from_utf8_lossy(END_SYNCHRONIZED_UPDATE).into_owned();
        let frames = recorder.written().split(&end).map(str::to_owned).collect();
        (graphics.strip_stats().flushes, frames)
    }

    /// A pixel-tier run on `protocol` through `wakes`: every frame drawn,
    /// the graphics state after, and how long each `draw` took.
    fn pixel_frames(
        protocol: graft_core::capabilities::PixelProtocol,
        wakes: Vec<Wake>,
    ) -> (Vec<String>, Graphics<NoReplies>, Vec<std::time::Duration>) {
        let (mut terminal, recorder) = terminal(PIXEL_COLUMNS, PIXEL_ROWS);
        let (mut session, mut state) = at_pixel_tier();
        session.capabilities.pixel_protocol = Some(protocol);
        let mut graphics = Graphics::new(NoReplies);
        let mut times = Vec::new();
        let started = std::time::Instant::now();
        draw(&mut terminal, &session, &state, &mut graphics).expect("memory");
        times.push(started.elapsed());
        for wake in wakes {
            let Wake::Event(event) = wake else {
                panic!("only events are scripted here");
            };
            state = update(&state, event).0;
            let started = std::time::Instant::now();
            draw(&mut terminal, &session, &state, &mut graphics).expect("memory");
            times.push(started.elapsed());
        }
        let end = String::from_utf8_lossy(END_SYNCHRONIZED_UPDATE).into_owned();
        let frames = recorder.written().split(&end).map(str::to_owned).collect();
        (frames, graphics, times)
    }

    #[test]
    fn a_resize_re_places_strips_without_retransmitting_them() {
        use graft_core::capabilities::PixelProtocol;
        let (_, still, _) = pixel_frames(PixelProtocol::Kitty, vec![]);
        let (frames, graphics, times) = pixel_frames(
            PixelProtocol::Kitty,
            vec![resized(PIXEL_COLUMNS - 20, PIXEL_ROWS - 4)],
        );
        assert!(!transmitted(&frames[0]).is_empty());
        assert!(
            transmitted(&frames[1]).is_empty(),
            "a resize transmits nothing: {:?}",
            frames[1]
        );
        assert!(frames[1].contains("a=p,"), "the strips are placed again");
        assert_eq!(graphics.strip_stats().flushes, 0);
        assert_eq!(
            graphics.strip_stats().rasterised,
            still.strip_stats().rasterised,
            "no strip rasterised again"
        );
        // Measured, not asserted: a debug build without a terminal's write
        // cost (m1-the-graph/25 reports the release numbers).
        eprintln!(
            "kitty first frame {:?}, resize frame {:?}",
            times[0], times[1]
        );
    }

    #[test]
    fn iterm2_resends_every_band_but_encodes_an_unchanged_one_once() {
        use graft_core::capabilities::PixelProtocol;
        let (frames, graphics, times) = pixel_frames(
            PixelProtocol::Iterm2,
            vec![
                resized(PIXEL_COLUMNS, PIXEL_ROWS),
                resized(PIXEL_COLUMNS - 20, PIXEL_ROWS),
            ],
        );
        let bands = frames[0].matches("\x1b]1337;File=").count();
        assert!(bands > 0, "{:?}", frames[0]);
        assert_eq!(frames[1].matches("\x1b]1337;File=").count(), bands);
        assert_eq!(frames[2].matches("\x1b]1337;File=").count(), bands);
        assert_eq!(
            graphics.bands_encoded(),
            u64::try_from(bands).unwrap(),
            "re-sent twice, encoded once"
        );
        assert!(!frames[1].contains("a=t,"), "no Kitty bytes on iTerm2");
        eprintln!(
            "iterm2 first frame {:?}, unchanged frame {:?}, resize frame {:?}",
            times[0], times[1], times[2]
        );
    }

    #[test]
    fn the_loop_no_longer_flushes_strips_on_a_resize() {
        let (flushes, frames) = pixel_flushes(vec![resized(PIXEL_COLUMNS, PIXEL_ROWS), quit_key()]);
        assert_eq!(flushes, 0);
        assert!(transmitted(&frames[1]).is_empty(), "{:?}", frames[1]);
    }

    #[test]
    fn a_new_cell_size_or_theme_flushes_the_strips() {
        use graft_core::capabilities::{Theme, ThemeHint, ThemeSource};
        let (session, _) = at_pixel_tier();
        let mut cell = session.capabilities.cell_size;
        cell.width += 1;
        let theme = ThemeHint {
            theme: Theme::Light,
            source: ThemeSource::Assumed,
        };
        let (flushes, _) = pixel_flushes(vec![
            Wake::Terminal(TerminalChange::CellSize(cell)),
            Wake::Terminal(TerminalChange::Theme(theme)),
            // Unchanged: nothing to flush.
            Wake::Terminal(TerminalChange::Theme(theme)),
            quit_key(),
        ]);
        assert_eq!(flushes, 2);
    }

    #[test]
    fn forced_pixels_on_a_silent_terminal_attempt_kitty_and_f3_recovers() {
        let (mut terminal, recorder) = terminal(PIXEL_COLUMNS, PIXEL_ROWS);
        let (mut session, mut state) = at_pixel_tier();
        session.capabilities.pixel_protocol = None;
        state.settled_tier = Some(session.rendering.tier);
        let mut graphics = Graphics::new(NoReplies);
        pump_with(
            &mut terminal,
            &mut ScriptedWakes::new(vec![key(KeyPress::plain(KeyCode::Function(3))), quit_key()]),
            &Recorded::default(),
            &mut session,
            state,
            &mut graphics,
        )
        .expect("writing to memory cannot fail");
        let end = String::from_utf8_lossy(END_SYNCHRONIZED_UPDATE).into_owned();
        let written = recorder.written();
        let frames: Vec<&str> = written.split(&end).collect();
        assert!(!transmitted(frames[0]).is_empty(), "Kitty attempted");
        assert!(
            frames[1].contains("a=d,d=I,i="),
            "uploads freed: {:?}",
            frames[1]
        );
        assert!(
            frames[1].contains("renderer: unicode (F3 cycles)"),
            "{:?}",
            frames[1]
        );
        assert!(!frames[1].contains("gfx"), "no pixels, no gfx status");
    }

    fn unbound_key() -> Wake {
        key(KeyPress::plain(KeyCode::Char('z')))
    }

    #[test]
    fn at_the_pixel_tier_strips_are_sent_once_and_freed_on_exit() {
        let (frames, tail) = pixel_run(vec![unbound_key(), unbound_key(), quit_key()], vec![]);

        assert_eq!(frames.len(), 3);
        let sent = transmitted(&frames[0]);
        assert!(!sent.is_empty(), "the first frame uploads its strips");
        assert!(frames[0].contains("a=p,i="), "and places them");
        assert!(frames[0].contains("gfx ok"), "{}", frames[0]);
        for frame in &frames[1..] {
            assert!(
                transmitted(frame).is_empty(),
                "steady state re-sends nothing"
            );
        }
        let everything = frames.concat() + &tail;
        assert!(!everything.contains("\x1b[2J"), "never a clear");
        assert!(!everything.contains("q=2"), "never silenced replies");
        for id in &sent {
            assert!(
                tail.contains(&format!("a=d,d=I,i={id},")),
                "{id} freed: {tail:?}"
            );
        }
    }

    #[test]
    fn an_enoent_reply_re_sends_the_strip_and_says_so() {
        let (frames, _tail) = pixel_run(
            vec![unbound_key(), quit_key()],
            vec![b"\x1b_Gi=1000;ENOENT:image not found\x1b\\".to_vec()],
        );

        assert_eq!(transmitted(&frames[0]).first(), Some(&1000));
        assert!(!transmitted(&frames[1]).is_empty(), "{:?}", frames[1]);
        assert!(
            frames[1].contains("gfx: ENOENT:image not found"),
            "{:?}",
            frames[1]
        );
    }

    /// Pixels, then `F3` three times (Unicode, ASCII, pixels again), then
    /// quit: the four frames and the tail.
    fn f3_round_trip() -> (Vec<String>, String) {
        let f3 = || key(KeyPress::plain(KeyCode::Function(3)));
        let (frames, tail) = pixel_run(vec![f3(), f3(), f3(), quit_key()], vec![]);
        assert_eq!(frames.len(), 4);
        (frames, tail)
    }

    #[test]
    fn f3_off_pixels_frees_every_upload_and_names_the_tier() {
        let (frames, _tail) = f3_round_trip();
        let first = transmitted(&frames[0]);
        assert!(!first.is_empty(), "{:?}", frames[0]);
        for id in &first {
            assert!(
                frames[1].contains(&format!("a=d,d=I,i={id},")),
                "{id} freed leaving pixels: {:?}",
                frames[1]
            );
        }
        assert!(frames[1].contains("renderer: unicode (F3 cycles)"));
        assert!(!frames[1].contains("gfx"), "no pixels, no gfx status");
        // Later frames are diffs: only the changed cells are written.
        assert!(frames[2].contains("ascii"), "{:?}", frames[2]);
        assert!(transmitted(&frames[2]).is_empty());
    }

    #[test]
    fn f3_back_onto_pixels_transmits_afresh_and_frees_every_id_by_exit() {
        let (frames, tail) = f3_round_trip();
        let first = transmitted(&frames[0]);
        let again = transmitted(&frames[3]);
        assert!(!again.is_empty(), "back on pixels, strips sent again");
        assert!(
            again.iter().all(|id| !first.contains(id)),
            "fresh ids, never a freed one: {first:?} {again:?}"
        );
        assert!(frames[3].contains("a=p,i="), "and placed");
        assert!(frames[3].contains("gfx ok"), "{:?}", frames[3]);
        assert!(!frames[3].contains("F3 cycles"), "nothing extra at the top");

        let everything = frames.concat() + &tail;
        for id in first.iter().chain(&again) {
            assert!(
                everything.contains(&format!("a=d,d=I,i={id},")),
                "{id} freed by exit"
            );
        }
    }

    #[test]
    fn leaving_the_graph_screen_removes_the_placements() {
        let (frames, _tail) = pixel_run(
            vec![key(KeyPress::plain(KeyCode::Function(2))), quit_key()],
            vec![],
        );

        assert!(frames[1].contains("a=d,d=i,i="), "{:?}", frames[1]);
        assert!(!frames[1].contains("gfx"), "no status off the pixel graph");
    }

    #[test]
    fn the_quit_key_ends_the_loop() {
        let (mut terminal, _recorder) = terminal(20, 3);
        let mut wakes = ScriptedWakes::new(vec![quit_key()]);

        let exit = pump(
            &mut terminal,
            &mut wakes,
            &Recorded::default(),
            &mut session(),
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
            &mut session(),
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
            &mut session(),
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
            &mut session(),
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
            &mut session(),
            AppState::default(),
        )
        .expect("writing to memory cannot fail");

        assert_eq!(exit, Exit::Quit);
    }

    /// The state Graft starts in, before `Task::ReadHistory`'s `Started`
    /// event arrives: `history` is `None`. `headless_settled` must not count
    /// this as settled, or `graft repo | cat` prints an "opening…" frame with
    /// no rows and exits 0 before the walk has said anything (M1 review,
    /// M4, `KNOWN_GAPS.md:181`).
    #[test]
    fn headless_is_not_settled_before_history_exists() {
        assert!(!headless_settled(&AppState::default()));
    }

    #[test]
    fn headless_is_not_settled_while_history_is_streaming() {
        let state = AppState {
            history: Some(crate::history::History::started()),
            ..AppState::default()
        };
        assert!(!headless_settled(&state));
    }

    #[test]
    fn headless_is_settled_once_history_completes() {
        let mut history = crate::history::History::started();
        history.phase = Phase::Complete;
        let state = AppState {
            history: Some(history),
            ..AppState::default()
        };
        assert!(headless_settled(&state));
    }

    #[test]
    fn headless_is_settled_when_the_walk_fails() {
        let mut history = crate::history::History::started();
        history.phase = Phase::Failed("disk read error".to_string());
        let state = AppState {
            history: Some(history),
            ..AppState::default()
        };
        assert!(headless_settled(&state));
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
            &mut session(),
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
    fn graft_starts_at_the_terminals_size_not_zero_by_zero() {
        // Found under a pty: with a 0×0 starting size the history walk was
        // asked for one row, and the graph drew one row, until a resize.
        let size = crate::update::Size {
            columns: 80,
            rows: 24,
        };

        let state = opening_repository(&Recorded::default(), Path::new("/code/api"), size);

        assert_eq!(state.size, size);
        assert_eq!(state.graph_rows(), 22);
    }

    #[test]
    fn the_status_screen_reaches_the_terminal() {
        let (mut terminal, recorder) = terminal(60, 12);
        let mut wakes = ScriptedWakes::new(vec![quit_key()]);
        let state = AppState {
            screen: crate::update::Screen::Status,
            ..AppState::default()
        };

        pump(
            &mut terminal,
            &mut wakes,
            &Recorded::default(),
            &mut session(),
            state,
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
            &mut session(),
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

        pump(&mut terminal, &mut wakes, &jobs, &mut session(), state)
            .expect("writing to memory cannot fail");

        assert_eq!(
            jobs.commands(),
            vec![
                Command::OpenRepository,
                Command::ReadHistory(Order::Topological),
                Command::ReadRefs,
                Command::ReadWorkingTree,
            ]
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
            &mut session(),
            AppState::default(),
        )
        .expect("writing to memory cannot fail");

        let bytes = recorder.written();
        for word in ["failed", "repository"] {
            assert!(bytes.contains(word), "{word:?} never reached the terminal");
        }
    }

    /// A pixel-tier loop through `wakes`, `idle` as [`ScriptedWakes::idle`],
    /// starting from `state`: strips rasterised, strips served from the cache.
    fn prefetch_run(state: AppState, wakes: Vec<Wake>, idle: Vec<usize>) -> (u64, u64) {
        let (mut terminal, _recorder) = terminal(PIXEL_COLUMNS, PIXEL_ROWS);
        let (mut session, _) = at_pixel_tier();
        let mut graphics = Graphics::new(NoReplies);
        pump_with(
            &mut terminal,
            &mut ScriptedWakes::idle(wakes, idle),
            &Recorded::default(),
            &mut session,
            state,
            &mut graphics,
        )
        .expect("writing to memory cannot fail");
        let counters = graphics.strip_stats();
        (counters.rasterised, counters.hits)
    }

    /// 255 commits, the screen showing row `top` with the cursor on it.
    fn scrolled_to(top: usize) -> AppState {
        let (_, mut state) = at_pixel_tier();
        let mut history = crate::view::tests::history_of(255, top);
        history.top = top;
        history.strip_rows = Some(graft_graph::raster::cache::STRIP_ROWS);
        state.history = Some(history);
        state
    }

    #[test]
    fn no_strip_is_prefetched_while_input_is_queued() {
        let queued = prefetch_run(scrolled_to(100), vec![quit_key()], vec![0]);
        let idle = prefetch_run(scrolled_to(100), vec![quit_key()], vec![usize::MAX]);

        let (session, _) = at_pixel_tier();
        let mut drawn_only = Graphics::new(NoReplies);
        let (mut terminal, _) = terminal(PIXEL_COLUMNS, PIXEL_ROWS);
        draw(&mut terminal, &session, &scrolled_to(100), &mut drawn_only).expect("memory");

        assert_eq!(
            queued.0,
            drawn_only.strip_stats().rasterised,
            "with a key queued the loop rasterises only what the frame shows"
        );
        assert!(
            idle.0 > queued.0,
            "idle, the loop does prefetch (else this proves nothing): {idle:?} vs {queued:?}"
        );
    }

    #[test]
    fn a_key_arriving_mid_prefetch_waits_for_at_most_one_strip() {
        // The key arrives after `checks` looks at the queue: each look more
        // may let one more strip through, never two.
        let rasterised: Vec<u64> = (0..12)
            .map(|checks| prefetch_run(scrolled_to(100), vec![quit_key()], vec![checks]).0)
            .collect();
        for pair in rasterised.windows(2) {
            assert!(
                pair[1] <= pair[0] + 1,
                "one more look at the queue let more than one strip through: {rasterised:?}"
            );
        }
        let whole = prefetch_run(scrolled_to(100), vec![quit_key()], vec![usize::MAX]).0;
        assert!(
            whole >= rasterised[0] + 2,
            "an uninterrupted prefetch does at least two strips (else this proves nothing): {whole} vs {rasterised:?}"
        );
    }

    #[test]
    fn page_keys_find_their_strips_already_rasterised() {
        for page in [KeyCode::PageDown, KeyCode::PageUp] {
            let (session, _) = at_pixel_tier();
            let (mut terminal, _) = terminal(PIXEL_COLUMNS, PIXEL_ROWS);
            let mut graphics = Graphics::new(NoReplies);
            let state = scrolled_to(100);
            draw(&mut terminal, &session, &state, &mut graphics).expect("memory");
            let done = prefetch(
                &session,
                &state,
                &mut graphics,
                &mut ScriptedWakes::idle(Vec::new(), vec![usize::MAX]),
            );
            let before = graphics.strip_stats();

            let paged = update(&state, AppEvent::Core(Event::Key(KeyPress::plain(page)))).0;
            draw(&mut terminal, &session, &paged, &mut graphics).expect("memory");
            let after = graphics.strip_stats();

            assert!(done > 0, "{page:?}: something was prefetched");
            assert_eq!(
                after.rasterised, before.rasterised,
                "{page:?}'s frame rasterised nothing: its strips were prefetched"
            );
            assert!(
                after.hits > before.hits,
                "{page:?}'s frame drew from the cache"
            );
        }
    }

    #[test]
    fn a_wake_seen_while_checking_is_handed_out_next_and_kept_as_unhandled_input() {
        let (mut wakes, sender) = ChannelWakes::new();
        assert!(!wakes.waiting(), "nothing sent yet");
        sender.send(quit_key()).expect("receiver alive");
        sender
            .send(key(KeyPress::plain(KeyCode::Char('j'))))
            .expect("receiver alive");
        assert!(wakes.waiting());
        assert!(wakes.waiting(), "asking again loses nothing");
        drop(sender);

        let unhandled = wakes.unhandled_input();
        assert_eq!(
            unhandled.len(),
            2,
            "the wake seen while checking is kept: {unhandled:?}"
        );

        let (mut wakes, sender) = ChannelWakes::new();
        sender.send(quit_key()).expect("receiver alive");
        assert!(wakes.waiting());
        drop(sender);
        assert!(matches!(wakes.next_wake(), Some(Wake::Event(_))));
        assert!(wakes.next_wake().is_none());
        assert!(
            wakes.waiting(),
            "an ended source is worth finding out about"
        );
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

    #[test]
    fn the_input_a_finished_loop_never_handled_is_returned_in_order_without_the_rest() {
        let (mut wakes, sender) = ChannelWakes::new();
        sender
            .send(key(KeyPress::plain(KeyCode::Char(']'))))
            .unwrap();
        sender.send(Wake::Shutdown(15)).unwrap();
        sender.send(resized(80, 24)).unwrap();
        sender
            .send(key(KeyPress::plain(KeyCode::Char('q'))))
            .unwrap();
        assert!(matches!(wakes.next_wake(), Some(Wake::Event(_))));

        let unhandled = wakes.unhandled_input();
        assert_eq!(
            unhandled,
            vec![
                Event::Resize {
                    columns: 80,
                    rows: 24
                },
                Event::Key(KeyPress::plain(KeyCode::Char('q'))),
            ]
        );
    }

    /// The first of 255 commits selected, at the default (Outside) session.
    fn copy_run(session: &mut Session) -> (Result<Exit, io::Error>, Recorder, String) {
        let (mut terminal, recorder) = terminal(80, 24);
        let history = crate::view::tests::history_of(255, 0);
        let sha = history
            .selected()
            .expect("the cursor row is on hand")
            .commit
            .id
            .to_string();
        let state = AppState {
            history: Some(history),
            ..AppState::default()
        };
        let mut wakes =
            ScriptedWakes::new(vec![key(KeyPress::plain(KeyCode::Char('y'))), quit_key()]);
        let exit = pump(
            &mut terminal,
            &mut wakes,
            &Recorded::default(),
            session,
            state,
        );
        (exit, recorder, sha)
    }

    #[test]
    fn y_writes_the_osc_52_request_to_the_terminal_and_not_to_the_jobs() {
        let (exit, recorder, sha) = copy_run(&mut session());
        exit.expect("writing to memory cannot fail");
        let sequence =
            String::from_utf8(graft_term::clipboard::osc52_copy(&sha)).expect("OSC 52 is ASCII");
        let written = recorder.written();
        assert!(written.contains(&sequence), "no OSC 52 for {sha}");
        assert!(written.contains(" → clipboard (osc 52)"), "no notice drawn");
    }

    #[test]
    fn inside_tmux_the_osc_52_request_is_wrapped_in_the_passthrough() {
        let mut inside = session();
        inside.capabilities.tmux = graft_core::capabilities::TmuxStatus::Passthrough {
            version: "3.4".to_owned(),
        };
        let (exit, recorder, sha) = copy_run(&mut inside);
        exit.expect("writing to memory cannot fail");
        let mut wrapped = Vec::new();
        graft_term::tmux_passthrough(&graft_term::clipboard::osc52_copy(&sha), &mut wrapped);
        let wrapped = String::from_utf8(wrapped).expect("ASCII");
        assert!(recorder.written().contains(&wrapped));
    }

    /// A terminal whose clipboard write fails: frames go through, the OSC 52
    /// request does not.
    struct RefusesOsc52;

    impl Write for RefusesOsc52 {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            if buf.windows(4).any(|window| window == b"]52;") {
                return Err(io::Error::other("the terminal went away"));
            }
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn a_failed_clipboard_write_ends_the_loop_with_its_error() {
        let mut terminal = Terminal::with_options(
            CrosstermBackend::new(RefusesOsc52),
            TerminalOptions {
                viewport: Viewport::Fixed(Rect::new(0, 0, 80, 24)),
            },
        )
        .expect("a fixed viewport never asks the backend for its size");
        let state = AppState {
            history: Some(crate::view::tests::history_of(10, 0)),
            ..AppState::default()
        };
        let mut wakes =
            ScriptedWakes::new(vec![key(KeyPress::plain(KeyCode::Char('y'))), quit_key()]);
        let error = pump(
            &mut terminal,
            &mut wakes,
            &Recorded::default(),
            &mut session(),
            state,
        )
        .expect_err("the write error reaches the caller");
        assert_eq!(error.to_string(), "the terminal went away");
    }
}
