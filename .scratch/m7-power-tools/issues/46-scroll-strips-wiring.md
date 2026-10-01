# 46-scroll-strips-wiring — the app draws backdrop + emphasis; a `j` never re-sends a strip

Status: ready
Build with: (blank: scout first — `view::strips`, `event_loop::graphics` are existing code)
Needs: `44-scroll-emphasis-layer` (`Backdrop`, `PlainRow<'a>`, `PlainStrip<'a>`, `Lit<'a>`, `Selection<'a>`,
`Emphasised`, `StripCache::{backdrop, emphasis, composited, contains_backdrop}`, `composite`; **merged only after its
2026-10-01 return** — see its "Planner ruling (44 return)");
`45-scroll-kitty-layer-deflate` (`Layer`, `EMPHASIS_ID_BIT`, `Place::layer`, `KittyImages::{deflating,
is_deflating}`, `Capabilities::kitty_deflate`; and its Step 0 table with no "no" in it);
`43-scroll-coalesce-input` merged first (shares `event_loop/running.rs`: prefetch call at `:115`).
Blocked by: 42, 43, 44, 45
File set: graft: `src/view/strips.rs`, `src/event_loop/graphics.rs`, `src/event_loop/running.rs` (prefetch call
only), the `KittyImages::new` capacity site, `tests/it/pixel_scroll.rs` (new) + its `mod` line,
`tests/it/held_key.rs` (the layer const only; root module since 42's planner ruling 2026-10-01),
`crates/graft/MAP.md`. graft-graph: deletions in
`src/raster.rs`, `src/raster/*.rs`, `src/raster/cache.rs`, `tests/it/strip_cache.rs`, `tests/it/emphasis_layer.rs`,
`tests/it/snapshots/it__raster__*` (re-taken), `crates/graft-graph/MAP.md`; **plus** `Backdrop::lowest` in
`src/raster.rs` and its tests in `tests/it/emphasis_layer.rs` (Planner ruling below). `docs/perf/fix-scroll-held-key.txt`
(append); `scripts/fix-scroll-held-key.sh` (an env switch for the runs in the ruling's item 3 only).
Overlap and order: last of 41–46; every file above was held by 42, 43, 44 or 45 and is free once they land.

## Read first
- `docs/adr/0053-selection-paint-is-its-own-layer.md` (whole).
- `42-scroll-held-key-gate.md` §Need, §Journeys, §Why the existing tests missed it.
- `44-scroll-emphasis-layer.md` §Decided 2–3 (borrowed rows; `Lit.thread` stays `Option<&BTreeSet>`; no `above` in
  the selection), §Signatures, NOTES findings 1–4, and "Planner ruling 2026-10-01 (44 return)".
- `45-scroll-kitty-layer-deflate.md` §Decided 1, 4 and its NOTES' Step 0 table.
- `crates/graft/MAP.md` `## view::strips` (`:67-70`), `max_lanes` (`:864`), `## event_loop` `pump` (`:909`).
- `crates/graft-graph/MAP.md` `## raster`, `## raster::cache` (as updated by 44).
- `crates/graft-term/MAP.md` `## kitty`, `## iterm2`.

## Scope
In: `pixel_rows` builds plain rows and per-row `Lit`s per strip and the frame's `Backdrop`; `event_loop::graphics`
places backdrop + emphasis on Kitty (deflating when `kitty_deflate`) and composites for iTerm2; idle prefetch uses
`contains_backdrop`/`backdrop`; `KittyImages` capacity doubled. Then **delete** the old path in graft-graph
(`RasterRow`, old `rasterise`, old `StripCache::get`/`contains`, old `layout_hash` if unused, `Backdrop::of`, and the
tests that exercise only them: `strip_cache.rs` `emphasis_is_part_of_the_key` (`:250`) and the test at `:493`) and
re-take the raster snapshots.
Out: Unicode/ASCII tiers; coalescing; Kitty encoding.

## Decided
1. **Backdrop per frame = `Backdrop::lowest` over every `Lit` of every strip in the frame** (Planner ruling item 2;
   replaces `Backdrop::of(state.focus, thread_on_screen)`). Every strip of a frame uses the same backdrop. It changes
   when the thread comes or goes (cursor on/off screen, 37), when Focus dims rows with no thread, and when a dimmed
   row (stash-only, m7/16) enters or leaves the frame with no thread.
2. **Waiting rule unchanged:** while the cursor is in the viewport and its row is missing, all strips wait (30b).
3. **Kitty:** per strip, `Place { layer: Strip }` for the backdrop, then `Place { layer: Emphasis }` at
   `first_column` when `Some`. **iTerm2:** `composited` only, one band per strip, as today.
4. **Deletion is part of this ticket**, after the app is switched: nothing reads a selection inside a strip any
   more, so the old key cannot come back by accident.
5. **The equivalence test goes with its oracle.** `emphasis_layer::composite_matches_the_single_pass` (44) compares
   against the live legacy `rasterise`. Before deleting it, add the case in the ruling's item 2 and quote the last
   green run at this ticket's base in the report. Then delete the test with `rasterise`. From then the re-taken
   snapshots pin the composite's output.
6. **Layer count in tests** by image id: `EMPHASIS_ID_BIT` set = emphasis. `held_key.rs`'s local const
   `EMPHASIS_ID_BIT_42` (42) is replaced by `graft_term::kitty::EMPHASIS_ID_BIT`.

## Signatures
```rust
// crates/graft-graph/src/raster.rs — new (ruling item 2); `Backdrop::of` deleted
impl Backdrop {
    /// The lowest alpha any of these rows can draw at (ADR-0053 Decision 2):
    /// `Faded` if some row has `thread: None` and `Emphasis::Dimmed`; else `OffThread` if some row has
    /// `thread: Some(_)` (∅ included); else `Full`.
    pub fn lowest<'a, 'b: 'a>(lits: impl IntoIterator<Item = &'a Lit<'b>>) -> Backdrop;
}

// crates/graft/src/view/strips.rs — lifetimes as today's `PixelRows` holds its rows (scout names how)
pub struct PixelRows</* as today */> {
    pub strips: Vec<PixelStrip</* … */>>,      // was: strips of the old `Strip`
    pub lane_columns: /* as today */, pub marker: /* as today */, pub uncommitted_on_top: bool,
    pub backdrop: graft_graph::raster::Backdrop, // Backdrop::lowest over every strip's `lit`
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
- [ ] `Backdrop::lowest` table: empty → `Full`; all `None`+`Normal` → `Full`; one `Some(∅)` → `OffThread`; one
      `None`+`Dimmed` with or without threaded rows → `Faded`; `Some(_)`+`Dimmed` only → `OffThread`.
      — `emphasis_layer::lowest_backdrop_table`.
- [ ] **Stash-only rows keep their dimming** (44 NOTES 1; m7/16, `strips.rs:369-371` at the base): a stash-only row
      on screen, cursor wheeled off screen → `PixelRows.backdrop == Faded`; the same window with no stash-only row on
      screen → `Full`. — `pixel_scroll::stash_only_rows_stay_dimmed_without_a_thread`.
- [ ] Focus `F` on, `j` ×5, `F` off: a backdrop transmit happens only on a frame whose `PixelRows.backdrop` differs
      from the previous frame's; the test prints the backdrop per frame. 0 backdrop transmits on the `j` frames.
      — `pixel_scroll::focus_changes_the_backdrop_only_when_lowest_changes`.
- [ ] Wheel moves the cursor off screen (no dimmed row on screen) → `Backdrop::Full`, no emphasis placed; back on
      screen → `OffThread`. — `pixel_scroll::cursor_off_screen_draws_full`.
- [ ] iTerm2 tier: one band per strip, composited; `j` re-encodes only bands whose selection changed.
      — `pixel_scroll::iterm2_bands_are_composites`.
- [ ] Deflate follows the capability: `kitty_deflate` on → every transmit `o=z`; off → none.
      — `pixel_scroll::deflate_follows_the_probe`.
- [ ] Old path deleted: `rg 'RasterRow|fn rasterise\(|pub fn get\(|pub fn contains\(|emphasis_is_part_of_the_key|Backdrop::of'
      crates/graft-graph crates/graft` shows no hit outside history comments (paste the command and its empty output).
      (`\.thread\b` is not in the pattern: `Lit.thread` is the new field.)
- [ ] Equivalence test's last green run at the base quoted, **including the ruling item 2 case**; test deleted with
      `rasterise` (Decided 5).
- [ ] Raster snapshots `it__raster__*` re-taken through the composite; the diff reviewed in the report (which files,
      why each changed: expected only anti-aliased edges and lit-over-unlit crossings, ADR-0053 Consequences).
      — the snapshot tests by name.
- [ ] Gate goes green at Pixel (release): `held_key::held_j_stops_on_release_pixel` and
      `held_key::j_then_journal_opens_at_once_pixel` via `scripts/fix-scroll-held-key.sh`, appended to
      `docs/perf/fix-scroll-held-key.txt` (`# rev strips`); Unicode still green. Any item not `ok` → report the line;
      do not loosen a budget.
- [ ] Cost lines appended (ruling item 3), recorded, not asserted beyond the gate: Kitty with `kitty_deflate` on,
      Kitty with it forced off, and iTerm2, each with `bytes_per_key`, `tail_ms` and the emphasis width p99 in cells.
      — the script's summary lines, quoted.
- [ ] Existing `frames::` Pixel scroll tests unchanged and green in the fast tier; release numbers in the perf file.
- [ ] MAP.md: graft `view::strips` lines; graft-graph removed items deleted, `Backdrop::lowest` one line.

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
- Stash-only dimming (m7/16, `strips.rs:369-371`): now through `Lit.emphasis` + `Backdrop::lowest`; its existing tests
  green (name them).
- Every caller of `Backdrop::of` (grep): none may remain.
- KNOWN_GAPS row "Focus and the ancestry highlight are bold/dim only" (Pixel half closed by ADR-0024): unchanged.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide; plus `cargo clippy --all-targets --features heavy-tests -- -D warnings`.

## Size
~70k. Split point if it compacts: `Backdrop::lowest` + switch + fast-tier tests first; deletion + snapshots + cost
lines second.

## Planner ruling 2026-10-01 (44 return)
1. **`lit_hash` (44 NOTES 2):** kept provisionally. 44 proves it before merge (one grep + one test). If 44 reverts to
   the derived `Hash`, this ticket changes nothing.
2. **Stash-only dimming (44 NOTES 1, line `graft/src/view/strips.rs:369-371`):** a real gap. `Backdrop::of(focus,
   thread_on_screen)` returns `Full` for a frame with no thread and no Focus, while stash-only rows are `Dimmed`
   (0.3, `FADED_ALPHA`, graft-graph MAP `:127`), so they would draw at 1.0. **Fix:** the backdrop is derived from the
   paint, never from the features that produce it. `Backdrop::lowest` (Signatures) replaces `of`, which goes. That is
   ADR-0053's "lowest alpha in play" read from the data, so a future dimming feature cannot reopen the gap. It also
   settles 44 NOTES 3: Focus with a thread gives `OffThread`, a narrower emphasis. Before deleting the equivalence
   test, add a case to it: no thread, one `Dimmed` row, backdrop `lowest` (= `Faded`), with `a < b` = 0 and the same
   pixel classes. Cost: when the cursor is off screen and a stash-only row is in view, nearly the whole strip is
   "lit". That costs about two strips per new strip during wheel scroll, plus one re-draw when the backdrop flips.
   It is rare and has no `j` in it (the cursor is off screen); accepted.
3. **Emphasis cost (44 NOTES 4): acceptable, not a blocker.** The tail is bytes-bound (`scroll-lag-big-repo.txt:45`
   vs `:53`; Kitty `:49`). The worst case adds about 0.5 ms p99 per re-drawn strip, under the 33 ms key period, and
   p50 drops 4521 → 792 µs (`44-scroll-emphasis-layer.txt:12,14`). **Open, measured here:** with deflate off, a wide
   thread re-sends about a strip's bytes per changed strip; on iTerm2, emphasis + composite p99 is ~7.8 ms per strip
   against 5.0 today, and every changed band is re-sent whole. Record the three cost lines (box above). If the
   held-key gate fails only in the deflate-off or iTerm2 run, report the line and stop. The follow-up is a new
   ticket (crop the emphasis to its row span as well as its columns), not a looser budget.
