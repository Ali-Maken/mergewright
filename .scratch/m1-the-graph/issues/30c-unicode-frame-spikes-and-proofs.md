# m1-the-graph/30c — Unicode chunk-edge frame spikes; frame asserts pass and are proven; benches run

Status: partial (part a done; part b seam 1 done: key 37 answered, release-only asserts, docs; proofs + release run wait on a user decision, see "Part b, seam 1")
Build with: **Opus** (high) — event loop timing, split from 30
Blocked by: 30b (part a done; part b unblocked by user decision)
Leaves runnable: yes — heavy `frames::` tests pass and each is proven to fail when tightened.

Why: 30 measured Unicode input-to-frame spikes of 32–41 ms (debug) at layout-chunk edges; release 2–12 of 420 keys over 16 ms, max 17–23 ms.

## Read first
- Ticket 30 "Handoff" items 2–4 and NOTES (the debug vs release question); results table
- DESIGN.md §3.6 (background advance paragraph); ADR-0008
- `crates/graft-graph/MAP.md`, `crates/graft/MAP.md`; scouts for source

## Scope
In:
1. Decide with the benches (`StreamingLayout::advance/one chunk`): background advance ahead of the viewport vs smaller `chunk_rows`. Numbers first; fix; re-measure.
2. **Debug vs release for frame asserts:** the conductor's decision goes in here before this ticket starts (see "Decision" below). Implement it (TESTING.md "The heavy tier" command updated to match).
3. Threshold-tightening proof for each `frames::` assertion, reverted.
4. Run the benches once (`cargo bench -p graft-graph -p graft-ui -- --warm-up-time 1 --measurement-time 3`), numbers into 30's table.

Out: pixel byte cost (30b), 50k refs real read (30d), 1M claim (extrapolated, in KNOWN_GAPS).

## Decision (debug vs release) — user, 2026-09-13
- **Frame budgets fail only in `--release`.** The debug heavy tier still runs the `frames::` tests and records the numbers, without failing.
- **The release perf run is part of every milestone review**, not only local: the heavy tier gains a release run for `frames::` (and the 50k-refs first-paint test from 30d). Update TESTING.md "The heavy tier" and `.claude/agents/reviewer.md` step 2 with the exact command. The conductor has updated CLAUDE.md.
- Record as **ADR-0021** (PROMPT §7.1's heavy-tier command gains a release frame-budget run; debug records only).
- 30d's debug first paint with 50k refs (494 ms) follows the same rule: recorded, not failing. No shared-ref-read work needed.
- **Memory gate stays `getrusage` peak RSS** (user). Add to KNOWN_GAPS: git/git peaks at 219–226 MiB RSS against the 300 MB budget — less headroom than it sounds (100k synthetic fixtures don't see what git/git does; ref 27d's guard gap).

## Key 37 (user question)
Before any fix: say whether the `j` that misses 16 ms in release (key 37, 17–18 ms, every run) is **the first keypress after idle** or **arbitrary**. Vary the idle time before it, its position in the sequence, and the key sequence; report the numbers. Warm-up (e.g. first key after an idle gap, a lazy init, a cache fill) is documentable — document it in TESTING.md and the ADR with its cause, and exclude it from the assertion only if the cause is named. An arbitrary stutter is a bug: find the cause and fix it.

## Acceptance
- [ ] Spike cause named with numbers; fix measured before/after.
- [ ] All four `frames::` tests pass in the chosen build mode; each proven by tightening.
- [ ] Bench numbers recorded.
- [ ] Gates pass.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`; heavy and benches once, alone.

## Size
Under ~80k tokens. Seam: (a) spike fix + decision, (b) proofs + benches.

## Part a (2026-09-13, builder-opus): spike fix and benches

**Cause, with numbers.** `history::answer` advanced a whole chunk (`chunk_rows` = 500) whenever the request reached past the frontier, while a `PageDown` needs a screenful. Timed in place (temporary instrumentation, removed): one 500-row chunk read from the 100k fixture costs 26–33 ms debug. The bench (`StreamingLayout::advance/one chunk`, in-memory source) is 0.98 ms release, so the cost is reading commits from the repository, not layout. Spikes fell on every 13th–14th `PageDown` (≈ 500 rows).

**Decision.** Neither named option. Smaller `chunk_rows` would also change cache, eviction and checkpoint granularity (memory tests). Background advance is the larger, concurrent change and isn't needed to remove the spike. The fix: `answer` advances `chunk.min(reach - rows_emitted)` per step, so progress reports and newer-request checks stay per chunk. `chunk_rows` is unchanged. Pinned by `history::tests::a_request_lays_out_only_the_rows_it_names` (red without the fix: 8 rows laid out, wanted 3).

**Side effect, handled in a test.** The rows on hand used to run up to the next chunk boundary past the request. `page_down_past_the_rows_on_hand_while_promotion_swaps_keeps_its_place` relied on that slack. It now applies a `Progress` update for rows laid out past the window before its keys, so it still tests the same scenario. Real effect: two keys pressed before an answer arrives stop the cursor at the last laid-out row (the old code did this too, at chunk edges only). In KNOWN_GAPS (row "Layout advances only as far as the screen asks").

**Frames, before → after** (`frames::`, each run alone, p50 / p99 / max, keys over 16 ms):

| Test | Build | Before | After |
|---|---|---|---|
| Unicode, 100k | debug | 8.2 / 35.4 / 43.4 ms, 27 over | 10.0 / 11.8 / 17.5 ms, 1 over (key 37) |
| Unicode, 50k refs | debug | 10.0 / 35.6 / 36.5 ms, 34 over | 14.3 / 16.3 / 28.3 ms, 12 over (keys 37–40 at 16.6–28 ms, 8 at ≤ 16.4 ms) |
| Pixel, 100k | debug | 37.7 / 95.2 / 113.2 ms | 25.3 / 72.0 / 72.7 ms |
| Pixel, 50k refs | debug | 46.0 / 101.4 / 133.8 ms | 34.4 / 93.1 / 95.2 ms |
| Unicode, 100k | release | 6.0 / 15.4 / 17.2 ms, 2 over (30 part 1) | 6.9 / 10.9 / 12.2 ms, **0 over** |
| Unicode, 50k refs | release | 7.7 / 18.1 / 23.3 ms, 12 over (30 part 1) | 9.6 / 12.0 / 17.8 ms, 1 over (key 37) |
| Pixel, 100k | release | 7.7 / 16.4 / 21.5 ms, 6 over (30b) | 7.9 / 15.1 / 18.0 ms, 2 over |
| Pixel, 50k refs | release | 9.2 / 15.8 / 19.9 ms, 5 over (30b) | 10.0 / 14.6 / 16.3 ms, 1 over |

No chunk-edge spike remains. Debug p50 rose (more, smaller answers). The misses left aren't at chunk edges: key 37 is a `j` about 0.6 s in (both tiers, both builds). Its cause isn't found; it may be a background job landing, such as refs or badges.

**Benches (item 4), release, once:** `StreamingLayout::advance/one chunk` 977.9 µs · `rasterise/one strip` 657.9 µs · `StripCache::get/hit` 1.66 µs · `StripCache::get/miss` 661.3 µs · `GraphColumn::render/one screen` 14.86 µs. The ticket's command fails (the lib test harness rejects criterion flags). The working command: `cargo bench -p graft-graph -p graft-ui --bench hot_paths --bench row_render -- --warm-up-time 1 --measurement-time 3`.

## Handoff (part b)

Done: scope 1 and 4. Not done: scope 2 (debug vs release for frame asserts: still with the user) and 3 (tightening proofs, which depend on 2). The four `frames::` tests still fail: in debug the pixel tier fails by a wide margin and the 50k-refs Unicode test fails 12 keys; in release they miss by 1–2 keys at 16.3–18 ms.

Next, in order:
1. Record the user's decision under "Decision" above.
2. Find key 37's spike (the first miss in every run): print the per-trigger split for keys 30–45, or time what lands on the event loop then (a refs/badges `Produced`?).
3. Implement the build-mode decision and update TESTING.md "The heavy tier".
4. Tightening proof per `frames::` assertion, reverted.
5. Fix the bench command wherever it's written (ticket 30, TESTING.md if present).

Gotchas: run frames tests alone and send output to a file (`cargo test [--release] -p graft --features heavy-tests --test it frames:: -- --test-threads=1 --nocapture`, about 30 s). Failure messages list the late keys.

## Part b, seam 1 (2026-09-13, builder-opus): key 37 answered; release-only asserts; docs

**Key 37: neither first-after-idle nor arbitrary. It is the first key that sends a row request**, wherever it falls. Release, Unicode, 50k refs, temporary tracing (removed):

| Variant | First requesting key | Its latency |
|---|---|---|
| default script, 5 runs | 37 | 13.0, 18.5, 20.7, 21.2, 21.4 ms |
| 0 / 500 / 2000 ms idle before the scroll | 37 | 13.0 / 13.3 / 17.5, 16.4 ms |
| 10 `k` first | 47 | 17.6, 25.4 ms |
| 20 / 60 no-op keys first | 57 / 97 | 15.8 / 18.9 ms |
| `PageDown` first, straight after the start-up answer | 0 | 5.9 ms |
| `PageDown` first after 1 s idle | 0 | 24.8 ms |

- Only Progress and Window wakes land during the scroll: no refs/badges job arrives at key 37 (the part a guess is ruled out).
- Its cost: the key's frame, a layout Progress frame and the Window frame, back to back. `j` frames before it 6–9 ms, after it 3–5 ms.
- **Cause: CPU power state.** Same test with one unrelated thread spinning on another core (3 runs): key 37 6.5–6.8 ms, p99 6.6–7.0 ms (without: 11.7–15.0), max 6.9–7.1 ms, no misses; the `PageUp` misses at 16.3–17.4 ms (keys 371–419, in ~half of runs) disappear too. Idle + `PageDown` first: 24.8 → 16.0 ms with the spinner.
- Second finding: the first key after ≥ 0.5 s idle misses in release (17.0–24.8 ms), same cause.
- Also seen: since part a, 353 of 420 keys send a request (every `j` past row 37 round-trips). Not a miss on a hot CPU, but it's why a cold CPU has no headroom.

**Done in this seam:** `frames::check` and `heavy::check` fail time budgets only under `cfg!(not(debug_assertions))`; debug prints `debug build, budget recorded not asserted` with the late keys / first paint (RSS still asserted in both). TESTING.md "The heavy tier": release perf run command + the power-state note. `.claude/agents/reviewer.md` step 2: exact command. ADR-0021 written (decision open on warm-up handling). KNOWN_GAPS: git/git RSS headroom row. Gates: fmt, clippy (with heavy-tests), `cargo test`.

**Not done (next seam):** tightening proofs (item 3), the release run once alone, acceptance boxes.

**Blocked on a user decision:** as it stands the release assert flakes (key 37 misses in 4 of 7 default runs; PageUp keys in ~half). Options:
1. Exclude the first requesting key (cause named) — PageUp keys still flake at 16.3–17.4 ms. Not enough alone.
2. Hold one core busy while measuring (a spinner thread in the test) — measures the code, not the governor; CLAUDE.md calls this a workaround needing the user's OK.
3. Cut per-key cost so a cold CPU fits: lay out a screen of read-ahead past each request (no round trip per `j`), and/or don't draw a frame for a Progress update when its Window is already queued. Code change, re-measure.
4. Combination: 3, then 1 for the first key after idle if it still misses.

Next exact step once decided: implement the choice, then per `frames::` test tighten `INPUT_TO_FRAME` (e.g. 4 ms) and `heavy::FIRST_PAINT_LIMIT` (e.g. 10 ms) in release, see each fail, revert; then run the release command in TESTING.md once, alone, output to a file; fill the acceptance boxes.
