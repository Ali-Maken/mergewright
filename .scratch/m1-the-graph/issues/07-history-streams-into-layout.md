# m1-the-graph/07 — History streams into the layout engine

Status: ready
Build with: **Opus** (high) — concurrency: a background walk feeding a layout between frames
Blocked by: 06, 31
Leaves runnable: yes — the status screen shows rows laid out so far and the history state.

Why: 06 built `StreamingLayout` and the `CommitSource` seam, but the product has no source, no seed builder, and `app::history::read` still drains the whole walk in one lump. Every screen after this reads from here.

## Read first
- DESIGN.md §3.6 and §8.3 only
- ADR-0001 (default-branch detection, rule 1) and ADR-0008
- KNOWN_GAPS.md M1 rows: "History arrives in one lump, not a stream", "`graph.default_branch` is not a config key", "The app's seed builder does not exist yet", "`CommitSource` has no implementation in the product"
- Code: `crates/graft-graph/src/layout/source.rs`, the public API of `layout/stream.rs`, `crates/graft-git/src/history/mod.rs`, the app's history module in `crates/graft/src/`, and `crates/graft-graph/tests/it/streaming.rs` (the test source to mirror)

## Scope
In:
- A product `CommitSource` over `graft-git`'s `Walk`, placed where ARCHITECTURE.md's dependency rules allow (`graph` must not depend on `git`).
- The seed builder: default branch in lane 0, integration branches in reserved lanes — the same choice the corpus snapshots assume.
- `graph.default_branch` config key, parsed with M0's no-silent-default rules; `Branches::resolve` finally receives it.
- Replace the one-lump read: chunks are laid out as they arrive and handed to the UI thread between frames. `Esc` still cancels.
- Status screen shows `rows laid out: N` and `history: streaming | complete | cancelled | failed: <reason>`.

Out:
- Drawing the graph (08, 09). Promotion and the provisional-order notice (18). Budgets (27, 30).

## Acceptance
- [ ] On corpus fixtures, rows from the product source equal rows from the test source (ids, lanes, edges).
- [ ] A chunk lands in `AppState` before the walk finishes — asserted with a source that blocks after its first chunk.
- [ ] `graph.default_branch = "x"` puts `x` in lane 0; an invalid value is a config error with file and line.
- [ ] `Esc` mid-walk gives `history: cancelled`, and no later chunk is applied.
- [ ] `cargo run -- <fixture>` under a pty shows the count reach the fixture's commit count; record what you saw.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.
