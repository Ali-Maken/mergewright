//! The package and the command follow the names table (m8-polish-and-ship/27a,
//! 49): the binary's file name, no old name left in a string the user can
//! read, no script running an earlier generation's binary, and no live file
//! naming the old package.

#![allow(clippy::expect_used, reason = "a failing test should panic")]

use std::fs;
use std::path::{Path, PathBuf};

use graft_core::on_disk::Generation;

use crate::rename_opens::{crates_dir, production_files};

/// The package is the current generation's dir name and the binary its
/// command: `mergewright` and `mwr` (49).
#[test]
fn package_and_bin_follow_the_table() {
    assert_eq!(env!("CARGO_PKG_NAME"), Generation::CURRENT.dir());
    let binary = Path::new(env!("CARGO_BIN_EXE_mwr"));
    let name = binary.file_stem().and_then(|name| name.to_str());
    assert_eq!(name, Some(graft::cli::COMMAND));
    assert_eq!(graft::cli::COMMAND, Generation::CURRENT.command());
}

/// The binary Cargo builds is named after the command every message uses.
#[test]
fn the_binary_file_is_the_command() {
    let binary = Path::new(env!("CARGO_BIN_EXE_mwr"));
    let name = binary
        .file_stem()
        .and_then(|name| name.to_str())
        .expect("a file name");
    assert_eq!(name, graft::cli::COMMAND);
}

/// Where a string literal in production code may still say `graft`, and why:
/// `(file or folder prefix, text the literal contains, reason)`. An empty
/// text allows every literal under the prefix. None of these is drawn on
/// screen.
const ALLOWED: &[(&str, &str, &str)] = &[
    (
        "graft-core/src/on_disk.rs",
        "",
        "the `Generation` table: every generation's names (26, 48)",
    ),
    (
        "graft-fixtures/",
        "",
        "test-fixture crate: identities, temp prefixes, its usage",
    ),
    (
        "graft-capabilities/",
        "",
        "the CAPABILITIES generator's prose: docs words, 27c",
    ),
    // Thread names: in a backtrace, never on screen; code names.
    (
        "graft/src/event_loop/input.rs",
        "graft-input-forward",
        "thread name",
    ),
    (
        "graft/src/event_loop/redetect.rs",
        "graft-redetect",
        "thread name",
    ),
    ("graft/src/event_loop/signals.rs", "graft-", "thread names"),
    ("graft/src/jobs/runner/mod.rs", "graft-job-", "thread name"),
    ("graft-term/src/input/mux.rs", "graft-input", "thread name"),
    // Temporary file, socket and marker names: code names, gone after use.
    (
        "graft-git/src/conflict/disk.rs",
        "graft-",
        "temp file prefix",
    ),
    (
        "graft-git/src/run/askpass.rs",
        "graft-askpass",
        "socket and helper temp names",
    ),
    (
        "graft-git/src/trace/function.rs",
        "graft-marker-",
        "internal trace marker",
    ),
    (
        "graft-git/src/fixture.rs",
        "Graft Fixture",
        "fixture author identity",
    ),
    (
        "graft-ops/src/merge_state.rs",
        ".graft-tmp",
        "temp file name",
    ),
    (
        "graft-ops/src/preview/ghost.rs",
        "graft-ghost",
        "scratch marker",
    ),
    (
        "graft-ops/src/rewrite/todo.rs",
        "graft-fork-",
        "temp dir prefix",
    ),
    (
        "graft-ops/src/rewrite/saved.rs",
        "Graft's message helper",
        "generated helper script: its comment and temp file name",
    ),
    // The journal's source column is 14 cells, sized for `outside Graft`;
    // `outside Tributary` is cut. Needs a word or width decision (27a NOTES).
    (
        "graft-ui/src/journal_screen/pieces.rs",
        "Graft",
        "journal source column width",
    ),
    (
        "graft-ui/src/journal_screen/legend.rs",
        "done in Graft: {can}",
        "legend names that column",
    ),
    // Messages written into git (stash and backup commits): left until a
    // reader audit renames them with their readers (27a NOTES).
    (
        "graft-ops/src/apply/set_aside.rs",
        "graft: your edits",
        "stash message in git",
    ),
    (
        "graft-ops/src/apply/set_aside/rebuild.rs",
        "graft: ",
        "commit messages in git",
    ),
    (
        "graft-ops/src/rewrite/absorb.rs",
        "graft: absorb",
        "commit message in git",
    ),
    (
        "graft-ops/src/snapshot/commit.rs",
        "graft",
        "backup commit messages in git",
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

/// `graft` as a whole word, any case: not `graft_core`, not `TRIB_X`,
/// not `graft-ui`'s path segment when joined by `_`.
fn says_old_name(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let word = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'_';
    lower.match_indices("graft").any(|(at, _)| {
        let before = at.checked_sub(1).map(|i| bytes[i]);
        let after = bytes.get(at + 5).copied();
        !before.is_some_and(word) && !after.is_some_and(word)
    })
}

/// No string literal in any crate's production source says `graft` but the
/// allowed files: the user reads `mwr` or Tributary everywhere.
#[test]
fn no_old_name_in_user_visible_strings() {
    let crates = crates_dir();
    let mut files: Vec<PathBuf> = Vec::new();
    let mut dirs: Vec<_> = fs::read_dir(&crates)
        .expect("crates/ reads")
        .map(|entry| entry.expect("a dir entry").path())
        .collect();
    dirs.sort();
    for krate in dirs {
        let src = krate.join("src");
        if src.is_dir() {
            production_files(&src, &mut files);
        }
    }
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
            let path_like = literal.contains("crates/graft") || literal.starts_with("../");
            if says_old_name(&literal) && !path_like {
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
fn the_word_test_reads_words_only() {
    assert!(says_old_name("graft crashed"));
    assert!(says_old_name("usage: Graft"));
    assert!(!says_old_name("graft_core"));
    assert!(!says_old_name("TRIB_PERF_TRACE"));
    assert!(!says_old_name("grafted"));
}

/// README.md and `scripts/**` name the command and the app, not the old
/// name (27c); what may still say `graft`, and why, is in `rename_docs`.
#[test]
fn user_docs_and_scripts_name_the_command() {
    let found = crate::rename_docs::old_name_lines();
    assert!(found.is_empty(), "the old name:\n{}", found.join("\n"));
}

mod scripts;
