//! The grep test (ticket m8/10, finished in 10c): no user-facing string
//! literal in graft-ui spells a default chord where a hint goes. Every key
//! a footer, box or bar names is spelled from his keymap
//! (`Keymap::spell_keys`/`key_verb`/`after`), so a remap moves the words
//! with the key; a literal `esc close` would keep saying `esc` after he
//! moved it.
//!
//! "Where a hint goes": a literal's start, after the bars' two-space gap,
//! after `· `, or a trailing `(x)` — then a default label (or a `/` pair of
//! them) followed by a space and a lowercase word. The label set is every
//! live chord of the shipped table.
//!
//! Test code is not scanned: `tests.rs`, `tests/` folders, the sweep
//! helpers, and everything after a file's `#[cfg(test)]`.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use graft_ui::keymap::{Keymap, defaults};

/// Literals that read like a hint but are not one, each with why. A file
/// and the literal's exact text.
const ALLOWED: &[(&str, &str, &str)] = &[
    (
        "conflict_resolver/panes/draw.rs",
        "? open {dot} {keys}",
        "`?` is the undecided block's mark, not the help key; the take keys after it are spelled",
    ),
    (
        "conflict_resolver/panes/draw.rs",
        "? keep or delete {dot} {keys}",
        "`?` is the undecided file's mark, not the help key; the take keys after it are spelled",
    ),
    (
        "finish_summary/frame.rs",
        "1 merge commit on {into} · nothing is committed {until}",
        "`1` is a count, not a tab key",
    ),
    (
        "finish_summary/frame.rs",
        "1 merge commit · nothing is committed {until}",
        "`1` is a count, not a tab key",
    ),
    ("stash_screen.rs", "1 file", "`1` is a count, not a tab key"),
    (
        "journal_screen.rs",
        "a branch deleted outside Graft and never checked out leaves no trace here",
        "`a` is the article",
    ),
    (
        "keymap/warning.rs",
        "⌃C always quits — app.quit_now can't move to {}",
        "⌃C is the one chord no config moves (ADR-0055): the warning names it on purpose",
    ),
    (
        "keymap/warning.rs",
        "⌃C always quits — not given to {action}",
        "⌃C is the one chord no config moves (ADR-0055): the warning names it on purpose",
    ),
    (
        "journal_screen/view.rs",
        "← next {word}",
        "`←` points at the row, not the left key: the mark of an unbound undo/redo",
    ),
    (
        "detail_pane/header.rs",
        "this is the compare base · x clears it",
        "KNOWN: `graph.compare`'s default; the detail pane takes no keymap yet — outside \
         10c's list, in its NOTES as a follow-up",
    ),
    (
        "detail_pane/line_render.rs",
        "… {more} more lines · L load all",
        "KNOWN: a literal key; the detail pane takes no keymap yet — outside 10c's list, in \
         its NOTES as a follow-up",
    ),
    (
        "graph_screen/banner.rs",
        "change files, space stages, a amends it, c continues · ⇥ banner",
        "KNOWN: the banner's prose names keys (`space`, `a`, `c`, `⇥`); the banner takes no \
         keymap yet — outside 10c's list, in its NOTES as a follow-up",
    ),
    (
        "rewrite_editor/right.rs",
        "CHANGES of {short} · ⏎ back to AFTER",
        "KNOWN: a literal key; the rewrite editor's right pane takes no keymap yet — outside \
         10c's list, in its NOTES as a follow-up",
    ),
    (
        "graph_screen/badges.rs",
        "x base",
        "KNOWN: the compare-base badge names `graph.compare`'s default; badges take no keymap \
         yet — outside 10c's list, in its NOTES as a follow-up",
    ),
];

#[test]
fn no_user_facing_literal_spells_a_default_key() {
    let labels = labels();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut hits = Vec::new();
    for file in rust_files(&root) {
        let source =
            fs::read_to_string(&file).unwrap_or_else(|why| panic!("{}: {why}", file.display()));
        let name = file
            .strip_prefix(&root)
            .unwrap_or(&file)
            .to_string_lossy()
            .replace('\\', "/");
        for literal in literals(&shipped_part(&source)) {
            if let Some(label) = hint_at(&literal, &labels)
                && !ALLOWED
                    .iter()
                    .any(|(path, text, _)| *path == name && *text == literal)
            {
                hits.push(format!("{name}: {literal:?} (`{label}`)"));
            }
        }
    }
    let mut stale = Vec::new();
    for (path, text, _) in ALLOWED {
        let source = fs::read_to_string(root.join(path))
            .unwrap_or_else(|why| panic!("allow-list file {path}: {why}"));
        if !literals(&shipped_part(&source)).any(|literal| literal == *text) {
            stale.push(format!("{path}: {text:?}"));
        }
    }
    assert!(
        hits.is_empty(),
        "{} literal key(s) — spell them from the keymap, or allow with a reason:\n  {}",
        hits.len(),
        hits.join("\n  ")
    );
    assert!(
        stale.is_empty(),
        "allow-list entries no longer in the source:\n  {}",
        stale.join("\n  ")
    );
}

/// Every live chord label of the shipped table: `y`, `esc`, `⏎`, `⌃R`, …
fn labels() -> BTreeSet<String> {
    let shipped = Keymap::shipped();
    defaults()
        .iter()
        .flat_map(|binding| shipped.live_labels(&binding.action).iter().cloned())
        .collect()
}

/// Every `.rs` file under `dir` but test files.
fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let entries = fs::read_dir(dir).unwrap_or_else(|why| panic!("{}: {why}", dir.display()));
    for entry in entries {
        let path = entry
            .unwrap_or_else(|why| panic!("{}: {why}", dir.display()))
            .path();
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned());
        let name = name.as_deref().unwrap_or_default();
        if path.is_dir() {
            if name != "tests" {
                out.extend(rust_files(&path));
            }
        } else if path.extension().is_some_and(|ext| ext == "rs")
            && name != "tests.rs"
            && name != "spell_sweep.rs"
            && name != "sweep.rs"
            // `#[cfg(test)] mod test_support;` in lib.rs.
            && name != "test_support.rs"
        {
            out.push(path);
        }
    }
    out.sort();
    out
}

/// The file up to its first `#[cfg(test)]`, comment lines dropped.
fn shipped_part(source: &str) -> String {
    source
        .lines()
        .take_while(|line| line.trim() != "#[cfg(test)]")
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The text of every `"…"` and `r#"…"#` literal in `source`: `\u{…}`
/// decoded, other escapes left as written.
fn literals(source: &str) -> impl Iterator<Item = String> {
    let chars: Vec<char> = source.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            // A char literal: `'"'`, `'\''`, `'\\'` — skip it whole.
            '\'' if chars.get(i + 2) == Some(&'\'') => i += 3,
            '\'' if chars.get(i + 1) == Some(&'\\') && chars.get(i + 3) == Some(&'\'') => i += 4,
            'r' if matches!(chars.get(i + 1), Some('"' | '#'))
                && !chars
                    .get(i.wrapping_sub(1))
                    .is_some_and(|c| c.is_alphanumeric() || *c == '_') =>
            {
                let hashes = chars[i + 1..].iter().take_while(|c| **c == '#').count();
                let start = i + 1 + hashes + 1;
                let close: String = std::iter::once('"')
                    .chain(std::iter::repeat_n('#', hashes))
                    .collect();
                let rest: String = chars[start.min(chars.len())..].iter().collect();
                let end = rest.find(&close).unwrap_or(rest.len());
                out.push(rest[..end].to_owned());
                i = start + rest[..end].chars().count() + close.chars().count();
            }
            '"' => {
                let mut text = String::new();
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    // `\u{b7}` is the `·` it draws: decoded, so a hint after
                    // an escaped separator is found too.
                    if chars[i] == '\\' && chars.get(i + 1) == Some(&'u') {
                        let hex: String =
                            chars[i + 3..].iter().take_while(|c| **c != '}').collect();
                        let decoded = u32::from_str_radix(&hex, 16)
                            .ok()
                            .and_then(char::from_u32)
                            .unwrap_or_else(|| panic!("bad escape \\u{{{hex}}}"));
                        text.push(decoded);
                        i += 3 + hex.chars().count() + 1;
                        continue;
                    }
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        text.push(chars[i]);
                        i += 1;
                    }
                    text.push(chars[i]);
                    i += 1;
                }
                out.push(text);
                i += 1;
            }
            _ => i += 1,
        }
    }
    out.into_iter()
}

/// The default label `literal` puts where a hint goes, if any.
fn hint_at(literal: &str, labels: &BTreeSet<String>) -> Option<String> {
    let is_label = |token: &str| {
        !token.is_empty()
            && (labels.contains(token) || token.split('/').all(|side| labels.contains(side)))
    };
    let mut starts = vec![0];
    for gap in ["  ", "· "] {
        starts.extend(literal.match_indices(gap).map(|(at, gap)| at + gap.len()));
    }
    for start in starts {
        let rest = &literal[start..];
        let Some((token, after)) = rest.split_once(' ') else {
            continue;
        };
        if is_label(token) && after.starts_with(|c: char| c.is_lowercase()) {
            return Some(token.to_owned());
        }
    }
    literal
        .strip_suffix(')')
        .and_then(|rest| rest.rsplit_once('('))
        .map(|(_, inner)| inner)
        .filter(|inner| is_label(inner))
        .map(str::to_owned)
}
