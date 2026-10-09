//! `update` tests: purity. Moved out of `update/tests.rs` (m8/21b-5, move-only).

/// Purity's other half, which no runtime test can observe: that `update`
/// cannot reach the world at all.
///
/// This module is small and self-contained precisely so this check is
/// possible — it reads its own source (every file under `update/` but
/// this one) and fails if anything that talks to the outside (a file, a
/// clock, a thread, a terminal) has been imported or called. A reviewer
/// would have to notice; the test cannot help noticing.
///
/// Walks the directory tree rather than a hand-kept file list, so a new
/// `update/**/*.rs` file is included automatically instead of silently
/// skipped — a file inside a submodule directory (`update/detail/*.rs`) is
/// exactly as much `update`'s code as one directly under `update/`, and must
/// be scanned too (m2-details-and-diffs review M8: `detail.rs` and `mod.rs`
/// split into subdirectories after this test was written).
fn update_sources(dir: &std::path::Path) -> Vec<String> {
    let mut sources = Vec::new();
    let mut dirs = vec![dir.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir).expect("update/ exists") {
            let path = entry.expect("readable dir entry").path();
            if path.is_dir() {
                dirs.push(path);
                continue;
            }
            if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
                continue;
            }
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .expect("utf-8 file name")
                .to_owned();
            if name == "tests.rs" {
                continue;
            }
            let contents = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
            // `mod.rs` names its own test module at the very end; strip that
            // file's tail, the same way a single-file module used to strip
            // its `#[cfg(test)]`. Every other file is pure end to end and is
            // kept whole — stripping used to happen only after joining every
            // file, which threw away everything past `mod.rs`'s own marker.
            let contents = if name == "mod.rs" {
                contents
                    .split_once("#[cfg(test)]")
                    .expect("mod.rs declares the test module")
                    .0
                    .to_owned()
            } else {
                contents
            };
            sources.push(contents);
        }
    }
    sources
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

// Search (m1-the-graph/26).
