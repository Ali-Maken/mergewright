//! Job fns: stashes, switch list, palette, index, search, remotes, leaves,
//! folds, the map and rows.

use std::sync::Arc;

use graft_core::job::JobId;

use graft_core::oid::Oid;

use super::JobRunner;
use crate::history::RowRequest;
use crate::jobs::kinds::search;
use crate::jobs::types::{Cancel, JobEvents, Outcome};
use crate::update::{AppEvent, Product, Progress, Task};

impl<S: JobEvents> JobRunner<S> {
    /// Reads the stash list for the `S` overlay, under an id `update`
    /// already chose.
    pub(super) fn read_stashes(&self, id: JobId) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, Some(Task::ReadStashes), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::Stashes(super::stash::read_stashes(&repos, cancel)))
            })
        });
    }

    /// Reads the branch picker's rows (`C`, m7-power-tools/10), under an id
    /// `update` already chose.
    pub(super) fn read_switch_list(&self, id: JobId) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, Some(Task::ReadSwitchList), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::SwitchList(super::switch_list::read_switch_list(
                    &repos, cancel,
                )))
            })
        });
    }

    /// Reads every branch for the palette's BRANCHES (m8-polish-and-ship/12),
    /// under an id `update` already chose.
    pub(super) fn read_palette_branches(&self, id: JobId) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, Some(Task::ReadPaletteBranches), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::PaletteBranches(super::palette::read_branches(
                    &repos, cancel,
                )))
            })
        });
    }

    /// Matches the palette's query against branch and tag names, under an
    /// id `update` already chose; cancelled by the next keystroke.
    pub(super) fn palette_names(&self, id: JobId, request: super::palette::NamesRequest) {
        self.queue_as(id, Some(Task::PaletteNames), move |_| {
            Box::new(move |cancel: &Cancel| {
                match super::palette::match_names(&request, &|| cancel.is_cancelled()) {
                    Some(hits) => Outcome::Produced(Product::PaletteNames(Arc::new(hits))),
                    None => Outcome::Cancelled,
                }
            })
        });
    }

    /// Reads every tracked path for the palette's FILES
    /// (m8-polish-and-ship/12b), under an id `update` already chose.
    pub(super) fn read_tracked_paths(&self, id: JobId) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, Some(Task::ReadTrackedPaths), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::TrackedPaths(super::palette::read_tracked(
                    &repos, cancel,
                )))
            })
        });
    }

    /// Finds the palette's first commits for one query
    /// (m8-polish-and-ship/12b), under an id `update` already chose;
    /// cancelled by the next keystroke.
    pub(super) fn palette_commits(&self, id: JobId, query: crate::search::Query) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, Some(Task::PaletteCommits), move |_| {
            Box::new(move |cancel: &Cancel| {
                let stop = || cancel.is_cancelled();
                let found = crate::search::first_hits(
                    &repos,
                    &query,
                    crate::update::palette::COMMIT_HITS,
                    &stop,
                );
                if stop() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::PaletteCommits(found.map(Arc::from)))
            })
        });
    }

    /// Reads the index now for a stash clash's tie, under an id `update`
    /// already chose (ADR-0045 §2, amended 2026-09-25).
    pub(super) fn read_index_state(&self, id: JobId) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, Some(Task::ReadIndexState), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::IndexState(super::stash::read_index_state(
                    &repos, cancel,
                )))
            })
        });
    }

    /// Starts a search. The id names the search on screen; `update` tells a
    /// superseded one apart by the order searches start in.
    pub(super) fn start_search(&self, query: crate::search::Query) {
        let repos = Arc::clone(&self.repos);
        let sink = self.sink.clone();
        self.queue_job(Some(Task::Search), |id| search(repos, query, sink, id));
    }

    /// Reads the remote facts `f`/`p`/`P` decide from, under an id `update`
    /// already chose (m4-remotes/05).
    pub(super) fn read_remotes(&self, id: JobId) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, Some(Task::ReadRemotes), move |_| {
            Box::new(move |cancel: &Cancel| {
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::Remotes(super::remote::read_remotes(
                    &repos, cancel,
                )))
            })
        });
    }

    /// Reads what a force-with-lease push would take off the remote, under
    /// an id `update` already chose (m7-power-tools/16b). The answer
    /// carries its lease key; `update` drops one a newer key superseded.
    pub(super) fn read_leaves(&self, id: JobId, sent: Oid, expected: Oid) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, None, move |_| {
            Box::new(move |cancel: &Cancel| {
                let result = super::remote::read_leaves(&repos, sent, expected, cancel);
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::Leaves {
                    sent,
                    expected,
                    result,
                })
            })
        });
    }

    /// Counts what a folded merge brought in, for its `+N`
    /// (m8-polish-and-ship/17b).
    pub(super) fn count_folded(&self, id: JobId, merge: Oid) {
        let repos = Arc::clone(&self.repos);
        self.queue_as(id, None, move |_| {
            Box::new(move |cancel: &Cancel| {
                let count = super::fold_count::count_folded(&repos, merge, cancel);
                if cancel.is_cancelled() {
                    return Outcome::Cancelled;
                }
                Outcome::Produced(Product::FoldedCounted { merge, count })
            })
        });
    }

    /// Builds the Map view's per-branch bucket counts, under an id `update`
    /// already chose (`update::map::MapState::open`, mirroring the detail
    /// pane's own jobs — see `queue_as`'s doc).
    pub(super) fn build_map(&self, id: JobId, req: crate::map::MapRequest) {
        let repos = Arc::clone(&self.repos);
        let sink = self.sink.clone();
        self.queue_as(id, Some(Task::Map), |id| {
            Box::new(move |cancel: &Cancel| {
                let mut report = |done, total| {
                    sink.deliver(AppEvent::Progress(id, Progress::Map { done, total }));
                };
                let product = super::map::build_map(&repos, &req, &mut report, cancel);
                Outcome::Produced(Product::Map(product))
            })
        });
    }

    /// Hand a running history job the window of rows the view now wants.
    pub(super) fn show_rows(&self, id: JobId, request: RowRequest) {
        let mut row_requests = self.row_requests();
        // No sender means the job was cancelled or has ended; its answer
        // would be dropped as stale anyway, so there is nobody to tell.
        if let Some(asks) = row_requests.get(&id)
            && asks.send(request).is_err()
        {
            // The job has ended on its own (it failed, or found the loop
            // gone). Its failure is reported through its own outcome.
            row_requests.remove(&id);
        }
    }
}
