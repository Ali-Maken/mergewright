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

## Amendment 2026-10-10 (m8-polish-and-ship 05 Part 2): emphasis in per-column pieces

**Decision 3 now reads:** per strip, the emphasis is split into **pieces, one per drawn lane column**. A piece holds
exactly the elements whose ADR-0024 alpha is decided in that column, drawn alone at `(a − b) / (1 − b)` and cropped to
the whole cells it touches. Ownership: an edge (straight or arc) belongs to the column it is drawn in; a row's node,
lane stubs and tail to the row's own lane; the rail to the marker column. **Piece key** = hash(strip backdrop key,
column, (row, element, alpha) of each owned element), nothing else. On Kitty a piece is rasterised only when
`KittyImages::holds` says the terminal lacks its key; it is sent, its pixels dropped, and placed by id from then on.
Unplaced uploads stay in the terminal up to 64 MiB of decoded bytes, least recently placed freed first ("the quota in
kitty is 320MB per buffer", sw.kovidgoyal.net/kitty/graphics-protocol, "Image persistence and storage quotas"). iTerm2
keeps one composited band per strip (Decision 5), from the whole-strip emphasis.

**Why:** one emphasis image per strip was keyed on the whole strip's paint, so any lit change anywhere in 32 rows × full
width made a new picture, and his trace showed it: 82 % of the run's bytes were emphasis
(`docs/perf/m8-trace-followup.txt`, before). Re-placing by id could not help while each `j` made new pixels; the key
was the fix. Gate before wiring: on the 50-key walk at 222×70 on MONOREPO the changed pieces' area was 18 % of today's
emphasis area (`piece_gate::pieces_localise_on_his_walk`).

**"One emphasis image per row" stays rejected**, and pieces differ from it: a row image was re-made whenever its
row's thread changed, which a move onto another lane does to most rows; a piece is keyed by content, so one the cursor
leaves and comes back to is placed again, never re-sent. A frame places more images (one per lit column per strip),
but a placement is a few dozen bytes.

**Overlaps (finding, replaces the `b × 255` bound below for Kitty):** the whole pass draws a row's node, stubs and tail
above every column's edges; pieces stack by column. Where two pieces paint one pixel the alpha matches the whole pass
(within 2: premultiplied "over" is order-free in alpha), but which lit colour is on top may differ. On the test's
branchy window 53 of 6 030 overlapped pixels differ by more than `b × 255` in a colour channel
(`raster_pieces::pieces_composite_equals_single_emphasis`, which counts them). Colour is **unverified** (CLAUDE.md
rule 5): the user judges `docs/shots/m8-05-emphasis-pieces.png`.

**Consequences changed:** `KittyImages` is budgeted in decoded bytes of unplaced images, not a count of images; a
frame places one strip plus its pieces per strip. Emphasis on Kitty is not cached Graft-side (no `RasterPool` buffer,
no LRU entry); the strip cache's emphasis entries serve iTerm2 only.

## Consequences

- Two placements per strip on Kitty; `KittyImages` capacity counts both layers.
- The emphasis layer adds ~55% to a strip's pixels (~4.3 MB backdrop + ~2.4 MB emphasis at 17×37 on MONOREPO), and
  since m8 04's buffer pool every cached entry holds a full-strip buffer, so an emphasis image weighs as much as its
  strip (5.6 MB + 5.6 MB measured). The strip cache's layers share one byte budget, not a count each (m8 04, DESIGN
  §3.8; numbers in `docs/perf/scroll-pixel-memory-budget.txt`).
- Edge pixels where a lit stroke's anti-aliasing overlaps the backdrop copy differ from the single-pass raster by at
  most `b × 255` per channel; where a lit line crosses an unlit one, the lit line is now always on top. Pinned by the
  equivalence test in `fix-scroll-emphasis-layer`. Colour stays **unverified** (CLAUDE.md rule 5); the user judges the
  PNG captures.
- DESIGN.md §3.8 gains one paragraph naming the backdrop and the emphasis layer (acceptance ticket).
- Idle prefetch rasterises strips only; emphasis waits for the cursor.
