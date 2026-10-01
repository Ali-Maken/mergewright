# 44-scroll-emphasis-layer — strips drawn from layout only; the selection's paint as a second image

Status: ready
Build with: (blank: scout first — `graft-graph::raster` is existing code. Conductor: this is the fix's signature
ticket; per CLAUDE.md "Models" it is the Fable (signature) candidate. The alpha split is where a wrong approach becomes
structural, and it is hard to judge from a diff.)
Needs: none (additive; `46-scroll-strips-wiring` switches callers over and deletes the old path).
Blocked by: none

## Read first
- `docs/adr/0053-selection-paint-is-its-own-layer.md` (whole, 1 page) and `docs/adr/0024-*.md` §Decision.
- `42-scroll-held-key-gate.md` §Need, §Prior art.
- `crates/graft-graph/MAP.md` `## raster`, `## raster::cache`, `## ancestry`.
- `docs/design/03-graph-layout-algorithm.md` §3.8 (`:371-412`); `docs/design/05-visualization.md` §5.2 (`:60-94`).
- `.scratch/m7-power-tools/debug-scroll-lag.md` cause 1 (the key at `raster/cache.rs:295-296`).

## Scope
In (graft-graph only): split what the rasteriser draws into a **backdrop strip** (layout only) and an **emphasis
image** (the selection's paint); cache both; a CPU composite for iTerm2 and tests. Old `RasterRow.thread`/`emphasis`,
old `rasterise`/`get` stay working, untouched, until the wiring ticket deletes them.
Out: graft, graft-term (placement, compression); Unicode tier.

## Decided (ADR-0053)
1. **Backdrop** = the lowest alpha in play for the frame: `Full` 1.0 (no thread on screen), `OffThread`
   `OFF_THREAD_ALPHA`, `Faded` `FADED_ALPHA` (Focus). Every element of a strip — lanes, curves, rings, tails, stubs,
   rail — is drawn at it. Strip key: `(row range, layout hash, theme id, cell size, backdrop)`. The layout hash covers
   the new layout-only row type, which **has no selection field**, so the key cannot pick one up again.
2. **Selection paint** (`Lit`) per row: its `Emphasis` and its thread columns (today's `RasterRow.emphasis` and
   `.thread`), plus the row above the strip (`above`), since incoming edges draw from it.
3. **Emphasis image:** every element whose ADR-0024 alpha `a` (what today's single pass gives it from `Lit`) is above
   the backdrop `b` is drawn at `(a − b) / (1 − b)`; nothing else. Cropped to the cell columns it touches
   (`first_column`, width in whole cells); `None` when it would be empty. Key: the strip's key + `lit_hash`.
4. **Draw order inside the emphasis image** is today's order among those elements.
5. **Composite** = premultiplied "over", emphasis on top, at its column offset.
6. **Equivalence with today** (the correctness anchor; colour itself stays unverified, rule 5): per pixel, with
   `legacy` = today's single-pass raster and `C` = composite:
   - emphasis alpha 0 there → `C == legacy` exactly;
   - emphasis alpha 255 there and `legacy` drew no unlit element over a lit one there → `C == legacy` exactly;
   - otherwise (anti-aliased edges) `|C − legacy| ≤ ceil(b × 255 / 4)` per channel (bound: ADR-0053 Consequences);
   - crossings where `legacy` drew an unlit line over a lit one: counted, ≤ 0.5% of non-transparent pixels.
   The oracle is today's single pass, copied verbatim into the test module **before** anything changes, HEAD hash in
   a comment.
7. **Prefetch** (`StripCache::contains` + `get` while idle): backdrop strips only.

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

/// A row as the layout draws it: no selection in it.
#[derive(Clone, Debug, PartialEq, Hash)]
pub struct PlainRow { pub row: /* today's RasterRow.row type */, pub ring: Ring, pub stubs: Vec<Stub> }

/// One row's selection paint (today's RasterRow.emphasis + .thread).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Lit { pub emphasis: Emphasis, pub thread: Vec<usize> }

/// Today's `Strip` with `rows: Vec<PlainRow>`, `above: Option<PlainRow>`; other fields as today.
pub struct PlainStrip { /* rows, above, lane_columns, marker, cell_width, cell_height, palette, muted */ }

/// The selection over one strip: one `Lit` per `PlainStrip::rows`, plus `above`.
pub struct Selection { pub rows: Vec<Lit>, pub above: Option<Lit>, pub backdrop: Backdrop }

pub struct Emphasised { pub frame: core::RasterFrame, pub first_column: u16 }

pub fn rasterise_backdrop(strip: &PlainStrip, backdrop: Backdrop) -> Result<core::RasterFrame, RasterError>;
pub fn rasterise_emphasis(strip: &PlainStrip, sel: &Selection) -> Result<Option<Emphasised>, RasterError>;
pub fn composite(base: &core::RasterFrame, top: &Emphasised) -> core::RasterFrame;

// raster::cache
impl StripCache {
    pub fn backdrop(&mut self, first_row: usize, strip: &PlainStrip, backdrop: Backdrop)
        -> Result<std::sync::Arc<core::RasterFrame>, CacheError>;
    pub fn emphasis(&mut self, first_row: usize, strip: &PlainStrip, sel: &Selection)
        -> Result<Option<std::sync::Arc<Emphasised>>, CacheError>;
    /// iTerm2: one image per strip, cached on (backdrop key, lit_hash).
    pub fn composited(&mut self, first_row: usize, strip: &PlainStrip, sel: &Selection)
        -> Result<std::sync::Arc<core::RasterFrame>, CacheError>;
    pub fn contains_backdrop(&self, first_row: usize, strip: &PlainStrip, backdrop: Backdrop) -> bool;
}
pub fn plain_layout_hash(strip: &PlainStrip) -> u64;
pub fn lit_hash(sel: &Selection) -> u64;
// CacheStats gains: emphasis_rasterised, emphasis_hits (u64)
```
Argument shapes (`first_row` vs a range type) follow today's `get`; the scout names them. Emphasis entries are bounded
by the same `capacity_for` count, in their own LRU.

## Acceptance (each box names its test; red first)
- [ ] Moving the selection across every row of a branchy window (≥ 8 lanes, ≥ 3 strips; built in the test from rows,
      or from an existing raster test helper the scout names) never changes a strip's backdrop key and rasterises 0
      backdrops after the first pass. — `raster::cache::tests::the_selection_never_misses_a_backdrop`.
- [ ] Equivalence (Decided 6) over that window's strips × every cursor row × `Backdrop::{OffThread, Faded}`, and `Full`
      with no thread (emphasis `None`, backdrop == legacy exactly). Prints each case's pixel-class counts.
      — `raster::tests::composite_matches_the_single_pass`.
- [ ] Emphasis is cropped to the span of lit cell columns (whole cells); `first_column` places it.
      — `raster::tests::emphasis_is_cropped_to_lit_columns`.
- [ ] `Lit` default on every row → `rasterise_emphasis` is `None`. — `raster::tests::no_paint_no_image`.
- [ ] Back-and-forth `j`/`k` over 10 rows rasterises each distinct selection's emphasis once.
      — `raster::cache::tests::emphasis_is_cached_by_selection`.
- [ ] `resident_bytes` counts both LRUs. — `raster::cache::tests::resident_bytes_counts_both_layers`.
- [ ] Cost (release, `scripts/fix-scroll-emphasis-cost.sh` → `docs/perf/44-scroll-emphasis-layer.txt`):
      `rasterise_emphasis` p99 per strip on the test window at 187 columns, against today's full strip; recorded, not
      asserted (the gate is the held-key test).
- [ ] PNGs for his eye: `docs/shots/m7/fix-scroll-thread-legacy.png` and `…-composite.png`, same strip, same cursor,
      `OffThread`, each ≤ 200 KB. — written and asserted by `raster::tests::thread_pngs` through
      `graft_fixtures::capture::capture_bytes`. **Right:** the same lit thread in both; off-thread lanes at the same
      half strength; no line wider than another. Colour **unverified** (rule 5).
- [ ] `crates/graft-graph/MAP.md`: every new item one line; the `StripCache` key line says backdrop, not selection.

## Files (exact)
- `crates/graft-graph/src/raster.rs` and `src/raster/*.rs` (scout lists; new `src/raster/emphasis.rs`),
  `src/raster/cache.rs`, their tests modules, `crates/graft-graph/MAP.md`.
- `docs/shots/m7/fix-scroll-thread-{legacy,composite}.png`, `scripts/fix-scroll-emphasis-cost.sh`,
  `docs/perf/44-scroll-emphasis-layer.txt`.
- graft-graph's dev-dependency on graft-fixtures (for `capture`) only if missing and allowed by ARCHITECTURE.md; say
  which.

## Touches
- Raster snapshots `tests/it/snapshots/it__raster__*`: untouched here (old path unchanged); the wiring ticket owns
  their change.
- `layout_hash` callers (grep): unchanged; `plain_layout_hash` is new.
- `ancestry::Ancestry`: read, not changed.
- Memory: two LRUs (box above).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
~65k. Split point if it compacts: backdrop + cache key (box 1) first; emphasis + composite second.
