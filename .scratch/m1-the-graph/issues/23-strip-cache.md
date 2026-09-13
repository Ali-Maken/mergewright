# m1-the-graph/23 — The strip cache

Status: done
Build with: **Opus** (high) — cache keying and invalidation rules
Blocked by: 21, 22
Leaves runnable: yes — cache with direct tests; still no pixels on screen until 24.

Why: a scroll frame must rasterise at most one new strip, or the 16 ms budget is gone.

## Read first
- DESIGN.md §3.8 (all of it)
- Code: 22's rasteriser API, 21's resize/theme events

## Scope
In:
- Fixed-height strips aligned to multiples of the strip height (start at 32 rows; choose the final height by measurement).
- A bounded LRU keyed exactly on (strip row range, layout hash, theme id, cell size).
- The layout hash derived from what the rasteriser consumes. No new field on `Row`.
- Invalidation: implicit on content change; explicit flush on theme change, cell-size change, re-detection and resize. Scrolling never invalidates.

Out:
- Encoding and placement (24, 25).

## Acceptance
- [x] Scrolling one row rasterises at most one new strip; scrolling back is a hit.
- [x] A theme change flushes; a layout change on one row invalidates only its strip.
- [x] Resident bytes for a full viewport plus buffer reported.
- [x] The strip height choice shown with the numbers that decided it.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.

## Result (builder, Opus)
- `graft-graph/src/raster/cache.rs`: `StripCache`, tests in `tests/it/strip_cache.rs`.
- Key: (first_row, rows, layout hash, theme id = hash(palette, muted), cell w×h). Layout hash per row: commit id, lane, incoming, outgoing columns, lanes, Mistake Radar marker, ring, emphasis, thread; plus row above's edges, lane_columns, `+N` marker.
- Invalidation: content/fold/selection change → new hash, old strip ages out of LRU. `flush(Flush::{Theme,CellSize,Redetection,Resize})` clears all. Scrolling never flushes (tested).
- Strip height, release, Apple M5, 17×37 cell, 12 lanes (`strip_height_numbers`, ignored tool):
  | rows | rasterise | key hash | bytes |
  |---|---|---|---|
  | 8 | 446 µs | 1.0 µs | 471 KiB |
  | 16 | 796 µs | 2.1 µs | 943 KiB |
  | 32 | 1360 µs | 3.2 µs | 1887 KiB |
  | 64 | 1736 µs | 4.1 µs | 3774 KiB |
  | 128 | 2653 µs | 7.2 µs | 7548 KiB |
  Chose 32: miss < 1/10 of frame; 64 doubles encode payload for little time saved.
- Resident: `capacity_for(32, 60-row viewport, 32 buffer)` = 5 strips → 5 × 1887 KiB ≈ 9.2 MiB at 12 lanes (test prints 1572 KiB for its 2-lane history).
- Not wired to 21's events yet → KNOWN_GAPS, ticket 24.
