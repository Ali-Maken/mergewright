//! The config folder's one-time move to the new name (ticket 26 Part B):
//! `<config home>/graft/` — `config.toml`, `recent`, `launches` — becomes
//! `<config home>/tributary/`, before the first `config::load` reads it.
//!
//! Copied into a staging folder beside it, every file read back, the
//! staging folder renamed into place in one step, then the old folder
//! removed. A crash at any point leaves either the old folder alone or the
//! new folder whole. When both are there (an older version wrote the old
//! one after the move), each old file is settled by its bytes: equal →
//! deleted, no new copy → carried forward, differs → kept aside under
//! `tributary/older-version/<n>/` (review m8-26 Q2). Only the config home in force is touched: another root
//! the environment could have named is never looked at.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use graft_core::on_disk::Generation;
use graft_git::file_lock::FileLock;
use graft_ops::rename::KEPT_ASIDE_DIR;

/// What [`migrate_config_dir`] did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ConfigDirMoved {
    /// No old folder: nothing to move.
    Nothing,
    /// Moved, this many files.
    Moved {
        /// Files moved.
        files: usize,
    },
    /// Both folders were there: an older version wrote the old one after
    /// the move (review m8-26 Q2). Each old file equal to its new copy was
    /// deleted, one with no new copy carried forward, and one that differs
    /// moved to `kept_aside`. Said on this launch only: the old folder is
    /// gone after it.
    OlderWrote {
        /// The old folder.
        old: PathBuf,
        /// `<config_root>/tributary/older-version/<n>/`, when a file
        /// differed.
        kept_aside: Option<PathBuf>,
    },
    /// The new folder is whole and in use, but the old one could not be
    /// removed (review m8-26 Minor 3).
    OldLeft {
        /// The old folder.
        old: PathBuf,
        /// The removal's `io::Error`, in words.
        why: String,
    },
}

/// The names the config folder uses this run, decided once by `load`; the
/// new names when nothing decided them (tests, the gallery).
static NAMES: OnceLock<Generation> = OnceLock::new();

/// The config folder's names in force: `CURRENT`'s, or the earlier
/// generation's when the move failed this run.
pub(crate) fn config_names() -> Generation {
    NAMES.get().copied().unwrap_or(Generation::CURRENT)
}

/// Decide the names for the rest of the run. The first decision stands: a
/// second `load` in one process finds the move already done.
pub(super) fn decide(names: Generation) {
    NAMES.get_or_init(|| names);
}

/// Move `<config_root>/<from>` to `<config_root>/<CURRENT>`, as the module
/// says, under an flock on `<config_root>/<CURRENT>.lock` so two launches
/// never move it under each other (review m8-26 Minor 4).
///
/// # Errors
/// Anything the lock, the copy, the read-back or the rename hit; when the
/// new folder did not exist yet, the old one is then untouched and still
/// the one in use.
pub(crate) fn migrate_config_dir(
    config_root: &Path,
    from: Generation,
) -> Result<ConfigDirMoved, io::Error> {
    let old = config_root.join(from.dir());
    let new = config_root.join(Generation::CURRENT.dir());
    // The common case — moved long ago — takes no lock and writes nothing.
    if !exists(&old)? {
        return Ok(ConfigDirMoved::Nothing);
    }
    let _lock = lock_alone(&config_root.join(format!("{}.lock", Generation::CURRENT.dir())))?;
    // Another launch may have moved it while this one waited.
    if !exists(&old)? {
        return Ok(ConfigDirMoved::Nothing);
    }
    if exists(&new)? {
        return settle_both(&old, &new);
    }
    let staging = config_root.join(format!("{}.moving", Generation::CURRENT.dir()));
    if exists(&staging)? {
        // Left by a move that died before its rename: never in use.
        fs::remove_dir_all(&staging)?;
    }
    let files = copy_tree(&old, &staging)?;
    fs::rename(&staging, &new)?;
    match fs::remove_dir_all(&old) {
        Ok(()) => Ok(ConfigDirMoved::Moved { files }),
        Err(error) => Ok(ConfigDirMoved::OldLeft {
            old,
            why: error.to_string(),
        }),
    }
}

/// The flock alone. Waiting is a kernel wait on the shared lock — which
/// blocks while anyone holds it alone — then another try: never a poll.
fn lock_alone(path: &Path) -> io::Result<FileLock> {
    loop {
        if let Some(held) = FileLock::try_exclusive(path)? {
            return Ok(held);
        }
        drop(FileLock::shared(path)?);
    }
}

/// Both folders present: the byte rule, file by file (review m8-26 Q2).
fn settle_both(old: &Path, new: &Path) -> Result<ConfigDirMoved, io::Error> {
    let mut pass = Pass {
        aside_root: new.join(KEPT_ASIDE_DIR),
        aside: None,
        wrote: false,
    };
    pass.settle_dir(old, new, Path::new(""))?;
    let moved = |kept_aside| ConfigDirMoved::OlderWrote {
        old: old.to_path_buf(),
        kept_aside,
    };
    match fs::remove_dir(old) {
        Ok(()) if pass.wrote => Ok(moved(pass.aside)),
        // Only copies equal to the new ones: nothing an older version
        // wrote is lost, so nothing to say.
        Ok(()) => Ok(ConfigDirMoved::Nothing),
        Err(error) => Ok(ConfigDirMoved::OldLeft {
            old: old.to_path_buf(),
            why: error.to_string(),
        }),
    }
}

/// One both-present pass: where differing files go, and whether anything
/// an older version wrote was carried or kept.
struct Pass {
    /// `<new>/older-version`.
    aside_root: PathBuf,
    /// `<new>/older-version/<n>/`, once a file differed.
    aside: Option<PathBuf>,
    /// A file was carried forward or kept aside.
    wrote: bool,
}

impl Pass {
    /// Settle every entry under `old_dir` (`rel` below the old folder),
    /// then remove `old_dir` if it is empty. A symlink or other non-file
    /// is left where it is, so the old folder stays and the caller says so.
    fn settle_dir(&mut self, old_dir: &Path, new: &Path, rel: &Path) -> io::Result<()> {
        for entry in fs::read_dir(old_dir)? {
            let entry = entry?;
            let source = entry.path();
            let rel = rel.join(entry.file_name());
            let kind = fs::symlink_metadata(&source)?.file_type();
            if kind.is_dir() {
                self.settle_dir(&source, new, &rel)?;
                remove_if_empty(&source)?;
            } else if kind.is_file() {
                self.settle_file(&source, &new.join(&rel), &rel)?;
            }
        }
        Ok(())
    }

    fn settle_file(&mut self, source: &Path, target: &Path, rel: &Path) -> io::Result<()> {
        let bytes = fs::read(source)?;
        match fs::read(target) {
            Ok(theirs) if theirs == bytes => {}
            Ok(_) => {
                let aside = self.aside()?.join(rel);
                if let Some(parent) = aside.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::rename(source, &aside)?;
                self.wrote = true;
                return Ok(());
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)?;
                }
                write_read_back(target, &bytes)?;
                self.wrote = true;
            }
            Err(error) => return Err(error),
        }
        fs::remove_file(source)
    }

    /// The pass's folder: the first `<n>` not yet used.
    fn aside(&mut self) -> io::Result<PathBuf> {
        if let Some(aside) = &self.aside {
            return Ok(aside.clone());
        }
        let mut n = 1_u32;
        let aside = loop {
            let candidate = self.aside_root.join(n.to_string());
            if !exists(&candidate)? {
                break candidate;
            }
            n += 1;
        };
        fs::create_dir_all(&aside)?;
        self.aside = Some(aside.clone());
        Ok(aside)
    }
}

fn remove_if_empty(dir: &Path) -> io::Result<()> {
    match fs::remove_dir(dir) {
        Ok(()) => Ok(()),
        // Something stayed (a symlink, say): the old folder's own removal
        // fails on it and says why.
        Err(error) if error.kind() == io::ErrorKind::DirectoryNotEmpty => Ok(()),
        Err(error) => Err(error),
    }
}

/// Write `bytes` to `target`, synced, and read it back.
fn write_read_back(target: &Path, bytes: &[u8]) -> io::Result<()> {
    fs::write(target, bytes)?;
    fs::File::open(target)?.sync_all()?;
    if fs::read(target)? != bytes {
        return Err(io::Error::other(format!(
            "{} read back different",
            target.display()
        )));
    }
    Ok(())
}

fn exists(path: &Path) -> io::Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

/// Copy every file under `from` to the same place under `to`, synced, and
/// read each back. The number of files.
fn copy_tree(from: &Path, to: &Path) -> io::Result<usize> {
    fs::create_dir_all(to)?;
    let mut files = 0;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let source = entry.path();
        let target = to.join(entry.file_name());
        let kind = fs::symlink_metadata(&source)?.file_type();
        if kind.is_dir() {
            files += copy_tree(&source, &target)?;
        } else if kind.is_file() {
            write_read_back(&target, &fs::read(&source)?)?;
            files += 1;
        } else {
            return Err(io::Error::other(format!(
                "{} is not a file or a folder; left for you to move",
                source.display()
            )));
        }
    }
    Ok(files)
}

#[cfg(test)]
mod tests;
