# syntax-highlighting: syntax roles and word-diff spans over sanitised text

Status: done
Blocked by: none

Build with: Sonnet — two pure functions over `&str` with fixed inputs and outputs; the library choices are made here.
Needs: none (pure; `detail-model-and-jobs` and `diff-widget` call these)

## Notes (build report)

- `crates/graft-ui/src/syntax.rs`: `syntect` 5.3.0, `default-syntaxes` +
  `regex-fancy`, no `onig`/default themes. `Language` holds `&'static
  SyntaxReference` directly (the `SyntaxSet` lives in a `OnceLock`, so the
  reference is `'static` — no name-based re-lookup needed). Role mapping
  scans the scope stack **bottom-up** (outermost first), not top-down: a
  comment/string pushes its own inner scope for the delimiter alone
  (`punctuation.definition.comment.rust`), and top-down matching split one
  comment into two spans in testing. Bottom-up fixed it and matches the
  acceptance criteria exactly.
- `crates/graft-ui/src/diff_words.rs`: `similar` 3.2.0, default features
  (`tokenize_words` fully partitions the line including whitespace runs, so
  a char cursor over `iter_all_changes()` gives exact char ranges). "No
  common word" pairs (full replacement) return empty spans on both sides.
- `Theme::syntax` added to `theme/mod.rs`.
- Build time (clean, `cargo build -p graft-ui --quiet`, deps included):
  before ~4.2s, after ~7.1s (syntect's cost, as expected — PROMPT.md names
  this trade-off).
- Gates: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test` (workspace, 308+ tests incl. this crate's 130), `cargo build
  --quiet`, edge test (`graft::tests::it::dependency_edges`) all pass.
- No compaction.

## Read first

- `docs/design-notes/m2-details-and-diffs.md` — Interfaces (`graft-ui::syntax`, `graft-ui::diff_words`); Proposed change "word and syntax spans computed on sanitised text in ui"
- `PROMPT.md` Technology (lines 42–64: "syntect or tree-sitter-highlight, off-thread and cached"; word diff "imara-diff or similar") and §5 Diffs (lines 371–394: "stronger on changed words")
- `crates/graft-ui/MAP.md` — `theme` section (`Theme::token`, `TokenColor`, `Theme::color_depth`, `Token`, `Token::ALL`) and `paint` (`color`, `foreground`)
- `crates/graft-term/MAP.md` lines 71–72 (`SanitizedText`: what the input has already been through)

## Scope

In:
- `crates/graft-ui/src/syntax.rs` (`pub mod syntax`): `syntect` with features `default-syntaxes`, `regex-fancy` (no onig, no default themes); `SyntaxSet` loaded once (`OnceLock`). Scopes map to `SyntaxRole` by prefix: `comment` → Comment; `string` → String; `keyword`, `storage` → Keyword; `entity.name.type`, `support.type` → Type; `entity.name.function`, `support.function` → Function; `constant.numeric` → Number; `punctuation` → Punctuation; `meta.attribute`, `entity.other.attribute-name` → Attribute; else Plain. Adjacent equal roles merge. Spans are `char` indices into the line (the widget slices by char, never by byte).
- Caps: `lines.len() > 5_000` or any line `> 4_000` chars → every line `Plain` (documented on the fn). `highlight` never panics on any input.
- `Theme::syntax(role)` in `theme.rs`, from existing tokens only: Plain, Function → `text`; Comment, Punctuation, Attribute → `text-muted`; Keyword → `accent`; String, Number → `info` (if `Token::ALL` has no `info`, use `accent`); Type → `warning`. At `Palette16` and under `NO_COLOR` every role returns `Theme::token(Text)` (the sign column carries the meaning, m1-the-graph/12c rule).
- `crates/graft-ui/src/diff_words.rs` (`pub mod diff_words`): `word_spans` with `similar` (`TextDiff::from_words` per paired line); a pair with no common word gets no spans (the line tint is enough); spans are `char` ranges, adjacent spans merged.
- `Cargo.toml`: `syntect`, `similar` added to `graft-ui` only. Edge test untouched (ui still imports no git/ops).
- MAP.md lines for every public item.

Out: caching (the job caches the finished `DiffView`), tree-sitter, any widget, colour tuning by eye (`diff-widget`'s capture is where the user judges it).

## Signatures

```rust
// crates/graft-ui/src/syntax.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SyntaxRole { Plain, Comment, String, Keyword, Type, Function, Number, Punctuation, Attribute }
#[derive(Clone)] pub struct Language { /* private: the syntect syntax name */ }
pub fn language_for_path(path: &str) -> Option<Language>;              // extension, then exact file name (Makefile, Dockerfile)
pub fn language_for_first_line(line: &str) -> Option<Language>;        // shebang / modeline, for extension-less files
pub struct SyntaxSpan { pub range: std::ops::Range<usize>, pub role: SyntaxRole }   // char indices, half-open, ascending, non-overlapping
pub fn highlight(lang: &Language, lines: &[&str]) -> Vec<Vec<SyntaxSpan>>;         // one Vec per line; parser state carried across lines
impl Theme { pub fn syntax(&self, role: SyntaxRole) -> TokenColor; }

// crates/graft-ui/src/diff_words.rs
pub struct WordSpans { pub removed: Vec<Vec<std::ops::Range<usize>>>, pub added: Vec<Vec<std::ops::Range<usize>>> }   // per line, char ranges
pub fn word_spans(removed: &[&str], added: &[&str]) -> WordSpans;   // removed[i] vs added[i] for i < min(len); extra lines get empty Vecs
```

## Acceptance

- [ ] `language_for_path("src/auth/token.rs")` → Rust; `"Makefile"` → Makefile; `"x.unknownext"` → `None`; `language_for_first_line("#!/usr/bin/env python3")` → Python.
- [ ] `highlight(rust, ["// hi", "fn main() { let s = \"a\"; }"])`: line 0 one `Comment` span covering it all; line 1 has `Keyword` at `fn` and `let`, `Function` at `main`, `String` at `"a"`; every span within the line's char count.
- [ ] A block comment opened on line 0 and closed on line 2 marks all of line 1 `Comment`.
- [ ] 5,001 lines → all `Plain`; a 4,001-char line → `Plain` for that call; proptest over arbitrary `Vec<String>` (`proptest_cases`) never panics and never yields overlapping or out-of-range spans, including strings with combining marks and emoji.
- [ ] `word_spans(["let x = 1;"], ["let y = 1;"])` → removed `[[4..5]]`, added `[[4..5]]`; identical lines → empty spans; `word_spans(&[], &["a"])` → `added == [[]]`.
- [ ] `Theme::syntax` at `Palette16` and under `NO_COLOR` equals `Theme::token(Token::Text)` for every role; at truecolor `Comment` equals `text-muted`.
- [ ] Report: `graft-ui` clean build wall time before and after (one line; syntect's cost is a known trade).
- [ ] MAP.md: `syntax`, `diff_words` sections; `Theme::syntax` line.

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet` · edge test.

## Size

One crate, two new files (~220 lines) + `theme.rs` (~25 lines) + tests (~150). Fits: ≤ 18 tool calls. Don't read `graph_screen.rs`; nothing here draws.
