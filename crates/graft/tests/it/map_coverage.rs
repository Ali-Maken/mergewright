//! Guards each crate's `MAP.md` as a map, not a change log.
//!
//! A builder reads MAP.md before source, so it has to be short, current and
//! complete. For every crate in [`CHECKED`]:
//! - every `pub` item declared under `src/` (outside test modules) is named
//!   in the MAP;
//! - every MAP line is at most 240 characters and carries no ticket id;
//! - every item the MAP lists still resolves to a declaration, so stale
//!   entries fail.
//!
//! Later MAP tickets add their crates to [`CHECKED`].

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// The crates whose MAP.md this guard holds to the format.
const CHECKED: &[&str] = &[
    "graft-core",
    "graft-term",
    "graft-graph",
    "graft-git",
    "graft-fixtures",
    "graft-capabilities",
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

/// Names declared as `#[cfg(test)] mod name;` by any file in `dir`.
fn test_only_mods(dir: &Path) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("listing {}: {e}", dir.display()));
    for entry in entries {
        let path = entry
            .unwrap_or_else(|e| panic!("listing {}: {e}", dir.display()))
            .path();
        if path.extension().is_none_or(|x| x != "rs") {
            continue;
        }
        let src = read(&path);
        let mut after_cfg_test = false;
        for line in src.lines().map(str::trim) {
            if line == "#[cfg(test)]" {
                after_cfg_test = true;
                continue;
            }
            if after_cfg_test
                && let Some(rest) = line.strip_prefix("mod ")
                && rest.ends_with(';')
            {
                out.insert(ident_prefix(rest).to_owned());
            }
            if !line.starts_with("#[") {
                after_cfg_test = false;
            }
        }
    }
    out
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let test_mods = test_only_mods(dir);
    let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("listing {}: {e}", dir.display()));
    for entry in entries {
        let path = entry
            .unwrap_or_else(|e| panic!("listing {}: {e}", dir.display()))
            .path();
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        // `#[cfg(test)] mod x;` files and folders (and any named `tests`)
        // are test scaffolding, not the crate's surface.
        if stem == "tests" || test_mods.contains(stem) {
            continue;
        }
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|x| x == "rs") {
            out.push(path);
        }
    }
}

/// Source lines with every `#[cfg(test)]` inline module removed.
fn non_test_lines(src: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut lines = src.lines();
    let mut pending_cfg_test = false;
    while let Some(line) = lines.next() {
        let t = line.trim();
        if t == "#[cfg(test)]" {
            pending_cfg_test = true;
            continue;
        }
        if pending_cfg_test && t.starts_with("mod ") && t.ends_with('{') {
            let (mut opens, mut closes) = (1usize, 0usize);
            for inner in lines.by_ref() {
                opens += inner.matches('{').count();
                closes += inner.matches('}').count();
                if closes >= opens {
                    break;
                }
            }
            pending_cfg_test = false;
            continue;
        }
        if !t.starts_with("#[") {
            pending_cfg_test = false;
        }
        out.push(line);
    }
    out
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

/// What a MAP may name besides `pub` items: pub fields, trait methods and
/// enum variants, i.e. any identifier declared in the source.
fn declared(line: &str) -> Vec<String> {
    let t = line.trim_start();
    let t = t.strip_prefix("pub ").unwrap_or(t);
    let mut out = Vec::new();
    if let Some(rest) = t.strip_prefix("fn ") {
        out.push(ident_prefix(rest).to_owned());
    }
    let head = ident_prefix(t);
    if !head.is_empty() {
        out.push(head.to_owned());
    }
    out
}

struct Surface {
    pub_items: BTreeSet<String>,
    declared: BTreeSet<String>,
}

fn surface(krate: &str) -> Surface {
    let mut files = Vec::new();
    rust_files(&crate_dir(krate).join("src"), &mut files);
    let mut pub_items = BTreeSet::new();
    let mut declared_names = BTreeSet::new();
    for file in files {
        let src = read(&file);
        for line in non_test_lines(&src) {
            if let Some(name) = pub_item(line) {
                pub_items.insert(name.clone());
                declared_names.insert(name);
            }
            declared_names.extend(declared(line));
        }
    }
    Surface {
        pub_items,
        declared: declared_names,
    }
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
        for item in surface(krate).pub_items {
            if !named.contains(&item) {
                missing.push(format!("{krate}: `{item}`"));
            }
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
        let known = surface(krate).declared;
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
        "MAP.md lists items no longer declared:\n{}",
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
