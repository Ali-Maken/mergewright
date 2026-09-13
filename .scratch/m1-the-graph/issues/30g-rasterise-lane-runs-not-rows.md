# m1-the-graph/30g — Pixel strips: rasterise cheaper, in the same frame

Status: done (A; B not needed)
Build with: **Opus** (high) — rasteriser hot path under the frame budget (split from 30f seam 2)
Blocked by: none
Leaves runnable: yes — pixel page keys fit 16 ms in release with rows and graph still in the same frame.

Why: 30f seam 2 (commits 72524df, 5b9fd6d). After the measurement fix, the pixel tier still misses 16 ms on page keys that rasterise 2–3 strips (3 strips: own frame 10.8 p50 / 14.7 max, plus 1–2 ms queued). Rasterise is ~70 % (2.3–3 ms p50 per strip). Full table and options A–D: ticket 30f, "Seam 2".

**Conductor decision (2026-09-13):** option A, then B if pixel still misses. Both leave what the user sees unchanged (graph and rows arrive together), so they're implementation calls, not user preferences. C (graph a frame after rows) is out: it's the blank-graph frame the user flagged in 30b and 37. D (accept >16 ms) is a spec change and goes to the user only if A and B both fail.

## Read first
- Ticket 30f "Seam 1" and "Seam 2" (numbers, scout findings: `Pen::row` strokes per lane per row, fresh `PathBuilder`/`Path` per segment, fresh zeroed `Pixmap` per miss)
- ADR-0021; ADR-0024 (accepted: one stroke width, emphasis by alpha — ticket 42 lands later; don't pre-empt it, don't make it harder)
- `crates/graft-graph/MAP.md`, `crates/graft/MAP.md`; scouts for source

## Scope
In:
- **Guards first:** byte-for-byte pixel guards on representative strips (straight lanes, merges, forks, octopus, wide ~28-lane) before any change; they must hold after (or, if anti-aliasing seams legitimately change, stop and show the before/after capture for the user — never re-bless snapshots silently).
- **A:** stroke each lane run across consecutive rows once instead of per row; reuse path builders and the pixmap buffer. Measure per-strip cost before/after (bench + the frames breakdown).
- Release frames run (frames:: alone). If pixel 100k/50k still miss → **B:** rasterise the strips above and below the viewport while idle on the loop thread, only when no input is waiting, checking input between strips; keys still rasterise on the key path when prefetch hasn't reached them. Test: a key arriving mid-prefetch waits for at most one strip.
- Report each fix: cause or symptom, and what it costs the user (B: idle CPU after each scroll stop).
- If A and B both leave misses: stop, record numbers, conductor puts D to the user.

Out: the 7 counted release runs, proofs, docs (30f seam 3); stroke width/alpha (42); fold stubs (40).

## Acceptance
- [x] Pixel guards added first, green before and after.
- [x] Per-strip cost before/after recorded.
- [x] Release frames run: pixel tiers pass (numbers recorded; one run, not the counted seven).
- [x] Gates pass (fmt, clippy default + heavy-tests, test).

## Size
Under ~80k tokens. Stop at a clean seam (A done, B not started is a good one) with a handoff note rather than compacting.

## Done (2026-09-13, builder-opus) — option A, B not built; no compaction

**Guards first** (commit 7664a62): `crates/graft-graph/tests/it/raster_pixels.rs::strips_keep_every_byte`. FNV-1a digests of the full RGBA buffer for 8 strips at 17×37 and 8×16 (foxtrot, criss-cross, octopus, a 1-lane chain, 7 lanes mid-history with `above`, 29 lanes mid-history, 29 lanes with per-column thread + dim/highlight rows, 12 columns with a `+17` marker). Captured before any change; unchanged after A1 and A2 (never re-blessed). `reports_per_strip_cost` prints release per-strip cost.

**Cause found by timing, not the scout's guess.** A1 (one stroke per straight run, commit 77f809b) barely helped: wide 4.85 → 4.47 ms. Phase timers (`target/30g/diag-phases.txt`, reverted): tiny-skia anti-aliasing costs **per painted scanline**, not per stroke call. A 29-lane strip spent 3.9 ms of 4.5 in lane columns even after merging runs.
**Fix, A2** (commit 55ffed2, `raster.rs` `Pen::straight`/`Pen::scanline`): each vertical line's scanline is the same between whole-pixel bounds, so it's rasterised once per (x, ink) on a clear 3-row sample and copied wherever the pixels under it are still clear. Rows already drawn on (an arc crossing) are stroked by tiny-skia as before, so its blending is kept. Byte-identical per guards. User cost: none (same pixels; tens of KB of scanline samples live only for one rasterise).

**Per strip, release, 32 rows, 17×37, p50 (files `target/30g/per-strip-{before,after-A1,after-A2}.txt`):**

| Strip | Before | A1 | A2 |
|---|---|---|---|
| 2 lanes | 1.20 ms | 1.21 | 1.13 |
| 7 lanes | 1.51 | 1.49 | 0.87 |
| 29 lanes | 4.85 | 4.47 | 1.10 |

What's left per strip is node discs (~0.35–0.66 ms, two anti-aliased fills per row) and lane stubs (~0.18–0.35 ms; they end at a fractional y, so they aren't copied).

**Release frames run** (frames:: alone, one run, not counted; `target/30g/release-frames-A.txt`): **4/4 pass.** Pixel 100k p50 7.0 / p99 14.3 / **max 15.8 ms**; Pixel 50k 7.8/11.9/13.4; Unicode 100k 5.8/8.9/9.4; Unicode 50k 7.0/12.6/14.1.
**B not built:** the ticket gates it on a miss. **Risk for 30f's seven counted runs:** pixel 100k max is 0.2 ms under budget, so one run proves little. If a counted run misses, build B next (spec unchanged), or copy node discs the same way (they are identical per ring/ink/sub-pixel offset, but they erase via `BlendMode::Clear` over non-uniform pixels, so that needs its own byte guard).

Gates: fmt, clippy (default + heavy-tests), `cargo test` — see `target/30g/gate-*.txt`.
