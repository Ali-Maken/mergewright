//! Find-and-pin 3 of m6-conflicts/61: who reads the stop line's shorter
//! rungs (`banner::stop_wordings`, `banner::stop_line_wordings`).
//!
//! Only rung 0 (`stop_reason`) has fixed words any other draw may quote; the
//! shorter rungs change with the fitting rules. The one draw that shows
//! them is the graph footer, through `view::rows::stopped_line`. A new
//! reader would change its words whenever the ladder does, so it is named
//! here on purpose, and its captures regenerated with the ladder's.

use std::fs;
use std::path::{Path, PathBuf};

/// The calls that return rungs past the first.
const LADDER: &[&str] = &["stop_wordings(", "stop_line_wordings("];

#[test]
fn only_the_graph_footer_reads_the_short_stop_rungs() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .to_owned();
    let mut readers = Vec::new();
    for krate in ["mergewright", "mergewright-ui"] {
        let src = crates.join(krate).join("src");
        for file in rust_files(&src) {
            if is_test_file(&file) {
                continue;
            }
            let text = match fs::read_to_string(&file) {
                Ok(text) => text,
                Err(error) => panic!("could not read {}: {error}", file.display()),
            };
            let calls = text
                .lines()
                .filter(|line| !line.trim_start().starts_with("//"))
                .any(|line| LADDER.iter().any(|call| line.contains(call)));
            if calls {
                let relative = file
                    .strip_prefix(&crates)
                    .expect("under crates/")
                    .to_string_lossy()
                    .replace('\\', "/");
                readers.push(relative);
            }
        }
    }
    readers.sort();
    assert_eq!(
        readers,
        [
            // The definitions: `stop_line_wordings` calls `stop_wordings`.
            "mergewright/src/update/banner/stop/wording.rs",
            // A stash clash's ladder (m6-conflicts/72b), which `stop/wording.rs`
            // hands on when nothing is stopped: a definition, not a reader.
            "mergewright/src/update/clash.rs",
            // `stopped_line`, which the graph footer draws.
            "mergewright/src/view/rows.rs",
        ]
    );
}

/// A file only tests compile: `tests.rs`, or anything under a `tests/`
/// directory.
fn is_test_file(file: &Path) -> bool {
    file.file_name().is_some_and(|name| name == "tests.rs")
        || file.components().any(|part| part.as_os_str() == "tests")
}

fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) => panic!("could not list {}: {error}", dir.display()),
    };
    let mut files = Vec::new();
    for entry in entries {
        let path = match entry {
            Ok(entry) => entry.path(),
            Err(error) => panic!("could not list {}: {error}", dir.display()),
        };
        if path.is_dir() {
            files.extend(rust_files(&path));
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
    files
}
