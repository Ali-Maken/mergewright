# pixel-roles-and-tall-row — the Pixel tier draws roles and the cursor row's second line; then the Ghostty lag is measured

Status: ready (Blocked by / Needs fitted to the 06 split, 2026-10-17)
Blocked by: 01-roles-and-route, 07-graph-rows, 07b-graph-rows-cursor, 06-graph-fold-and-lanes, 06b-lanes-on-screen-and-header, 03-ease-vocabulary
Build with: (blank — brownfield; conductor sets after the scout). Not Fable: correctness is judged by trace and tests, not by eye.
Needs: `LaneRole`, `Route` (roles-and-route); `lane_roles`, `SecondLine`, `commit_rows`, `AppState.{route, colours_off}` (graph-rows); fold rows and `InsideMerge` (06); the window's drawn columns with node-less lanes dropped (06b — strips draw what `Fold` keeps); role tokens (ease-vocabulary).

## Read first
- `docs/design-notes/ease-pass.md`: Mock behaviours G1, G2, G9, G11, G14; Genuinely risky tickets; Question 4 (answered a: build both, then measure; results go to him before anything is cut).
- `docs/design/03-graph-layout-algorithm.md` §3.8. ADR-0021 (release frame budgets), ADR-0053 amendment 1 (pieces).
- `crates/graft-graph/MAP.md`: `raster`, `raster::cache`, `fold`. `crates/graft/MAP.md`: `event_loop::graphics`, `view::strips`, `trace`. `crates/graft-term/MAP.md`: the Kitty/iTerm2 placement entries.
- `scripts/scroll-lag-big-repo.sh` header and `scripts/scroll-lag-pty.py` usage (the Ghostty-profile pty), `scripts/m8-trace-summary.sh` usage, `scripts/41-scroll-team-fixture.sh` usage.
- The scout's file.

## Scout questions (≤1 page)
- What a strip's key holds today (`plain_layout_hash`, `lit_hash`) and what invalidates it on `j`; how a strip is placed (by screen row?).
- Where the Unicode tier's second line (G) would sit relative to strip boundaries; whether placement can offset strips below the cursor by one cell without re-rasterising.
- How the pixel tier draws dashed/dotted strokes today (same-hue lanes, "shape/dash").
- Whether the strip key already depends on the window's `Fold` (06b drops node-less columns per window: a scroll that changes which columns hold nodes changes the drawn columns).

## Scope
In (graft-graph `raster`, `raster::cache`; graft `view::strips`, `event_loop::graphics`; `scripts/ease-pixel-lag.sh`):
- **Roles:** stroke and colour by `LaneRole` — This thick, Under thin, Trunk double, Other dotted grey; connectors in the joining branch's role. Roles enter the strip key, so a focus change re-rasterises; `j` within one branch does not. `colours_off` draws every role in the muted hue, strokes unchanged.
- **Tall row:** the cursor row's second line in the pixel tier — lanes continue straight through it; strips above unchanged; strips below placed one cell row lower (recommended: placement offset, no re-raster); the continuation band drawn as its own one-row piece. Builder may differ after the scout; say why.
- Commit in **two commits**: (1) roles, (2) tall row — the script measures each.
- **Measurement (Q4 a)** — `scripts/ease-pixel-lag.sh`, run by the builder **with no other cargo-running helper active**, writing `docs/perf/ease-pixel.txt`; the builder reads only its `SUMMARY` lines:
  - Builds release binaries at three commits — base (before commit 1), roles, roles + tall row — each in its own `CARGO_TARGET_DIR` outside `target/`. Base is after 06b, so the lane drop is in all three.
  - For each: the team fixture (`41-scroll-team-fixture.sh`), Pixel tier, probe answered like Ghostty (kitty), 120×40 and 200×50, `j` held at 30 Hz for 3 s, at open and settled (`scroll-lag-pty.py`); plus a focus-changing walk (cursor alternating branches every 5 rows).
  - `SUMMARY` per binary × size × walk: `input_age_us p50/p95/p99`, `draw_us p95`, `bytes p95`, `tail_ms`; then deltas vs base.
  - Last lines: the exact command for him to run in his own Ghostty (`MWR_PERF_TRACE=<file> mwr <copy of his work repo>`, hold `j` 3 s, quit) and the summariser line that turns it into `docs/perf/ease-pixel-ghostty.txt` (`scripts/m8-trace-summary.sh`).
- **No cut, no fallback, no switch** on the result. The numbers go to the conductor, who takes them to him with Q4's options (b)/(c) still open. ADR-0021 budgets are judged in the milestone review's release run, not here.
Out: Unicode tier (G), roles' meaning (R), the right pane (B), which columns are drawn (06b).

## Signatures
graft-graph `raster`:
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Stroke { Thick, Thin, Double, Dotted }
impl Stroke { pub fn of(role: LaneRole) -> Stroke; }
pub struct PlainStrip { /* existing fields */ pub roles: Vec<(BranchIdentity, LaneRole)> }   // absent → Other
/// The cursor row's second line inside a strip: lanes straight through, one cell row tall.
pub fn rasterise_continuation(strip: &PlainStrip, row: usize) -> Option<RasterFrame>;
```
graft app (`view::strips`):
```rust
pub struct TallRow { pub row: usize }   // history row with the second line; strips below it shift one cell row
```
`plain_layout_hash` includes `roles`; `rasterise_backdrop`/`emphasis_pieces` keep their signatures.

## Touches
Strip cache keys and hit rates (`CacheStats` in NOTES before/after); `RasterPool` buffer sizes; Kitty and iTerm2 placement; `event_loop::graphics` frees on resize; the frame trace fields; Pixel captures (`docs/shots/` pixel-tier text captures show placement only); memory budget (`scroll-pixel-memory.sh` run in the review, not here); 06b's per-window column drop (strip invalidation on scroll — say in NOTES whether it raised the miss rate at base).

## Acceptance
- [ ] `Stroke::of` table test; strip key differs when one lane's role changes and is equal across `j` within one branch (cache test, hit counter).
- [ ] Raster test: Other lane pixels are dotted (gaps every N px), Trunk two parallel strokes, This wider than Under (measured in the frame buffer, not by eye).
- [ ] Tall row: strips above the cursor are not re-rasterised on `j` (counter); strips below are placed one row lower (placement command test); the continuation frame has a straight segment in every lane column passing the row.
- [ ] `colours_off`: every lane pixel in the muted hue; stroke shapes unchanged.
- [ ] `scripts/ease-pixel-lag.sh` exits 0 and `docs/perf/ease-pixel.txt` has `SUMMARY` lines for all three binaries, both sizes, both walks, and the Ghostty command lines. The builder's report quotes only the `SUMMARY` lines.
- [ ] Capture: `docs/shots/ease-pass/main-pixel-120.txt` (placement text capture; the pixels are his to judge in Ghostty). Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide; plus `scripts/ease-pixel-lag.sh` run once, alone.

## Size
Large (~70k). Split point: roles (commit 1) here; tall row + measurement as a follow-on.
