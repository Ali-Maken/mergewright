# m1-the-graph/22 — The pixel graph rasteriser

Status: done
Build with: **Opus** (high) — drawing, not algorithm design: the spike (ADR-0012) settled the approach; anti-aliased lines, Béziers and rings are standard coverage techniques over lane geometry 05 already produces
Blocked by: 05
Leaves runnable: yes — pure `graph` code with buffer snapshots; nothing reaches the terminal until 24.

Why: the spike drew chunky straight segments. The pixel tier is only worth having if it looks clearly better than Unicode.

**Unproven:** the spike only drew straight lines. Anti-aliasing and curves have never been drawn in this project; buffer snapshots prove the pixels are deterministic, not that they look good. **The user judges them by eye at the M1 checkpoint** — no builder or reviewer ticks that.

## Read first
- PROMPT.md §1.1 (the pixel tier paragraph) and §7.2 item 3
- docs/design/03-graph-layout-algorithm.md §3.8 (the paragraphs about `RasterFrame`, not the cache)
- ADR-0003 (transparency) and ADR-0012 (spike findings only)
- Code: public API of `crates/graft-graph/src/layout.rs`; grep `RasterFrame` for any existing type; 14's fold description

## Scope
In:
- A pure rasteriser in `graph`: a row range, cell size and lane colours in, RGBA out. No I/O, no cache.
- Anti-aliased lane lines, Bézier curves where branches fork and merge, circular nodes with a distinct ring per commit kind.
- Real alpha: never paint the terminal background.
- The `+N` column and the paint attribute (dimmed / highlighted).

Out:
- The strip cache (23). Any escape-sequence encoding (24, 25, 29).

## Acceptance
- [x] RGBA buffer snapshots at a pinned 8×16 cell for criss-cross, octopus and foxtrot.
- [x] A pixel outside every line and node has alpha 0 (asserted on the buffer).
- [x] Commit kinds differ in ring shape, asserted on the buffer independent of colour.
- [x] Render time for a 32-row strip at a 17×37 cell (the user's measured size) reported with the machine.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.

## Notes (builder, 2026-09-13)
- API: `graft_graph::raster::rasterise(&Strip) -> Result<core::raster::RasterFrame, RasterError>`. `RasterFrame` added to `core` (docs/design/08-architecture.md §8.2). `tiny-skia` 0.12, `simd` off so snapshot bytes do not depend on the CPU's code path.
- Snapshots: `tests/it/snapshots/it__raster__raster-{foxtrot,criss-cross,octopus}-merge.snap`, alpha/slot text art at 8×16.
- Curves proven on pixels: `a_fork_is_an_antialiased_curve_not_an_elbow` (arc centroid moves monotonically across ≥3 x positions; partial alpha present).
- Render time: 32 rows × 5 lanes at 17×37, Apple M5, 24 GB: median 1.38 ms release, 13.4 ms debug.
- Low colour depth dashing (13): n/a — an image carries full RGB; no pixel dashing.
- Emphasis (16): built now — `Emphasis` per row plus `thread` columns (on-thread heavier, off-thread faded), so 24 only wires `Ancestry::lit`.
- Shots for the user's eye: `docs/shots/22-raster-{foxtrot,criss-cross,octopus}-merge{,-on-dark}.png`, `22-raster-rings-on-dark.png` (17×37). Regenerate: `cargo test -p graft-graph --test it write_shots -- --ignored`.
- `+N`: drawn with a built-in 3×5 bitmap font on a cleared backing; pass-through verticals are not drawn in the marker column.
- No compaction.
