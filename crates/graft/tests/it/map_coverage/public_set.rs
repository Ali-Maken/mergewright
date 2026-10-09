//! The public set: what a caller outside a crate can name, computed from
//! its `src/` (ticket 37 § Addition, ruling).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::{ident_prefix, pub_item, read};

/// What one walk of a crate's `src/` collects on the way to its public set.
#[derive(Default)]
struct Walk {
    /// Names a caller outside the crate can name, found so far.
    public: BTreeSet<String>,
    /// Every non-test `pub` item, by the name of the module declaring it:
    /// what a `pub use m::*` makes public.
    by_module: BTreeMap<String, BTreeSet<String>>,
    /// Module names a public module glob-re-exports.
    globs: Vec<String>,
    /// `alias → original` for each `pub use … as alias` in a public module.
    renamed: BTreeMap<String, String>,
    /// `(type, item)` for each `pub fn`/`pub const` in an inherent impl.
    methods: Vec<(String, String)>,
}

/// The public set of the crate whose `src/` is `src`: every item a caller
/// outside the crate can name (ticket 37 § Addition, ruling).
/// 1. `pub` items in modules reached from `lib.rs` through `pub mod` only;
/// 2. names a public module `pub use`s (an `as X` adds `X`; `m::*` adds
///    `m`'s `pub` items);
/// 3. `pub fn`/`pub const` in an inherent `impl T` whose `T` is public;
/// 4. never anything under `#[cfg(test)]`, on the item or a module above it.
pub(super) fn public_items(src: &Path) -> BTreeSet<String> {
    let mut walk = Walk::default();
    let lib = read(&src.join("lib.rs"));
    let lines: Vec<&str> = lib.lines().collect();
    scan_module(&lines, src, true, "lib", &mut walk);
    let Walk {
        mut public,
        by_module,
        globs,
        renamed,
        methods,
    } = walk;
    for module in globs {
        public.extend(by_module.get(&module).into_iter().flatten().cloned());
    }
    let mut types = public.clone();
    types.extend(renamed.into_values());
    for (ty, item) in methods {
        if types.contains(&ty) {
            public.insert(item);
        }
    }
    public
}

/// `[pub[(…)]] mod name` → `(name, is plain pub, has an inline body)`.
fn mod_decl(line: &str) -> Option<(String, bool, bool)> {
    let (is_pub, rest) = if let Some(rest) = line.strip_prefix("pub ") {
        (true, rest)
    } else if line.starts_with("pub(") {
        (false, &line[line.find(')')? + 1..])
    } else {
        (false, line)
    };
    let rest = rest.trim_start().strip_prefix("mod ")?;
    let name = ident_prefix(rest);
    (!name.is_empty()).then(|| (name.to_owned(), is_pub, !rest.trim_end().ends_with(';')))
}

/// The type an inherent `impl` line implements on; `None` for a trait impl.
fn impl_target(line: &str) -> Option<String> {
    let mut rest = line.strip_prefix("impl")?;
    if rest.starts_with('<') {
        let mut depth = 0usize;
        let end = rest.char_indices().find(|&(_, c)| {
            match c {
                '<' => depth += 1,
                '>' => depth -= 1,
                _ => {}
            }
            depth == 0
        })?;
        rest = &rest[end.0 + 1..];
    } else if !rest.starts_with(' ') {
        return None;
    }
    if rest.contains(" for ") {
        return None;
    }
    let path = rest.trim_start();
    let path = &path[..path.find(['<', ' ', '{']).unwrap_or(path.len())];
    let name = path.rsplit("::").next()?;
    (!name.is_empty()).then(|| name.to_owned())
}

/// Lines of the brace-delimited body opening on `lines[0]`, and how many
/// lines (including both braces' lines) it spans.
fn inline_body<'a>(lines: &[&'a str]) -> (Vec<&'a str>, usize) {
    let mut depth = 0isize;
    for (i, line) in lines.iter().enumerate() {
        depth += brace_delta(line);
        if depth <= 0 {
            return (lines[1.min(i)..i].to_vec(), i + 1);
        }
    }
    (lines[1..].to_vec(), lines.len())
}

fn brace_delta(line: &str) -> isize {
    let code = line.split("//").next().unwrap_or("");
    code.chars()
        .map(|c| match c {
            '{' => 1,
            '}' => -1,
            _ => 0,
        })
        .sum()
}

/// Adds a `pub use` tree's names to the walk.
fn scan_use(tree: &str, walk: &mut Walk) {
    let tree = tree.trim();
    if let Some(open) = tree.find('{') {
        let prefix = tree[..open].trim_end_matches("::");
        let inner = &tree[open + 1..tree.rfind('}').unwrap_or(tree.len())];
        let (mut depth, mut start) = (0usize, 0usize);
        let mut parts = Vec::new();
        for (i, c) in inner.char_indices() {
            match c {
                '{' => depth += 1,
                '}' => depth -= 1,
                ',' if depth == 0 => {
                    parts.push(&inner[start..i]);
                    start = i + 1;
                }
                _ => {}
            }
        }
        parts.push(&inner[start..]);
        for part in parts.into_iter().map(str::trim).filter(|p| !p.is_empty()) {
            scan_use(&format!("{prefix}::{part}"), walk);
        }
        return;
    }
    let (path, alias) = match tree.split_once(" as ") {
        Some((path, alias)) => (path.trim(), Some(alias.trim())),
        None => (tree, None),
    };
    let mut segments: Vec<&str> = path.split("::").collect();
    let last = segments.pop().unwrap_or("");
    if last == "*" {
        if let Some(module) = segments.pop() {
            walk.globs.push(module.to_owned());
        }
        return;
    }
    let original = if last == "self" {
        segments.pop().unwrap_or("")
    } else {
        last
    };
    match alias {
        Some(alias) => {
            walk.renamed.insert(alias.to_owned(), original.to_owned());
            walk.public.insert(alias.to_owned());
        }
        None => {
            walk.public.insert(original.to_owned());
        }
    }
}

/// Where one module's walk stands, line by line.
struct Cursor<'a> {
    lines: &'a [&'a str],
    next: usize,
    /// Brace depth inside the module body; items live at 0.
    depth: isize,
    /// The last attribute line was `#[cfg(test)]`.
    cfg_test: bool,
    /// The inherent impl the cursor is inside, at depth 1.
    in_impl: Option<String>,
    /// An inherent impl header whose `{` has not come yet.
    pending_impl: Option<String>,
}

/// Walks one module body: `lines` are its top level, `dir` holds its child
/// module files, `public` says whether a caller outside can reach it.
fn scan_module(lines: &[&str], dir: &Path, public: bool, name: &str, walk: &mut Walk) {
    let mut at = Cursor {
        lines,
        next: 0,
        depth: 0,
        cfg_test: false,
        in_impl: None,
        pending_impl: None,
    };
    while let Some(line) = at.lines.get(at.next) {
        let t = line.trim();
        at.next += 1;
        if t.starts_with("//") {
            continue;
        }
        if at.depth != 0 {
            inside_block(t, &mut at, walk);
            continue;
        }
        if t == "#[cfg(test)]" {
            at.cfg_test = true;
            continue;
        }
        if t.starts_with("#[") || t.is_empty() {
            continue;
        }
        let skip = std::mem::take(&mut at.cfg_test);
        if let Some(decl) = mod_decl(t) {
            child_module(decl, &mut at, dir, public && !skip, skip, walk);
        } else if let Some(tree) = t.strip_prefix("pub use ") {
            let tree = whole_statement(tree, &mut at);
            if public && !skip {
                scan_use(tree.trim_end().trim_end_matches(';'), walk);
            }
        } else {
            top_level_item(t, skip, public, name, &mut at, walk);
        }
    }
}

/// A line inside a brace block: collects inherent-impl members at depth 1.
fn inside_block(t: &str, at: &mut Cursor<'_>, walk: &mut Walk) {
    if at.depth == 1
        && let Some(ty) = &at.in_impl
    {
        if t == "#[cfg(test)]" {
            at.cfg_test = true;
        } else if !t.starts_with("#[") {
            if !at.cfg_test
                && let Some(item) = pub_item(t)
            {
                walk.methods.push((ty.clone(), item));
            }
            at.cfg_test = false;
        }
    }
    at.depth += brace_delta(t);
    if at.depth == 0 {
        at.in_impl = None;
    }
}

/// A `mod` declaration: walks the child, inline or from its file.
fn child_module(
    (child, is_pub, inline): (String, bool, bool),
    at: &mut Cursor<'_>,
    dir: &Path,
    parent_reach: bool,
    skip: bool,
    walk: &mut Walk,
) {
    let reach = parent_reach && is_pub;
    let child_dir = dir.join(&child);
    if inline {
        let (body, span) = inline_body(&at.lines[at.next - 1..]);
        at.next += span - 1;
        if !skip {
            scan_module(&body, &child_dir, reach, &child, walk);
        }
    } else if !skip {
        let file = [dir.join(format!("{child}.rs")), child_dir.join("mod.rs")]
            .into_iter()
            .find(|p| p.is_file())
            .unwrap_or_else(|| panic!("no file for `mod {child}` in {}", dir.display()));
        let text = read(&file);
        let body: Vec<&str> = text.lines().collect();
        scan_module(&body, &child_dir, reach, &child, walk);
    }
    if reach {
        walk.public.insert(child);
    }
}

/// `first` plus following lines up to the one ending the statement with `;`.
fn whole_statement(first: &str, at: &mut Cursor<'_>) -> String {
    let mut text = first.to_owned();
    while !text.trim_end().ends_with(';')
        && let Some(line) = at.lines.get(at.next)
    {
        text.push(' ');
        text.push_str(line.trim());
        at.next += 1;
    }
    text
}

/// Any other line at depth 0: a `pub` item, and maybe an impl opening.
fn top_level_item(
    t: &str,
    skip: bool,
    public: bool,
    name: &str,
    at: &mut Cursor<'_>,
    walk: &mut Walk,
) {
    if !skip && let Some(item) = pub_item(t) {
        walk.by_module
            .entry(name.to_owned())
            .or_default()
            .insert(item.clone());
        if public {
            walk.public.insert(item);
        }
    }
    // An `impl` header may run over lines (`where` clauses) before its `{`.
    let header = impl_target(t);
    at.depth += brace_delta(t);
    if at.depth > 0 {
        at.in_impl = if skip {
            None
        } else {
            header.or(at.pending_impl.take())
        };
    } else if t.starts_with("impl") {
        at.pending_impl = header;
    }
}
