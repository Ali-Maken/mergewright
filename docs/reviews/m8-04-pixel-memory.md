# Review — m8-polish-and-ship 04, pixel memory budget (one round)

Reviewed at main `8c3cb9f6`. Diffs: `938cb611^1..938cb611` (pass 1) and `83267c65^1..83267c65` (continuation 1).
Scout read: `.scratch/m8-polish-and-ship/scouts/04-pixel-memory.md`. Reviewer worktree was detached, with its target
dir outside it. Both were removed after the review.

**Counts: Critical 0 · Major 5 · Minor 5 (one of them a question).**

## Gates (what was run, and how)

| Check | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo clippy --all-targets --features graft/heavy-tests -- -D warnings` | pass (heavy code, `heavy.rs` / `held_key.rs` new tests included) |
| `cargo nextest run --workspace` | **5373 passed, 9 skipped**, 837 s nextest / 940 s wall including build, **load 20–30 on 10 cpus** (other agents), so the time is not comparable to the ~50 s baseline. Peak RSS (`/usr/bin/time -l`) 1.37 GB. The 5 slowest tests were not captured: the output was filtered to the summary, and a rerun under this load would measure the load, not the tests. |
| `cargo test --doc --workspace` | 3 passed, 0 failed |
| `target/` (reviewer's) | 5.5 GB |
| Heavy tier, release perf run | **Not run**. The conductor's instruction for this round was never to run perf or memory measurements; they run from the milestone review's script. Load was 10–30 throughout anyway. So box 3 (`heavy::budgets_hold_at_pixel_on_monorepo_with_j_held_{free,40mb}`) is still **never run** and box 5 is unverified. Both stay open, under gap `M8/04-quiet-measure`. |
| Fuzz / bench | none touched by this ticket (`benches/hot_paths.rs` only had its constructor converted) |

## Ticks spot-checked: 2 checked, 2 held

- Box 1 (step 1 before code): `899f4c1a` is an ancestor of `f8739219` (`git merge-base --is-ancestor`), 7 minutes
  earlier, and its stat shows only the script, the patch and the perf file. **Held.**
- Box 4 (fast tier): all three named tests exist at HEAD (`pixel_scroll.rs:195`, `strip_cache.rs:338`, the kitty test)
  and passed in the nextest run above. The tick's own wording, "budget or floor every frame", matches the assertion at
  `pixel_scroll.rs:225`. **Held as worded, but the graft-level test is weak** (Major 1).
- Open boxes 2, 3 and 5 claim nothing. The NOTES record every under-load number as "recorded, not judged". No tick
  rests on a measurement taken under load.

## Critical

None.

## Major

**1. The graft-level "byte budget" test never sees the budget bind; it tests the floor.**
`crates/graft/src/event_loop/tests/pixel_scroll.rs:212` sets the budget to 1.5× the first frame. The floor is
`capacity_for(32, ~37 rows, 32)` = 5 entries per layer (`graphics.rs:159-163`), and that weighs more than 1.5 frames.
I verified it by instrumenting the test in the reviewer worktree with a per-frame `eprintln` of
`strip_budget_report`, so this is observed, not inferred. On **210 of 230** frames `minimum_wins` was true. At the end,
4,147,200 bytes were resident against a 2,488,320-byte budget, and every eviction came from the floor. The assertion
`resident <= budget.0 || minimum_wins` (`:225`) therefore passes through its second operand. The "premise: the budget
bound" check (`:236`) is satisfied by floor-driven evictions, so the premise is not what its message says. The real
loop at the user's 17×37 cell sits in the other regime: `minimum_wins_frames=0/127`
(`docs/perf/scroll-pixel-memory-budget.txt:29`). So no fast test in the `graft` crate shows the regime that ships,
and the "pool steady" check (`buffers_allocated` unchanged) is proven only under floor-bound eviction. The cache-level
test `strip_cache.rs:338` does make the budget bind: budget 4 strips, floor 2.
*Why it matters:* the test name and `TESTING.md` "The strip memory bound" both read as a proof of the byte budget at
the app level, and it isn't one.
*Fix:* set the budget above the floor's weight, for example the floor's weight plus two strips. Then assert that at
least one eviction happened with `minimum_wins == false`, and that `resident <= budget` on every frame after warm-up.

**2. Every pooled entry weighs the largest strip since the last flush, so the budget holds about half the strips the
code claims.**
Cause, from the code: `crates/graft-graph/src/raster/pool.rs:41` raises `size` to the largest request, and `:47` makes
every new buffer `Vec::with_capacity(self.size)`. `crates/graft-graph/src/raster/emphasis.rs:417` crops the emphasis
image **in place** with `truncate`, which keeps the full-strip capacity. `cache.rs` weighs entries by `capacity()`.
Evidence: per-entry weight went from 4.25 MB backdrop / 2.4 MB emphasis
(`scroll-pixel-memory-budget.txt:8`, `16/68MB`, `16/39MB`) to **5.6 MB / 5.6 MB** (`:29`, `8/45MB`, `8/45MB`). At
24×52 it went from 8.6 MB to 11.2 MB (`:10` vs `:31`). So:
- `crates/graft/src/event_loop/graphics.rs:37-39` says "keeps ~14 strips of both; at a 24×52 cell, ~7". Measured
  is 8 + 8 and 5 + 5. The comment is false.
- At 24×52 the floor of 5 + 5 entries at 11.2 MB is 113 MB, over the 96 MiB budget, which is why `minimum_wins` fired
  on 38/77 and 64/106 frames. With cropped-size emphasis the floor would be about 5 × (8.6 + 4.9) ≈ 67 MB, under
  budget.
*Why it matters:* it costs memory and adds re-rasterisation: 11 strips were evicted and drawn again during one hold
(`:29`). It is also why the floor wins at a bigger font, and the docs hide it.
*Fix:* a decision for the builder. Options: give emphasis images their own size class in the pool, keep the cropped
copy in its own exact buffer, or accept the cost and correct the `graphics.rs:37-39` comment, the ADR-0053
consequence line (2.4 MB) and DESIGN §3.8. Whichever is chosen, name it in DESIGN §3.8.

**3. The reference that box 5 compares against was overwritten by an under-load run.**
Box 5 reads "no worse than `docs/perf/47-scroll-acceptance.txt`". Commit `af004d70` replaced that file: its header now
reads `# HEAD 1c83f5a8` and `# load before: 39.24 32.20 35.90`. The quiet baseline (`a4d1f93b`, load 1.11, Pixel-deflate
`tail_ms=47`, `strip_transmits_held=81`) now exists only at `453cd384:docs/perf/47-scroll-acceptance.txt`.
*Why it matters:* the quiet re-run that `M8/04-quiet-measure` waits for would be judged against a run taken at load
39. A regression could pass, and so could a number inflated by the load. The script writes to the same path every
time, so this will happen again.
*Fix:* make box 5 and the gap row name `453cd384:docs/perf/47-scroll-acceptance.txt` as the reference. Better, have
the script write each run to a dated or per-HEAD file and keep the baseline fixed.

**4. `crates/graft-graph/src/raster/cache.rs` grew from 496 to 705 lines** of non-test code (`git show
938cb611^1:…| wc -l` vs HEAD). PROMPT.md 6.2 says about 400 lines per file. It was already over, and no gap row names
it.
*Fix:* split by responsibility. `Lru` with its weight and eviction (`Entry`, `oldest`, `evict_oldest`) goes to
`cache/lru.rs`. Key building (`layer_key`, `backdrop_key`, `paint_key`, `one_number`, `plain_layout_hash`,
`lit_hash`) goes to `cache/key.rs`. Or add a size row naming it.

**5. An observed RSS over budget has no gap row.** At a 24×52 cell, RSS was 434 MB on one of two runs, with Malloc
Large (empty) at 267 MB (`scroll-pixel-memory-budget.txt` reading lines, ticket Findings 3). `M8/04-quiet-measure`
covers only 17×37, with kitty-yes, kitty-no and iTerm2. CLAUDE.md rule 4 says unverified work goes in
`KNOWN_GAPS.md`. A user with a bigger font crosses PROMPT 7.1's 300 MB through the floor and freed-block retention,
and nothing tracks that.
*Fix:* add a row, **unverified** (taken at load 10–30). Link it to Major 2, because cropped-size emphasis would bring
the floor back under budget.

## Minor

1. **Keys computed twice per strip per frame.** `graphics.rs:302` (`paint_key`) and `:307` (`backdrop_key`) each
   rebuild `layer_key`, which hashes every row (`plain_layout_hash`, `cache.rs:529`). Then `backdrop()` (`:308`) and
   `emphasis()` (`:310`) rebuild it again. That is 4 layout hashes and 2 `lit_hash` per strip, against 2 and 1 before.
   The cost is small but it lands on the held-`j` hot path. Fix: have `backdrop` and `emphasis` / `composited` return
   their key with the frame.
2. **`type Layers` is a tuple of tuples** (`graphics.rs:316`), destructured as `Ok(((frame, key), emphasis))`. A
   two-field struct (`base: Keyed`, `emphasis: Option<(Keyed, u16)>`) reads better (PROMPT 6.2).
3. **Transmit scratch is held for the life of `KittyImages`** (`kitty/mod.rs:152`). It is never released by
   `free_all`, `hide` or a tier change, and it is outside the byte budget: 14 MB at 17×37, 21 MB raw, 28 MB at 24×52
   (perf `scratch_mb`). Fix: drop it in `free_all`, or document it in DESIGN §3.8 as a fixed overhead.
4. **Question (rule 7, no confirmed line):** why is the pool not steady in the real loop? `buffers_allocated` rose by
   19 to 116 during a 3 s hold (gap `M8/raster-pool-allocs`). Two candidates I could not confirm without a
   measurement:
   - (a) `pool.rs:41` and `:58`. When the fold's `drawn_columns` changes width (`fold.rs:153`, where the rail marker
     comes and goes), `size` rises, and every older, smaller buffer given back is dropped.
   - (b) `MAX_SPARE = 4` (`pool.rs:18`). An emphasis entry that paints nothing caches `None`, which holds no buffer.
     As `None` and `Some` entries turn over, the count of live buffers swings by more than 4, so spares are dropped
     and then made again.

   The fast test cannot see either, because its strips have one width and are floor-bound (Major 1).
5. **`KNOWN_GAPS` `M7/47-pixel-rss` still quotes pre-04 numbers (367–396 MB) as broken** and does not point to
   `M8/04-quiet-measure`, which holds the current state (251/252 MB under load). The conductor should cross-link them.

## Feature pipeline

This is a re-homed M7 performance ticket with no new screen, keys or journey, so it has no design-note journey to
count. The prior-art and journey steps do not apply to it. Step 1 (measure before building) was followed and is
evidenced by commit order.

## Touches, checked

- `StripCache::new` is gone. A grep finds no caller. The bench, `emphasis_layer`, `ghostty_shot`, `strips/tests` and
  `kitty_check` were all converted.
- `Arc::ptr_eq` no longer appears in `graft-term`. Kitty and iTerm2 both match on `UploadKey`. iTerm2 bands use
  `paint_key` (`graphics.rs:302-305`), which covers backdrop alpha and selection, so a composited band is keyed
  correctly.
- Idle prefetch (`graphics.rs:215`) walks only the viewport strips (`pixel_rows`, `strips.rs:217`). That is at most
  the floor, so it cannot evict what it just drew.
- `Arc` holders of `RasterFrame` outside the cache: only `Drawn` within one frame (`graphics.rs:326-329`), so
  `try_unwrap` on eviction fails only within a frame.
- Docs updated: DESIGN §3.8, ADR-0053, `TESTING.md` "The strip memory bound" and all three MAP.md files. The
  `graphics.rs` comment is wrong (Major 2).
- Out-of-test `unwrap`/`expect`: none added. No change to terminal escapes; the status text still goes through
  `sanitize_line`. No git work involved.
