//! M7's promise, end to end (m7-acceptance): "you can rewrite, trace and
//! recover history without leaving the app". One fixture per scenario; no
//! shell step after the fixture is built. Each scenario's key count is the
//! sum of its journeys' routes, asserted exactly.
//!
//! Also the review-28 Q2 carry: a guard refusal at 80 columns names the file
//! and the next step on the status line ([`guard`]).

pub mod guard;
pub mod recover;
pub mod rewrite;
pub mod trace;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Every file under `repo`'s working tree (not `.git`), with its bytes: what
/// a byte-exact `u` must give back.
pub fn tree_bytes(repo: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut dirs = vec![repo.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        let entries =
            std::fs::read_dir(&dir).unwrap_or_else(|why| panic!("{}: {why}", dir.display()));
        for entry in entries {
            let path = entry
                .unwrap_or_else(|why| panic!("{}: {why}", dir.display()))
                .path();
            if path.file_name().is_some_and(|name| name == ".git") {
                continue;
            }
            if path.is_dir() {
                dirs.push(path);
            } else {
                let bytes =
                    std::fs::read(&path).unwrap_or_else(|why| panic!("{}: {why}", path.display()));
                let relative = path.strip_prefix(repo).unwrap_or(&path).to_path_buf();
                files.insert(relative, bytes);
            }
        }
    }
    files
}
