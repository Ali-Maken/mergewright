//! The source tree, for the tests that grep it (`rename_names`, `rename_docs`).

#![allow(clippy::expect_used, reason = "a failing test should panic")]

use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn crates_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/mergewright has a parent")
        .to_path_buf()
}

/// Every production `.rs` file under `root`: no `tests.rs`, `*_tests.rs`
/// or file under a `tests` folder.
pub(crate) fn production_files(root: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(root)
        .unwrap_or_else(|why| panic!("{}: {why}", root.display()))
        .map(|entry| entry.expect("a dir entry").path())
        .collect();
    entries.sort();
    for path in entries {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if path.is_dir() {
            if name != "tests" {
                production_files(&path, out);
            }
        } else if path.extension().is_some_and(|ext| ext == "rs")
            && name != "tests.rs"
            && !name.ends_with("_tests.rs")
        {
            out.push(path);
        }
    }
}
