//! The practice tour's repository (m8-polish-and-ship/19a, ADR-0056 §3): a
//! throwaway repo built under the temp folder for eight tasks done by doing,
//! the guard that keeps every write and the removal inside it, and the ticks
//! read from its own journal.
//!
//! **Not an [`Operation`](crate::operation::Operation)**, as `init`: building
//! writes no journal entry in any repository, and it runs git through
//! [`GitInvocation::in_folder`], so an inherited `GIT_DIR` cannot send a
//! write into his repository.
//!
//! **The path guard is the one place a wrong approach deletes his files.**
//! Every file write and every git folder goes through [`inside`]; removal
//! goes through [`remove_practice_repo`]'s checks and nothing else. There is
//! no sweep of the temp folder by name: another Graft's live tour lives
//! there too.

mod build;
mod tasks;
#[cfg(test)]
mod tests;

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

pub use build::build_practice_repo;
pub use tasks::{TourTask, tasks_done};

use graft_core::journal::{EntryId, EntryStatus, HeadRef, JournalEntry, OperationKind};
use graft_core::oid::Oid;
use graft_core::on_disk::Generation;
use graft_git::history_reads::{Forecast, forecast_merge};
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation};
use graft_git::version::system_git;

use crate::journal::{JournalError, journal_path, parse};

/// When the practice repo's commits are dated (m8/19c R2). Every built
/// commit gets its own minute, strictly increasing in build order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PracticeAt {
    /// The app's: the last commit is a minute before `now_unix_secs` (the
    /// session clock's). Folder `mergewright-practice-<pid>-<unix_ms>`.
    Now {
        /// The instant to end before, Unix seconds.
        now_unix_secs: i64,
    },
    /// Reproducible: the first commit at `first_unix_secs`, folder
    /// `mergewright-practice-<first_unix_secs>`, so two builds at the same
    /// instant give the same commits and the same drawn path.
    Fixed {
        /// The first commit's date, Unix seconds.
        first_unix_secs: i64,
    },
}

/// The file at a practice repo's root that marks it as one. Removal refuses a
/// folder without it (as a regular file, not a link).
pub const PRACTICE_MARKER: &str = ".practice";

/// `<dir>-practice-`: a new practice root's name starts with this for
/// [`Generation::CURRENT`] (`mergewright-practice-`), then `<pid>-<unix_ms>`
/// ([`PracticeAt::Now`]) or `<first_unix_secs>` ([`PracticeAt::Fixed`]).
/// Removal accepts every generation's prefix, so a rename never makes the
/// guard refuse a root this build recorded. Nothing scans for older
/// folders: one an earlier build left ages out of the temp dir.
#[must_use]
pub fn practice_prefix(generation: Generation) -> String {
    format!("{}-practice-", generation.dir())
}

/// A built practice repo. Only [`build_practice_repo`] makes one, so holding
/// one means the folder was made by us, under `under`, with the marker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PracticeRepo {
    /// `<under>/mergewright-practice-<pid>-<unix_ms>`, canonical.
    root: PathBuf,
    /// `<root>/work`: the repository the app opens.
    work: PathBuf,
    /// `<root>/remote.git`: bare, `origin` of `work`.
    remote: PathBuf,
    /// The canonical folder `root` was made in.
    under: PathBuf,
}

impl PracticeRepo {
    /// The practice folder: everything the tour made is under it.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The working tree the app opens.
    #[must_use]
    pub fn work(&self) -> &Path {
        &self.work
    }
}

/// Why the tour could not build, read or remove its repo.
#[derive(Debug, thiserror::Error)]
pub enum TourError {
    /// The temp folder could not be made, written or read.
    #[error("couldn't make the practice folder: {0}")]
    TempDir(std::io::Error),
    /// git refused a step of the build, or a read of the practice repo.
    #[error("{words}")]
    Git {
        /// git's reason, or the runner's.
        words: String,
    },
    /// The folder is not one the tour made: no marker, a linked marker, a
    /// linked root, or the wrong name. Nothing was removed.
    #[error("{} is not a practice repo", .0.display())]
    NotPractice(PathBuf),
    /// A path that would land outside the practice folder (or a practice
    /// folder outside the folder it was made in). Nothing was written or
    /// removed.
    #[error("{} is outside {}", path.display(), root.display())]
    Outside {
        /// The refused path.
        path: PathBuf,
        /// The folder it had to be under.
        root: PathBuf,
    },
    /// `remove_dir_all` failed part way; what is left is still the practice
    /// folder.
    #[error("couldn't remove it: {0}")]
    Remove(std::io::Error),
    /// The practice repo's journal could not be read.
    #[error("couldn't read its journal: {0}")]
    Journal(JournalError),
    /// The build failed and the half-built folder could not be removed
    /// either. Both reasons are kept; neither is dropped.
    #[error("{cause} · {} was left behind: {left_why}", left.display())]
    Left {
        /// Why the build failed.
        cause: Box<TourError>,
        /// The half-built practice folder still on disk.
        left: PathBuf,
        /// Why removing it failed.
        left_why: Box<TourError>,
    },
}

impl TourError {
    /// The `<reason>` in the status `tour: <reason>`. Not sanitised: the
    /// caller does that, as every other error line.
    #[must_use]
    pub fn explain(&self) -> String {
        self.to_string()
    }
}

/// `root.join(rel)`, refused unless `rel` is a plain relative path of named
/// parts: no absolute path, no `..`, no `.`, no drive prefix, not empty. The
/// one door every write in this module goes through.
///
/// # Errors
/// [`TourError::Outside`] for any refused `rel`.
fn inside(root: &Path, rel: &Path) -> Result<PathBuf, TourError> {
    let outside = || TourError::Outside {
        path: rel.to_path_buf(),
        root: root.to_path_buf(),
    };
    let mut parts = 0usize;
    for component in rel.components() {
        match component {
            Component::Normal(_) => parts += 1,
            Component::Prefix(_)
            | Component::RootDir
            | Component::CurDir
            | Component::ParentDir => return Err(outside()),
        }
    }
    if parts == 0 {
        return Err(outside());
    }
    let joined = root.join(rel);
    if !joined.starts_with(root) {
        return Err(outside());
    }
    Ok(joined)
}

/// Remove a practice repo, after checking it is one: its root, canonical, is
/// the recorded root; the root's parent is the folder it was made in; its
/// name starts with [`practice_prefix`] of any generation; the root is not a link; and
/// `<root>/.practice` is a regular file. Any miss removes nothing.
/// `remove_dir_all` does not follow links inside.
///
/// # Errors
/// [`TourError::NotPractice`] / [`TourError::Outside`] when a check misses;
/// [`TourError::Remove`] when removal fails part way.
pub fn remove_practice_repo(repo: &PracticeRepo) -> Result<(), TourError> {
    check_practice(repo)?;
    std::fs::remove_dir_all(&repo.root).map_err(TourError::Remove)
}

/// The removal checks, in an order where each check's premise holds.
fn check_practice(repo: &PracticeRepo) -> Result<(), TourError> {
    let not_practice = || TourError::NotPractice(repo.root.clone());
    // A root that cannot even be looked at is not one we can vouch for:
    // refused, and the refusal names it.
    let root_meta = std::fs::symlink_metadata(&repo.root).map_err(|_unreadable| not_practice())?;
    if root_meta.file_type().is_symlink() || !root_meta.is_dir() {
        return Err(not_practice());
    }
    let canonical = repo.root.canonicalize().map_err(TourError::Remove)?;
    if canonical != repo.root {
        return Err(not_practice());
    }
    let under = repo.under.canonicalize().map_err(TourError::Remove)?;
    if canonical.parent() != Some(under.as_path()) {
        return Err(TourError::Outside {
            path: canonical,
            root: under,
        });
    }
    let named = canonical
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            Generation::GENERATIONS
                .iter()
                .any(|generation| name.starts_with(&practice_prefix(*generation)))
        });
    if !named {
        return Err(not_practice());
    }
    let marker = inside(&canonical, Path::new(PRACTICE_MARKER))?;
    // A missing or unreadable marker is no marker: refused, not an error.
    let marker_is_file =
        std::fs::symlink_metadata(&marker).is_ok_and(|meta| meta.file_type().is_file());
    if !marker_is_file {
        return Err(not_practice());
    }
    Ok(())
}

/// Which of the eight tasks the practice repo's journal shows done: reads
/// `<work>`'s journal file (no write: a torn tail is left for the app's own
/// `Journal::open`), then [`tasks_done`]. No journal yet → nothing done.
///
/// # Errors
/// [`TourError::Git`] when `work` does not open or its journal path cannot be
/// resolved; [`TourError::Journal`] when the file cannot be read or parsed.
pub fn tour_progress(work: &Path) -> Result<[bool; 8], TourError> {
    let repo = RepoHandle::open(work).map_err(|error| TourError::Git {
        words: error.to_string(),
    })?;
    let path = journal_path(&repo).map_err(|error| TourError::Git {
        words: error.to_string(),
    })?;
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok([false; 8]),
        Err(why) => return Err(TourError::Journal(why.into())),
    };
    let mut entries = parse(&bytes).map_err(TourError::Journal)?.entries;
    crate::rename::map_entries(&mut entries, repo.on_disk_names());
    let conflicted = conflicted_merges(&repo, &entries)?;
    Ok(tasks_done(&entries, &conflicted))
}

/// The committed `Merge` entries whose result git could not have made
/// alone (m8/19c R1): the result commit has two parents and
/// `forecast_merge` of those parents answers `Conflicts`.
///
/// `forecast_merge` **writes** the merged tree into `repo`'s object store.
/// That is the practice repo's only: `tour_progress` is only ever called on
/// `PracticeRepo::work()`.
fn conflicted_merges(
    repo: &RepoHandle,
    entries: &[JournalEntry],
) -> Result<BTreeSet<EntryId>, TourError> {
    let git_failed = |error: &dyn std::fmt::Display| TourError::Git {
        words: error.to_string(),
    };
    let mut conflicted = BTreeSet::new();
    let mut version = None;
    for merge in entries
        .iter()
        .filter(|e| e.kind == OperationKind::Merge && e.status == EntryStatus::Committed)
    {
        // No branch HEAD after, or no recorded value for it: there is no
        // recorded result to judge, so no proof of a resolution. The tick
        // is withheld; nothing failed.
        let Some(HeadRef::Branch(name)) = &merge.head_after else {
            continue;
        };
        let Some(Some(result)) = merge.refs_after.get(name) else {
            continue;
        };
        let parents = parents_of(repo, *result)?;
        // One parent (a fast-forward) or an octopus: not a two-way merge
        // someone resolved.
        let [ours, theirs] = parents[..] else {
            continue;
        };
        let git = if let Some(found) = version {
            found
        } else {
            let found = system_git().map_err(|e| git_failed(&e))?.version();
            version = Some(found);
            found
        };
        match forecast_merge(repo, &git, None, ours, theirs, &()).map_err(|e| git_failed(&e))? {
            Forecast::Conflicts { .. } => {
                conflicted.insert(merge.id.clone());
            }
            Forecast::Clean => {}
            // Graft's floor (2.38) checks the plain form, so this is an
            // answer not recognised, not an old git: an error, never a tick.
            Forecast::NotCheckable(why) => return Err(TourError::Git { words: why }),
        }
    }
    Ok(conflicted)
}

/// `git rev-list --parents -n 1 <oid>`: the commit's parents.
fn parents_of(repo: &RepoHandle, commit: Oid) -> Result<Vec<Oid>, TourError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("rev-list"),
            GitArg::Flag("--parents"),
            GitArg::Flag("-n"),
            GitArg::Flag("1"),
            GitArg::Oid(commit),
        ])
        .run(&())
        .map_err(|error| TourError::Git {
            words: error.to_string(),
        })?;
    let line = String::from_utf8_lossy(&output.stdout);
    line.split_whitespace()
        .skip(1)
        .map(|hex| {
            Oid::from_hex(hex).map_err(|error| TourError::Git {
                words: error.to_string(),
            })
        })
        .collect()
}
