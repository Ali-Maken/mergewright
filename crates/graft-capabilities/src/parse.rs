//! Reads `docs/capabilities/*.md` into [`Capability`] values.
//!
//! The prose is authored by hand and never rewritten here; this module only
//! reads it. Each capability is a `### M<milestone>-<n>. <title>` heading,
//! followed immediately by a fenced ` ```meta ` block (id/area/milestone/
//! status) and then the existing prose bullets (`How`, `Behaviour`,
//! `Proven by`, optionally `Looks like`, `Decided in`, `Status`).

use std::fs;
use std::path::{Path, PathBuf};

use crate::model::{Capability, Decision, DecisionPath, Status};

/// Everything that can go wrong turning `docs/capabilities/*.md` into
/// [`Capability`] values.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    /// Could not read `docs/capabilities/` itself.
    #[error("could not read {path}: {source}")]
    Directory {
        /// The directory that could not be read.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// Could not read one capability file.
    #[error("could not read {path}: {source}")]
    File {
        /// The file that could not be read.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// A capability's schema is malformed: a missing or unparsable `meta`
    /// field, or an id that doesn't match its heading.
    #[error("{path}:{line}: malformed capability schema — {reason}")]
    MalformedSchema {
        /// The file this was found in, relative to the repo root.
        path: PathBuf,
        /// 1-based line number of the offending heading.
        line: usize,
        /// What's wrong.
        reason: String,
    },
}

/// Reads every capability out of `<repo_root>/docs/capabilities/*.md`, in
/// file order then heading order. Fails fast on the first malformed entry —
/// this is a small, hand-authored corpus, not user input, so one error at a
/// time is the right trade.
pub fn load(repo_root: &Path) -> Result<Vec<Capability>, LoadError> {
    let dir = repo_root.join("docs/capabilities");
    let mut entries: Vec<PathBuf> = fs::read_dir(&dir)
        .map_err(|source| LoadError::Directory {
            path: dir.clone(),
            source,
        })?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        // `README.md` documents the schema (with an example block that
        // isn't a real capability) rather than holding capabilities itself.
        .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
        .filter(|path| path.file_name().and_then(|n| n.to_str()) != Some("README.md"))
        .collect();
    entries.sort();

    let mut out = Vec::new();
    for path in entries {
        let content = fs::read_to_string(&path).map_err(|source| LoadError::File {
            path: path.clone(),
            source,
        })?;
        let rel = path.strip_prefix(repo_root).unwrap_or(&path).to_path_buf();
        out.extend(parse_file(&rel, &content)?);
    }
    Ok(out)
}

/// Parses one capability file's content. `source_file` is the path recorded
/// on each [`Capability`] (relative to the repo root, for error messages).
fn parse_file(source_file: &Path, content: &str) -> Result<Vec<Capability>, LoadError> {
    let lines: Vec<&str> = content.lines().collect();
    let mut caps = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let Some((id, title)) = heading(lines[i]) else {
            i += 1;
            continue;
        };
        let heading_line = i + 1; // 1-based
        i += 1;

        // The end of this capability's span is the next `### ` or `## `
        // heading, or EOF.
        let span_start = i;
        let span_end = lines[span_start..]
            .iter()
            .position(|line| line.starts_with("### ") || line.starts_with("## "))
            .map_or(lines.len(), |offset| span_start + offset);
        let span = &lines[span_start..span_end];

        let meta = meta_block(span).ok_or_else(|| LoadError::MalformedSchema {
            path: source_file.to_path_buf(),
            line: heading_line,
            reason: "no fenced ```meta block right after the heading".to_owned(),
        })?;

        let meta_id = required_meta_field(&meta, "id", source_file, heading_line)?;
        if meta_id != id {
            return Err(LoadError::MalformedSchema {
                path: source_file.to_path_buf(),
                line: heading_line,
                reason: format!("meta id `{meta_id}` does not match heading id `{id}`"),
            });
        }
        let area = required_meta_field(&meta, "area", source_file, heading_line)?;
        let milestone = required_meta_field(&meta, "milestone", source_file, heading_line)?;
        let status_raw = required_meta_field(&meta, "status", source_file, heading_line)?;
        let status = Status::parse(&status_raw).ok_or_else(|| LoadError::MalformedSchema {
            path: source_file.to_path_buf(),
            line: heading_line,
            reason: format!("status `{status_raw}` is not one of works/partial/unverified/broken"),
        })?;
        // Optional: most capabilities have no unverified aspect to carry.
        let unverified_aspect = meta_field(&meta, "unverified_aspect");

        let tests = proven_by_citations(span, "**Proven by:**");
        let captures = looks_like_captures(span, "**Looks like:**");
        let decided_in = bullet_text(span, "**Decided in:**").unwrap_or_default();
        let status_prose = bullet_text(span, "**Status:**").unwrap_or_default();
        let decisions = decisions_from(&decided_in);

        caps.push(Capability {
            id,
            title,
            area,
            milestone,
            status,
            unverified_aspect,
            status_prose,
            tests,
            captures,
            decided_in,
            decisions,
            source_file: source_file.to_path_buf(),
            source_line: heading_line,
        });
    }
    Ok(caps)
}

/// Matches `### M<n>-<n>. <title>`, returning `(id, title)`.
fn heading(line: &str) -> Option<(String, String)> {
    let rest = line.strip_prefix("### ")?;
    let (id, title) = rest.split_once(". ")?;
    if !id.starts_with('M') || !id.contains('-') {
        return None;
    }
    Some((id.to_owned(), title.to_owned()))
}

/// Pulls the first ` ```meta ` … ` ``` ` block out of a capability's span,
/// as raw `key: value` lines.
fn meta_block(span: &[&str]) -> Option<Vec<String>> {
    let start = span.iter().position(|line| line.trim() == "```meta")?;
    let end = span[start + 1..]
        .iter()
        .position(|line| line.trim() == "```")
        .map(|offset| start + 1 + offset)?;
    Some(
        span[start + 1..end]
            .iter()
            .map(|s| (*s).to_owned())
            .collect(),
    )
}

/// Reads a `key: value` line out of a meta block's raw lines.
fn meta_field(meta: &[String], key: &str) -> Option<String> {
    let prefix = format!("{key}:");
    for line in meta {
        if let Some(value) = line.trim().strip_prefix(&prefix) {
            return Some(value.trim().to_owned());
        }
    }
    None
}

/// [`meta_field`], turned into a [`LoadError::MalformedSchema`] when the
/// key is missing — the shape every one of the four required fields needs.
fn required_meta_field(
    meta: &[String],
    key: &str,
    source_file: &Path,
    heading_line: usize,
) -> Result<String, LoadError> {
    meta_field(meta, key).ok_or_else(|| LoadError::MalformedSchema {
        path: source_file.to_path_buf(),
        line: heading_line,
        reason: format!("meta block has no `{key}` field"),
    })
}

/// Finds `- **Proven by:**` and returns every backtick span that reads as a
/// Rust path (two or more `::`-joined segments of identifier characters) —
/// what distinguishes an actual `crate::module::tests::name` citation from
/// a keystroke (`` `Enter` ``, no `::`) or a file path (`` `a/b.rs` ``,
/// rejected by the character check) mentioned in the same prose. A brace
/// group (`` `crate::tests::{a,b,c}_shared_suffix` ``) expands to one
/// citation per alternative, so each of a shared-suffix family of tests is
/// checked on its own.
fn proven_by_citations(span: &[&str], label: &str) -> Vec<String> {
    let Some(text) = text_of_bullet(span, label) else {
        return Vec::new();
    };
    backtick_spans(&text)
        .into_iter()
        .flat_map(|raw| expand_braces(&raw))
        .filter(|candidate| is_path_like(candidate))
        .collect()
}

/// Finds `- **Looks like:**` and returns every backtick span that is a
/// `docs/shots/...` path — what every real capture citation looks like, so
/// stray backticks elsewhere in the sentence (there aren't any today, but
/// nothing guarantees that stays true) can't be mistaken for one.
fn looks_like_captures(span: &[&str], label: &str) -> Vec<String> {
    let Some(text) = text_of_bullet(span, label) else {
        return Vec::new();
    };
    backtick_spans(&text)
        .into_iter()
        // `<stop>`-style placeholders describe a whole family of captures
        // ("one pair per stop") rather than naming a literal file; the
        // concrete example prose gives right after (e.g.
        // `docs/shots/35-trivial-unicode.txt`) is a normal, checked
        // citation.
        .filter(|s| s.starts_with("docs/shots/") && !s.contains('<'))
        .collect()
}

/// True if `text` is two or more `::`-joined segments, each made only of
/// ASCII letters, digits, `_` or `-` (crate names in this prose sometimes
/// use the Cargo hyphenated spelling, e.g. `graft-ui::journal_screen`).
fn is_path_like(text: &str) -> bool {
    let mut segments = text.split("::").peekable();
    segments.peek().is_some()
        && text.contains("::")
        && text.split("::").all(|segment| {
            !segment.is_empty()
                && segment
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        })
}

/// Expands one `{a,b,c}` brace group in `text` into `text` with each
/// alternative substituted — the shorthand `docs/capabilities` uses for a
/// family of tests sharing a prefix and suffix. No brace group: returns
/// `text` unchanged, as the one element.
fn expand_braces(text: &str) -> Vec<String> {
    let Some(open) = text.find('{') else {
        return vec![text.to_owned()];
    };
    let Some(close_offset) = text[open..].find('}') else {
        return vec![text.to_owned()];
    };
    let close = open + close_offset;
    let prefix = &text[..open];
    let suffix = &text[close + 1..];
    text[open + 1..close]
        .split(',')
        .map(|alt| format!("{prefix}{alt}{suffix}"))
        .collect()
}

/// Same span-finding as [`bullet_backticks`], but returns the joined raw
/// text instead of extracting code spans.
fn bullet_text(span: &[&str], label: &str) -> Option<String> {
    text_of_bullet(span, label)
}

/// Finds `- **<label>** <rest of line>` and joins every following line up
/// to the next top-level bullet (`- **...`) or the end of the span.
fn text_of_bullet(span: &[&str], label: &str) -> Option<String> {
    let marker = format!("- {label}");
    let start = span
        .iter()
        .position(|line| line.trim_start().starts_with(&marker))?;
    let mut text = span[start]
        .trim_start()
        .strip_prefix(&marker)
        .unwrap_or("")
        .to_owned();
    let mut i = start + 1;
    while i < span.len() {
        let line = span[i];
        if line.trim_start().starts_with("- **") {
            break;
        }
        // A hard-wrapped line can break mid-span, e.g. a path split as
        // `` `docs/design/ `` / `` 04-thing.md` ``. Joining with a space
        // there would corrupt the citation, so only insert one when we're
        // not inside an open backtick span (an even number of backticks
        // seen so far).
        if text.matches('`').count() % 2 == 0 {
            text.push(' ');
        }
        text.push_str(line.trim());
        i += 1;
    }
    Some(text.trim().to_owned())
}

/// Every substring between paired backticks, in order.
fn backtick_spans(text: &str) -> Vec<String> {
    text.split('`')
        .enumerate()
        .filter(|&(idx, _part)| idx % 2 == 1)
        .map(|(_idx, part)| part.to_owned())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Pulls every file citation out of `**Decided in:**` prose: backtick spans
/// that look like a markdown file, and bare `ADR-NNNN` references anywhere
/// in the text (the prose often names an ADR without a path, e.g.
/// `"ADR-0016 (tier order)"`).
fn decisions_from(decided_in: &str) -> Vec<Decision> {
    let mut out = Vec::new();
    for span in backtick_spans(decided_in) {
        if Path::new(&span)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
        {
            out.push(Decision {
                cited: span.clone(),
                resolved: DecisionPath::Exact(span),
            });
        }
    }
    for adr in adr_refs(decided_in) {
        out.push(Decision {
            resolved: DecisionPath::GlobPrefix {
                dir: "docs/adr".to_owned(),
                prefix: adr.clone(),
            },
            cited: format!("ADR-{adr}"),
        });
    }
    out
}

/// Finds every `ADR-NNNN` (4 ASCII digits) in `text`, returning the digit
/// strings, deduplicated in first-seen order.
fn adr_refs(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i + 8 <= bytes.len() {
        if &bytes[i..i + 4] == b"ADR-" && bytes[i + 4..i + 8].iter().all(u8::is_ascii_digit) {
            let digits = text[i + 4..i + 8].to_owned();
            if !out.contains(&digits) {
                out.push(digits);
            }
            i += 8;
        } else {
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r"# Capabilities — M9 Sample

## Capabilities

### M9-01. Does a thing

```meta
id: M9-01
area: sample
milestone: m9-sample
status: works
unverified_aspect: colour, unverified by eye
```

- **How:** press a key.
- **Behaviour:** does the thing.
- **Proven by:**
  `graft::tests::it::sample::does_the_thing`
  `graft_core::tests::sample_unit`
- **Looks like:** `docs/shots/sample.txt`
- **Decided in:** `docs/design/09-sample.md` §1; ADR-0099.
- **Status:** works. Colour is unverified by eye — captures record no colour attributes.

## Deliberately not built

- **Nothing else.**
";

    #[test]
    fn parses_the_meta_block_and_title() {
        let cap = parse_sample();
        assert_eq!(cap.id, "M9-01");
        assert_eq!(cap.title, "Does a thing");
        assert_eq!(cap.area, "sample");
        assert_eq!(cap.milestone, "m9-sample");
        assert_eq!(cap.status, Status::Works);
        assert_eq!(
            cap.unverified_aspect.as_deref(),
            Some("colour, unverified by eye")
        );
        assert_eq!(cap.source_line, 5);
    }

    #[test]
    fn unverified_aspect_is_none_when_the_meta_field_is_absent() {
        let without_aspect =
            SAMPLE.replacen("unverified_aspect: colour, unverified by eye\n", "", 1);
        let caps = parse_file(Path::new("x.md"), &without_aspect).unwrap();
        assert_eq!(caps[0].unverified_aspect, None);
    }

    #[test]
    fn parses_tests_and_captures() {
        let cap = parse_sample();
        assert_eq!(
            cap.tests,
            vec![
                "graft::tests::it::sample::does_the_thing".to_owned(),
                "graft_core::tests::sample_unit".to_owned(),
            ]
        );
        assert_eq!(cap.captures, vec!["docs/shots/sample.txt".to_owned()]);
    }

    #[test]
    fn parses_decisions_as_a_path_and_an_adr_glob() {
        let cap = parse_sample();
        assert!(cap.decided_in.contains("ADR-0099"));
        assert_eq!(cap.decisions.len(), 2);
        assert!(matches!(
            &cap.decisions[0].resolved,
            DecisionPath::Exact(path) if path == "docs/design/09-sample.md"
        ));
        assert!(matches!(
            &cap.decisions[1].resolved,
            DecisionPath::GlobPrefix { dir, prefix }
                if dir == "docs/adr" && prefix == "0099"
        ));
    }

    fn parse_sample() -> Capability {
        let caps = parse_file(Path::new("docs/capabilities/sample.md"), SAMPLE).unwrap();
        assert_eq!(caps.len(), 1);
        caps.into_iter().next().unwrap()
    }

    #[test]
    fn missing_meta_block_is_malformed() {
        let bad = SAMPLE.replacen("```meta", "not-meta", 1);
        let err = parse_file(Path::new("x.md"), &bad).unwrap_err();
        assert!(matches!(err, LoadError::MalformedSchema { .. }));
    }

    #[test]
    fn mismatched_id_is_malformed() {
        let bad = SAMPLE.replacen("id: M9-01", "id: M9-02", 1);
        let err = parse_file(Path::new("x.md"), &bad).unwrap_err();
        let LoadError::MalformedSchema { reason, .. } = err else {
            panic!("expected MalformedSchema");
        };
        assert!(reason.contains("does not match"));
    }

    #[test]
    fn bad_status_value_is_malformed() {
        let bad = SAMPLE.replacen("status: works", "status: kinda", 1);
        let err = parse_file(Path::new("x.md"), &bad).unwrap_err();
        assert!(matches!(err, LoadError::MalformedSchema { .. }));
    }

    #[test]
    fn adr_refs_finds_bare_and_repeated() {
        let text = "ADR-0016 (tier order); also see ADR-0016 and ADR-0024.";
        assert_eq!(adr_refs(text), vec!["0016".to_owned(), "0024".to_owned()]);
    }
}
