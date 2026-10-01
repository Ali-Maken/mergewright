# scroll-pixel-memory-budget — Pixel tier stays under 300 MB on a big repo

Status: **parked** (planner, 2026-10-02; 47 ruling 2). Gap `M7/47-pixel-rss`, **broken**, expiry M8. Unpark at the
latest at M8's planning, or earlier if the user asks.
Build with: Fable (signature). It changes `StripCache`'s capacity contract (graft-graph) and `KittyImages`' upload
identity (graft-term), and graft's event loop calls both. The exact signatures are confirmed by the step 2 spec-only
pass.
Needs: 46c merged (fold churn); `48-scroll-cursor-under-uncommitted-row` merged (it changes the strip viewport).
Blocked by: parked.

## Need
- Who: him, in Ghostty (Pixel, Kitty with deflate), on his repo.
- Budget: PROMPT.md §7.1, "a peak-memory test that fails if loading and scrolling the 100k repo uses more than 300 MB".
  It is not loosened.
- Today: on MONOREPO at 187×50, with `j` held for 3 s, Graft's RSS is 367–396 MB (`docs/perf/47-debug.txt:5,11-13`;
  47 box 5). The Unicode tier uses 24 MB on the same run.
- History: 977 MB at 0d38d93d.
- Not felt yet. He has not reported memory. The cost is the stated budget, and what happens on a wider graph or a
  longer session.

## What holds the memory (debugger, `47-debug.txt:18-20`; lines from 47 NOTES)
1. **Strips are counted, not weighed.**
   - `event_loop/graphics.rs:33` sets `STRIPS_KEPT = 16`. That was sized for 12 lanes at about 1.9 MB a strip.
   - On MONOREPO a backdrop is about 4.1 MB, and since ADR-0053 each strip has two layers.
   - StripCache holds 117.8 MB live.
2. **Evicted uploads stay alive.**
   - `graft-term/src/kitty/mod.rs:123`: `Upload::frame: Arc<RasterFrame>` keeps pixels only so it can compare
     pointers.
   - That is 45.2 MB of frames the cache has already evicted.
3. **Freed large blocks stay resident.**
   - macOS malloc keeps 166 MB of freed large blocks in its free cache.
   - The `Weak` experiment cut live memory by 45 MB and RSS by nothing: the freed buffers just moved into that cache.
   - So a fix that only frees memory sooner does not close this. Allocation churn has to stop.

## Steps (each produces a file; the next refuses to start without it)
1. **Measure, by script** (`scripts/scroll-pixel-memory.sh` into `docs/perf/scroll-pixel-memory-budget.txt`;
   summary lines only). Build on `scripts/47-debug-rss.sh` and `scroll-lag-pty.py MEM_OUT=`.
   - Per layer (backdrop, emphasis, composite): bytes resident and the number of rasterisations.
   - Large allocations per held second.
   - `Malloc Large (empty)` resident.
   - Runs: MONOREPO 187×50, plus a wider and a narrower graph, each with deflate on and with iTerm2.
2. **Planner, spec-only pass:** the chosen shape and its exact signatures. It reads the MAP.md files only.
3. **Build** (Fable, signature), then **`scripts/47-scroll-acceptance.sh`** step 3 to judge.

## Shape to confirm in step 2 (proposed, not decided)
- **A byte budget, not a strip count.**
  - `StripCache` takes a byte budget shared across its three LRUs and evicts by bytes (`resident_bytes` exists).
  - `capacity_for` keeps its job: the strips a viewport plus its buffer needs.
  - If the budget cannot hold that minimum, the minimum wins and the run prints it. A cache that cannot cover the
    viewport resends every frame, and that breaks the held-key verdicts.
- **An upload identity, not pixels.**
  - `KittyImages` compares uploads by a stable id from the strip key, and holds no `Arc<RasterFrame>` after transmit.
- **Reuse buffers.**
  - Strip rasterisation draws into a buffer lent by a pool sized to the strip, and eviction returns it to the pool.
  - Steady-state held `j` then allocates no new large blocks, so malloc's free cache stops growing.
  - Rule 2: this is the cause (churn), not a call that asks malloc to release memory, which would treat the
    symptom.

```rust
// graft-graph (proposed)
pub struct ByteBudget(pub usize);
impl StripCache {
    pub fn with_budget(budget: ByteBudget, min_strips: usize) -> StripCache; // replaces new(capacity)
}
pub struct RasterPool { /* buffers of one strip size; flushed with the cache */ }

// graft-term (proposed)
pub struct UploadKey(pub u64); // from the strip key; Upload keeps this, not Arc<RasterFrame>
```

## Acceptance (for the eventual build; restated in the step 2 spec)
- [ ] 47 box 5 ticks: pty `kitty-yes` free and 40 MB/s show **Graft RSS ≤ 300 MB** (`scripts/47-scroll-acceptance.sh`
      step 3, `SUMMARY pty`). Also printed: deflate-no and iTerm2.
- [ ] Heavy peak-memory test at Pixel: PROMPT §7.1's `getrusage` test with a Pixel run, MONOREPO, held `j`, ≤ 300 MB.
      Add it if `heavy::budgets_hold_*` has no Pixel variant (check by name).
- [ ] Fast-tier guard: after held `j` on wide strips, `StripCache::resident_bytes() ≤` the budget, and the pool
      allocates nothing new in the steady state. New test, named in the step 2 spec.
- [ ] Nothing gets slower. `pixel_scroll::{a_capped_graph_resends_strips_only_when_the_fold_changes,
      j_inside_resident_strips_sends_no_strip}` stay green. The 47 script's `held_key` verdicts and transmit counts
      are no worse than `docs/perf/47-scroll-acceptance.txt`.

## Touches (for step 2 to complete)
- graft-graph: `StripCache::{new, capacity_for, flush, resident_bytes}`, the rasteriser's output buffer.
- graft-term: `KittyImages::{frame, answer, forget_uploads, free_all}`; and iTerm2, which has no upload cache and
  should be checked.
- graft: `event_loop/graphics.rs` (`STRIPS_KEPT`, the idle prefetch); `view::strips`.
- Docs: DESIGN.md §3.8 "The key" / cache sizing; `docs/adr/0053`; `TESTING.md` "The strip bound"; MAP.md for all
  three crates.
- `46d-scroll-kitty-without-deflate`: the raw path's frames are larger. Sequence these two so only one changes the
  upload struct at a time.

## Size
Unknown until step 2. Expect a signature ticket plus one wiring ticket.
