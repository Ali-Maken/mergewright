//! Opening a repository, and the handful of facts that are true the moment it
//! is open: where it lives, and where `HEAD` points.
//!
//! Everything here is a read, and every read goes through `gix` (ADR-0011).
//! Paths stay `Path`/`PathBuf` end to end — never `String` — because a
//! repository can sit at a path that is not valid UTF-8 (PROMPT.md §6.3), and
//! converting early would mean opening the wrong directory or refusing to open
//! a real one. Turning any of this into text for the screen is `app`'s job, at
//! the view-model boundary, through `graft_term::sanitize_path` (DESIGN.md
//! §8.2).

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use graft_core::oid::Oid;
use graft_core::ref_name::RefName;

/// An open repository.
///
/// Cheap to hold and cheap to clone within a process; opening one reads the
/// repository's configuration, so open it once and keep it.
pub struct RepoHandle {
    repo: gix::Repository,
}

impl RepoHandle {
    /// Opens the repository that governs `path`, exactly as git would.
    ///
    /// `path` may be the working tree's root, any directory inside it, a bare
    /// repository, a linked worktree or a submodule's working directory:
    /// discovery walks *up* from `path` until it finds a repository, so a
    /// subdirectory opens the repository above it. It never walks *down*: a
    /// directory that merely contains repositories is not itself one, and is
    /// reported as [`OpenError::NotARepo`].
    ///
    /// # Errors
    /// [`OpenError::NotARepo`] when neither `path` nor any of its ancestors is
    /// a repository — the one case the caller answers with a first-run screen
    /// rather than a failure. The other three are real failures: a path that
    /// cannot be read ([`Inaccessible`](OpenError::Inaccessible),
    /// [`Io`](OpenError::Io)), or a repository git itself would refuse
    /// ([`Unusable`](OpenError::Unusable)).
    pub fn open(path: &Path) -> Result<Self, OpenError> {
        match gix::discover(path) {
            Ok(repo) => Ok(RepoHandle { repo }),
            Err(error) => Err(OpenError::classify(path, error)),
        }
    }

    /// The repository's own directory: `<worktree>/.git` for an ordinary
    /// repository, the repository itself when it is bare, and
    /// `<main>/.git/worktrees/<name>` for a linked worktree.
    #[must_use]
    pub fn git_dir(&self) -> &Path {
        self.repo.git_dir()
    }

    /// The directory holding everything a linked worktree shares with the
    /// repository it was added from — objects, refs, config.
    ///
    /// Equal to [`git_dir`](Self::git_dir) unless this handle is on a linked
    /// worktree, which is how the two are told apart.
    #[must_use]
    pub fn common_dir(&self) -> &Path {
        self.repo.common_dir()
    }

    /// The root of the working tree, or `None` for a bare repository — the one
    /// case where there are no files to show a status for.
    #[must_use]
    pub fn workdir(&self) -> Option<&Path> {
        self.repo.workdir()
    }

    /// Whether the repository has no working tree.
    #[must_use]
    pub fn is_bare(&self) -> bool {
        self.repo.is_bare()
    }

    /// The repository's resolved `.mailmap`, merging every source git would
    /// (working tree file, `mailmap.blob`, `mailmap.file`).
    ///
    /// Never fails: a missing or unreadable `.mailmap` resolves every
    /// identity to itself, which is how git treats it too — the file is
    /// optional, not a repository requirement.
    #[must_use]
    pub fn mailmap(&self) -> crate::mailmap::Mailmap {
        crate::mailmap::Mailmap::from_snapshot(self.repo.open_mailmap())
    }

    /// Where `HEAD` points right now.
    ///
    /// Read on every call rather than cached at open: a checkout, a commit or
    /// a rebase moves `HEAD` underneath us, and a stale answer here would put
    /// the wrong branch name on the screen.
    ///
    /// # Errors
    /// [`HeadError`] if `HEAD` cannot be read, or holds a name or an id that
    /// git's own rules reject.
    pub fn head(&self) -> Result<HeadState, HeadError> {
        let head = self.repo.head().map_err(|source| HeadError::Read {
            git_dir: self.git_dir().to_path_buf(),
            source: Box::new(source),
        })?;

        let malformed = |source| HeadError::Malformed {
            git_dir: self.git_dir().to_path_buf(),
            source,
        };

        match head.kind {
            gix::head::Kind::Symbolic(reference) => Ok(HeadState::Branch(
                RefName::new(reference.name.as_bstr().to_vec()).map_err(malformed)?,
            )),
            gix::head::Kind::Unborn(name) => Ok(HeadState::Unborn(
                RefName::new(name.as_bstr().to_vec()).map_err(malformed)?,
            )),
            gix::head::Kind::Detached { target, .. } => Ok(HeadState::Detached(
                Oid::from_bytes(target.as_bytes()).map_err(malformed)?,
            )),
        }
    }

    /// Every ref in the repository, sorted by full name.
    ///
    /// Branches, tags, remote-tracking branches, `refs/stash` and anything
    /// else the ref store holds — but not `HEAD`, which is
    /// [`head`](Self::head)'s job because its three states are not a ref's
    /// three states.
    ///
    /// An annotated tag's target is the *tag object*, not the commit it
    /// points at: peeling costs an object read per tag, and the caller that
    /// needs the commit is the one that should pay for it.
    ///
    /// # Errors
    /// [`RefsError`] if the ref store cannot be read, or holds a name or an
    /// id that git's own rules reject.
    pub fn references(&self) -> Result<Vec<RefEntry>, RefsError> {
        let read = |source: Box<dyn std::error::Error + Send + Sync>| RefsError::Read {
            git_dir: self.git_dir().to_path_buf(),
            source,
        };
        let malformed = |source| RefsError::Malformed {
            git_dir: self.git_dir().to_path_buf(),
            source,
        };

        let platform = self.repo.references().map_err(|why| read(Box::new(why)))?;
        let iter = platform.all().map_err(|why| read(Box::new(why)))?;
        let mut found = Vec::new();
        for reference in iter {
            let reference = reference.map_err(read)?;
            let target = match reference.target() {
                gix::refs::TargetRef::Object(id) => {
                    RefTarget::Object(Oid::from_bytes(id.as_bytes()).map_err(malformed)?)
                }
                gix::refs::TargetRef::Symbolic(name) => {
                    RefTarget::Symbolic(RefName::new(name.as_bstr().to_vec()).map_err(malformed)?)
                }
            };
            let name = RefName::new(reference.name().as_bstr().to_vec()).map_err(malformed)?;
            found.push(RefEntry { name, target });
        }
        found.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(found)
    }

    /// Every commit a ref reaches, `HEAD` included: where a walk starts.
    ///
    /// Annotated tags are peeled, which costs an object read each, and that
    /// is the point — a walk seeded from the tag object rather than the
    /// commit it names would find nothing.
    ///
    /// A ref that does not lead to a commit is skipped, not an error: a tag
    /// on a blob and `refs/notes/commits` are both legal, and neither has a
    /// place in a commit graph. A ref that cannot be *read* is an error, and
    /// is returned as one.
    ///
    /// The result is sorted and free of duplicates, so two refs at the same
    /// commit seed the walk once.
    ///
    /// # Errors
    /// [`TipsError`] if the ref store cannot be read, or a ref cannot be
    /// peeled for a reason other than not naming a commit.
    pub fn commit_tips(&self) -> Result<Vec<Oid>, TipsError> {
        let read = |source: Box<dyn std::error::Error + Send + Sync>| TipsError::Read {
            git_dir: self.git_dir().to_path_buf(),
            source,
        };

        let platform = self.repo.references().map_err(|why| read(Box::new(why)))?;
        let iter = platform.all().map_err(|why| read(Box::new(why)))?;

        let mut tips = Vec::new();
        for reference in iter {
            let mut reference = reference.map_err(read)?;
            self.push_tip(&mut tips, &mut reference)?;
        }
        // `HEAD` is not in the ref store's iteration, and on a detached
        // `HEAD` it is the only thing holding its commit. An unborn `HEAD`
        // peels to nothing and adds no tip, which is the right answer.
        let mut head = self.repo.head().map_err(|why| read(Box::new(why)))?;
        if let Some(id) = head.try_peel_to_id().map_err(|why| read(Box::new(why)))? {
            tips.push(self.oid(id.as_bytes())?);
        }

        tips.sort_unstable();
        tips.dedup();
        Ok(tips)
    }

    /// Peels one ref to a commit and records it, ignoring refs that do not
    /// name a commit at all.
    /// Every ref that reaches a commit, with the commit it peels to, sorted
    /// by full name.
    ///
    /// Where [`references`](Self::references) leaves an annotated tag
    /// pointing at its tag object, this follows it to the commit — what a
    /// ref badge is drawn on. Refs that peel to a tree or a blob, and
    /// symbolic refs whose target is gone, are left out: they sit on no
    /// commit.
    ///
    /// # Errors
    /// [`TipsError`] if the ref store cannot be read or a ref cannot be
    /// peeled.
    pub fn peeled_references(&self) -> Result<Vec<(RefName, Oid)>, TipsError> {
        let read = |source: Box<dyn std::error::Error + Send + Sync>| TipsError::Read {
            git_dir: self.git_dir().to_path_buf(),
            source,
        };
        let platform = self.repo.references().map_err(|why| read(Box::new(why)))?;
        let iter = platform.all().map_err(|why| read(Box::new(why)))?;
        let mut found = Vec::new();
        for reference in iter {
            let mut reference = reference.map_err(read)?;
            let name = RefName::new(reference.name().as_bstr().to_vec()).map_err(|source| {
                TipsError::Malformed {
                    git_dir: self.git_dir().to_path_buf(),
                    source,
                }
            })?;
            let mut tips = Vec::with_capacity(1);
            self.push_tip(&mut tips, &mut reference)?;
            if let Some(id) = tips.pop() {
                found.push((name, id));
            }
        }
        found.sort_by(|left, right| left.0.cmp(&right.0));
        Ok(found)
    }

    /// The remote-tracking ref `branch` pulls from, as git's
    /// `branch.<name>.remote` and `branch.<name>.merge` settings and the
    /// remote's fetch refspecs name it — `refs/remotes/origin/main` for
    /// `refs/heads/main` set up the usual way. `None` when the branch has no
    /// upstream configured. The ref named may not exist.
    ///
    /// # Errors
    /// [`RefsError`] if the configuration names an upstream that cannot be
    /// resolved (an unknown remote, a malformed refspec).
    pub fn upstream(&self, branch: &RefName) -> Result<Option<RefName>, RefsError> {
        let read = |source: Box<dyn std::error::Error + Send + Sync>| RefsError::Read {
            git_dir: self.git_dir().to_path_buf(),
            source,
        };
        let full = gix::refs::FullName::try_from(gix::bstr::BString::from(branch.as_bytes()))
            .map_err(|why| read(Box::new(why)))?;
        match self
            .repo
            .branch_remote_tracking_ref_name(full.as_ref(), gix::remote::Direction::Fetch)
        {
            None => Ok(None),
            Some(Err(why)) => Err(read(Box::new(why))),
            Some(Ok(tracking)) => {
                RefName::new(tracking.as_bstr().to_vec())
                    .map(Some)
                    .map_err(|source| RefsError::Malformed {
                        git_dir: self.git_dir().to_path_buf(),
                        source,
                    })
            }
        }
    }

    fn push_tip(
        &self,
        tips: &mut Vec<Oid>,
        reference: &mut gix::Reference<'_>,
    ) -> Result<(), TipsError> {
        match reference.peel_to_commit() {
            Ok(commit) => {
                let id = self.oid(commit.id().as_bytes())?;
                tips.push(id);
                Ok(())
            }
            // A tag on a tree or a blob, and `refs/notes/*`, peel to
            // something that is not a commit. Legal, and not a tip: `gix`
            // reports the object it stopped on, which is how "this ref has
            // no commit" is told apart from "this ref cannot be read".
            Err(gix::reference::peel::to_kind::Error::PeelObject(
                gix::object::peel::to_kind::Error::NotFound { .. },
            )) => Ok(()),
            Err(other) => Err(TipsError::Read {
                git_dir: self.git_dir().to_path_buf(),
                source: Box::new(other),
            }),
        }
    }

    /// An object id from raw bytes, blaming this repository if it is the
    /// wrong length.
    fn oid(&self, bytes: &[u8]) -> Result<Oid, TipsError> {
        Oid::from_bytes(bytes).map_err(|source| TipsError::Malformed {
            git_dir: self.git_dir().to_path_buf(),
            source,
        })
    }

    /// The repository `gix` is holding.
    ///
    /// Crate-private: `gix`'s types are this crate's business, and letting
    /// one out would put `gix` in every caller's signature (ADR-0011).
    pub(crate) fn gix(&self) -> &gix::Repository {
        &self.repo
    }
}

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
    /// The same four cases as [`RepoHandle::open`].
    pub fn open(path: &Path) -> Result<Self, OpenError> {
        match gix::ThreadSafeRepository::discover(path) {
            Ok(repo) => Ok(SharedRepo {
                repo,
                object_cache_bytes: Self::OBJECT_CACHE_BYTES,
            }),
            Err(error) => Err(OpenError::classify(path, error)),
        }
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
        RepoHandle { repo }
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
        RepoHandle { repo }
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

/// Why a walk's starting points could not be found.
#[derive(Debug, thiserror::Error)]
pub enum TipsError {
    /// The ref store could not be read, or a ref could not be peeled.
    #[error("cannot read the refs in {}: {source}", git_dir.display())]
    Read {
        /// The repository whose refs were being read.
        git_dir: PathBuf,
        /// What `gix` said. Boxed to keep `TipsError` small.
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// A ref peeled to an object id git's own rules reject.
    #[error("a ref in {} names a malformed object: {source}", git_dir.display())]
    Malformed {
        /// The repository whose refs were being read.
        git_dir: PathBuf,
        /// Which invariant the value broke.
        source: graft_core::error::Error,
    },
}

/// One ref, as the ref store holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefEntry {
    /// The full name — `refs/heads/main`, `refs/tags/v1.0`.
    pub name: RefName,
    /// What the ref points at.
    pub target: RefTarget,
}

/// What a ref points at: an object, or another ref.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RefTarget {
    /// An object id, read as stored and not peeled.
    Object(Oid),
    /// Another ref's full name. Rare outside `HEAD`, but legal anywhere.
    Symbolic(RefName),
}

/// Why the refs could not be read.
#[derive(Debug, thiserror::Error)]
pub enum RefsError {
    /// The ref store could not be read.
    #[error("cannot read the refs in {}: {source}", git_dir.display())]
    Read {
        /// The repository whose refs were being read.
        git_dir: PathBuf,
        /// What `gix` said. Boxed to keep `RefsError` small.
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// A ref was read, but names something git's own rules reject.
    #[error("a ref in {} is malformed: {source}", git_dir.display())]
    Malformed {
        /// The repository whose refs were being read.
        git_dir: PathBuf,
        /// Which invariant the value broke.
        source: graft_core::error::Error,
    },
}

impl fmt::Debug for RepoHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RepoHandle")
            .field("git_dir", &self.git_dir())
            .field("workdir", &self.workdir())
            .finish()
    }
}

/// Where `HEAD` points: the three states every screen has to survive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HeadState {
    /// On a branch, with at least one commit on it. The name is the full ref
    /// name — `refs/heads/main`.
    Branch(RefName),
    /// Detached: `HEAD` names a commit directly, with no branch to advance.
    /// Mid-rebase and after `git checkout <sha>`.
    Detached(Oid),
    /// On a branch that does not exist yet, because nothing has been committed
    /// — a fresh `git init`, or an orphan branch. The name is where the first
    /// commit will land.
    Unborn(RefName),
}

/// Why a repository could not be opened.
///
/// The split that matters is [`NotARepo`](OpenError::NotARepo) against
/// everything else: "you are not in a repository" is an ordinary thing for a
/// person to do and gets an explanation, while an unreadable path or a
/// repository git refuses is a failure and gets reported as one.
#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    /// Neither the path nor any ancestor of it is a repository.
    #[error("{} is not inside a git repository", path.display())]
    NotARepo {
        /// The path Graft was asked to open.
        path: PathBuf,
    },
    /// The path itself is unusable as a starting point: it does not exist, is
    /// not a directory, or reaches above the root of the filesystem.
    #[error("{} is not a directory Graft can read", path.display())]
    Inaccessible {
        /// The path Graft was asked to open.
        path: PathBuf,
    },
    /// Something on the way up from the path could not be read.
    #[error("cannot read {}: {source}", path.display())]
    Io {
        /// The path Graft was asked to open.
        path: PathBuf,
        /// What the operating system said.
        source: io::Error,
    },
    /// A repository was found, but git itself could not use it — a broken
    /// config file, an unsupported repository format.
    #[error("the repository above {} cannot be opened: {source}", path.display())]
    Unusable {
        /// The path Graft was asked to open.
        path: PathBuf,
        /// What `gix` said. Boxed: `gix`'s error is large, and an `OpenError`
        /// is returned by value on every failed open.
        source: Box<gix::discover::Error>,
    },
}

impl OpenError {
    /// Sorts a `gix` discovery failure into the four cases above.
    fn classify(path: &Path, error: gix::discover::Error) -> Self {
        use gix::discover::upwards::Error as Upwards;

        let path = path.to_path_buf();
        match error {
            // The search ran and found nothing. The only ordinary failure:
            // the person is simply not in a repository.
            gix::discover::Error::Discover(
                Upwards::NoGitRepository { .. }
                | Upwards::NoGitRepositoryWithinFs { .. }
                | Upwards::NoGitRepositoryWithinCeiling { .. }
                | Upwards::NoMatchingCeilingDir,
            ) => OpenError::NotARepo { path },
            // The search could not start. `gix` keeps no `io::Error` for
            // either of these, so neither can become an `Io`.
            gix::discover::Error::Discover(
                Upwards::InaccessibleDirectory { .. } | Upwards::InvalidInput { .. },
            ) => OpenError::Inaccessible { path },
            gix::discover::Error::Discover(
                Upwards::CurrentDir(source) | Upwards::CheckTrust { err: source, .. },
            ) => OpenError::Io { path, source },
            // A repository was found and rejected: an unreadable config, an
            // unsupported format, or one owned by another user. `gix`'s own
            // message is the specific one, so it is kept as the source.
            other => OpenError::Unusable {
                path,
                source: Box::new(other),
            },
        }
    }

    /// The path Graft was asked to open, whichever way the open failed.
    #[must_use]
    pub fn path(&self) -> &Path {
        match self {
            OpenError::NotARepo { path }
            | OpenError::Inaccessible { path }
            | OpenError::Io { path, .. }
            | OpenError::Unusable { path, .. } => path,
        }
    }
}

/// Why `HEAD` could not be read.
#[derive(Debug, thiserror::Error)]
pub enum HeadError {
    /// `HEAD` itself could not be read.
    #[error("cannot read HEAD in {}: {source}", git_dir.display())]
    Read {
        /// The repository whose `HEAD` was being read.
        git_dir: PathBuf,
        /// What `gix` said. Boxed to keep `HeadError` small.
        source: Box<gix::reference::find::existing::Error>,
    },
    /// `HEAD` was read, but names something git's own rules reject: a ref name
    /// with a control character in it, or an object id of the wrong length.
    #[error("HEAD in {} is malformed: {source}", git_dir.display())]
    Malformed {
        /// The repository whose `HEAD` was being read.
        git_dir: PathBuf,
        /// Which invariant the value broke.
        source: graft_core::error::Error,
    },
}

#[cfg(test)]
mod tests;
