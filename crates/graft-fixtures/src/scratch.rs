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
        let dir = new_dir(prefix, fixed_root().as_deref())?;
        Ok(Scratch { dir })
    }

    pub(crate) fn from(source: &Path, prefix: &str) -> Result<Self, io::Error> {
        let dir = new_dir(prefix, fixed_root().as_deref())?;
        copy_tree(source, dir.path())?;
        // The shared corpus is read-only and `fs::copy` copies permissions
        // with the bytes, so the copy arrives read-only too.
        crate::permissions::unseal(dir.path())?;
        Ok(Scratch { dir })
    }
}

/// The variable naming a fixed scratch root (m8-polish-and-ship/22).
///
/// A pty capture shows the directory the app really opened, and a random
/// temp name under a per-user `$TMPDIR` makes that text differ run to run
/// and machine to machine. The pty walk scripts set this to
/// `/tmp/graft-capture/<scenario>`; every scratch is then
/// `<root>/graft-<prefix>-<n>`, `n` counting this process's scratches in
/// order, so the same walk draws the same path every time. Unset — every
/// test — a scratch is a fresh random temp dir as before.
pub const FIXED_ROOT_VAR: &str = "TRIB_SCRATCH_ROOT";

fn fixed_root() -> Option<std::path::PathBuf> {
    std::env::var_os(FIXED_ROOT_VAR)
        .filter(|root| !root.is_empty())
        .map(Into::into)
}

/// A scratch directory: random under the OS temp dir, or, given a fixed
/// root, the next deterministic name under it (still deleted on drop).
fn new_dir(prefix: &str, root: Option<&Path>) -> Result<TempDir, io::Error> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let Some(root) = root else {
        return TempDir::with_prefix(format!("graft-{prefix}-"));
    };
    named_dir(prefix, root, NEXT.fetch_add(1, Ordering::Relaxed))
}

/// `<root>/graft-<prefix>-<n>`, replacing any leftover of that name.
fn named_dir(prefix: &str, root: &Path, n: usize) -> Result<TempDir, io::Error> {
    fs::create_dir_all(root)?;
    let path = root.join(format!("graft-{prefix}-{n}"));
    // A previous run of the same walk left it only if it was killed before
    // its drop ran; the name is ours by construction, so clearing it is safe.
    if path.exists() {
        fs::remove_dir_all(&path)?;
    }
    tempfile::Builder::new()
        .prefix(&format!("graft-{prefix}-{n}"))
        .rand_bytes(0)
        .tempdir_in(root)
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

#[cfg(test)]
mod tests {
    use super::{named_dir, new_dir};

    #[test]
    fn a_fixed_root_gives_the_same_name_every_run_and_deletes_on_drop() {
        let root = tempfile::tempdir().unwrap();
        let a = named_dir("linear", root.path(), 3).unwrap();
        assert_eq!(a.path(), root.path().join("graft-linear-3"));
        let gone = a.path().to_path_buf();
        drop(a);
        assert!(!gone.exists());
        let again = named_dir("linear", root.path(), 3).unwrap();
        assert_eq!(again.path(), gone);
    }

    #[test]
    fn a_leftover_from_a_killed_run_is_replaced() {
        let root = tempfile::tempdir().unwrap();
        let left = root.path().join("graft-left-0");
        std::fs::create_dir_all(&left).unwrap();
        std::fs::write(left.join("stale"), b"x").unwrap();
        let fresh = named_dir("left", root.path(), 0).unwrap();
        assert_eq!(fresh.path(), left);
        assert!(!left.join("stale").exists());
    }

    #[test]
    fn a_fixed_root_counts_up_within_a_process() {
        let root = tempfile::tempdir().unwrap();
        let a = new_dir("count", Some(root.path())).unwrap();
        let b = new_dir("count", Some(root.path())).unwrap();
        assert_ne!(a.path(), b.path());
        assert_eq!(a.path().parent(), Some(root.path()));
    }

    #[test]
    fn no_root_is_a_random_temp_dir() {
        let a = new_dir("linear", None).unwrap();
        let b = new_dir("linear", None).unwrap();
        assert_ne!(a.path(), b.path());
    }
}
