//! Source greps that keep the one-time rename (ticket 26 Part B) whole:
//! every repository the app opens is settled first, and no code names the
//! old folder but `graft_core::on_disk` and the spots ticket 27 has not reached.

#![allow(clippy::expect_used, reason = "a failing test should panic")]

use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn crates_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/graft has a parent")
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

/// The code lines of a production file, numbered from 1: comments and the
/// inline `#[cfg(test)]` module at its end left out.
pub(crate) fn code_lines(path: &Path) -> Vec<(usize, String)> {
    let text = fs::read_to_string(path).unwrap_or_else(|why| panic!("{}: {why}", path.display()));
    let lines: Vec<&str> = text.lines().collect();
    let mut code = Vec::new();
    for (at, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        let next = lines.get(at + 1).map_or("", |next| next.trim_start());
        if trimmed == "#[cfg(test)]" && next.starts_with("mod tests {") {
            break;
        }
        if !trimmed.starts_with("//") {
            code.push((at + 1, (*line).to_owned()));
        }
    }
    code
}

/// Each `(file, line)` in `files` whose code contains one of `needles`,
/// as `crate/src/path:line: code`.
pub(crate) fn hits(files: &[PathBuf], needles: &[&str]) -> Vec<String> {
    let crates = crates_dir();
    let mut found = Vec::new();
    for file in files {
        let shown = file
            .strip_prefix(&crates)
            .unwrap_or(file)
            .display()
            .to_string();
        for (number, line) in code_lines(file) {
            if needles.iter().any(|needle| line.contains(needle)) {
                found.push(format!("{shown}:{number}: {}", line.trim()));
            }
        }
    }
    found
}

/// The app's every repository open goes through `RepoStore::shared`, which
/// settles the names before any job takes a handle (`graft::rename`). A
/// bare open anywhere else in `crates/graft/src` would read the old names
/// on a repository not yet moved, or move it under a job's feet.
#[test]
fn every_open_settles_first() {
    let mut files = Vec::new();
    production_files(&crates_dir().join("graft/src"), &mut files);
    let found: Vec<String> = hits(&files, &["SharedRepo::open(", "RepoHandle::open("])
        .into_iter()
        // The one place that opens: it settles, then reopens if the names
        // changed. Its doc line names `SharedRepo::open` too.
        .filter(|hit| !hit.starts_with("graft/src/rename.rs:"))
        .collect();
    assert!(
        found.is_empty(),
        "repository opens that skip the settle:\n{}",
        found.join("\n")
    );
    // The filter above is not hiding the open itself.
    assert!(
        hits(
            &[crates_dir().join("graft/src/rename.rs")],
            &["SharedRepo::open("]
        )
        .len()
            >= 2,
        "rename.rs opens, then reopens"
    );
}

/// Where the old name may still be spelled in code, and why.
const ALLOWED: &[(&str, &str)] = &[
    (
        "graft-core/src/on_disk.rs",
        "the one place every generation's names live (the `Generation` table)",
    ),
    (
        "graft-fixtures/src/settled.rs",
        "may not depend on graft-core; `rename_status.rs` pins its marker to the table",
    ),
];

/// No production code spells an old on-disk path but the allowed files:
/// every path goes through `Generation` (or `config_names`), so the move
/// and the names in force can never disagree.
#[test]
fn no_old_dir_literal() {
    let mut files = Vec::new();
    let mut crates: Vec<_> = fs::read_dir(crates_dir())
        .expect("crates/ reads")
        .map(|entry| entry.expect("a dir entry").path())
        .collect();
    crates.sort();
    for krate in crates {
        let src = krate.join("src");
        if src.is_dir() {
            production_files(&src, &mut files);
        }
    }
    // Every generation's folder name, the one in force included: each path
    // is built from the table, never spelled (ticket 48 Part B).
    let needles: Vec<String> = graft_core::on_disk::Generation::GENERATIONS
        .iter()
        .flat_map(|generation| {
            let dir = generation.dir();
            [
                format!("\"{dir}\""),
                format!("\"{dir}/"),
                format!("/{dir}\""),
                format!("/{dir}/"),
            ]
        })
        .collect();
    let needles: Vec<&str> = needles.iter().map(String::as_str).collect();
    let found: Vec<String> = hits(&files, &needles)
        .into_iter()
        .filter(|hit| {
            !ALLOWED
                .iter()
                .any(|(file, _)| hit.starts_with(&format!("{file}:")))
        })
        .collect();
    assert!(
        found.is_empty(),
        "old on-disk names spelled in code:\n{}",
        found.join("\n")
    );
}
