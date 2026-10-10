//! The app's folders under the old name — the common dir's and every
//! worktree's — copied whole to the new name, every file read back, and
//! removed once the switch is made.
//!
//! Whole folders, not a list of known files: the old folder is deleted
//! afterwards, so a file this code did not know about would otherwise be
//! deleted unread.

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use graft_core::on_disk::OnDiskNames;

use super::MigrateError;
use crate::journal;

/// The journal's file name; a copy of one is also compared entry by entry.
const JOURNAL_FILE: &str = "undo-journal.jsonl";

/// One old folder and where it moves.
#[derive(Debug)]
pub(super) struct Tree {
    old: PathBuf,
    new: PathBuf,
}

/// Every old folder of the repository whose common dir is `common`:
/// `<common>/graft` (the main worktree's `git-path graft` too) and
/// `<common>/worktrees/*/graft` (each linked worktree's `git_dir`'s and
/// `git-path`'s), each once. Reads the folders, spawns nothing.
pub(super) fn old_trees(common: &Path) -> Result<Vec<Tree>, MigrateError> {
    let old = OnDiskNames::Old.dir();
    let mut parents = vec![common.to_path_buf()];
    let worktrees = common.join("worktrees");
    match fs::read_dir(&worktrees) {
        Ok(entries) => {
            for entry in entries {
                let entry = entry.map_err(|error| MigrateError::files(&worktrees, &error))?;
                parents.push(entry.path());
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(MigrateError::files(&worktrees, &error)),
    }
    let mut seen = BTreeSet::new();
    let mut trees = Vec::new();
    for parent in parents {
        let folder = parent.join(old);
        match fs::symlink_metadata(&folder) {
            Ok(meta) if meta.is_dir() => {}
            Ok(_) => {
                return Err(MigrateError::Files {
                    path: folder,
                    words: "not a folder; left for you to move".to_owned(),
                });
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(MigrateError::files(&folder, &error)),
        }
        let canonical = folder
            .canonicalize()
            .map_err(|error| MigrateError::files(&folder, &error))?;
        if seen.insert(canonical) {
            trees.push(Tree {
                new: parent.join(OnDiskNames::New.dir()),
                old: folder,
            });
        }
    }
    Ok(trees)
}

/// Copy every file of `tree.old` to the same place under `tree.new`,
/// synced, and read each back. A file already there with the same bytes
/// (an earlier move stopped part-way) counts as copied; with other bytes it
/// is refused, never overwritten. The number of files.
pub(super) fn copy(tree: &Tree) -> Result<usize, MigrateError> {
    let mut files = 0;
    walk(&tree.old, &mut |old| {
        let relative = old
            .strip_prefix(&tree.old)
            .map_err(|_| MigrateError::Files {
                path: old.to_path_buf(),
                words: "found outside the folder being copied".to_owned(),
            })?;
        let new = tree.new.join(relative);
        copy_one(old, &new)?;
        files += 1;
        Ok(())
    })?;
    Ok(files)
}

/// The number of files under `tree.old`.
pub(super) fn count(tree: &Tree) -> Result<usize, MigrateError> {
    let mut files = 0;
    walk(&tree.old, &mut |_| {
        files += 1;
        Ok(())
    })?;
    Ok(files)
}

/// Remove `tree.old`, whole.
pub(super) fn remove(tree: &Tree) -> Result<(), MigrateError> {
    fs::remove_dir_all(&tree.old).map_err(|error| MigrateError::files(&tree.old, &error))
}

/// Call `each` on every file under `folder`, depth first. Anything that is
/// neither a file nor a folder (a symlink) stops the walk: it would not
/// survive a byte copy as what it is.
fn walk(
    folder: &Path,
    each: &mut dyn FnMut(&Path) -> Result<(), MigrateError>,
) -> Result<(), MigrateError> {
    let entries = fs::read_dir(folder).map_err(|error| MigrateError::files(folder, &error))?;
    for entry in entries {
        let path = entry
            .map_err(|error| MigrateError::files(folder, &error))?
            .path();
        let meta =
            fs::symlink_metadata(&path).map_err(|error| MigrateError::files(&path, &error))?;
        if meta.is_dir() {
            walk(&path, each)?;
        } else if meta.is_file() {
            each(&path)?;
        } else {
            return Err(MigrateError::Files {
                path,
                words: "not a file or a folder; left for you to move".to_owned(),
            });
        }
    }
    Ok(())
}

fn copy_one(old: &Path, new: &Path) -> Result<(), MigrateError> {
    let original = fs::read(old).map_err(|error| MigrateError::files(old, &error))?;
    match fs::read(new) {
        Ok(there) if there == original => return Ok(()),
        Ok(_) => {
            return Err(MigrateError::Files {
                path: new.to_path_buf(),
                words: "already there with other bytes; left for you to compare".to_owned(),
            });
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(MigrateError::files(new, &error)),
    }
    if let Some(folder) = new.parent() {
        fs::create_dir_all(folder).map_err(|error| MigrateError::files(folder, &error))?;
    }
    fs::write(new, &original)
        .and_then(|()| File::open(new)?.sync_all())
        .map_err(|error| MigrateError::files(new, &error))?;
    let back = fs::read(new).map_err(|error| MigrateError::files(new, &error))?;
    if back != original {
        return Err(MigrateError::ReadBack {
            what: new.display().to_string(),
        });
    }
    if new.file_name().is_some_and(|name| name == JOURNAL_FILE) {
        let entries = |bytes: &[u8]| journal::parse(bytes).ok().map(|parsed| parsed.entries);
        if entries(&back) != entries(&original) {
            return Err(MigrateError::ReadBack {
                what: format!("{} (entries)", new.display()),
            });
        }
    }
    Ok(())
}
