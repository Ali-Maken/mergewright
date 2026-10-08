//! Where Graft is: a repository, or outside one (m8-polish-and-ship/08).
//!
//! `main` [`locate`]s the path it was given before touching the terminal,
//! opens one [`Terminal`], and hands both to [`drive`], which runs one place
//! after another on that terminal: the empty state, then the repository it
//! opened. The alternate screen is entered once and left once.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::time::SystemTime;

use graft_core::capabilities::RenderingTier;
use graft_core::event::Event;
use ratatui::backend::CrosstermBackend;

use super::exit_wait::Pool;
use super::startup::{Arrival, Ran, Terminal, UNDO, hand_back_input, run_repo};
use super::{ChannelWakes, Exit, Wake, Wakes, input, is_job_event, signals};
use crate::config::Loaded;
use crate::empty_state::{
    OutsideState, OutsideStep, clone_started, outside_edge, outside_job, outside_paste, render,
};
use crate::first_launch::{self, FirstLaunch};
use crate::jobs::JobRunner;
use crate::recent::{self, Recent};
use crate::repos::RepoStore;
use crate::session::Session;
use crate::update::credential::Answer;
use crate::update::{AppEvent, Harness};
use crate::view::rows::home_relative;

/// A place Graft can be.
pub enum Place {
    /// A repository, already opened (`RepoStore::shared` answered).
    Repo {
        /// The open repository.
        repos: Arc<RepoStore>,
        /// The path it was found from.
        path: PathBuf,
    },
    /// Outside any repository: the empty state.
    Outside(OutsideStart),
}

/// What the empty state starts from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutsideStart {
    /// The folder shown, absolute.
    pub folder: PathBuf,
    /// It does not exist.
    pub missing: bool,
    /// The RECENT row selected.
    pub cursor: usize,
    /// Read in the bar row until the next key.
    pub problem: Option<String>,
}

/// Decide where `path` is, opening the repository if it is in one. Called
/// before the terminal is touched, so a broken repository is still a plain
/// line on a plain terminal (decision 2).
///
/// # Errors
/// `path` is in a repository that cannot be opened, or cannot be read — a
/// broken repository is not "no repository here". The reason to print.
pub fn locate(path: &Path, config: &Loaded) -> Result<Place, String> {
    let absolute =
        std::path::absolute(path).map_err(|error| format!("{}: {error}", path.display()))?;
    if !absolute.exists() {
        return Ok(Place::Outside(OutsideStart {
            folder: absolute,
            missing: true,
            cursor: 0,
            problem: None,
        }));
    }
    let repos = Arc::new(RepoStore::new(
        path.to_path_buf(),
        config.config.graph.default_branch.clone(),
    ));
    match repos.shared() {
        Ok(_) => Ok(Place::Repo {
            repos,
            path: path.to_path_buf(),
        }),
        Err(graft_git::repo::OpenError::NotARepo { .. }) => Ok(Place::Outside(OutsideStart {
            // Canonical, so `.` and `..` read as the folder they are. It
            // exists (checked above); if it cannot be resolved after all
            // (removed in between), the absolute spelling is still a true
            // name for what was asked.
            folder: absolute.canonicalize().unwrap_or(absolute),
            missing: false,
            cursor: 0,
            problem: None,
        })),
        Err(error) => Err(error.to_string()),
    }
}

/// Run places on `term` until one ends Graft, then give the terminal back.
/// Handles [`Exit::Open`] itself (re-locates, runs the next place); never
/// returns it.
///
/// # Errors
/// A place could not be set up, or the terminal could not be restored.
pub fn drive(mut term: Terminal, first: Place, config: &Loaded) -> io::Result<Exit> {
    let config_dir = recent::config_dir();
    let mut place = first;
    let mut carried = Carried::first(config, term.capabilities.no_color, config_dir.as_deref());
    // Every place lands on `HEAD`: the first, where Graft was started, as
    // much as every later one opened from it (note Lands row "Open / init
    // succeeds"; m8-polish-and-ship/12d — a launch is an arrival too).
    loop {
        match place {
            Place::Repo { repos, path } => {
                let arrival =
                    carried.repo_arrival(remember_opened(config_dir.as_deref(), &repos, &path));
                let ran = run_repo(&mut term, repos, &path, config, Harness::default(), arrival)?;
                match ran.outcome {
                    Ok(Exit::Open(next)) => {
                        Ran {
                            outcome: Ok(Exit::Quit),
                            pool: ran.pool,
                        }
                        .stop_for_the_next_place()?;
                        place = match locate(&next, config) {
                            Ok(next) => next,
                            Err(reason) => {
                                return term.close(Ran::without_jobs(Ok(Exit::Fatal(reason))));
                            }
                        };
                    }
                    outcome => {
                        return term.close(Ran {
                            outcome,
                            pool: ran.pool,
                        });
                    }
                }
            }
            Place::Outside(mut start) => {
                // Never a clone's notice (a clone opens a repository); said
                // where this screen says problems all the same.
                start.problem = joined(start.problem, carried.notice.take());
                let shown = Shown {
                    keymap: &carried.keymap,
                    problems: &carried.problems,
                    first_launch: &carried.first_launch,
                    warning: carried.warning.take(),
                };
                let left = run_outside(&mut term, start, config, shown, config_dir.as_deref());
                match leave_outside(left, &mut carried)? {
                    Next::Place(next) => place = next,
                    Next::End(ran) => return term.close(*ran),
                }
            }
        }
    }
}

/// Where `drive` goes once the empty state has ended.
enum Next {
    /// On to this place.
    Place(Place),
    /// Graft ends: close the terminal on this.
    End(Box<Ran>),
}

/// How the empty state ended, for `drive`: a repository opened goes on, with
/// the pool stopped now (a clone's, its job done: at once) and the notice
/// taken on; anything else ends the process with the pool, if any, stopped
/// once the terminal is back (`wind_down`) — a clone still running among
/// it, stoppable, so with no waiting line (rulings R4).
fn leave_outside(left: io::Result<Left>, carried: &mut Carried) -> io::Result<Next> {
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

/// What the process carries from place to place: one keymap (ADR-0055),
/// his config's problems (said on the first repository screen only, ticket
/// 08 Scope), the first-launch line and its warning (R7: counted once, a
/// failure said once), and what the last place handed on.
struct Carried {
    keymap: Arc<graft_ui::keymap::Keymap>,
    problems: Vec<String>,
    first_launch: FirstLaunch,
    warning: Option<String>,
    /// Said once, on the next place's first frame: a clone's notice
    /// (m8/18b). Set only by the empty state's `Outcome::Opened`.
    notice: Option<String>,
}

impl Carried {
    fn first(config: &Loaded, no_color: bool, config_dir: Option<&Path>) -> Self {
        let Arrival {
            keymap,
            config_problems,
            ..
        } = Arrival::first(config, no_color);
        let (first_launch, warning) = first_launch::for_this_launch(config_dir);
        Self {
            keymap,
            problems: config_problems,
            first_launch,
            warning,
            notice: None,
        }
    }

    /// The next repository's arrival: `remembered` (the recent list's
    /// word), else the first-launch warning; the notice handed on, once;
    /// the config's problems, once.
    fn repo_arrival(&mut self, remembered: Option<String>) -> Arrival {
        Arrival {
            status: remembered.or_else(|| self.warning.take()),
            notice: self.notice.take(),
            on_head: true,
            keymap: self.keymap.clone(),
            config_problems: std::mem::take(&mut self.problems),
            first_launch: self.first_launch.clone(),
        }
    }
}

/// Two lines said in one place, the first first.
pub(super) fn joined(first: Option<String>, then: Option<String>) -> Option<String> {
    match (first, then) {
        (Some(first), Some(then)) => Some(format!("{first} \u{b7} {then}")),
        (first, then) => first.or(then),
    }
}

/// Put an opened repository on the recent list; what to say on its first
/// frame when that did not go cleanly.
fn remember_opened(config_dir: Option<&Path>, repos: &RepoStore, path: &Path) -> Option<String> {
    let Some(config_dir) = config_dir else {
        return Some("recent: not saved · no config folder".to_owned());
    };
    // The working tree is what the list shows and opens again; a bare
    // repository (no working tree) is remembered by the path it was opened
    // from. A workdir read failing here was already survived by the open.
    let workdir = repos
        .workdir()
        .ok()
        .flatten()
        .unwrap_or_else(|| path.to_path_buf());
    match recent::remember(config_dir, &workdir, SystemTime::now()) {
        Ok(remembered) if remembered.dropped > 0 => Some(format!(
            "recent: {} unreadable lines dropped",
            remembered.dropped
        )),
        Ok(_) => None,
        Err(error) => Some(format!("recent: not saved · {error}")),
    }
}

/// What the empty state shows besides its folder: the keys, and his
/// config's problems (the palette's `YOUR CONFIG`; still said on the first
/// repository screen after).
struct Shown<'a> {
    keymap: &'a std::sync::Arc<graft_ui::keymap::Keymap>,
    problems: &'a [String],
    /// The process's first-launch line.
    first_launch: &'a FirstLaunch,
    /// The count could not be read or written: said once, in the bar row.
    warning: Option<String>,
}

/// How the empty state ended.
enum Outcome {
    /// A repository was opened (by `⏎`, `o`, `i` or a clone).
    Opened {
        place: Place,
        /// Said once on its first frame (a clone's askpass close failure).
        notice: Option<String>,
    },
    /// Graft is leaving.
    Exit(Exit),
}

/// What the empty state leaves: how it ended, and its job pool when a clone
/// started one (rulings R4), to stop as a repository's is stopped.
struct Left {
    outcome: io::Result<Outcome>,
    pool: Option<Pool>,
}

/// The empty state's job pool: none until the first clone (rulings R4), so
/// a launch that never clones starts no threads.
struct OutsideJobs {
    pool: Option<JobRunner<Sender<Wake>>>,
    /// The loop's own channel, for the pool's answers (as `run_repo` wires
    /// its pool, `startup.rs`).
    sender: Sender<Wake>,
}

impl OutsideJobs {
    fn new(sender: Sender<Wake>) -> Self {
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
    fn into_pool(
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
fn run_outside(
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
        first_launch: shown.first_launch.clone(),
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
        (config, &mut jobs),
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
fn read_recents(config_dir: Option<&Path>) -> (Vec<Recent>, Option<String>) {
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

type Screen = ratatui::Terminal<CrosstermBackend<io::Stdout>>;

/// One frame as one synchronized update (ADR-0054 §3, DESIGN.md §4.1), as
/// the repository's frames are (`running::frame::draw_frame`): a frame
/// reaches the terminal in pieces, so without the bracket it can show the
/// top of a frame over the bottom of the last one (13h). The end marker goes out
/// even when the draw failed: a terminal left inside an update freezes on
/// what it showed before.
fn draw_whole(screen: &mut Screen, render: impl FnOnce(&mut ratatui::Frame<'_>)) -> io::Result<()> {
    use std::io::Write;
    screen.backend_mut().write_all(graft_term::SYNC_BEGIN)?;
    let drawn = screen.draw(render).map(|_| ());
    let backend = screen.backend_mut();
    let ended = backend
        .write_all(graft_term::SYNC_END)
        .and_then(|()| Write::flush(backend));
    drawn.and(ended)
}

fn outside_loop(
    screen: &mut Screen,
    wakes: &mut ChannelWakes,
    state: &mut OutsideState,
    (settled, ascii): (&mut Session, bool),
    (config, jobs): (&Loaded, &mut OutsideJobs),
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
                if let Some(outcome) = carry_out(step, state, jobs, config) {
                    return Ok(outcome);
                }
            }
        }
    }
}

/// Carry out what a key or a job asked for, but a retheme (the loop's: it
/// holds the session). `Some` when the empty state is over.
fn carry_out(
    step: OutsideStep,
    state: &mut OutsideState,
    jobs: &mut OutsideJobs,
    config: &Loaded,
) -> Option<Outcome> {
    match step {
        OutsideStep::Stay | OutsideStep::Retheme(_) => {}
        OutsideStep::Quit => return Some(Outcome::Exit(Exit::Quit)),
        OutsideStep::Open(path) => {
            return try_open(state, &path, "open", config).map(|place| Outcome::Opened {
                place,
                notice: None,
            });
        }
        OutsideStep::Init(folder) => match graft_ops::init::init_here(&folder) {
            Ok(made) => {
                return try_open(state, &made, "init", config).map(|place| Outcome::Opened {
                    place,
                    notice: None,
                });
            }
            Err(error) => state.problem = Some(error.to_string()),
        },
        OutsideStep::StartClone(request) => match jobs.started(&state.folder) {
            Ok(pool) => {
                let into = request.into.clone();
                let job = pool.clone_repo(request);
                clone_started(state, job, into);
            }
            Err(error) => {
                state.problem = Some(format!("clone: could not start a job \u{b7} {error}"));
            }
        },
        OutsideStep::StopClone(job) => {
            if let Some(pool) = &jobs.pool {
                pool.cancel(job);
            }
        }
        // No pool, no job: nothing is waiting for an answer.
        OutsideStep::Credential(job, answer) => {
            if let Some(pool) = &jobs.pool {
                // `false`: the job ended while the key was in flight, and the
                // answer is dropped there and then, zeroed — nobody is left
                // to tell (as `JobRunner::credential`).
                let secret = answer.map(Answer::into_secret);
                if secret.is_none() {
                    // The job goes first, then the ask is refused: the other
                    // order lets git die of the refusal before the stop
                    // lands, and that reads as a wrong password
                    // (`update::credential::credential_key`).
                    pool.cancel(job);
                }
                let _delivered: bool = pool.answer_credential(job, secret);
            }
        }
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

/// Open `path` if it is a repository; otherwise put the reason in the bar
/// row (`<verb>: …`) and stay.
fn try_open(state: &mut OutsideState, path: &Path, verb: &str, config: &Loaded) -> Option<Place> {
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

#[cfg(test)]
mod tests;
