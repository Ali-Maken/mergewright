//! The app's folders under the old name — the common dir's and every
//! worktree's — copied whole to the new name, every file read back; after
//! the switch, set aside by the stray pass.
//!
//! Whole folders, not a list of known files: the old folder is emptied
//! afterwards, so a file this code did not know about would otherwise be
//! deleted unread.
//!
//! **Before the switch the new folders are the move's own scratch**: the
//! marker is absent, so no run has ever read them as the user's. A copy
//! therefore makes each new folder an exact copy of its old one — a file
//! that differs is overwritten, a file with no old counterpart is deleted —
//! so a move that failed part-way leaves nothing a retry trips on (review
//! m8-26 M1).

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use graft_core::on_disk::Generation;

use super::MigrateError;

/// The journal's file name: a copy of one has its backups checked, and a
/// journal is compared as an append-only file.
pub(super) const JOURNAL_FILE: &str = "undo-journal.jsonl";

/// One old folder and where it moves.
#[derive(Debug)]
pub(super) struct Tree {
    old: PathBuf,
    new: PathBuf,
}

/// Every folder that may hold the app's files: the common dir and each
/// `<common>/worktrees/*`.
fn parents(common: &Path) -> Result<Vec<PathBuf>, MigrateError> {
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
    Ok(parents)
}

/// Every folder of `from`'s in the repository whose common dir is
/// `common`, each paired with `CURRENT`'s beside it: `<common>/<from>`
/// (the main worktree's `git-path <from>` too) and
/// `<common>/worktrees/*/<from>` (each linked worktree's `git_dir`'s and
/// `git-path`'s), each once. Reads the folders, spawns nothing.
pub(super) fn old_trees(common: &Path, from: Generation) -> Result<Vec<Tree>, MigrateError> {
    let mut seen = BTreeSet::new();
    let mut trees = Vec::new();
    for parent in parents(common)? {
        let folder = parent.join(from.dir());
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
                new: parent.join(Generation::CURRENT.dir()),
                old: folder,
            });
        }
    }
    Ok(trees)
}

/// Every `CURRENT` folder with no `from` one beside it — scratch a failed
/// earlier move left, emptied by [`mirror`] like any other extra.
pub(super) fn new_without_old(common: &Path, from: Generation) -> Result<Vec<Tree>, MigrateError> {
    let mut trees = Vec::new();
    for parent in parents(common)? {
        let (old, new) = (
            parent.join(from.dir()),
            parent.join(Generation::CURRENT.dir()),
        );
        if exists(&new)? && !exists(&old)? {
            trees.push(Tree { old, new });
        }
    }
    Ok(trees)
}

fn exists(path: &Path) -> Result<bool, MigrateError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(MigrateError::files(path, &error)),
    }
}

/// Make `tree.new` an exact copy of `tree.old` (absent counts as empty):
/// every old file written to the same place under `tree.new` and read
/// back, every new file with no old counterpart deleted except `keep`, then
/// every folder that gained or lost an entry synced. The new paths of the
/// files copied.
pub(super) fn mirror(tree: &Tree, keep: &[PathBuf]) -> Result<Vec<PathBuf>, MigrateError> {
    let mut copied = Vec::new();
    if exists(&tree.old)? {
        walk(&tree.old, &mut |old| {
            let new = tree.new.join(relative(old, &tree.old)?);
            let bytes = fs::read(old).map_err(|error| MigrateError::files(old, &error))?;
            write_read_back(&new, &bytes)?;
            copied.push(new);
            Ok(())
        })?;
    }
    let mut extras = Vec::new();
    if exists(&tree.new)? {
        walk(&tree.new, &mut |new| {
            let old = tree.old.join(relative(new, &tree.new)?);
            if !keep.iter().any(|kept| kept == new) && !exists(&old)? {
                extras.push(new.to_path_buf());
            }
            Ok(())
        })?;
    }
    for extra in &extras {
        fs::remove_file(extra).map_err(|error| MigrateError::files(extra, &error))?;
    }
    remove_empty_below(&tree.new)?;
    let stop = tree.new.parent().unwrap_or(&tree.new);
    let touched: Vec<PathBuf> = copied.iter().chain(&extras).cloned().collect();
    sync_dirs(&dirs_to_sync(stop, &touched))?;
    Ok(copied)
}

/// Every folder whose entries a write or delete of `files` may have
/// changed, from each file's own folder up to and including `stop`: a
/// created folder and its parent both, so a power loss after the switch
/// cannot lose a copy's directory entry (review m8-26 Minor 2). Pure.
pub(super) fn dirs_to_sync(stop: &Path, files: &[PathBuf]) -> BTreeSet<PathBuf> {
    let mut dirs = BTreeSet::new();
    for file in files {
        for dir in file.ancestors().skip(1) {
            if !dir.starts_with(stop) {
                break;
            }
            dirs.insert(dir.to_path_buf());
        }
    }
    dirs
}

/// `fsync` each folder.
fn sync_dirs(dirs: &BTreeSet<PathBuf>) -> Result<(), MigrateError> {
    for dir in dirs {
        match File::open(dir).and_then(|folder| folder.sync_all()) {
            Ok(()) => {}
            // Removed because it was left empty: there is no entry in it
            // to make durable, and its parent is in the set too.
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(MigrateError::files(dir, &error)),
        }
    }
    Ok(())
}

/// The stray pass over one old folder (after the switch): each old file is
/// first renamed into `pass` at its path relative to `common` — atomic, so
/// an older build that opens by path afterwards makes a new old file for
/// the next pass to find — then deleted when equal to its new copy, copied
/// forward when it has none, and left in `pass` when it differs. Then the
/// old folder's empty folders go. Files handled, and how many stayed.
pub(super) fn set_aside(
    common: &Path,
    tree: &Tree,
    pass: &Path,
) -> Result<(usize, usize), MigrateError> {
    let mut olds = Vec::new();
    walk(&tree.old, &mut |old| {
        olds.push(old.to_path_buf());
        Ok(())
    })?;
    let mut kept = 0;
    for old in &olds {
        let aside = pass.join(relative(old, common)?);
        if let Some(folder) = aside.parent() {
            fs::create_dir_all(folder).map_err(|error| MigrateError::files(folder, &error))?;
        }
        fs::rename(old, &aside).map_err(|error| MigrateError::files(old, &error))?;
        let new = tree.new.join(relative(old, &tree.old)?);
        let bytes = fs::read(&aside).map_err(|error| MigrateError::files(&aside, &error))?;
        match fs::read(&new) {
            Ok(there) if holds_the_same(&new, &there, &bytes) => {}
            Ok(_) => {
                kept += 1;
                continue;
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                write_read_back(&new, &bytes)?;
                let stop = tree.new.parent().unwrap_or(&tree.new);
                sync_dirs(&dirs_to_sync(stop, std::slice::from_ref(&new)))?;
            }
            Err(error) => return Err(MigrateError::files(&new, &error)),
        }
        fs::remove_file(&aside).map_err(|error| MigrateError::files(&aside, &error))?;
    }
    remove_empty(&tree.old)?;
    Ok((olds.len(), kept))
}

/// Whether the new file at `new`, holding `there`, already holds the old
/// file's `old` bytes. Byte-equal, or — for a journal, which is only ever
/// appended to — `old` is where `there` begins: this build appended after
/// the copy (the `Migrate` entry, at least), and the old file holds nothing
/// the new one lacks.
fn holds_the_same(new: &Path, there: &[u8], old: &[u8]) -> bool {
    there == old
        || (new.file_name().is_some_and(|name| name == JOURNAL_FILE) && there.starts_with(old))
}

/// After every folder's pass: what stayed in `pass` made durable, then
/// `pass` and its parent removed if they hold nothing. `true` when `pass`
/// holds something.
pub(super) fn finish_pass(pass: &Path) -> Result<bool, MigrateError> {
    if !exists(pass)? {
        return Ok(false);
    }
    let mut kept = Vec::new();
    walk(pass, &mut |file| {
        kept.push(file.to_path_buf());
        Ok(())
    })?;
    let parent = pass.parent().unwrap_or(pass);
    let stop = parent.parent().unwrap_or(parent);
    sync_dirs(&dirs_to_sync(stop, &kept))?;
    remove_empty(pass)?;
    remove_empty(parent)?;
    Ok(!kept.is_empty())
}

/// `path` below `root`.
fn relative<'a>(path: &'a Path, root: &Path) -> Result<&'a Path, MigrateError> {
    path.strip_prefix(root).map_err(|_| MigrateError::Files {
        path: path.to_path_buf(),
        words: "found outside the folder being copied".to_owned(),
    })
}

/// Remove every empty folder below `folder`, deepest first, keeping
/// `folder` itself.
fn remove_empty_below(folder: &Path) -> Result<(), MigrateError> {
    if !exists(folder)? {
        return Ok(());
    }
    let entries = fs::read_dir(folder).map_err(|error| MigrateError::files(folder, &error))?;
    for entry in entries {
        let path = entry
            .map_err(|error| MigrateError::files(folder, &error))?
            .path();
        let meta =
            fs::symlink_metadata(&path).map_err(|error| MigrateError::files(&path, &error))?;
        if meta.is_dir() {
            remove_empty(&path)?;
        }
    }
    Ok(())
}

/// [`remove_empty_below`], then `folder` itself if it is empty now.
fn remove_empty(folder: &Path) -> Result<(), MigrateError> {
    if !exists(folder)? {
        return Ok(());
    }
    remove_empty_below(folder)?;
    match fs::remove_dir(folder) {
        Ok(()) => Ok(()),
        // Something is still in it — a file kept aside, or one an older
        // build wrote since the walk: it stays for the next pass, which
        // finds it. Nothing is lost by leaving a folder.
        Err(error) if error.kind() == io::ErrorKind::DirectoryNotEmpty => Ok(()),
        Err(error) => Err(MigrateError::files(folder, &error)),
    }
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

/// `new` holds `bytes`, read back: left alone when it already does,
/// otherwise written whole — a temporary beside it, synced, renamed over it.
fn write_read_back(new: &Path, bytes: &[u8]) -> Result<(), MigrateError> {
    match fs::read(new) {
        Ok(there) if there == bytes => return Ok(()),
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(MigrateError::files(new, &error)),
    }
    if let Some(folder) = new.parent() {
        fs::create_dir_all(folder).map_err(|error| MigrateError::files(folder, &error))?;
    }
    let mut name = new.file_name().unwrap_or_default().to_os_string();
    name.push(".tributary-tmp");
    let temporary = new.with_file_name(name);
    fs::write(&temporary, bytes)
        .and_then(|()| File::open(&temporary)?.sync_all())
        .and_then(|()| fs::rename(&temporary, new))
        .map_err(|error| MigrateError::files(new, &error))?;
    let back = fs::read(new).map_err(|error| MigrateError::files(new, &error))?;
    if back != bytes {
        return Err(MigrateError::ReadBack {
            what: new.display().to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::dirs_to_sync;

    /// A copy into a folder made for it syncs that folder, every folder
    /// above it up to the worktree's (or common) dir, and that dir — whose
    /// entry for the new top folder is what a power loss would drop.
    #[test]
    fn every_created_dir_is_synced() {
        let stop = Path::new("/r/.git");
        let files = [
            PathBuf::from("/r/.git/tributary/rewrite/id/plan.json"),
            PathBuf::from("/r/.git/tributary/layout"),
        ];
        let dirs: Vec<PathBuf> = dirs_to_sync(stop, &files).into_iter().collect();
        assert_eq!(
            dirs,
            [
                "/r/.git",
                "/r/.git/tributary",
                "/r/.git/tributary/rewrite",
                "/r/.git/tributary/rewrite/id",
            ]
            .map(PathBuf::from)
        );
        assert!(dirs_to_sync(stop, &[]).is_empty());
    }
}
