//! The detail pane's saved split (ADR-0028): how wide the `Right` pane was,
//! how tall the `Below` strip was, each remembered on its own because a
//! column count means nothing as a row count once the window narrows into
//! the other orientation.
//!
//! Lives in the repository's common directory, like the commit-graph offer
//! (`graft_git::commit_graph`'s module doc): the split is a per-repository
//! preference, not a per-worktree one, and every worktree shares it.
//!
//! Reading or writing this file failing is not a Graft error: the file may
//! not exist yet (first run, or every prior run stayed at the default), or
//! the directory may be read-only. Either way the pane falls back to its
//! percentage default and says nothing, since a file that simply is not
//! there yet is the expected case here, not an exceptional one.

use std::io;
use std::path::Path;

use crate::update::detail::PaneSize;

/// The file the split lives in, under `common_dir`.
fn layout_path(common_dir: &Path) -> std::path::PathBuf {
    common_dir.join("graft").join("layout")
}

/// The saved split, if a file is there and at least one line of it parses.
/// `None` when the file is missing or unreadable — the caller falls back to
/// the percentage default, no error surfaced.
#[must_use]
pub fn load_split(common_dir: &Path) -> Option<PaneSize> {
    let text = std::fs::read_to_string(layout_path(common_dir)).ok()?;
    let mut split = PaneSize::default();
    let mut found = false;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("right=")
            && let Ok(columns) = value.trim().parse()
        {
            split.right_columns = Some(columns);
            found = true;
        } else if let Some(value) = line.strip_prefix("below=")
            && let Ok(rows) = value.trim().parse()
        {
            split.below_rows = Some(rows);
            found = true;
        }
    }
    found.then_some(split)
}

/// Write `split` to `<common_dir>/graft/layout`, two lines, one per
/// orientation — a line omitted when its value is `None`.
///
/// # Errors
/// [`io::Error`] if the `graft` directory cannot be created or the file
/// cannot be written. The caller (`event_loop::running`) treats this as
/// expected and non-fatal, per the module doc.
pub fn save_split(common_dir: &Path, split: &PaneSize) -> io::Result<()> {
    let path = layout_path(common_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut text = String::new();
    if let Some(columns) = split.right_columns {
        text.push_str(&format!("right={columns}\n"));
    }
    if let Some(rows) = split.below_rows {
        text.push_str(&format!("below={rows}\n"));
    }
    std::fs::write(path, text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_both_values() {
        let dir = tempfile::tempdir().expect("tempdir");
        let split = PaneSize {
            right_columns: Some(72),
            below_rows: Some(14),
        };

        save_split(dir.path(), &split).expect("save");
        let loaded = load_split(dir.path());

        assert_eq!(loaded, Some(split));
    }

    #[test]
    fn a_missing_file_is_none_without_error() {
        let dir = tempfile::tempdir().expect("tempdir");

        assert_eq!(load_split(dir.path()), None);
    }

    #[test]
    fn a_line_that_does_not_parse_leaves_that_orientation_none() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join("graft")).expect("mkdir");
        std::fs::write(
            dir.path().join("graft").join("layout"),
            "right=72\nbelow=nope\n",
        )
        .expect("write");

        let loaded = load_split(dir.path());

        assert_eq!(
            loaded,
            Some(PaneSize {
                right_columns: Some(72),
                below_rows: None,
            })
        );
    }
}
