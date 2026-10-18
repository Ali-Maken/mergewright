//! After the switch: a failed earlier move's scratch removed unread, then
//! the stray pass over every other earlier generation (ADR-0057, Amendment
//! 2026-10-11 §5).
//!
//! **Scratch** is a generation newer than the one the move came from, with
//! no marker: `refs/tributary/**` or `.git/tributary/` in a repository
//! still on Graft. By review m8-26 M1 no run ever read it as his, and its
//! source was in force and is copied, so comparing it would only produce a
//! false "an older version wrote" warning.
//!
//! Which generation the move came from is what the `switched` marker
//! records, never what is inferred from the markers left (review m8-48
//! Major 1). Each generation's marker is looked at again under the locks,
//! just before its scratch would go: a build of it that has moved since put
//! it in force for that build, so what it holds is his, and goes through
//! the stray pass instead.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use mergewright_core::on_disk::{Generation, MIGRATED_MARKER};
use mergewright_git::repo::RepoHandle;

use super::strays::{Strays, settle_strays};
use super::{LOCK_FILE, MigrateError, refs, trees};

/// Steps 7–8 after a switch from `was`, the generation in force before it
/// (`None`: not recorded, so nothing is taken for scratch): every
/// generation newer than `was` but older than `CURRENT`, with no marker, is
/// scratch, removed unread; every other earlier one goes through one stray
/// pass.
pub(super) fn after_the_switch(
    repo: &RepoHandle,
    was: Option<Generation>,
) -> Result<Strays, MigrateError> {
    let common = repo.common_dir();
    let mut scratch = Vec::new();
    let mut strays = Vec::new();
    for generation in Generation::CURRENT.older() {
        let newer = was.is_some_and(|was| generation > was);
        if newer && !marked(common, generation)? {
            scratch.push(generation);
        } else {
            strays.push(generation);
        }
    }
    remove(repo, &scratch)?;
    settle_strays(repo, &strays)
}

/// Whether `generation`'s marker is there. One `stat`.
fn marked(common: &Path, generation: Generation) -> Result<bool, MigrateError> {
    if !generation.has_marker() {
        return Ok(false);
    }
    let marker = common.join(generation.dir()).join(MIGRATED_MARKER);
    marker
        .try_exists()
        .map_err(|error| MigrateError::files(&marker, &error))
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

/// After the stray pass over a generation: its `machinery` but the lock
/// (marker temporary, then marker) removed unread. The lock is held by this
/// move; it goes once released (`locks::Alone::release`), never while held
/// (review m8-48 Minor 1).
pub(super) fn remove_marker(machinery: &[PathBuf]) -> Result<(), MigrateError> {
    let mut last_first: Vec<&PathBuf> = machinery
        .iter()
        .filter(|path| path.file_name() != Some(LOCK_FILE.as_ref()))
        .collect();
    last_first.sort_by_key(|path| path.file_name() == Some(MIGRATED_MARKER.as_ref()));
    for file in last_first {
        match fs::remove_file(file) {
            Ok(()) => {}
            // Safe to pass over: absent is the state this step makes.
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(MigrateError::files(file, &error)),
        }
    }
    Ok(())
}

/// Every ref under `scratch`'s prefixes deleted at the value read, in one
/// transaction read back after, then every folder of theirs, the common
/// dir's and each worktree's, removed with all it holds but the lock this
/// move holds.
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
            let lock = common.join(generation.dir()).join(LOCK_FILE);
            remove_all_but(&parent.join(generation.dir()), &lock)?;
        }
    }
    Ok(())
}

/// `folder` and everything in it, except `kept` (and so `folder` too, when
/// `kept` is in it).
fn remove_all_but(folder: &Path, kept: &Path) -> Result<(), MigrateError> {
    let entries = match fs::read_dir(folder) {
        Ok(entries) => entries,
        // Safe to pass over: a scratch folder never made is gone.
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(MigrateError::files(folder, &error)),
    };
    for entry in entries {
        let path = entry
            .map_err(|error| MigrateError::files(folder, &error))?
            .path();
        if path == kept {
            continue;
        }
        let meta =
            fs::symlink_metadata(&path).map_err(|error| MigrateError::files(&path, &error))?;
        let removed = if meta.is_dir() {
            fs::remove_dir_all(&path)
        } else {
            fs::remove_file(&path)
        };
        removed.map_err(|error| MigrateError::files(&path, &error))?;
    }
    trees::remove_empty(folder)
}
