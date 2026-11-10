//! The grep test (ticket m8/10, finished in 10c): no user-facing string
//! literal in mergewright-ui spells a default chord where a hint goes. Every key
//! a footer, box or bar names is spelled from his keymap
//! (`Keymap::spell_keys`/`key_verb`/`after`), so a remap moves the words
//! with the key; a literal `esc close` would keep saying `esc` after he
//! moved it.
//!
//! "Where a hint goes": a literal's start, after the bars' two-space gap,
//! after `· `, or a trailing `(x)` — then a default label (or a `/` pair of
//! them) followed by a space and a lowercase word; or a heading's key — a
//! label after an all-caps word, before ` · ` or the end (`AFTER y ·
//! checking…`, m8/09h). The label set is every live chord of the shipped
//! table.
//!
//! Test code is not scanned: `tests.rs`, `*_tests.rs`, `tests/` folders,
//! the sweep helpers, and everything after a file's `#[cfg(test)]`.
//!
//! Scanned: mergewright-ui, mergewright and mergewright-ops (ticket m8/10b). This is a
//! ratchet: a new literal key anywhere in the three crates fails at once;
//! a fixed literal still listed fails as stale, so each area ticket
//! (10e–10i) deletes its own rows from its `no_literal_keys/known_*.rs`
//! file. `prose.rs` holds the permanent not-a-key rows of mergewright and
//! mergewright-ops; mergewright-ui's stay in `ALLOWED` below.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use mergewright_ui::keymap::{Keymap, defaults};

#[path = "no_literal_keys/known_ops.rs"]
mod known_ops;
#[path = "no_literal_keys/known_preview.rs"]
mod known_preview;
#[path = "no_literal_keys/known_resolver.rs"]
mod known_resolver;
#[path = "no_literal_keys/known_stops.rs"]
mod known_stops;
#[path = "no_literal_keys/known_undo.rs"]
mod known_undo;
#[path = "no_literal_keys/prose.rs"]
mod prose;

/// Every allow-list row: mergewright-ui's `ALLOWED`, then mergewright's and mergewright-ops'
/// prose and the `KNOWN` rows each area ticket removes.
fn allowed() -> impl Iterator<Item = &'static (&'static str, &'static str, &'static str)> {
    ALLOWED
        .iter()
        .chain(prose::ROWS)
        .chain(known_ops::ROWS)
        .chain(known_stops::ROWS)
        .chain(known_resolver::ROWS)
        .chain(known_undo::ROWS)
        .chain(known_preview::ROWS)
}

/// Literals that read like a hint but are not one, each with why. A file
/// and the literal's exact text.
const ALLOWED: &[(&str, &str, &str)] = &[
    (
        "failure.rs",
        "(x)",
        "the failure mark's ASCII form, a glyph, not a key",
    ),
    (
        "finish_summary.rs",
        "1 clash solved. Nothing committed yet.",
        "`1` is the review's clash count, not a tab key",
    ),
    (
        "finish_summary.rs",
        "1 commit from {from}",
        "`1` is the review's incoming count, not a tab key",
    ),
    (
        "finish_summary.rs",
        "? commits from {from}",
        "`?` is a count git could not give, not the help key",
    ),
    (
        "graph_screen/fold_mark.rs",
        "1 commit",
        "`1` is a folding merge's count, not the first-parent key",
    ),
    (
        "graph_screen/fold_mark.rs",
        "? commits",
        "`?` is a count git could not give, not the help key",
    ),
    (
        "stash_screen/view.rs",
        "1 file",
        "`1` is a count, not a tab key",
    ),
    (
        "journal_screen.rs",
        "a branch deleted outside Mergewright and never checked out leaves no trace",
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
];

#[test]
fn no_user_facing_literal_spells_a_default_key() {
    let labels = labels();
    let mut hits = Vec::new();
    for (prefix, root) in roots() {
        for file in rust_files(&root) {
            let source =
                fs::read_to_string(&file).unwrap_or_else(|why| panic!("{}: {why}", file.display()));
            let name = format!(
                "{prefix}{}",
                file.strip_prefix(&root)
                    .unwrap_or(&file)
                    .to_string_lossy()
                    .replace('\\', "/")
            );
            for literal in literals(&shipped_part(&source)) {
                if let Some(label) = hint_at(&literal, &labels)
                    && !allowed().any(|(path, text, _)| *path == name && *text == literal)
                {
                    hits.push(format!("{name}: {literal:?} (`{label}`)"));
                }
            }
        }
    }
    let mut stale = Vec::new();
    for (path, text, _) in allowed() {
        let source = fs::read_to_string(source_path(path))
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

/// The trees scanned, each with the prefix its allow-list paths carry:
/// mergewright-ui's own (none), then mergewright's and mergewright-ops' status, refusal and
/// error words (ticket m8/10b). Read by path — no crate dependency added.
fn roots() -> Vec<(&'static str, PathBuf)> {
    let ui = Path::new(env!("CARGO_MANIFEST_DIR"));
    // mergewright-ui sits in `crates/`, beside mergewright and mergewright-ops.
    let crates = ui.join("..");
    vec![
        ("", ui.join("src")),
        ("mergewright:", crates.join("mergewright").join("src")),
        (
            "mergewright-ops:",
            crates.join("mergewright-ops").join("src"),
        ),
    ]
}

/// The file an allow-list path names.
fn source_path(path: &str) -> PathBuf {
    let roots = roots();
    roots
        .iter()
        .skip(1)
        .find_map(|(prefix, root)| path.strip_prefix(prefix).map(|rest| root.join(rest)))
        .unwrap_or_else(|| roots[0].1.join(path))
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
            && !name.ends_with("_tests.rs")
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
    // A heading's key (m8/09h): a label after an all-caps word, then ` · `
    // or the end — `AFTER y · checking…`, which the rules above missed
    // because no lowercase word follows the key.
    let words: Vec<&str> = literal.split(' ').collect();
    for (at, pair) in words.windows(2).enumerate() {
        let heading =
            pair[0].chars().count() >= 2 && pair[0].chars().all(|c| c.is_ascii_uppercase());
        let closes = words.get(at + 2).is_none_or(|next| *next == "·");
        if heading && closes && is_label(pair[1]) {
            return Some(pair[1].to_owned());
        }
    }
    literal
        .strip_suffix(')')
        .and_then(|rest| rest.rsplit_once('('))
        .map(|(_, inner)| inner)
        .filter(|inner| is_label(inner))
        .map(str::to_owned)
}
