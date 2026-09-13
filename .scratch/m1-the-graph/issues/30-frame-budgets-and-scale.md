# m1-the-graph/30 — Frame budgets, benches, 50k refs, and the 1M claim

Status: partial (handoff below)
Build with: **Opus** (high) — timing the event loop and the pixel path
Blocked by: 11, 25, 27, 27d
Leaves runnable: yes — benches run at review; input-to-frame asserted in the heavy tier.

Why: the rest of the retired performance ticket (16, text in 35bec3e). Split from 27 so each fits one context.

## Read first
- PROMPT.md §1.2 (the 60 fps bullet), §6.3 "Performance budgets" and the 50k-refs bullet, §7.1 "Benchmarks" bullet
- DESIGN.md §3.8 (the 16 ms frame budget paragraphs only)
- KNOWN_GAPS.md rows "50k+ refs in the sidebar and ref badges (review N4)", "Resize frame busts the frame budget"
- 25's measured resize numbers; 27's results table
- Code: `MAP.md` of `graft-graph`, `graft-ui`, `graft`

## Scope
In:
- `criterion` benches: `StreamingLayout::advance` per chunk, Unicode row render, strip rasterise, strip cache hit/miss. Review-only runs (`--warm-up-time 1 --measurement-time 3`).
- Heavy tier on the 100k fixture: input-to-frame < 16 ms at the Unicode and pixel tiers; sustained scroll at 60 fps.
- 50k refs: generate them in the heavy tier, measure badge lookup and the row model, meet the budget or record what breaks.
- The 1M+ claim: fixture is 100k. Measure a real 1M repo once by hand, or state in KNOWN_GAPS.md that 1M is extrapolated. **Never report a 1M number not observed.**

Out:
- Memory and first paint (27). Search timing (26). CI wiring.

## Signatures
Expose: none public. Benches in `benches/` of the crate that owns each hot path.
Call: `StreamingLayout::advance(budget, stop)`, `rows(Range<usize>)`; 22's rasteriser entry point; 23's cache lookup; 11's badge lookup; `graft_fixtures::synthetic::borrow(Scale::HEAVY)`.

## Acceptance
- [ ] Results table in this ticket: budget · measured · tier · fixture · machine.
- [ ] Each heavy assertion proven to fail by tightening its threshold, then reverted.
- [ ] 50k-refs result recorded; N4 row closed or re-owned with a reason.
- [ ] 1M claim measured or explicitly marked extrapolated.
- [ ] Fast tier still under 3 minutes; benches not in it.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`; heavy tier and benches once, alone, at the end

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.

## Results (2026-09-13, builder-opus, part 1)

Machine: Apple M5, 24 GB, macOS (Darwin 25.6). Each run alone.
What is timed: `crates/graft/tests/it/frames.rs` — the real `event_loop::pump`, real job pool, in-memory terminal (no pty parse/paint, no write back-pressure), 120×40, fixture `Scale::HEAVY.with_commit_graph()`. Script: `j`×120, `PageDown`×240, `PageUp`×60, keys on a 60 Hz clock. Latency = the key's *due* tick → the frame drawn after the window answering the newest `ShowRows` the key sent (or the key's own frame when it sent none), so falling behind accumulates.

| Budget | Measured (p50 / p99 / max) | Tier | Fixture | Build | Result |
|---|---|---|---|---|---|
| input→frame < 16 ms | 8.2 / 35.4 / 41.3 ms; 31/420 keys over | Unicode | 100k, cg | debug | **missed** |
| input→frame < 16 ms | 73 / 549 / 606 ms; 419/420 over, lateness grows ~5 ms/key from key 0 | Pixel (Kitty, 17×37 cell) | 100k, cg | debug | **missed** |
| input→frame < 16 ms, 50k refs | 10.0 / 34.9 / 37.8 ms; 38/420 over | Unicode | 100k, cg + 50k synthetic refs | debug | **missed** |
| input→frame < 16 ms, 50k refs | 109 / 1024 / 1086 ms; 420/420 over | Pixel | same | debug | **missed** |
| `refs::badges` on 50k refs (off-thread job) | 1.35 s | — | synthetic 25k branches + 25k upstreams | debug | no budget; recorded |
| `RefBadges::on` lookup | 179 ns | — | 50k refs | debug | fine |

Same four tests, **release** build (`cargo test --release -p graft --features heavy-tests --test it frames:: -- --test-threads=1 --nocapture`), one run:

| Budget | Measured (p50 / p99 / max) | Tier | Fixture | Build | Result |
|---|---|---|---|---|---|
| input→frame < 16 ms | 6.0 / 15.4 / 17.2 ms; 2/420 over | Unicode | 100k, cg | release | **missed (narrowly)** |
| input→frame < 16 ms | 8.4 / 14.1 / 20.7 ms; 2/420 over | Pixel | 100k, cg | release | **missed (narrowly)** |
| input→frame < 16 ms, 50k refs | 7.7 / 18.1 / 23.3 ms; 12/420 over | Unicode | 100k, cg + 50k refs | release | **missed** |
| input→frame < 16 ms, 50k refs | 8.7 / 15.9 / 20.6 ms; 5/420 over | Pixel | same | release | **missed (narrowly)** |
| `refs::badges` on 50k refs | 458–461 ms | — | synthetic | release | no budget; recorded |
| `RefBadges::on` lookup | 22–24 ns | — | 50k refs | release | fine |

Release pixel runs wrote 388 MB over 420 keys (~925 KB, ~230 Kitty commands per frame). The heavy tier is a debug build, so as written these tests fail there by a wide margin that is mostly debug overhead; whether the heavy tier should assert frame time in debug at all is a question for the conductor (see NOTES).

Reading of the misses (debug runs; unconfirmed, from the per-key lists, not profiled):
- Unicode: every spike is a `PageDown` roughly every 13–14 pages ≈ one 500-row layout chunk (`StreamConfig::chunk_rows`). The key waits for the history job to lay out the next chunk on demand. This is the "background advance between frames" decision (27, re-owned here): laying out ahead of the viewport would remove it.
- Pixel: every frame, including `j` frames that need no new strip, costs more than a tick; 141 MB written over 420 keys (~336 KB and ~84 Kitty commands per frame) suggests strips are re-transmitted on frames that should only re-place. Needs a look at what `Graphics::frame` sends per scroll frame before any tuning.

## Handoff (what remains)

Done (committed, gates green): criterion benches `crates/graft-graph/benches/hot_paths.rs` (advance per chunk, rasterise one strip, strip-cache hit/miss) and `crates/graft-ui/benches/row_render.rs` (one screen of `GraphColumn`); heavy tests `crates/graft/tests/it/frames.rs` (4 tests, Unicode/pixel × fixture/50k refs), sharing `heavy::MEASURING`; `budget_run::{open, pump_until}` made public for it. **The four heavy tests fail**: they are the measurement, and the budget is not met.

Next, in order (each is its own ticket-sized piece):
1. **Pixel frame cost**: find why a one-row scroll frame writes ~336 KB / ~84 Kitty commands (expected: place existing ids, ≤1 new strip). Start by counting transmits vs placements per frame in `frames.rs` (the `Sink` can split on `a=t` vs `a=p`). Fix the cause, re-run.
2. **Unicode chunk-boundary spikes**: background advance ahead of the viewport (DESIGN.md §3.6) or a smaller `chunk_rows`; decide with the benches (`StreamingLayout::advance/one chunk`).
3. **Threshold-tightening proof** for each heavy assertion (acceptance box) once they pass.
4. **Benches run once** (`cargo bench -p graft-graph -p graft-ui -- --warm-up-time 1 --measurement-time 3`), numbers into this table.
5. **50k refs, real read**: `refs::read` does one `RepoHandle::upstream` per branch; not measured on a real repo with 50k refs (only the pure `badges`, 1.35 s debug). Generate one in the heavy tier (packed-refs + `[branch]` config sections) and time it.
6. **1M claim**: not measured; no 1M repo on this machine. Mark extrapolated in KNOWN_GAPS (done in part 1); compact `EmittedOrder`, checkpoint thinning stay open there.

Gotchas: run the frames tests alone (`cargo test -p graft --features heavy-tests --test it frames:: -- --test-threads=1 --nocapture`, ~30 s debug); output is long, send it to a file. zsh does not word-split `$VAR` commands.

## NOTES

- **Question for conductor/user:** the heavy tier runs debug (PROMPT.md §7.1 command has no `--release`). Debug frame times are 5-10x release. Options: (a) assert frame budgets only under `--release` (heavy tier command gains a release run for `frames::`), (b) keep debug and accept the tests measure debug. Recommendation: (a).
- Context: no compaction; stopped at ~110k tokens (over the 80k target) at this seam rather than start diagnosis.
