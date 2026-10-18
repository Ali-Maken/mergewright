//! Which of the written paths his bytes stand on, and whether git calls
//! each one untracked or ignored. The only worktree `symlink_metadata` in
//! the crate.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use mergewright_git::repo::RepoHandle;

use super::Blocking;
use crate::facts::FactsError;
use crate::operation::RepoFacts;

/// The one `symlink_metadata` loop: which of `paths` his bytes stand in the
/// way of, as the path on disk that holds them.
///
/// - Something at the path — a file, a link, or a directory holding
///   anything the index doesn't track — counts as the path itself. A
///   directory of only tracked files is git's to empty (a tracked `dist/`
///   turning into a file `dist`), and an empty one holds no bytes.
/// - A file where the path needs a directory (`a` for `a/b`, `ENOTDIR`)
///   counts as that file, the first ancestor that isn't a directory —
///   unless the index tracks it: then git removes it as part of the change.
/// - Nothing there (`NotFound`) is nothing in the way.
///
/// # Errors
/// [`FactsError::OnDisk`] for any other failure to look.
pub(super) fn on_disk(
    repo: &RepoHandle,
    paths: &BTreeSet<PathBuf>,
    tracked: &BTreeSet<PathBuf>,
) -> Result<Vec<PathBuf>, FactsError> {
    let Some(root) = repo.workdir() else {
        return Ok(Vec::new());
    };
    let mut found = BTreeSet::new();
    for path in paths {
        match std::fs::symlink_metadata(root.join(path)) {
            Ok(meta) => {
                // A case-insensitive disk finds `Readme.md` under
                // `README.md`: a tracked file under another spelling is
                // git's to rename, not his in the way (review 28 M2).
                let stored = stored_as(root, path)?;
                if stored != *path && tracked.contains(&stored) {
                    continue;
                }
                if !meta.is_dir() || holds_untracked(root, &stored, tracked)? {
                    found.insert(path.clone());
                }
            }
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => {}
            Err(why) if why.kind() == std::io::ErrorKind::NotADirectory => {
                if let Some(above) = file_above(root, path)?
                    && !tracked.contains(&above)
                    && !tracked.contains(&stored_as(root, &above)?)
                {
                    found.insert(above);
                }
            }
            Err(source) => {
                return Err(FactsError::OnDisk {
                    path: path.clone(),
                    source,
                });
            }
        }
    }
    Ok(found.into_iter().collect())
}

/// The spelling the disk keeps `path` under, one component at a time. On
/// a disk that ignores case (macOS's default) or normalises Unicode, a
/// look-up of `README.md` finds `Readme.md`; the entry it found is the one
/// in the directory with the same inode. Only called for a path something
/// stands at, so the extra `read_dir` per component is paid only when
/// there is a file to judge.
fn stored_as(root: &Path, path: &Path) -> Result<PathBuf, FactsError> {
    use std::os::unix::fs::{DirEntryExt, MetadataExt};
    let mut stored = PathBuf::new();
    for name in path {
        let looked_up = stored.join(name);
        let on_disk = |source| FactsError::OnDisk {
            path: looked_up.clone(),
            source,
        };
        let inode = std::fs::symlink_metadata(root.join(&looked_up))
            .map_err(on_disk)?
            .ino();
        let mut same_inode = None;
        let mut exact = false;
        for entry in std::fs::read_dir(root.join(&stored)).map_err(on_disk)? {
            let entry = entry.map_err(on_disk)?;
            if entry.file_name() == name {
                exact = true;
                break;
            }
            if same_inode.is_none() && entry.ino() == inode {
                same_inode = Some(entry.file_name());
            }
        }
        match same_inode {
            Some(spelled) if !exact => stored.push(spelled),
            _ => stored.push(name),
        }
    }
    Ok(stored)
}

/// The first ancestor of `path` that stands on disk and isn't a directory.
fn file_above(root: &Path, path: &Path) -> Result<Option<PathBuf>, FactsError> {
    let mut ancestors: Vec<&Path> = path
        .ancestors()
        .skip(1)
        .filter(|above| !above.as_os_str().is_empty())
        .collect();
    ancestors.reverse();
    for above in ancestors {
        match std::fs::symlink_metadata(root.join(above)) {
            Ok(meta) if meta.is_dir() => {}
            Ok(_) => return Ok(Some(above.to_path_buf())),
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(source) => {
                return Err(FactsError::OnDisk {
                    path: above.to_path_buf(),
                    source,
                });
            }
        }
    }
    Ok(None)
}

/// Whether the directory at `dir` holds anything the index doesn't track,
/// at any depth. Stops at the first such entry.
fn holds_untracked(
    root: &Path,
    dir: &Path,
    tracked: &BTreeSet<PathBuf>,
) -> Result<bool, FactsError> {
    let on_disk = |source| FactsError::OnDisk {
        path: dir.to_path_buf(),
        source,
    };
    for entry in std::fs::read_dir(root.join(dir)).map_err(on_disk)? {
        let entry = entry.map_err(on_disk)?;
        let path = dir.join(entry.file_name());
        let is_dir = entry.file_type().map_err(on_disk)?.is_dir();
        if is_dir {
            if holds_untracked(root, &path, tracked)? {
                return Ok(true);
            }
        } else if !tracked.contains(&path) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Ignored or untracked: `git status` lists an untracked file (or, for a
/// directory, something under it); an ignored one it never lists.
pub(super) fn sort_out(found: Vec<PathBuf>, facts: &RepoFacts) -> Blocking {
    let listed: Vec<&Path> = facts
        .status
        .entries
        .iter()
        .filter(|entry| entry.untracked)
        .map(|entry| entry.path.as_path())
        .collect();
    let mut blocking = Blocking::default();
    for path in found {
        if listed.iter().any(|untracked| untracked.starts_with(&path)) {
            blocking.untracked.push(path);
        } else {
            blocking.ignored.push(path);
        }
    }
    blocking
}
