//! The repository, kept open for as long as Graft runs.
//!
//! A [`RepoHandle`](graft_git::repo::RepoHandle) is not `Send`, so it cannot
//! cross to a worker thread, and [`AppState`](crate::update::AppState) is
//! cloned on every event and must stay pure, so it cannot hold one either.
//! What *is* shareable is [`SharedRepo`]: the loaded configuration and object
//! database, from which each job takes a cheap thread-local handle.
//!
//! This is where that lives. One store per run, owned by the job runner and
//! never by the state, so a job borrows a repository that is already open
//! instead of discovering and parsing one again — which is what the M0 gap
//! "the repository handle does not outlive the job" was about.
//!
//! It also caches the answer to ADR-0001's question — which branch owns lane
//! 0 — because the last of that ADR's five rules counts merges over the whole
//! of history and is far too slow to ask twice.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use graft_core::ref_name::RefName;
use graft_git::branches::{BranchError, Branches};
use graft_git::history::{CommitTable, TableError};
use graft_git::repo::{OpenError, SharedRepo};

/// The open repository, and what has been worked out about it once.
pub struct RepoStore {
    path: PathBuf,
    /// The user's `graph.default_branch`, if they set one (ADR-0001 rule 1).
    override_branch: Option<RefName>,
    open: Mutex<Option<Arc<SharedRepo>>>,
    branches: Mutex<Option<Arc<Branches>>>,
    /// The repository's one id table (ADR-0026): every walk and source hands
    /// out handles from it, so a handle means the same commit in every job.
    table: Mutex<Option<Arc<CommitTable>>>,
    /// How many times the repository has actually been discovered and opened.
    ///
    /// Kept because "the handle outlives the job" is not something a type
    /// signature can promise — only a count can — and a test asserts it stays
    /// at one across two jobs.
    opens: AtomicUsize,
}

impl RepoStore {
    /// A store for the repository at `path`. Opens nothing yet: the first job
    /// that needs the repository is the one that pays for opening it, on a
    /// worker thread rather than on the UI thread.
    #[must_use]
    pub fn new(path: PathBuf, override_branch: Option<RefName>) -> Self {
        RepoStore {
            path,
            override_branch,
            open: Mutex::new(None),
            branches: Mutex::new(None),
            table: Mutex::new(None),
            opens: AtomicUsize::new(0),
        }
    }

    /// The path Graft was pointed at.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The open repository, opening it if this is the first time.
    ///
    /// # Errors
    /// [`OpenError`] if the path is not a repository, or is one that cannot be
    /// used. A failure is not remembered: the directory may be a repository by
    /// the time the user presses `Ctrl+R`.
    pub fn shared(&self) -> Result<Arc<SharedRepo>, OpenError> {
        let mut open = lock(&self.open);
        if let Some(repo) = open.as_ref() {
            return Ok(Arc::clone(repo));
        }
        let repo = Arc::new(SharedRepo::open(&self.path)?);
        self.opens.fetch_add(1, Ordering::Release);
        *open = Some(Arc::clone(&repo));
        Ok(repo)
    }

    /// Which branch owns lane 0, and which branches sit beside it (ADR-0001).
    ///
    /// Worked out once and kept. Called from a job, never from `update`: rule
    /// 5 walks the first-parent chain of every branch.
    ///
    /// # Errors
    /// [`StoreError`] if the repository cannot be opened or its refs cannot be
    /// read.
    pub fn branches(&self) -> Result<Arc<Branches>, StoreError> {
        let repo = self.shared()?;
        let mut cached = lock(&self.branches);
        if let Some(branches) = cached.as_ref() {
            return Ok(Arc::clone(branches));
        }
        let resolved = Arc::new(Branches::resolve(
            &repo.handle(),
            self.override_branch.as_ref(),
        )?);
        *cached = Some(Arc::clone(&resolved));
        Ok(resolved)
    }

    /// The repository's id table, opened the first time it is asked for and
    /// shared from then on.
    ///
    /// Kept across [`refresh`](Self::refresh): a handle is stable for the
    /// process's life, and a commit made since the table opened simply
    /// interns past the commit-graph.
    ///
    /// # Errors
    /// [`StoreError`] if the repository cannot be opened or its commit-graph
    /// cannot be read.
    pub fn table(&self) -> Result<Arc<CommitTable>, StoreError> {
        let repo = self.shared()?;
        let mut cached = lock(&self.table);
        if let Some(table) = cached.as_ref() {
            return Ok(Arc::clone(table));
        }
        let table = Arc::new(CommitTable::open(&repo.handle())?);
        *cached = Some(Arc::clone(&table));
        Ok(table)
    }

    /// How many times the repository has been opened from scratch.
    #[must_use]
    pub fn opens(&self) -> usize {
        self.opens.load(Ordering::Acquire)
    }

    /// Forget what was worked out, so the next job reads it again.
    ///
    /// What `Ctrl+R` means for the cached parts: the open repository stays
    /// open — `gix` re-reads refs and objects from disk anyway — but the
    /// default branch is a conclusion drawn from refs that may have moved.
    pub fn refresh(&self) {
        *lock(&self.branches) = None;
    }
}

impl std::fmt::Debug for RepoStore {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RepoStore")
            .field("path", &self.path)
            .field("opens", &self.opens())
            .finish_non_exhaustive()
    }
}

/// Why the store could not answer.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// The repository could not be opened.
    #[error(transparent)]
    Open(#[from] OpenError),
    /// It opened, but ADR-0001's question could not be answered.
    #[error(transparent)]
    Branches(#[from] BranchError),
    /// It opened, but its commit-graph could not be read for the id table.
    #[error(transparent)]
    Table(#[from] TableError),
}

/// A poisoned lock here is recovered from rather than propagated.
///
/// What these locks guard is an `Option` holding a value that is worked out
/// from the repository on disk and never mutated afterwards. A panic while
/// one is held leaves the `Option` either empty or holding a complete value —
/// there is no half-built state to observe — so the worst a later reader sees
/// is `None` and does the work again. Refusing to read the repository at all
/// because some other job panicked would turn one bug into a Graft that can
/// show nothing.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use graft_fixtures::Shape;

    use super::*;

    /// A store over a fixture from the corpus, borrowed read-only.
    fn store(shape: Shape) -> RepoStore {
        RepoStore::new(graft_fixtures::borrow(shape), None)
    }

    /// ADR-0026: one id table per open repository, so a handle means the
    /// same commit in every job.
    #[test]
    fn the_id_table_is_opened_once_and_shared() {
        let store = store(Shape::Linear);

        let first = store.table().expect("the table opens");
        store.refresh();
        let second = store.table().expect("the table opens");

        assert!(
            Arc::ptr_eq(&first, &second),
            "one table, kept across a refresh"
        );
    }

    /// The M0 gap this module closes: every job used to discover and parse
    /// the repository again.
    #[test]
    fn a_repository_is_opened_once_however_often_it_is_asked_for() {
        let store = store(Shape::Linear);

        let first = store.shared().expect("the fixture is a repository");
        let second = store.shared().expect("the fixture is a repository");

        assert!(
            Arc::ptr_eq(&first, &second),
            "both jobs get the same repository"
        );
        assert_eq!(store.opens(), 1);
    }

    /// Nothing is opened until something asks, so startup never pays for the
    /// disk on the UI thread.
    #[test]
    fn a_new_store_has_opened_nothing() {
        assert_eq!(store(Shape::Linear).opens(), 0);
    }

    #[test]
    fn a_path_that_is_not_a_repository_says_so_every_time() {
        let empty = tempfile::tempdir().expect("a temporary directory");
        let store = RepoStore::new(empty.path().to_path_buf(), None);

        let first = store
            .shared()
            .expect_err("an empty directory is not a repository");
        assert!(!first.to_string().is_empty(), "a refusal must say why");
        // Not cached: `git init` may have run by the time the user presses
        // `Ctrl+R`, and a remembered failure would outlive the reason for it.
        store.shared().expect_err("still not a repository");
        assert_eq!(store.opens(), 0, "a failed open is not an open");
    }

    /// ADR-0001 rule 5 counts merges over the whole of history. Asking twice
    /// would be the slowest thing Graft does, twice.
    #[test]
    fn the_branch_layout_is_worked_out_once() {
        let store = store(Shape::FeatureBranches);

        let first = store.branches().expect("the fixture's refs can be read");
        let second = store.branches().expect("the fixture's refs can be read");

        assert!(Arc::ptr_eq(&first, &second));
    }

    /// What `Ctrl+R` means here: refs may have moved, so the conclusion drawn
    /// from them is dropped — but the repository itself stays open.
    #[test]
    fn a_refresh_forgets_the_branches_and_keeps_the_repository() {
        let store = store(Shape::FeatureBranches);
        let before = store.branches().expect("the fixture's refs can be read");

        store.refresh();
        let after = store.branches().expect("the fixture's refs can be read");

        assert!(
            !Arc::ptr_eq(&before, &after),
            "the answer is worked out again"
        );
        assert_eq!(*before, *after, "nothing moved, so it is the same answer");
        assert_eq!(store.opens(), 1, "the repository was not re-opened");
    }

    /// One store, many worker threads: the whole reason `SharedRepo` exists.
    #[test]
    fn jobs_on_different_threads_share_one_open_repository() {
        let store = Arc::new(store(Shape::Linear));

        let workers: Vec<_> = (0..4)
            .map(|_| {
                let store = Arc::clone(&store);
                std::thread::spawn(move || {
                    let repo = store.shared().expect("the fixture is a repository");
                    // A handle is thread-local and is what a job actually
                    // reads through.
                    repo.handle().is_bare()
                })
            })
            .collect();

        for worker in workers {
            let bare = worker.join().expect("no worker panics");
            assert!(!bare, "the fixture has a working tree");
        }
        assert_eq!(store.opens(), 1);
    }
}
