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

use super::Exit;
use super::startup::{Arrival, Ran, Terminal, run_repo};
use crate::config::Loaded;
use crate::first_launch::{self, FirstLaunch};
use crate::recent;
use crate::repos::RepoStore;
use crate::session::Clock;
use crate::update::Harness;

mod outside;
mod practice;

use outside::{Next, Shown, leave_outside, run_outside};
use practice::{Practice, end, moved, practise};
// Only the tests reach these through `places`; the children use them.
#[cfg(test)]
use super::signals;
#[cfg(test)]
use outside::{AnswersAsks, Outcome, OutsideJobs, answer_ask, carry_out};
#[cfg(test)]
use practice::practice_at;

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
/// Handles [`Exit::Open`], [`Exit::Practise`] and [`Exit::LeaveTour`]
/// itself (runs the next place); never returns them.
///
/// The practice tour's repository (m8-polish-and-ship/19b, ticket Amendment
/// §A8) is held here, in one slot, from the moment it is built until it is
/// removed: on leaving the tour, on opening anything else from inside it,
/// and once the terminal is back when Graft ends inside it.
///
/// # Errors
/// A place could not be set up, or the terminal could not be restored.
pub fn drive(
    mut term: Terminal,
    mut place: Place,
    config: &Loaded,
    clock: Clock,
) -> io::Result<Exit> {
    let config_dir = recent::config_dir();
    let no_color = term.capabilities.no_color;
    let mut carried = Carried::first(config, no_color, config_dir.as_deref(), clock);
    let mut practice: Option<Practice> = None;
    // Every place lands on `HEAD`: the first, where Graft was started, as
    // much as every later one opened from it (note Lands row "Open / init
    // succeeds"; m8-polish-and-ship/12d — a launch is an arrival too). The
    // way back from the tour lands on the commit he left from (§A4).
    loop {
        let went = match place {
            Place::Repo { repos, path } => {
                // The practice repository is never remembered (§A6): it is
                // gone when the tour is, and a recent row would open nothing.
                // Nor is the way back: he was already here, and the tour
                // leaves his state folder byte-for-byte as it found it.
                let returning = std::mem::take(&mut carried.returning);
                let remembered = if practice.is_some() || returning {
                    None
                } else {
                    remember_opened(config_dir.as_deref(), &repos, &path)
                };
                let arrival = carried.repo_arrival(remembered);
                let ran = run_repo(
                    &mut term,
                    repos,
                    &path,
                    config,
                    carried.harness(),
                    arrival,
                    Some(&carried.first_launch),
                )?;
                match ran.outcome {
                    Ok(exit @ (Exit::Open(_) | Exit::Practise { .. } | Exit::LeaveTour { .. })) => {
                        Ran {
                            outcome: Ok(Exit::Quit),
                            pool: ran.pool,
                        }
                        .stop_for_the_next_place()?;
                        moved(exit, &mut practice, &mut carried, config)
                    }
                    outcome => {
                        return end(
                            term,
                            Ran {
                                outcome,
                                pool: ran.pool,
                            },
                            practice,
                        );
                    }
                }
            }
            Place::Outside(mut start) => {
                start.problem = carried.outside_problem(start.problem);
                let shown = carried.shown();
                let left = run_outside(&mut term, start, config, shown, config_dir.as_deref());
                match leave_outside(left, &mut carried)? {
                    Next::Place(next) => Ok(next),
                    Next::Practise(from) => practise(from, &mut practice, &mut carried, config),
                    Next::End(ran) => return end(term, *ran, practice),
                }
            }
        };
        place = match went {
            Ok(next) => next,
            Err(reason) => return end(term, Ran::without_jobs(Ok(Exit::Fatal(reason))), practice),
        };
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
    /// (m8/18b), or the tour's word on a build or removal that failed.
    notice: Option<String>,
    /// The next repository lands on this commit, not `HEAD`: the tour's
    /// way back (m8/19b §A4).
    at: Option<graft_core::oid::Oid>,
    /// The next repository is the practice one.
    practice: bool,
    /// The tour was finished: said on the next place's first frame.
    tour_done: bool,
    /// The next place is the way back from the tour (or from a tour that
    /// could not start): not put on the recent list again (§A6).
    returning: bool,
    /// The session clock, for the whole process (`GRAFT_NOW`, m8/19d-b):
    /// every repository's harness and the practice repo's `now`.
    clock: Clock,
}

impl Carried {
    fn first(config: &Loaded, no_color: bool, config_dir: Option<&Path>, clock: Clock) -> Self {
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
            at: None,
            practice: false,
            tour_done: false,
            returning: false,
            clock,
        }
    }

    /// What the empty state says in its bar row on arrival: its own
    /// problem, then what the last place handed on — never a clone's notice
    /// (a clone opens a repository), but said where this screen says
    /// problems all the same — then the tour's finish.
    /// The shipped harness, on this process's session clock.
    fn harness(&self) -> Harness {
        Harness {
            clock: self.clock,
            ..Harness::default()
        }
    }

    fn outside_problem(&mut self, problem: Option<String>) -> Option<String> {
        self.returning = false;
        let problem = joined(problem, self.notice.take());
        if std::mem::take(&mut self.tour_done) {
            return joined(problem, Some("practice done".to_owned()));
        }
        problem
    }

    /// What the empty state is handed: the keys, his config's problems, the
    /// first-launch switch, and the count's warning, once.
    fn shown(&mut self) -> Shown<'_> {
        let warning = self.warning.take();
        Shown {
            keymap: &self.keymap,
            problems: &self.problems,
            first_launch: &self.first_launch,
            warning,
        }
    }

    /// The next repository's arrival: `remembered` (the recent list's
    /// word), else the first-launch warning; the notice handed on, once;
    /// the config's problems, once; the tour's landing, card and finish,
    /// once.
    fn repo_arrival(&mut self, remembered: Option<String>) -> Arrival {
        Arrival {
            status: remembered.or_else(|| self.warning.take()),
            notice: self.notice.take(),
            on_head: true,
            at: self.at.take(),
            practice: std::mem::take(&mut self.practice),
            tour_done: std::mem::take(&mut self.tour_done),
            keymap: self.keymap.clone(),
            config_problems: std::mem::take(&mut self.problems),
            first_launch: self.first_launch.line(),
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

#[cfg(test)]
mod tests;
