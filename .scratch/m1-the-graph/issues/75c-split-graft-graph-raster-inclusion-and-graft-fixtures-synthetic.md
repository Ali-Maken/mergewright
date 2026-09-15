# m1-the-graph/75c — Review M1: split graft-graph raster, inclusion and graft-fixtures synthetic under the 400-line rule

Status: done
Build with: **Sonnet** (medium) — mechanical moves along existing seams; no behaviour change.
Blocked by: 75b

From the M1 review round 1, "M1" (PROMPT.md 6.2: files over 400 non-test lines). Split from 75: 70 and 71 each compacted on one or two big files, so one big file (or two small ones) per ticket. After the fix tickets so moves don't conflict with fixes.

## Read first
- docs/reviews/m1-the-graph.md "M1" only; PROMPT.md §6.1–6.2; ARCHITECTURE.md crate map section.
- The `codebase-design` skill (module changes).
- The crate MAP.md files for: crates/graft-graph/src/raster.rs (654), crates/graft-graph/src/inclusion.rs (436), crates/graft-fixtures synthetic.rs.

## Scope
In: split crates/graft-graph/src/raster.rs (654), crates/graft-graph/src/inclusion.rs (436), crates/graft-fixtures synthetic.rs, along existing concerns. Each resulting file ≤400 non-test lines, one concern per file. Pure moves plus the visibility changes they need; public API unchanged unless a MAP.md line must move. Update MAP.md, and ARCHITECTURE.md if structure changes. Before finishing, check the named files are ≤400 non-test lines (`wc`-based, tests excluded). Don't survey other files: the conductor holds the full list. Use `wc`/`grep` and a scout to find cut points; move code with shell tools rather than reading whole files into context.
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

## Result

Before (non-test lines):
- `crates/graft-graph/src/raster.rs`: 654
- `crates/graft-graph/src/inclusion.rs`: 436 (all non-test; `mod tests;` is external)
- `crates/graft-fixtures/src/synthetic.rs`: 678 (no test module)

After:
- `raster.rs` 300 (rasterise entry, types, constants); new `raster/pen.rs` 271 (`Pen`'s drawing methods — `pub(super)` on `new`/`column`/`row`); new `raster/geometry.rs` 119 (line/arc/paint/pixel helpers, `pub(super)`). `raster/node.rs`'s import of `eraser`/`paint` updated to `super::geometry::{eraser, paint}`.
- `inclusion.rs` 196 (`Follow`, `Inclusion`, `Frontier`, `seeds_for`/`apply`/`kept_parents`/`drawn_link`); new `inclusion/included.rs` 253 (`Included<S>` + `reinclude`), re-exported via `pub use included::{Included, reinclude};` so the public path is unchanged. `inclusion/tests.rs`'s `use super::*` lost the `CommitSource`/`Pull`/`StreamingLayout`/`Invalidated` imports that used to live in `inclusion.rs`; added them explicitly.
- `synthetic.rs` 340 (orchestration: `ensure`/`borrow`/`build`/etc., `Report`, `SyntheticError`, `git_error`/`io_error`); new `synthetic/scale.rs` 138 (`CommitGraph`, `CommitSize`, `Scale`; fields made `pub(super)` for the parent's field access); new `synthetic/history.rs` 219 (`History` + commit-message generators, `pub(super)` on the struct/`new`/`write`). All re-exported the same way (`pub use scale::{CommitGraph, CommitSize, Scale};`), so `lib.rs`'s `pub use synthetic::{CommitGraph, Scale};` is untouched.

No file in these three crates/paths is over 400 non-test lines afterward. No behaviour change, no public path change, no snapshot change.
