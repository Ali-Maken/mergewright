//! Which generation's names a repository is using (ADR-0057, Amendment
//! 2026-10-11 §3), read from the markers in its common dir.

use std::io;
use std::path::Path;

use graft_core::on_disk::{Generation, MIGRATED_MARKER};

use super::OpenError;

/// The generation in force in the repository whose common dir is
/// `common_dir`: the newest, up to [`Generation::CURRENT`], whose marker
/// exists; with none, Graft, which needs no marker. One `stat` per marked
/// generation, no spawn. Markers live in the common dir, never a worktree's
/// own: the refs are shared, so the names are one state per repository.
///
/// A handle reads this once at open ([`RepoHandle::on_disk_names`]); the
/// move reads it again under its lock, where a stale answer would copy a
/// generation onto itself.
///
/// [`RepoHandle::on_disk_names`]: super::RepoHandle::on_disk_names
///
/// # Errors
/// A marker whose existence cannot be told: guessing would risk reading one
/// set of names while the repository uses another.
pub fn names_in_force(common_dir: &Path) -> io::Result<Generation> {
    let newest_first = std::iter::once(Generation::CURRENT).chain(Generation::CURRENT.older());
    for generation in newest_first.filter(|generation| generation.has_marker()) {
        if common_dir
            .join(generation.dir())
            .join(MIGRATED_MARKER)
            .try_exists()?
        {
            return Ok(generation);
        }
    }
    Ok(Generation::Graft)
}

/// [`names_in_force`] at open: an unreadable marker is [`OpenError::Io`] on
/// the path the caller opened.
pub(super) fn names_in(common_dir: &Path, opened_from: &Path) -> Result<Generation, OpenError> {
    names_in_force(common_dir).map_err(|source| OpenError::Io {
        path: opened_from.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Read fresh, not from a handle: a marker removed after open reads as
    /// gone. Each marked generation older than `CURRENT` is found in turn.
    #[test]
    fn names_in_force_reads_the_markers_now() {
        let common = tempfile::tempdir().expect("a temp dir");
        assert_eq!(
            names_in_force(common.path()).expect("reads"),
            Generation::Graft
        );
        let newest_first = std::iter::once(Generation::CURRENT).chain(Generation::CURRENT.older());
        let with_one: Vec<Generation> = newest_first.filter(|g| g.has_marker()).collect();
        for generation in with_one.iter().rev() {
            let dir = common.path().join(generation.dir());
            std::fs::create_dir_all(&dir).expect("dir");
            std::fs::write(dir.join(MIGRATED_MARKER), "clean\n").expect("marker");
            assert_eq!(names_in_force(common.path()).expect("reads"), *generation);
        }
        for generation in &with_one {
            let marker = common.path().join(generation.dir()).join(MIGRATED_MARKER);
            std::fs::remove_file(marker).expect("removed");
        }
        assert_eq!(
            names_in_force(common.path()).expect("reads"),
            Generation::Graft
        );
    }
}
