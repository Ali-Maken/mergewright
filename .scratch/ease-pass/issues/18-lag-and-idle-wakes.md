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
- [ ] Cause of the queue wait named with file:line and the measurement that shows it.
- [ ] Cause of the at-rest `FoldedCounted` wakes named with file:line; same or different cause as above, shown.
- [ ] The review-vs-16 difference on `fifty_thousand_refs_…` explained, or "not established" with what was tried.
- [ ] If fixed: release p99 for the three 50k-ref tests and the two 100k scroll tests, before/after, from `docs/perf/18-lag.txt`.
- [ ] Four gates + heavy clippy green.
