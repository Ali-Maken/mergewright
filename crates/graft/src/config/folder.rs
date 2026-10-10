//! The config folder's one-time move to the new name (ticket 26 Part B):
//! `<config home>/graft/` — `config.toml`, `recent`, `launches` — becomes
//! `<config home>/tributary/`, before the first `config::load` reads it.
//!
//! Copied into a staging folder beside it, every file read back, the
//! staging folder renamed into place in one step, then the old folder
//! removed. A crash at any point leaves either the old folder alone or the
//! new folder whole. Only the config home in force is touched: another root
//! the environment could have named is never looked at.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use graft_core::on_disk::OnDiskNames;

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
    /// Both folders are there: the new one is used, the old one left as it
    /// is for him to look at — said once, at startup.
    BothPresent {
        /// The old folder.
        old: PathBuf,
    },
}

/// The names the config folder uses this run, decided once by `load`; the
/// new names when nothing decided them (tests, the gallery).
static NAMES: OnceLock<OnDiskNames> = OnceLock::new();

/// The config folder's names in force: `tributary`, or `graft` when the
/// move failed this run.
pub(crate) fn config_names() -> OnDiskNames {
    NAMES.get().copied().unwrap_or(OnDiskNames::New)
}

/// Decide the names for the rest of the run. The first decision stands: a
/// second `load` in one process finds the move already done.
pub(super) fn decide(names: OnDiskNames) {
    NAMES.get_or_init(|| names);
}

/// Move `<config_root>/graft` to `<config_root>/tributary`, as the module
/// says.
///
/// # Errors
/// Anything the copy, the read-back or the rename hit; the old folder is
/// then untouched and still the one in use.
pub(crate) fn migrate_config_dir(config_root: &Path) -> Result<ConfigDirMoved, io::Error> {
    let old = config_root.join(OnDiskNames::Old.dir());
    let new = config_root.join(OnDiskNames::New.dir());
    if !exists(&old)? {
        return Ok(ConfigDirMoved::Nothing);
    }
    if exists(&new)? {
        return Ok(ConfigDirMoved::BothPresent { old });
    }
    let staging = config_root.join(format!("{}.moving", OnDiskNames::New.dir()));
    if exists(&staging)? {
        // Left by a move that died before its rename: never in use.
        fs::remove_dir_all(&staging)?;
    }
    let files = copy_tree(&old, &staging)?;
    fs::rename(&staging, &new)?;
    match fs::remove_dir_all(&old) {
        Ok(()) => Ok(ConfigDirMoved::Moved { files }),
        // The new folder is whole and in use; the old one is now the
        // "both present" case, said as such.
        Err(_) => Ok(ConfigDirMoved::BothPresent { old }),
    }
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
            let bytes = fs::read(&source)?;
            fs::write(&target, &bytes)?;
            fs::File::open(&target)?.sync_all()?;
            if fs::read(&target)? != bytes {
                return Err(io::Error::other(format!(
                    "{} read back different",
                    target.display()
                )));
            }
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
