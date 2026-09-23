# fix-resolver-line-numbers: both sides numbered by the file on disk; no number on any row that stands for a marker

Status: draft
Build with: set by the conductor after a scout (brownfield); expected **Sonnet** — the rule is fully stated here and one property test proves it against `render()`.
Needs: none.
Crates: graft-git, graft-ui, graft (`view::resolver` only).

## Read first
- `docs/design-notes/m6-conflicts.md` — Amendment: Need, Prior art rows "Line numbers" and "Number on the marker", Interfaces (`SideLines`, `SideNumbers`), Hard choice 2, New UI (the card mockup).
- `crates/graft-ui/MAP.md` §`conflict_resolver` (the "Line numbers:" sentences — what is drawn today); `crates/graft-git/MAP.md` §conflict (`render`, `blocks`, `BlockAt::first_line`, `attach_base`); `crates/graft/MAP.md` §`update::resolver` (where `DocView` is built).
- `docs/field-notes.md` 2026-09-23 (first entry); `docs/research/conflict-resolution.md` §2.3.
- Scout questions (answer before building): which code builds `BlockView` from `ConflictFile`; does `render()` write an attached base (from `attach_base`) or only a base parsed from diff3/zdiff3 markers; where `draw_header` draws its number.

## Scope
In:
1. `ConflictFile::side_lines(block)` in graft-git: for an **Open** block, the 1-based line in `render()` of the first OURS line, the first BASE line (only when `render()` writes the base section), the first THEIRS line; `None` for a side with no lines or no bytes on disk; `None` overall for a Resolved block or an index past the end.
2. graft-ui: `BlockView.numbers: SideNumbers`. Drawing rule, at every width and tier: every content line whose bytes are on disk draws its disk line number (side's first number + i); THEIRS numbered exactly like OURS (≥100: its half gets the same gutter width; <100: stacked rows numbered); BASE numbered only when `numbers.base` is `Some`; **`Header`, `Labels` and `End` rows draw no number**. A Took block keeps today's numbering of its taken lines.
3. graft `view::resolver`: fill `numbers` from `side_lines`.

Out: any change to `BlockAt::first_line`'s meaning; plain-line numbering (already the disk's); line numbers anywhere but the resolver.

## Signatures
```rust
// graft-git/src/conflict.rs
pub struct SideLines { pub ours: Option<usize>, pub base: Option<usize>, pub theirs: Option<usize> }
impl ConflictFile { pub fn side_lines(&self, block: usize) -> Option<SideLines>; }
// graft-ui/src/conflict_resolver.rs
pub struct SideNumbers { pub ours: Option<usize>, pub base: Option<usize>, pub theirs: Option<usize> }
pub struct BlockView { /* existing fields */ pub numbers: SideNumbers }
```

## Acceptance (name the verifying test at tick time)
- [ ] graft-git `conflict::tests::side_lines_point_at_their_own_bytes` — over every `StoppedMerge`/`StoppedRebase` fixture file and a fuzz-style generated set: for each `Some(n)`, line `n` of `render()` (split keeping terminators) equals that side's first `Line` bytes; a zdiff3 block's `base` is `Some`; after `attach_base` on a diff3-less file, `base` is `None` unless `render()` now writes it.
- [ ] graft-ui `conflict_resolver::tests::no_marker_row_draws_a_number` — `Header`, `Labels`, `End` rows at 60, 80, 99, 100, 120, 200 columns, Unicode and ASCII: no digit in the gutter cells.
- [ ] graft-ui `…::theirs_is_numbered_at_every_width` — THEIRS's first line draws `numbers.theirs` paired (≥100) and stacked (<100).
- [ ] Capture `docs/shots/m6/fix-line-numbers-120x40.txt` and `…-80x24.txt`: the resolver on `three.txt`. **What right looks like:** the header row reads `── conflict 1 of 3 · open ──` with blank gutter (no `2`); the first pair row reads `3 one-ours` on the left and `5 one-theirs` on the right (80x24: `5 one-theirs` on its own stacked row); the next plain line reads `7`. User judges by eye (rule 6).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
Small: one fn + one field + one draw rule + three tests + two captures. One context.
