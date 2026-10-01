# 44-scroll-emphasis-layer — strips drawn from layout only; the selection's paint as a second image

Status: done
Build with: Fable (signature) — introduces the backdrop/emphasis types and cache keys 46 builds on; the alpha split
is where a wrong approach becomes structural, and it cannot be judged from the diff (CLAUDE.md "Models").
Needs: none (additive; `46-scroll-strips-wiring` switches callers over and deletes the old path).
Blocked by: none
File set (crate `graft-graph` only): `src/raster.rs`, `src/raster/pen.rs`, `src/raster/node.rs` (only if ring/stub
alpha needs it), `src/raster/cache.rs`, `src/raster/emphasis.rs` (new), `tests/it/strip_cache.rs`,
`tests/it/emphasis_layer.rs` (new) + its `mod` line in the `tests/it` binary's root, `tests/it/raster_pixels.rs`
(helper visibility only), `crates/graft-graph/MAP.md`; `scripts/fix-scroll-emphasis-cost.sh`,
`docs/perf/44-scroll-emphasis-layer.txt`, `docs/shots/m7/fix-scroll-thread-{legacy,composite}.png`.
Overlap: none by file with 41/43/45 (scout 44). 46 edits these graft-graph files later (deletions): 46 after 44.

## Read first
- `docs/adr/0053-selection-paint-is-its-own-layer.md` (whole, 1 page) and `docs/adr/0024-*.md` §Decision.
- `.scratch/m7-power-tools/scouts/44.md` (whole; 1 page) — today's types, key, paint order, with lines.
- `42-scroll-held-key-gate.md` §Need, §Prior art.
- `crates/graft-graph/MAP.md` `## raster` (`RasterRow` `:120`, `Emphasis` `:124`, alphas `:126-127`),
  `## raster::cache` (`layout_hash` `:144`), `## ancestry`.
- `docs/design/03-graph-layout-algorithm.md` §3.8 (`:371-412`); `docs/design/05-visualization.md` §5.2 (`:60-94`).
- `.scratch/m7-power-tools/debug-scroll-lag.md` cause 1 (the key: `raster/cache.rs:294-298`).

## Scope
In (graft-graph only): split what the rasteriser draws into a **backdrop strip** (layout only) and an **emphasis
image** (the selection's paint); cache both; a CPU composite for iTerm2 and tests. Old `RasterRow.thread`/`emphasis`,
old `rasterise`/`get`/`contains`/`layout_hash` stay working, **byte-identical**, until the wiring ticket deletes them.
Out: graft, graft-term (placement, compression); Unicode tier.

## Decided (ADR-0053)
1. **Backdrop** = the lowest alpha in play for the frame: `Full` 1.0 (no thread on screen), `OffThread`
   `OFF_THREAD_ALPHA`, `Faded` `FADED_ALPHA` (Focus). Every element of a strip — lanes, curves, rings, tails, stubs,
   rail — is drawn at it. Strip key: `(row range, layout hash, theme id, cell size, backdrop)`. The layout hash covers
   the new layout-only row type, which **has no selection field**, so the key cannot pick one up again.
2. **Rows borrow, as today** (settles scout 44 items 2 and 4 without assuming anything about `Row`):
   - `PlainRow<'a>` holds `row: &'a Row` like `RasterRow` (`raster.rs:99-114`). It derives **no** `Hash`: whether `Row`
     derives `Hash` is not confirmed (MAP.md `:18-25` does not say) and is not needed. `plain_layout_hash` hashes the
     fields explicitly, exactly as `layout_hash` does today (`cache.rs:280-308`) minus the selection lines
     (`:294-298`).
   - `Lit<'a>.thread` is `Option<&'a BTreeSet<usize>>`, today's type (`raster.rs:110`), **not** a `Vec`: today `None`
     means "alpha from `emphasis`" and `Some(∅)` means "every column off-thread" (`alpha_at`, `raster.rs:130-135`).
     A `Vec` would merge the two. Equality and hash keep the distinction (derived on `Option`).
3. **Selection paint** (`Lit`) per row: its `Emphasis` and its thread columns (today's `RasterRow.emphasis` and
   `.thread`). **No `above` in the selection** (scout 44 item 3): today `above` is `Option<&Row>` with no paint, and
   the first row's incoming edges take that row's own alpha (`raster.rs:154`). `PlainStrip.above` keeps that; the
   emphasis pass reads the first row's `Lit` for them, as today.
4. **Emphasis image:** every element whose ADR-0024 alpha `a` (what today's single pass gives it) is above the
   backdrop `b` is drawn at `(a − b) / (1 − b)`; nothing else. Cropped to the cell columns it touches
   (`first_column`, width in whole cells); `None` when it would be empty. Key: the strip's key + `lit_hash`.
   An element with `a < b` would be overdrawn by the backdrop: `debug_assert!` in the pass, counted in the
   equivalence test (must be 0).
5. **Draw order inside the emphasis image** is today's order among those elements (edges column by column, then per
   row stubs, tails, rail, node: `raster.rs:192-261`).
6. **Composite** = premultiplied "over", emphasis on top, at its column offset.
7. **Equivalence with today** (the correctness anchor; colour itself stays unverified, rule 5): per pixel, with
   `legacy` = today's `rasterise` on a `Strip` built from the same rows, ring, stubs, emphasis and thread, and
   `C` = composite:
   - emphasis alpha 0 there → `C == legacy` exactly;
   - emphasis alpha 255 there and `legacy` drew no unlit element over a lit one there → `C == legacy` exactly;
   - otherwise (anti-aliased edges) `|C − legacy| ≤ ceil(b × 255)` per channel (ADR-0053 Consequences); the max seen
     is printed;
   - crossings where `legacy` drew an unlit line over a lit one: counted, ≤ 0.5% of non-transparent pixels.
   **Oracle:** the live `rasterise`, which this ticket must leave byte-identical. Proof: every `it__raster__*`
   snapshot passes with no re-take. A needed change to the shared `Pen` that would move a legacy pixel → stop and
   report; do not re-take.
8. **Prefetch** (`contains_backdrop` + `backdrop` while idle): backdrop strips only.

## Signatures (graft-graph)
```rust
// raster
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Backdrop { Full, OffThread, Faded }
impl Backdrop {
    pub fn alpha(self) -> f32;                                   // 1.0 / OFF_THREAD_ALPHA / FADED_ALPHA
    /// Faded if `focus`, else OffThread if a thread is on screen, else Full.
    pub fn of(focus: bool, thread_on_screen: bool) -> Backdrop;
}

/// A row as the layout draws it: no selection in it. Fields as `RasterRow` minus `emphasis`, `thread`.
#[derive(Clone, Debug)]
pub struct PlainRow<'a> { pub row: &'a Row, pub ring: Ring, pub stubs: /* RasterRow.stubs's type, unchanged */ }
impl<'a> PlainRow<'a> {
    /// `Ring::of` the row, no stubs (as `RasterRow::plain`).
    pub fn of(row: &'a Row) -> PlainRow<'a>;
}

/// One row's selection paint (today's RasterRow.emphasis + .thread, same types).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Lit<'a> { pub emphasis: Emphasis, pub thread: Option<&'a std::collections::BTreeSet<usize>> }
// `Emphasis` (raster.rs:69) derives Default, Hash; add PartialEq, Eq there if missing (fieldless enum).

/// Today's `Strip` with `rows: &'a [PlainRow<'a>]`; `above: Option<&'a Row>` and every other field as today.
pub struct PlainStrip<'a> { /* rows, above, lane_columns, marker, cell_width, cell_height, palette, muted */ }

/// The selection over one strip: one `Lit` per `PlainStrip::rows`, and the frame's backdrop.
pub struct Selection<'a> { pub rows: &'a [Lit<'a>], pub backdrop: Backdrop }

pub struct Emphasised { pub frame: core::RasterFrame, pub first_column: u16 }

pub fn rasterise_backdrop(strip: &PlainStrip<'_>, backdrop: Backdrop) -> Result<core::RasterFrame, RasterError>;
pub fn rasterise_emphasis(strip: &PlainStrip<'_>, sel: &Selection<'_>) -> Result<Option<Emphasised>, RasterError>;
pub fn composite(base: &core::RasterFrame, top: &Emphasised) -> core::RasterFrame;

// raster::cache — `first_row: usize`, as today's `get` (cache.rs:159)
impl StripCache {
    pub fn backdrop(&mut self, first_row: usize, strip: &PlainStrip<'_>, backdrop: Backdrop)
        -> Result<std::sync::Arc<core::RasterFrame>, CacheError>;
    pub fn emphasis(&mut self, first_row: usize, strip: &PlainStrip<'_>, sel: &Selection<'_>)
        -> Result<Option<std::sync::Arc<Emphasised>>, CacheError>;
    /// iTerm2: one image per strip, cached on (backdrop key, lit_hash).
    pub fn composited(&mut self, first_row: usize, strip: &PlainStrip<'_>, sel: &Selection<'_>)
        -> Result<std::sync::Arc<core::RasterFrame>, CacheError>;
    pub fn contains_backdrop(&self, first_row: usize, strip: &PlainStrip<'_>, backdrop: Backdrop) -> bool;
}
pub fn plain_layout_hash(strip: &PlainStrip<'_>) -> u64;
/// Hash of `sel.backdrop` and every `Lit` in order (derived `Hash`: emphasis, thread presence, thread set).
pub fn lit_hash(sel: &Selection<'_>) -> u64;
// CacheStats gains: emphasis_rasterised, emphasis_hits (u64)
```
Emphasis and composite entries are bounded by the same `capacity_for` count, each in its own LRU.
`Misaligned` (`cache.rs:195-218`) applies to the new keys as to `get`.

## Acceptance (each box names its test; red first)
Tests live in the `tests/it` binary: `cache.rs` has no test module, and the branchy-window helpers are integration
helpers (`tests/it/raster_pixels.rs:97 corpus`, `:174 threaded`; `tests/it/raster.rs:50 rows_of`). Make a helper
`pub(crate)` if another file needs it; nothing else in those files changes.
- [x] Moving the selection across every row of a branchy window (≥ 8 lanes, ≥ 3 strips) never changes a strip's
      backdrop key and rasterises 0 backdrops after the first pass.
      — `strip_cache::the_selection_never_misses_a_backdrop` (window 96 rows × 11 lanes, 3 strips, both backdrops).
- [x] Equivalence (Decided 7) over that window's strips × every cursor row × `Backdrop::{OffThread, Faded}`, and `Full`
      with no thread (emphasis `None`, backdrop == legacy exactly); elements with `a < b`: 0. Prints each case's
      pixel-class counts and the max edge difference. — `emphasis_layer::composite_matches_the_single_pass`.
      Last run: 186 distinct paints drawn (856 repeats by `lit_hash` skipped); clear-pixel diffs 0; `a < b` 0;
      crossings 211 px of 3.55 M painted (5 per 100 000), max diff 1; edge max 32–35 (OffThread, tol 128),
      19–22 (Faded, tol 77). At 8×16 every cursor; at 17×37 (his cell) 1 cursor in 4 — 8×16 strokes never fully
      cover a pixel, so the solid/crossing classes need the big cell.
- [x] `Lit::thread = Some(∅)` and `None` on the same row give different `lit_hash` and different emphasis images
      whenever their alphas differ. — `emphasis_layer::an_empty_thread_is_not_no_thread`.
- [x] Emphasis is cropped to the span of lit cell columns (whole cells); `first_column` places it.
      — `emphasis_layer::emphasis_is_cropped_to_lit_columns`.
- [x] `Backdrop::Full`, any `Lit` → `rasterise_emphasis` is `None`; `OffThread` with `Some(∅)` on every row → `None`.
      — `emphasis_layer::no_paint_no_image`.
- [x] Back-and-forth `j`/`k` over 10 rows rasterises each distinct selection's emphasis once.
      — `strip_cache::emphasis_is_cached_by_selection`.
- [x] `resident_bytes` counts all three LRUs (four: the legacy one too, until 46).
      — `strip_cache::resident_bytes_counts_every_layer`.
- [x] Old path byte-identical: every `it__raster__*` snapshot green, none re-taken; `strip_cache.rs:250
      emphasis_is_part_of_the_key` and `:493` green unchanged. — `raster::*` snapshot tests,
      `raster_pixels::strips_keep_every_byte`, `strip_cache::emphasis_is_part_of_the_key`,
      `strip_cache::a_threaded_rows_own_emphasis_changes_neither_pixels_nor_key`; `git status` shows no snapshot file
      changed.
- [x] Cost (release, `scripts/fix-scroll-emphasis-cost.sh` → `docs/perf/44-scroll-emphasis-layer.txt`):
      `rasterise_emphasis` p99 per strip on the test window at 187 columns, against today's full strip; recorded, not
      asserted (the gate is the held-key test). — ignored test `emphasis_layer::emphasis_cost`, run by the script.
      p50/p99 µs per strip at 17×37: branchy (22 cells) today 2018/2525, emphasis 761/1389; wide-187 (190 cells)
      today 4521/4982, emphasis 792/**5453** (p99 thread spans 176 of 190 cells — see NOTES 4).
- [x] PNGs for his eye: `docs/shots/m7/fix-scroll-thread-legacy.png` (27 KB) and `…-composite.png` (28 KB), same
      strip, same cursor, `OffThread`, each ≤ 200 KB. — `emphasis_layer::thread_pngs` (via `check_bytes`, so both are
      written on update; `capture_bytes` stops at the first). **Right:** the same lit thread in both; off-thread lanes
      at the same half strength; no line wider than another. **Not judged by me** (rule 6); colour **unverified**
      (rule 5).
- [x] `crates/graft-graph/MAP.md`: every new item one line; `StripCache`: "`backdrop`/`emphasis`/`composited` key on
      layout + backdrop (+ `lit_hash`); legacy `get`/`contains` still key on the selection until 46". — read in the diff.

## Touches
- Raster snapshots `tests/it/snapshots/it__raster__*`: untouched (box above); the wiring ticket re-takes them.
- `layout_hash`, `get`, `contains`, `rasterise` callers (grep; `graft/src/event_loop/graphics.rs:17`,
  `view/strips.rs`): unchanged; 46 switches them.
- `ancestry::Ancestry`: read, not changed.
- `core::RasterFrame`: read only.
- Memory: three LRUs (box above).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
~65k. Split point if it compacts: backdrop + cache key (box 1) first; emphasis + composite second.

## NOTES (builder, 2026-10-01)
**Compacted once**, after all code, tests, PNGs and the perf file were done and green in graft-graph; only the
workspace gates, this ticket and the commit remained. Nothing of the scope remains.

Scout: used `.scratch/m7-power-tools/scouts/44.md`, plus my own reads of `raster.rs`, `pen.rs`, `node.rs`, `cache.rs`.

How it is built: the single pass's body moved, unchanged in order, into a private `paint(look, rows, lift)`;
`rasterise` calls it with `lift: None` (legacy, byte-identical), `rasterise_backdrop` with every row painted at the
backdrop, `rasterise_emphasis` with `lift: Some(b)`: the pen's `tone()` maps `a > b` to `(a − b)/(1 − b)` and skips
everything else (`debug_assert!(a >= b)`). Node discs are always punched (so lanes under a node clear exactly as
today), ink skipped when unlit. Geometry and draw order are shared code, not a copy.

Findings:
1. **Gap for 46 (behaviour, with line):** `Backdrop::of(focus, thread_on_screen)` (the signature as specced) gives
   `Full` when there is no thread and no Focus, but `graft/src/view/strips.rs:369-371` dims stash-only rows
   (`Emphasis::Dimmed`, 0.3) exactly then. On the new path those rows draw at 1.0: the m7/16 stash-only dimming is
   lost. The backdrop must be the lowest alpha in play, so 46 needs `Faded` (or a third input) when any stash-only
   row is on screen with no thread. With a thread, `thread` overrides `emphasis`, so the dimming is moot there.
2. **Deviation:** `lit_hash` is **not** the derived `Hash` the signature comment names. It hashes the backdrop, the
   row count, and per row the thread set if `Some`, else the emphasis, plus `thread.is_some()`. Reason: under a thread
   `emphasis` changes no pixel (30b; `a_threaded_rows_own_emphasis_changes_neither_pixels_nor_key`), so the derived
   hash would miss the cache on a Focus kin change that draws nothing different. `Some(∅)` vs `None` stay distinct
   (`an_empty_thread_is_not_no_thread`). Conductor: confirm or revert.
3. Under Focus with a thread, the single pass never draws at 0.3 (the thread overrides), so with the `Faded` backdrop
   every 0.5 element is "lit" too: correct pixels, a wider emphasis image. 46 could pick `OffThread` when Focus and a
   thread are both on — a choice for 46, not made here.
4. **Cost:** wide-187 emphasis p99 5453 µs is above today's full strip p99 4982 µs, when the thread spans 176 of 190
   cells (a near-full-width emphasis is a second full strip). p50 is 792 vs 4521. Recorded, not asserted.
5. The equivalence test samples the 17×37 cell at 1 cursor in 4 (debug run time 34–44 s → ~11 s); 8×16 covers every
   distinct paint.

Verified: equivalence classes per pixel against live `rasterise` (numbers in box 2); snapshots untouched. Not
verified: how the composite looks (PNGs for the user, rule 6); colour (rule 5); anything in the app (46 wires it).

Touches: `rasterise`/`get`/`contains`/`layout_hash` callers in graft (`event_loop/graphics.rs`, `view/strips.rs`) —
unchanged signatures, legacy tests green; raster snapshots — none re-taken; `CacheStats` gains two fields — grep shows
only `strip_stats()` readers (`graft/src/event_loop/graphics.rs:239`, its tests), no exhaustive struct literal;
`RasterError::Selection` new variant — no exhaustive match outside graft-graph (workspace clippy/build green);
`RasterRow::alpha_at` removed — it was private (`raster.rs:130` at HEAD), moved to `Lit::alpha_at`; `ancestry` read only;
`core::RasterFrame` read only.
