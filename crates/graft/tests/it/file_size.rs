//! File-size ratchet (PROMPT.md 6.2: files stay around 400 lines).
//!
//! Every `.rs` under `crates/*/src` and `crates/*/tests` that is over 400
//! lines has a row in `over_400.txt` giving its exact count. A new oversize
//! file, a listed file that grew or shrank, and a row whose file is gone or
//! back under the cap all fail. Lowering a row is a one-line edit; raising
//! one shows in the diff. Lines are newline bytes, as `wc -l` counts them.
#![allow(
    clippy::expect_used,
    reason = "an unreadable workspace tree has nothing to measure; the panic names why"
)]
#![allow(
    clippy::naive_bytecount,
    reason = "a few hundred source files; not worth the bytecount dependency"
)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const CAP: usize = 400;
const LIST: &str = include_str!("over_400.txt");

fn repo_root() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    // crates/graft -> repo root
    manifest
        .parent()
        .and_then(Path::parent)
        .expect("crates/graft sits two levels below the repo root")
        .to_path_buf()
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display()));
    for entry in entries {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == "target") {
                continue;
            }
            collect(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Every workspace `.rs` file's line count, keyed by path from the repo root.
fn measured(root: &Path) -> BTreeMap<String, usize> {
    let mut files = Vec::new();
    let crates = root.join("crates");
    for krate in fs::read_dir(&crates).expect("read crates/") {
        let krate = krate.expect("crate entry").path();
        for sub in ["src", "tests"] {
            let dir = krate.join(sub);
            if dir.is_dir() {
                collect(&dir, &mut files);
            }
        }
    }
    files
        .into_iter()
        .map(|p| {
            let bytes = fs::read(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()));
            let lines = bytes.iter().filter(|&&b| b == b'\n').count();
            let rel = p
                .strip_prefix(root)
                .expect("under root")
                .to_string_lossy()
                .replace('\\', "/");
            (rel, lines)
        })
        .collect()
}

/// The list's rows: path -> recorded count.
fn listed(text: &str) -> BTreeMap<String, usize> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let (n, path) = l.split_once(' ').unwrap_or_else(|| panic!("bad row: {l}"));
            let n = n
                .parse()
                .unwrap_or_else(|e| panic!("bad count in {l}: {e}"));
            (path.trim().to_owned(), n)
        })
        .collect()
}

fn is_test_path(path: &str) -> bool {
    path.split('/').any(|seg| seg == "tests") || path.ends_with("/tests.rs")
}

/// Rules 1–3: unlisted oversize files, and listed files whose count moved.
fn unlisted_or_moved(
    sizes: &BTreeMap<String, usize>,
    rows: &BTreeMap<String, usize>,
) -> Vec<String> {
    let mut problems = Vec::new();
    for (path, &lines) in sizes {
        if lines <= CAP {
            continue;
        }
        match rows.get(path) {
            None => problems.push(format!(
                "{path}: {lines} lines, over {CAP} and not in over_400.txt"
            )),
            Some(&n) if lines > n => problems.push(format!("{path}: grew from {n} to {lines}")),
            Some(&n) if lines < n => {
                problems.push(format!(
                    "{path}: shrank to {lines}: lower the row (was {n})"
                ));
            }
            Some(_) => {}
        }
    }
    problems
}

/// Rule 4: rows whose file is gone or back at or under the cap.
fn stale(sizes: &BTreeMap<String, usize>, rows: &BTreeMap<String, usize>) -> Vec<String> {
    rows.keys()
        .filter_map(|path| match sizes.get(path) {
            None => Some(format!("{path}: no longer exists: remove the row")),
            Some(&lines) if lines <= CAP => {
                Some(format!("{path}: now {lines} lines: remove the row"))
            }
            Some(_) => None,
        })
        .collect()
}

#[test]
fn every_file_over_400_lines_is_listed_with_its_count() {
    let sizes = measured(&repo_root());
    let over: Vec<&String> = sizes
        .iter()
        .filter(|&(_, &n)| n > CAP)
        .map(|(p, _)| p)
        .collect();
    let tests = over.iter().filter(|p| is_test_path(p)).count();
    eprintln!(
        "over_400: {} production, {} test files",
        over.len() - tests,
        tests
    );
    let problems = unlisted_or_moved(&sizes, &listed(LIST));
    assert!(
        problems.is_empty(),
        "file-size ratchet:\n{}",
        problems.join("\n")
    );
}

#[test]
fn the_size_list_has_no_stale_rows() {
    let problems = stale(&measured(&repo_root()), &listed(LIST));
    assert!(
        problems.is_empty(),
        "file-size ratchet:\n{}",
        problems.join("\n")
    );
}

#[test]
fn the_rules_fire_on_a_made_up_tree() {
    let sizes: BTreeMap<String, usize> =
        [("a.rs", 401), ("b.rs", 500), ("c.rs", 450), ("d.rs", 10)]
            .into_iter()
            .map(|(p, n)| (p.to_owned(), n))
            .collect();
    let rows = listed("# rule\n499 b.rs\n460 c.rs\n10 d.rs\n500 gone.rs\n");
    let moved = unlisted_or_moved(&sizes, &rows).join("\n");
    assert!(moved.contains("a.rs: 401 lines"), "{moved}");
    assert!(moved.contains("grew from 499 to 500"), "{moved}");
    assert!(moved.contains("shrank to 450: lower the row"), "{moved}");
    let stale = stale(&sizes, &rows).join("\n");
    assert!(
        stale.contains("d.rs: now 10 lines: remove the row"),
        "{stale}"
    );
    assert!(
        stale.contains("gone.rs: no longer exists: remove the row"),
        "{stale}"
    );
}
