//! Queueing: `submit`, the announced and id-chosen queues, and the detail
//! pane's and history's reads routed onto them, with the radar probes.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use graft_core::event::Event;
use graft_core::job::{JobFailure, JobId};

use graft_core::oid::Oid;
use graft_core::ref_name::RefName;

use super::JobRunner;
use crate::jobs::pool::Assignment;
use crate::jobs::radar::RadarContext;
use crate::jobs::types::{Cancel, JobEvents, Outcome, Work};
use crate::jobs::{detail, radar, stash_only};
use crate::repos::RepoStore;
use crate::update::{AppEvent, Command, Product, Task};
use graft_ops::mistake_radar::probes::{Duplicates, EvilVerdict, ForcePushVerdict};

/// The Mistake Radar's context job: reads the default branch's tip and
/// table from the store `run_radar_context` needs and joins any open error
/// to the same error string every other detail job reports.
fn radar_context(
    repos: &RepoStore,
    default_tip: Oid,
    default_ref: &RefName,
    stop: &Cancel,
) -> Result<Arc<RadarContext>, String> {
    let repo = repos.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    let table = repos.table().map_err(|error| error.to_string())?;
    radar::run_radar_context(&handle, &table, default_tip, default_ref, stop)
}

/// The evil-merge probe: opens the repository the probe needs and reports an
/// open failure as the verdict's own `NotCheckable`, because "could not
/// check" is an answer the pane shows, not an error to report elsewhere.
fn evil_probe(repos: &RepoStore, merge: Oid, stop: &Cancel) -> EvilVerdict {
    let repo = match repos.shared() {
        Ok(repo) => repo,
        Err(error) => return EvilVerdict::NotCheckable(error.to_string()),
    };
    radar::run_evil_probe(&repo.handle(), merge, stop)
}

/// The force-push probe, with the same open-failure rule as [`evil_probe`].
fn force_push_probe(
    repos: &RepoStore,
    branch: &str,
    current: Oid,
    stop: &Cancel,
) -> ForcePushVerdict {
    let repo = match repos.shared() {
        Ok(repo) => repo,
        Err(error) => return ForcePushVerdict::NotCheckable(error.to_string()),
    };
    radar::run_force_push_probe(&repo.handle(), branch, current, stop)
}

/// The duplicates probe, with the same open-failure rule as [`evil_probe`],
/// reported as an error since [`Duplicates`] has no `NotCheckable` slot of
/// its own (the pane renders it as "duplicates: not checked — <line>").
fn duplicates_probe(
    repos: &RepoStore,
    window: u32,
    stop: &Cancel,
) -> Result<Arc<Duplicates>, String> {
    let repo = repos.shared().map_err(|error| error.to_string())?;
    radar::run_duplicates_probe(&repo.handle(), window, stop)
}
impl<S: JobEvents> JobRunner<S> {
    /// Queue some work, and return the id its result will carry.
    ///
    /// Returns as soon as the work is on the queue: the caller is the UI
    /// thread, and it has a frame to draw.
    pub fn submit(&self, work: impl FnOnce(&Cancel) -> Outcome + Send + 'static) -> JobId {
        self.queue_job(None, |_| Box::new(work))
    }

    /// Queue some work and tell the loop, before it is queued, which id the
    /// result will carry.
    ///
    /// The announcement goes down the same channel as the result and is sent
    /// first, so the loop always learns the id before it learns the answer.
    /// Without that, `Esc` could not cancel a walk that has not come back
    /// yet: there would be nothing to name.
    pub(super) fn announce(
        &self,
        task: Task,
        work: impl FnOnce(&Cancel) -> Outcome + Send + 'static,
    ) -> JobId {
        self.queue_job(Some(task), |_| Box::new(work))
    }

    /// Queue the work `build` makes once it knows the job's id — which work
    /// that reports progress needs, because every progress event carries it.
    pub(super) fn queue_job(&self, task: Option<Task>, build: impl FnOnce(JobId) -> Work) -> JobId {
        let id = JobId::new(self.next_id.fetch_add(1, Ordering::Relaxed));
        self.queue_as(id, task, build)
    }

    /// The detail pane's reads (`dispatch` routes only those here).
    pub(super) fn dispatch_detail(&self, command: Command) {
        match command {
            Command::ListFiles(id, spec, options) => {
                let repos = Arc::clone(&self.repos);
                self.detail_job(id, Task::ListFiles { spec }, move |cancel| {
                    Product::Files(detail::read_files(&repos, spec, options, cancel))
                });
            }
            Command::ReadFileDiff(id, key, entry) => {
                let repos = Arc::clone(&self.repos);
                self.detail_job(id, Task::FileDiff { key }, move |cancel| {
                    let result = detail::read_file_diff(&repos, &key, &entry, cancel);
                    Product::FileDiff(key, result)
                });
            }
            Command::ReadBlame(id, request) => self.blame_job(id, *request),
            command @ (Command::TraceHistory { .. } | Command::ReadWholeFile { .. }) => {
                self.dispatch_history(command);
            }
            Command::ContainedIn(id, oid) => {
                let repos = Arc::clone(&self.repos);
                self.detail_job(id, Task::ContainedIn { oid }, move |cancel| {
                    Product::ContainedIn(oid, detail::read_contained(&repos, oid, cancel))
                });
            }
            Command::EvilProbe(id, merge) => {
                let repos = Arc::clone(&self.repos);
                self.detail_job(id, Task::EvilProbe { merge }, move |cancel| {
                    Product::EvilProbe(merge, evil_probe(&repos, merge, cancel))
                });
            }
            Command::ForcePushProbe(id, branch, current) => {
                let repos = Arc::clone(&self.repos);
                let task = Task::ForcePushProbe {
                    branch: branch.clone(),
                    current,
                };
                self.detail_job(id, task, move |cancel| {
                    let verdict = force_push_probe(&repos, &branch, current, cancel);
                    Product::ForcePushProbe(branch, current, verdict)
                });
            }
            Command::RadarContext(id, default_tip, default_ref) => {
                let repos = Arc::clone(&self.repos);
                let task = Task::RadarContext {
                    default_tip,
                    default_ref: default_ref.clone(),
                };
                self.detail_job(id, task, move |cancel| {
                    Product::RadarContext(radar_context(&repos, default_tip, &default_ref, cancel))
                });
            }
            Command::DuplicatesProbe(id, window) => {
                let repos = Arc::clone(&self.repos);
                self.detail_job(id, Task::DuplicatesProbe { window }, move |cancel| {
                    Product::DuplicatesProbe(duplicates_probe(&repos, window, cancel))
                });
            }
            // `dispatch` routes only the ones above here; anything else is a
            // routing bug, not a runtime condition.
            other => unreachable!("not a detail command: {other:?}"),
        }
    }

    /// The history place's reads: the trace, and one commit's whole file.
    fn dispatch_history(&self, command: Command) {
        let repos = Arc::clone(&self.repos);
        match command {
            Command::TraceHistory { id, request } => {
                self.detail_job(id, Task::Trace, move |cancel| {
                    Product::Trace(detail::read_trace(&repos, &request, cancel))
                });
            }
            Command::ReadWholeFile {
                id,
                commit,
                path,
                old_path,
                max_lines,
                ignore_whitespace,
            } => {
                self.detail_job(id, Task::WholeFile, move |cancel| {
                    let read = detail::read_whole_file(
                        &repos,
                        commit,
                        &path,
                        old_path.as_deref(),
                        max_lines,
                        ignore_whitespace,
                        cancel,
                    );
                    Product::WholeFile(read)
                });
            }
            // `dispatch_detail` routes only the two above here.
            other => unreachable!("not a history command: {other:?}"),
        }
    }

    /// Which commits only a stash reaches (m7-power-tools/16), under the
    /// id `update` chose, so an answer to a superseded read is dropped there.
    pub(super) fn read_stash_only(&self, id: JobId, stashes: Vec<Oid>) {
        let repos = Arc::clone(&self.repos);
        self.detail_job(id, Task::StashOnly, move |cancel| {
            Product::StashOnly(stash_only::read_stash_only(&repos, &stashes, cancel))
        });
    }

    /// A blame place's read (m7-power-tools/20), cancellable like the
    /// pane's.
    fn blame_job(&self, id: JobId, request: crate::update::place::BlameRequest) {
        let repos = Arc::clone(&self.repos);
        self.detail_job(id, Task::Blame, move |cancel| {
            Product::Blame(detail::read_blame(&repos, &request, cancel))
        });
    }

    /// Queue one of the detail pane's reads under `update`'s id. A job
    /// stopped part way reports `Cancelled`, never its half-read product.
    fn detail_job(
        &self,
        id: JobId,
        task: Task,
        read: impl FnOnce(&Cancel) -> Product + Send + 'static,
    ) {
        self.queue_as(id, Some(task), |_| {
            Box::new(move |cancel: &Cancel| {
                let product = read(cancel);
                if cancel.is_cancelled() {
                    Outcome::Cancelled
                } else {
                    Outcome::Produced(product)
                }
            })
        });
    }

    /// Queue a job under an id `update` already chose (the detail pane's,
    /// `update::detail::LOCAL_JOB_BIT` set), so it can cancel or drop the
    /// job's answer before the job has even started.
    pub(super) fn queue_as(
        &self,
        id: JobId,
        task: Option<Task>,
        build: impl FnOnce(JobId) -> Work,
    ) -> JobId {
        let work = build(id);
        let cancel = Cancel::new();
        self.live.register(id, cancel.clone());

        if let Some(task) = task {
            // The announcement is deliberately not acted on either way. If
            // the loop has already finished, the job is still queued: the
            // worker that picks it up finds the same dead channel, and that
            // is the one place stopping the rest belongs. Deciding it twice,
            // in two places, is how the two come to disagree.
            self.sink.deliver(AppEvent::Started(id, task));
        }

        if let Err(undelivered) = self.queue.send(Assignment { id, cancel, work }) {
            // Every worker has gone, so this job can never run. Dropping it
            // quietly is exactly the failure that ships: the screen would wait
            // for something that is never coming.
            drop(undelivered);
            self.live.finish(id);
            self.sink.deliver(AppEvent::Core(Event::JobFailed(
                id,
                JobFailure::Failed("the job runner has stopped".to_owned()),
            )));
        }

        id
    }
}
