//! The app's one-time move to its new on-disk names (m8/26) marked done in
//! a fixture, as the app leaves every repository it has opened:
//! `<git dir>/tributary/migrated` = `clean` (`graft_core::on_disk`, which
//! this crate may not depend on; `graft/tests/it/rename_status.rs` pins the
//! two together). Without it the first test to open a shared cached
//! fixture would write the marker and its lock into it while other tests
//! copy it. One writer for the corpus, the synthetic and the team fixtures
//! (review m8-26 Minor 9).

use std::fs;
use std::io;
use std::path::Path;

/// Write the marker into the non-bare repository at `dir`.
pub(crate) fn names_settled(dir: &Path) -> io::Result<()> {
    let folder = dir.join(".git").join("tributary");
    fs::create_dir_all(&folder)?;
    fs::write(folder.join("migrated"), "clean\n")
}

/// Whether `dir` carries the marker, as [`names_settled`] writes it.
#[cfg(test)]
pub(crate) fn is_settled(dir: &Path) -> bool {
    fs::read_to_string(dir.join(".git/tributary/migrated")).is_ok_and(|text| text == "clean\n")
}

#[cfg(test)]
mod tests {
    use super::is_settled;
    use crate::team::{TeamSpec, borrow};

    /// Review m8-26 Minor 9: the cached team fixture carries the marker.
    /// Here rather than in `team.rs`, which the file-size ratchet holds.
    #[test]
    fn team_built_with_the_marker() {
        let root = borrow(&TeamSpec::SMALL).expect("borrow");
        assert!(is_settled(&root));
    }
}
