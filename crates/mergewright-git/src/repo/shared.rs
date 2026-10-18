//! `SharedRepo`: the `Send`-able half a [`super::RepoHandle`] is taken
//! from.

use std::fmt;
use std::path::Path;

use mergewright_core::on_disk::Generation;

use super::{OpenError, RepoHandle, names_in};

/// A repository handle that can be sent between threads, and the thing that
/// keeps it open.
///
/// Opening a repository reads its configuration, probes its object database
/// and works out which refs backend it uses: real work, and work that must
/// not happen again on every rev-walk. But `gix::Repository` is `Send` and
/// not `Sync`, so it cannot simply be shared. `gix` splits the two apart, and
/// so does this: `SharedRepo` owns the expensive, shared half and hands out a
/// cheap per-thread [`RepoHandle`] on demand.
///
/// This is the type that closes M0's "the repository handle does not outlive
/// the job that opened it": the app keeps one `SharedRepo` for the life of
/// the window, and each job takes a handle from it.
pub struct SharedRepo {
    repo: gix::ThreadSafeRepository,
    /// How much decompressed object data a handle may keep. Applied per
    /// handle, because the cache lives in the thread-local half.
    object_cache_bytes: usize,
    /// Which on-disk names the repository uses, read once at open and
    /// handed to every handle (ADR-0057, Amendment §4).
    names: Generation,
}

impl SharedRepo {
    /// The per-handle object cache, in bytes.
    ///
    /// A rev-walk reads the same commit objects repeatedly — once to find its
    /// parents while exploring, once again to render it — and every read of a
    /// packed object walks its delta chain. `gix` leaves the cache off unless
    /// asked, and 16 MiB is what `git` itself defaults `core.deltaBaseCacheLimit`
    /// to plus room for the objects themselves, which is small against
    /// PROMPT.md §7.1's memory budget and large enough to hold the working set
    /// of a chunked walk.
    pub const OBJECT_CACHE_BYTES: usize = 16 * 1024 * 1024;

    /// Opens the repository that governs `path`, exactly as
    /// [`RepoHandle::open`] does, and keeps it open.
    ///
    /// # Errors
    /// The same cases as [`RepoHandle::open`].
    pub fn open(path: &Path) -> Result<Self, OpenError> {
        match gix::ThreadSafeRepository::discover(path) {
            Ok(repo) => {
                // `ThreadSafeRepository` has no `common_dir`; a thread-local
                // view of it is a clone of what was just loaded.
                let names = names_in(repo.to_thread_local().common_dir(), path)?;
                Ok(SharedRepo {
                    repo,
                    object_cache_bytes: Self::OBJECT_CACHE_BYTES,
                    names,
                })
            }
            Err(error) => Err(OpenError::classify(path, error)),
        }
    }

    /// Which on-disk names the repository uses. See
    /// [`RepoHandle::on_disk_names`].
    #[must_use]
    pub fn on_disk_names(&self) -> Generation {
        self.names
    }

    /// A handle for the calling thread.
    ///
    /// Cheap: it clones the already-loaded configuration and object database
    /// rather than discovering or parsing anything. A job takes one of these
    /// and drops it when it finishes; the repository stays open regardless.
    #[must_use]
    pub fn handle(&self) -> RepoHandle {
        let mut repo = self.repo.to_thread_local();
        repo.object_cache_size_if_unset(self.object_cache_bytes);
        RepoHandle {
            repo,
            names: self.names,
        }
    }

    /// A handle for the calling thread with an object cache of at most
    /// `bytes`, or none at all for `0`.
    ///
    /// For a job whose reads do not repeat — a walk that reads each commit
    /// once, like ADR-0008's promotion — where [`handle`](Self::handle)'s
    /// cache would fill and never be hit (ticket m1-the-graph/27b).
    #[must_use]
    pub fn handle_with_object_cache(&self, bytes: usize) -> RepoHandle {
        let mut repo = self.repo.to_thread_local();
        repo.object_cache_size(bytes);
        RepoHandle {
            repo,
            names: self.names,
        }
    }

    /// The repository's own directory. See [`RepoHandle::git_dir`].
    #[must_use]
    pub fn git_dir(&self) -> &Path {
        self.repo.git_dir()
    }
}

impl fmt::Debug for SharedRepo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SharedRepo")
            .field("git_dir", &self.git_dir())
            .finish()
    }
}
