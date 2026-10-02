# Scout 04 — pixel memory budget (HEAD 261c6be0)

## Scout answers
- **Raw path frame copy:** the Kitty raw path holds no copy of its own after transmit. `encode::transmit` (`graft-term/src/kitty/encode.rs:28`) makes a temporary `straight_alpha` Vec (`:155-156`, `vec![0; len]`) on every transmit. On the raw path that Vec becomes the payload (`:34-35`), then base64 into `encoded` (`:37`), then `write_command` copies it again (`:137`). All of these are dropped when the call returns. What persists is `Upload.frame: Arc<RasterFrame>` (`kitty/mod.rs:123`, set at `:290`), and it is kept on both paths, deflate or raw.
- **Upload identity today:** `KittyImages.uploads: Vec<Upload>` (`kitty/mod.rs:134-135`). An upload is matched by `(first_row, layer)` plus `Arc::ptr_eq` on the frame (`:270-276`). There is no hash or key. iTerm2 does the same: `ItermImages` holds `frame: Arc<RasterFrame>` (`iterm2.rs:37`, `:164`) and checks `ptr_eq` (`:141`).
- **Rasteriser output buffer:** `Pixmap::new(width, height)` at `graft-graph/src/raster.rs:202`, once per strip rasterisation. It is moved into `RasterFrame.pixels` via `pixmap.take()` at `:225`. The empty case is `Vec::new()` at `:198`. `pen.rs:232` makes a small `to_vec` per sample.

## State and sizing
- `StripCache` (`raster/cache.rs:180-189`) has a count capacity (`:182`) shared by 3 LRUs: backdrops, emphasis, composites (each `Arc<RasterFrame>` or image). Eviction is by count in `Lru::insert` (`:147-148`). `new(strip_rows, capacity)` is at `:197`, `resident_bytes` at `:405`, `flush` at `:382`.
- `capacity_for` (`:211`) has **no caller in code**. It is mentioned only in a comment, `graft/src/event_loop/graphics.rs:30`.
- `STRIPS_KEPT = 16` (`graphics.rs:33`). It sets `KittyImages::new(2*16)` (`:84`), `ItermImages::new(16)` (`:85`) and `StripCache::new(STRIP_ROWS,16)` (`:86`). `prefetch` is at `:204`.
- Kitty evicts by count (`kitty/mod.rs:317-319`).

## Where the ticket's assumptions disagree with the code
- The ticket's Touches line says "`capacity_for`'s viewport minimum wins". `capacity_for` is not called anywhere today (`graphics.rs:30` is a comment). The budget must be wired in, not just adjusted.
- The ticket says `UploadKey` comes "from the strip key". `KittyImages` gets `Place { frame: &Arc<RasterFrame> }` (`kitty/mod.rs:81`), and `Place` carries no strip key today. The caller in `graft` must supply one, so the `Place` signature changes. This touches iTerm2 (`iterm2.rs:141`) and every test that builds a `Place`.
- `straight_alpha` allocates a full-strip Vec on every transmit (`encode.rs:156`). It is a large per-send allocation outside the cache, and `RasterPool` as specified does not cover it.
- The heavy tier has no Pixel variant. `heavy.rs:52,57,143,148,158` are all `budgets_hold_*` tests, none at Pixel.

## File set
- graft-graph: `src/raster.rs`, `src/raster/cache.rs`, `src/lib.rs`, `MAP.md`
- graft-term: `src/kitty/{mod,encode,tests}.rs`, `src/iterm2.rs`, `src/lib.rs`, `MAP.md`
- graft: `src/event_loop/graphics.rs`, `src/view/strips.rs`, `src/event_loop/tests.rs` (prefetch tests `:1218-1252`), `tests/it/heavy.rs`, `MAP.md`
- Scripts and docs: `scripts/scroll-pixel-memory.sh` (new), `scripts/47-scroll-acceptance.sh`

## Overlap with 03
- 03 holds `update` (cursor motion, detail follow) per its `03-settle-work-at-rest.md:11`, and touches `held_key` and `pixel_scroll` tests and `crates/graft/MAP.md` (`:39-42`). Shared with 04: `crates/graft/MAP.md`, and possibly `event_loop/tests.rs` if 03 adds tests there.
- Could not confirm whether 03 edits `event_loop/graphics.rs` or `view/strips.rs`. 03 lists no file paths.
