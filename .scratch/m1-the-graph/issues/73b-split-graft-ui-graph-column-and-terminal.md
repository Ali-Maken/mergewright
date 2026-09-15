# m1-the-graph/73b — Review M1: split graft-ui graph_column and terminal under the 400-line rule

Status: done

## Report

- `crates/graft-ui/src/graph_column/mod.rs`: 583 → 325 non-test lines. Split
  off two new private submodules: `geometry.rs` (192 lines: `Cell`, `Owner`,
  `BLANK`, `row_cells`, `Geometry`, `Sides` — the per-row edge geometry) and
  `style.rs` (87 lines: `lane_x`, `off_thread`, `on_thread`,
  `dashes_odd_columns`, `dashed`, `to_ascii` — colour/dash/ASCII-tier
  helpers). `mod.rs` keeps the module doc, public types (`NodeKind`,
  `RowPaint`, `GraphLine`, `GraphColumn`) and the `Widget` impl. Cross-file
  items marked `pub(super)` (including `Cell`/`Owner` fields, since `render`
  reads them directly); public API unchanged, so `graft-ui/MAP.md` needed no
  edit. `tests.rs` needed one added import (`BranchIdentity`, previously
  reached transitively via `mod.rs`'s own `use`).
- `crates/graft/src/terminal.rs`: checked and **not split**. Total file is
  591 lines (what the M1 review's "400–600" bucket flagged), but its tests
  are one inline `#[cfg(test)] mod tests { ... }` block; non-test lines
  (before that block) are 331 — already under the 400 non-test-line bar this
  ticket's acceptance criteria and PROMPT.md 6.2 actually set. Splitting it
  anyway would be scope creep with no rule violation to fix, so left as is.
- Gates: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo clippy --all-targets --features heavy-tests -- -D warnings`,
  `cargo test` all green. No test changed except the one import; no snapshot
  diffs (`git status` clean of `.snap` files).
Build with: **Sonnet** (medium) — mechanical moves along existing seams; no behaviour change.
Blocked by: 73a

From the M1 review round 1, "M1" (PROMPT.md 6.2: files over 400 non-test lines). Split from 73: 70 and 71 each compacted on one or two big files, so one big file (or two small ones) per ticket. After the fix tickets so moves don't conflict with fixes.

## Read first
- docs/reviews/m1-the-graph.md "M1" only; PROMPT.md §6.1–6.2; ARCHITECTURE.md crate map section.
- The `codebase-design` skill (module changes).
- The crate MAP.md files for: crates/graft-ui graph_column/mod.rs (583), terminal.rs.

## Scope
In: split crates/graft-ui graph_column/mod.rs (583), terminal.rs, along existing concerns. Each resulting file ≤400 non-test lines, one concern per file. Pure moves plus the visibility changes they need; public API unchanged unless a MAP.md line must move. Update MAP.md, and ARCHITECTURE.md if structure changes. Before finishing, check the named files are ≤400 non-test lines (`wc`-based, tests excluded). Don't survey other files: the conductor holds the full list. Use `wc`/`grep` and a scout to find cut points; move code with shell tools rather than reading whole files into context.
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
