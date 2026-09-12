# m1-the-graph/13 — The pixel graph: rasteriser and strip cache

Status: ready
Build with: Opus
Blocked by: 10

Why: the pixel tier is what makes Graft look unlike every other terminal Git client, and the spike proved the approach works in Ghostty and iTerm2. Drawing and protocol encoding are deliberately two crates (`graph` rasterises, `term` encodes — ARCHITECTURE.md); this ticket is the drawing half.

## Read first
- PROMPT.md §1.1 Tier 1 (anti-aliasing, curves, transparency, cell size, alignment, caching)
- DESIGN.md §3.8 (strip cache, the key, the layout hash, what invalidates and what does not)
- ADR-0003 (transparent graph rendering — never paint the background) and ADR-0012, "What M1 must do differently"
- KNOWN_GAPS.md prototype row "Resize frame busts the frame budget"

## Goal
`graph` produces `RasterFrame` strips: anti-aliased, transparent, cached, and cheap enough that a scroll frame rasterises at most one new strip.

## Scope
In:
- The rasteriser: anti-aliased lane lines, Bézier curves where branches fork and merge, circular nodes with distinct ring styles per commit kind. The spike's chunky straight segments are the thing to beat.
- Real alpha; never paint the terminal background into a strip (ADR-0003) — a translucent terminal must stay translucent.
- Fixed-height strips aligned to multiples of the strip height (start at 32 rows), so a scrolled viewport re-hits existing strips instead of missing on every offset.
- Bounded LRU keyed exactly on `(strip row range, layout hash, theme id, cell size)`, renderer-private state.
- The layout hash derived from rows the renderer already consumes (commit id, lane, incoming, outgoing, marker, plus pass-through lanes and their identities). No new field on `Row`.
- Invalidation per §3.8: implicit on content change; explicit flush on theme change, cell-size change, re-detection and resize. **Scrolling is never an invalidation.**
- Resize must not mean "retransmit the whole viewport": the spike measured 32–41 ms for that against a 16 ms budget. Re-key and rebuild only what actually changed.

Out:
- Escape-sequence encoding, Kitty/iTerm2/Sixel, tmux (ticket 14).
- Sixel rasterisation specifics if ticket 03's ADR left Sixel out.

## Acceptance
- [ ] Snapshot the **rasterised RGBA buffer at a pinned synthetic cell size (8×16)** per PROMPT.md §7.2 item 3 — never a PNG from a real terminal.
- [ ] Alpha proven: a pixel outside a line is fully transparent, asserted on the buffer.
- [ ] Cache behaviour tested directly: scrolling one row rasterises at most one new strip; scrolling back hits the cache; a theme change flushes; a layout change to one row invalidates only its strip.
- [ ] Strip height is a measured choice, not a guess — report render time per strip at a realistic cell size.
- [ ] Cache memory is bounded; report the resident bytes for a full viewport plus buffer.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`
