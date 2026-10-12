//! After the switch: a failed earlier move's scratch removed unread, then
//! the stray pass over every other earlier generation (ADR-0057, Amendment
//! 2026-10-11 §5).
//!
//! **Scratch** is a generation newer than the one that was in force, with
//! no marker: `refs/tributary/**` or `.git/tributary/` in a repository
//! still on Graft. By review m8-26 M1 no run ever read it as his, and its
//! source was in force and is copied, so comparing it would only produce a
//! false "an older version wrote" warning.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use graft_core::on_disk::{Generation, MIGRATED_MARKER};
use graft_git::repo::RepoHandle;

use super::strays::{Strays, settle_strays};
use super::{LOCK_FILE, MigrateError, refs, trees};

/// Steps 7–8 after a switch from `was`, the generation in force before it:
/// every generation newer than `was` but older than `CURRENT` is scratch,
/// removed unread; every other earlier one goes through one stray pass.
pub(super) fn after_the_switch(repo: &RepoHandle, was: Generation) -> Result<Strays, MigrateError> {
    let (scratch, strays): (Vec<Generation>, Vec<Generation>) = Generation::CURRENT
        .older()
        .partition(|generation| *generation > was);
    remove(repo, &scratch)?;
    settle_strays(repo, &strays)
}

/// The generation a `switched` move came from: the newest earlier one whose
/// marker is still there (the stray pass removes a generation's marker
/// last of all it holds), else Graft. One `stat` per marked generation.
pub(super) fn was_in_force(common: &Path) -> Result<Generation, MigrateError> {
    for generation in Generation::CURRENT
        .older()
        .filter(|generation| generation.has_marker())
    {
        let marker = common.join(generation.dir()).join(MIGRATED_MARKER);
        if marker
            .try_exists()
            .map_err(|error| MigrateError::files(&marker, &error))?
        {
            return Ok(generation);
        }
    }
    Ok(Generation::Graft)
}

/// A generation's own bookkeeping in `<common>/<its dir>/`: its lock, its
/// marker and the marker's temporary. Never his data, so never copied,
/// compared or kept: the move takes each generation's lock itself, and the
/// marker says which names are in force, which is the move's to decide.
pub(super) fn machinery(common: &Path, generation: Generation) -> [PathBuf; 3] {
    let dir = common.join(generation.dir());
    [
        dir.join(LOCK_FILE),
        dir.join(MIGRATED_MARKER),
        dir.join(MIGRATED_MARKER).with_extension("tmp"),
    ]
}

/// After the stray pass over `folder`'s generation: its `machinery` (lock,
/// marker temporary, marker — in that order, the marker last) removed
/// unread, then `folder` itself if nothing else is left in it.
pub(super) fn remove_machinery(folder: &Path, machinery: &[PathBuf]) -> Result<(), MigrateError> {
    let mut last_first: Vec<&PathBuf> = machinery.iter().collect();
    last_first.sort_by_key(|path| path.file_name() == Some(MIGRATED_MARKER.as_ref()));
    for file in last_first {
        match fs::remove_file(file) {
            Ok(()) => {}
            // Safe to pass over: absent is the state this step makes.
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(MigrateError::files(file, &error)),
        }
    }
    trees::remove_empty(folder)
}

/// Every ref under `scratch`'s prefixes deleted at the value read, in one
/// transaction read back after, then every folder of theirs, the common
/// dir's and each worktree's, removed whole.
fn remove(repo: &RepoHandle, scratch: &[Generation]) -> Result<(), MigrateError> {
    if scratch.is_empty() {
        return Ok(());
    }
    refs::delete_all(repo, scratch)?;
    let common = repo.common_dir();
    for generation in scratch {
        // What git leaves of `refs/<dir>/`: empty folders only.
        trees::remove_empty(&common.join("refs").join(generation.dir()))?;
    }
    for parent in trees::parents(common)? {
        for generation in scratch {
            let folder = parent.join(generation.dir());
            match fs::remove_dir_all(&folder) {
                Ok(()) => {}
                // Safe to pass over: a scratch folder never made is gone.
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(MigrateError::files(&folder, &error)),
            }
        }
    }
    Ok(())
}
