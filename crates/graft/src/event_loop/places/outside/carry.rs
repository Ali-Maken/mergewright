//! What the empty state carries out: its job pool, the steps a key or a job
//! asks for, and the answers to a clone's asks. Moved out of `outside.rs`
//! (m8/21b-5 5j); it mirrors `running/carry.rs`.

use std::io;
use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc::Sender;

use graft_core::job::JobId;
use graft_git::run::askpass::Secret;

use super::Outcome;
use crate::config::Loaded;
use crate::empty_state::{OutsideState, OutsideStep, clone_started};
use crate::event_loop::exit_wait::Pool;
use crate::event_loop::places::{OutsideStart, Place, joined, locate};
use crate::event_loop::{Exit, Open, Wake, signals};
use crate::first_launch::{self, FirstLaunch};
use crate::jobs::JobRunner;
use crate::jobs::clone::CloneRequest;
use crate::repos::RepoStore;
use crate::update::credential::Answer;
use crate::view::rows::home_relative;

/// The empty state's job pool: none until the first clone (rulings R4), so
/// a launch that never clones starts no threads.
pub(in crate::event_loop::places) struct OutsideJobs {
    pub(in crate::event_loop::places) pool: Option<JobRunner<Sender<Wake>>>,
    /// The loop's own channel, for the pool's answers (as `run_repo` wires
    /// its pool, `startup.rs`).
    sender: Sender<Wake>,
}

impl OutsideJobs {
    pub(in crate::event_loop::places) fn new(sender: Sender<Wake>) -> Self {
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
    pub(in crate::event_loop::places) fn into_pool(
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

/// Carry out what a key or a job asked for, but a retheme (the loop's: it
/// holds the session). `Some` when the empty state is over.
pub(in crate::event_loop::places) fn carry_out(
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
pub(in crate::event_loop::places) trait AnswersAsks {
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
pub(in crate::event_loop::places) fn answer_ask(
    pool: &impl AnswersAsks,
    job: JobId,
    answer: Option<Answer>,
) {
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
pub(in crate::event_loop::places) fn try_open(
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
