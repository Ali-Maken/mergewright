# m1-the-graph/69 — Review M2: every event clones AppState, 50k-ref badges by value

Status: done
Build with: **Opus** (high) — profiling attribution plus a state-sharing change on the event path.
Blocked by: none

From the M1 review round 1, "M2". The m1/64 `fifty_thousand_refs_…_unicode` flake is real: max 14.5 ms against 16.

## Read first
- docs/reviews/m1-the-graph.md "M2" only; KNOWN_GAPS.md "30h" row (idle prefetch and the clone suspicion).
- `crates/graft/MAP.md`; `crates/graft/src/update.rs` around 160–180 and 495–520; `crates/graft/src/refs/mod.rs` ~25–45; `crates/graft/tests/it/frames.rs` `check`.

## Scope
In:
1. Profile first: the release 50k-refs Unicode frames run, attributing per-frame time to the `AppState` clone vs badge lookup/render. Numbers into the ticket.
2. Fix what the profile names: likely `Arc<RefBadges>`, and `Arc` for other large immutable sub-states, so the per-event clone is O(1). Everything stays safe code with no `unsafe`.
3. Re-run the release perf run (four `frames::` tests) 3×, alone. Report p99/max per test before and after.
Out: raising any bar, asserting p99 instead of max, retries (all symptom fixes); file splits (70–74).

## Acceptance
- [x] Profile attribution in the ticket.
- [x] 50k-refs Unicode max well under 16 ms in all 3 runs; other frames tests not worse (one caveat, see results).
- [x] Report: cause or symptom; what it costs the user.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`

## Size
One context, under ~80k tokens. Near it, stop at a clean seam with a handoff note.

## Result (2026-09-15)

### Profile attribution
No sampling profiler on the machine (`samply` and `flamegraph` are not installed). Instead, `frames.rs` now prints a recorded-not-asserted probe, `state_clone_and_drop_us`. It times 64 laps of `drop(state.clone())` on the state the scroll starts from. Every event pays exactly this: `update` clones, and the loop drops the state it replaces.

| | clone+drop p50 / max | badge lookup |
|---|---|---|
| 50k refs, before | **~1.0 ms** / 1.7–1.9 ms | 18–19 ns each (`refs:` line), so under 1 µs for a screen of rows |
| fixture refs, before | 25 µs / 31 µs | — |
| 50k refs, after | 0 µs / 1–14 µs | unchanged |

A key frame runs `update` at least twice: once for the key and once for the `Window` answer. At the pixel tier the idle prefetch adds two more what-if `update`s per frame. So the 50k copy cost about 2 ms or more per frame, plus the tail of a 25k-entry `HashMap` of `Vec`s being freed. Badge lookup and rendering are negligible. Before the fix, the 50k-vs-fixture gap inside the same run was Unicode p50 +1.7 ms and p99 +3–4.5 ms. After, it is p50 +0.2 ms and p99 about +0.1 ms.

### Fix (cause)
`AppState::refs` is now `Option<Arc<RefBadges>>`. The refs answer is wrapped once in `took_fact`; views read it via `as_deref()`. No other large sub-state showed up: with fixture refs the whole clone+drop is 25 µs before and ~0 after. Test: `update::tests::the_next_state_shares_the_ref_badges_instead_of_copying_them` (`Arc::ptr_eq` across an event).

### Release `frames::` runs, p99 / max in µs (outputs in `target/m1-69/before-{1,2,3}.txt`, `after-{1,2,3}.txt`)

| test | before r1 | before r2 | before r3 | after r1 | after r2 | after r3 |
|---|---|---|---|---|---|---|
| Unicode, 50k refs | 13179 / 14396 | 12693 / 13590 | 11846 / 14929 | 5225 / 5484 | 5234 / 5303 | 5435 / 5645 |
| Unicode, fixture | 9080 / 9573 | 8674 / 8962 | 8693 / 9066 | 5239 / 5506 | 5105 / 5310 | 5222 / 5287 |
| Pixel, 50k refs | 7547 / 7651 | 7199 / 7605 | 7229 / 8861 | 6660 / 10161 | 6651 / 9808 | 6548 / 9233 |
| Pixel, fixture | 9336 / 11450 | 9123 / 11399 | 9032 / 10513 | 6370 / 9028 | 5970 / 9511 | 6653 / 9189 |

Pixel idle prefetch per frame, p50: 50k refs 3.3 ms → 0.6 ms; fixture 0.85 → 0.6 ms.

**Caveat, not explained.** The pixel 50k max rose from 7.6–8.9 ms to 9.2–10.2 ms, while its p99 fell. It now matches the fixture pixel max (9.0–9.5 ms after, 10.5–11.5 before). A guess, not verified: before the fix, each prefetch what-if cost ~1 ms for the clone. That changed how many strips got rasterised before a key interrupted, and so which key paid the worst frame. It is still 6 ms under the bar.

**Also unexplained:** the fixture runs got faster too (Unicode p99 ~8.8 → ~5.2 ms), even though their clone only fell from 25 µs to 0. Machine state may differ between the two sessions. The within-run 50k-vs-fixture comparison above doesn't depend on that.
