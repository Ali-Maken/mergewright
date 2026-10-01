# ADR-0053: The selection's paint is its own image layer; strips key on layout only

Status: accepted (planner, 2026-10-01, M7 checkpoint fix "scroll lag"). Keeps ADR-0024's look; changes how it is
sent. Restores DESIGN.md §3.8's key, which never named the selection.

## Context

Field note 2026-10-01: on his work repo (17 694 commits, 1 323 refs) held `j` lagged and kept moving after he let go;
`J` opened late. Diagnosis `.scratch/m7-power-tools/debug-scroll-lag.md`: at the Pixel tier the selection's thread is
part of the strip cache key (`crates/graft-graph/src/raster/cache.rs:295-296`), so on a branchy history every `j`
misses every viewport strip, rasterises it again and re-sends it uncompressed: 6.35 MB per `j` frame, 169 strip
transmits per 120 `j` (`docs/perf/scroll-lag-big-repo.txt:26`). Thread left out of the key (probe): 0.38 MB.

DESIGN.md §3.8 says the key is `(row range, layout hash, theme id, cell size)` and "scrolling is not an
invalidation". Moving the cursor became one when ADR-0024 made the thread an alpha inside the strip.

## Decision

1. **Strips are drawn from layout only.** A strip's pixels and key depend on its rows' layout (commit, lanes, edges,
   ring, stubs), theme, cell size and one **backdrop** alpha. Nothing that follows the cursor (thread columns, Focus
   fading) is in the strip, so no cursor move can miss.
2. **Backdrop** is the lowest alpha in play: `Full` (1.0, no thread on screen), `OffThread` (`OFF_THREAD_ALPHA`),
   `Faded` (`FADED_ALPHA`, Focus mode). Every element of the strip is drawn at it. It changes only when Focus toggles
   or the thread comes or goes, never per `j`.
3. **Emphasis layer.** Per strip, a second image, cropped to the cell columns it touches, holds every element whose
   ADR-0024 alpha `a` is above the backdrop `b`, drawn at `(a − b) / (1 − b)`. Composited over the strip
   (premultiplied "over") it gives alpha `a` again: the look ADR-0024 decided, unchanged. Cached on the strip's key
   plus a hash of the selection's paint. Placed one z-step above its strip.
4. **Kitty sends zlib (`o=z`)** when the probe found the terminal takes it. Both layers are mostly transparent.
5. **iTerm2** cannot layer images in a cell: Graft composites the two on the CPU and sends one band, as today.

## Why not the alternatives

- **Compression alone:** still rasterises ~3 strips per `j` (draw p50 5.7 ms, `docs/perf/scroll-lag-big-repo.txt:26`)
  and re-sends all of them. Treats the bytes, keeps the cause.
- **No thread at the Pixel tier, or only the cursor's own lane lit:** reverses ADR-0024 and DESIGN.md §5.2, a change he
  would see.
- **Crop the lit cells out of an all-lit copy:** lines and curves cross cells; a cell crop takes other lines with it.
- **One emphasis image per row:** 40 placements a frame, and a move onto another lane changes most rows' thread anyway.
- Prior art (`docs/research/big-repo-scroll.md`, gitui#2823): a value that changes every tick must not be in a cache
  key. The selection is that value here.

## Consequences

- Two placements per strip on Kitty; `KittyImages` capacity counts both layers.
- Edge pixels where a lit stroke's anti-aliasing overlaps the backdrop copy differ from the single-pass raster by at
  most `b × 255` per channel; where a lit line crosses an unlit one, the lit line is now always on top. Pinned by the
  equivalence test in `fix-scroll-emphasis-layer`. Colour stays **unverified** (CLAUDE.md rule 5); the user judges the
  PNG captures.
- DESIGN.md §3.8 gains one paragraph naming the backdrop and the emphasis layer (acceptance ticket).
- Idle prefetch rasterises strips only; emphasis waits for the cursor.
