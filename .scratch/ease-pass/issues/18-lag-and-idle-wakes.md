# fix round: lag with 50 000 refs, and wakes at rest — find the cause

Status: ready
Blocked by: 16-fix-heavy-tier (merged)
Build with: debugger (Opus, high)

## Problem
- Review Major 6 (`docs/reviews/ease-pass-07-14.md:191-203`): release, 50 000 refs, p99 key-to-frame 293 / 292 / 977 ms (diff / detail pane / map view) vs 16 ms; slow keys wait ~240–280 ms queued, then draw in 8–80 ms; `idle_prefetch_us` max ~0.55 s (`docs/perf/ease-pass-review.txt:39,55,70,74`). Cause **not established**; review's candidates: `view/entries/branch_column.rs:86-139` `names_of` per frame (now edited by 15 — re-read), the idle prefetch step, fold counts.
- 16's heavy run (`docs/perf/16-heavy.txt:48-49`): `scrolling_the_100k_fixture_…_{pixel,unicode}_tier` fail at `frames.rs:393` — wakes with no input, `Produced::FoldedCounted`. The `fifty_thousand_refs_…` pair failed in the review's run but **passed** in 16's; difference not established.

## Scope
Find the cause of each by observation (timers, traces, bisect against 9cb03ce8), not by reading code. Measurement from a script into `docs/perf/18-lag.txt`; read summary lines. Then fix at the cause if the fix needs no decision; if it does (e.g. whether a fold count may wake the loop at rest), stop and report the choice with its lines. **Never loosen a budget or `frames.rs:393`.**

## Acceptance (tick each with its test/command)
- [x] Cause of the queue wait named with file:line and the measurement that shows it. (`docs/perf/18-lag.txt` SUMMARY cause 1: `scripts/18-sample.sh` samples + the [DEBUG-18] wake list; fixed, re-run by `scripts/18-lag.sh`)
- [x] Cause of the at-rest `FoldedCounted` wakes named with file:line; same or different cause as above, shown. (SUMMARY cause 2: different — fails identically with and without 50k refs, before and after the fix; `scripts/18-lag.sh` blocks "baseline, five tests" and "after"). **Not fixed: decision.**
- [x] The review-vs-16 difference on `fifty_thousand_refs_…` explained, or "not established" with what was tried. (SUMMARY review-vs-16: debug vs release; `BUILD=debug scripts/18-lag.sh` block)
- [x] If fixed: release p99 for the three 50k-ref tests and the two 100k scroll tests, before/after, from `docs/perf/18-lag.txt`. (SUMMARY fix table; three pane tests from their own p99 lines, closed tests from `scripts/18-trace-stats.sh`)
- [x] Four gates + heavy clippy green. (`cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`; `cargo clippy --all-targets --features heavy-tests -- -D warnings`; `cargo nextest run`: 7205 passed; `cargo test --doc`: ok. Heavy tier itself not run beyond the five `frames::` tests.)

## NOTES (debugger)
Status: queue wait fixed at the cause; at-rest wakes a decision. Scout: none (debugger ticket, cause found by `sample` + probe). No compaction.

- **Cause 1 (queue wait, cause not symptom):** `update::branch_pane::focused` rebuilt every candidate per call (format ~25k names, sort, parse each for identity), called on every update at rest (`update/mod.rs:265`), every loop turn mid-burst (`event_loop/running/turn.rs:344-346` via `rest_owed`), and every frame (`view/branch_pane.rs` `pane_view`). 5–10 ms a call at 50k refs; a startup burst of 64 ForcePushProbe events queued the first keys ~0.7 s. Fix: `refs::branch_list`, built once per refs read in `refs::badges`; `focused` is two hash lookups; `candidates()` (full list with kinds) only when a read is asked. Cost to the user: none visible; refs read does one more pass + sort per read on the job thread.
- **Cause 2 (at-rest `FoldedCounted`):** each is the answer to a `CountFolded` the app asks at rest for a folding merge the key brought on screen, one per answer (`update/fold_count.rs:27-43`); the harness counts every job event but a key's first Window as unasked (`tests/it/frames/readings.rs:243-258`). **Decision for the user/conductor:** (a) the harness treats a count answering a read the loop asked as asked (as it does Windows) — `frames.rs:393` unchanged, its reading of "asked" widened; or (b) the app counts the screen's folding merges with the window (no answer after rest) — app change, more reads up front. Not touched.
- **Review vs 16:** 16 ran debug (`scripts/ease-16-heavy.sh:10`, no `--release`); with cause 1 the debug loop fell 20–28 s behind, so idle keys arrived queued and the pauses held chance wakes.
- **Left:** at 50k refs the unicode draw is 3.1 ms p50 vs 0.4 ms without; `sample` puts most in `view/entries/branch_column.rs` `names_of` (ticket 17's file, not touched). Pixel closed p99 14.3 ms: under 16, close.
- **Touches:** `refs/mod.rs` (field + build; `AheadBehindCache` moved to `refs/ahead.rs` to stay ≤400 lines, path re-exported, callers unchanged: `jobs/runner/mod.rs`, refs tests — gates); `update/branch_pane.rs` (`Focused.candidates` → `Focused.trunk`); its users `view/branch_pane.rs` (trunk) and `update/picker/open_on.rs` (branch only, unchanged) — grep `branch_pane::focused|\.candidates`; MAP.md (`refs::branch_list` heading, map_coverage test); no keymap, capture, journal or other-crate change. Equivalence pinned by `update::branch_pane::tests::index` (3 tests).
