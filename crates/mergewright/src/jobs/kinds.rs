//! The work behind each [`Command`](crate::update::Command): one function per
//! job kind, each returning a [`Work`] closure a worker runs.
//!
//! Kept together rather than one file per kind: individually these are 7 to
//! 30 lines, too small to justify a file each (the same call made for
//! `update/mod.rs`'s small pieces, ticket m1-the-graph/70).

use std::sync::Arc;
use std::sync::mpsc::Receiver;

use mergewright_core::job::JobId;
use mergewright_git::history::Order;

use super::types::{Delivery, JobEvents, Outcome, Work};
use crate::history::{self, Ended, RowRequest};
use crate::repos::RepoStore;
use crate::repository;
use crate::update::{AppEvent, GitCheck, Product, Progress};

/// The work behind [`Command::WriteCommitGraph`](crate::update::Command::WriteCommitGraph):
/// git's own `commit-graph write`, through a handle on the store's repository.
pub(super) fn write_commit_graph(
    repos: &RepoStore,
) -> Result<mergewright_git::commit_graph::Written, String> {
    let repo = repos.shared().map_err(|error| error.to_string())?;
    mergewright_git::commit_graph::write(&repo.handle()).map_err(|error| error.to_string())
}

/// The work behind [`Command::DeclineCommitGraph`](crate::update::Command::DeclineCommitGraph):
/// remember "never for this repository" in its common directory.
pub(super) fn decline_commit_graph(repos: &RepoStore) -> Result<(), String> {
    let repo = repos.shared().map_err(|error| error.to_string())?;
    mergewright_git::commit_graph::remember_declined(&repo.handle()).map_err(|error| error.to_string())
}

/// The work behind [`Command::OpenRepository`](crate::update::Command::OpenRepository).
///
/// Opening a repository touches the disk and can take a moment on a cold cache
/// or a network filesystem, which is exactly why it is a job and not something
/// `update` does. Only the first one pays: after that the store hands out a
/// handle onto a repository that is already open.
///
/// This is also what `Ctrl+R` re-runs; dispatching it is where anything
/// derived from refs — the refs read, the default branch (ADR-0001) — is
/// marked stale, before the jobs that read them start.
pub(super) fn open_repository(store: Arc<RepoStore>) -> Work {
    Box::new(move |cancel| {
        if cancel.is_cancelled() {
            return Outcome::Cancelled;
        }
        // Whether it opened or not is the answer, and both halves go back the
        // same way: "not a repository" is what the user asked about.
        let facts = store
            .shared()
            .map_err(|error| error.to_string())
            .and_then(|repo| repository::facts(&repo.handle()));
        Outcome::Produced(Product::Repository(facts))
    })
}

/// The work behind [`Command::ReadHistory`](crate::update::Command::ReadHistory), as job `id`.
///
/// The long one, and it stays: it answers the screen's row requests from
/// `requests` for as long as the walk is on screen, laying out only as far as
/// they reach. The token is checked before every chunk, so `Esc` stops it
/// within one; dropping the request sender ends it once idle.
pub(super) fn read_history<S: JobEvents>(
    store: Arc<RepoStore>,
    order: Order,
    requests: Receiver<RowRequest>,
    sink: S,
    id: JobId,
) -> Work {
    Box::new(move |cancel| {
        if cancel.is_cancelled() {
            return Outcome::Cancelled;
        }
        let mut report = |update| sink.deliver(AppEvent::Progress(id, Progress::History(update)));
        let stop = || cancel.is_cancelled();
        match history::stream(&store, order, &requests, &stop, &mut report) {
            // Stopped by its token, its requests closed, or a report found the
            // loop gone — then
            // nobody reads the ending either, and the worker delivering it
            // finds the same dead channel and stops there (`Worker::run`).
            Ok(Ended::Cancelled | Ended::NobodyListening) => Outcome::Cancelled,
            Err(message) => Outcome::Failed(message),
        }
    })
}

/// The work behind [`Command::Search`](crate::update::Command::Search), as job
/// `id`: hits go to the screen a chunk at a time, so the first one can be
/// jumped to while the rest are read.
pub(super) fn search<S: JobEvents>(
    store: Arc<RepoStore>,
    query: crate::search::Query,
    sink: S,
    id: JobId,
) -> Work {
    Box::new(move |cancel| {
        let stop = || cancel.is_cancelled();
        let mut found = |hits| {
            sink.deliver(AppEvent::Progress(id, Progress::Search(hits)))
                != Delivery::NobodyListening
        };
        match crate::search::scan(&store, &query, mergewright_git::history::CHUNK, &stop, &mut found) {
            Ok(crate::search::Scanned::Finished) => Outcome::Done,
            Ok(crate::search::Scanned::Stopped) => Outcome::Cancelled,
            Err(message) => Outcome::Failed(message),
        }
    })
}

/// The work behind [`Command::CheckSystemGit`](crate::update::Command::CheckSystemGit).
///
/// Spawns `git --version` and waits for it, which is not something the UI
/// thread may do — on a cold `PATH` over a network mount it is not instant.
pub(super) fn check_system_git() -> Work {
    Box::new(move |cancel| {
        if cancel.is_cancelled() {
            return Outcome::Cancelled;
        }
        let check = match mergewright_git::version::system_git() {
            Ok(found) => GitCheck::Ran(found),
            // Not a job failure: the job did its work and the answer is "this
            // machine's git cannot be asked". The user still needs to hear it.
            Err(error) => GitCheck::Unavailable(error.to_string()),
        };
        Outcome::Produced(Product::SystemGit(check))
    })
}

/// Untracked files whose lines are counted for the commit screen: each costs
/// one read of the file, so past this many the rest show no counts.
const UNTRACKED_COUNTED: usize = 50;

/// The work behind [`Command::ReadCommitFiles`](crate::update::Command::ReadCommitFiles):
/// every change against `HEAD` with its line counts, for the commit screen's
/// file rows and its summary (ease-pass/08, C4). `git diff-index` gives the
/// tracked files' counts; an untracked file has none from git, so it is read
/// as the all-added diff the screen shows for it and its lines counted. A
/// file too long to read whole, or binary, keeps `None`.
///
/// A path in `staged_parts` goes in as staged only, so its counts are the
/// index against `HEAD` (`git diff --cached`), what goes in (08s M3).
pub(super) fn read_commit_files(
    repos: &RepoStore,
    staged_parts: &[std::path::PathBuf],
    stop: &super::Cancel,
) -> Result<Arc<mergewright_git::diff::FileList>, String> {
    use mergewright_git::diff::{DiffOptions, DiffSpec, FileStatus, LineKind, file_diff, list_files};
    let repo = repos.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    let options = DiffOptions::default();
    let mut list = list_files(&handle, &DiffSpec::WorkingTreeVsHead, &options, stop)
        .map_err(|error| error.to_string())?;
    if !staged_parts.is_empty() {
        let staged = list_files(&handle, &DiffSpec::Staged, &options, stop)
            .map_err(|error| error.to_string())?;
        let is_part = |path: &[u8]| {
            staged_parts
                .iter()
                .any(|part| part.as_os_str().as_encoded_bytes() == path)
        };
        for entry in list.files.iter_mut().filter(|entry| is_part(&entry.path)) {
            if let Some(index) = staged.files.iter().find(|index| index.path == entry.path) {
                entry.clone_from(index);
            }
        }
    }
    for entry in list
        .files
        .iter_mut()
        .filter(|entry| entry.status == FileStatus::Untracked)
        .take(UNTRACKED_COUNTED)
    {
        let diff = file_diff(
            &handle,
            &DiffSpec::WorkingTreeVsHead,
            entry,
            &options,
            crate::update::detail::MAX_LINES,
            stop,
        )
        .map_err(|error| error.to_string())?;
        if diff.binary.is_some() || diff.truncated.is_some() {
            continue;
        }
        let added = diff
            .hunks
            .iter()
            .flat_map(|hunk| &hunk.lines)
            .filter(|line| line.kind == LineKind::Added)
            .count();
        entry.added = u32::try_from(added).ok();
        entry.removed = Some(0);
    }
    Ok(Arc::new(list))
}
