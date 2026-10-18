//! Every generation's lock, taken alone for a move or a stray pass
//! (ADR-0057, Amendment 2026-10-11 §6).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use mergewright_core::on_disk::Generation;
use mergewright_git::file_lock::FileLock;

use super::{LOCK_FILE, MigrateError, trees};

/// Every lock held alone; dropping it releases them all.
pub(super) struct Alone {
    locks: Vec<FileLock>,
    /// Each earlier generation's lock file, to go once released.
    earlier: Vec<PathBuf>,
}

impl Alone {
    /// Release every lock, then remove each earlier generation's lock file
    /// and its folder if that is all it held. Called once the marker reads
    /// `clean`: no earlier generation's names are left to guard.
    ///
    /// Removed only once released, never while held (review m8-48 Minor
    /// 1). A build of an earlier generation that opens the file between
    /// the two holds an unlinked one; nothing is lost by that, as whatever
    /// it then writes under its names is a stray the next open's pass
    /// carries or keeps.
    pub(super) fn release(self) -> Result<(), MigrateError> {
        let Alone { locks, earlier } = self;
        drop(locks);
        for lock in earlier {
            match fs::remove_file(&lock) {
                Ok(()) => {}
                // Safe to pass over: absent is the state this step makes.
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(MigrateError::files(&lock, &error)),
            }
            if let Some(folder) = lock.parent() {
                trees::remove_empty(folder)?;
            }
        }
        Ok(())
    }
}

/// `<common_dir>/<CURRENT dir>/lock`, its folder made if missing, then each
/// earlier generation's that moves names (`has_marker`), each taken alone
/// if nobody else holds it: a build of any generation staying on earlier
/// names holds its own lock shared. `None` when any is held.
///
/// An earlier generation's lock is made when it is not there yet (review
/// m8-48 Major 1, trigger B): a build of it launched during this move would
/// otherwise make it, take it, and move the names alongside this move,
/// which knows nothing of what it writes. Graft's builds never lock: its
/// is taken only if it is there.
pub(super) fn lock_alone(common: &Path) -> Result<Option<Alone>, MigrateError> {
    let dir = common.join(Generation::CURRENT.dir());
    fs::create_dir_all(&dir).map_err(|error| MigrateError::files(&dir, &error))?;
    let mut alone = Alone {
        locks: Vec::new(),
        earlier: Vec::new(),
    };
    let order = std::iter::once(Generation::CURRENT).chain(Generation::CURRENT.older());
    for generation in order {
        let path = common.join(generation.dir()).join(LOCK_FILE);
        if generation != Generation::CURRENT {
            if !generation.has_marker()
                && !path
                    .try_exists()
                    .map_err(|error| MigrateError::files(&path, &error))?
            {
                continue;
            }
            let folder = common.join(generation.dir());
            fs::create_dir_all(&folder).map_err(|error| MigrateError::files(&folder, &error))?;
            alone.earlier.push(path.clone());
        }
        match FileLock::try_exclusive(&path) {
            Ok(Some(lock)) => alone.locks.push(lock),
            Ok(None) => return Ok(None),
            Err(error) => return Err(MigrateError::files(&path, &error)),
        }
    }
    Ok(Some(alone))
}
