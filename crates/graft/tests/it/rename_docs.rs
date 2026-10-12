//! The lines of README.md and `scripts/**` that still say `graft` where a
//! person would read it as the app's name (m8-polish-and-ship/27c). The test
//! is `rename_names::user_docs_and_scripts_name_the_command`.

#![allow(clippy::expect_used, reason = "a failing test should panic")]

use std::fs;
use std::path::{Path, PathBuf};

use crate::rename_opens::crates_dir;

/// Whole files allowed to say `graft`, and why.
const ALLOWED_FILES: &[(&str, &str)] = &[
    (
        ".patch",
        "a diff against the source: its context lines must match the code",
    ),
    (
        "scripts/shot-drift.py",
        "finds the old name in captures, so it spells it",
    ),
    (
        "scripts/shot-drift-test.py",
        "plants each earlier name in a capture for shot-drift.py to find",
    ),
];

/// `(text right before the word, text right after it, reason)`: a match is
/// allowed when the line has `before` ending at the word and `after`
/// starting after it. Empty means "anything".
const ALLOWED_AT: &[(&str, &str, &str)] = &[
    ("-p ", "", "package name `graft` (cargo -p): a code name"),
    ("crates/", "", "crate folder `crates/graft`: a code name"),
    (
        "",
        "-",
        "crate names `graft-*`, and dist archives named after the package",
    ),
    (
        "",
        "::",
        "library paths `graft::…`: the lib crate keeps its name",
    ),
    (
        "",
        ".rb",
        "dist names the Homebrew formula after the package",
    ),
    ("the ", " package", "the package by name"),
    ("package ", "", "the package by name"),
    (
        "--features ",
        "/",
        "cargo feature `graft/<feature>`: package name",
    ),
    (
        "old `",
        "/`",
        "README: the old config folder, moved on first launch (26)",
    ),
];

/// Every whole-word `graft` (any case; `_` and letters join a word, `-`
/// does not) in README.md and `scripts/**` not on an allow-list, as
/// `file:line: text`.
pub(crate) fn old_name_lines() -> Vec<String> {
    let root = crates_dir().parent().expect("repo root").to_path_buf();
    let mut files = vec![root.join("README.md")];
    every_file(&root.join("scripts"), &mut files);
    assert!(files.len() > 40, "only {} files swept", files.len());
    let mut found = Vec::new();
    for file in &files {
        let shown = file
            .strip_prefix(&root)
            .unwrap_or(file)
            .display()
            .to_string();
        if ALLOWED_FILES.iter().any(|(end, _)| shown.ends_with(end)) {
            continue;
        }
        let Ok(text) = fs::read_to_string(file) else {
            continue; // not text: nothing a person reads
        };
        for (at, line) in text.lines().enumerate() {
            if old_words(line).any(|(before, after)| !allowed(before, after)) {
                found.push(format!("{shown}:{}: {}", at + 1, line.trim()));
            }
        }
    }
    found
}

fn allowed(before: &str, after: &str) -> bool {
    ALLOWED_AT
        .iter()
        .any(|(lead, tail, _)| before.ends_with(lead) && after.starts_with(tail))
}

/// Each whole-word `graft` in `line`, as (text before, text after).
fn old_words(line: &str) -> impl Iterator<Item = (&str, &str)> {
    let lower = line.to_ascii_lowercase();
    let word = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'_';
    let bytes = lower.as_bytes().to_vec();
    let hits: Vec<usize> = lower
        .match_indices("graft")
        .map(|(at, _)| at)
        .filter(|&at| {
            let before = at.checked_sub(1).map(|i| bytes[i]);
            let after = bytes.get(at + 5).copied();
            !before.is_some_and(word) && !after.is_some_and(word)
        })
        .collect();
    hits.into_iter()
        .map(move |at| (&line[..at], &line[at + 5..]))
}

fn every_file(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let path = entry.expect("a dir entry").path();
        if path.is_dir() {
            every_file(&path, out);
        } else {
            out.push(path);
        }
    }
}

#[test]
fn the_word_finder_reads_words_only() {
    let words: Vec<_> = old_words("Graft and graft_x and graft-ui, grafted").collect();
    assert_eq!(words.len(), 2);
    assert!(allowed("x graft", "-ui"));
    assert!(!allowed("run ", " now"));
}
