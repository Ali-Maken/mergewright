# ADR-0060 — At the Pixel tier, an open Ghost Preview draws the graph column at the Unicode tier

Date: 2026-10-07
Status: **Accepted**, 2026-10-07 (field notes :92, item 1: "your recommendation"). Spec
`.scratch/m8-polish-and-ship/issues/17b-fold-count-and-pixel-ghosts.md`, decision 2.
Amends: DESIGN §1.7 (`docs/design/01-view-mockups.md`), "dashed stroke in the real renderer".

## Context
- §1.7 asks for ghost nodes and edges drawn dashed and dimmed, real nodes solid, "one continuous picture, not two
  separate graphs". At the Unicode tier that is `┊`/`┈`; §1.7 left the Pixel tier to "the real renderer". PROMPT §1.5
  asks for "dashed or dimmed" ghost nodes, tier unspecified.
- At the Pixel tier the graph column is drawn as strips: images over 32-row aligned bands (DESIGN §3.8). A ghost row
  sits mid-strip. Drawing ghost rows as text and real rows as strips (ticket 17's decision 6) needs the strip punched
  around each ghost row, and a real lane crossing a ghost row is then half pixel, half text: two pictures, which §1.7
  rules out.
- Rasterising ghosts (a dashed, low-alpha stroke in the strip itself) puts the preview into the strip cache key and
  re-rasters every strip the preview touches. That is new raster work on the draw path in the milestone whose promise
  is "stays smooth on your biggest repo".

## Decision
1. While a preview with ghost rows is open and the tier in force is Pixel, the **whole graph column** draws at the
   Unicode tier: `view::graph_tier(session, state)` = Unicode then, else `tier_in_force` (name corrected 2026-10-07 after the 17b scout; decision unchanged). Every draw and hit-test path of the graph
   column reads `graph_tier()`.
2. The tier in force does not change: the header's `gfx` field and F3 keep reading `tier()`. F3 during the preview:
   `graph_tier()` = `min(tier(), Unicode)`.
3. On close the strips are placed again from the cache. No strip is rastered for the preview and none is evicted by it.
4. Pixel-rasterised ghosts (dashed, low-alpha strokes) move **past v1**. The conductor files the KNOWN_GAPS row
   `M8/pixel-ghosts-text` (asked for in 17b's NOTES).

## Why
- One rule, one picture: every lane on screen during a preview is drawn by one renderer, so ghost and real rows read
  as one graph (§1.7's own test). PROMPT §1.5's "dashed or dimmed" still holds: `┊`/`┈`, dimmed.
- The preview is modal and short; he confirms or cancels, then the pixel graph returns unchanged.
- No new raster work and no strip-cache key change on the scroll path (ADR-0054's pacing, the M8 promise).

## Trade-off
During a preview at Pixel the graph loses the pixel tier's smooth curves; it looks as it does in his daily Unicode
terminals (Zed, VS Code). Accepted by him 2026-10-07.

## Rejected
- **Text ghost rows over pixel real rows** (17's decision 6): punched strips; half-pixel, half-text lanes.
- **Rasterised ghosts in v1**: preview state in the strip cache key; re-rasters on open and close; budget pressure.
- **No graph during a preview at Pixel** (preview box only): the preview's point is the picture (PROMPT §1.5).
