//! The forced-write bookkeeping of the job pool (moved out of `pool.rs`,
//! m8-polish-and-ship/13f): which unstoppable writes a stop was forced on,
//! and where each stop left git.

use std::sync::PoisonError;
use std::time::{Duration, Instant};

use graft_core::job::JobId;
use graft_ops::apply::StoppedAt;

use super::pool::Live;

/// A forced write: its description, and its answer.
pub(super) struct ForcedJob {
    what: String,
    pub(super) answer: Answer,
}

/// Where a forced write's stop left git, once it has said.
#[derive(Clone, Copy)]
pub(super) enum Answer {
    /// Still running: its git being stopped, or its after-state reads.
    Awaited,
    /// Ended. `None`: nothing to say — never run, or no stop reached git.
    Given(Option<StoppedAt>),
}

impl ForcedJob {
    /// `what`, just forced. One no worker has started never will — its
    /// cancel is seen before it runs — so it is answered now, with nothing.
    pub(super) fn new(what: String, started: bool) -> Self {
        Self {
            what,
            answer: if started {
                Answer::Awaited
            } else {
                Answer::Given(None)
            },
        }
    }
}

impl Live {
    /// Lift `id`'s mark and cancel it: its git is stopped half-way. The
    /// quit stop box's `y` only (`Command::ForceStop`). A write still live
    /// is remembered as forced, so the shutdown after the quit waits for
    /// its answer.
    pub(super) fn force_stop(&self, id: JobId) {
        let mut jobs = self.lock();
        let what = jobs.unstoppable.remove(&id);
        let Some(token) = jobs.tokens.get(&id) else {
            // Already over: nothing to stop, and its answer went to the
            // loop as any job's does.
            return;
        };
        token.cancel();
        if let Some(what) = what {
            let started = jobs.started.contains(&id);
            jobs.forced.insert(id, ForcedJob::new(what, started));
        }
    }

    /// Lift every live unstoppable job's mark, cancel it, and remember it
    /// as forced ([`Live::wait_for_forced`] names them). A job still queued
    /// is cancelled before it starts, so it never runs (`run_job`).
    /// Shutdown's, once its bound for writes is spent or a second signal
    /// asks for it.
    pub(super) fn force_all_unstoppable(&self) {
        let mut jobs = self.lock();
        let live: Vec<JobId> = jobs
            .unstoppable
            .keys()
            .filter(|id| jobs.tokens.contains_key(id))
            .copied()
            .collect();
        for id in live {
            if let Some(token) = jobs.tokens.get(&id) {
                token.cancel();
            }
            if let Some(what) = jobs.unstoppable.remove(&id) {
                let started = jobs.started.contains(&id);
                jobs.forced.insert(id, ForcedJob::new(what, started));
            }
        }
    }

    /// Block until every forced write has answered, or `bound` has passed;
    /// then every forced write, in id order, with its answer.
    ///
    /// A forced write's answer is read **after** its git is stopped: the
    /// after-state reads that close its entry (13c item 11). One that has
    /// not answered by `bound` is named with no answer and abandoned. **Why
    /// that is safe:** its entry is `Pending` or already closed, and the
    /// next launch's recovery closes a `Pending` one — nothing is lost by
    /// not hearing the answer, only the words that say where git was left.
    pub(super) fn wait_for_forced(&self, bound: Duration) -> Vec<Forced> {
        let deadline = Instant::now() + bound;
        let mut jobs = self.lock();
        while jobs
            .forced
            .values()
            .any(|forced| matches!(forced.answer, Answer::Awaited))
        {
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                break;
            };
            let (guard, _timeout) = self
                .ended
                .wait_timeout(jobs, remaining)
                .unwrap_or_else(PoisonError::into_inner);
            jobs = guard;
        }
        jobs.forced
            .iter()
            .map(|(id, forced)| Forced {
                job: *id,
                what: forced.what.clone(),
                at: match forced.answer {
                    Answer::Given(at) => at,
                    Answer::Awaited => None,
                },
            })
            .collect()
    }
}

/// A write a stop was forced on, and where the stop left git
/// (m8-polish-and-ship/13c item 11): what the shell's exit line says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Forced {
    /// The write's job.
    pub job: JobId,
    /// The write's description.
    pub what: String,
    /// Where its stop left git, as `graft_ops` read it from disk. `None`:
    /// no answer within [`super::FORCED_WAIT`] (the job was abandoned), or
    /// nothing to say (never run, or the stop did not reach git).
    pub at: Option<StoppedAt>,
}
