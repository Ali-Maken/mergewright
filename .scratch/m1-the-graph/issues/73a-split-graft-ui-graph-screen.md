# m1-the-graph/73a — Review M1: split graft-ui graph_screen under the 400-line rule

Status: done
Build with: **Sonnet** (medium) — mechanical moves along existing seams; no behaviour change.
Blocked by: 72c

From the M1 review round 1, "M1" (PROMPT.md 6.2: files over 400 non-test lines). Split from 73: 70 and 71 each compacted on one or two big files, so one big file (or two small ones) per ticket. After the fix tickets so moves don't conflict with fixes.

## Read first
- docs/reviews/m1-the-graph.md "M1" only; PROMPT.md §6.1–6.2; ARCHITECTURE.md crate map section.
- The `codebase-design` skill (module changes).
- The crate MAP.md files for: crates/graft-ui/src/graph_screen/mod.rs (~1,130).

## Scope
In: split crates/graft-ui/src/graph_screen/mod.rs (~1,130), along existing concerns. Each resulting file ≤400 non-test lines, one concern per file. Pure moves plus the visibility changes they need; public API unchanged unless a MAP.md line must move. Update MAP.md, and ARCHITECTURE.md if structure changes. Before finishing, check the named files are ≤400 non-test lines (`wc`-based, tests excluded). Don't survey other files: the conductor holds the full list. Use `wc`/`grep` and a scout to find cut points; move code with shell tools rather than reading whole files into context.
Out: behaviour changes, renames of public items, anything outside the named files' crates.

## Acceptance
- [ ] Named files split; each new file ≤400 non-test lines.
- [ ] No test changed except import paths; all gates green; no snapshot changes.
- [ ] MAP.md (and ARCHITECTURE.md if needed) updated.
- [ ] Report: before/after line counts; any file still over 400 in these crates.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`

## Size
One context, under ~80k tokens. Near it, stop at a clean seam with a handoff note.

## Report
Split `graph_screen/mod.rs` (1130 lines) into:
- `mod.rs` (331): plain-data types (`GraphView`, `Filters`, `Compaction`, `GraphEntry`,
  `AuthorChip`, `OrderNotice`, `WalkView`, `SearchLine`, `CommitGraphLine`, `GraphScreen`
  struct + `new`), module wiring, re-exports.
- `layout.rs` (291): geometry — header composition, `body_layout`/`graph_area`, lane-count
  math (`lanes_that_fit`, `max_lanes_for`, breakpoints), `Columns` layout struct.
- `footer.rs` (230): footer text — `footer_line`, `commit_graph_line`, key hints, the
  commit-graph offer's wording.
- `render.rs` (329): drawing — `impl Widget`, row/badge drawing, `draw_cell`.

All four ≤400 non-test lines (`tests.rs` unchanged at 921, only its imports touched: added
explicit `use`s for items that moved out of `mod.rs`'s glob scope — `super::layout::walk_text`,
`super::footer::{HINTS, key_hints, commit_graph_segments, grouped}`, plus `Rect`, `Modifier`,
`UnicodeWidthStr`, `RowPaint`, `GraphColumn`, `color`, `Token`, `Theme` that used to arrive via
`mod.rs`'s top-level `use` block and `use super::*`).

`lanes_that_fit`/`max_lanes_for` moved into the private `layout` module but are re-exported
from `mod.rs` (`pub use layout::{lanes_that_fit, max_lanes_for};`) since
`graft_ui::graph_screen::max_lanes_for` is called from `crates/graft/src/session.rs` —
public API path unchanged. No other public item moved. `Columns` and the cross-file free
functions (`graph_width`, `subject_truncated`, etc.) are `pub(super)`.

No behaviour change, no snapshot changes (module path for tests unchanged), no MAP.md/
ARCHITECTURE.md edit needed (no public path changed).

Gates: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo clippy --all-targets --features heavy-tests -- -D warnings`, `cargo test` — all green.
