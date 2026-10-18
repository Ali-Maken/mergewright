//! Crate-wide guards on `core` itself (m8/21b-3).

use std::fs;
use std::path::{Path, PathBuf};

/// Every `.rs` file under `dir`, recursively.
fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).expect("a source directory reads") {
        let path = entry.expect("a directory entry reads").path();
        if path.is_dir() {
            files.extend(rust_files(&path));
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
    files
}

/// A public free function's line: `pub` with or without a restriction,
/// any of `const`/`async`/`unsafe`/`extern`, then `fn `, at column 0.
fn is_public_free_fn(line: &str) -> bool {
    let Some(rest) = line.strip_prefix("pub") else {
        return false;
    };
    let rest = match rest.strip_prefix('(') {
        Some(restricted) => match restricted.split_once(')') {
            Some((_, after)) => after,
            None => return false,
        },
        None => rest,
    };
    let mut words = rest.split_whitespace();
    loop {
        match words.next() {
            Some("const" | "async" | "unsafe" | "extern" | "\"C\"") => {}
            Some("fn") => return true,
            _ => return false,
        }
    }
}

/// ADR-0007: `core` holds plain data and exports no free functions —
/// behaviour lives in the crate that owns it. Until m8 only review held
/// this, and a spec broke it once (m8/11 :35).
///
/// What it sees: every line under `src/` that starts at column 0 with
/// `pub` (any restriction) and declares a `fn`. Code inside an `impl` or a
/// `mod { … }` block is indented, so it is not a top-level fn.
///
/// What it cannot see: a method that carries logic (an `impl` is how data
/// gets its constructors and accessors, so methods are allowed and judged
/// by review); a free fn inside an inline `mod` block, which is indented;
/// a fn written by a macro.
#[test]
fn core_has_no_free_functions() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut found = Vec::new();
    for file in rust_files(&src) {
        let text = fs::read_to_string(&file).expect("a source file reads");
        for (number, line) in text.lines().enumerate() {
            if is_public_free_fn(line) {
                found.push(format!("{}:{}: {line}", file.display(), number + 1));
            }
        }
    }
    assert!(
        found.is_empty(),
        "mergewright-core exports a free fn (ADR-0007):\n{}",
        found.join("\n")
    );
}

#[test]
fn the_scan_reads_each_form_of_a_public_fn() {
    for line in [
        "pub fn x() {}",
        "pub(crate) fn x() {}",
        "pub(super) const fn x() {}",
        "pub(in crate::journal) fn x() {}",
        "pub async fn x() {}",
        "pub unsafe extern \"C\" fn x() {}",
    ] {
        assert!(is_public_free_fn(line), "{line}");
    }
    for line in [
        "    pub fn method(&self) {}",
        "pub struct Fn;",
        "pub const FN: u8 = 0;",
        "fn private() {}",
        "pub type Fnord = u8;",
    ] {
        assert!(!is_public_free_fn(line), "{line}");
    }
}
