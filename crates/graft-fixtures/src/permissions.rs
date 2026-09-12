//! Making the shared corpus read-only, and a copy of it writable again.
//!
//! The corpus is shared by every test in the workspace and cached between
//! runs, so a stray write into it is not a failure that shows up where it
//! happened: it silently changes what every later run sees. Clearing the
//! write bits turns that mistake into an error at the moment it is made.
//!
//! Files only. The directories stay writable so `cargo clean` can still
//! remove the tree, and so a lost build race can throw its own staging
//! directory away.

use std::fs;
use std::io;
use std::path::Path;

/// Clears every write bit on every file under `dir`.
pub fn seal(dir: &Path) -> Result<(), io::Error> {
    walk(dir, &|path, mut permissions| {
        set_writable(&mut permissions, false);
        fs::set_permissions(path, permissions)
    })
}

/// Gives the owner back write access to every file under `dir` — what a
/// copy of a sealed corpus needs before a test can write in it.
pub fn unseal(dir: &Path) -> Result<(), io::Error> {
    walk(dir, &|path, mut permissions| {
        set_writable(&mut permissions, true);
        fs::set_permissions(path, permissions)
    })
}

type Change = dyn Fn(&Path, fs::Permissions) -> Result<(), io::Error>;

/// Applies `change` to every file in the tree, leaving directories and
/// symlinks alone. A symlink has no permissions of its own worth changing,
/// and following one would reach outside the tree.
fn walk(dir: &Path, change: &Change) -> Result<(), io::Error> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            walk(&entry.path(), change)?;
        } else if kind.is_file() {
            let path = entry.path();
            let permissions = entry.metadata()?.permissions();
            change(&path, permissions)?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn set_writable(permissions: &mut fs::Permissions, writable: bool) {
    use std::os::unix::fs::PermissionsExt;

    // Owner write only. Group and world write are nobody's business here,
    // and sealing must not be a way to widen access.
    const OWNER_WRITE: u32 = 0o200;
    let mode = permissions.mode();
    permissions.set_mode(if writable {
        mode | OWNER_WRITE
    } else {
        mode & !0o222
    });
}

#[cfg(not(unix))]
fn set_writable(permissions: &mut fs::Permissions, writable: bool) {
    permissions.set_readonly(!writable);
}
