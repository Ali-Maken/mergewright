//! Sets up the terminal and runs the interactive loop, or hands off to
//! [`super::headless`] when stdout is not a terminal.

use std::io::{self, IsTerminal};
use std::sync::Arc;
use std::sync::mpsc::Sender;

use graft_core::capabilities::Supported;
use graft_term::Transport;
use graft_term::detect::{self, TtyHost};
use graft_term::input::{InputMux, Inputs};

use super::exit_wait::{Ending, Pool};
use super::graphics::Graphics;
use super::headless::run_headless;
use super::redetect::Redetector;
use super::running::{Screen, pump_with};
use super::{ChannelWakes, Exit, Wake, signals};

use crate::crash;
use crate::jobs::JobRunner;
use crate::repos::RepoStore;
use crate::session::{Session, Startup};
use crate::terminal::{SessionOptions, TerminalSession};

mod arrival;
mod listen;
mod state;
mod wind_down;

pub(super) use arrival::{Arrival, arrive};
pub(super) use listen::hand_back_input;
use listen::{Listening, listen, outside};
use state::terminal_and_state;
use wind_down::{put_away, wind_down};
// Only the tests reach these through `startup`; the children use them.
#[cfg(test)]
use state::seed_config;
#[cfg(test)]
use wind_down::{excused_by_hangup, finish_to, report_stalled_jobs_to};

/// Undo, spelled once per place for the exit line of a write git could not
/// pause (13d): the process has one keymap (ADR-0055), and that line is
/// printed after the loop, and its state, are gone (`exit_wait::Pool`).
pub(super) const UNDO: graft_ui::keymap::ActionId = graft_ui::keymap::ActionId("app.undo");

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

/// One repository, opened once and shared by every job that follows
/// (`crate::repos`).
///
/// Opened *before* the terminal is touched: a path that is not a repository
/// is a plain line on a plain terminal, never an alternate screen flashed for
/// the length of the capability probe. `graph.default_branch` is ADR-0001's
/// rule 1, so it goes where the branches are resolved.
pub(super) fn open_repository(
    repo_path: &std::path::Path,
    config: &crate::config::Config,
) -> Result<Arc<RepoStore>, String> {
    let repos = Arc::new(RepoStore::new(
        repo_path.to_path_buf(),
        config.graph.default_branch.clone(),
    ));
    match repos.shared() {
        Ok(_) => Ok(repos),
        Err(error) => Err(error.to_string()),
    }
}

/// Ask the terminal what it can do, and push the kitty keyboard flag if it
/// turns out to speak the protocol — which could not be decided when the
/// session opened, because the probe needs the raw mode the session turns on.
fn probe(
    environment: &detect::Environment,
    mux: &InputMux,
    session: &TerminalSession,
) -> io::Result<graft_core::capabilities::Capabilities> {
    let capabilities = detect::detect(environment, &mut TtyHost::new(mux.waiter()));
    if capabilities.kitty_keyboard == Supported::Yes {
        session.enable_kitty_keyboard()?;
    }
    Ok(capabilities)
}

/// The terminal session, with the panic hook already able to restore it.
///
/// Kitty keyboard is deliberately off here and pushed on later: whether the
/// terminal speaks the protocol is something only the probe knows, and the
/// probe needs the raw mode this session turns on.
fn open_session() -> io::Result<TerminalSession> {
    let session = TerminalSession::on_stdout(SessionOptions::default())?;
    crash::install(session.restorer());
    Ok(session)
}

/// The terminal Graft draws on, opened **once per process** by `main` and
/// handed from place to place (m8-polish-and-ship/08, decision 1): the
/// alternate screen is entered once, the capability probe asked once, the
/// crash hook installed once, and the terminal given back once, by
/// [`Terminal::close`]. A switch from the empty state to a repository (or,
/// later, between repositories) keeps all of it.
pub struct Terminal {
    session: TerminalSession,
    /// The process's input reader. Each place takes a fresh [`Inputs`] from
    /// it ([`Terminal::inputs`]) and holds it back when done.
    pub(super) mux: InputMux,
    /// The first place's `Inputs`, from the same call that started the mux.
    first: Option<Inputs>,
    environment: detect::Environment,
    /// What the one probe found.
    pub(super) capabilities: graft_core::capabilities::Capabilities,
}

impl Terminal {
    /// Take the terminal over: raw mode and the alternate screen, the crash
    /// hook, the input reader, then the probe — the only caller of
    /// [`probe`] on the `main` path.
    ///
    /// # Errors
    /// The terminal could not be set up or probed.
    pub fn open() -> io::Result<Self> {
        let session = open_session()?;
        // The mux owns stdin for the process (ticket m1-the-graph/20). It
        // starts before the probe, so the probe's replies reach detection
        // through a waiter while any key typed meanwhile is queued, in order,
        // for the first place.
        let (mux, inputs) = InputMux::on_stdin()?;
        let environment = detect::Environment::from_process();
        let capabilities = probe(&environment, &mux, &session)?;
        Ok(Self {
            session,
            mux,
            first: Some(inputs),
            environment,
            capabilities,
        })
    }

    /// The `Inputs` for the place about to run: the first one, then a fresh
    /// one from the mux each time (keys held since the last place come
    /// first, `InputMux::inputs`).
    pub(super) fn inputs(&mut self) -> Inputs {
        match self.first.take() {
            Some(inputs) => inputs,
            None => self.mux.inputs(),
        }
    }

    /// Give the terminal back, then report what the last place left behind:
    /// jobs that would not stop, a restore that failed.
    ///
    /// # Errors
    /// The place's own failure, or the restore's.
    pub fn close(self, ran: Ran) -> io::Result<Exit> {
        wind_down(&self.session, ran.pool, ran.outcome)
    }
}

/// How one place ended: the loop's outcome, and the job pool still to stop
/// (m8/13b review fix: stopped **after** the terminal is restored, since
/// its wait for a write can last up to `jobs::EXIT_WRITE_WAIT`).
pub struct Ran {
    pub(super) outcome: io::Result<Exit>,
    pub(super) pool: Option<Pool>,
}

impl Ran {
    /// A place that ran no jobs (the empty state).
    pub(super) fn without_jobs(outcome: io::Result<Exit>) -> Self {
        Self {
            outcome,
            pool: None,
        }
    }

    /// Stop the pool now, with the terminal still Graft's: the place is
    /// being left for another on the same screen (`Exit::Open`, or into or
    /// out of the practice tour). What the
    /// shutdown reports is dropped, as it was before 13b's review fix: no
    /// line can be printed while the next place owns the screen.
    pub(super) fn stop_for_the_next_place(self) -> io::Result<()> {
        match self.pool {
            Some(pool) => pool.stop(Ending::NextPlace, None).map(drop),
            None => Ok(()),
        }
    }
}

/// Run Graft on one repository until the user quits or a signal says stop:
/// open the terminal, run, give it back. Kept as open-run-close for the
/// gallery and the walks, which call it again and again (each call probes
/// again, as it always did); `main` goes through [`Terminal`] and
/// `places::drive` instead.
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
    let repos = match open_repository(&repo_path, &config.config) {
        Ok(repos) => repos,
        Err(reason) => return Ok(Exit::Fatal(reason)),
    };

    // A stdout that is not a tty — piped, redirected — never gets the
    // alternate screen or a single escape byte (ticket m1-the-graph/19): no
    // probe can tell the other end of a pipe apart from a terminal that
    // simply has not answered yet, so the only safe thing to draw there is
    // plain text (DESIGN.md §4.2 treats an unprobeable terminal as ASCII for
    // the same reason).
    if !io::stdout().is_terminal() {
        return run_headless(&repo_path, &config, repos);
    }

    let mut terminal = Terminal::open()?;
    let arrival = Arrival::first(&config, terminal.capabilities.no_color);
    let ran = run_repo(
        &mut terminal,
        repos,
        &repo_path,
        &config,
        harness,
        arrival,
        None,
    )?;
    terminal.close(ran)
}

/// One repository's run on an open [`Terminal`]: the job pool, the signal
/// and resize watchers, the input forwarder and the loop. Everything it
/// starts it stops before returning, except the terminal itself.
/// `first_launch` is the process's switch, lent by `places::drive` for the
/// loop to end (m8/18e); `None` from `run`, which shows no line.
///
/// # Errors
/// Setting the run up failed; the terminal is still the caller's.
pub(super) fn run_repo(
    term: &mut Terminal,
    repos: Arc<RepoStore>,
    repo_path: &std::path::Path,
    config: &crate::config::Loaded,
    harness: crate::update::Harness,
    arrival: Arrival,
    first_launch: Option<&crate::first_launch::FirstLaunch>,
) -> io::Result<Ran> {
    let inputs = term.inputs();
    let mux = &term.mux;
    let mut settled = Session::settle(config, term.capabilities.clone());

    let (mut wakes, sender) = ChannelWakes::new();
    let signals = signals::watch(sender.clone())?;
    // Read before `repos` moves into the pool: the app's own dir in the
    // common directory (`RepoStore::app_dir`), where the split and `Ctrl+E`'s
    // file live, wanted at startup and on every resize. Its parent is the
    // common directory `git var GIT_EDITOR` is asked in (`listen::outside`).
    let app_dir = repos.app_dir().ok();
    // The worktree root, for the directory `$EDITOR` is started in and the
    // root a porcelain path is joined onto (m3/05 review, Major 2).
    let workdir = repos.workdir().ok().flatten();
    // Journal stamps come from the wall clock, always: the runner's default.
    // The session clock (`harness.clock`, `MWR_NOW`) draws ages only; a
    // pin must never write false history into his repository, which `u`,
    // J and the time machine order by stamp (m8/19e §R1).
    let jobs = JobRunner::with_default_size(sender.clone(), repos)?;
    let redetector = start_redetector(&term.environment, mux, &settled, &sender)?;
    let resizes = signals::watch_resizes(sender.clone(), redetector.clone())?;
    let Listening {
        forwarder,
        mut tracer,
        out,
        fences,
    } = listen(mux, inputs, sender, redetector, &settled)?;

    let (mut terminal, mut state) = terminal_and_state(
        out,
        &jobs,
        repo_path,
        &settled,
        harness,
        &config.config,
        app_dir.as_deref(),
    )?;
    let undo = arrival.keymap.spell(&UNDO).0;
    arrive(&mut state, arrival);
    let mut graphics = Graphics::new(mux.waiter(), &settled.capabilities)
        .with_transport(Transport::for_tmux(&settled.capabilities.tmux));
    let outcome = pump_with(
        Screen {
            terminal: &mut terminal,
            graphics: &mut graphics,
        },
        &mut wakes,
        &jobs,
        &mut settled,
        state,
        outside(
            &term.session,
            mux,
            app_dir.as_deref(),
            workdir.as_deref(),
            first_launch,
        ),
        &mut tracer,
    );
    // Answers from here on are for nobody; they go back to being replies.
    drop(fences);
    let outcome = hand_back_input(mux, forwarder, wakes, outcome);
    // Left for another place on this screen (m8/19b: into or out of the
    // tour, or an open): that place's first frame only draws cells that
    // differ from blank, so the screen is blanked first, as the empty state
    // does before a repository (`places::run_outside`) — without it the card
    // and the palette stayed on his graph after the tour. The backend's
    // clear, not `Terminal::clear`, which asks the terminal where its cursor
    // is and waits on stdin the mux owns.
    let outcome = match outcome {
        Ok(exit @ (Exit::Open(_) | Exit::Practise { .. } | Exit::LeaveTour { .. })) => {
            ratatui::backend::Backend::clear(terminal.backend_mut()).map(|()| exit)
        }
        other => other,
    };
    // Put away before the restore so that its own teardown (it shows the
    // cursor it hid) goes out while the session is still in charge of the
    // terminal.
    put_away(terminal);

    // The pool is stopped by the caller, **after** the terminal is restored
    // (m8/13b review fix): its wait for a write that cannot stop half-way
    // can last up to `jobs::EXIT_WRITE_WAIT`, and a screen held in raw mode
    // that long looks frozen. A job still writing its result meanwhile
    // writes to a channel nobody reads, never to the screen. The watchers
    // stay up with it, so a second signal during that wait is heard.
    Ok(Ran {
        outcome,
        pool: Some(Pool::new(jobs, signals, resizes, undo)),
    })
}

#[cfg(test)]
mod tests;
