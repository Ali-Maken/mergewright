# m1-the-graph/24b — Rows for aligned pixel strips

Status: done
Build with: **Opus** (high) — history window and strip cache contract
Blocked by: 24 (partial: `graft-term::kitty` encoder done, commit f313456)
Leaves runnable: yes — nothing visible changes; the pixel tier can build full strips.

Split from 24 (no compaction; 105k tokens, remainder too big). 24's "Handoff" section is the spec: use its findings directly, don't re-derive them.

## Read first
- Ticket 24, "Handoff" section only
- `crates/graft-graph/MAP.md`, `crates/graft-ui/MAP.md`, `crates/graft-term/MAP.md`
- ADR-0016 (tier order)

## Scope
In:
- `StripCache::get` needs whole 32-row aligned strips; `History.window` holds only what the walk sent (~viewport). Make the pixel tier obtain rows padded to strip boundaries, carrying uncommitted virtual row, fold marker, thread emphasis and theme palette into `Strip`.
- Choose: pad the request in `History::show`, or rasterise short strips with an offset key. Pick the one that keeps the cache key honest; record the choice in the ticket and, if it changes a public item, MAP.md.
- Unicode/ASCII tiers must not pay for the padding.

Out:
- Drawing, replies, status (24c). Flush wiring, F3, real-terminal check (24d).

## Acceptance
- [x] Tests: a viewport starting mid-strip yields a full aligned strip; the last partial strip at end of history is handled; fold/emphasis/uncommitted row reach `Strip`.
- [x] Unicode tier requests unchanged (test or assertion).
- [x] Say what you verified and how.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Scouts for source. If you compact, say so; stop at a clean seam with a handoff note.

## Done (2026-09-13, builder-opus, no compaction)

**Choice: pad the request, not an offset key.** `History::strip_rows: Option<usize>` (pub field). When set, `wanted` widens both the coverage check and the request to whole 32-row strips plus the row above the first strip (its edges reach in). `StripCache::get` already refuses misaligned strips, and an offset key would give the same pixels two keys; padding keeps the key = the real row range. `None` (text tiers, the default from `History::started`) is the identity: requests unchanged.

Built:
- `graft::view::strips` (new submodule of `view`): `pixel_rows(state, max_lanes, strip_rows) -> PixelRows { strips, lane_columns, marker }`; `StripRows { first_row, row_count(), uncommitted_at(), with_strip(frame, paint, |&Strip| ..) }`; `StripPaint::of(session)`.
  - One `Fold::plan` across all strips drawn (lanes line up at strip seams); pinned as the text tier: integration lanes + thread columns.
  - Ring: HEAD > stash > `Ring::of`. Emphasis: cursor Highlighted; Focus + unrelated Dimmed. Thread remapped real→drawn columns.
  - Uncommitted row inserted above HEAD's row in the strip holding `top`; own strip on an unborn branch.
  - A strip whose rows or row above are not in the window is left out (transient between scroll and answer).
- `StripCache::get`: a `Ring::Uncommitted` row does not count toward the strip height (strip with it = 33 rows, still aligned; keys apart).
- `Theme::raster_lanes` / `raster_muted`: truecolor hues at any depth, one muted hue when colour is off.

Verified (tests only; nothing drawn, nothing wired):
- `history/tests.rs`: text tier asks `top-h..bottom+h` exactly as before and `strip_rows` defaults to `None`; mid-strip viewport asks `0..96` / `63..160` and is not covered by viewport-only or strip-without-above windows; last partial strip at end of history covered by the rows there.
- `view/strips/tests.rs`: mid-strip viewport → `(32,32),(64,32)`; end of history → `(64,32),(96,4)`; missing row-above strip left out; uncommitted row at index 0 over a Head ring; fold marker reaches `Strip` and expanded removes it; cursor Highlighted, thread lit on cursor lane, Focus dims chain off the thread. Every strip goes through a real `StripCache::get` + `rasterise`.
- `strip_cache.rs`: 32 rows + virtual row accepted (frame 33 rows); 33 + virtual refused.
- Not run: `code-review` skill (skipped to keep the ticket small); no pixel output looked at.

Notes for 24c / 24d:
- **Nothing sets `History::strip_rows` yet.** 24d must set it to `STRIP_ROWS` when the settled tier (renderer override else session tier) is Pixel, at `History::started` in `update.rs` and when F3 leaves Pixel (back to `None`, then `show`).
- 24c screen mapping: history row `h` sits at strip index `h - first_row`, +1 if `uncommitted_at()` is at or before it; screen row 0 is the uncommitted row when present.
- Fold spans the strips drawn, so scrolling into a new strip can change the fold of the others (re-rasterise); a selection move changes thread/emphasis of its strips (miss). Both honest misses; measure in 24d.
