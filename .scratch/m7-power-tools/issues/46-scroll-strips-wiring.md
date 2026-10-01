# 46-scroll-strips-wiring — the app draws backdrop + emphasis; a `j` never re-sends a strip

Status: ready
Build with: (blank: scout first — `view::strips`, `event_loop::graphics` are existing code)
Needs: `44-scroll-emphasis-layer` (`Backdrop`, `PlainRow<'a>`, `PlainStrip<'a>`, `Lit<'a>`, `Selection<'a>`,
`Emphasised`, `StripCache::{backdrop, emphasis, composited, contains_backdrop}`, `composite`);
`45-scroll-kitty-layer-deflate` (`Layer`, `EMPHASIS_ID_BIT`, `Place::layer`, `KittyImages::{deflating,
is_deflating}`, `Capabilities::kitty_deflate`; and its Step 0 table with no "no" in it);
`43-scroll-coalesce-input` merged first (shares `event_loop/running.rs`: prefetch call at `:115`).
Blocked by: 42, 43, 44, 45
File set: graft: `src/view/strips.rs`, `src/event_loop/graphics.rs`, `src/event_loop/running.rs` (prefetch call
only), the `KittyImages::new` capacity site, `tests/it/pixel_scroll.rs` (new) + its `mod` line,
`tests/it/held_key.rs` (the layer const only; root module since 42's planner ruling 2026-10-01),
`crates/graft/MAP.md`. graft-graph: deletions in
`src/raster.rs`, `src/raster/*.rs`, `src/raster/cache.rs`, `tests/it/strip_cache.rs`, `tests/it/emphasis_layer.rs`,
`tests/it/snapshots/it__raster__*` (re-taken), `crates/graft-graph/MAP.md`. `docs/perf/fix-scroll-held-key.txt`
(append).
Overlap and order: last of 41–46; every file above was held by 42, 43, 44 or 45 and is free once they land.

## Read first
- `docs/adr/0053-selection-paint-is-its-own-layer.md` (whole).
- `42-scroll-held-key-gate.md` §Need, §Journeys, §Why the existing tests missed it.
- `44-scroll-emphasis-layer.md` §Decided 2–3 (borrowed rows; `Lit.thread` stays `Option<&BTreeSet>`; no `above` in
  the selection) and §Signatures.
- `45-scroll-kitty-layer-deflate.md` §Decided 1, 4 and its NOTES' Step 0 table.
- `crates/graft/MAP.md` `## view::strips` (`:67-70`), `max_lanes` (`:864`), `## event_loop` `pump` (`:909`).
- `crates/graft-graph/MAP.md` `## raster`, `## raster::cache` (as updated by 44).
- `crates/graft-term/MAP.md` `## kitty`, `## iterm2`.

## Scope
In: `pixel_rows` builds plain rows and per-row `Lit`s per strip and the frame's `Backdrop`; `event_loop::graphics`
places backdrop + emphasis on Kitty (deflating when `kitty_deflate`) and composites for iTerm2; idle prefetch uses
`contains_backdrop`/`backdrop`; `KittyImages` capacity doubled. Then **delete** the old path in graft-graph
(`RasterRow`, old `rasterise`, old `StripCache::get`/`contains`, old `layout_hash` if unused, and the tests that
exercise only them: `strip_cache.rs` `emphasis_is_part_of_the_key` (`:250`) and the test at `:493`) and re-take the
raster snapshots.
Out: Unicode/ASCII tiers; coalescing; Kitty encoding.

## Decided
1. **Backdrop per frame:** `Backdrop::of(state.focus, thread_on_screen)`, where `thread_on_screen` is today's "cursor in
   the viewport with its rows" condition (`pixel_rows`: a cursor the wheel left off screen draws unlit, 37). Every
   strip of a frame uses the same backdrop.
2. **Waiting rule unchanged:** while the cursor is in the viewport and its row is missing, all strips wait (30b).
3. **Kitty:** per strip, `Place { layer: Strip }` for the backdrop, then `Place { layer: Emphasis }` at
   `first_column` when `Some`. **iTerm2:** `composited` only, one band per strip, as today.
4. **Deletion is part of this ticket**, after the app is switched: nothing reads a selection inside a strip any
   more, so the old key cannot come back by accident.
5. **The equivalence test goes with its oracle.** `emphasis_layer::composite_matches_the_single_pass` (44) compares
   against the live legacy `rasterise`; when that is deleted, the test is deleted with it, after one last green run
   at this ticket's base, quoted in the report. From then the re-taken snapshots pin the composite's output.
6. **Layer count in tests** by image id: `EMPHASIS_ID_BIT` set = emphasis. `held_key.rs`'s local const
   `EMPHASIS_ID_BIT_42` (42) is replaced by `graft_term::kitty::EMPHASIS_ID_BIT`.

## Signatures
```rust
// crates/graft/src/view/strips.rs — lifetimes as today's `PixelRows` holds its rows (scout names how)
pub struct PixelRows</* as today */> {
    pub strips: Vec<PixelStrip</* … */>>,      // was: strips of the old `Strip`
    pub lane_columns: /* as today */, pub marker: /* as today */, pub uncommitted_on_top: bool,
    pub backdrop: graft_graph::raster::Backdrop,
}
/// Owns what 44's borrowed views borrow; `PlainStrip`/`Selection` are built from it per call.
pub struct PixelStrip</* … */> {
    pub first_row: usize,
    pub rows: Vec<graft_graph::raster::PlainRow</* … */>>,
    pub lit: Vec<graft_graph::raster::Lit</* … */>>,   // one per row; `thread: None` vs `Some(∅)` as today
    pub above: Option</* &Row, as today's Strip.above */>,
}
pub fn pixel_rows(/* as today */) -> PixelRows</* … */>;
```
`event_loop::graphics`'s private functions change freely; no new public item there.

## Acceptance (each box names its test; red first)
- [ ] **Fast-tier guard** (gap (b)/(c) of the gate spec): on `team::TeamSpec::SMALL` at Pixel, 120×40, 20 `j` within
      the strips already on screen transmit **0** backdrop strips (count `a=t`/`a=T` by `EMPHASIS_ID_BIT` in the frame
      bytes), and ≥ 1 emphasis image. Red at the base commit (old path: one strip transmit per viewport strip per `j`).
      — `tests/it/pixel_scroll.rs::j_inside_resident_strips_sends_no_strip` (new file).
- [ ] Focus `F` on, then `j` ×5: 0 backdrop transmits after `F`'s frame; `F` off: one backdrop pass (backdrop change),
      then 0 again. — `pixel_scroll::focus_changes_the_backdrop_once`.
- [ ] Wheel moves the cursor off screen → `Backdrop::Full`, no emphasis placed; back on screen → `OffThread`.
      — `pixel_scroll::cursor_off_screen_draws_full`.
- [ ] iTerm2 tier: one band per strip, composited; `j` re-encodes only bands whose selection changed.
      — `pixel_scroll::iterm2_bands_are_composites`.
- [ ] Deflate follows the capability: `kitty_deflate` on → every transmit `o=z`; off → none.
      — `pixel_scroll::deflate_follows_the_probe`.
- [ ] Old path deleted: `rg 'RasterRow|fn rasterise\(|pub fn get\(|pub fn contains\(|emphasis_is_part_of_the_key'
      crates/graft-graph crates/graft` shows no hit outside history comments (paste the command and its empty output).
      (`\.thread\b` is not in the pattern: `Lit.thread` is the new field.)
- [ ] Equivalence test's last green run at the base quoted; test deleted with `rasterise` (Decided 5).
- [ ] Raster snapshots `it__raster__*` re-taken through the composite; the diff reviewed in the report (which files,
      why each changed: expected only anti-aliased edges and lit-over-unlit crossings, ADR-0053 Consequences).
      — the snapshot tests by name.
- [ ] Gate goes green at Pixel (release): `held_key::held_j_stops_on_release_pixel` and
      `held_key::j_then_journal_opens_at_once_pixel` via `scripts/fix-scroll-held-key.sh`, appended to
      `docs/perf/fix-scroll-held-key.txt` (`# rev strips`); Unicode still green. Any item not `ok` → report the line;
      do not loosen a budget.
- [ ] Existing `frames::` Pixel scroll tests unchanged and green in the fast tier; release numbers in the perf file.
- [ ] MAP.md: graft `view::strips` lines; graft-graph removed items deleted.

## Touches (report each on the **Touches:** line)
- Every caller of `pixel_rows` / `PixelRows` (grep): the hit-test (`max_lanes`, `screen_area`) reads columns, not
  pixels — confirm unchanged.
- The gallery and every pixel capture in `docs/shots/` (character grids show no pixels; confirm none changes: run the
  capture tests).
- Resize: "a resize flushes nothing and retransmits nothing" (`pump`, MAP `:909`) must still hold for both layers;
  test or name the existing one that covers it.
- Theme / cell-size flush: flushes every LRU (graft-graph `flush`).
- Deflate runtime fallback (45 Decided 4): `KittyImages::answer` is still fed every reply; name the call site.
- Uncommitted row on `HEAD` (`uncommitted_on_top`): its ring is layout; its selection paint goes in `Lit`.
- Fold stubs (39): layout, in `PlainRow`.
- KNOWN_GAPS row "Focus and the ancestry highlight are bold/dim only" (Pixel half closed by ADR-0024): unchanged.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide; plus `cargo clippy --all-targets --features heavy-tests -- -D warnings`.

## Size
~65k. Split point if it compacts: switch + fast-tier tests first; deletion + snapshots second.
