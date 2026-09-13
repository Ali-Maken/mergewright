# m1-the-graph/30g — Pixel strips: rasterise cheaper, in the same frame

Status: ready
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
- [ ] Pixel guards added first, green before and after.
- [ ] Per-strip cost before/after recorded.
- [ ] Release frames run: pixel tiers pass (numbers recorded; one run, not the counted seven).
- [ ] Gates pass (fmt, clippy default + heavy-tests, test).

## Size
Under ~80k tokens. Stop at a clean seam (A done, B not started is a good one) with a handoff note rather than compacting.
