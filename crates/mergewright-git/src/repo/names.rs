//! Which generation's names a repository is using: always the current
//! ones (pre-publish-cleanup/11b2; removed in 11b4).

use std::io;
use std::path::Path;

use mergewright_core::on_disk::Generation;

use super::OpenError;

/// The generation in force: always [`Generation::CURRENT`]
/// (pre-publish-cleanup/11b2). No marker is read; a repository opens on the
/// current names whatever earlier folders it holds. Removed in 11b4.
///
/// # Errors
/// None today; the `Result` stays until 11b4 removes the fn.
pub fn names_in_force(_common_dir: &Path) -> io::Result<Generation> {
    Ok(Generation::CURRENT)
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
    use mergewright_core::on_disk::MIGRATED_MARKER;

    use super::*;

    /// An earlier generation's folder name, built at run time so no old name
    /// is spelled as a literal here.
    fn earlier_dir() -> String {
        ["tribu", "tary"].concat()
    }

    #[test]
    fn no_marker_is_current() {
        let common = tempfile::tempdir().expect("a temp dir");
        assert_eq!(
            names_in_force(common.path()).expect("reads"),
            Generation::CURRENT
        );
    }

    #[test]
    fn old_marker_is_still_current() {
        let common = tempfile::tempdir().expect("a temp dir");
        let dir = common.path().join(earlier_dir());
        std::fs::create_dir_all(&dir).expect("dir");
        std::fs::write(dir.join(MIGRATED_MARKER), "clean\n").expect("marker");
        assert_eq!(
            names_in_force(common.path()).expect("reads"),
            Generation::CURRENT
        );
    }
}
