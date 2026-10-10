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
//! 2. make the new refs an exact copy of the old in one `update-ref
//!    --stdin` — before the switch the new names are the move's own
//!    scratch, so a stale one is overwritten and an extra one deleted;
//! 3. read the new refs back;
//! 4. make every new folder an exact copy of its old one, file by file,
//!    each copy read back, a copied journal's backups checked, every folder
//!    touched synced;
//! 5. write the marker `switched` — **the switch**: from here the new names
//!    are in force for every opener;
//! 6. append the `Migrate` entry to the opening worktree's new journal;
//! 7. and 8. together, the stray pass (`strays`): each old ref and file
//!    deleted when its new copy holds the same, carried when it has none,
//!    and kept aside in `older-version/<n>/` when it differs — an older
//!    build wrote it after step 4;
//! 9. write the marker `clean`.
//!
//! A failure before 5 leaves the old names in force with nothing old
//! deleted, and the next attempt starts from the old names again. A
//! failure after 5 leaves the new names in force; the stray pass runs again
//! on the next open, which finds the marker still `switched`. A `clean`
//! open that finds old names on disk again (`stat`, no spawn) runs the
//! stray pass too. Nothing here touches the working tree or the index.
//!
//! **Two windows.** A run that stays on the old names holds the lock shared
//! for its life ([`OldNamesHold`]), so a second window can never move the
//! names while the first still writes the old ones. A run that finds the
//! lock taken alone waits for it (the kernel's wait, not a poll) and reads
//! the marker again.

use std::path::{Path, PathBuf};

use graft_core::on_disk::{MIGRATED_MARKER, OnDiskNames};
use graft_git::file_lock::FileLock;
use graft_git::repo::RepoHandle;

mod journals;
mod map;
mod marker;
mod record;
mod refs;
mod strays;
mod trees;

pub use map::{map_entries, map_entry, map_ref};
use marker::Marker;
pub use record::description;
use strays::{Strays, old_names_present, settle_strays};

/// The lock file, in `<common_dir>/<OnDiskNames::New.dir()>/`.
pub const LOCK_FILE: &str = "lock";

/// Where the stray pass keeps what an older build wrote under the old
/// names after its copy was taken: `<common>/<New.dir()>/older-version/<n>/`
/// for files, `refs/<New.dir()>/older-version/<n>/` for refs.
pub const KEPT_ASIDE_DIR: &str = "older-version";

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
        /// Where what an older build wrote during the move was kept.
        kept_aside: Option<PathBuf>,
    },
    /// An earlier move was switched over but not finished; this open
    /// carried or removed what it left.
    CleanedUp {
        /// Old refs handled.
        refs: usize,
        /// Old files handled.
        files: usize,
        /// Where what differed from its new copy was kept.
        kept_aside: Option<PathBuf>,
    },
    /// Already moved, yet an older build — still open on the repository —
    /// wrote under the old names again; this open carried or kept it.
    OlderWrote {
        /// Old refs handled.
        refs: usize,
        /// Old files handled.
        files: usize,
        /// Where what differed from its new copy was kept.
        kept_aside: Option<PathBuf>,
    },
    /// The move did not happen; the old names stay in force, nothing old was
    /// deleted, and the next open tries again.
    KeptOld(MigrateError),
    /// The new names are in force, but some of the old ones are still on
    /// disk; the next open passes over them again. A stray pass that fails
    /// or finds the lock taken lands here too.
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
/// repository. With the marker reading `clean` and nothing under the old
/// names, it reads that file and three folders and spawns nothing.
#[must_use]
pub fn settle_names(repo: &RepoHandle) -> Settled {
    let common = repo.common_dir();
    match marker::read(common) {
        Ok(Some(Marker::Clean)) => match old_names_present(repo) {
            Ok(false) => new_names(SettleOutcome::Done),
            Ok(true) => match lock_alone(common) {
                Ok(Some(alone)) => older_wrote(repo, &alone),
                Ok(None) => new_names(SettleOutcome::LeftOld(MigrateError::Busy)),
                Err(error) => new_names(SettleOutcome::LeftOld(error)),
            },
            Err(error) => new_names(SettleOutcome::LeftOld(error)),
        },
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
    if copied.refs == 0 && copied.trees == 0 {
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
    let not_journaled = record::append(repo, copied.refs, copied.files, &copied.command).err();
    // Steps 7–8: the stray pass deletes each old copy that reads back the
    // same and keeps aside what an older build wrote since step 4.
    let strays = match settle_strays(repo) {
        Ok(strays) => strays,
        Err(why) => return new_names(SettleOutcome::LeftOld(why)),
    };
    if let Err(why) = marker::write(common, Marker::Clean) {
        return new_names(SettleOutcome::LeftOld(why));
    }
    new_names(SettleOutcome::Migrated {
        refs: copied.refs,
        files: copied.files,
        not_journaled,
        kept_aside: strays.kept_aside,
    })
}

/// What steps 2–4 made.
struct Copied {
    /// Old refs, each now also under the new prefix.
    refs: usize,
    /// Old folders, each now also under the new name.
    trees: usize,
    /// Files copied, across every folder.
    files: usize,
    /// The mirror transaction's command line, for the journal entry.
    command: String,
}

/// Steps 2–4: the new refs and folders made exact copies of the old, read
/// back, and every folder touched synced. Deletes nothing old; deletes
/// what an earlier, failed attempt left under the new names.
fn copy_everything(repo: &RepoHandle) -> Result<Copied, MigrateError> {
    let common = repo.common_dir();
    let old = refs::list(repo, OnDiskNames::Old)?;
    let (command, new_refs) = refs::mirror_and_read_back(repo, &old)?;
    let new_dir = common.join(OnDiskNames::New.dir());
    let keep = [new_dir.join(LOCK_FILE), new_dir.join(MIGRATED_MARKER)];
    let trees = trees::old_trees(common)?;
    let mut files = 0;
    for tree in &trees {
        let copied = trees::mirror(tree, &keep)?;
        journals::check_journals(&copied, &new_refs)?;
        files += copied.len();
    }
    for scratch in trees::new_without_old(common)? {
        trees::mirror(&scratch, &keep)?;
    }
    Ok(Copied {
        refs: old.len(),
        trees: trees.len(),
        files,
        command,
    })
}

/// The marker reads `clean`, yet something is under the old names again:
/// an older build wrote it. The stray pass carries or keeps it.
fn older_wrote(repo: &RepoHandle, _alone: &FileLock) -> Settled {
    match settle_strays(repo) {
        Ok(Strays {
            refs: 0, files: 0, ..
        }) => new_names(SettleOutcome::Done),
        Ok(Strays {
            refs,
            files,
            kept_aside,
        }) => new_names(SettleOutcome::OlderWrote {
            refs,
            files,
            kept_aside,
        }),
        Err(why) => new_names(SettleOutcome::LeftOld(why)),
    }
}

/// The marker reads `switched`: the stray pass over what the switched move
/// left, then `clean`.
fn clean_up(repo: &RepoHandle, _alone: &FileLock) -> Settled {
    match clean_up_steps(repo) {
        Ok(Strays {
            refs,
            files,
            kept_aside,
        }) => new_names(SettleOutcome::CleanedUp {
            refs,
            files,
            kept_aside,
        }),
        Err(why) => new_names(SettleOutcome::LeftOld(why)),
    }
}

/// [`clean_up`]'s steps.
fn clean_up_steps(repo: &RepoHandle) -> Result<Strays, MigrateError> {
    let strays = settle_strays(repo)?;
    marker::write(repo.common_dir(), Marker::Clean)?;
    Ok(strays)
}

#[cfg(test)]
mod tests;
