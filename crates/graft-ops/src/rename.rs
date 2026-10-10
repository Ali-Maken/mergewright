//! The one-time move from the old on-disk names to the new ones
//! (ADR-0057, Amendment 2026-10-09; m8-polish-and-ship/26 Part B).
//!
//! A repository the app used before the rename keeps its backups under
//! `refs/graft/` and its files under `…/graft/`. [`settle_names`] moves them,
//! once, the first time the repository is opened by a build that knows the
//! new names: refs to `refs/tributary/`, every `graft/` folder of the
//! repository — the common dir's and **every** worktree's, opened or not —
//! to `tributary/`. The scope is the repository, never one worktree: the refs
//! are shared, so a half-moved repository would read one worktree's journal
//! against refs that are no longer where it says.
//!
//! The order, which is the whole of this module's correctness:
//!
//! 1. lock `<common_dir>/tributary/lock` alone (`try`, never wait);
//! 2. create every new ref in one `update-ref --stdin` (a new ref already at
//!    the old value is skipped; at another value, refuse);
//! 3. read the new refs back;
//! 4. copy every old folder, file by file, and read each copy back (a
//!    journal is also parsed and compared entry by entry);
//! 5. write the marker `switched` — **the switch**: from here the new names
//!    are in force for every opener;
//! 6. append the `Migrate` entry to the opening worktree's new journal;
//! 7. delete the old refs (each expected at its old value);
//! 8. remove the old folders;
//! 9. write the marker `clean`.
//!
//! A failure before 5 leaves the old names in force with nothing old
//! deleted. A failure after 5 leaves the new names in force; the leftovers
//! are removed on the next open, which finds the marker still `switched`.
//! Nothing here touches the working tree or the index.
//!
//! **Two windows.** A run that stays on the old names holds the lock shared
//! for its life ([`OldNamesHold`]), so a second window can never move the
//! names while the first still writes the old ones. A run that finds the
//! lock taken alone waits for it (the kernel's wait, not a poll) and reads
//! the marker again.

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use graft_core::journal::JournalEntry;
use graft_core::on_disk::OnDiskNames;
use graft_git::file_lock::FileLock;
use graft_git::repo::RepoHandle;

mod marker;
mod record;
mod refs;
mod trees;

use marker::Marker;
pub use record::description;

/// The lock file, in `<common_dir>/<OnDiskNames::New.dir()>/`.
pub const LOCK_FILE: &str = "lock";

/// What [`settle_names`] found and did.
#[derive(Debug)]
pub struct Settled {
    /// The names in force now. When they differ from the handle's, the
    /// caller reopens the repository: a handle never re-reads the marker.
    pub names: OnDiskNames,
    /// What happened, for the status row.
    pub outcome: SettleOutcome,
    /// Held for the life of a run that stays on the old names; `None`
    /// otherwise.
    pub hold: Option<OldNamesHold>,
}

/// What [`settle_names`] did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettleOutcome {
    /// Already on the new names, or nothing of the old names was there to
    /// move. Nothing to say.
    Done,
    /// Moved, all of it. `not_journaled` says why the `Migrate` entry could
    /// not be written, when it could not; the move itself stands.
    Migrated {
        /// Refs moved.
        refs: usize,
        /// Files moved, across every folder.
        files: usize,
        /// Why the journal has no entry for the move.
        not_journaled: Option<String>,
    },
    /// An earlier move was switched over but not finished; this open
    /// removed what it left.
    CleanedUp {
        /// Old refs deleted.
        refs: usize,
        /// Old files removed.
        files: usize,
    },
    /// The move did not happen; the old names stay in force, nothing old was
    /// deleted, and the next open tries again.
    KeptOld(MigrateError),
    /// The new names are in force, but some of the old ones are still on
    /// disk; the next open removes them.
    LeftOld(MigrateError),
}

/// The lock held shared by a run on the old names. Dropping it releases it.
#[derive(Debug)]
pub struct OldNamesHold {
    _lock: FileLock,
}

/// Why the move stopped.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum MigrateError {
    /// Someone else holds the lock: another window is open on this
    /// repository.
    #[error("another trib is open on this repository")]
    Busy,
    /// git refused or failed a ref read or write.
    #[error("{words}")]
    Refs {
        /// git's words, or ours.
        words: String,
    },
    /// A file or folder could not be read, written or removed.
    #[error("{}: {words}", path.display())]
    Files {
        /// The path.
        path: PathBuf,
        /// What went wrong.
        words: String,
    },
    /// A copy did not read back the same as its original.
    #[error("{what} read back different")]
    ReadBack {
        /// What was compared.
        what: String,
    },
}

impl MigrateError {
    fn files(path: &Path, error: &std::io::Error) -> Self {
        MigrateError::Files {
            path: path.to_path_buf(),
            words: error.to_string(),
        }
    }
}

/// Settle which names `repo` uses, moving the old ones to the new if they
/// are still there. Called once per open, before any job sees the
/// repository. With the marker reading `clean` it reads that one file and
/// spawns nothing.
#[must_use]
pub fn settle_names(repo: &RepoHandle) -> Settled {
    let common = repo.common_dir();
    match marker::read(common) {
        Ok(Some(Marker::Clean)) => new_names(SettleOutcome::Done),
        Ok(Some(Marker::Switched)) => match lock_alone(common) {
            Ok(Some(alone)) => clean_up(repo, &alone),
            Ok(None) => new_names(SettleOutcome::LeftOld(MigrateError::Busy)),
            Err(error) => new_names(SettleOutcome::LeftOld(error)),
        },
        // The marker exists (graft-git's open said so or will) but cannot be
        // read: the new names are in force, and what is left is unknown.
        Err(error) => new_names(SettleOutcome::LeftOld(error)),
        Ok(None) => match lock_alone(common) {
            Ok(Some(alone)) => migrate(repo, alone),
            Ok(None) => stay_old(common, MigrateError::Busy),
            Err(error) => stay_old(common, error),
        },
    }
}

/// `name` as the names in force spell it: under the new names, a ref the
/// journal recorded under the old prefix is now under the new one. The
/// journal's bytes are never rewritten; every reader maps at read.
#[must_use]
pub fn map_ref(name: &str, names: OnDiskNames) -> Cow<'_, str> {
    let (old, new) = (OnDiskNames::Old.ref_prefix(), OnDiskNames::New.ref_prefix());
    match (names, name.strip_prefix(old)) {
        (OnDiskNames::New, Some(rest)) => Cow::Owned(format!("{new}{rest}")),
        _ => Cow::Borrowed(name),
    }
}

/// Every ref name an entry holds, through [`map_ref`]: the backup ref, the
/// touched refs, the before/after maps and a fetch's moved refs.
pub fn map_entry(entry: &mut JournalEntry, names: OnDiskNames) {
    if names == OnDiskNames::Old {
        return;
    }
    let map = |name: &String| map_ref(name, names).into_owned();
    if let Some(backup) = entry.backup_ref.as_mut() {
        *backup = map(backup);
    }
    for name in &mut entry.touches.refs {
        *name = map(name);
    }
    entry.refs_before = std::mem::take(&mut entry.refs_before)
        .into_iter()
        .map(|(name, oid)| (map(&name), oid))
        .collect();
    entry.refs_after = std::mem::take(&mut entry.refs_after)
        .into_iter()
        .map(|(name, oid)| (map(&name), oid))
        .collect();
    if let Some(fetched) = entry.fetched_refs.take() {
        entry.fetched_refs = Some(
            fetched
                .into_iter()
                .map(|(name, moved)| (map(&name), moved))
                .collect(),
        );
    }
}

/// [`map_entry`] over a whole journal.
pub fn map_entries(entries: &mut [JournalEntry], names: OnDiskNames) {
    for entry in entries {
        map_entry(entry, names);
    }
}

fn new_names(outcome: SettleOutcome) -> Settled {
    Settled {
        names: OnDiskNames::New,
        outcome,
        hold: None,
    }
}

/// `<common_dir>/tributary/`, made if missing, and the lock in it taken
/// alone if nobody else holds it.
fn lock_alone(common: &Path) -> Result<Option<FileLock>, MigrateError> {
    let dir = common.join(OnDiskNames::New.dir());
    std::fs::create_dir_all(&dir).map_err(|error| MigrateError::files(&dir, &error))?;
    let path = dir.join(LOCK_FILE);
    FileLock::try_exclusive(&path).map_err(|error| MigrateError::files(&path, &error))
}

/// Stay on the old names for this run: take the lock shared (waiting out a
/// move in progress), then read the marker again — a move that finished
/// while this waited puts the new names in force after all.
fn stay_old(common: &Path, why: MigrateError) -> Settled {
    let path = common.join(OnDiskNames::New.dir()).join(LOCK_FILE);
    let hold = match FileLock::shared(&path) {
        Ok(lock) => lock,
        // Still nothing moved, and nothing now keeps another window from
        // moving the names under this run: both go on the status row.
        Err(error) => {
            return Settled {
                names: OnDiskNames::Old,
                outcome: SettleOutcome::KeptOld(MigrateError::Files {
                    path,
                    words: format!("{why}; the lock could not be held either: {error}"),
                }),
                hold: None,
            };
        }
    };
    match marker::read(common) {
        Ok(Some(_)) => new_names(SettleOutcome::Done),
        Ok(None) => Settled {
            names: OnDiskNames::Old,
            outcome: SettleOutcome::KeptOld(why),
            hold: Some(OldNamesHold { _lock: hold }),
        },
        Err(error) => new_names(SettleOutcome::LeftOld(error)),
    }
}

/// Steps 2–9 under the lock held alone.
fn migrate(repo: &RepoHandle, alone: FileLock) -> Settled {
    let common = repo.common_dir();
    // Another window may have finished the move between the first read and
    // the lock.
    match marker::read(common) {
        Ok(None) => {}
        Ok(Some(Marker::Clean)) => return new_names(SettleOutcome::Done),
        Ok(Some(Marker::Switched)) => return clean_up(repo, &alone),
        Err(error) => return new_names(SettleOutcome::LeftOld(error)),
    }
    let copied = match copy_everything(repo) {
        Ok(copied) => copied,
        Err(why) => {
            drop(alone);
            return stay_old(common, why);
        }
    };
    if copied.refs.is_empty() && copied.trees.is_empty() {
        // Never used under the old names: nothing to move, nothing to
        // journal. The marker alone puts the new names in force.
        return match marker::write(common, Marker::Clean) {
            Ok(()) => new_names(SettleOutcome::Done),
            Err(why) => {
                drop(alone);
                stay_old(common, why)
            }
        };
    }
    if let Err(why) = marker::write(common, Marker::Switched) {
        drop(alone);
        return stay_old(common, why);
    }
    // Switched: the new names are in force from here, whatever follows.
    let not_journaled =
        record::append(repo, copied.refs.len(), copied.files, &copied.command).err();
    if let Err(why) = remove_old(repo, &copied.refs, &copied.trees) {
        return new_names(SettleOutcome::LeftOld(why));
    }
    if let Err(why) = marker::write(common, Marker::Clean) {
        return new_names(SettleOutcome::LeftOld(why));
    }
    new_names(SettleOutcome::Migrated {
        refs: copied.refs.len(),
        files: copied.files,
        not_journaled,
    })
}

/// What steps 2–4 made.
struct Copied {
    /// Every old ref and its value, each now also under the new prefix.
    refs: refs::Listed,
    /// Every old folder, each now also under the new name.
    trees: Vec<trees::Tree>,
    /// Files copied, across every folder.
    files: usize,
    /// The create transaction's command line, for the journal entry.
    command: String,
}

/// Steps 2–4: every new ref created and read back, every old folder copied
/// and read back. Deletes nothing.
fn copy_everything(repo: &RepoHandle) -> Result<Copied, MigrateError> {
    let old = refs::list(repo, OnDiskNames::Old)?;
    let command = refs::create_and_read_back(repo, &old)?;
    let trees = trees::old_trees(repo.common_dir())?;
    let mut files = 0;
    for tree in &trees {
        files += trees::copy(tree)?;
    }
    Ok(Copied {
        refs: old,
        trees,
        files,
        command,
    })
}

/// Steps 7–8.
fn remove_old(
    repo: &RepoHandle,
    old: &refs::Listed,
    old_trees: &[trees::Tree],
) -> Result<(), MigrateError> {
    refs::delete(repo, old)?;
    for tree in old_trees {
        trees::remove(tree)?;
    }
    Ok(())
}

/// The marker reads `switched`: remove what the switched move left — each
/// old ref whose new copy holds the same value, every old folder — and
/// write `clean`. Creates nothing.
fn clean_up(repo: &RepoHandle, _alone: &FileLock) -> Settled {
    let cleaned = (|| {
        let old = refs::list(repo, OnDiskNames::Old)?;
        let new = refs::list(repo, OnDiskNames::New)?;
        let copied: refs::Listed = old
            .iter()
            .filter(|(name, oid)| new.get(&*map_ref(name, OnDiskNames::New)) == Some(oid))
            .map(|(name, oid)| (name.clone(), *oid))
            .collect();
        let trees = trees::old_trees(repo.common_dir())?;
        let mut files = 0;
        for tree in &trees {
            files += trees::count(tree)?;
        }
        remove_old(repo, &copied, &trees)?;
        if copied.len() != old.len() {
            return Err(MigrateError::ReadBack {
                what: format!(
                    "{} old ref(s) without the same new one",
                    old.len() - copied.len()
                ),
            });
        }
        marker::write(repo.common_dir(), Marker::Clean)?;
        Ok((copied.len(), files))
    })();
    match cleaned {
        Ok((refs, files)) => new_names(SettleOutcome::CleanedUp { refs, files }),
        Err(why) => new_names(SettleOutcome::LeftOld(why)),
    }
}

#[cfg(test)]
mod tests;
