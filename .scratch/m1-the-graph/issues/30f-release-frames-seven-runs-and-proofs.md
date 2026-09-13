# m1-the-graph/30f — Release frame check 7/7, idle-wake report, proofs

Status: blocked — seam 4 (7 counted runs from run 1, proofs, docs) waits on 30h idle prefetch
Build with: **Opus** (high) — frame timing under the job/redraw loop (split from 30c part b seam 2)
Blocked by: 30h
Leaves runnable: yes — release frame check passes 7 runs of 7; idle-wake reported separately.

Why: the remainder of 30c part b after seam 2 (commit 5c4594e). Scope is the unchecked boxes of 30c's "Decision (key 37 / release assert flake) — user, 2026-09-13" section, minus the key-path fix (done in seam 2) and the eviction bug (30e).

State: `crates/graft/tests/it/frames.rs` reworked in 5c4594e (own-frame latency, replay check, idle-wake printed): **compiles, never run**.

## Read first
- Ticket 30c: "Decision (key 37 …)" section and part b handoff notes; ADR-0021
- TESTING.md "The heavy tier" (release perf run command)
- `crates/graft/MAP.md`; scouts for source

## Scope
In:
- Run and fix the reworked frames.rs (debug records, release fails — ADR-0021).
- Check that no key waits on a background answer (assert on the key path, already tested in seam 2 — confirm it holds in the frames run).
- Idle-wake (first key after ≥0.5 s idle): measured and printed separately in the release run, not failing.
- Release frame check: **7 runs, all 7 pass**; record every run's numbers in the ticket. A failure is a finding: find its cause, don't retry until green.
- The proofs from 30c's original scope, and one full release perf run.
- TESTING.md/KNOWN_GAPS updates the result implies. Tick 30c's boxes; mark 30c done if all are done.
- Report: cause or symptom per fix, and what it costs the user.

Out: eviction (30e). The user's by-eye check (no lag after a pause) happens at the gallery re-walk.

## Acceptance
- [ ] frames.rs green in debug (records) and release.
- [ ] 7/7 release runs recorded.
- [ ] Idle-wake numbers printed separately.
- [ ] Proofs done; release perf run numbers in the ticket.
- [ ] Gates pass.

## Size
Under ~80k tokens. Stop with a handoff note rather than compacting.

## Seam 1 (2026-09-13, builder-opus): partial — stopped at ~60k tokens, no compaction

**Release runs, recorded (frames:: only, alone, output in `target/30f/`).** None counts toward 7/7: each failed, and each after run 1 was a diagnostic run, not a retry.

| Run | File | Change before it | Result |
|---|---|---|---|
| 1 | `release-1.txt` | none (HEAD f2170c2) | 3/4 pass. **Pixel 100k fails**: key 158 17.3 ms, key 355 16.2 ms (p50 6.7 / p99 14.8 / max 17.3). Pixel 50k 6.4/11.7/14.7; Unicode 100k 4.2/5.2/5.3; Unicode 50k 5.8/9.2/10.5. `waited` empty in all four (no key drew without its rows). |
| 2 | `release-2-diag.txt` | per-key breakdown added to frames.rs | 2/4. Pixel 50k: keys 63 (18.2), 109 (16.6); Pixel 100k: key 400 (19.2). |
| 3 | `release-3-diag-timers.txt` | temporary frame timers (reverted) | Pixel 100k only: 5 late (keys 158, 201, 233, 389, 400; 16.8–18.4 ms). |
| 4 | `release-4-diag-encode.txt` | Kitty encode fix | Pixel 100k only: 8 late (p50 6.9 / p99 16.4 / max 18.6). |

Idle-wake (printed, not asserted), run 1, µs: Pixel 50k 10648 9870 11416 5752 7645; Unicode 50k 9366 10709 9444 11109 12300; Pixel 100k 7027 7107 5800 7938 3736; Unicode 100k 7016 6729 6586 8676 2797.

**Finding: every late key is a pixel-tier frame that sends strips; the key's own frame is the cost, not queueing.** Breakdown (frames.rs now prints it per late key): queued 0.9–2.2 ms behind the wake before; own frame 14–18 ms with 2–3 transmits (≈430 KB base64 each). Text tiers never miss (max 10.8 ms). Temporary timers inside `event_loop::frame` / `Graphics::frame` (run 3 → run 4), for a 3-strip `PageUp`/`PageDown` frame:
- rasterise (`StripCache::get` misses): 1.7–3.5 ms **per strip**, all strips on the key path → 5–10 ms. Bench says 0.66 ms/strip (in-memory, simpler strip); CPU power state spreads it run to run (ADR-0021).
- test `Sink` write: 3.1–5.5 ms (p50 3.6). **Harness cost counted as latency**: it parses 1.3 MB byte by byte under a mutex. A real pty write is a copy (plus whatever the terminal does, which is real).
- Kitty encode (`straight_alpha` + `base64`): p50 2.23 ms, max 4.1 before the fix; p50 1.20, max 2.1 after.
- ratatui draw ≈1 ms; `pixel_rows` ≈0.2 ms.

**Fix in this seam (cause, graft-term):** `kitty::straight_alpha` skips the per-channel division for alpha 0 and 255 (nearly every graph pixel) and writes into a zeroed buffer; `kitty::base64` writes into an exact-size slice instead of per-byte `push`. Byte-for-byte guards added first: `straight_alpha_matches_the_per_channel_division_for_every_value_and_alpha` (all 65 536 value×alpha), `base64_matches_the_standard_alphabet_for_every_length_and_byte`. User cost: none (same bytes, ~1 ms less per transmitted strip, iTerm2 PNG path shares `straight_alpha`). **Not enough alone**: run 4 still misses.

Also in: frames.rs `Paced::breakdown` (per key: queued behind which wake, own-frame time, bytes/transmits/placements), printed in the failure message. Measurement only.

**Next exact steps:**
1. Decide the Sink: its per-byte parse inflates pixel latency by ~3.6 ms. Measure it alone (time `Sink::write` inside the test) and, if confirmed, make it skip payload bytes (search for the next `ESC` instead of a per-byte state step). That is a measurement fix, not a Graft fix — report it as such, and say what a real terminal write costs instead (unmeasured).
2. Rasterise on the key path: 3 strip misses per page key. Scout `graft-graph` `raster::rasterise` for the hot loop (why 1.7–3.5 ms vs the 0.66 ms bench). If no cheap cause, the structural options are a **decision for the user/planner** (rasterise the strips either side ahead of the key, off the key path — the strip cache already keeps a strip either side per `STRIPS_KEPT`; or accept pixel page keys at >16 ms). Don't exclude pixel page keys.
3. Only then the 7 counted release runs (frames:: alone), debug frames run (records), proofs (INPUT_TO_FRAME 4 ms / FIRST_PAINT_LIMIT 10 ms, revert), one full release perf run (TESTING.md), ADR-0021 "open" → decided (a), KNOWN_GAPS/TESTING updates, tick 30c boxes.

Gotchas: temporary timers go in `event_loop/mod.rs::frame` (draw / graphics / write) and `event_loop/graphics.rs::frame` (pixel_rows / each `cache.get` / `images.frame`); revert with `git checkout` before committing. Pixel 100k test alone takes ~10 s release.

## Seam 2 (2026-09-13, builder-opus): partial — stopped for a user decision, no compaction

**1. Sink: measurement fix (commit 72524df).** The frames test's in-memory terminal parsed every byte (Kitty command state machine, under a mutex) inside `write`, so ~3.6 ms of test bookkeeping per pixel page key landed inside the key's timed frame. Now `Sink::write` appends to a chunked log (a copy, as a pty write copies into the kernel buffer; chunks so growth never reallocates megabytes mid-frame); frames record only a byte offset; `Sink::count` parses the log once, after `pump` returns and every latency is stamped. Proof: `frames::the_sink_counts_after_the_run_what_parsing_each_write_counted_during_it` (commands split at every awkward boundary, a write bigger than a chunk; counts at every mark equal the old parse-per-write counts).
Why this is not hiding a user cost: the parse was never Graft's work nor a model of the terminal's — a real terminal parses in its own process. What the user *does* pay for writing ~1.3 MB to a real pty (back-pressure while the terminal drains it) is still unmeasured here, as the module doc already said; it can only add to the numbers below.

**Release runs (seam 2).** None counts toward 7/7.

| Run | File | Change before it | Result |
|---|---|---|---|
| 5 | `release-5-sink.txt` | sink fix | 3/4 pass. **Pixel 100k fails 1 key**: key 389 16.2 ms (queued 1.5, own frame 14.7, 3 transmits). Pixel 50k p50 7.0 / p99 10.9 / max 14.7; Unicode 50k 6.8/10.2/10.7. |
| 6 | `release-6-diag-timers.txt` | temporary frame timers (reverted) | Pixel 100k alone: passed, max 13.1 ms. Diagnostic, not counted. |
| 7 | `release-7-diag-every-transmit.txt` | timers printing every frame that sends images (reverted) | Pixel 100k alone: **failed**, max 16.6 ms. |

**2. What the user pays per pixel page key (run 7, every image-sending frame, release, ms):**

| Strips rasterised | Frames | Own draw total p50 / max | Rasterise | Kitty encode | Cell draw | Write (copy) |
|---|---|---|---|---|---|---|
| 0 (re-placed only) | 10 | 2.7 / 2.9 | 0 | 1.0 | 1.2 | 0.1 |
| 1 | 186 | 5.1 / 6.8 | 2.9 / 3.9 | 0.65 | 1.1 | 0.1 |
| 2 | 103 | 8.4 / 12.5 | 5.4 / 9.0 | 1.2 | 1.1 | 0.2 |
| 3 | 13 | 10.8 / 14.7 | 7.4 / 10.3 | 2.0 | 1.1 | 0.3 |

Plus 1–2 ms queued behind the previous frame. Rasterise is ~70 % of a late key. Per strip 2.3–3 ms p50, 4.8 max; the 0.66 ms bench is the same code on a ~2-lane strip, `STRIP_ROWS`'s own note measured 1.36 ms at 12 lanes — the gap is work size (lanes), not a skipped path. Scout (code-visible, unprofiled): `Pen::row` (`graft-graph/src/raster.rs:246-283`) strokes each lane column separately on every row (O(rows × lanes) `stroke_path` calls, a straight lane through 32 rows is 32 strokes); a fresh `PathBuilder`/`Path` per segment (`raster.rs:334-376`); a fresh zeroed `Pixmap` per miss (`raster.rs:178`). `STRIPS_KEPT` (16) is LRU capacity only; nothing rasterises ahead of need.

**Not done, as instructed:** rasterising was not moved off the key path.

**Options for the user (conductor to put):**

| | Option | What the user sees | Cost / risk |
|---|---|---|---|
| A | **Make rasterise cheaper, same frame.** One stroke per lane run across consecutive rows instead of per row; reuse path builders and the pixmap buffer. | Nothing changes: rows and graph still arrive together. | Gain unmeasured until built (guess: the per-row strokes are most of it; wide repos, e.g. git/git's ~28 lanes, cost more). Needs byte-for-byte pixel guards. May not be enough alone. graft-graph work, one ticket. |
| B | **Rasterise the strip above and below the viewport while idle, on the loop thread** (after a frame, only when no input is waiting); keys still rasterise on the key path when the prefetch hasn't reached them. | Same frame as now: never a blank graph. A held PgDn faster than idle time still falls back to today's cost. | Idle CPU and power after each scroll stop (~3 strips × 2.5 ms); a key arriving mid-prefetch waits for at most one strip (~3 ms) — must check input between strips. |
| C | Rasterise on a worker, draw the graph when it's ready. | **Graph arrives a frame after the rows** — the blank-graph frame flagged on PgDn in 30b and ticket 37. | Meets 16 ms by showing less. Not recommended. |
| D | Accept pixel page keys over 16 ms (up to ~18 ms seen); record in an ADR. | Occasional dropped 60 Hz frame on held PgUp/PgDn at the pixel tier only. | Spec change (PROMPT.md §6.3) — user's call. |

Recommendation: A first (no visible change), measure with this ticket's frames run; then B if pixel still misses. Both keep "graph and rows in the same frame".

**Next exact step after the decision:** build the chosen fix (A: `graft-graph` raster, guards first); then step 3 of the order: 7 counted release runs (frames:: alone), debug frames run, proofs, full release perf run, ADR-0021, TESTING.md/KNOWN_GAPS, tick 30c.

## Seam 3 (2026-09-13, builder-opus)

**Counted release runs, first count (HEAD 0086425, frames:: alone, `target/30f/counted-N.txt`).** Stopped at the first miss, per instruction; no reruns.

| Run | Result | Pixel 50k p50/p99/max | Unicode 50k | Pixel 100k | Unicode 100k |
|---|---|---|---|---|---|
| 1 | 4/4 pass | 7.1 / 12.2 / 13.6 | 6.7 / 12.4 / 13.8 | 6.8 / 12.9 / 15.3 | 5.3 / 8.8 / 9.2 |
| 2 | 4/4 pass | 7.7 / 12.0 / 14.4 | 7.2 / 13.4 / 14.7 | 7.0 / 13.2 / 15.7 | 5.9 / 8.8 / 9.3 |
| 3 | **3/4, Pixel 100k fails** | 7.6 / 12.7 / 14.8 | 7.0 / 12.7 / 14.4 | 6.9 / 13.2 / **16.8** | 5.4 / 8.6 / 8.9 |

Run 3 miss: key 226, 16.78 ms = queued 6.51 ms behind key 225 (whose frame took 3.06 ms) + own frame 10.27 ms, 864 KB, 2 transmits. Finding: pixel page keys still rasterise on the key path; the margin after 30g is ~0.2–1 ms. → option B (idle prefetch), count restarts from 1.

Idle-wake µs (printed, not asserted), runs 1–3: Pixel 50k 15795 14256 16221 14221 15672 / 15842 5986 18919 6444 10976 / 15545 10394 11475 11337 17579; Unicode 50k 15816 12186 15577 15162 15564 / 13349 15100 14834 7347 10789 / 7298 5302 18355 16670 16312; Pixel 100k 10350 6224 3149 10283 11727 / 10294 10134 1787 12122 10896 / 3752 4503 4370 8951 12008; Unicode 100k 6453 7291 2934 3025 6764 / 11673 11704 11227 11725 11554 / 7675 10867 5153 11813 2336.

**Stopped here (partial, ~75k tokens, no compaction):** building B would exceed the ticket's context. No code changed in this seam.

**Handoff: build option B (idle prefetch), then restart the count.** Found while scouting, no code written:
- **Rows are already there.** `History` asks for `top - 3·height .. bottom + 3·height`, widened to whole strips plus the row above (`history/scroll.rs` ~435–453, `drawn`). So the strips above and below the viewport normally have their rows on hand; no History change needed.
- **Strip choice.** `view::strips::pixel_rows(state, max_lanes, STRIP_ROWS)` builds strips only for `history.top..top + height` (`strips.rs:148–163`, `strip_ranges(range, &on_hand, strip_rows)`). Prefetch needs the same for the one strip above and one below: add a row-range variant (or a `pixel_rows_around`) that reuses the fold, so a prefetched strip's cache key is exactly what the key's frame asks for (same `layout_hash`, theme, cell size), or it is a miss anyway. Guard with a test: the prefetched strips are hits on the next `PageDown`/`PageUp` frame (`Graphics::strip_stats`).
- **Where.** `event_loop/mod.rs::frames` (loop at ~550): after `draw`, before blocking in `wakes.next_wake()`. Add a `Graphics::prefetch(&mut self, session, state, waiting: impl FnMut() -> bool)` that rasterises (via `StripCache::get`) one missing strip at a time and returns as soon as `waiting()` is true. It only fills the cache; it writes nothing (upload happens when a frame places it).
- **Input check.** `Wakes` only has blocking `next_wake`. Add a non-blocking `fn waiting(&mut self) -> bool` (or `try_wake` that buffers one wake). `ChannelWakes`: `try_recv` into a one-slot buffer that `next_wake` returns first (never drops a wake — `unhandled_input` must include the buffered one). `ScriptedWakes` (mod.rs tests ~732) and `frames.rs::Paced` (`next_wake` ~841, `recv_timeout` pacing) need it too: Paced should say `true` once the next key is due, so its latency counts any strip it waited behind.
- **Test (scope):** a key arriving mid-prefetch waits at most one strip: scripted wakes whose `waiting()` turns true after the first strip → exactly one strip rasterised before the key is handled.
- **Keep:** `the_ui_thread_never_waits_on_a_job` (forbids `recv_timeout`/`sleep` in mod.rs: use `try_recv`).
- **Cost to report:** idle CPU after each scroll stop (strips rasterised × per-strip cost, ~1.1 ms wide after 30g), and the at-most-one-strip wait.
- Then: 7 counted runs from run 1 (`target/30f/counted-b-N.txt`), stop at the first miss; if B misses → option D to the user. After 7/7: debug frames run, proofs (INPUT_TO_FRAME 4 ms, FIRST_PAINT_LIMIT 10 ms, release, revert), full release perf run (TESTING.md), ADR-0021 open → decided, TESTING/KNOWN_GAPS (row "Input to frame misses 16 ms", line ~295, is stale), tick 30c/30f boxes.
