# scroll-old-raster-deletion — delete the single-pass raster path; port its tests and bench to the layers

Status: ready
Build with: Sonnet — deletion and a port onto 44's fixed API; no design left, the oracle comparison is done in 46.
Needs: `46-scroll-strips-wiring` merged (the app no longer reads `RasterRow`/`StripCache::get`; `Backdrop::lowest`).
Blocked by: 46
Scout: `.scratch/m7-power-tools/scouts/46.md` §Disagreements 4, 5, 7, §Files. Re-locate lines by name.

File set: graft-graph only — `src/raster.rs` (delete `RasterRow` `:104-148`, `Strip` `:157`, `rasterise` `:209`, and
its `mod tests` `:361` where old-path only), `src/raster/emphasis.rs` (delete `Backdrop::of` `:51`),
`src/raster/cache.rs` (delete `get` `:250`, `contains` `:273`, the legacy LRU, and `layout_hash` `:490` if nothing
else calls it — check `plain_layout_hash` `:517` does not), `tests/it/strip_cache.rs`, `tests/it/emphasis_layer.rs`,
`tests/it/raster.rs`, `tests/it/raster_pixels.rs`, `tests/it/snapshots/it__raster__raster-*` (re-taken),
`benches/hot_paths.rs`, `crates/graft-graph/MAP.md`.

## Read first
- `46-scroll-strips-wiring.md` §Decided 2, 9, §Signatures.
- `44-scroll-emphasis-layer.md` §Signatures.
- `crates/graft-graph/MAP.md` `## raster` (`:117-130`), `## raster::emphasis` (`:132-141`), `## raster::cache`
  (`:143-161`).

## Scope
In: delete the old path; port every test and bench that used it to `PlainStrip` + `Selection` + `backdrop` /
`emphasis` / `composite`; re-take the raster snapshots through the composite.
Out: any change in crate `graft`; any change to the new path's behaviour.

## Decided
1. **The equivalence test goes with its oracle.** Quote `emphasis_layer::composite_matches_the_single_pass`'s green
   run at this ticket's base (it has 46's ruling-2 case), then delete it, `legacy` (`emphasis_layer.rs:158`) and
   `rasterise` together. From then the re-taken snapshots pin the composite's output.
2. **Tests that exist only for the old key go:** `strip_cache.rs` `emphasis_is_part_of_the_key` (`:250`) and
   `a_threaded_rows_own_emphasis_changes_neither_pixels_nor_key` (`:493`), and any `Backdrop::of` test (replaced by
   46's `lowest_backdrop_table`). Every other test using `RasterRow`/`rasterise`/`Strip` is **ported, keeping its
   name** — `raster.rs` (`:19, 69-83, 239, 294, 377, 441, 692, 727`), `raster_pixels.rs` (`:25, 125-249`). List each
   test in the report as deleted or ported.
3. **Bench:** `hot_paths.rs` (`:28, 190, 223`) measures `backdrop` + `emphasis` + `composite` for the same strips;
   bench id renamed to say so; old → new id in the report.
4. Snapshot names stay (`it__raster__raster-*`, 4 files); contents re-taken.

## Signatures
None added. Removed: `RasterRow`, `RasterRow::plain`, `Strip`, `rasterise`, `Backdrop::of`, `StripCache::{get,
contains}`, `layout_hash` (if unused). `CacheStats`/`len`/`resident_bytes` lose the legacy LRU.

## Acceptance (each box names its check)
- [ ] Definitions and uses gone — paste each command and its empty output:
      `rg -n 'pub fn (get|contains)\(' crates/graft-graph/src/raster/cache.rs`;
      `rg -n 'fn of\(\s*focus|Backdrop::of\b' crates/graft-graph crates/graft`;
      `rg -n '\bRasterRow\b|\bfn rasterise\(|\bstruct Strip\b|raster::Strip\b|\blegacy\(|emphasis_is_part_of_the_key|a_threaded_rows_own_emphasis' crates/graft-graph crates/graft`;
      `rg -n '\blayout_hash\b' crates/graft-graph/src` (empty, or name its caller and why it stays).
      A comment naming a deleted item is stale: rewrite it, don't exempt it.
- [ ] MAP clean — empty: `` rg -n 'RasterRow|`rasterise`|`Strip`|`of\(|legacy|StripCache::(get|contains)\b' crates/graft-graph/MAP.md ``
      and `` rg -n 'RasterRow|`rasterise`|Backdrop::of' crates/graft/MAP.md ``; `:136`, `:144`, `:154`, `:158`
      reworded.
- [ ] Equivalence run quoted before deletion (Decided 1).
- [ ] Snapshots `it__raster__raster-*` re-taken; report says per file what changed and why — expected only
      anti-aliased edges and lit-over-unlit crossings (ADR-0053 Consequences); anything else → stop and report.
      — the snapshot tests by name.
- [ ] Ported tests green under their old names — listed (Decided 2).
- [ ] Bench builds: `cargo bench --no-run`; numbers not asserted.

## Touches
- graft: compiles unchanged (workspace gates); `view::strips::tests` no longer reaches `cache.get` (46 ported it).
- `StripCache::stats` readers (graft `pump` tests, perf scripts parsing `rasterised`): name any whose counts move.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide; plus `cargo clippy --all-targets --features heavy-tests -- -D warnings`.

## Size
~45k.
