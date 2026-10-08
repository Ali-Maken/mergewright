//! Sets up the terminal and runs the interactive loop, or hands off to
//! [`super::headless`] when stdout is not a terminal.

use std::io::{self, IsTerminal, Write};
use std::sync::Arc;
use std::sync::mpsc::Sender;

use graft_core::capabilities::Supported;
use graft_term::Transport;
use graft_term::detect::{self, TtyHost};
use graft_term::input::{FenceAnswers, InputMux, Inputs};
use ratatui::Terminal as Frames;
use ratatui::backend::CrosstermBackend;
use signal_hook::consts::SIGHUP;

use super::exit_wait::{Boxed, Ending, Pool};
use super::graphics::Graphics;
use super::headless::run_headless;
use super::outside::{Handover, Outside};
use super::redetect::Redetector;
use super::row_clear::RowClear;
use super::running::{Screen, pump_with};
use super::{ChannelWakes, Exit, Wake, input, opening_repository, signals};

use crate::crash;
use crate::jobs::{self, JobRunner, Shutdown};
use crate::repos::RepoStore;
use crate::session::{Session, Startup};
use crate::terminal::{SessionOptions, TerminalSession};
use crate::trace::{self, TerminalOut, Tracer};
use crate::update::{AppState, Size};

/// Undo, spelled once per place for the exit line of a write git could not
/// pause (13d): the process has one keymap (ADR-0055), and that line is
/// printed after the loop, and its state, are gone (`exit_wait::Pool`).
const UNDO: graft_ui::keymap::ActionId = graft_ui::keymap::ActionId("app.undo");

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
    /// being left for another on the same screen (`Exit::Open`). What the
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
    let ran = run_repo(&mut terminal, repos, &repo_path, &config, harness, arrival)?;
    terminal.close(ran)
}

/// How a repository's run begins, beyond the repository itself.
#[derive(Debug, Default)]
pub(super) struct Arrival {
    /// Put on the first frame's status row (`recent: not saved · …`).
    pub status: Option<String>,
    /// Opened from the empty state: the cursor lands on `HEAD`'s row
    /// (`update::lands_on_head_at_open`), not the top row.
    pub on_head: bool,
    /// The process's one keymap (`config::keymap_from`, ADR-0055).
    pub keymap: Arc<graft_ui::keymap::Keymap>,
    /// His config's problems in words — on the process's first repository
    /// screen only; empty on every later one.
    pub config_problems: Vec<String>,
    /// The process's first-launch line (m8/18d): counted by `drive` once;
    /// never shown by default (`run`, the gallery, the walks).
    pub first_launch: crate::first_launch::FirstLaunch,
}

impl Arrival {
    /// The first repository screen of a process started on `config`: its
    /// keymap, and every problem with the file — the file's and the keymap's
    /// (`config::keymap_from`), then each contrast floor his `[theme.*]`
    /// colours miss (none under `NO_COLOR`, where no colour is drawn).
    pub(super) fn first(config: &crate::config::Loaded, no_color: bool) -> Self {
        let (keymap, mut config_problems) = crate::config::keymap_from(config);
        config_problems.extend(crate::config::theme_problems(
            &config.config.theme,
            no_color,
        ));
        Self {
            keymap: Arc::new(keymap),
            config_problems,
            ..Self::default()
        }
    }
}

/// One repository's run on an open [`Terminal`]: the job pool, the signal
/// and resize watchers, the input forwarder and the loop. Everything it
/// starts it stops before returning, except the terminal itself.
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
) -> io::Result<Ran> {
    let inputs = term.inputs();
    let mux = &term.mux;
    let mut settled = Session::settle(config, term.capabilities.clone());

    let (mut wakes, sender) = ChannelWakes::new();
    let signals = signals::watch(sender.clone())?;
    // Cloned before `repos` moves into the pool: the common directory (the
    // detail-pane split lives under it, `crate::layout`) is wanted once more
    // below, at startup and on every resize.
    let common_dir = repos.common_dir().ok();
    // The worktree root, for the directory `$EDITOR` is started in and the
    // root a porcelain path is joined onto (m3/05 review, Major 2).
    let workdir = repos.workdir().ok().flatten();
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
        common_dir.as_deref(),
    )?;
    let undo = arrival.keymap.spell(&UNDO).0;
    state.seed_keys(arrival.keymap, arrival.config_problems);
    state.first_launch = arrival.first_launch;
    if let Some(reason) = arrival.status {
        state.status = crate::update::Status::Sticky { reason, keys: None };
    }
    if arrival.on_head {
        crate::update::lands_on_head_at_open(&mut state);
    }
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
            common_dir.as_deref(),
            workdir.as_deref(),
        ),
        &mut tracer,
    );
    // Answers from here on are for nobody; they go back to being replies.
    drop(fences);
    let outcome = hand_back_input(mux, forwarder, wakes, outcome);
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

/// Show the cursor ratatui hid, then drop its `Terminal`.
///
/// ratatui's own `Drop` shows the cursor too, and `eprintln!`s when that
/// fails. After a hang-up the terminal and stderr are the same dead pty:
/// the show fails with `EIO`, `eprintln!` panics on its own failed write,
/// and the process exits 101 here — before the pool is stopped, so git was
/// left running (m8/13g, observed: `failed printing to stderr: Input/output
/// error (os error 5)` from ratatui-core 0.1.2 `terminal.rs:483`, dropped at
/// this call). Shown here instead, where the failure can be handled.
fn put_away(mut terminal: Frames<RowClear<CrosstermBackend<TerminalOut>>>) {
    if terminal.show_cursor().is_err() {
        // Not reported here, and that is safe: the session's restore, which
        // `wind_down` runs next, shows the cursor again and its result is the
        // one the exit acts on. After a hang-up there is nobody to tell, and
        // the next launch shows where git was left.
        terminal.backend_mut().cursor_given_up();
    }
}

/// What the loop may reach outside itself: the repository's directories,
/// and the terminal to hand to `$EDITOR` or a mergetool.
fn outside<'a>(
    session: &'a TerminalSession,
    mux: &'a InputMux,
    common_dir: Option<&'a std::path::Path>,
    workdir: Option<&'a std::path::Path>,
) -> Outside<'a> {
    Outside {
        common_dir,
        handover: Some(Handover {
            session,
            mux,
            // The common directory, not the worktree's own: `crate::layout`
            // already keeps Graft's files there, and the message file is
            // written and read back within one key press.
            git_dir: common_dir,
            workdir,
        }),
    }
}

/// The loop's input side: the key forwarder, the frame trace and the fence
/// listener.
struct Listening {
    forwarder: input::Forwarder,
    tracer: Tracer,
    /// The terminal's writer, metered when tracing.
    out: TerminalOut,
    /// Held for the loop's life; `None` when the terminal is never fenced.
    fences: Option<FenceAnswers>,
}

/// Start forwarding input to the loop. The frame trace (`crate::trace`) is
/// opened first: the forwarder stamps keys only when there is a trace to
/// stamp them for. Fence answers (ADR-0054) wake the loop straight from the
/// reader thread; a terminal that never answered the startup DSR is never
/// fenced.
fn listen(
    mux: &InputMux,
    inputs: Inputs,
    sender: Sender<Wake>,
    redetector: Option<Redetector>,
    settled: &Session,
) -> io::Result<Listening> {
    let trace::Opened {
        tracer,
        meter,
        stamps,
    } = Tracer::from_env();
    let fences = settled
        .capabilities
        .answers_dsr
        .then(|| answer_fences(mux, sender.clone()));
    let forwarder = input::spawn_reader(inputs, sender, redetector, stamps)?;
    Ok(Listening {
        forwarder,
        tracer,
        out: TerminalOut::for_meter(meter),
        fences,
    })
}

/// The run is over: give the terminal back, **then** stop the pool and close
/// the watchers (m8/13b review fix: nothing that can wait longer than
/// `jobs::SHUTDOWN_GRACE` runs before the restore), and pick the error the
/// process exits on.
fn wind_down(
    session: &TerminalSession,
    pool: Option<Pool>,
    outcome: io::Result<Exit>,
) -> io::Result<Exit> {
    // Restore explicitly rather than leaving it to `Drop`, so a restore
    // failure can be returned instead of merely printed.
    let restored = session.restore();
    let ending = if matches!(outcome, Ok(Exit::Signal(_))) {
        Ending::Signalled
    } else {
        Ending::Exited
    };
    let boxed = match &outcome {
        Ok(Exit::StoppedHalfWay { job, what, after }) => Some(Boxed {
            job: *job,
            what,
            after: *after,
        }),
        _ => None,
    };
    let stopped = pool.map_or(Ok(Shutdown::CLEAN), |pool| {
        pool.stop(ending, boxed.as_ref())
    });
    let stalled_report = stopped.and_then(|shutdown| report_stalled_jobs(&shutdown));
    // `finish` already picks which of two possible errors the process exits
    // on; a failure to report stalled jobs is folded in afterwards so it
    // still turns into `ExitCode::FAILURE` (main.rs) instead of vanishing —
    // except after a hang-up, where stderr is the dead terminal: the process
    // dies of the `SIGHUP` either way, and the words have nowhere to go.
    finish(outcome, restored).and_then(|exit| match stalled_report {
        Err(_nowhere_to_go) if exit == Exit::Signal(SIGHUP) => Ok(exit),
        report => report.map(|()| exit),
    })
}

/// Send every fence answer the mux reads to the loop as a [`Wake::Fence`],
/// until the loop has gone.
fn answer_fences(mux: &InputMux, wakes: Sender<Wake>) -> FenceAnswers {
    mux.answer_fences(Box::new(move |answers| {
        (0..answers).all(|_| wakes.send(Wake::Fence).is_ok())
    }))
}

/// This run is done reading (ticket m1-the-graph/35c). A key typed from here
/// on is kept for the next `on_stdin` (a wrapper's next run), and so is any
/// key already forwarded past the loop when it stopped: `]q` typed in one go
/// must step once and then quit, not step and lose the `q`. Joining the
/// forwarder first makes the hand-back complete — it ends once `hold` drops
/// its `Inputs` sender, after forwarding all it had.
pub(super) fn hand_back_input<T>(
    mux: &InputMux,
    forwarder: input::Forwarder,
    wakes: ChannelWakes,
    outcome: io::Result<T>,
) -> io::Result<T> {
    mux.hold();
    let outcome = outcome.and_then(|exit| forwarder.finish().map(|()| exit));
    mux.give_back(wakes.unhandled_input());
    outcome
}

/// The loop's outcome, unless putting the terminal back failed: a terminal
/// left in raw mode is never reported silently.
fn finish(outcome: io::Result<Exit>, restored: io::Result<()>) -> io::Result<Exit> {
    finish_to(&mut io::stderr(), outcome, restored)
}

/// [`finish`], writing the restore-failure detail to `stderr` instead of the
/// real one — the seam a failing writer is plugged into in tests.
fn finish_to(
    stderr: &mut impl Write,
    outcome: io::Result<Exit>,
    restored: io::Result<()>,
) -> io::Result<Exit> {
    match (outcome, restored) {
        (Ok(exit), Ok(())) => Ok(exit),
        // The terminal hung up (m8/13g): there is nothing left to restore,
        // and nobody to tell that the restore was refused. The process dies
        // of the `SIGHUP` as a `kill -HUP` makes it die, never exit 1.
        (Ok(Exit::Signal(SIGHUP)), Err(_nothing_left_to_restore)) => Ok(Exit::Signal(SIGHUP)),
        (Ok(_), Err(restore_error)) => Err(restore_error),
        (Err(loop_error), Ok(())) => Err(loop_error),
        (Err(loop_error), Err(restore_error)) => {
            // Both went wrong. The loop's failure is the one worth returning,
            // but a terminal left in raw mode must never be silent.
            match writeln!(
                stderr,
                "graft: could not restore the terminal: {restore_error}"
            )
            .and_then(|()| stderr.flush())
            {
                // Both outcomes are handled the same way, deliberately:
                // `loop_error` is returned below either way, so the run
                // already exits non-zero. This write is only a best-effort
                // extra detail about `restore_error`; losing it when stderr
                // itself is gone loses detail, not the failure.
                Ok(()) | Err(_) => {}
            }
            Err(loop_error)
        }
    }
}

/// Say so when jobs had to be abandoned, now that the terminal is the user's
/// again.
///
/// Rare, and a bug when it happens: it means a job did not check its
/// cancellation token often enough. Printing it is the only way anyone finds
/// out, since by this point there is no screen left to put it on — so a
/// failure to print it is propagated rather than dropped: the caller folds it
/// into the run's outcome, and the exit status is the only signal left that
/// something was lost.
fn report_stalled_jobs(shutdown: &Shutdown) -> io::Result<()> {
    report_stalled_jobs_to(&mut io::stderr(), shutdown)
}

/// [`report_stalled_jobs`], writing to `stderr` instead of the real one — the
/// seam a failing writer is plugged into in tests.
fn report_stalled_jobs_to(stderr: &mut impl Write, shutdown: &Shutdown) -> io::Result<()> {
    let workers = shutdown.stalled;
    if workers == 0 {
        return Ok(());
    }

    writeln!(
        stderr,
        "graft: {workers} background job(s) did not stop in time and were abandoned"
    )
    .and_then(|()| stderr.flush())
}

/// The terminal (resizes cleared row by row: a whole-screen clear frees Kitty
/// image data in Ghostty, and every resize would retransmit the viewport,
/// ADR-0012) and the first frame's `AppState`: `opening_repository`'s shell,
/// sized from the terminal's own report (nothing else knows it before the
/// first resize), plus the harness banner, the tier the probe settled on, and
/// `[diff]`'s startup values into the pane's live options — same fields the
/// `w`/`[`/`]`/`W` keys change afterwards, just seeded once instead of left
/// at `DetailState::default()`'s. `common_dir` seeds the detail pane's saved
/// split (ADR-0028, `crate::layout::load_split`); `None` when the repository
/// could not be opened for it, in which case the pane starts at its
/// percentage default like a first run would. `[radar]`'s startup values seed
/// `AppState.radar.policy` the same way `[diff]` seeds the pane's diff
/// options — `Policy::default()` otherwise (m2-details-and-diffs/13c;
/// `KNOWN_GAPS` m2-details-and-diffs/13b).
fn terminal_and_state(
    out: TerminalOut,
    jobs: &impl jobs::Dispatch,
    repo_path: &std::path::Path,
    settled: &Session,
    harness: crate::update::Harness,
    config: &crate::config::Config,
    common_dir: Option<&std::path::Path>,
) -> io::Result<(Frames<RowClear<CrosstermBackend<TerminalOut>>>, AppState)> {
    let terminal = Frames::new(RowClear::new(CrosstermBackend::new(out)))?;
    let size = terminal.size()?;
    let size = Size {
        columns: size.width,
        rows: size.height,
    };
    let mut state = opening_repository(jobs, repo_path, size);
    state.settled_tier = Some(settled.rendering.tier);
    state.harness = harness;
    seed_config(&mut state, config);
    if let Some(common_dir) = common_dir
        && let Some(pane_size) = crate::layout::load_split(common_dir)
    {
        state.detail.pane_size = pane_size;
    }
    Ok((terminal, state))
}

/// Seeds `state` with the config file's startup values — `[diff]`'s live
/// pane options, `[radar]`'s policy, `keys.suspend_on_ctrl_z`, and the two
/// `[ui]`/`[graph]` values that move cells (`ui.icons`, `graph.max_lanes`) —
/// split out of `terminal_and_state` so it is testable without a real
/// terminal (m2-details-and-diffs/13c; `KNOWN_GAPS` m2-details-and-diffs/13b).
fn seed_config(state: &mut AppState, config: &crate::config::Config) {
    let (diff, radar, keys) = (config.diff, &config.radar, &config.keys);
    state.detail.options = graft_git::diff::DiffOptions {
        context: diff.context,
        ignore_whitespace: diff.ignore_whitespace,
    };
    state.detail.word_tint = diff.word_tint;
    state.radar.policy = radar.policy();
    state.suspend_on_ctrl_z = keys.suspend_on_ctrl_z;
    // `ui.icons` and `graph.max_lanes` decide how wide a badge is and how
    // wide the graph column is, so they decide where a *drag* has to hit a
    // badge. They are state, not session, for that reason: the frame and
    // the mouse hit-test build one `GraphView` from them
    // (m5-history-and-ghost-preview/33, review M1).
    state.icons = config.ui.icons.glyphs();
    state.max_lanes = config.graph.max_lanes;
    state.strip_shown = config.ui.overview;
}

#[cfg(test)]
mod tests {
    use std::io;

    use graft_core::mistake_radar::MistakeRadarKind;

    use super::{AppState, Exit, SIGHUP, Shutdown, finish_to, report_stalled_jobs_to, seed_config};
    use crate::config::{Radar, RadarSetting};

    /// A writer whose every write fails, standing in for a closed stderr.
    struct FailingWriter;

    impl io::Write for FailingWriter {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("stderr is gone"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("stderr is gone"))
        }
    }

    #[test]
    fn report_stalled_jobs_propagates_a_failing_writer() {
        let shutdown = Shutdown {
            stalled: 2,
            forced: Vec::new(),
        };

        let result = report_stalled_jobs_to(&mut FailingWriter, &shutdown);

        assert!(result.is_err(), "a failing writer must not be swallowed");
    }

    #[test]
    fn report_stalled_jobs_clean_shutdown_never_writes() {
        // `FailingWriter` would fail the call if `Clean` reached the write at
        // all, so this also proves the early return is taken.
        let result = report_stalled_jobs_to(&mut FailingWriter, &Shutdown::CLEAN);

        assert!(result.is_ok());
    }

    /// `KNOWN_GAPS` m2-details-and-diffs/13b: `[radar]` config was never
    /// threaded into `AppState.radar.policy`, so it had no effect. A
    /// non-default setting must change what the policy reports.
    #[test]
    fn a_non_default_radar_setting_reaches_app_state_policy() {
        let mut default_state = AppState::default();
        seed_config(&mut default_state, &crate::config::Config::default());
        assert_eq!(
            default_state.radar.policy,
            graft_ops::mistake_radar::Policy::default(),
            "an all-default [radar] section must leave the default policy in effect"
        );

        let mut state = AppState::default();
        let radar = Radar {
            foxtrot: RadarSetting::Off,
            ..Radar::default()
        };
        seed_config(
            &mut state,
            &crate::config::Config {
                radar,
                ..crate::config::Config::default()
            },
        );

        assert_eq!(
            state.radar.policy.severity(MistakeRadarKind::FoxtrotMerge),
            None,
            "[radar] foxtrot = off must turn the finding off on AppState.radar.policy"
        );
        assert_ne!(
            state.radar.policy, default_state.radar.policy,
            "a non-default [radar] setting must change what the policy reports"
        );
    }

    /// The same mistake the radar setting made, one section along: a config
    /// key nothing threads into `AppState` is a setting that does nothing.
    #[test]
    fn keys_suspend_on_ctrl_z_reaches_app_state() {
        let mut state = AppState::default();
        seed_config(&mut state, &crate::config::Config::default());
        assert!(
            !state.suspend_on_ctrl_z,
            "ADR-0002: Ctrl+Z is undo unless the user says otherwise"
        );

        let mut state = AppState::default();
        seed_config(
            &mut state,
            &crate::config::Config {
                keys: crate::config::Keys {
                    suspend_on_ctrl_z: true,
                    remaps: Vec::new(),
                },
                ..crate::config::Config::default()
            },
        );
        assert!(state.suspend_on_ctrl_z);
    }

    /// m8-polish-and-ship/17 item 5: `ui.overview = true` starts the strip shown.
    #[test]
    fn ui_overview_seeds_strip_shown() {
        let mut state = AppState::default();
        seed_config(&mut state, &crate::config::Config::default());
        assert!(!state.strip_shown, "off by default");

        let mut state = AppState::default();
        let mut config = crate::config::Config::default();
        config.ui.overview = true;
        seed_config(&mut state, &config);
        assert!(state.strip_shown);
    }

    #[test]
    fn finish_with_failing_writer_still_returns_the_loop_error() {
        let loop_error = io::Error::other("loop failed");
        let restore_error = io::Error::other("restore failed");

        // Both the run and the restore failed, and even the stderr note about
        // the restore failure fails to write: the loop's error must still be
        // the one returned, per the comment in `finish_to`.
        let result = finish_to(&mut FailingWriter, Err(loop_error), Err(restore_error));

        let err = result.expect_err("both inputs were errors");
        assert_eq!(err.to_string(), "loop failed");
    }

    /// m8/13g: after a hang-up the restore fails with `EIO` (the terminal
    /// is gone); the exit is still the `SIGHUP`, so `main` re-raises it.
    #[test]
    fn finish_after_a_hangup_dies_of_it_whatever_the_restore_said() {
        let refused = io::Error::from_raw_os_error(5);

        let result = finish_to(&mut FailingWriter, Ok(Exit::Signal(SIGHUP)), Err(refused));

        assert_eq!(result.expect("a hang-up exits"), Exit::Signal(SIGHUP));
    }

    #[test]
    fn finish_reports_restore_failure_when_loop_succeeded() {
        let restore_error = io::Error::other("restore failed");

        let result = finish_to(&mut io::sink(), Ok(Exit::Quit), Err(restore_error));

        let err = result.expect_err("restore failed");
        assert_eq!(err.to_string(), "restore failed");
    }
}
