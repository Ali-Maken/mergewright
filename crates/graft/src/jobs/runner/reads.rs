//! Job fns: opening, refs, working tree, conflicts, finish, commit message,
//! rewrite and row tags.

use std::sync::Arc;

use graft_core::job::JobId;

use super::JobRunner;
use crate::history;
use crate::jobs::kinds::{decline_commit_graph, open_repository, write_commit_graph};
use crate::jobs::types::{Cancel, JobEvents, Outcome};
use crate::jobs::{commit_message, conflict, finish, rewrite, rewrite_open};
use crate::update::{Product, Task};

impl<S: JobEvents> JobRunner<S> {
    /// Opens (or, on `Ctrl+R`, re-opens) the repository. The refresh runs
    /// here, on the dispatching thread, not in the job: the jobs that read
    /// refs are dispatched right after and may start first on another
    /// worker, and must not be served refs read before it (m1-the-graph/55).
    ///
    /// A refresh failure (the repository disappeared, or its commit-graph
    /// could not be read while checking whether it changed) is not swallowed:
    /// it is reported through the same `Product::Repository` the job below
    /// would otherwise produce, so it reaches the status line exactly as an
    /// open failure does.
    pub(super) fn open_repository(&self) {
        if let Err(error) = self.repos.refresh() {
            let message = error.to_string();
            self.announce(Task::OpenRepository, move |_| {
                Outcome::Produced(Product::Repository(Err(message)))
            });
            return;
        }
        self.announce(
            Task::OpenRepository,
            open_repository(Arc::clone(&self.repos)),
        );
    }

    /// Sorts the whole range topologically off the loop's thread, for a walk
    /// whose order is provisional (ADR-0008).
    pub(super) fn promote_history(&self) {
        let repos = Arc::clone(&self.repos);
        self.announce(Task::PromoteHistory, move |cancel| {
            let stop = || cancel.is_cancelled();
            if stop() {
                return Outcome::Cancelled;
            }
            let started = std::time::Instant::now();
            match history::topological_order(&repos, &stop) {
                Ok(Some(order)) => {
                    Outcome::Produced(Product::TopologicalOrder(crate::update::SortedOrder {
                        order,
                        took: started.elapsed(),
                    }))
                }
                Ok(None) => Outcome::Cancelled,
                Err(message) => Outcome::Failed(message),
            }
        });
    }

    /// Builds the commit-graph with system git off the loop's thread
    /// (m1-the-graph/54).
    pub(super) fn write_commit_graph(&self) {
        let repos = Arc::clone(&self.repos);
        self.announce(Task::WriteCommitGraph, move |cancel| {
            // Checked once, before git starts: a write not yet begun can be
            // dropped (nothing is recorded, the next open offers again). Once
            // git runs it is never cancelled nor killed — see
            // `graft_git::commit_graph`.
            if cancel.is_cancelled() {
                return Outcome::Cancelled;
            }
            Outcome::Produced(Product::CommitGraph(write_commit_graph(&repos)))
        });
    }

    /// Remembers the declined commit-graph offer off the loop's thread.
    pub(super) fn decline_commit_graph(&self) {
        let repos = Arc::clone(&self.repos);
        self.announce(Task::DeclineCommitGraph, move |cancel| {
            if cancel.is_cancelled() {
                return Outcome::Cancelled;
            }
            Outcome::Produced(Product::CommitGraphDeclined(decline_commit_graph(&repos)))
        });
    }
    /// Reads the refs off the loop's thread, under the id `update` chose
    /// for it (m6-conflicts/12: a landing waits on one particular read).
    pub(super) fn read_refs(&self, id: JobId) {
        let repos = Arc::clone(&self.repos);
        let cache = Arc::clone(&self.ahead_behind);
        self.queue_as(id, Some(Task::ReadRefs), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::Refs(crate::refs::read(&repos, &cache)))
            })
        });
    }

    /// Reads the working tree's full status off the loop's thread, under an
    /// id `update` chose so the read a write asked for can be told from an
    /// older one (`update::status_read`).
    pub(super) fn read_working_tree(&self, id: JobId) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, Some(Task::ReadWorkingTree), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::WorkingTree(
                    crate::working_tree::read(&repos, cancel).map(Box::new),
                ))
            })
        });
    }

    /// Reads one conflicted path for the resolver, under an id `update`
    /// chose so a stale answer can be dropped (m6-conflicts/04).
    pub(super) fn open_conflict(&self, id: JobId, path: std::path::PathBuf) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, Some(Task::OpenConflict), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                let opened = conflict::open(&repos, &path, cancel);
                Outcome::Produced(Product::ConflictOpened(opened))
            })
        });
    }

    /// Takes one side of one conflict block and writes the file back, under
    /// an id `update` chose (m6-conflicts/04).
    ///
    /// Not cancellable part-way on purpose: the write is one temp-file
    /// rename deep inside `ops`, and a token checked between "decide" and
    /// "rename" would only ever report a take that had in fact already
    /// landed.
    pub(super) fn take_side(&self, id: JobId, request: crate::update::TakeRequest) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, Some(Task::TakeSide), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                let crate::update::TakeRequest {
                    path,
                    file,
                    at,
                    take,
                } = request;
                let taken = conflict::take(&repos, &path, &file, at, take).map(Arc::new);
                Outcome::Produced(Product::Taken(taken))
            })
        });
    }

    /// Puts the last pick back and writes the file once, under an id
    /// `update` chose (ease-pass 05). On the take's queue, and not
    /// cancellable part-way for the take's reason.
    pub(super) fn untake_side(&self, id: JobId, pick: crate::update::resolver::Pick) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, Some(Task::UntakeSide), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                let untaken =
                    conflict::untake(&repos, &pick.path, &pick.after, &pick.blocks, cancel);
                Outcome::Produced(Product::Untaken(untaken.map(Arc::new)))
            })
        });
    }

    /// Attaches the merge base to every block of the open file, under an id
    /// `update` chose so a stale answer can be dropped (`B`, m6-conflicts/05).
    pub(super) fn load_base(
        &self,
        id: JobId,
        path: std::path::PathBuf,
        file: graft_git::conflict::ConflictFile,
    ) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, Some(Task::LoadBase), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                let loaded =
                    conflict::load_base(&repos, &path, file).map(|(file, n)| (Arc::new(file), n));
                Outcome::Produced(Product::BaseLoaded(loaded))
            })
        });
    }

    /// Reads the set of files the operation in progress conflicted on, for
    /// the resolver's file list, under an id `update` chose
    /// (m6-conflicts/13).
    pub(super) fn read_conflict_set(&self, id: JobId) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, Some(Task::ReadConflictSet), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                let set = conflict::conflict_set(&repos, cancel).map(Arc::new);
                Outcome::Produced(Product::ConflictSet(set))
            })
        });
    }

    /// Reads what the merge about to be committed changes, for the finish
    /// summary, under an id `update` chose (m6-conflicts/19).
    pub(super) fn read_finish_summary(&self, id: JobId) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, Some(Task::ReadFinishSummary), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::FinishSummary(finish::summary(&repos)))
            })
        });
    }

    /// Reads one staged file's diff for the finish summary's `space`, under
    /// an id `update` chose (m6-conflicts/19).
    pub(super) fn read_finish_diff(&self, id: JobId, path: std::path::PathBuf) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, Some(Task::ReadFinishDiff), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                let diff = finish::staged_diff(&repos, &path, cancel);
                Outcome::Produced(Product::FinishDiff(diff))
            })
        });
    }

    /// Reads `commit.template`'s file for the commit editor's empty body,
    /// under an id `update` chose (m3-working-tree-undo-and-stash/05).
    pub(super) fn read_commit_template(&self, id: JobId) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, Some(Task::ReadCommitTemplate), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::CommitTemplate(commit_message::template(&repos)))
            })
        });
    }

    /// Reads `HEAD`'s commit message for an amend's prefill, under an id
    /// `update` chose (m3-working-tree-undo-and-stash/05).
    pub(super) fn read_head_message(&self, id: JobId) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, Some(Task::ReadHeadMessage), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::HeadMessage(commit_message::head_message(&repos)))
            })
        });
    }

    /// Plans `a` into an older commit, or absorb, under an id `update`
    /// chose (m7-power-tools/09). A read: no `AppState::operating`.
    pub(super) fn plan_rewrite(
        &self,
        id: JobId,
        request: graft_ops::rewrite::RewriteRequest,
        head: Option<graft_ops::operation::Tip>,
    ) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, Some(Task::PlanRewrite), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                let planned = rewrite::plan(&repos, &request, head.as_ref(), cancel);
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::RewritePlanned(planned))
            })
        });
    }

    /// Reads what the rewrite editor opens on (`R`, m7-power-tools/08),
    /// under an id `update` chose. A read: `Esc` cancels it.
    pub(super) fn open_rewrite(&self, id: JobId, req: graft_ops::rewrite::RewriteRequest) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, Some(Task::OpenRewrite), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                let opened = rewrite_open::open(&repos, &req, cancel);
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::RewriteOpened(opened))
            })
        });
    }

    /// After a put-in landed: the commits it changed on the new line
    /// (m7-power-tools/09).
    pub(super) fn resolve_row_tags(&self, id: JobId, offsets: Vec<u32>) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, Some(Task::ResolveRowTags), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::RowTagsResolved(rewrite::resolve_row_tags(
                    &repos, &offsets, cancel,
                )))
            })
        });
    }
}
