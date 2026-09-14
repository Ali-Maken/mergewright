# m1-the-graph/56 — Acceptance: the memory curve after ADR-0026

Status: partial (seam: measurements and code done; docs, heavy full run, perf run remain)
Build with: **Fable** (high) — the standing acceptance ticket: it decides what "memory tracks the viewport" looks like in numbers and writes the checklist that proves it.
Needs: every other ticket of this group (commit-handles-and-table, walk-reads-the-commit-graph, order-and-inclusion-on-handles, rows-hydrate-on-demand, **rows-pass-through** — split from 52 on 2026-09-14), plus **first-paint-at-a-million-named-cause** (its first-paint numbers and named cause are what the checklist reports) and **commit-graph-offer** (the end-to-end below accepts the offer on the 1M no-cg fixture).
Blocked by: 49, 50, 51, 51b, 51c, 51d, 52, 52b (rows-pass-through), 53, 54, 55

Amended 2026-09-14 (planner-fable, spec-only pass) with the user's condition on approving ADR-0026: wherever memory is **reported rather than failed** (1M without a commit-graph), the numbers must be visible in the milestone review file and in KNOWN_GAPS.md — not only in a results file.

## Read first
- ADR-0026 (all; "The gate at 1M" is what this ticket proves; "User decisions on approval" are the conditions)
- Ticket 46 "Curve" and "What holds memory at 1M" tables (the before); the measurement sections of the design tickets above (the after, per piece; the `G` odb-read and RSS numbers are in rows-pass-through); first-paint-at-a-million-named-cause "Acceptance" (the numbers and the named cause); commit-graph-offer "New UI" (what the offer line reads)
- PROMPT.md §1.2 "Incremental and virtualized", §6.3 "Performance budgets", §7.1 heavy tier
- TESTING.md "The heavy tier" and "Memory curve to a million commits (m1/46)"; KNOWN_GAPS.md rows "The 300 MB budget fails at a million commits", "No-cg `EmittedOrder` holds twice the table", "No heavy-tier guard on the memory curve's slope", "First paint at 1M commits is 292–383 ms", "The emitted order grows with history", "Least-recently-used is the wrong policy once checkpoints pass `max_checkpoints`", "Commit-graph offer plumbing unspecified (R2-4)" (closed by commit-graph-offer; confirm)
- `crates/graft-fixtures/MAP.md` (`synthetic`, `measure`, `peak_rss`); `scripts/memory-curve.sh`

## Scope
In:
- **The command:** `scripts/memory-curve.sh 10000 100000 1000000` re-run as it is (release, 3 runs per cell, small/no-cg, small/cg, realistic/no-cg), output to `docs/shots/m1-memory-curve.txt` (the table plus per-cell peak RSS, footprint, first paint, `G` time, odb objects read during `G`, `CommitTable::overflow_len`, the fixture's pack size on disk). Add the `1M realistic, cg` cell if the script lacks it: it is the kernel-shaped case.
- **The offer, end to end, on the reported cell:** open a temp copy of the 1M small no-cg fixture in a pty at 120×40; capture the first frame (the offer line reads as commit-graph-offer specifies); press `B`; capture the written frame and the wall time of the write; re-run the curve on that same repository: it is now a cg cell and must meet the gate. Captures → `docs/shots/m1-memory-curve-offer.txt`; the before/after pair (no-cg reported → cg gated) is one row in TESTING.md. This is the promise-level proof that the fallback path is not silent and has a way out.
- **The heavy-tier slope guard:** `heavy::peak_rss_slope_per_commit_with_a_commit_graph` — peak RSS on the 100k cg fixture minus peak RSS on the 10k cg fixture, divided by 90,000, ≤ 150 B (ADR-0026). Same harness as `budgets_hold_*`, serialised under `MEASURING`. Prove it fails by tightening once, revert.
- **A checkpoint-thinning check at 1M:** 64 checkpoints × 2,000 rows cover 128k; at 1M the LRU leaves checkpoint-less stretches (KNOWN_GAPS row). Measure `g` after `G` and a `Find` to row ~900k on the 1M cg fixture (time and rows replayed, from `Work`). If a jump replays more than ~50k rows, write the finding and a spec seam for the conductor (thin instead of LRU); do not build it here.
- **`TESTING.md`:** replace the m1/46 curve table with the after table; per cell what right looks like, in numbers: 1M cg peak RSS < 300 MB (the gate); 1M no-cg heap ≤ 100 B/commit and footprint < 300 MB, RSS reported with the pack bytes it includes; 1M first paint < 500 ms and its number against 150 with the cause first-paint-at-a-million-named-cause named; 100k and 10k under 300 as before; slope ≤ 150 B/commit; the offer row. A one-line procedure per number (which column of `docs/shots/m1-memory-curve.txt`).
- **`KNOWN_GAPS.md` — the reported cell carries its numbers:** the "300 MB budget fails at 1M" row is rewritten as "1M without a commit-graph is reported, not gated (ADR-0026)" and holds, **in the row itself**, the 1M no-cg numbers: peak RSS (all 3 runs), footprint, heap B/commit, the pack size those pages come from, and first paint — a reader must not need to open a results file. Its expiry: "stands while the user declines the offer; the offer is built (commit-graph-offer)". Close or rewrite the other rows listed above with their numbers; the 1M provisional-paint option stays open only if first-paint-at-a-million-named-cause ended with that recommendation, expiry "user decision at the M1 checkpoint".
- **`## For the M1 review` section in this ticket:** the same reported-cell table, verbatim. **The M1 review file (`docs/reviews/m1-the-graph.md`) must reproduce it** — the conductor hands this section to the reviewer, and the milestone is not done while the review file lacks the numbers. Say so in the report so the conductor logs it.
- **DESIGN.md:** §3.1 `EmittedOrder` line (~90 B/commit → handles, ~8 B), `walker_state` line (dense by handle; the graph is read, not the odb); §3.6 "the emitted order… is the one term that grows with history" paragraph → the after numbers. One sentence each; no restructuring.
- **Context guard:** every run writes to files under `target/` and `docs/shots/`; the report names paths and the verdict line, never pastes a curve or a capture.

Out: any fix. A cell that misses is reported with the holder attribution (`prototypes/heap-profile-27b`, `owners.py`) and goes back to the conductor; this ticket does not tune.

## Signatures
None new. Calls: `graft_fixtures::synthetic::Scale::{of, with_commit_graph, with_realistic_commits}`, `graft_fixtures::peak_rss`, `examples/budgets` (and its `--phases`), `StreamingLayout::work` (replayed rows, `hydrated`, `merge_reads`), `CommitTable::overflow_len`, `graft_git::commit_graph::status` (the after-write count).

## Acceptance
- [x] `docs/shots/m1-memory-curve.txt` committed: 10k / 100k / 1M × {small no-cg, small cg, realistic no-cg, realistic cg}, 3 runs each, the columns above.
- [x] 1M cg (small and realistic): peak RSS < 300 MB on all runs. 100k and 10k: < 300 MB, all variants.
- [ ] 1M no-cg: heap after `G` ≤ 100 B/commit, footprint < 300 MB; RSS reported next to the fixture's pack size.
- [ ] 1M first paint: < 500 ms on every run; the number written against 150 ms with a yes/no and the named cause of any residual.
- [x] The offer end-to-end: `docs/shots/m1-memory-curve-offer.txt` committed; the same repository's post-write curve cell meets the gate; write time recorded.
- [ ] Slope guard in the heavy tier, seen failing by tightening, passing at 150 B/commit; heavy tier 100 % green run once, alone.
- [x] The 1M jump measurements written (`g` after `G`, `Find` to ~900k: time, rows replayed).
- [ ] KNOWN_GAPS.md's reported-cell row holds the numbers themselves; `## For the M1 review` section written; TESTING.md, DESIGN.md updated as scoped; ADR-0026 status line updated to what the user decided.
- [ ] Report: paths only, plus one line per budget: pass/fail and the number; one line saying the review file must carry the reported-cell table.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`; heavy tier and the release perf run (ADR-0021) once, alone, at the end.

## Size
One context, under ~80k tokens: the 1M runs are ~10 s each, 3 × 12 cells ≈ 6 minutes wall, the 1M write once, output to files. If the 1M realistic-cg fixture build is too slow or large for the laptop, stop and say so with the number; don't drop the cell silently.

## Result so far (2026-09-14, builder-fable; seam reached, see Handoff)

Code: `examples/budgets` gained `--gate` (only open → paint → G → g, so `/usr/bin/time -l` describes the gated path alone), per-stage `peak RSS now` lines, and two far jumps (`budget_run::far_jump`: rows/2 and 9·rows/10 after a cold `G`). `scripts/memory-curve.sh`: 12 cells (× realistic-cg), 3 gate runs + 1 counters run per cell, pack size on disk, `--repo <path> <label>` for one repository, `APPEND=1`. `heavy::peak_rss_slope_per_commit_with_a_commit_graph` added (150 B). pty script: `.scratch/m1-the-graph/pty-memory-curve-offer.py`.

Raw: `docs/shots/m1-memory-curve.txt` (release, 3 runs per cell, in-process getrusage peak = the gate; footprint = `time -l` anonymous pages). Offer: `docs/shots/m1-memory-curve-offer.txt` (120×40 frames, user judges by eye; write time; the post-write cell).

**Measurement harness finding:** `budgets`'s counters (`order_bytes`, `cold_jump`, `far_jump`) lay history out again through `direct_layout` and hold 4× the app path (100k cg: 124 MiB process max RSS vs 33 MiB for the gated path; 1M cg: 997 vs 240). The first curve run took its footprint from that whole process, which is why `--gate` exists. The holder is `order_bytes` first (10k: 12.7 → 20.5 MiB). Harness only; the app path is what the gate measures. KNOWN_GAPS row to write.

## For the M1 review

The reported cell (1M without a commit-graph) and its neighbours. Release, 3 runs, `scripts/memory-curve.sh 10000 100000 1000000` on 2026-09-14; peak RSS = in-process getrusage over open → first paint → `G` → `g` (the gate); footprint = `/usr/bin/time -l` peak memory footprint of `budgets --gate` (anonymous pages, excludes clean file-backed pack pages); first paint run 1 is cold (see below).

| Cell | pack on disk | peak RSS (3 runs) | footprint | first paint (run 1 / 2 / 3) | `G` | `overflow_len` | verdict |
|---|---|---|---|---|---|---|---|
| 1M small, **no cg** | 300 MiB | **476.3 / 477.3 / 478.2 MiB** | 186.5–188.3 MiB | 983 / 107 / 109 ms | 5.31–5.42 s | 1,000,000 | **reported** (ADR-0026): footprint < 300 MB yes; RSS = footprint + the pack pages the full walk touched |
| 1M realistic, **no cg** | 364 MiB | **537.5 / 539.2 / 539.7 MiB** | 186.7–188.9 MiB | 1,040 / 112 / 112 ms | 5.73–5.98 s | 1,000,000 | **reported**: same |
| 1M small, cg | 297 MiB | 238.6 / 239.1 / 239.9 MiB | 36.8–38.1 MiB | 975 / 101 / 101 ms | 496–507 ms | 0 | **gate pass** (< 300 MB) |
| 1M realistic, cg | 365 MiB | 245.0 / 245.2 / 246.0 MiB | 40.4–41.4 MiB | 1,088 / 99 / 100 ms | 499–518 ms | 0 | **gate pass** |
| 1M small, no cg → **after `B`** (the offer, same repository) | 297 MiB | 236.9 / 236.9 / 238.5 MiB | 35.2–36.8 MiB | 122 / 94 / 92 ms | 509–517 ms | 0 | **gate pass**; write 5.1 s wall (`git commit-graph write --reachable`) |
| 100k: small / cg / realistic / realistic cg | 30–36 MiB | 69.2–69.3 / 32.7–33.0 / 76.0–76.5 / 37.0–37.2 MiB | 37 / 9 / 38 / 13 MiB | 95, 12, 12 / 13, 11, 11 / 100, 12, 12 / 106, 11, 11 ms | 463–475 / 48–49 / 503–521 / 50–51 ms | 100k / 0 / 100k / 0 | pass |
| 10k: small / cg / realistic / realistic cg | 2–3 MiB | 17.4–17.8 / 12.5–12.7 / 25.5–26.2 / 16.4–16.6 MiB | 11 / 6 / 19 / 10 MiB | ≤ 19 ms all | 37–48 / 12–13 / 39–42 / 13–16 ms | 10k / 0 / 10k / 0 | pass |

- **No-cg heap per commit (ADR-0026 asks ≤ 100 B):** footprint rise 100k → 1M no-cg = (188.3 − 37.2) MiB / 900k = **~176 B/commit at peak — misses 100**. Peak, not "after `G`" (no in-process footprint reading exists; `graft_fixtures` has only getrusage). Suspected holder: the no-cg promotion sort's transient dense arrays (parents `Vec` per commit) on top of the interner (~40 B) and order (9 B); not attributed with a profile. Footprint < 300 MB holds (188 MiB).
- **First paint at 1M:** warm runs 92–112 ms on every path — under 150 ms, yes. **Run 1 of every 1M cell is 975–1,088 ms — over 500 ms.** Cause not proven (m1/55 left it uninvestigated). Evidence: in the earlier curve run the realistic-cg fixture, built minutes before, painted in 134 ms on run 1; in this run, after ~700 MiB of other 1M packs went through the page cache, every 1M cell's run 1 was ~1 s. Suspect: the 6,498 loose ref files and the pack `.idx` not in page cache. Proof needs `sudo purge` between runs; not run.
- **The slope guard (ADR-0026, ≤ 150 B/commit, 10k cg → 100k cg): FAILS at 237 B/commit** (12.5 → 33.0 MiB over 90k). 19.6 of the 20.5 MiB rise is already held **at first paint** (8.6 → 28.2 MiB), i.e. the open path, not the walk: the commit-graph pages (~56 B/commit, file-backed, counted by `ru_maxrss`) plus the tips' header lookups (KNOWN_GAPS, +147 MiB at 1M ≈ 150 B/commit). The 100k → 1M cg slope is the same, 240 B/commit. No fix here (out of scope): the test stands at 150 B and is red until that residual is fixed or the user re-sets the number. Not tightened-to-fail: it fails as specified.
- **Checkpoint thinning at 1M (`g` after `G` 0.62–0.71 ms; jumps after a cold `G`, both cg and no-cg):** `Find` to row 900,000: 1.9–2.4 ms, 500 rows replayed. **`Find` to row 500,000: 1.39–1.46 s, 500,500 rows replayed** — the 64 LRU checkpoints × 2,000 rows anchor only the last 128k rows laid out; the middle has none. Well over the ~50k line. **Spec seam for the conductor:** replace LRU with thinning that keeps checkpoints evenly spaced (drop every other one when full, so spacing doubles: at 1M ≈ 16k rows apart, a jump replays ≤ 16k links ≈ 45 ms at the measured 2.9 µs/row); `layout::cache` / `StreamingLayout` checkpoints only; no interface change. Not built here.
- The 2× check (ADR-0026): `overflow_len` = commits without a cg, 0 with one, every cell. `EmittedOrder` 9–12 B/commit, every cell. Commits read from the odb during a cold `G`: 1,500 at 10k/100k, 1,501 at 1M.

## Handoff (2026-09-14, builder-fable; ~80k tokens, no compaction)

Done and committed: the code above, `docs/shots/m1-memory-curve.txt`, `docs/shots/m1-memory-curve-offer.txt`, fast gates green (fmt, clippy, clippy heavy, cargo test).

Remaining, in order (all doc work except the two runs):
1. **KNOWN_GAPS.md**: rewrite "The 300 MB budget fails at a million commits" as "1M without a commit-graph is reported, not gated (ADR-0026)" carrying the two no-cg rows above verbatim (RSS ×3, footprint, ~176 B/commit, pack size, first paint), expiry "stands while the user declines the offer; the offer is built (m1/54)". Close "No-cg `EmittedOrder` holds twice the table" (overflow_len = commits, order 9 B). Rewrite "No heavy-tier guard on the memory curve's slope" → guard exists, **red at 237 B/commit**, holder = open path (tips' header lookups + cg pages). "Least-recently-used is the wrong policy" → measured: 500k jump replays 500,500 rows, 1.4 s; the thinning seam above. "The emitted order grows with history" → 9–12 B/commit, closed. New rows: the ~1 s cold first paint at 1M (suspect page cache, unproven); the no-cg ~176 B/commit peak vs 100; the `budgets` counters harness holding 4× the app path. Confirm "Commit-graph offer plumbing" row closed (it is).
2. **TESTING.md** "Memory curve to a million commits": replace the m1/46 table with the table above (all 12 cells + the offer row), one-line procedure per number (which line of `docs/shots/m1-memory-curve.txt`: `runN: first paint …` for RSS/paint/G, `runN: footprint …`, `once: far jump …`), the slope guard in "What is in the heavy tier today", `budgets --gate`.
3. **DESIGN.md** §3.1 line 597 `EmittedOrder` (~90 B → handles, 9–12 B measured), line 601 `walker_state` (dense by handle; the graph is read), §3.6 lines 826–827 (the one term that grows → 9 B/commit; what grows now is pack pages without a cg).
4. **ADR-0026** status line: accepted with the user's two conditions; add "Landed: m1/49–56" and the slope-guard miss.
5. Heavy tier full run once, alone (`cargo test --features heavy-tests -- --test-threads=2`): expect **1 red** (`peak_rss_slope…`), everything else green — report it, don't loosen. Then the release perf run (ADR-0021).
6. Tick the remaining acceptance boxes honestly (slope: fails; first paint: warm yes / cold no).

Gotchas: run `scripts/memory-curve.sh` only with nothing else on the machine; the 1M cells take ~90 s all told with `--gate`. `target/56/*.txt` hold the per-fixture jump runs (`jumps-1m*.txt`). The offer copy was deleted after its run (336 MiB); re-run the pty script from a fresh `cp -R` of the 1M no-cg fixture if the frames need retaking.
