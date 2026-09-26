//! Which of the written paths his bytes stand on, and whether git calls
//! each one untracked or ignored. The only worktree `symlink_metadata` in
//! the crate.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use graft_git::repo::RepoHandle;

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
            Ok(meta) if meta.is_dir() => {
                if holds_untracked(root, path, tracked)? {
                    found.insert(path.clone());
                }
            }
            Ok(_) => {
                found.insert(path.clone());
            }
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => {}
            Err(why) if why.kind() == std::io::ErrorKind::NotADirectory => {
                if let Some(above) = file_above(root, path)?
                    && !tracked.contains(&above)
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
