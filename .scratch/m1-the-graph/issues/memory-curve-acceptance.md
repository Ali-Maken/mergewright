# Acceptance: the memory curve after ADR-0026

Status: draft
Build with: **Fable** (high) — the standing acceptance ticket: it decides what "memory tracks the viewport" looks like in numbers and writes the checklist that proves it.
Needs: every other ticket of this group (commit-handles-and-table, walk-reads-the-commit-graph, order-and-inclusion-on-handles, rows-hydrate-on-demand).

## Read first
- ADR-0026 (all; "The gate at 1M" is what this ticket proves, as approved or amended by the user)
- Ticket 46 "Curve" and "What holds memory at 1M" tables (the before); the measurement sections of the four tickets above (the after, per piece)
- PROMPT.md §1.2 "Incremental and virtualized", §6.3 "Performance budgets", §7.1 heavy tier
- TESTING.md "The heavy tier" and "Memory curve to a million commits (m1/46)"; KNOWN_GAPS.md rows "The 300 MB budget fails at a million commits", "No-cg `EmittedOrder` holds twice the table", "No heavy-tier guard on the memory curve's slope", "First paint at 1M commits is 292–383 ms", "The emitted order grows with history", "Least-recently-used is the wrong policy once checkpoints pass `max_checkpoints`"
- `crates/graft-fixtures/MAP.md` (`synthetic`, `measure`, `peak_rss`); `scripts/memory-curve.sh`

## Scope
In:
- **The command:** `scripts/memory-curve.sh 10000 100000 1000000` re-run as it is (release, 3 runs per cell, small/no-cg, small/cg, realistic/no-cg), output to `docs/shots/m1-memory-curve.txt` (the table plus per-cell peak RSS, footprint, first paint, `G` time, odb objects read during `G`, `CommitTable::overflow_len`). Add the `1M realistic, cg` cell if the script lacks it: it is the kernel-shaped case.
- **The heavy-tier slope guard:** `heavy::peak_rss_slope_per_commit_with_a_commit_graph` — peak RSS on the 100k cg fixture minus peak RSS on the 10k cg fixture, divided by 90,000, ≤ 150 B (ADR-0026). Same harness as `budgets_hold_*`, serialised under `MEASURING`. Prove it fails by tightening once, revert.
- **A checkpoint-thinning check at 1M:** 64 checkpoints × 2,000 rows cover 128k; at 1M the LRU leaves checkpoint-less stretches (KNOWN_GAPS row). Measure `g` after `G` and a `Find` to row ~900k on the 1M cg fixture (time and rows replayed, from `Work`). If a jump replays more than ~50k rows, write the finding and a spec seam for the conductor (thin instead of LRU); do not build it here.
- **`TESTING.md`:** replace the m1/46 curve table with the after table; per cell what right looks like, in numbers: 1M cg peak RSS < 300 MB (the gate); 1M no-cg heap ≤ 100 B/commit and footprint < 300 MB, RSS reported with the pack bytes it includes; 1M first paint < 500 ms and its number against 150; 100k and 10k under 300 as before; slope ≤ 150 B/commit. A one-line procedure per number (which column of `docs/shots/m1-memory-curve.txt`).
- **`KNOWN_GAPS.md`:** close or rewrite the rows listed above with the numbers; the no-cg pack-page residual stays open, expiry M2 (the commit-graph offer); the 1M provisional-paint option stays open only if 1M first paint misses 150 ms, expiry "user decision at the M1 checkpoint".
- **DESIGN.md:** §3.1 `EmittedOrder` line (~90 B/commit → handles, ~8 B), `walker_state` line (dense by handle; the graph is read, not the odb); §3.6 "the emitted order… is the one term that grows with history" paragraph → the after numbers. One sentence each; no restructuring.
- **Context guard:** every run writes to files under `target/` and `docs/shots/`; the report names paths and the verdict line, never pastes a curve.

Out: any fix. A cell that misses is reported with the holder attribution (`prototypes/heap-profile-27b`, `owners.py`) and goes back to the conductor; this ticket does not tune.

## Signatures
None new. Calls: `graft_fixtures::synthetic::Scale::{of, with_commit_graph, with_realistic_commits}`, `graft_fixtures::peak_rss`, `examples/budgets`, `StreamingLayout::work` (replayed rows), `CommitTable::overflow_len`.

## Acceptance
- [ ] `docs/shots/m1-memory-curve.txt` committed: 10k / 100k / 1M × {small no-cg, small cg, realistic no-cg, realistic cg}, 3 runs each, the columns above.
- [ ] 1M cg (small and realistic): peak RSS < 300 MB on all runs. 100k and 10k: < 300 MB, all variants.
- [ ] 1M no-cg: heap after `G` ≤ 100 B/commit, footprint < 300 MB; RSS reported next to the fixture's pack size.
- [ ] 1M first paint: < 500 ms on every run; the number written against 150 ms with a yes/no.
- [ ] Slope guard in the heavy tier, seen failing by tightening, passing at 150 B/commit; heavy tier 100 % green run once, alone.
- [ ] The 1M jump measurements written (`g` after `G`, `Find` to ~900k: time, rows replayed).
- [ ] TESTING.md, KNOWN_GAPS.md, DESIGN.md updated as scoped; ADR-0026 status line updated to what the user decided.
- [ ] Report: paths only, plus one line per budget: pass/fail and the number.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`; heavy tier and the release perf run (ADR-0021) once, alone, at the end.

## Size
One context, under ~80k tokens: the 1M runs are ~10 s each, 3 × 12 cells ≈ 6 minutes wall, output to files. If the 1M realistic-cg fixture build is too slow or large for the laptop, stop and say so with the number; don't drop the cell silently.
