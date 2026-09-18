# m1-the-graph/07 — History streams into the layout engine

Status: done (2026-09-13)
Build with: **Opus** (high) — concurrency: a background walk feeding a layout between frames
Blocked by: 06, 31
Leaves runnable: yes — the status screen shows rows laid out so far and the history state.

Why: 06 built `StreamingLayout` and the `CommitSource` seam, but the product has no source, no seed builder, and `app::history::read` still drains the whole walk in one lump. Every screen after this reads from here.

## Read first
- docs/design/03-graph-layout-algorithm.md §3.6 and docs/design/08-architecture.md §8.3 only
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
- [x] On corpus fixtures, rows from the product source equal rows from the test source (ids, lanes, edges).
- [x] A chunk lands in `AppState` before the walk finishes — asserted with a source that blocks after its first chunk.
- [x] `graph.default_branch = "x"` puts `x` in lane 0; an invalid value is a config error with file and line.
- [x] `Esc` mid-walk gives `history: cancelled`, and no later chunk is applied.
- [x] `cargo run -- <fixture>` under a pty shows the count reach the fixture's commit count; record what you saw.

## Verified
- Product source vs layout: `history::tests::the_product_source_lays_out_every_shape_as_the_whole_layout_does` — every corpus shape, rows (ids, lanes, edges) equal `layout(all commits, seeds)`, the same oracle the graph crate's test source is held to; `chunks_rebuilt == 0`.
- Chunk before walk ends: `history::tests::a_chunk_lands_in_app_state_before_the_walk_finishes` (gated source blocks after chunk 1; `AppState` has 8 rows, phase streaming). Also `jobs::tests::a_history_job_sends_its_rows_before_it_ends`.
- Default branch: `history::tests::a_configured_default_branch_takes_lane_0`; `config::tests::a_default_branch_git_would_refuse_names_the_file_and_the_line`.
- Esc: `update::tests::no_chunk_lands_after_esc`, `history::tests::nothing_is_reported_once_stop_is_asked`.
- Pty (`script`, 100x30, xterm-256color), status screen History row:
  - linear fixture (200 commits): streaming, 0 -> complete, 200 rows laid out.
  - synthetic 20,000: streaming 500, 1000, 1500 ... -> complete, 20000 rows laid out.
  - synthetic 100,000, Esc at 1.0 s: streaming 22000 -> cancelled, 22000 rows laid out; count stayed at 22000 until quit. (Esc at 1.2 s on 20k: walk already complete.)

## Notes
- Builder context compacted once mid-ticket (after pty runs, before gates and commit). Nothing remains.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.
