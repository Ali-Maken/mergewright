//! No shipped file says the project's codename or an earlier name, in its
//! content or its path (pre-publish-cleanup/11a). The shipped set is the
//! export allow-list; the exceptions are rows in `codename_allowed.txt`.

#![allow(clippy::expect_used, reason = "a failing test should panic")]

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::source_files::crates_dir;

/// The files that ship: `docs/release/export-inventory.md` §8, plus the
/// conductor's ruling that `DESIGN.md` and `docs/design/**` ship.
/// `TESTING.md` is left out until the licence step splits its private
/// "Times I left" section from the checklist that ships.
/// A trailing `/` takes the folder, a trailing `*` any path with that start.
const SHIPPED: &[&str] = &[
    "Cargo.toml",
    "Cargo.lock",
    ".cargo/config.toml",
    ".config/nextest.toml",
    "clippy.toml",
    "rustfmt.toml",
    ".gitignore",
    ".github/workflows/",
    "README.md",
    "CHANGELOG.md",
    "ARCHITECTURE.md",
    "CONTEXT.md",
    "CAPABILITIES.md",
    "LICENSE*",
    "crates/",
    "fuzz/Cargo.toml",
    "fuzz/Cargo.lock",
    "fuzz/README.md",
    "fuzz/fuzz_targets/",
    "packaging/",
    "docs/man/",
    "docs/shots/",
    "docs/capabilities/",
    "scripts/lib/",
    "scripts/testdata/",
    "scripts/check-scripts.sh",
    "scripts/check-sandbox.py",
    "scripts/check-sandbox-test.py",
    "scripts/mwr_sandbox.py",
    "scripts/contact-sheet.sh",
    "scripts/shot-drift.py",
    "scripts/shot-drift-test.py",
    "scripts/release-local.sh",
    "scripts/cargo-install-check.sh",
    "scripts/git-matrix.sh",
    "scripts/linux-fast-tier.sh",
    "scripts/fuzz-run.sh",
    "scripts/memory-curve.sh",
    "scripts/clone-git-git.sh",
    "scripts/formula.py",
    "scripts/m8-accept.sh",
    "DESIGN.md",
    "docs/design/",
];

/// Every earlier name (ADR-0057 Amendment §2), matched in any case.
const WORDS: &[&str] = &["graft", "tributary", "trib_"];

/// Shipped paths another ticket clears, with that ticket.
/// Nothing is pending: pre-publish-cleanup/16 regenerated the last stale
/// captures (the eight M4 stems), so none still names an earlier product.
const PENDING: &[(&str, &str)] = &[];

/// Rows of `<path>\t<exact substring>\t<reason>`.
const ALLOWED: &str = include_str!("codename_allowed.txt");
const ALLOWED_PATH: &str = "crates/mergewright/tests/it/codename_allowed.txt";

fn matches(pattern: &str, path: &str) -> bool {
    if let Some(start) = pattern.strip_suffix('*') {
        path.starts_with(start)
    } else if pattern.ends_with('/') {
        path.starts_with(pattern)
    } else {
        path == pattern
    }
}

fn repo_root() -> PathBuf {
    crates_dir().parent().expect("repo root").to_path_buf()
}

/// Tracked files in the shipped set, minus the pending ones.
fn shipped_files() -> Vec<String> {
    let out = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(repo_root())
        .output()
        .expect("git ls-files runs");
    assert!(out.status.success(), "git ls-files failed");
    let all = String::from_utf8(out.stdout).expect("utf-8 paths");
    all.split('\0')
        .filter(|path| !path.is_empty())
        .filter(|path| SHIPPED.iter().any(|pattern| matches(pattern, path)))
        .filter(|path| !PENDING.iter().any(|(pattern, _)| matches(pattern, path)))
        // The allow-list itself: every row in it names a word by design.
        .filter(|path| *path != ALLOWED_PATH)
        .map(str::to_owned)
        .collect()
}

/// `(path, substring, reason)` for each allow-list row.
fn allowed_rows() -> Vec<(&'static str, &'static str, &'static str)> {
    ALLOWED
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let mut cells = line.split('\t');
            let path = cells.next().expect("path");
            let found = cells
                .next()
                .unwrap_or_else(|| panic!("no substring: {line}"));
            let reason = cells.next().unwrap_or_else(|| panic!("no reason: {line}"));
            assert!(
                ["git-word", "guard"].contains(&reason),
                "unknown reason: {line}"
            );
            (path, found, reason)
        })
        .collect()
}

/// `text` with every allowed substring blanked out, then whether it still
/// holds a word.
fn says_word(text: &str, allowed: &[&str]) -> bool {
    let mut text = text.to_owned();
    for found in allowed {
        text = text.replace(found, " ");
    }
    let lower = text.to_lowercase();
    WORDS.iter().any(|word| lower.contains(word))
}

#[test]
fn no_shipped_file_names_the_codename() {
    let root = repo_root();
    let rows = allowed_rows();
    let files = shipped_files();
    assert!(files.len() > 1000, "only {} files swept", files.len());
    let mut hits = Vec::new();
    for path in &files {
        let allowed: Vec<&str> = rows
            .iter()
            .filter(|(at, _, _)| at == path)
            .map(|(_, found, _)| *found)
            .collect();
        if says_word(path, &allowed) {
            hits.push(format!("{path}: (path)"));
        }
        let Ok(text) = fs::read_to_string(root.join(path)) else {
            continue; // not UTF-8 text: a PNG or a fuzz input, nothing read as words
        };
        for (at, line) in text.lines().enumerate() {
            if says_word(line, &allowed) {
                hits.push(format!("{path}:{}: {}", at + 1, line.trim()));
            }
        }
    }
    assert!(
        hits.is_empty(),
        "{} shipped lines name the codename:\n{}",
        hits.len(),
        hits.join("\n")
    );
}

#[test]
fn every_allowed_codename_occurrence_still_exists() {
    let root = repo_root();
    let mut stale = Vec::new();
    for (path, found, _) in allowed_rows() {
        match fs::read_to_string(root.join(path)) {
            Ok(text) if text.contains(found) => {}
            Ok(_) => stale.push(format!("{path}\t{found}")),
            Err(why) => stale.push(format!("{path}\t{found}\t({why})")),
        }
    }
    assert!(
        stale.is_empty(),
        "stale allow-list rows:\n{}",
        stale.join("\n")
    );
}

#[test]
fn the_word_test_blanks_allowed_text() {
    assert!(says_word("Graft opens", &[]));
    assert!(!says_word("a grafted commit", &["grafted"]));
    assert!(says_word("a grafted graft", &["grafted"]));
    assert!(says_word("Trib_x", &[]));
    assert!(matches("crates/", "crates/a.rs"));
    assert!(matches("LICENSE*", "LICENSE-MIT"));
    assert!(!matches("README.md", "README.md.bak"));
}
