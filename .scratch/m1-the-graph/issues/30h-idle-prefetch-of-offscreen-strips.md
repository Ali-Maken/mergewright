# m1-the-graph/30h — Idle prefetch of the strips above and below the screen

Status: done
Build with: **Opus** (high) — event loop, wakes, frame budget (option B from 30g, mandated by 30f seam 3)
Blocked by: none
Leaves runnable: yes — pixel page keys find their strips already rasterised; nothing the user sees changes.

Why: 30f seam 3 (commit 3a0263d). Counted release runs 1–2 passed (max 15.3, 15.7 ms); run 3 missed: Pixel 100k key 226 at 16.8 ms (6.5 ms queued + 10.3 ms own frame, 2 strips drawn inside the key's frame). Per 30g's scope, a miss after option A means option B. Outputs: `target/30f/counted-{1,2,3}.txt`.

## Read first
- Ticket 30f "Seam 3" handoff (exact places: the `Wakes` trait, frames.rs's test wake source, `pixel_rows` for off-screen strips; rows above/below are already fetched, History needs no change)
- Ticket 30g scope, option B paragraph; ADR-0021
- `crates/graft/MAP.md`, `crates/graft-graph/MAP.md`; scouts for source

## Scope
In:
- After a frame, only when no input is waiting: rasterise the strip above and the strip below the viewport on the loop thread, checking for input between strips. Keys still rasterise on the key path when prefetch hasn't reached them.
- Tests: a key arriving mid-prefetch waits for at most one strip; prefetch never runs while input is queued; the prefetched strips are the ones a PgUp/PgDn then uses (cache hit).
- Note key 226's 6.5 ms queue: say whether prefetch work could ever be what a key queues behind, and prove it can't exceed one strip.
- MAP.md for any public change. Gates.
- Report: cause or symptom, and what it costs the user (idle CPU after each scroll stop, in ms and strips).

Out: the counted release runs, proofs, docs (30f seam 4).

## Acceptance
- [x] Tests above, red before, green after.
- [x] One uncounted release frames run recorded (target/30h/).
- [x] Idle cost measured and stated.
- [x] Gates pass (fmt, clippy default + heavy-tests, test).

## Size
Under ~80k tokens. Stop with a handoff note rather than compacting.

## Done (2026-09-13, builder-opus)

**Built.** `event_loop::prefetch`: after each frame, while `Wakes::waiting()` is false, runs `update` for `PageDown` and `PageUp` on a copy (the cursor moves, so thread and fold change; only that state gives the key frame's cache keys), then `Graphics::prefetch` rasterises its uncached strips one at a time via `StripCache::contains`/`get`, looking at the queue before every strip and before each page's what-if. Uploads nothing. Skipped off pixels and while the search prompt is open. `ChannelWakes::waiting` uses `try_recv` into a one-slot buffer (`next_wake` and `unhandled_input` hand it out). `frames.rs::Paced` stamps the frame at the loop's first look, says waiting once the next key is due, records prefetch time.

**Tests (red on a stub, green after):** `no_strip_is_prefetched_while_input_is_queued`, `a_key_arriving_mid_prefetch_waits_for_at_most_one_strip` (sweeps the look at which the key arrives: each extra look lets ≤1 more strip through; uninterrupted does ≥2), `page_keys_find_their_strips_already_rasterised` (PgDn and PgUp frames: 0 rasterised, hits up), `a_wake_seen_while_checking_is_handed_out_next_and_kept_as_unhandled_input`.

**Key 226's 6.5 ms queue** was behind key 225's own frame (3.06 ms), not prefetch (none existed). Prefetch can now be what a key queues behind; bounded to one strip (~1.1 ms wide) or one page's what-if (`update` + `pixel_rows`, up to ~2.5 ms at 50k refs, estimate from per-frame totals), never both.

**Release frames, uncounted** (`target/30h/release-frames.txt`; pre-final-look run `release-frames-9919481.txt`): 4/4 pass. Keys p50/p99/max ms: Pixel 100k 4.0/6.4/8.0 (was 6.9/13.2/16.8); Pixel 50k 4.3/4.8/6.0; Unicode 100k 4.2/5.2/5.3; Unicode 50k 5.7/10.2/11.1.

**Idle cost (what the user pays):** fixes the cause (page keys rasterised on the key path), at a CPU price. Per pixel frame with idle time, p50/p99/max: 100k 0.8/4.6/6.7 ms; 50k refs 3.3/5.1/5.6 ms. After a scroll stop: one prefetch, 2 strips (~2.2 ms raster) plus the what-ifs, ~3–6 ms once. Most of the 50k cost is not rasterising (suspected `AppState` clone with `RefBadges` by value, not profiled) → KNOWN_GAPS row. Text tiers: ~0 (skipped).

Not done (out of scope): counted runs, proofs, docs → 30f seam 4. No compaction.
