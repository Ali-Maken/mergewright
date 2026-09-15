//! Sets up the terminal and runs the interactive loop, or hands off to
//! [`super::headless`] when stdout is not a terminal.

use std::io::{self, IsTerminal, Write};
use std::sync::Arc;
use std::sync::mpsc::Sender;

use graft_core::capabilities::Supported;
use graft_term::Transport;
use graft_term::detect::{self, TtyHost};
use graft_term::input::InputMux;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use super::graphics::Graphics;
use super::headless::run_headless;
use super::redetect::Redetector;
use super::row_clear::RowClear;
use super::running::pump_with;
use super::{ChannelWakes, Exit, Wake, input, opening_repository, signals};
use crate::crash;
use crate::jobs::{self, JobRunner, Shutdown};
use crate::repos::RepoStore;
use crate::session::{Session, Startup};
use crate::terminal::{SessionOptions, TerminalSession};
use crate::update::Size;

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
        Size {
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
