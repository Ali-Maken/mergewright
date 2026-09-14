# m1-the-graph/56c — Memory curve acceptance: the runs, the numbers, the mechanical doc edits

Status: done (per session log; builder left status stale — conductor, 2026-09-14)
Build with: **Sonnet** (medium) — runs existing commands and writes their numbers down; no verdicts. User decision 2026-09-14: 56's remainder split so Fable only judges (56d).
Blocked by: 59b, 60, 63 (58b, 61 done)

Split from 56 (conductor, 2026-09-14) along "Spec-only pass 2 → Remaining here". Scope and commands are 56's, unchanged; only who does which part moved. **This ticket records numbers; it never says pass/fail on a budget** — 56d does.

## Read first (only these)
- `56-memory-curve-acceptance.md`: "Scope" (the command, the columns, the offer, the jumps, context guard), "Signatures", "Gates", "Spec-only pass 2 → Remaining here" items 1, 3, 4, 5.
- Result sections of 59b, 60 and 61 (their numbers are what the re-run confirms).
- TESTING.md "The heavy tier" and "Memory curve to a million commits (m1/46)"; DESIGN.md §3.1 `EmittedOrder` and `walker_state` lines, §3.6 the "grows with history" paragraph; ADR-0026 status line only.
- `scripts/memory-curve.sh`; `crates/graft-fixtures/MAP.md`.

## Scope
In:
1. Remaining item 1: full curve re-run (`scripts/memory-curve.sh 10000 100000 1000000`, release, 3 runs, 12 cells) → regenerate `docs/shots/m1-memory-curve.txt`; far jumps (rows/2, 9·rows/10) on both 1M fixtures; offer pair only if the offer line or its numbers changed.
2. Remaining item 5: heavy tier full run once, alone (`cargo test --release --features heavy-tests -- --test-threads=2`), then the release perf run (ADR-0021) with the 1M `frames::` scroll numbers. Report a red test with its output path; don't touch it.
3. **`## Numbers` section in this ticket** — the one input 56d reads. One table per group, every number with its unit and the path of the raw file it came from:
   - curve cells: peak RSS (3 runs), footprint peak and after `G`, heap B/commit, first paint (run 1 and warm), `G` time, odb objects during `G`, `overflow_len`, pack size, checkpoint stride, **live heap after `g` (100k, 1M) and its slope** (copy from 59's `target/59/prof-{100k,1M}-run{1,2,3}.txt` and 59b's `target/59b/prof-{1M,100k}.txt`; if `prototypes/heap-profile-59` still builds, one fresh run each to `target/56c/prof-{100k,1M}.txt`, ≈30 s total);
   - far jumps: time and rows replayed at 500k and ~900k, both 1M fixtures;
   - slope guard: heap and RSS slopes;
   - heavy tier: pass/fail count, names of any red tests;
   - perf run: each `frames::` budget number next to its bar.
   Bars go in a column **copied from TESTING.md/ADR-0026 as written**, never computed or changed; the no-cg heap row's bar column reads "live ≤ 100 (ADR-0026 amendment 2); footprint: reported, floor ~140".
4. Remaining items 3–4, mechanical parts only: KNOWN_GAPS rows listed in 56's item 3 get their **numbers** updated (leave each row's verdict/expiry sentence to 56d; mark it `verdict: 56d`); TESTING.md curve table replaced with the new numbers; DESIGN.md §3.1/§3.6 one sentence each with the after numbers.
5. Context guard: all run output to `target/56c/` and `docs/shots/`; never paste curves or logs into your context beyond the numbers you copy.

Out: pass/fail verdicts, the `## For the M1 review` section, ADR-0026's status line, acceptance boxes in 56, any fix, any bar.

## Acceptance
- [x] `docs/shots/m1-memory-curve.txt` regenerated, 12 cells × 3 runs.
- [x] `## Numbers` complete as scoped, every row with a raw path; bars copied verbatim.
- [x] Heavy tier run once alone and release perf run done; results in `## Numbers`.
- [x] KNOWN_GAPS numbers, TESTING.md table, DESIGN.md sentences updated; verdict sentences left for 56d.
- [x] Gates (56's list) green. A 1M fixture too slow or large: stop, say so with the number.
- [x] Report: paths only; no verdicts.

Status: done.

## Size
One context, under ~60k. Runs ~15–25 min wall, all to files.

## Numbers (2026-09-14, builder-sonnet)

Raw: `docs/shots/m1-memory-curve.txt` (full re-run), `docs/shots/m1-memory-curve-offer.txt` (re-taken), `target/56c/heavy-full.log`, `target/56c/perf-run.log`, `target/59b/prof-{100k,1M}.txt`, `target/61/guard-{1,2,3}.txt`.

**Curve cells** — release, 3 runs/cell, `scripts/memory-curve.sh 10000 100000 1000000` (`docs/shots/m1-memory-curve.txt`). Peak RSS = process max RSS (`time -l`); footprint = `time -l` peak memory footprint (anonymous pages).

| Cell | pack on disk | peak RSS (run1/2/3) | footprint (run1/2/3) | first paint (run1/2/3) | `G` (run1/2/3) | `overflow_len` | checkpoint stride | Bar (copied, ADR-0026) |
|---|---|---|---|---|---|---|---|---|
| 10k small, no cg | 2 MiB | 17.0/15.8/15.7 MiB | 10.5/9.4/9.3 MiB | 23.3/2.5/2.4 ms | 60.9/37.5/36.8 ms | 10,000 | 2,000 | < 300 MB |
| 10k small, cg | 2 MiB | 12.1/12.0/12.1 MiB | 6.4/6.3/6.4 MiB | 11.0/2.2/2.1 ms | 16.1/12.8/12.7 ms | 0 | 2,000 | < 300 MB |
| 10k realistic, no cg | 3 MiB | 24.6/24.5/24.6 MiB | 17.5/17.4/17.5 MiB | 13.5/2.5/2.5 ms | 42.2/40.0/40.0 ms | 10,000 | 2,000 | < 300 MB |
| 10k realistic, cg | 3 MiB | 15.9/16.0/16.0 MiB | 10.0/10.1/10.1 MiB | 11.2/2.1/2.2 ms | 17.5/13.5/13.6 ms | 0 | 2,000 | < 300 MB |
| 100k small, no cg | 29 MiB | 68.1/67.6/68.0 MiB | 36.0/35.5/36.0 MiB | 95.9/11.7/11.9 ms | 506.5/469.3/468.6 ms | 100,000 | 2,000 | < 300 MB |
| 100k small, cg | 29 MiB | 26.8/26.6/26.8 MiB | 9.3/9.1/9.3 MiB | 16.4/12.5/10.8 ms | 58.0/79.2/51.9 ms | 0 | 2,000 | < 300 MB |
| 100k realistic, no cg | 35 MiB | 75.6/75.8/76.1 MiB | 37.4/37.6/37.9 MiB | 108.5/12.2/13.8 ms | 544.0/516.4/537.0 ms | 100,000 | 2,000 | < 300 MB |
| 100k realistic, cg | 35 MiB | 30.5/30.5/30.3 MiB | 12.8/12.8/12.6 MiB | 79.8/10.1/10.0 ms | 60.3/50.5/50.3 ms | 0 | 2,000 | < 300 MB |
| **1M small, no cg** | 297 MiB | **445.5/445.2/444.7 MiB** | **155.6/155.3/154.8 MiB** | 997.2/112.2/103.8 ms | 6.083/5.459/5.330 s | 1,000,000 | 16,000 | live heap ≤ 100 (ADR-0026 amendment 2); footprint: reported, floor ~140 |
| 1M small, cg | 297 MiB | 174.3/173.5/174.3 MiB | 37.8/37.0/37.8 MiB | 709.7/90.5/90.0 ms | 564.1/538.7/526.8 ms | 0 | 16,000 | < 300 MB |
| **1M realistic, no cg** | 358 MiB | **507.5/508.5/508.8 MiB** | **156.7/157.6/157.9 MiB** | 1031.6/108.2/110.6 ms | 6.300/5.877/6.155 s | 1,000,000 | 16,000 | live heap ≤ 100 (ADR-0026 amendment 2); footprint: reported, floor ~140 |
| 1M realistic, cg | 358 MiB | 183.7/182.6/183.2 MiB | 46.7/45.5/46.2 MiB | 789.6/88.3/91.2 ms | 609.4/566.4/636.8 ms | 0 | 16,000 | < 300 MB |
| 1M small, no cg → **after `B`** (offer, same repo, `docs/shots/m1-memory-curve-offer.txt`) | 297 MiB | 173.4/172.8/173.9 MiB | 37.0/36.3/37.5 MiB | 124.3/88.7/84.4 ms | 534.7/541.7/552.3 ms | 0 | 16,000 | < 300 MB (write 5.3 s wall, `git commit-graph write --reachable`) |

**Live heap after `g`** (1M, 100k; `target/59b/prof-{1M,100k}.txt`, `prototypes/heap-profile-59`, reused from 59b — fresh, post-fix run, no re-run needed): 100k 27.0 MiB, 1M 92.5 MiB → **76 B/commit**. Footprint slope (from the curve cells above, 100k→1M no-cg small): (154.8–155.6 − 35.5–36.0) MiB / 900,000 ≈ **138–140 B/commit**. Bar: live heap ≤ 100 (pass, 76); footprint reported, floor ~140 (not gated further).

**Far jumps** (rows/2, 9·rows/10; both 1M fixtures shown, plus cg variants; from the `once:` lines of `docs/shots/m1-memory-curve.txt`):

| Fixture | half (row 500,000) | nine-tenths (row 900,000) | stride |
|---|---|---|---|
| 1M small, no cg | 13.0 ms, 4,500 rows | 13.3 ms, 4,500 rows | 16,000 |
| 1M small, cg | 14.9 ms, 4,500 rows | 14.9 ms, 4,500 rows | 16,000 |
| 1M realistic, no cg | 15.3 ms, 4,500 rows | 14.5 ms, 4,500 rows | 16,000 |
| 1M realistic, cg | 16.3 ms, 4,500 rows | 16.0 ms, 4,500 rows | 16,000 |
| 1M small, after `B` (cg) | 14.9 ms, 4,500 rows | 15.0 ms, 4,500 rows | 16,000 |

Before m1-the-graph/60 (LRU): row 500,000 replayed 500,500 rows, 1.39–1.46 s.

**Slope guard** (`heavy::slopes_per_commit_with_a_commit_graph`, `target/61/guard-{1,2,3}.txt`, release, 3 runs alone — reused from 61, not re-run: the guard's own fixtures/code unchanged by 56c): heap slope 29/31/29 B/commit (bar ≤ 50); RSS slope 167/169/167 B/commit (bar ≤ 200, floor ~166 named).

**Heavy tier**, full run once alone, release (`cargo test --release --features heavy-tests -- --test-threads=2`, `target/56c/heavy-full.log`): **691 passed, 0 failed, 2 ignored** (0 red).

**Release perf run** (ADR-0021, `cargo test --release -p graft --features heavy-tests --test it -- frames:: heavy::budgets_hold --test-threads=1 --nocapture`, `target/56c/perf-run.log`): **10 passed, 0 failed**, 21 filtered out (non-matching tests).

| Test | Numbers | Bar |
|---|---|---|
| `frames::fifty_thousand_refs…pixel` | p50 4,348 µs, p99 5,740 µs, max 7,523 µs | ≤ 16 ms |
| `frames::fifty_thousand_refs…unicode` | p50 5,576 µs, p99 10,003 µs, max 11,739 µs | ≤ 16 ms |
| `frames::scrolling_the_100k_fixture…pixel` | p50 4,135 µs, p99 6,218 µs, max 7,397 µs | ≤ 16 ms |
| `frames::scrolling_the_100k_fixture…unicode` | p50 3,912 µs, p99 5,318 µs, max 5,581 µs | ≤ 16 ms |
| `heavy::budgets_hold_100k_with_50k_refs` | first_paint 40,652 µs, peak_rss 96.3 MiB | first paint ≤ 150 ms warm; RSS < 300 MB |
| `heavy::budgets_hold_100k_cg` | first_paint 12,388 µs, peak_rss 27.0 MiB | same |
| `heavy::budgets_hold_100k_no_cg` | first_paint 14,117 µs, peak_rss 68.8 MiB | same |
| `heavy::budgets_hold_100k_realistic_cg` | first_paint 54,867 µs, peak_rss 30.7 MiB | same |
| `heavy::budgets_hold_100k_realistic_no_cg` | first_paint 14,421 µs, peak_rss 74.7 MiB | same |

No 1M-scale `frames::` test exists in the codebase (only 100k fixture and 50k-refs synthetic scroll scripts); the 1M scroll-shaped numbers this perf run covers are the far-jump table above (worker-thread jumps, not a frame-budget test). Flagging for the conductor/56d, not building one here (out of scope).
