# pixel-roles-and-tall-row — the Pixel tier draws roles and the cursor row's second line; then the Ghostty lag is measured

Status: ready (planner spec-only pass 2026-10-18, fitted to scout `.scratch/ease-pass/scouts/13-pixel-roles-and-tall-row.md` at c021e6a6: 5 mismatches ruled, `colours_off` settled by a test first)
Blocked by: 07b-graph-rows-cursor (`SecondLine`, `commit_rows`, `body_line`; the tall row's input). Built and merged: 01, 03, 06, 06b, 06s, 06t, 07 part (f2269602: `Look.route`, `Look.colours_off`, `lane_roles` at `view/entries/roles.rs:16`). 07c is not needed (scout). **Not beside 09**: 09 changes `view/frame/layout.rs:169` `graph_columns`, which sets the strips' width. Either order works; the later one re-takes its captures. The measurement runs with no other cargo-running helper active.
Build with: Opus — raster, cache key, placement and a perf script; the tall row's placement is a judgement the tests then pin (scout's call). Not Fable: correctness is judged by trace and tests, not by eye.
Needs: graph `LaneRole` (`role.rs:14`), `PlainStrip<'a>` (`raster/emphasis.rs:164-165`, `Clone, Copy`, borrowed fields), `StripKey` / `plain_layout_hash` (`raster/cache/key.rs:~14,123`), `stroke_width` (`raster/pen.rs:30`), tiny-skia dash support (`raster/geometry.rs:154-160`); app `look.route` via `frame_route` (`view/entries/roles.rs:27`, `pub(in crate::view)` — `view::strips` is inside `view`), `look.colours_off` (`update/state/look.rs:10`), `lane_roles`; strips `view/strips.rs` (palette :49, `graph_rows` :215, strip cut :279-318), placement `event_loop/graphics/layers.rs:142-175`; 07b `SecondLine`, `commit_rows`, `body_line`.

## Read first
- `docs/design-notes/ease-pass.md`: Mock behaviours G1, G2, G9, G11, G14; Genuinely risky tickets; Question 4 (answered a: build both, then measure; results go to him before anything is cut).
- `docs/design/03-graph-layout-algorithm.md` §3.8. ADR-0021 (release frame budgets), ADR-0053 amendment 1 (pieces).
- `crates/mergewright-graph/MAP.md`: `raster`, `raster::cache`, `fold`. `crates/mergewright/MAP.md`: `event_loop::graphics`, `view::strips`, `trace`. `crates/mergewright-term/MAP.md`: the Kitty/iTerm2 placement entries.
- `scripts/scroll-lag-big-repo.sh` header, `scripts/scroll-lag-pty.py` usage (Ghostty-profile pty), `scripts/m8-trace-summary.sh` usage, `scripts/41-scroll-team-fixture.sh` usage.
- `07b-graph-rows-cursor.md` Decided (second line, `commit_rows`, cursor never on the last line).
- The scout's file.

## Settle first — committed tests, before the code they decide
1. **`colours_off` reaches the strips** (scout: *could not confirm*; the palette comes from `session.theme.raster_lanes()`, `view/strips.rs:49`, not `Look`). Test `colours_off_mutes_every_strip_lane`: set `look.colours_off`, build the frame's strips, and assert every lane takes the muted hue and `theme_id` differs from colours-on. Expected red today. It must pass after commit 1: when `colours_off`, the palette is muted for every role, `theme_id` re-keys and strokes stay the same. NOTES records whether it was red.
2. **Two placements of one strip image** (M3, below). A placement-command test for **Kitty** and **iTerm2** on a strip holding the cursor row: two placements with disjoint source rows and one transmit. If the iTerm2 path must re-send the image for the second placement, the test records the bytes and NOTES says so. The perf script measures it; no fallback is chosen here.

## Decided here (the scout's mismatches)
- **M1 — `PlainStrip` stays `Copy`:** roles ride as a borrowed slice, `roles: &'a [(BranchIdentity, LaneRole)]`; an identity absent from it is `Other`.
- **M2 — the stroke enum is `LaneStroke`**, so it doesn't clash with `tiny_skia::Stroke` (`raster/geometry.rs:5`).
  - This thick, Under thin, Trunk double, Other dotted grey (tiny-skia dash, `geometry.rs:154-160`); connectors in the joining branch's role.
  - Width still comes from `stroke_width(cell_width)` per strip. The role scales it (This ×1.5, Under ×1, Trunk two ×1 strokes, Other ×1 dashed), so `pen.rs:112` "never varies with emphasis" still holds.
- **M3 — the cursor row sits inside a strip.** Strips are `strip_rows` history rows (`view/strips.rs:279-318`), so offsetting whole strips is not enough.
  - The cursor's strip is placed **twice from the one raster**: rows up to the cursor at `at`, rows after it at `at + 1`, each cropped by `rows: Range` (`layers.rs:142-175`).
  - Strips wholly below shift one cell row; strips above are unchanged.
  - The continuation band (lanes straight through, one cell row) is its own one-row piece.
  - `j` within one strip changes only placements, never the raster. Re-rasterising with a gap is **rejected**: it costs a raster per `j`.
- **M4 — heights from 07b:** `graph_rows` (`strips.rs:215`) counts with `commit_rows`, and screen line → row goes through `body_line`.
- **M5 — the route is `look.route` through `frame_route`** (view-internal, no visibility change). Roles enter the strip key: `plain_layout_hash` includes `roles`. A focus change re-rasterises; `j` within one branch does not.
- Commit in **two commits**: (1) roles + `colours_off`, (2) tall row. The script measures each.

## Scope
In: mergewright-graph `raster`, `raster::cache`; app `view::strips`, `event_loop::graphics` (placement); `scripts/ease-pixel-lag.sh`.
- **Measurement (Q4 a)** — `scripts/ease-pixel-lag.sh`, run by the builder **with no other cargo-running helper active**, writes `docs/perf/ease-pixel.txt`. The builder reads only its `SUMMARY` lines.
  - Release binaries at base (before commit 1), roles, and roles + tall row, each in its own `CARGO_TARGET_DIR` outside `target/`.
  - Each run: team fixture (`41-scroll-team-fixture.sh`), Pixel tier, probe answered like Ghostty (kitty), 120×40 and 200×50, `j` held at 30 Hz for 3 s, at open and settled (`scroll-lag-pty.py`); plus a focus-changing walk (cursor alternating branches every 5 rows).
  - `SUMMARY` per binary × size × walk: `input_age_us p50/p95/p99`, `draw_us p95`, `bytes p95`, `tail_ms`, then deltas vs base.
  - Last lines: the exact command for him to run in his own Ghostty (`MWR_PERF_TRACE=<file> mwr <copy of his work repo>`, hold `j` 3 s, quit) and the summariser line → `docs/perf/ease-pixel-ghostty.txt` (`scripts/m8-trace-summary.sh`).
- **No cut, no fallback, no switch** on the result. The numbers go to the conductor, who takes them to him with Q4's options (b)/(c) still open. ADR-0021 budgets are judged in the milestone review's release run.
Out: Unicode tier (07/07b/07c), roles' meaning (01), the right pane (09), which columns are drawn (06b).

## Signatures
mergewright-graph `raster`:
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LaneStroke { Thick, Thin, Double, Dotted }
impl LaneStroke { pub fn of(role: LaneRole) -> LaneStroke; }
#[derive(Clone, Copy)]
pub struct PlainStrip<'a> { /* existing fields */ pub roles: &'a [(BranchIdentity, LaneRole)] }   // absent → Other
/// The cursor row's second line inside a strip: lanes straight through, one cell row tall.
pub fn rasterise_continuation(strip: &PlainStrip<'_>, row: usize) -> Option<RasterFrame>;
```
app `view::strips`:
```rust
pub(crate) struct TallRow { pub(crate) row: usize }   // history row with the second line
// the cursor's strip yields two placements (rows ≤ row at `at`, rows > row at `at + 1`); strips below shift one row
```
`plain_layout_hash` includes `roles`; `rasterise_backdrop`/`emphasis_pieces` keep their signatures.

## Touches
Strip cache keys and hit rates (`CacheStats` in NOTES before/after); `RasterPool` buffer sizes; Kitty and iTerm2 placement (two placements per cursor strip); `event_loop::graphics` frees on resize; frame trace fields; Pixel captures (text captures show placement only); memory budget (`scroll-pixel-memory.sh`, review); 06b's per-window column drop (whether it raised the miss rate at base, NOTES); 09's pane width (strip width).

## Acceptance
Each tick names its test inline.
- [ ] Settle tests 1–2 committed first; outcomes in NOTES.
- [ ] `LaneStroke::of` table test. The strip key differs when one lane's role changes and is equal across `j` within one branch (cache test, hit counter).
- [ ] Raster test: Other lane pixels dotted (gaps every N px), Trunk two parallel strokes, This wider than Under (measured in the frame buffer, not by eye). `PlainStrip` still `Copy` (compile).
- [ ] `colours_off`: every lane pixel in the muted hue, stroke shapes unchanged (`colours_off_mutes_every_strip_lane`).
- [ ] Tall row: strips above the cursor not re-rasterised on `j` (counter); the cursor's strip placed twice with disjoint rows, strips below one row lower (placement command test, both protocols); the continuation frame has a straight segment in every lane column passing the row.
- [ ] `scripts/ease-pixel-lag.sh` exits 0; `docs/perf/ease-pixel.txt` has `SUMMARY` lines for all three binaries, both sizes, both walks, and the Ghostty command lines. The report quotes only `SUMMARY` lines.
- [ ] Capture: `docs/shots/ease-pass/main-pixel-120.txt` (placement text capture; the pixels are his to judge in Ghostty).

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide; plus `scripts/ease-pixel-lag.sh` run once, alone.

## Size
~70k. Split point: roles + `colours_off` (commit 1) here; tall row + measurement as a follow-on.
