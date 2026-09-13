# m1-the-graph/23 — The strip cache

Status: ready
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
- [ ] Scrolling one row rasterises at most one new strip; scrolling back is a hit.
- [ ] A theme change flushes; a layout change on one row invalidates only its strip.
- [ ] Resident bytes for a full viewport plus buffer reported.
- [ ] The strip height choice shown with the numbers that decided it.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.
