//! The empty-state place's loop: the screen run outside a repository, the
//! pool its clone runs on, the steps it carries out, and how it hands on to
//! `drive`. Moved out of `places.rs` (m8/18e B1).

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::time::SystemTime;

use graft_core::capabilities::RenderingTier;
use graft_core::event::Event;
use graft_core::job::JobId;
use graft_git::run::askpass::Secret;
use ratatui::backend::CrosstermBackend;

use super::super::exit_wait::Pool;
use super::super::startup::{Ran, Terminal, UNDO, hand_back_input};
use super::super::{ChannelWakes, Exit, Open, Wake, Wakes, input, is_job_event, signals};
use super::{Carried, OutsideStart, Place, joined, locate};
use crate::config::Loaded;
use crate::empty_state::{
    OutsideState, OutsideStep, clone_started, outside_edge, outside_job, outside_paste, render,
};
use crate::first_launch::{self, FirstLaunch};
use crate::jobs::JobRunner;
use crate::jobs::clone::CloneRequest;
use crate::recent::{self, Recent};
use crate::repos::RepoStore;
use crate::session::Session;
use crate::update::AppEvent;
use crate::update::credential::Answer;
use crate::view::rows::home_relative;

/// Where `drive` goes once the empty state has ended.
pub(super) enum Next {
    /// On to this place.
    Place(Place),
    /// Into the practice tour, coming back to this (m8/19b).
    Practise(Open),
    /// Graft ends: close the terminal on this.
    End(Box<Ran>),
}

/// How the empty state ended, for `drive`: a repository opened goes on, with
/// the pool stopped now (a clone's, its job done: at once) and the notice
/// taken on; anything else ends the process with the pool, if any, stopped
/// once the terminal is back (`wind_down`) — a clone still running among
/// it, stoppable, so with no waiting line (rulings R4).
pub(super) fn leave_outside(left: io::Result<Left>, carried: &mut Carried) -> io::Result<Next> {
    match left {
        Ok(Left {
            outcome: Ok(Outcome::Opened { place, notice }),
            pool,
        }) => {
            Ran {
                outcome: Ok(Exit::Quit),
                pool,
            }
            .stop_for_the_next_place()?;
            carried.notice = notice;
            Ok(Next::Place(place))
        }
        Ok(Left {
            outcome: Ok(Outcome::Practise { from }),
            pool,
        }) => {
            Ran {
                outcome: Ok(Exit::Quit),
                pool,
            }
            .stop_for_the_next_place()?;
            Ok(Next::Practise(from))
        }
        Ok(Left {
            outcome: Ok(Outcome::Exit(exit)),
            pool,
        }) => Ok(Next::End(Box::new(Ran {
            outcome: Ok(exit),
            pool,
        }))),
        Ok(Left {
            outcome: Err(error),
            pool,
        }) => Ok(Next::End(Box::new(Ran {
            outcome: Err(error),
            pool,
        }))),
        Err(error) => Ok(Next::End(Box::new(Ran::without_jobs(Err(error))))),
    }
}

/// What the empty state shows besides its folder: the keys, and his
/// config's problems (the palette's `YOUR CONFIG`; still said on the first
/// repository screen after).
pub(super) struct Shown<'a> {
    pub(super) keymap: &'a std::sync::Arc<graft_ui::keymap::Keymap>,
    pub(super) problems: &'a [String],
    /// The process's first-launch line.
    pub(super) first_launch: &'a FirstLaunch,
    /// The count could not be read or written: said once, in the bar row.
    pub(super) warning: Option<String>,
}

/// How the empty state ended.
pub(super) enum Outcome {
    /// A repository was opened (by `⏎`, `o`, `i` or a clone).
    Opened {
        place: Place,
        /// Said once on its first frame (a clone's askpass close failure).
        notice: Option<String>,
    },
    /// `t` or the palette's `Practice tour`: into the practice tour,
    /// coming back here (m8/19b).
    Practise { from: Open },
    /// Graft is leaving.
    Exit(Exit),
}

/// What the empty state leaves: how it ended, and its job pool when a clone
/// started one (rulings R4), to stop as a repository's is stopped.
pub(super) struct Left {
    outcome: io::Result<Outcome>,
    pool: Option<Pool>,
}

/// The empty state's job pool: none until the first clone (rulings R4), so
/// a launch that never clones starts no threads.
pub(super) struct OutsideJobs {
    pub(super) pool: Option<JobRunner<Sender<Wake>>>,
    /// The loop's own channel, for the pool's answers (as `run_repo` wires
    /// its pool, `startup.rs`).
    sender: Sender<Wake>,
}

impl OutsideJobs {
    pub(super) fn new(sender: Sender<Wake>) -> Self {
        Self { pool: None, sender }
    }

    /// The pool, started now if it is not yet. One worker: one clone runs
    /// at a time.
    fn started(&mut self, folder: &Path) -> io::Result<&JobRunner<Sender<Wake>>> {
        let pool = match self.pool.take() {
            Some(pool) => pool,
            // A clone reads no repository: the store is never opened.
            None => JobRunner::start(
                self.sender.clone(),
                1,
                Arc::new(RepoStore::new(folder.to_path_buf(), None)),
            )?,
        };
        Ok(self.pool.insert(pool))
    }

    /// The pool and both watchers as a [`Pool`] to stop later, or the
    /// watchers closed now when no pool was started.
    pub(super) fn into_pool(
        self,
        signals: signals::Watch,
        resizes: signal_hook::iterator::Handle,
        undo: String,
    ) -> Option<Pool> {
        if let Some(jobs) = self.pool {
            return Some(Pool::new(jobs, signals, resizes, undo));
        }
        signals.close();
        resizes.close();
        None
    }
}

/// The empty state's loop on `term`: draw, take a key, act, until a
/// repository opens or Graft quits. An open or init that fails stays here,
/// with the reason in the bar row and the cursor where it was: the screen is
/// never left, so it visibly stays (decision 4, done inside the loop rather
/// than by re-entering it, which would clear and redraw).
pub(super) fn run_outside(
    term: &mut Terminal,
    start: OutsideStart,
    config: &Loaded,
    shown: Shown<'_>,
    config_dir: Option<&Path>,
) -> io::Result<Left> {
    let mut settled = Session::settle(config, term.capabilities.clone());
    let ascii = settled.rendering.tier == RenderingTier::Ascii;
    let (recents, said) = read_recents(config_dir);
    let cursor = start.cursor.min(recents.len().saturating_sub(1));
    let mut state = OutsideState {
        folder: start.folder,
        missing: start.missing,
        home: std::env::var_os("HOME").map(PathBuf::from),
        recents,
        cursor,
        prompt: None,
        problem: start.problem.or(said).or(shown.warning),
        palette: None,
        config_problems: shown.problems.to_vec(),
        first_launch: shown.first_launch.line(),
        credential: None,
    };

    let inputs = term.inputs();
    let (mut wakes, sender) = ChannelWakes::new();
    let signals = signals::watch(sender.clone())?;
    let resizes = signals::watch_resizes(sender.clone(), None)?;
    let mut jobs = OutsideJobs::new(sender.clone());
    let forwarder = input::spawn_reader(inputs, sender, None, None)?;
    let mut screen = ratatui::Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let outcome = outside_loop(
        &mut screen,
        &mut wakes,
        &mut state,
        (&mut settled, ascii),
        (config, &mut jobs, shown.first_launch),
        shown.keymap,
    );
    let pool = jobs.into_pool(signals, resizes, shown.keymap.spell(&UNDO).0);
    let outcome = hand_back_input(&term.mux, forwarder, wakes, outcome).and_then(|outcome| {
        if matches!(outcome, Outcome::Opened { .. }) {
            // The repository's screen starts from a blank one: its first
            // frame only draws cells that differ from blank. The backend's
            // clear, not `ratatui::Terminal::clear`: that one asks the
            // terminal where its cursor is (`ESC [6n`) and waits on stdin
            // for the answer, which the input mux owns — it hung the switch.
            ratatui::backend::Backend::clear(screen.backend_mut())?;
        }
        Ok(outcome)
    });
    Ok(Left { outcome, pool })
}

/// The recent list, and what to say about it once.
pub(super) fn read_recents(config_dir: Option<&Path>) -> (Vec<Recent>, Option<String>) {
    let Some(config_dir) = config_dir else {
        return (Vec::new(), Some("recent: no config folder".to_owned()));
    };
    match recent::read_recent(config_dir) {
        Ok(list) if list.skipped > 0 => (
            list.entries,
            Some(format!("recent: {} unreadable lines skipped", list.skipped)),
        ),
        Ok(list) => (list.entries, None),
        Err(error) => (Vec::new(), Some(format!("recent: {error}"))),
    }
}

pub(super) type Screen = ratatui::Terminal<CrosstermBackend<io::Stdout>>;

/// One frame as one synchronized update (ADR-0054 §3, DESIGN.md §4.1), as
/// the repository's frames are (`running::frame::draw_frame`): a frame
/// reaches the terminal in pieces, so without the bracket it can show the
/// top of a frame over the bottom of the last one (13h). The end marker goes out
/// even when the draw failed: a terminal left inside an update freezes on
/// what it showed before.
pub(super) fn draw_whole(
    screen: &mut Screen,
    render: impl FnOnce(&mut ratatui::Frame<'_>),
) -> io::Result<()> {
    use std::io::Write;
    screen.backend_mut().write_all(graft_term::SYNC_BEGIN)?;
    let drawn = screen.draw(render).map(|_| ());
    let backend = screen.backend_mut();
    let ended = backend
        .write_all(graft_term::SYNC_END)
        .and_then(|()| Write::flush(backend));
    drawn.and(ended)
}

pub(super) fn outside_loop(
    screen: &mut Screen,
    wakes: &mut ChannelWakes,
    state: &mut OutsideState,
    (settled, ascii): (&mut Session, bool),
    (config, jobs, line): (&Loaded, &mut OutsideJobs, &FirstLaunch),
    keymap: &std::sync::Arc<graft_ui::keymap::Keymap>,
) -> io::Result<Outcome> {
    loop {
        let now = SystemTime::now();
        draw_whole(screen, |frame| {
            let area = frame.area();
            render(
                state,
                keymap,
                &settled.theme,
                ascii,
                now,
                area,
                frame.buffer_mut(),
            );
        })?;
        let Some(wake) = wakes.next_wake() else {
            return Err(io::Error::other("the terminal's input ended"));
        };
        let step = match wake {
            Wake::Event(AppEvent::Core(Event::Key(key))) => outside_edge(state, keymap, key),
            Wake::Event(AppEvent::Core(Event::Paste(text))) => {
                outside_paste(state, keymap, &text);
                OutsideStep::Stay
            }
            Wake::Shutdown(signal) => return Ok(Outcome::Exit(Exit::Signal(signal))),
            // The terminal hung up: the same exit as its `SIGHUP` (13g).
            Wake::InputFailed(error) if input::hung_up(&error) => {
                return Ok(Outcome::Exit(Exit::Signal(signal_hook::consts::SIGHUP)));
            }
            Wake::InputFailed(error) | Wake::TerminalFailed(error) => return Err(error),
            // The clone's progress, its question and its answer (m8/18b).
            wake @ Wake::Event(_) if is_job_event(&wake) => outside_job(state, &wake),
            // A resize redraws (ratatui resizes on `draw`); focus, mouse,
            // fences and re-detection answers change nothing here — no
            // re-detector or fence listener runs on this screen.
            Wake::Event(_) | Wake::Terminal(_) | Wake::Fence => OutsideStep::Stay,
        };
        match step {
            // The next pass round this loop draws in it, changed or not.
            // This screen draws no pixels, so there are no strips to flush
            // (`turn::rethemed` does that in a repository).
            OutsideStep::Retheme(kind) => {
                settled.choose(kind);
            }
            step => {
                if let Some(outcome) = carry_out(step, state, jobs, config, line) {
                    return Ok(outcome);
                }
            }
        }
    }
}

/// Carry out what a key or a job asked for, but a retheme (the loop's: it
/// holds the session). `Some` when the empty state is over.
pub(super) fn carry_out(
    step: OutsideStep,
    state: &mut OutsideState,
    jobs: &mut OutsideJobs,
    config: &Loaded,
    line: &FirstLaunch,
) -> Option<Outcome> {
    match step {
        OutsideStep::Stay | OutsideStep::Retheme(_) => {}
        OutsideStep::Quit => return Some(Outcome::Exit(Exit::Quit)),
        OutsideStep::Practise => {
            return Some(Outcome::Practise {
                from: Open::at(state.folder.clone()),
            });
        }
        OutsideStep::Open(path) => {
            return try_open(state, &path, "open", config).map(|place| Outcome::Opened {
                place,
                notice: None,
            });
        }
        OutsideStep::Init(folder) => return init_and_open(state, &folder, config),
        OutsideStep::StartClone(request) => start_clone(state, jobs, request),
        OutsideStep::StopClone(job) => {
            if let Some(pool) = &jobs.pool {
                pool.cancel(job);
            }
        }
        // No pool, no job: nothing is waiting for an answer.
        OutsideStep::Credential(job, answer) => {
            if let Some(pool) = &jobs.pool {
                answer_ask(pool, job, answer);
            }
        }
        // The switch first, as the repository's loop does: ended once per
        // process (it names the folder the state was seeded with), so no
        // later place shows the line; already ended, nothing to write.
        OutsideStep::FirstLaunchDone(_seeded_from) => end_first_launch(state, line),
        OutsideStep::Cloned { path, notice } => {
            match try_open(state, &path, "clone", config) {
                Some(place) => return Some(Outcome::Opened { place, notice }),
                // The open's reason first, then the notice: neither dropped.
                None => state.problem = joined(state.problem.take(), notice),
            }
        }
    }
    None
}

/// `i`: `git init` the folder, then open it; a failure is said in the bar
/// row.
fn init_and_open(state: &mut OutsideState, folder: &Path, config: &Loaded) -> Option<Outcome> {
    match graft_ops::init::init_here(folder) {
        Ok(made) => try_open(state, &made, "init", config).map(|place| Outcome::Opened {
            place,
            notice: None,
        }),
        Err(error) => {
            state.problem = Some(error.to_string());
            None
        }
    }
}

/// Run the clone on the empty state's pool, started now if it is not yet.
fn start_clone(state: &mut OutsideState, jobs: &mut OutsideJobs, request: CloneRequest) {
    match jobs.started(&state.folder) {
        Ok(pool) => {
            let into = request.into.clone();
            let job = pool.clone_repo(request);
            clone_started(state, job, into);
        }
        Err(error) => {
            state.problem = Some(format!("clone: could not start a job \u{b7} {error}"));
        }
    }
}

/// End the shell's first-launch switch and write `done`; a failed write is
/// said in the bar row.
fn end_first_launch(state: &mut OutsideState, line: &FirstLaunch) {
    if let Some(config_dir) = line.end()
        && let Err(error) = first_launch::first_launch_done(&config_dir)
    {
        state.problem = Some(error.words());
    }
}

/// The two pool calls the password box's answer makes, behind a seam so
/// their order is tested (m8/18e A3): on the real pool a swap only loses a
/// race, which no test can be relied on to see.
pub(super) trait AnswersAsks {
    fn cancel(&self, job: JobId);
    fn answer_credential(&self, job: JobId, answer: Option<Secret>) -> bool;
}

impl AnswersAsks for JobRunner<Sender<Wake>> {
    fn cancel(&self, job: JobId) {
        JobRunner::cancel(self, job);
    }

    fn answer_credential(&self, job: JobId, answer: Option<Secret>) -> bool {
        JobRunner::answer_credential(self, job, answer)
    }
}

/// The box's answer to git's ask for `job`; `None` is `esc`, which stops
/// the clone (rulings R6).
pub(super) fn answer_ask(pool: &impl AnswersAsks, job: JobId, answer: Option<Answer>) {
    let secret = answer.map(Answer::into_secret);
    if secret.is_none() {
        // The job goes first, then the ask is refused: the other order lets
        // git die of the refusal before the stop lands, and that reads as a
        // wrong password (`update::credential::credential_key`).
        pool.cancel(job);
    }
    // `false`: the job ended while the key was in flight, and the answer is
    // dropped there and then, zeroed — nobody is left to tell (as
    // `JobRunner::credential`).
    let _delivered: bool = pool.answer_credential(job, secret);
}

/// Open `path` if it is a repository; otherwise put the reason in the bar
/// row (`<verb>: …`) and stay.
pub(super) fn try_open(
    state: &mut OutsideState,
    path: &Path,
    verb: &str,
    config: &Loaded,
) -> Option<Place> {
    let shown = home_relative(path, state.home.as_deref());
    match locate(path, config) {
        Ok(place @ Place::Repo { .. }) => return Some(place),
        Ok(Place::Outside(OutsideStart { missing: true, .. })) => {
            state.problem = Some(format!("{verb}: {} · no such folder", shown.display()));
        }
        Ok(Place::Outside(_)) => {
            state.problem = Some(format!(
                "{verb}: {} is not inside a git repository",
                shown.display()
            ));
        }
        Err(reason) => state.problem = Some(format!("{verb}: {reason}")),
    }
    None
}
