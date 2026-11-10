//! The package and the command follow the names table (m8-polish-and-ship/27a,
//! 49): the binary's file name, no old name left in a string the user can
//! read, no script running an earlier generation's binary, and no live file
//! naming the old package.

#![allow(clippy::expect_used, reason = "a failing test should panic")]

use std::fs;
use std::path::{Path, PathBuf};

use mergewright_core::on_disk;

use crate::source_files::{crates_dir, production_files};

/// The package is the current generation's dir name and the binary its
/// command: `mergewright` and `mwr` (49).
#[test]
fn package_and_bin_follow_the_table() {
    assert_eq!(env!("CARGO_PKG_NAME"), on_disk::DIR);
    let binary = Path::new(env!("CARGO_BIN_EXE_mwr"));
    let name = binary.file_stem().and_then(|name| name.to_str());
    assert_eq!(name, Some(mergewright::cli::COMMAND));
    assert_eq!(mergewright::cli::COMMAND, on_disk::COMMAND);
}

/// The binary Cargo builds is named after the command every message uses.
#[test]
fn the_binary_file_is_the_command() {
    let binary = Path::new(env!("CARGO_BIN_EXE_mwr"));
    let name = binary
        .file_stem()
        .and_then(|name| name.to_str())
        .expect("a file name");
    assert_eq!(name, mergewright::cli::COMMAND);
}

/// Where a string literal in production code may still say `graft`, and why:
/// `(file or folder prefix, text the literal contains, reason)`. An empty
/// text allows every literal under the prefix. None of these is drawn on
/// screen.
const ALLOWED: &[(&str, &str, &str)] = &[
    (
        "mergewright-fixtures/",
        "",
        "test-fixture crate: identities, temp prefixes, its usage",
    ),
    (
        "mergewright-capabilities/",
        "",
        "the CAPABILITIES generator's prose: docs words, 27c",
    ),
    // Thread names: in a backtrace, never on screen; code names.
    (
        "mergewright-term/src/input/mux.rs",
        "graft-input",
        "thread name",
    ),
    // Temporary file, socket and marker names: code names, gone after use.
    (
        "mergewright-git/src/conflict/disk.rs",
        "graft-",
        "temp file prefix",
    ),
    (
        "mergewright-git/src/run/askpass.rs",
        "graft-askpass",
        "socket and helper temp names",
    ),
    (
        "mergewright-git/src/trace/function.rs",
        "graft-marker-",
        "internal trace marker",
    ),
    (
        "mergewright-git/src/fixture.rs",
        "Graft Fixture",
        "fixture author identity",
    ),
    (
        "mergewright-ops/src/merge_state.rs",
        ".graft-tmp",
        "temp file name",
    ),
    (
        "mergewright-ops/src/preview/ghost.rs",
        "graft-ghost",
        "scratch marker",
    ),
    (
        "mergewright-ops/src/rewrite/todo.rs",
        "graft-fork-",
        "temp dir prefix",
    ),
    (
        "mergewright-ops/src/rewrite/saved.rs",
        "Mergewright's message helper",
        "generated helper script: its comment and temp file name",
    ),
    // Perf examples (50): run by hand to measure, never drawn to the user.
    (
        "mergewright-git/examples/stash_only_cost.rs",
        "Graft Perf",
        "perf fixture's author identity",
    ),
    (
        "mergewright-ops/examples/snapshot_cost.rs",
        "graft",
        "perf example's temp prefix and scratch folder",
    ),
];

/// The string literals of a production file, with the line each starts on:
/// comments and the inline `#[cfg(test)] mod tests` at its end left out.
fn literals(path: &Path) -> Vec<(usize, String)> {
    let text = fs::read_to_string(path).unwrap_or_else(|why| panic!("{}: {why}", path.display()));
    let cut = text.find("#[cfg(test)]\nmod tests {").unwrap_or(text.len());
    let chars: Vec<char> = text[..cut].chars().collect();
    let mut found = Vec::new();
    let mut line = 1;
    let mut at = 0;
    while at < chars.len() {
        let here = chars[at];
        let next = chars.get(at + 1).copied();
        match here {
            '\n' => line += 1,
            '/' if next == Some('/') => {
                while at < chars.len() && chars[at] != '\n' {
                    at += 1;
                }
                continue;
            }
            // A char literal (`'"'`, `'\''`); a lifetime has no closing quote.
            '\'' if next == Some('\\') || chars.get(at + 2) == Some(&'\'') => {
                at += 1;
                while at < chars.len() && chars[at] != '\'' {
                    if chars[at] == '\\' {
                        at += 1;
                    }
                    at += 1;
                }
            }
            '"' => {
                let start = line;
                let mut literal = String::new();
                at += 1;
                while at < chars.len() && chars[at] != '"' {
                    if chars[at] == '\\' {
                        literal.push(chars[at]);
                        at += 1;
                    }
                    if chars.get(at) == Some(&'\n') {
                        line += 1;
                    }
                    if let Some(&ch) = chars.get(at) {
                        literal.push(ch);
                    }
                    at += 1;
                }
                found.push((start, literal));
            }
            _ => {}
        }
        at += 1;
    }
    found
}

/// `word` as a whole word, case-sensitive: `_` and alphanumerics bound it,
/// so `mergewright_core`, `MWR_X` and `grafted` do not match.
fn says_word(text: &str, word: &str) -> bool {
    let bytes = text.as_bytes();
    let inside = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'_';
    text.match_indices(word).any(|(at, _)| {
        let before = at.checked_sub(1).map(|i| bytes[i]);
        let after = bytes.get(at + word.len()).copied();
        !before.is_some_and(inside) && !after.is_some_and(inside)
    })
}

/// Every earlier generation's spellings: dir, title, command, env prefix,
/// ref prefix, trailer key, backup email. Written out once from the
/// on-disk names table as it stood (ticket pre-publish-cleanup/11b1), so the
/// guards outlive the table. Spells the codename on purpose.
const OLD_NAMES: &[&str] = &[
    "graft",
    "Graft",
    // Split so `scripts::no_old_env_name` does not read this list as a use.
    concat!("GRAFT", "_"),
    "refs/graft/",
    "Graft-Link:",
    "graft@backup",
    "tributary",
    "Tributary",
    "trib",
    concat!("TRIB", "_"),
    "refs/tributary/",
    "Tributary-Link:",
    "tributary@backup",
];

/// The earlier name `text` says as a whole word (`Graft`, `tributary` as in
/// a folder prefix, 51; `trib`), if any.
fn says_old_name(text: &str) -> Option<&'static str> {
    OLD_NAMES.iter().copied().find(|old| says_word(text, old))
}

/// The earlier message tag `line` starts with (`graft:`, `tributary ` …),
/// if any: the check `no_old_message_literal` runs on each literal. A tag
/// is a generation's dir, read off its ref prefix `refs/<dir>/`.
fn old_name_in(line: &str) -> Option<&'static str> {
    OLD_NAMES
        .iter()
        .filter_map(|old| old.strip_prefix("refs/")?.strip_suffix('/'))
        .find(|tag| {
            line.strip_prefix(tag)
                .is_some_and(|rest| rest.starts_with([':', ' ']))
        })
}

/// Every production source file under `crates/*/src` and `crates/*/examples`,
/// in path order.
fn every_production_file(crates: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = Vec::new();
    let mut dirs: Vec<_> = fs::read_dir(crates)
        .expect("crates/ reads")
        .map(|entry| entry.expect("a dir entry").path())
        .collect();
    dirs.sort();
    for krate in dirs {
        for part in ["src", "examples"] {
            let dir = krate.join(part);
            if dir.is_dir() {
                production_files(&dir, &mut files);
            }
        }
    }
    files
}

/// No production string literal starts with an earlier generation's
/// message tag (`graft:`, `graft `, `tributary:` …): every message written into git takes its tag from
/// `on_disk::MESSAGE_TAG`, and every reader goes through
/// `on_disk::Own::is_message` (ticket m8/52).
#[test]
fn no_old_message_literal() {
    let crates = crates_dir();
    let mut found = Vec::new();
    for file in every_production_file(&crates) {
        let shown = file
            .strip_prefix(&crates)
            .unwrap_or(&file)
            .display()
            .to_string();
        for (line, literal) in literals(&file) {
            if old_name_in(&literal).is_some() {
                found.push(format!("{shown}:{line}: \"{literal}\""));
            }
        }
    }
    assert!(
        found.is_empty(),
        "an earlier generation's message tag in a literal:\n{}",
        found.join("\n")
    );
}

/// No string literal in any crate's production source names an earlier
/// generation but the allowed files: the user reads `mwr` or Mergewright
/// everywhere (50).
#[test]
fn no_old_name_in_user_visible_strings() {
    let crates = crates_dir();
    let files = every_production_file(&crates);
    let mut found = Vec::new();
    for file in &files {
        let shown = file
            .strip_prefix(&crates)
            .unwrap_or(file)
            .display()
            .to_string();
        for (line, literal) in literals(file) {
            let allowed = ALLOWED
                .iter()
                .any(|(prefix, text, _)| shown.starts_with(prefix) && literal.contains(text));
            if allowed {
                continue;
            }
            // `include_str!`/`env!` paths name crate folders, not the app.
            let path_like = literal.contains("crates/mergewright") || literal.starts_with("../");
            if says_old_name(&literal).is_some() && !path_like {
                found.push(format!("{shown}:{line}: \"{literal}\""));
            }
        }
    }
    assert!(
        found.is_empty(),
        "the old name in a string the user can read:\n{}",
        found.join("\n")
    );
}

#[test]
fn old_name_in_flags_old_tags() {
    assert_eq!(old_name_in("graft: your edits, set aside"), Some("graft"));
    assert_eq!(old_name_in("tributary 01J0 stage"), Some("tributary"));
    assert_eq!(old_name_in("mergewright: your edits, set aside"), None);
    assert_eq!(old_name_in("mergewright 01J0 stage"), None);
}

#[test]
fn the_word_test_reads_words_only() {
    assert_eq!(says_old_name("graft crashed"), Some("graft"));
    assert_eq!(says_old_name("usage: Graft"), Some("Graft"));
    assert_eq!(
        says_old_name("Tributary cannot read this"),
        Some("Tributary")
    );
    assert_eq!(says_old_name("trib: stopped"), Some("trib"));
    assert_eq!(says_old_name("mergewright_core"), None);
    assert_eq!(says_old_name("MWR_PERF_TRACE"), None);
    assert_eq!(says_old_name("grafted"), None);
    assert_eq!(says_old_name("tributary_ish attribute"), None);
    assert_eq!(says_old_name("tributary-practice-"), Some("tributary"));
    assert_eq!(says_old_name("Mergewright · mwr"), None);
}

/// The gallery looks for git/git where `scripts/clone-git-git.sh` puts it
/// (review m8-mergewright-51, Major 1): same `HOME`, no overrides, same path.
#[test]
fn gallery_git_git_default_is_the_scripts_path() {
    let home = tempfile::tempdir().expect("a temp home");
    let root = crates_dir().parent().expect("repo root").to_path_buf();
    let out = std::process::Command::new("bash")
        .arg(root.join("scripts/clone-git-git.sh"))
        .arg("--where")
        .env_clear()
        .env("HOME", home.path())
        .env("PATH", std::env::var_os("PATH").expect("PATH"))
        .output()
        .expect("the script runs");
    assert!(out.status.success(), "{out:?}");
    let script = String::from_utf8(out.stdout).expect("utf-8");
    let roots = crate::gallery::roots_with(&root, |key| {
        (key == "HOME").then(|| home.path().as_os_str().to_owned())
    });
    assert_eq!(roots.git_git, PathBuf::from(script.trim_end()));
}

mod scripts;
