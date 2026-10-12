//! The lines of README.md, CHANGELOG.md's top entry and `scripts/**` that
//! still say an earlier generation's title or command (`graft`, `tributary`,
//! `trib`) where a person would read it as the app's name
//! (m8-polish-and-ship/27c, 55). The test is
//! `rename_names::user_docs_and_scripts_name_the_command`.

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
];

/// `(text right before the word, text right after it, reason)`: a `graft`
/// is allowed when the line has `before` ending at the word and `after`
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

/// The same for `tributary` and `trib`: the second generation's names are
/// allowed only where a script names the path it moves away from.
const ALLOWED_LATER_AT: &[(&str, &str, &str)] = &[
    (
        "/",
        "/git-git.git",
        "clone-git-git.sh: the pre-rename cache it moves once",
    ),
    (
        "old `",
        "/`",
        "README: the old config folder, moved on first launch",
    ),
];

/// The earlier generations' words, and whether `_` ends the word: `trib_` was the
/// second generation's env prefix, while `graft_*` and `tributary_*` are Rust paths.
const OLD_WORDS: &[(&str, bool)] = &[("graft", false), ("tributary", false), ("trib", true)];

/// Every whole-word old name (any case; letters join a word, `-` does not)
/// in README.md, CHANGELOG.md's top entry and `scripts/**` not on an
/// allow-list, as `file:line: text`.
pub(crate) fn old_name_lines() -> Vec<String> {
    let root = crates_dir().parent().expect("repo root").to_path_buf();
    let mut found = Vec::new();
    let changelog = fs::read_to_string(root.join("CHANGELOG.md")).expect("CHANGELOG.md");
    for (at, line) in top_entry(&changelog).enumerate() {
        if old_words(line).any(|(word, before, after)| !allowed(word, before, after)) {
            found.push(format!("CHANGELOG.md:{}: {}", at + 1, line.trim()));
        }
    }
    let mut files = vec![root.join("README.md")];
    every_file(&root.join("scripts"), &mut files);
    assert!(files.len() > 40, "only {} files swept", files.len());
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
            if old_words(line).any(|(word, before, after)| !allowed(word, before, after)) {
                found.push(format!("{shown}:{}: {}", at + 1, line.trim()));
            }
        }
    }
    found
}

/// CHANGELOG.md up to its second `### ` heading: the preamble and the
/// newest entry. Older entries are records.
fn top_entry(text: &str) -> impl Iterator<Item = &str> {
    let mut headings = 0;
    text.lines().take_while(move |line| {
        if line.starts_with("### ") {
            headings += 1;
        }
        headings < 2
    })
}

fn allowed(word: &str, before: &str, after: &str) -> bool {
    let table = if word == "graft" {
        ALLOWED_AT
    } else {
        ALLOWED_LATER_AT
    };
    table
        .iter()
        .any(|(lead, tail, _)| before.ends_with(lead) && after.starts_with(tail))
}

/// Each whole-word old name in `line`, as (word, text before, text after).
fn old_words(line: &str) -> impl Iterator<Item = (&'static str, &str, &str)> {
    let lower = line.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let mut hits = Vec::new();
    for &(word, underscore_ends) in OLD_WORDS {
        let joins_after =
            |byte: u8| byte.is_ascii_alphanumeric() || (byte == b'_' && !underscore_ends);
        for (at, _) in lower.match_indices(word) {
            let before = at.checked_sub(1).map(|i| bytes[i]);
            let after = bytes.get(at + word.len()).copied();
            let joins_before = before.is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_');
            if !joins_before && !after.is_some_and(joins_after) {
                hits.push((word, at));
            }
        }
    }
    hits.sort_by_key(|&(_, at)| at);
    hits.into_iter()
        .map(move |(word, at)| (word, &line[..at], &line[at + word.len()..]))
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
    assert!(allowed("graft", "x ", "-ui"));
    assert!(!allowed("graft", "run ", " now"));
    let later: Vec<_> = old_words("Tributary, trib ., trib_dir, tributary_x, tributes, trib-12")
        .map(|(word, _, _)| word)
        .collect();
    assert_eq!(later, ["tributary", "trib", "trib", "trib"]);
    assert!(!allowed("trib", "x ", "-12"));
    assert!(allowed("tributary", ".../", "/git-git.git"));
    let top: Vec<_> = top_entry("pre\n### new\nnew\n### old\nold").collect();
    assert_eq!(top, ["pre", "### new", "new"]);
}
