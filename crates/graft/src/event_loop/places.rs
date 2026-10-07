//! Where Graft is: a repository, or outside one (m8-polish-and-ship/08).
//!
//! `main` [`locate`]s the path it was given before touching the terminal,
//! opens one [`Terminal`], and hands both to [`drive`], which runs one place
//! after another on that terminal: the empty state, then the repository it
//! opened. The alternate screen is entered once and left once.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use graft_core::capabilities::RenderingTier;
use graft_core::event::Event;
use ratatui::backend::CrosstermBackend;

use super::startup::{Arrival, Ran, Terminal, hand_back_input, run_repo};
use super::{ChannelWakes, Exit, Wake, Wakes, input, signals};
use crate::config::Loaded;
use crate::empty_state::{OutsideState, OutsideStep, outside_edge, outside_paste, render};
use crate::recent::{self, Recent};
use crate::repos::RepoStore;
use crate::session::Session;
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
    // One keymap for the process, every screen it opens (ADR-0055). Config
    // problems are said on the first repository screen of the process, not
    // again after a switch (ticket 08 Scope); `take` leaves later ones none.
    let Arrival {
        keymap,
        config_problems: mut problems,
        ..
    } = Arrival::first(config, term.capabilities.no_color);
    // The first place is where Graft was started; every later one was
    // opened, and an open lands on `HEAD` (note Lands row "Open / init
    // succeeds").
    let mut opened = false;
    loop {
        match place {
            Place::Repo { repos, path } => {
                let status = remember_opened(config_dir.as_deref(), &repos, &path);
                let arrival = Arrival {
                    status,
                    on_head: opened,
                    keymap: keymap.clone(),
                    config_problems: std::mem::take(&mut problems),
                };
                let ran = run_repo(&mut term, repos, &path, config, Harness::default(), arrival)?;
                match ran.outcome {
                    Ok(Exit::Open(next)) => {
                        Ran {
                            outcome: Ok(Exit::Quit),
                            pool: ran.pool,
                        }
                        .stop_for_the_next_place()?;
                        opened = true;
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
            Place::Outside(start) => {
                let shown = Shown {
                    keymap: &keymap,
                    problems: &problems,
                };
                match run_outside(&mut term, start, config, shown, config_dir.as_deref()) {
                    Ok(Outcome::Opened(next)) => {
                        opened = true;
                        place = next;
                    }
                    Ok(Outcome::Exit(exit)) => return term.close(Ran::without_jobs(Ok(exit))),
                    Err(error) => return term.close(Ran::without_jobs(Err(error))),
                }
            }
        }
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
#[derive(Clone, Copy)]
struct Shown<'a> {
    keymap: &'a std::sync::Arc<graft_ui::keymap::Keymap>,
    problems: &'a [String],
}

/// How the empty state ended.
enum Outcome {
    /// A repository was opened (by `⏎`, `o` or `i`).
    Opened(Place),
    /// Graft is leaving.
    Exit(Exit),
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
) -> io::Result<Outcome> {
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
        problem: start.problem.or(said),
        palette: None,
        config_problems: shown.problems.to_vec(),
    };

    let inputs = term.inputs();
    let (mut wakes, sender) = ChannelWakes::new();
    let signals = signals::watch(sender.clone())?;
    let resizes = signals::watch_resizes(sender.clone(), None)?;
    let forwarder = input::spawn_reader(inputs, sender, None, None)?;
    let mut screen = ratatui::Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let outcome = outside_loop(
        &mut screen,
        &mut wakes,
        &mut state,
        (&mut settled, ascii),
        config,
        shown.keymap,
    );
    signals.close();
    resizes.close();
    let outcome = hand_back_input(&term.mux, forwarder, wakes, outcome)?;
    if matches!(outcome, Outcome::Opened(_)) {
        // The repository's screen starts from a blank one: its first frame
        // only draws cells that differ from blank. The backend's clear, not
        // `ratatui::Terminal::clear`: that one asks the terminal where its
        // cursor is (`ESC [6n`) and waits on stdin for the answer, which the
        // input mux owns — it hung the switch.
        ratatui::backend::Backend::clear(screen.backend_mut())?;
    }
    Ok(outcome)
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

fn outside_loop(
    screen: &mut Screen,
    wakes: &mut ChannelWakes,
    state: &mut OutsideState,
    (settled, ascii): (&mut Session, bool),
    config: &Loaded,
    keymap: &std::sync::Arc<graft_ui::keymap::Keymap>,
) -> io::Result<Outcome> {
    loop {
        let now = SystemTime::now();
        screen.draw(|frame| {
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
            // A resize redraws (ratatui resizes on `draw`); focus, mouse,
            // fences and re-detection answers change nothing here — no
            // re-detector or fence listener runs on this screen.
            Wake::Event(_) | Wake::Terminal(_) | Wake::Fence => OutsideStep::Stay,
        };
        match step {
            OutsideStep::Stay => {}
            OutsideStep::Quit => return Ok(Outcome::Exit(Exit::Quit)),
            // The next pass round this loop draws in it, changed or not.
            // This screen draws no pixels, so there are no strips to flush
            // (`turn::rethemed` does that in a repository).
            OutsideStep::Retheme(kind) => {
                settled.choose(kind);
            }
            OutsideStep::Open(path) => {
                if let Some(place) = try_open(state, &path, "open", config) {
                    return Ok(Outcome::Opened(place));
                }
            }
            OutsideStep::Init(folder) => match graft_ops::init::init_here(&folder) {
                Ok(made) => {
                    if let Some(place) = try_open(state, &made, "init", config) {
                        return Ok(Outcome::Opened(place));
                    }
                }
                Err(error) => state.problem = Some(error.to_string()),
            },
        }
    }
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
