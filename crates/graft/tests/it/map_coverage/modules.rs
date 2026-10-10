//! The app's MAP is a map of modules, not of `pub` items: its items are
//! mostly `pub(crate)` and it has no outside callers. For every crate in
//! [`MODULES_CHECKED`]:
//! - every non-test module to depth [`HEADING_DEPTH`] has a `## <path>` heading;
//! - every `## ` heading names exactly one module, once;
//! - every item a MAP line lists is declared in the module under whose
//!   heading it sits, or in a descendant;
//! - the MAP stays within [`MODULE_MAP_MAX_LINES`].
//!
//! Headings match exact paths, so a deleted heading fails even when its
//! module's name recurs elsewhere in the MAP.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::public_set::{brace_delta, inline_body, mod_decl};
use super::{crate_dir, ident_prefix, listed_item, map_text, read, src_tree};

/// Crates checked as maps of modules (the app). Empty in pass 1.
pub(super) const MODULES_CHECKED: &[&str] = &["graft"];
/// Deepest module path that must have its own heading.
const HEADING_DEPTH: usize = 2;
const MODULE_MAP_MAX_LINES: usize = 450;

type Tree = BTreeMap<String, BTreeSet<String>>;

const DECL_KINDS: &[&str] = &[
    "fn", "struct", "enum", "trait", "type", "const", "static", "mod",
];

/// The name an item line declares, at any visibility: `fn`, `struct`, `enum`,
/// `trait`, `type`, `const`, `static`, `mod` or `macro_rules!`.
fn declared_item(line: &str) -> Option<String> {
    let mut rest = line.trim_start();
    if let Some(r) = rest.strip_prefix("pub(") {
        rest = &r[r.find(')')? + 1..];
    } else if let Some(r) = rest.strip_prefix("pub ") {
        rest = r;
    }
    if let Some(r) = rest.trim_start().strip_prefix("macro_rules!") {
        let name = ident_prefix(r.trim_start());
        return (!name.is_empty()).then(|| name.to_owned());
    }
    let mut words = rest.split_whitespace();
    let mut word = words.next()?;
    while matches!(word, "async" | "unsafe" | "extern" | "\"C\"" | "default")
        || (word == "const" && rest.trim_start().starts_with("const fn"))
    {
        word = words.next()?;
    }
    if !DECL_KINDS.contains(&word) {
        return None;
    }
    let name = ident_prefix(words.next()?);
    (!name.is_empty()).then(|| name.to_owned())
}

/// Every module under `src/lib.rs` that isn't `#[cfg(test)]` (nor inside one),
/// any visibility, file or inline, keyed by path from the root (`update::geometry`;
/// the root is `crate`), mapped to the names declared directly in it outside
/// `cfg(test)`: fn (incl. methods in impl blocks), struct, enum, trait, type,
/// const, static, mod, `macro_rules`.
/// Panics naming the file on `#[path = …]` (unsupported; never skipped silently).
fn module_tree(src: &Path) -> BTreeMap<String, BTreeSet<String>> {
    let mut tree = BTreeMap::new();
    let lib = src.join("lib.rs");
    let text = read(&lib);
    let lines: Vec<&str> = text.lines().collect();
    walk_module(&lines, src, "crate", &lib, &mut tree);
    tree
}

/// Walks one module body; `dir` holds its child module files, `file` is the
/// source file it sits in (for the `#[path]` panic).
fn walk_module(lines: &[&str], dir: &Path, path: &str, file: &Path, tree: &mut Tree) {
    tree.entry(path.to_owned()).or_default();
    let mut at = ModWalk::default();
    while let Some(line) = lines.get(at.next) {
        let t = line.trim();
        at.next += 1;
        assert!(
            !t.starts_with("#[path"),
            "`#[path]` is unsupported, in {}",
            file.display()
        );
        if t.starts_with("//") {
            continue;
        }
        if at.depth != 0 {
            inside_block(t, path, &mut at, tree);
        } else if t == "#[cfg(test)]" {
            at.cfg_test = true;
        } else if !t.starts_with("#[") && !t.is_empty() {
            let skip = std::mem::take(&mut at.cfg_test);
            if let Some((child, _, inline)) = mod_decl(t) {
                let child_path = if path == "crate" {
                    child.clone()
                } else {
                    format!("{path}::{child}")
                };
                let child_dir = dir.join(&child);
                if inline {
                    let (body, span) = inline_body(&lines[at.next - 1..]);
                    at.next += span - 1;
                    if !skip {
                        walk_module(&body, &child_dir, &child_path, file, tree);
                    }
                } else if !skip {
                    walk_file(dir, &child, &child_path, tree);
                }
                if !skip {
                    insert(tree, path, child);
                }
            } else {
                top_level_line(t, skip, path, &mut at, tree);
            }
        }
    }
}

/// Where one module body's walk stands.
#[derive(Default)]
struct ModWalk {
    next: usize,
    /// Brace depth inside the body; items live at 0.
    depth: isize,
    /// The last attribute line was `#[cfg(test)]`.
    cfg_test: bool,
    /// Inside an `impl` (at depth 1) not under `cfg(test)`.
    in_impl: bool,
    /// An `impl` header whose `{` has not come yet.
    pending_impl: bool,
}

/// Walks the file module `child` declared in a module whose files live in `dir`.
fn walk_file(dir: &Path, child: &str, child_path: &str, tree: &mut Tree) {
    let child_dir = dir.join(child);
    let child_file = [dir.join(format!("{child}.rs")), child_dir.join("mod.rs")]
        .into_iter()
        .find(|p| p.is_file())
        .unwrap_or_else(|| panic!("no file for `mod {child}` in {}", dir.display()));
    let text = read(&child_file);
    let body: Vec<&str> = text.lines().collect();
    walk_module(&body, &child_dir, child_path, &child_file, tree);
}

/// A line inside a brace block: records `impl` members at depth 1.
fn inside_block(t: &str, path: &str, at: &mut ModWalk, tree: &mut Tree) {
    if at.depth == 1 && at.in_impl {
        if t == "#[cfg(test)]" {
            at.cfg_test = true;
        } else if !t.starts_with("#[")
            && !t.is_empty()
            && !std::mem::take(&mut at.cfg_test)
            && let Some(item) = declared_item(t)
        {
            insert(tree, path, item);
        }
    }
    at.depth += brace_delta(t);
    if at.depth == 0 {
        at.in_impl = false;
    }
}

/// Any other line at depth 0: an item, and maybe an `impl` opening.
fn top_level_line(t: &str, skip: bool, path: &str, at: &mut ModWalk, tree: &mut Tree) {
    if !skip && let Some(item) = declared_item(t) {
        insert(tree, path, item);
    }
    let head = t.trim_start_matches("unsafe ");
    let is_impl = head.starts_with("impl ") || head.starts_with("impl<");
    at.depth += brace_delta(t);
    if at.depth > 0 {
        at.in_impl = !skip && (is_impl || std::mem::take(&mut at.pending_impl));
    } else if is_impl {
        // A multi-line impl header (`where` clauses): its `{` comes later.
        at.pending_impl = !skip;
    } else if t.ends_with(';') || t.ends_with('}') {
        at.pending_impl = false;
    }
}

fn insert(tree: &mut Tree, path: &str, item: String) {
    tree.entry(path.to_owned()).or_default().insert(item);
}

fn tree_of(krate: &str) -> BTreeMap<String, BTreeSet<String>> {
    module_tree(&crate_dir(krate).join("src"))
}

fn depth_of(path: &str) -> usize {
    path.split("::").count()
}

#[test]
fn module_tree_lists_private_and_nested_modules() {
    let dir = src_tree(&[
        ("lib.rs", "mod a;\npub(crate) mod b {\n    mod c {}\n}\n"),
        ("a.rs", "fn f() {}\n"),
    ]);
    let tree = module_tree(&dir.path().join("src"));
    let keys: Vec<&str> = tree.keys().map(String::as_str).collect();
    assert_eq!(keys, ["a", "b", "b::c", "crate"]);
}

#[test]
fn module_tree_skips_cfg_test_modules_and_items() {
    let dir = src_tree(&[(
        "lib.rs",
        "#[cfg(test)]\nmod tests;\nmod x {\n    #[cfg(test)]\n    mod t {}\n    #[cfg(test)]\n    fn only() {}\n    fn kept() {}\n}\n",
    )]);
    let tree = module_tree(&dir.path().join("src"));
    assert!(!tree.contains_key("tests"), "{tree:?}");
    assert!(!tree.contains_key("x::t"), "{tree:?}");
    let x = &tree["x"];
    assert!(!x.contains("only"), "{x:?}");
    assert!(x.contains("kept"), "{x:?}");
}

#[test]
fn module_tree_records_items_per_module() {
    let dir = src_tree(&[(
        "lib.rs",
        "mod a {\n    pub(crate) struct S;\n    impl S {\n        fn go() {}\n    }\n}\n",
    )]);
    let tree = module_tree(&dir.path().join("src"));
    let want: BTreeSet<String> = ["S", "go"].into_iter().map(String::from).collect();
    assert_eq!(tree["a"], want);
}

#[test]
fn every_module_has_its_heading() {
    let mut missing = Vec::new();
    for krate in MODULES_CHECKED {
        let map = map_text(krate);
        let headings: BTreeSet<&str> = map.lines().filter_map(|l| l.strip_prefix("## ")).collect();
        let tree = tree_of(krate);
        let absent: Vec<&str> = tree
            .keys()
            .map(String::as_str)
            .filter(|p| *p != "crate" && depth_of(p) <= HEADING_DEPTH)
            .filter(|p| !headings.contains(p))
            .collect();
        if !absent.is_empty() {
            missing.push(format!(
                "{krate}: {} missing headings: {} (of {} modules)",
                absent.len(),
                absent.join(", "),
                tree.len()
            ));
        }
    }
    assert!(missing.is_empty(), "{}", missing.join("\n"));
}

#[test]
fn every_heading_names_one_module() {
    let mut bad = Vec::new();
    for krate in MODULES_CHECKED {
        let tree = tree_of(krate);
        let mut seen = BTreeSet::new();
        for (n, line) in map_text(krate).lines().enumerate() {
            let Some(h) = line.strip_prefix("## ") else {
                continue;
            };
            if !tree.contains_key(h) {
                bad.push(format!("{krate}/MAP.md:{}: not a module: {h}", n + 1));
            } else if !seen.insert(h.to_owned()) {
                bad.push(format!("{krate}/MAP.md:{}: duplicate: {h}", n + 1));
            }
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn every_listed_item_is_in_its_module() {
    let mut bad = Vec::new();
    for krate in MODULES_CHECKED {
        let tree = tree_of(krate);
        let mut under: Option<&str> = None;
        let map = map_text(krate);
        for (n, line) in map.lines().enumerate() {
            if let Some(h) = line.strip_prefix("## ") {
                under = Some(h);
                continue;
            }
            let (Some(p), Some(item)) = (under, listed_item(line)) else {
                continue;
            };
            let prefix = format!("{p}::");
            let found = tree.iter().any(|(path, items)| {
                (p == "crate" || path == p || path.starts_with(&prefix)) && items.contains(&item)
            });
            if !found {
                bad.push(format!("{krate}/MAP.md:{}: `{item}` not in {p}", n + 1));
            }
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn module_map_fits_its_budget() {
    let mut bad = Vec::new();
    for krate in MODULES_CHECKED {
        let lines = map_text(krate).lines().count();
        if lines > MODULE_MAP_MAX_LINES {
            bad.push(format!(
                "{krate}/MAP.md: {lines} lines > {MODULE_MAP_MAX_LINES}"
            ));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}
