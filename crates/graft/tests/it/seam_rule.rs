//! Guards the seam rule in ARCHITECTURE.md: `ops` decides, `app` wires.
//!
//! `app` never builds a `GitInvocation` for a write, never touches the
//! journal file and never chooses a snapshot; a key becomes an `Operation`
//! and `ops` does the rest. Cargo's dependency edges cannot see this — `app`
//! legitimately depends on `git` for reads — so this test reads the source:
//! `GitInvocation` may appear under `crates/graft/src/` only inside `jobs/`
//! (the read jobs), and the journal's line renderers and the inverse's step
//! constructors may not appear anywhere in `app`. A builder wiring a key
//! that reaches for one gets this failure instead of a review finding.

use std::fs;
use std::path::{Path, PathBuf};

/// The identifiers `app` may not name, and where each is still allowed.
const FORBIDDEN: &[(&str, Allowed)] = &[
    ("GitInvocation", Allowed::Under("jobs")),
    ("render_pending", Allowed::Nowhere),
    ("render_outcome", Allowed::Nowhere),
    ("PathScope::", Allowed::Nowhere),
    ("Step::UpdateRef", Allowed::Nowhere),
    ("Step::SetHead", Allowed::Nowhere),
    ("Step::RestoreIndex", Allowed::Nowhere),
    ("Step::RestoreWorktree", Allowed::Nowhere),
    ("Step::StashStore", Allowed::Nowhere),
    ("Step::StashDrop", Allowed::Nowhere),
    // m6/03: writing a resolved conflict back to disk is an operation, not a
    // thing `app` does. `ops` holds the blob-id lease that makes the write
    // refuse a file that changed underneath it; a direct call from `app` would
    // skip the lease and clobber.
    ("write_back", Allowed::Nowhere),
];

#[derive(Clone, Copy)]
enum Allowed {
    /// Only in files under this directory of `crates/graft/src`.
    Under(&'static str),
    /// In no file of `crates/graft/src`.
    Nowhere,
}

#[test]
fn app_never_builds_a_write_invocation_or_touches_the_journal() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let offences = offences_under(&src);
    assert!(
        offences.is_empty(),
        "the seam rule (ARCHITECTURE.md, \"Changing what a write operation does\") is broken:\n{}\n\
         `app` wires a key to an `Operation`; `ops` plans, journals and restores.",
        offences.join("\n")
    );
}

#[test]
fn the_guard_sees_an_offence_and_the_allowed_place() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src");
    fs::create_dir_all(src.join("update")).unwrap();
    fs::create_dir_all(src.join("jobs")).unwrap();
    fs::write(
        src.join("jobs/status.rs"),
        "let inv = GitInvocation::new();\n",
    )
    .unwrap();
    assert!(offences_under(&src).is_empty());

    fs::write(
        src.join("update/keys.rs"),
        "fn f() {\n    let inv = GitInvocation::new();\n    render_pending(&e);\n}\n",
    )
    .unwrap();
    let offences = offences_under(&src);
    assert_eq!(offences.len(), 2, "{offences:?}");
    assert!(offences[0].contains("update/keys.rs:2: GitInvocation"));
    assert!(offences[1].contains("update/keys.rs:3: render_pending"));
}

/// `<file>:<line>: <identifier>` for every forbidden identifier in a `.rs`
/// file under `src`, outside the place it is allowed.
fn offences_under(src: &Path) -> Vec<String> {
    let mut offences = Vec::new();
    for file in rust_files(src) {
        let relative = file.strip_prefix(src).unwrap_or(&file).to_path_buf();
        let text = match fs::read_to_string(&file) {
            Ok(text) => text,
            Err(error) => panic!("could not read {}: {error}", file.display()),
        };
        for (needle, allowed) in FORBIDDEN {
            if let Allowed::Under(dir) = allowed
                && relative.starts_with(dir)
            {
                continue;
            }
            for (index, line) in text.lines().enumerate() {
                if line.contains(needle) {
                    offences.push(format!("{}:{}: {needle}", relative.display(), index + 1));
                }
            }
        }
    }
    offences.sort();
    offences
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
