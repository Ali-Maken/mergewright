# m1-the-graph/30f — Release frame check 7/7, idle-wake report, proofs

Status: partial (seam 1: release runs 1-4 all fail at the pixel tier, cause located; Kitty encode fix in; see "Seam 1")
Build with: **Opus** (high) — frame timing under the job/redraw loop (split from 30c part b seam 2)
Blocked by: 30e
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
