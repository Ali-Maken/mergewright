# scroll-old-raster-deletion — delete the single-pass raster path; port its tests and bench to the layers

Status: done
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
- [x] Definitions and uses gone — the six `rg` patterns run as perl regexes (no `rg` binary on this machine; same
      patterns, same paths) by `scratchpad/greps.sh`; every one printed nothing:
      `== 1 pub fn (get|contains)\(` / `== 2 Backdrop::of` / `== 3 old names` / `== 4 layout_hash` /
      `== 5 graft-graph MAP` / `== 6 graft MAP` / `== end` — no line between headers. `layout_hash` deleted (no
      caller; `plain_layout_hash` never called it). Stale comments naming deleted items rewritten in `raster.rs`,
      `emphasis.rs`, `cache.rs` (module doc, `lit_hash` doc), `emphasis_layer.rs`.
- [x] MAP clean — checks 5 and 6 above empty; `raster` / `raster::emphasis` / `raster::cache` sections reworded
      (`PlainStrip` described in full, `Backdrop` loses `of`, `StripCache` legacy lines dropped, stats/len/bytes say
      backdrop-only).
- [x] Equivalence run quoted before deletion (Decided 1) — `emphasis_layer::composite_matches_the_single_pass` at
      base decb1550: `192 cases drawn (856 cursors repeating an earlier paint skipped, the user's cell sampled 1 in
      4): 211 crossing pixels of 3724500 painted (5 per 100 000), 0 elements below the backdrop` / `test result: ok.
      1 passed`. Then deleted with `legacy`, `tally`, `one_case`, `no_thread`, `below_backdrop`.
- [x] Snapshots re-taken — `raster::strips_snapshot_at_a_pinned_cell` now draws
      through `layered` (backdrop + emphasis + composite), `folded_merge_snapshot` through `rasterise_backdrop` at `Full`; the four `it__raster__raster-*` files are **unchanged**:
      no selection → `Full` backdrop, no emphasis layer, same pixels. Changed, and checked:
      - `it__raster_pixels__raster-pixel-digests.snap` (`raster_pixels::strips_keep_every_byte`): only the two
        `wide-thread` lines (the one case with a selection). Classified at base with the old tally against the
        single pass: 17×37 Faded — clear differ 0, edge 91 879 px max Δ21 (over ceil(b·255) 0), crossings 5 px max
        Δ1; 8×16 — clear differ 0, edge max Δ20 (over 0), crossings 0; 0 elements below the backdrop. AA edges
        and crossings only.
      - `docs/shots/42-raster-thread-on-dark.png` (`raster::write_shots`): 938 px differ, max channel Δ18, all in
        lane 0's two cell columns (the lit lane) — its anti-aliased edges. Regenerated; user's eye, colour
        unverified (rule 5).
- [x] Ported tests green under their old names (Decided 2) — `cargo nextest run --workspace`: 5111 run, all
      passed after the capability citation fix (below). Per test:
      - **Deleted:** `strip_cache::emphasis_is_part_of_the_key`,
        `strip_cache::a_threaded_rows_own_emphasis_changes_neither_pixels_nor_key`,
        `emphasis_layer::composite_matches_the_single_pass`. No `Backdrop::of` test existed.
      - **Ported, `raster.rs`:** `strips_snapshot_at_a_pinned_cell`,
        `colour_never_exceeds_alpha_and_untouched_pixels_are_clear`, `a_fork_is_an_antialiased_curve_not_an_elbow`,
        `rings_differ_in_shape_whatever_the_colour`, `emphasis_fades_without_thickening_the_strokes`,
        `the_thread_is_alpha_not_a_wider_stroke`, `folded_merge_snapshot` (+ its folded-band tests),
        `reports_render_time_for_a_32_row_strip`, `write_shots`.
      - **Ported, `raster_pixels.rs`:** `strips_keep_every_byte`, `reports_per_strip_cost`.
      - **Ported, `strip_cache.rs`** (onto `StripCache::backdrop` at `Full`; every count assertion kept):
        `scrolling_one_row_rasterises_at_most_one_new_strip_and_scrolling_back_is_a_hit`,
        `a_cached_strip_is_the_strip_rasterise_draws`, `a_theme_change_flushes_and_misses`,
        `a_cell_size_change_is_a_different_key`, `a_layout_change_on_one_row_invalidates_only_its_strip`,
        `the_cache_is_bounded_and_evicts_the_least_recently_used`, `a_misaligned_or_oversized_strip_is_refused`,
        `resident_bytes_for_a_viewport_plus_buffer`, `strip_height_numbers` (ignored tool),
        `the_uncommitted_row_does_not_count_toward_the_strip_height`,
        `a_row_gaining_a_stub_changes_the_layout_hash` (on `plain_layout_hash`).
      - **Ported, `emphasis_layer.rs`:** `emphasis_is_cropped_to_lit_columns` — size, tightness and placement kept;
        its two single-pass comparisons went with the oracle (Decided 1) and are replaced by: lane 4's cell column
        lies in the crop, and the composite differs from the backdrop there. `thread_pngs` — composite PNG only;
        `docs/shots/m7/fix-scroll-thread-legacy.png` deleted (47 expects it gone). `emphasis_cost` — the
        `today-full-strip` series dropped.
- [x] Bench builds — `cargo bench -p graft-graph --no-run`: `Executable benches/hot_paths.rs` built.
      Ids: `rasterise/one strip` → `layers/backdrop+emphasis+composite one strip`; `StripCache::get/hit` →
      `StripCache::composited/hit`; `StripCache::get/miss` → `StripCache::composited/miss`. Measured under a
      lane-0 thread (`OffThread`), so the emphasis layer is drawn. Numbers not run.

## Touches
- graft: compiles unchanged (workspace gates); `view::strips::tests` no longer reaches `cache.get` (46 ported it).
- `StripCache::stats` readers (graft `pump` tests, perf scripts parsing `rasterised`): name any whose counts move.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide; plus `cargo clippy --all-targets --features heavy-tests -- -D warnings`.

## Size
~45k.

## NOTES (builder, 46b)
- Scout: used `.scratch/m7-power-tools/scouts/46.md` §Disagreements 4, 5, 7; no new scout (deletion within 46's
  surveyed file set).
- Outside the file set, needed for the workspace gate: `docs/capabilities/m1-the-graph.md:567` cited the deleted
  `a_threaded_rows_own_emphasis_changes_neither_pixels_nor_key` (`graft-capabilities` checker failed); now cites
  `strip_cache::tests::the_selection_never_misses_a_backdrop`. Also `docs/shots/42-raster-thread-on-dark.png`
  re-taken and `docs/shots/m7/fix-scroll-thread-legacy.png` deleted.
- Stale, not touched (outside file set): `scripts/fix-scroll-emphasis-cost.sh:11` still describes the
  `today-full-strip` layer; `docs/perf/44-scroll-emphasis-layer.txt` keeps its old lines (a record).
- `GRAFT_UPDATE_SHOTS=1` cannot regenerate `write_shots` past its first PNG: `check_png` panics on the first
  rewritten file, so later files are never reached. Regenerated with a local, reverted patch. Worth a gap row.
- Gates: fmt, clippy (workspace, all targets), clippy `--features graft/heavy-tests`, nextest workspace 5111/5111,
  `cargo test --doc` workspace — all pass.
