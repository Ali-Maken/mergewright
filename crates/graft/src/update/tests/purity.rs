//! `update`'s purity guard (m8/21b-7: moved out of `update/tests.rs`, which
//! it no longer has to stay in — the scan now skips every test module, not
//! only files named `tests.rs`).

use std::path::{Path, PathBuf};

/// A file the scan skips: `tests.rs`, or anything under a `tests/` directory
/// below `src/update/`.
///
/// Sound only with [`every_tests_module_is_cfg_test`]: that makes each such
/// file reachable only through a `#[cfg(test)] mod tests`. It can't see a
/// `#[path]` mod pointing into a `tests/` dir.
fn is_test_module(relative_to_update: &Path) -> bool {
    let mut parts = relative_to_update.components().rev();
    let named_tests = parts
        .next()
        .is_some_and(|file| file.as_os_str() == "tests.rs");
    named_tests || parts.any(|dir| dir.as_os_str() == "tests")
}

/// Every `.rs` file under `dir` the scan reads, whole, in path order;
/// test modules ([`is_test_module`]) left out.
///
/// Walks the directory tree rather than a hand-kept file list, so a new
/// `update/**/*.rs` file is included automatically instead of silently
/// skipped — a file inside a submodule directory (`update/detail/*.rs`) is
/// exactly as much `update`'s code as one directly under `update/`, and must
/// be scanned too (m2-details-and-diffs review M8: `detail.rs` and `mod.rs`
/// split into subdirectories after this test was written).
fn scanned_files(dir: &Path) -> Vec<(PathBuf, String)> {
    let mut files = Vec::new();
    let mut dirs = vec![dir.to_path_buf()];
    while let Some(next) = dirs.pop() {
        for entry in std::fs::read_dir(&next).expect("update/ exists") {
            let path = entry.expect("readable dir entry").path();
            if path.is_dir() {
                dirs.push(path);
                continue;
            }
            if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
                continue;
            }
            let relative = path.strip_prefix(dir).expect("walked from dir");
            if is_test_module(relative) {
                continue;
            }
            let contents = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
            files.push((path, contents));
        }
    }
    files.sort();
    files
}

/// Purity's other half, which no runtime test can observe: that `update`
/// cannot reach the world at all.
///
/// This module is small and self-contained precisely so this check is
/// possible — it reads its own source (every file under `update/` but its
/// test modules) and fails if anything that talks to the outside (a file, a
/// clock, a thread, a terminal) has been imported or called. A reviewer
/// would have to notice; the test cannot help noticing.
fn update_sources(dir: &Path) -> Vec<String> {
    scanned_files(dir)
        .into_iter()
        .map(|(path, contents)| {
            // `mod.rs` names its own test module at the very end; strip that
            // file's tail, the same way a single-file module used to strip
            // its `#[cfg(test)]`. Every other file is pure end to end and is
            // kept whole — stripping used to happen only after joining every
            // file, which threw away everything past `mod.rs`'s own marker.
            if path.file_name().is_some_and(|name| name == "mod.rs") {
                contents
                    .split_once("#[cfg(test)]")
                    .expect("mod.rs declares the test module")
                    .0
                    .to_owned()
            } else {
                contents
            }
        })
        .collect()
}

#[test]
fn update_cannot_reach_the_world() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/update");
    let sources = update_sources(std::path::Path::new(dir));
    assert!(
        sources.len() >= 7,
        "expected every non-test file under update/, found {}: \
         the directory scan is broken",
        sources.len()
    );
    let code = sources.join("\n");

    for forbidden in [
        "std::io",
        "std::fs",
        "std::time",
        "std::thread",
        "std::process",
        "std::net",
        "std::env",
        "SystemTime",
        "Instant",
        "print!",
        "println!",
        "eprint!",
        "eprintln!",
        "crossterm",
        "ratatui",
    ] {
        assert!(
            !code.contains(forbidden),
            "`update` must stay pure, but its module mentions `{forbidden}`"
        );
    }

    // No state shared across clones (m8/18e, review M1): an atomic, or the
    // shell's first-launch switch that holds one, would let `update` change
    // its input state. The empty state's `OutsideState` holds the same plain
    // value, so `empty_state*` is held to it too.
    let src = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
    let mut outside =
        vec![std::fs::read_to_string(format!("{src}/empty_state.rs")).expect("empty_state.rs")];
    outside.extend(update_sources(
        &std::path::Path::new(src).join("empty_state"),
    ));
    for (place, code) in [("update", code), ("empty_state", outside.join("\n"))] {
        for switch in ["atomic", "Atomic", "FirstLaunch"] {
            let held = code.match_indices(switch).any(|(at, _)| {
                // `Command::FirstLaunchDone` is the request, not the switch.
                !code[at + switch.len()..].starts_with(|c: char| c.is_alphanumeric() || c == '_')
            });
            assert!(
                !held,
                "`{place}` holds a shared switch: it mentions `{switch}`"
            );
        }
    }
}

/// Which files the scan skips (m8/21b-7 ruling 1): test modules by module
/// tree, never production by name.
#[test]
fn the_scan_skips_test_modules_only() {
    let table = [
        ("undo.rs", false),
        ("undo/consequence.rs", false),
        ("undo/box_answers.rs", false),
        ("mod.rs", false),
        ("tests.rs", true),
        ("undo/tests.rs", true),
        ("tests/purity.rs", true),
        ("undo/tests/set_aside.rs", true),
        ("tests/keymap_words/stops.rs", true),
    ];
    let wrong: Vec<String> = table
        .iter()
        .filter(|(path, skipped)| is_test_module(Path::new(path)) != *skipped)
        .map(|(path, skipped)| format!("{path} → want {skipped}"))
        .collect();
    assert!(wrong.is_empty(), "the scan's filter is wrong on: {wrong:?}");
}

/// Whether `line` declares a module named `tests` (`;` or `{`), at any
/// visibility.
fn declares_tests(line: &str) -> bool {
    let line = line.trim_start();
    let line = match line.strip_prefix("pub") {
        Some(rest) => match rest.strip_prefix('(') {
            Some(scoped) => scoped.split_once(')').map_or(rest, |(_, after)| after),
            None => rest,
        }
        .trim_start(),
        None => line,
    };
    line.strip_prefix("mod tests")
        .is_some_and(|rest| rest.trim_start().starts_with([';', '{']))
}

/// The other half of [`is_test_module`]'s soundness: every `mod tests` a
/// scanned file declares sits directly under `#[cfg(test)]` (comments and
/// blank lines between are allowed, no other attribute), so every file the
/// scan skips is compiled only into tests.
#[test]
fn every_tests_module_is_cfg_test() {
    let dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src/update"));
    let mut declared = 0;
    for (path, contents) in scanned_files(dir) {
        let lines: Vec<&str> = contents.lines().collect();
        for (at, line) in lines.iter().enumerate() {
            if !declares_tests(line) {
                continue;
            }
            declared += 1;
            let above = lines[..at]
                .iter()
                .rev()
                .map(|line| line.trim())
                .find(|line| !line.is_empty() && !line.starts_with("//"));
            assert_eq!(
                above,
                Some("#[cfg(test)]"),
                "{}:{} declares `mod tests` without `#[cfg(test)]` directly above it: \
                 the purity scan skips test modules, so each must compile only into tests",
                path.strip_prefix(dir).unwrap_or(&path).display(),
                at + 1
            );
        }
    }
    assert!(
        declared >= 20,
        "expected the `mod tests` of update's files, found {declared}: the line match is broken"
    );
}
