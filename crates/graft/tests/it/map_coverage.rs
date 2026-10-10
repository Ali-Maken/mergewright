//! Guards each crate's `MAP.md` as a map, not a change log.
//!
//! A builder reads MAP.md before source, so it has to be short, current and
//! complete. For every crate in [`CHECKED`]:
//! - every item in the crate's public set (`public_items`: what a caller
//!   outside the crate can name) is named in the MAP;
//! - every MAP line is at most 240 characters and carries no ticket id;
//! - every item the MAP lists is in the public set, so stale and
//!   crate-private entries fail.
//!
//! Later MAP tickets add their crates to [`CHECKED`].

mod public_set;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use public_set::public_items;

/// The crates whose MAP.md this guard holds to the format.
const CHECKED: &[&str] = &[
    "graft-core",
    "graft-term",
    "graft-graph",
    "graft-git",
    "graft-fixtures",
    "graft-capabilities",
    "graft-ops",
    "graft-ui",
];

const MAX_LINE: usize = 240;

const ITEM_KINDS: &[&str] = &[
    "fn", "struct", "enum", "trait", "type", "const", "static", "mod",
];

fn crate_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(name)
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

fn ident_prefix(s: &str) -> &str {
    let end = s
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(s.len());
    &s[..end]
}

/// `pub <kind> name` on this line (qualifiers like `const fn` allowed),
/// excluding `pub(…)`.
fn pub_item(line: &str) -> Option<String> {
    let rest = line.trim_start().strip_prefix("pub ")?;
    let mut words = rest.split_whitespace();
    let mut word = words.next()?;
    while matches!(word, "async" | "unsafe" | "extern" | "\"C\"")
        || (word == "const" && rest.contains("const fn"))
    {
        word = words.next()?;
    }
    if !ITEM_KINDS.contains(&word) {
        return None;
    }
    let name = ident_prefix(words.next()?);
    (!name.is_empty()).then(|| name.to_owned())
}

fn map_text(krate: &str) -> String {
    read(&crate_dir(krate).join("MAP.md"))
}

/// Every identifier in a module heading or inside backticks on an item line.
fn backticked_idents(map: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    // A module is named by its `## module` (or `### a/b`) heading.
    for line in map.lines().filter(|l| l.starts_with("##")) {
        for word in line.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
            if !word.is_empty() {
                out.insert(word.to_owned());
            }
        }
    }
    for line in map.lines().filter(|l| l.trim_start().starts_with("- ")) {
        for (i, span) in line.split('`').enumerate() {
            if i % 2 == 1 {
                for word in span.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
                    if !word.is_empty() {
                        out.insert(word.to_owned());
                    }
                }
            }
        }
    }
    out
}

/// The item a MAP line names: the last path segment of its first backtick
/// span, e.g. `Commit::date` → `date`.
fn listed_item(line: &str) -> Option<String> {
    let rest = line.trim_start().strip_prefix("- `")?;
    let span = &rest[..rest.find('`')?];
    let last = span.rsplit("::").next()?;
    let name = ident_prefix(last.trim_start_matches(['&', '*']));
    (!name.is_empty()).then(|| name.to_owned())
}

#[test]
fn every_pub_item_is_in_its_map() {
    let mut missing = Vec::new();
    for krate in CHECKED {
        let named = backticked_idents(&map_text(krate));
        let absent: Vec<String> = public_items(&crate_dir(krate).join("src"))
            .into_iter()
            .filter(|item| !named.contains(item))
            .map(|item| format!("`{item}`"))
            .collect();
        if !absent.is_empty() {
            missing.push(format!(
                "{krate}: {} missing: {}",
                absent.len(),
                absent.join(", ")
            ));
        }
    }
    assert!(
        missing.is_empty(),
        "pub items missing from their crate's MAP.md:\n{}",
        missing.join("\n")
    );
}

/// `m<digits>` then optional `-word` parts, then `/<digits>…` — a ticket id.
fn has_ticket_id(line: &str) -> bool {
    let b = line.as_bytes();
    for start in 0..b.len() {
        if b[start] != b'm'
            || (start > 0 && (b[start - 1].is_ascii_alphanumeric() || b[start - 1] == b'_'))
        {
            continue;
        }
        let mut i = start + 1;
        let digits = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if i == digits {
            continue;
        }
        while i < b.len() && b[i] == b'-' {
            let part = i + 1;
            let mut j = part;
            while j < b.len() && (b[j].is_ascii_lowercase() || b[j].is_ascii_digit()) {
                j += 1;
            }
            if j == part {
                break;
            }
            i = j;
        }
        if i + 1 < b.len() && b[i] == b'/' && b[i + 1].is_ascii_digit() {
            return true;
        }
    }
    false
}

#[test]
fn map_lines_are_short_and_carry_no_ticket_ids() {
    let mut bad = Vec::new();
    for krate in CHECKED {
        for (n, line) in map_text(krate).lines().enumerate() {
            let heading = line.starts_with('#');
            if !heading && line.chars().count() > MAX_LINE {
                bad.push(format!("{krate}/MAP.md:{}: longer than {MAX_LINE}", n + 1));
            }
            if has_ticket_id(line) {
                bad.push(format!("{krate}/MAP.md:{}: ticket id: {line}", n + 1));
            }
        }
    }
    assert!(bad.is_empty(), "MAP.md format broken:\n{}", bad.join("\n"));
}

#[test]
fn every_map_item_still_exists() {
    let mut stale = Vec::new();
    for krate in CHECKED {
        let known = public_items(&crate_dir(krate).join("src"));
        for (n, line) in map_text(krate).lines().enumerate() {
            if let Some(item) = listed_item(line)
                && !known.contains(&item)
            {
                stale.push(format!("{krate}/MAP.md:{}: `{item}`", n + 1));
            }
        }
    }
    assert!(
        stale.is_empty(),
        "MAP.md lists items outside the crate's public set:\n{}",
        stale.join("\n")
    );
}

#[test]
fn ticket_id_pattern_matches_ids_and_nothing_else() {
    assert!(has_ticket_id("see m8-polish-and-ship/25"));
    assert!(has_ticket_id("m6/03 rule"));
    assert!(!has_ticket_id("a 1m/2 ratio"));
    assert!(!has_ticket_id("item/3 plain"));
}

/// Writes `files` (path relative to `src/`, contents) into a temp `src/`.
fn src_tree(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("a temp dir: {e}"));
    for (path, text) in files {
        let path = dir.path().join("src").join(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .unwrap_or_else(|e| panic!("creating {}: {e}", parent.display()));
        }
        fs::write(&path, text).unwrap_or_else(|e| panic!("writing {}: {e}", path.display()));
    }
    dir
}

#[test]
fn public_set_skips_private_module_items() {
    let dir = src_tree(&[
        ("lib.rs", "mod hidden;\npub mod shown;\n"),
        ("hidden.rs", "pub struct H;\n"),
        ("shown.rs", "pub struct S;\n"),
    ]);
    let set = public_items(&dir.path().join("src"));
    assert!(set.contains("S"), "{set:?}");
    assert!(!set.contains("H"), "{set:?}");
}

#[test]
fn public_set_keeps_reexports_and_their_methods() {
    let dir = src_tree(&[
        (
            "lib.rs",
            "mod hidden;\npub use hidden::H;\npub use hidden::K as Renamed;\n",
        ),
        (
            "hidden.rs",
            "pub struct H;\npub struct K;\nimpl H {\n    pub fn go() {}\n}\n",
        ),
    ]);
    let set = public_items(&dir.path().join("src"));
    for name in ["H", "go", "Renamed"] {
        assert!(set.contains(name), "{name} missing: {set:?}");
    }
    assert!(!set.contains("K"), "{set:?}");
}

#[test]
fn public_set_skips_cfg_test_items() {
    let dir = src_tree(&[
        ("lib.rs", "pub mod shown;\n"),
        (
            "shown.rs",
            "pub fn kept() {}\n#[cfg(test)]\npub fn only_tests() {}\n",
        ),
    ]);
    let set = public_items(&dir.path().join("src"));
    assert!(set.contains("kept"), "{set:?}");
    assert!(!set.contains("only_tests"), "{set:?}");
}
