//! A private, writable copy of a fixture.
//!
//! The corpus is shared and must stay read-only: one test that commits into
//! it would change what every other test sees, and the damage would show up
//! somewhere else entirely. A test that needs to write borrows a copy
//! instead, in a temporary directory that is deleted when the test ends
//! (PROMPT.md §7.1).

use std::fs;
use std::io;
use std::path::Path;

use tempfile::TempDir;

/// A writable copy of one fixture, deleted when this value is dropped.
#[derive(Debug)]
pub struct Scratch {
    dir: TempDir,
}

impl Scratch {
    /// The copy's working-tree root.
    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// An empty scratch directory, for a fixture that builds its own
    /// contents (the remotes).
    pub(crate) fn empty(prefix: &str) -> Result<Self, io::Error> {
        let dir = TempDir::with_prefix(format!("graft-{prefix}-"))?;
        Ok(Scratch { dir })
    }

    pub(crate) fn from(source: &Path, prefix: &str) -> Result<Self, io::Error> {
        let dir = TempDir::with_prefix(format!("graft-{prefix}-"))?;
        copy_tree(source, dir.path())?;
        // The shared corpus is read-only and `fs::copy` copies permissions
        // with the bytes, so the copy arrives read-only too.
        crate::permissions::unseal(dir.path())?;
        Ok(Scratch { dir })
    }
}

/// Copies a directory tree. Symlinks are copied as symlinks: a fixture may
/// contain one, and following it would change what the fixture means.
fn copy_tree(from: &Path, to: &Path) -> Result<(), io::Error> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let target = to.join(entry.file_name());
        if kind.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else if kind.is_symlink() {
            copy_symlink(&fs::read_link(entry.path())?, &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn copy_symlink(link_target: &Path, at: &Path) -> Result<(), io::Error> {
    std::os::unix::fs::symlink(link_target, at)
}

#[cfg(not(unix))]
fn copy_symlink(link_target: &Path, at: &Path) -> Result<(), io::Error> {
    // Good enough for a fixture copy: the only symlinks git itself writes are
    // to files, and a directory symlink in a fixture would be deliberate.
    std::os::windows::fs::symlink_file(link_target, at)
}
