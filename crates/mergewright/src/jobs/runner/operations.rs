//! Job fns: resume, operate, clone, credentials, history, undo, preview,
//! journal, tour and recovery.

use std::sync::Arc;

use mergewright_core::job::JobId;

use super::JobRunner;
use crate::jobs::kinds::read_history;
use crate::jobs::types::{Cancel, JobEvents, Outcome};
use crate::jobs::{clone, journal_list, operate, preview, undo};
use crate::update::{AppEvent, Product, Progress, Task};

impl<S: JobEvents> JobRunner<S> {
    /// Runs one working-tree write, under an id `update` already chose
    /// (m3-working-tree-undo-and-stash/04): the runner reports
    /// [`Progress::Operating`] as soon as the job starts, then `ops::apply`
    /// does the rest — the seam rule (`tests/it/seam_rule.rs`).
    /// `c`/`s`/`a` on the operation banner: carry the live operation
    /// forward, skip its step, or abort it
    /// (m5-history-and-ghost-preview/09).
    ///
    /// Simpler than [`Self::operate`]: no progress stream and no askpass,
    /// because `git <kind> --continue|--skip|--abort` talks to no remote
    /// and asks no credential.
    pub(super) fn resume(
        &self,
        id: JobId,
        what: mergewright_ops::apply::Resume,
        keys: Arc<mergewright_ui::keymap::Keymap>,
    ) {
        let repos = Arc::clone(&self.repos);
        let sink = self.sink.clone();
        let clock = self.clock;
        let description = match what {
            mergewright_ops::apply::Resume::Continue { .. }
            | mergewright_ops::apply::Resume::ContinueWith { .. } => "continue",
            mergewright_ops::apply::Resume::Skip => "skip",
            mergewright_ops::apply::Resume::Abort => "abort",
        };
        self.queue_as(id, Some(Task::Resume), move |id| {
            Box::new(move |cancel: &Cancel| {
                sink.deliver(AppEvent::Progress(
                    id,
                    Progress::Operating {
                        description: description.to_owned(),
                    },
                ));
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::Resumed(Box::new(operate::run_resume(
                    &repos, what, clock, cancel, &keys,
                ))))
            })
        });
    }

    pub(super) fn operate(
        &self,
        id: JobId,
        op: Box<mergewright_ops::operation::Operation>,
        keys: Arc<mergewright_ui::keymap::Keymap>,
    ) {
        let repos = Arc::clone(&self.repos);
        let sink = self.sink.clone();
        let clock = self.clock;
        let description = op.describe();
        let waiting = self.waiting.clone();
        let helper = operate::askpass_helper(self.askpass_helper.as_ref());
        self.queue_as(id, Some(Task::Operate), move |id| {
            Box::new(move |cancel: &Cancel| {
                sink.deliver(AppEvent::Progress(id, Progress::Operating { description }));
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                // A fetch, pull or push streams git's progress lines to the
                // status line (m4-remotes/05) and asks through the prompt.
                let hooks = operate::job_hooks(sink, id, waiting, cancel, helper);
                operate::outcome(operate::run_operation(
                    &repos, &op, clock, cancel, hooks, &keys,
                ))
            })
        });
    }

    /// Clone `request.url` into `request.into` (m8/18b): answered as
    /// [`Product::Cloned`], progress as `Progress::RemoteProgress`, git's
    /// questions as `Progress::CredentialNeeded` under the returned id.
    /// Stoppable: [`Self::cancel`] stops git and the answer still comes,
    /// saying what the stop left.
    pub(crate) fn clone_repo(&self, request: clone::CloneRequest) -> JobId {
        let sink = self.sink.clone();
        let waiting = self.waiting.clone();
        let helper = operate::askpass_helper(self.askpass_helper.as_ref());
        self.queue_job(None, move |id| {
            Box::new(clone::work(sink, id, waiting, helper, request))
        })
    }

    /// The user's answer to git's credential question, or their cancelling
    /// of it (m4-remotes/06).
    ///
    /// A `false` from `answer_credential` is the job having ended while the
    /// key was in flight: the answer is dropped there and then, zeroed, and
    /// there is nobody left to tell.
    pub(super) fn credential(&self, id: JobId, answer: Option<crate::update::credential::Answer>) {
        let _delivered: bool = self.answer_credential(
            id,
            answer.map(crate::update::credential::Answer::into_secret),
        );
    }

    /// Starts the history walk and keeps the channel its row requests go down.
    ///
    /// Under the id `update` already chose, like the detail pane's reads: a
    /// `Ctrl+R` pressed before this walk's `Started` event lands still has a
    /// name to cancel (m5-history-and-ghost-preview/05).
    pub(super) fn read_history(&self, id: JobId, order: mergewright_git::history::Order) {
        let repos = Arc::clone(&self.repos);
        let sink = self.sink.clone();
        let (asks, requests) = std::sync::mpsc::channel();
        // Registered before `update` can see the job has started: the
        // announcement travels through the loop's queue, and this runs
        // on the loop's own thread before it reads that queue again.
        let id = self.queue_as(id, Some(Task::ReadHistory), |id| {
            read_history(repos, order, requests, sink, id)
        });
        self.row_requests().insert(id, asks);
    }

    /// Runs one undo or redo, under an id `update` already chose
    /// (m3-working-tree-undo-and-stash/06). A write like any other: it takes
    /// `AppState::operating`, so the same `Progress::Operating` line names it
    /// while it runs.
    pub(super) fn run_undo(
        &self,
        id: JobId,
        redo: bool,
        force: bool,
        keys: Arc<mergewright_ui::keymap::Keymap>,
    ) {
        let repos = Arc::clone(&self.repos);
        let sink = self.sink.clone();
        let clock = self.clock;
        let description = if redo { "redo" } else { "undo" };
        self.queue_as(id, Some(Task::Undo), move |id| {
            Box::new(move |cancel: &Cancel| {
                sink.deliver(AppEvent::Progress(
                    id,
                    Progress::Operating {
                        description: description.to_owned(),
                    },
                ));
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::Undone(Box::new(undo::run_undo(
                    &repos, redo, force, clock, cancel, &keys,
                ))))
            })
        });
    }

    /// Draws one Ghost Preview, under an id `update` already chose.
    ///
    /// A read like the detail pane's, not a write: it takes no
    /// `AppState::operating`, announces no progress, and `Esc` cancels it
    /// the way it cancels any other read.
    pub(super) fn plan_preview(
        &self,
        id: JobId,
        request: Box<crate::update::preview::PreviewRequest>,
    ) {
        let repos = Arc::clone(&self.repos);
        let clock = self.clock;
        self.queue_as(id, Some(Task::PlanPreview), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::Preview(preview::run_preview(
                    &repos, &request, clock, cancel,
                )))
            })
        });
    }

    /// Runs the journal's time machine, under an id `update` already chose.
    ///
    /// A write, like an undo: it takes `AppState::operating`, so the same
    /// `Progress::Operating` line names it while it runs.
    pub(super) fn run_revert_to(
        &self,
        id: JobId,
        target: mergewright_core::journal::EntryId,
        force: bool,
        keys: Arc<mergewright_ui::keymap::Keymap>,
    ) {
        let repos = Arc::clone(&self.repos);
        let sink = self.sink.clone();
        let clock = self.clock;
        self.queue_as(id, Some(Task::RevertTo), move |id| {
            Box::new(move |cancel: &Cancel| {
                sink.deliver(AppEvent::Progress(
                    id,
                    Progress::Operating {
                        description: "revert".to_owned(),
                    },
                ));
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::RevertedTo(Box::new(undo::run_revert_to(
                    &repos, &target, force, clock, cancel, &keys,
                ))))
            })
        });
    }

    /// Reads the undo journal, under an id `update` already chose. Closes
    /// whatever a previous run left open before it answers.
    pub(super) fn read_journal(&self, id: JobId) {
        let repos = Arc::clone(&self.repos);
        let clock = self.clock;
        self.queue_as(id, Some(Task::ReadJournal), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::Journal(undo::read_journal(&repos, clock, cancel)))
            })
        });
    }

    /// The practice tour's ticks (m8-polish-and-ship/19b): the practice
    /// repository's journal, read without writing, judged by `mergewright-ops`.
    pub(super) fn tour_progress(&self, id: JobId) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, Some(Task::TourProgress), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                let done = match repos.workdir() {
                    Ok(Some(work)) => {
                        mergewright_ops::tour::tour_progress(&work).map_err(|error| error.explain())
                    }
                    Ok(None) => Err("the practice repository has no working tree".to_owned()),
                    Err(error) => Err(error.to_string()),
                };
                Outcome::Produced(Product::TourProgress(done))
            })
        });
    }

    /// J's stage 2 (m7-power-tools/24): git's reflog records.
    pub(super) fn read_outside(&self, id: JobId) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, Some(Task::ReadOutside), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::Outside(journal_list::read_outside(&repos, cancel)))
            })
        });
    }

    /// J's stage 3 (m7-power-tools/24): the lost commits and edits, over
    /// stage 2's records.
    pub(super) fn find_lost(
        &self,
        id: JobId,
        outside: Arc<Vec<mergewright_ops::journal_list::OutsideRecord>>,
    ) {
        let repos = Arc::clone(&self.repos);
        let clock = self.clock;
        self.queue_as(id, Some(Task::FindLost), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::LostFound(journal_list::find_lost(
                    &repos, clock, &outside, cancel,
                )))
            })
        });
    }

    /// What `⏎` on `list.rows[row]` in J puts back (m7-power-tools/24).
    pub(super) fn recover_for(
        &self,
        id: JobId,
        list: Arc<mergewright_ops::journal_list::JournalList>,
        row: usize,
        keys: Arc<mergewright_ui::keymap::Keymap>,
    ) {
        let repos = Arc::clone(&self.repos);
        let clock = self.clock;
        self.queue_as(id, Some(Task::RecoverFor), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::RecoverFor(journal_list::recover_for(
                    &repos, clock, &list, row, cancel, &keys,
                )))
            })
        });
    }
}
