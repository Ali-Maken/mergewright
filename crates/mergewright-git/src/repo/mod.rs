//! Opening a repository, and the handful of facts that are true the moment it
//! is open: where it lives, and where `HEAD` points.
//!
//! Everything here is a read, and every read goes through `gix` (ADR-0011).
//! Paths stay `Path`/`PathBuf` end to end — never `String` — because a
//! repository can sit at a path that is not valid UTF-8 (PROMPT.md §6.3), and
//! converting early would mean opening the wrong directory or refusing to open
//! a real one. Turning any of this into text for the screen is `app`'s job, at
//! the view-model boundary, through `mergewright_term::sanitize_path` (DESIGN.md
//! §8.2).
//!
//! Split across files by concern (m1-the-graph/74a): this file holds
//! opening and `HEAD`; `refs` and `peel` the ref-reading methods
//! (public queries and their private peeling helpers); `types` the ref
//! and tip types and errors; `shared` `SharedRepo`.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use mergewright_core::oid::Oid;
use mergewright_core::ref_name::RefName;

use crate::run::{GitArg, GitInvocation, RunError};

pub use head_error::HeadError;

mod head_error;
mod peel;
mod refs;
mod shared;
mod types;

pub use shared::SharedRepo;
pub use types::{RefEntry, RefSnapshot, RefTarget, RefsError, TipStats, TipsError};

/// What each object a ref points straight at peeled to, so refs sharing a
/// target are peeled once.
type KnownTips = std::collections::HashMap<gix::ObjectId, Option<Oid>>;

/// Says whether an id is a commit without reading its object, when something
/// already knows.
///
/// A tip whose id is not a commit's header away but a *position* in an
/// already-open table (ADR-0026's [`CommitTable`](crate::history::CommitTable))
/// costs nothing to identify. `false` means "not known here": the caller
/// reads the header, exactly as it always did.
pub trait KnownCommits {
    /// Whether `id` is already known to be a commit.
    fn is_known_commit(&self, id: Oid) -> bool;

    /// Whether this knows (nearly) every commit the refs reach, so an id it
    /// does not know is most likely a tag, a tree or a blob: the caller then
    /// reads it whole at once instead of its header first
    /// (m1-the-graph/58b). Changes what is read, never the answer.
    fn covers_the_graph(&self) -> bool {
        false
    }
}

impl KnownCommits for () {
    fn is_known_commit(&self, _id: Oid) -> bool {
        false
    }
}

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
    ///
    /// It knows the user's `user.email` as git resolves it (repository,
    /// global and system config), for the graph's `you` (ease-pass G8);
    /// unset, no commit reads `you`.
    #[must_use]
    pub fn mailmap(&self) -> crate::mailmap::Mailmap {
        let me = self.repo.config_snapshot().string("user.email");
        crate::mailmap::Mailmap::from_snapshot(self.repo.open_mailmap())
            .with_me(me.as_deref().map(AsRef::as_ref))
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
    /// The commit `HEAD` resolves to, symbolic refs followed; `None` while
    /// the branch is unborn. Read on every call, as [`Self::head`].
    ///
    /// # Errors
    /// [`HeadError`] if `HEAD` cannot be read or resolved.
    pub fn head_commit(&self) -> Result<Option<Oid>, HeadError> {
        let git_dir = || self.git_dir().to_path_buf();
        let mut head = self.repo.head().map_err(|source| HeadError::Read {
            git_dir: git_dir(),
            source: Box::new(source),
        })?;
        let id = head.try_peel_to_id().map_err(|source| HeadError::Peel {
            git_dir: git_dir(),
            source: Box::new(source),
        })?;
        id.map(|id| Oid::from_bytes(id.as_bytes()))
            .transpose()
            .map_err(|source| HeadError::Malformed {
                git_dir: git_dir(),
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

    /// Where a Mergewright-owned file lives under this worktree's `git_path`:
    /// `git rev-parse --git-path <relative>`, resolved to an absolute path.
    ///
    /// On the main worktree git prints a path relative to `-C`, which is
    /// joined against [`workdir`](Self::workdir) (or [`git_dir`](Self::git_dir)
    /// when bare); on a linked worktree it already prints absolute (verified
    /// on git 2.54). `--git-path` cannot be built from a [`GitArg::Path`] —
    /// that variant's automatic `--` insertion breaks the invocation — so
    /// this passes it as two [`GitArg::Word`]s instead.
    ///
    /// # Errors
    /// [`RunError`] if `git` could not be run or exited non-zero.
    pub fn git_path(&self, relative: &Path) -> Result<PathBuf, RunError> {
        let output = GitInvocation::new(self)
            .args([
                GitArg::Flag("rev-parse"),
                GitArg::Word("--git-path".to_owned()),
                GitArg::Word(relative.to_string_lossy().into_owned()),
            ])
            .run(&())?;
        let text = String::from_utf8_lossy(&output.stdout);
        let printed = Path::new(text.trim());
        Ok(if printed.is_absolute() {
            printed.to_path_buf()
        } else {
            self.workdir()
                .unwrap_or_else(|| self.git_dir())
                .join(printed)
        })
    }

    /// This worktree's name: `"main"` on the main worktree (where
    /// [`git_dir`](Self::git_dir) and [`common_dir`](Self::common_dir) are
    /// the same directory), else the last component of `git_dir` —
    /// `<main>/.git/worktrees/<name>`, the name `git worktree add` was given.
    #[must_use]
    pub fn worktree_name(&self) -> String {
        if self.git_dir() == self.common_dir() {
            return "main".to_owned();
        }
        self.git_dir().file_name().map_or_else(
            || "main".to_owned(),
            |name| name.to_string_lossy().into_owned(),
        )
    }
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
        /// The path Mergewright was asked to open.
        path: PathBuf,
    },
    /// The path itself is unusable as a starting point: it does not exist, is
    /// not a directory, or reaches above the root of the filesystem.
    #[error("{} is not a directory Mergewright can read", path.display())]
    Inaccessible {
        /// The path Mergewright was asked to open.
        path: PathBuf,
    },
    /// Something on the way up from the path could not be read.
    #[error("cannot read {}: {source}", path.display())]
    Io {
        /// The path Mergewright was asked to open.
        path: PathBuf,
        /// What the operating system said.
        source: io::Error,
    },
    /// A repository was found, but git itself could not use it — a broken
    /// config file, an unsupported repository format.
    #[error("the repository above {} cannot be opened: {source}", path.display())]
    Unusable {
        /// The path Mergewright was asked to open.
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

    /// The path Mergewright was asked to open, whichever way the open failed.
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

#[cfg(test)]
mod tests;
