//! Guards the seam rule in ARCHITECTURE.md: `ops` decides, `app` wires.
//!
//! `app` never builds a `GitInvocation` for a write, never touches the
//! journal file and never chooses a snapshot; a key becomes an `Operation`
//! and `ops` does the rest. Cargo's dependency edges cannot see this — `app`
//! legitimately depends on `git` for reads — so this test reads the source:
//! `GitInvocation` may appear under `crates/mergewright/src/` only inside `jobs/`
//! (the read jobs), and the journal's line renderers and the inverse's step
//! constructors may not appear anywhere in `app`. A builder wiring a key
//! that reaches for one gets this failure instead of a review finding.
//!
//! `ops` narrows what it can to `pub(crate)`, and the compiler keeps `app`
//! off those. A few `ops` internals stay `pub` because an outside caller
//! needs them: `examples/snapshot_cost.rs` times `snapshot::take` and
//! `raw_record`, the app crate's own tests seed journals with
//! `append_pending`, and `restore::execute` and the
//! journal's line renderers are driven by `ops` integration tests that
//! cannot move in-crate (m8 ticket 39). The renderers are in [`FORBIDDEN`];
//! the rest each have their own clause ([`KEPT_PUB`]), forbidden in `app`'s
//! `src/` whether named by path or imported by `use` and called bare.

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

/// An `ops` internal kept `pub` for an outside caller but forbidden in
/// `app`: its name, and the module it lives in when the bare name is too
/// common to forbid on its own (`take`, `execute`).
#[derive(Clone, Copy)]
struct KeptPub {
    module: Option<&'static str>,
    name: &'static str,
}

const TAKE: KeptPub = KeptPub {
    module: Some("snapshot"),
    name: "take",
};
const RAW_RECORD: KeptPub = KeptPub {
    module: None,
    name: "raw_record",
};
const APPEND_PENDING: KeptPub = KeptPub {
    module: None,
    name: "append_pending",
};
const RESTORE_EXECUTE: KeptPub = KeptPub {
    module: Some("restore"),
    name: "execute",
};

/// Every kept-`pub` clause, for the guard's own test.
const KEPT_PUB: &[KeptPub] = &[TAKE, RAW_RECORD, APPEND_PENDING, RESTORE_EXECUTE];

#[derive(Clone, Copy)]
enum Allowed {
    /// Only in files under this directory of `crates/mergewright/src`.
    Under(&'static str),
    /// In no file of `crates/mergewright/src`.
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

/// `snapshot::take` chooses a snapshot; `ops` takes one inside every write
/// operation, `app` never does.
#[test]
fn app_never_takes_a_snapshot() {
    assert_app_never_names(TAKE);
}

/// `raw_record` builds the before-snapshot's raw tree: `ops`' own step.
#[test]
fn app_never_records_a_raw_snapshot() {
    assert_app_never_names(RAW_RECORD);
}

/// The journal's pending line is written by `ops` before a write runs.
#[test]
fn app_never_appends_a_pending_journal_line() {
    assert_app_never_names(APPEND_PENDING);
}

/// An undo reaches `restore::execute` through `ops`' undo entry point, never
/// by `app` calling it.
#[test]
fn app_never_runs_restore_execute() {
    assert_app_never_names(RESTORE_EXECUTE);
}

#[test]
fn the_kept_pub_guard_sees_a_path_and_an_import_and_skips_comments() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src");
    fs::create_dir_all(src.join("jobs")).unwrap();
    fs::write(
        src.join("jobs/quiet.rs"),
        "// mergewright_ops::restore::execute is ops' own\n\
         use mergewright_ops::restore::RestoreError;\n\
         let n = it.take(3);\n\
         let v = std::mem::take(&mut x);\n",
    )
    .unwrap();
    for item in KEPT_PUB {
        assert!(kept_pub_offences(&src, *item).is_empty());
    }

    fs::write(
        src.join("jobs/loud.rs"),
        "use mergewright_ops::snapshot::{\n    Snapshot,\n    take,\n};\n\
         fn f() {\n    take(&ctx, &id);\n    mergewright_ops::restore::execute(&plan);\n\
         \x20   journal.append_pending(&e); // a comment\n}\n",
    )
    .unwrap();
    assert_eq!(
        kept_pub_offences(&src, TAKE),
        vec!["jobs/loud.rs:1: snapshot::take (imported)".to_string()]
    );
    assert_eq!(
        kept_pub_offences(&src, RESTORE_EXECUTE),
        vec!["jobs/loud.rs:7: restore::execute".to_string()]
    );
    assert_eq!(
        kept_pub_offences(&src, APPEND_PENDING),
        vec!["jobs/loud.rs:8: append_pending".to_string()]
    );
    assert!(kept_pub_offences(&src, RAW_RECORD).is_empty());
}

fn assert_app_never_names(item: KeptPub) {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let offences = kept_pub_offences(&src, item);
    assert!(
        offences.is_empty(),
        "the seam rule (ARCHITECTURE.md, \"Changing what a write operation does\") is broken:\n{}\n\
         this `ops` item is `pub` only for an outside caller (an example or a test); \
         `app` reaches it through an `Operation`.",
        offences.join("\n")
    );
}

/// `<file>:<line>: <what>` for every place under `src` whose code (comments
/// skipped) names `item`: by its path (`restore::execute`), or bare when the
/// item has no module; and every `use` that imports it from its module,
/// reported at the `use`'s first line.
fn kept_pub_offences(src: &Path, item: KeptPub) -> Vec<String> {
    let needle = match item.module {
        Some(module) => format!("{module}::{}", item.name),
        None => item.name.to_string(),
    };
    let mut offences = Vec::new();
    for file in rust_files(src) {
        let relative = file.strip_prefix(src).unwrap_or(&file).to_path_buf();
        let text = match fs::read_to_string(&file) {
            Ok(text) => text,
            Err(error) => panic!("could not read {}: {error}", file.display()),
        };
        // The `use` statement being read, from its first line to its `;`.
        let mut open_use: Option<(usize, String)> = None;
        for (index, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or(line);
            if open_use.is_none() && is_use(code) {
                open_use = Some((index, String::new()));
            }
            if let Some((start, statement)) = &mut open_use {
                statement.push_str(code);
                statement.push(' ');
                if code.contains(';') {
                    let imported = match item.module {
                        Some(module) => {
                            statement.contains(&format!("{module}::"))
                                && has_word(statement, item.name)
                        }
                        None => has_word(statement, item.name),
                    };
                    if imported {
                        offences.push(format!(
                            "{}:{}: {needle} (imported)",
                            relative.display(),
                            *start + 1
                        ));
                    }
                    open_use = None;
                }
                continue;
            }
            if has_word(code, &needle) {
                offences.push(format!("{}:{}: {needle}", relative.display(), index + 1));
            }
        }
    }
    offences.sort();
    offences
}

fn is_use(code: &str) -> bool {
    let code = code.trim_start();
    ["use ", "pub use ", "pub(crate) use ", "pub(super) use "]
        .iter()
        .any(|start| code.starts_with(start))
}

/// Whether `word` appears in `text` with no identifier character either side.
fn has_word(text: &str, word: &str) -> bool {
    let ident = |c: char| c.is_alphanumeric() || c == '_';
    text.match_indices(word).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let after = text[at + word.len()..].chars().next();
        !before.is_some_and(ident) && !after.is_some_and(ident)
    })
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
