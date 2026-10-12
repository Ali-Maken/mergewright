//! Every generation's lock, taken alone for a move or a stray pass
//! (ADR-0057, Amendment 2026-10-11 §6).

use std::path::Path;

use graft_core::on_disk::Generation;
use graft_git::file_lock::FileLock;

use super::{LOCK_FILE, MigrateError};

/// Every lock held alone; dropping it releases them all.
pub(super) struct Alone {
    _locks: Vec<FileLock>,
}

/// `<common_dir>/<CURRENT dir>/`, made if missing, the lock in it, and every
/// earlier generation's lock that exists, each taken alone if nobody else
/// holds it: a build of any generation staying on earlier names holds its
/// own lock shared. `None` when any is held. An earlier generation's lock
/// that is not there is never made: no build of it is open on its names.
pub(super) fn lock_alone(common: &Path) -> Result<Option<Alone>, MigrateError> {
    let dir = common.join(Generation::CURRENT.dir());
    std::fs::create_dir_all(&dir).map_err(|error| MigrateError::files(&dir, &error))?;
    let mut locks = Vec::new();
    for generation in Generation::GENERATIONS {
        let path = common.join(generation.dir()).join(LOCK_FILE);
        let there = path
            .try_exists()
            .map_err(|error| MigrateError::files(&path, &error))?;
        if generation != Generation::CURRENT && !there {
            continue;
        }
        match FileLock::try_exclusive(&path) {
            Ok(Some(lock)) => locks.push(lock),
            Ok(None) => return Ok(None),
            Err(error) => return Err(MigrateError::files(&path, &error)),
        }
    }
    Ok(Some(Alone { _locks: locks }))
}
