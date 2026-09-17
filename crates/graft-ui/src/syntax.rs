//! Line-level syntax highlighting over already-sanitised text.
//!
//! `git` hands `ui` plain lines (`term::SanitizedText`); nothing here reads a
//! file or spawns `git` — the caller decides *what* language a diff or blob
//! is in ([`language_for_path`], [`language_for_first_line`]) and this module
//! only turns lines into [`SyntaxRole`] spans ([`highlight`]). Spans are
//! **char** indices, not byte offsets: sanitising can already have changed a
//! line's byte length (`�`/`␛` are multi-byte), and the widgets that slice a
//! line for drawing always slice by char (PROMPT.md §6.3).
//!
//! Roles are Graft's own nine-value vocabulary, not syntect's hundreds of
//! scope names — [`Theme::syntax`] is the only place a role becomes a colour,
//! and it draws from the same tokens as the rest of the chrome.
//!
//! # Caps
//!
//! A pathological file (a minified bundle, a generated fixture) must never
//! make the parser expensive: past `MAX_LINES` lines, or a single line past
//! `MAX_LINE_CHARS` chars, [`highlight`] returns every line as one `Plain`
//! span without running the parser at all. `NO_COLOR` and low colour depths
//! already make roles collapse to the same colour ([`Theme::syntax`]), so this
//! only costs colour on files nobody reads a diff of comfortably anyway.

use std::sync::OnceLock;

use syntect::parsing::{ScopeStack, SyntaxReference, SyntaxSet};

/// A highlighted token's category, independent of language.
///
/// Deliberately coarse: PROMPT.md's diff view paints by role, not by the
/// hundreds of scope names a `.sublime-syntax` file can produce, and a role
/// the theme has no colour for would be a role nobody could tell apart from
/// `Plain` anyway.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SyntaxRole {
    /// No known role, or highlighting was skipped ([`highlight`]'s caps).
    Plain,
    /// A comment, of any language's spelling.
    Comment,
    /// A string or character literal.
    String,
    /// A keyword or storage modifier (`fn`, `let`, `static`, `pub`).
    Keyword,
    /// A type or type-like name.
    Type,
    /// A function or method name.
    Function,
    /// A numeric literal.
    Number,
    /// Punctuation: brackets, operators, separators.
    Punctuation,
    /// An attribute or annotation (`#[derive(..)]`, `@Override`).
    Attribute,
}

/// A language, resolved once from a path or a first line.
///
/// Holds the `syntect` syntax definition directly: [`syntax_set`] lives for
/// the process's life ([`OnceLock`]), so borrowing from it is `'static` and
/// there is nothing to re-look-up in [`highlight`].
#[derive(Clone, Copy)]
pub struct Language(&'static SyntaxReference);

/// The `syntect` syntax set, loaded once. Built-in syntaxes only
/// (`default-syntaxes`); no user-supplied `.sublime-syntax` files.
fn syntax_set() -> &'static SyntaxSet {
    static SET: OnceLock<SyntaxSet> = OnceLock::new();
    SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

/// The language a path's extension or exact file name implies.
///
/// Tries the extension first (`token.rs` → Rust); falls back to the whole
/// file name for the extension-less names `syntect`'s bundled syntaxes match
/// literally (`Makefile`, `Dockerfile`). `None` for an extension `syntect`
/// doesn't recognise — the caller draws `Plain`.
#[must_use]
pub fn language_for_path(path: &str) -> Option<Language> {
    let set = syntax_set();
    let file_name = std::path::Path::new(path).file_name()?.to_str()?;

    if let Some(extension) = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        && let Some(syntax) = set.find_syntax_by_extension(extension)
    {
        return Some(Language(syntax));
    }

    set.find_syntax_by_extension(file_name).map(Language)
}

/// The language a file's first line implies: a shebang or an editor
/// modeline, for a file with no recognisable extension.
#[must_use]
pub fn language_for_first_line(line: &str) -> Option<Language> {
    syntax_set().find_syntax_by_first_line(line).map(Language)
}

/// One highlighted run within a line: a half-open, ascending, non-overlapping
/// **char** range and the role it was given.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyntaxSpan {
    /// The run's char range within the line.
    pub range: std::ops::Range<usize>,
    /// The run's role.
    pub role: SyntaxRole,
}

/// Lines past this count are never parsed: every line comes back `Plain`.
const MAX_LINES: usize = 5_000;
/// A single line past this many chars is never parsed either: the whole call
/// falls back to `Plain`, since a parser state carried past an unparsed line
/// would be meaningless for the lines after it.
const MAX_LINE_CHARS: usize = 4_000;

/// Highlight every line of a file (or a diff's old/new side) in one language.
///
/// Parser state carries across lines, so a block comment or a triple-quoted
/// string opened on one line and closed several lines later marks every line
/// between as its role. Never panics: a `syntect` parse error on a line (a
/// syntax whose rules can't advance on that input) falls back to `Plain` for
/// that line only, and the caps above fall back for the whole call.
#[must_use]
pub fn highlight(lang: &Language, lines: &[&str]) -> Vec<Vec<SyntaxSpan>> {
    if lines.len() > MAX_LINES
        || lines
            .iter()
            .any(|line| line.chars().count() > MAX_LINE_CHARS)
    {
        return lines.iter().map(|line| plain_line(line)).collect();
    }

    let set = syntax_set();
    let mut parse_state = syntect::parsing::ParseState::new(lang.0);
    let mut stack = ScopeStack::new();

    lines
        .iter()
        .map(|line| highlight_line(line, set, &mut parse_state, &mut stack))
        .collect()
}

/// One line's worth of spans, from the ops `syntect` reports for it, applied
/// to the scope stack carried from previous lines.
fn highlight_line(
    line: &str,
    set: &SyntaxSet,
    parse_state: &mut syntect::parsing::ParseState,
    stack: &mut ScopeStack,
) -> Vec<SyntaxSpan> {
    let Ok(ops) = parse_state.parse_line(line, set) else {
        return plain_line(line);
    };

    let byte_to_char = byte_to_char_table(line);
    let mut spans = Vec::new();
    let mut start_byte = 0usize;
    let mut role = role_for_scope(stack);

    for (pos, op) in ops {
        if pos > start_byte {
            spans.push(raw_span(start_byte, pos, role, &byte_to_char));
        }
        if stack.apply(&op).is_err() {
            // A malformed op from a broken syntax definition: stop trusting
            // the stack for the rest of this line rather than panic or loop.
            spans.push(raw_span(pos, line.len(), SyntaxRole::Plain, &byte_to_char));
            return merge_adjacent(spans);
        }
        role = role_for_scope(stack);
        start_byte = pos;
    }
    if start_byte < line.len() {
        spans.push(raw_span(start_byte, line.len(), role, &byte_to_char));
    }

    merge_adjacent(spans)
}

/// One `Plain` span covering the whole line, or none for an empty line.
fn plain_line(line: &str) -> Vec<SyntaxSpan> {
    let chars = line.chars().count();
    if chars == 0 {
        Vec::new()
    } else {
        vec![SyntaxSpan {
            range: 0..chars,
            role: SyntaxRole::Plain,
        }]
    }
}

/// Each byte offset a char boundary starts at, in order — `table[i]` is the
/// char index of byte `table_bytes[i]`. Built once per line so an op stream
/// with many positions doesn't re-walk the line from the start each time.
fn byte_to_char_table(line: &str) -> Vec<(usize, usize)> {
    let mut table: Vec<(usize, usize)> = line
        .char_indices()
        .enumerate()
        .map(|(char_index, (byte_index, _))| (byte_index, char_index))
        .collect();
    table.push((line.len(), line.chars().count()));
    table
}

/// The char index for a byte offset that is known to land on a char boundary
/// (every position `syntect` reports does, since it only ever matches whole
/// chars).
fn char_index_for(byte: usize, table: &[(usize, usize)]) -> usize {
    match table.binary_search_by_key(&byte, |&(b, _)| b) {
        Ok(i) => table[i].1,
        // Not on a recorded boundary: nearest lower one plus 0, which only
        // happens if `syntect` ever reports a mid-char byte offset. Falling
        // back rather than panicking keeps the "never panics" guarantee.
        Err(i) => table[i.saturating_sub(1)].1,
    }
}

fn raw_span(
    start_byte: usize,
    end_byte: usize,
    role: SyntaxRole,
    table: &[(usize, usize)],
) -> SyntaxSpan {
    SyntaxSpan {
        range: char_index_for(start_byte, table)..char_index_for(end_byte, table),
        role,
    }
}

/// Merge consecutive spans that ended up with the same role — `syntect`
/// reports a stack change at every op even when the role it maps to didn't.
fn merge_adjacent(spans: Vec<SyntaxSpan>) -> Vec<SyntaxSpan> {
    let mut merged: Vec<SyntaxSpan> = Vec::with_capacity(spans.len());
    for span in spans {
        if span.range.start == span.range.end {
            continue;
        }
        if let Some(last) = merged.last_mut()
            && last.role == span.role
            && last.range.end == span.range.start
        {
            last.range.end = span.range.end;
            continue;
        }
        merged.push(span);
    }
    merged
}

/// The role for the current scope stack: the outermost scope that matches
/// one of the prefixes below, else `Plain`.
///
/// Outermost, not innermost: a comment or a string pushes its own inner
/// scope for the delimiter alone (`punctuation.definition.comment.rust` for
/// `//`, `punctuation.definition.string.begin.rust` for the opening `"`), and
/// that inner scope would otherwise split one comment or string into two or
/// three spans. Checking from the bottom of the stack up and stopping at the
/// first match means the comment/string scope wins before its own
/// punctuation gets a chance to.
fn role_for_scope(stack: &ScopeStack) -> SyntaxRole {
    for scope in stack.as_slice() {
        let name = scope.to_string();
        let role = if starts_with_any(&name, &["comment"]) {
            Some(SyntaxRole::Comment)
        } else if starts_with_any(&name, &["string"]) {
            Some(SyntaxRole::String)
        } else if starts_with_any(&name, &["keyword", "storage"]) {
            Some(SyntaxRole::Keyword)
        } else if starts_with_any(&name, &["entity.name.type", "support.type"]) {
            Some(SyntaxRole::Type)
        } else if starts_with_any(&name, &["entity.name.function", "support.function"]) {
            Some(SyntaxRole::Function)
        } else if starts_with_any(&name, &["constant.numeric"]) {
            Some(SyntaxRole::Number)
        } else if starts_with_any(&name, &["punctuation"]) {
            Some(SyntaxRole::Punctuation)
        } else if starts_with_any(&name, &["meta.attribute", "entity.other.attribute-name"]) {
            Some(SyntaxRole::Attribute)
        } else {
            None
        };
        if let Some(role) = role {
            return role;
        }
    }
    SyntaxRole::Plain
}

fn starts_with_any(name: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|prefix| name.starts_with(prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rust() -> Language {
        language_for_path("x.rs").expect("rust is a default syntax")
    }

    #[test]
    fn extension_resolves_a_known_language() {
        assert!(language_for_path("src/auth/token.rs").is_some());
    }

    #[test]
    fn unknown_extension_resolves_to_none() {
        assert!(language_for_path("x.unknownext").is_none());
    }

    #[test]
    fn exact_file_name_resolves_makefile() {
        assert!(language_for_path("Makefile").is_some());
    }

    #[test]
    fn first_line_resolves_a_shebang() {
        let lang = language_for_first_line("#!/usr/bin/env python3").expect("python shebang");
        // No public accessor for the name; highlighting a python-only
        // construct is the behavioural proof used elsewhere in this module.
        let spans = highlight(&lang, &["def f():"]);
        assert!(spans[0].iter().any(|s| s.role == SyntaxRole::Keyword));
    }

    #[test]
    fn a_line_comment_is_one_span() {
        let spans = highlight(&rust(), &["// hi"]);
        assert_eq!(spans[0].len(), 1);
        assert_eq!(spans[0][0].role, SyntaxRole::Comment);
        assert_eq!(spans[0][0].range, 0..5);
    }

    #[test]
    fn keywords_function_and_string_are_found() {
        let spans = highlight(&rust(), &["fn main() { let s = \"a\"; }"]);
        let line = &spans[0];
        let role_at = |i: usize| line.iter().find(|s| s.range.contains(&i)).map(|s| s.role);

        assert_eq!(role_at(0), Some(SyntaxRole::Keyword)); // fn
        assert_eq!(role_at(3), Some(SyntaxRole::Function)); // main
        assert_eq!(role_at(13), Some(SyntaxRole::Keyword)); // let
        assert_eq!(role_at(21), Some(SyntaxRole::String)); // "a"

        for span in line {
            assert!(span.range.end <= line_char_len("fn main() { let s = \"a\"; }"));
        }
    }

    fn line_char_len(s: &str) -> usize {
        s.chars().count()
    }

    #[test]
    fn a_block_comment_spans_the_lines_between_open_and_close() {
        let spans = highlight(&rust(), &["/*", "middle", "*/"]);
        assert!(spans[1].iter().all(|s| s.role == SyntaxRole::Comment));
    }

    #[test]
    fn past_5000_lines_everything_is_plain() {
        let lines: Vec<&str> = std::iter::repeat_n("fn main() {}", 5_001).collect();
        let spans = highlight(&rust(), &lines);
        for line in &spans {
            assert!(line.iter().all(|s| s.role == SyntaxRole::Plain));
        }
    }

    #[test]
    fn a_line_past_4000_chars_falls_back_to_plain_for_the_call() {
        let long = "a".repeat(4_001);
        let spans = highlight(&rust(), &["fn main() {}", long.as_str()]);
        for line in &spans {
            assert!(line.iter().all(|s| s.role == SyntaxRole::Plain));
        }
    }

    #[test]
    fn empty_line_has_no_spans() {
        let spans = highlight(&rust(), &[""]);
        assert!(spans[0].is_empty());
    }

    #[test]
    fn spans_never_overlap_or_run_past_the_line() {
        assert_spans_are_well_formed(&["fn foo(x: i32) -> i32 { x + 1 }", "// a", "\"s\"", ""]);
    }

    fn assert_spans_are_well_formed(lines: &[&str]) {
        let spans = highlight(&rust(), lines);
        for (line, line_spans) in lines.iter().zip(spans.iter()) {
            let len = line.chars().count();
            let mut last_end = 0;
            for span in line_spans {
                assert!(span.range.start >= last_end);
                assert!(span.range.end <= len);
                assert!(span.range.start < span.range.end);
                last_end = span.range.end;
            }
        }
    }

    mod properties {
        use proptest::prelude::*;

        use super::*;

        /// The case count for whichever tier this was compiled for
        /// (`graft-term`'s sanitiser tests document the tiering; ADR-0017).
        fn tier() -> ProptestConfig {
            ProptestConfig::with_cases(graft_fixtures::proptest_cases())
        }

        proptest! {
            #![proptest_config(tier())]

            /// Arbitrary text — combining marks and emoji included — never
            /// panics and never yields an overlapping or out-of-range span.
            #[test]
            fn highlight_never_panics_or_overlaps(lines in proptest::collection::vec(".*", 0..12)) {
                let lang = language_for_path("x.rs").expect("rust is a default syntax");
                let borrowed: Vec<&str> = lines.iter().map(String::as_str).collect();
                let spans = highlight(&lang, &borrowed);

                for (line, line_spans) in borrowed.iter().zip(spans.iter()) {
                    let len = line.chars().count();
                    let mut last_end = 0usize;
                    for span in line_spans {
                        prop_assert!(span.range.start >= last_end);
                        prop_assert!(span.range.end <= len);
                        prop_assert!(span.range.start < span.range.end);
                        last_end = span.range.end;
                    }
                }
            }
        }
    }
}
