# m1-the-graph/46 — Memory curve to a million commits

Status: done — budget FAILS at 1M (881–949 MiB); design finding below for planner-fable
Build with: **Opus** (high) — memory under background history loading; heavy tier only
Blocked by: 43
Leaves runnable: yes — one heavy-tier command reports peak RSS against history size.

Why (user, 2026-09-13, for 27/30): "git/git peaks at 219–226 MiB with 85k commits. If memory tracked the viewport that number would be roughly the same at 1M. Report the curve — peak RSS at 10k, 100k and 1M commits — and say what holds memory as history grows: rows, commit order, gix caches or refs. Then say plainly whether the 300 MB budget survives a million-commit branch. If it doesn't, that's a design finding for planner-fable, not a number to adjust."

## Read first
- PROMPT.md §1.2 (budgets), §7.1 (heavy tier)
- Tickets 27, 27b, 27c, 27d result notes (fixes B, C, D; what the git/git numbers measured); ADR-0008
- TESTING.md "The heavy tier"; KNOWN_GAPS git/git RSS headroom row
- `crates/graft-fixtures/MAP.md`, `crates/graft/MAP.md`, `crates/graft-git/MAP.md`; scouts for source

## Scope
In:
- Heavy tier only (`--features heavy-tests`, release build, one heavy job at a time). Generate linear-ish and merge-heavy synthetic histories at **10k, 100k, 1M** commits (reuse graft-fixtures generators; if 1M generation is too slow or large for the laptop, stop and say so with the number — don't shrink silently).
- Measure peak RSS for: open → first paint → full background load → scroll to bottom, per size. Same method as 27d (ru_maxrss; ADR records RSS as the gate).
- Attribute the memory: rows / laid-out lanes, commit order (topological index), gix object and pack caches, refs — by measurement (counters, allocator stats, or A/B with each piece disabled), not by guess. Say how each scales (flat, per-commit bytes).
- Report the curve as a table in the ticket and in TESTING.md next to the heavy-tier budgets.
- Verdict, plainly: does the 300 MB budget survive a 1M-commit branch? If not → stop there: write the finding (what grows, per-commit cost, projected 1M peak) for planner-fable. **Don't change the budget, don't tune a cap to squeeze under it.**
- A heavy-tier guard for the curve's slope if it's cheap; otherwise a KNOWN_GAPS row.

Out: memory fixes (they follow from the verdict, via planner-fable if the design is wrong).

## Acceptance
- [x] Table: size × history shape → peak RSS, with the phase where it peaks.
- [x] Attribution by measurement, per component, with scaling.
- [x] Plain verdict on 300 MB at 1M.
- [x] If it fails: design finding written for planner-fable, no budget change.
- [x] Gates pass (fast tier + heavy clippy). Slope guard not built: KNOWN_GAPS row.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`

## Size
Under ~80k tokens. Stop with a handoff note rather than compacting.

## Result (2026-09-14, builder-opus)

**Verdict: no. The 300 MB budget does not survive a 1M-commit branch.** Measured peak RSS 881–949 MiB, 3× over, in every variant. Not tuned, budget unchanged. Finding for planner-fable below.

**Command:** `scripts/memory-curve.sh [sizes…]` (builds fixtures via the new heavy-only `graft-fixtures` example `synthetic`, runs release `examples/budgets` 3× per cell, one process at a time). Raw: `target/46/curve.txt`. Attribution tool: `prototypes/heap-profile-27b` (counting allocator + self-`SIGSTOP`), `target/46/prof/stop.sh`, `owners.py`; raw vmmap/malloc_history in `target/46/prof/`.

### Curve (release, M1 fixture shape: trunk runs of 10, 3-commit topics merged, every 32nd open)

Peak RSS (`getrusage`, the gate) always peaks in the **`G` phase** (full background load + layout to the bottom); `g` adds nothing. Ranges over 3 runs.

| Commits | Variant | RSS at first paint | **Peak RSS** | Footprint | Live heap after G / heap peak | First paint | G |
|---|---|---|---|---|---|---|---|
| 10k | small, no cg | 7.8 | **24.0–25.0** | 17.8–18.8 | — | 4–11 ms | 39–60 ms |
| 10k | small, cg | 8.3 | **24.2–24.3** | 17.5–17.6 | — | 4 ms | 31 ms |
| 10k | realistic, no cg | 8.5 | **36.4–36.6** | 29.6–29.8 | — | 4–5 ms | 43–45 ms |
| 100k | small, no cg | 22.3 | **117.3–117.8** | 85.6–86.0 | 43 / 62 | 27–83 ms | 0.50–0.54 s |
| 100k | small, cg | 27.2 | **104.1–104.3** | 67.7–68.0 | 51 / 51 | 26–85 ms | 0.33–0.37 s |
| 100k | realistic, no cg | 23.4 | **125.9–128.8** | 88.0–90.9 | — | 27–73 ms | 0.56–0.62 s |
| 1M | small, no cg | 162 | **881.3–882.4** | 592–593 | 223 / 481 | 297–360 ms | 5.8–6.0 s |
| 1M | small, cg | 211 | **932.9–934.6** | 590–592 | 370 / 419 | 292–346 ms | 4.0 s |
| 1M | realistic, no cg | 170 | **947.1–949.2** | 597–599 | — | 308–383 ms | 6.6 s |

MiB unless marked. Fixtures on disk: 1M small 298 MiB (built 26.5 s), cg 355 MiB (30 s), realistic 359 MiB (32 s); 100k 30/35/36 MiB. Slope 100k→1M ≈ **0.85–0.93 KiB RSS per commit**. First paint at 1M (292–383 ms) is also over 150 ms: not this ticket's budget, logged.

Linear-ish shape not measured: the generator has one shape (this one) and adding a shape is a fixture-scope change. The drivers below are per-commit, not per-lane, so a linear history would differ only in the flat rows term.

### What holds memory at 1M (measured)

Method: live heap by phase from a counting global allocator; at the heap peak and again after `g`, the process stopped itself; `vmmap --summary` split RSS into regions; `MallocStackLogging=lite` + `malloc_history -callTree -invert` gave live bytes by call site, attributed to the first non-generic graft/gix frame (`owners.py`). Stack logging shifts peaks by up to ~100 MiB RSS (784–808 vs 881), heap numbers agree (407 vs 481 heap peak no cg).

| Holder | 1M no cg | 1M cg | Per commit | Alive after G? | Scaling |
|---|---|---|---|---|---|
| **Mapped pack + commit-graph pages** (gix mmap, touched by the full walk; `vmmap` "mapped file", resident) | 275–287 MiB | 328–340 MiB | ≈ pack bytes (~300 B) | yes (clean, reclaimable; counted by `ru_maxrss`, not footprint) | per commit, ∝ repository bytes |
| **`Topological::discover` → `known` HashMap** | 267 MiB at peak (promotion; old+new table during rehash), freed after | 267 peak, **178 MiB kept** | 178–187 B | cg: yes, the stream walk lives for the view; no cg: no | per commit, power-of-two steps |
| **`EmittedOrder::note`** (`ids` Vec + `rows` HashMap) | **222.5 MiB** (one 178 MiB table + ids) | 131 MiB (98 table + 33 ids) | 137 B (cg), **233 B (no cg, unexplained 2×)** | yes | per commit, unbounded by design |
| `Included` (`walked` Oids + `shown`) | 23 peak, 33 after | 45.5 | 35–48 B | yes | per commit |
| `Views.promoted` (`topological_order` result, `Arc<[Oid]>`) | 31.5 | — | 33 B | yes (no cg) | per commit |
| `RepoNodes::node` (walk nodes in flight) | 49 at peak | — | transient | no | per commit, transient |
| `Dated::push` (`seen`) | 18 at peak | — | ~19 B | no (replaced at promotion) | per commit, transient |
| Allocator retained (`MALLOC_LARGE (empty)` + small empty) | 45 at peak → 148 after | 229+ after | — | freed-reusable, still resident | follows churn |
| gix object caches (`MemoryCappedHashmap`, clru) | 20.4 | 20.4 | — | yes | **flat** (capped) |
| Rows, lanes, checkpoints (`LayoutCursor`, `RowCache`) | ~7 | ~7 | — | yes | **flat** (8,000 resident rows) |
| Refs (`RepoHandle::references`) | 0.8 | 0.8 | — | yes | ∝ refs, not commits |

Rows are not the problem: the viewport-bound part of the design holds (flat ~7 MiB). Everything that grows is **commit order and ids** (four structures keyed or filled by 33-byte `Oid`s) plus **pages of the pack** the walk reads.

### Design finding for planner-fable

- **What grows:** (1) resident pack/commit-graph pages, ≈ repository size, ~290–340 MiB at 1M, which alone consumes the whole budget; (2) id-sized order state that PROMPT §1.2 ("memory grows with the viewport, not history") does not account for: `EmittedOrder` 137–233 B/commit, `Topological.known` 187 B/commit kept for the view's life with a commit-graph, `Included` ~40 B, `Views.promoted` 33 B; (3) walk churn retained by macOS malloc (~100–230 MiB after G).
- **Per-commit cost:** ≈ 0.85–0.93 KiB RSS; live heap after G 223 B (no cg) / 370 B (cg) per commit, plus the pages.
- **Projected vs measured at 1M:** measured 881–949 MiB. Even with zero heap growth, (1) leaves ~10 MiB of the 300.
- **Decisions this needs (not taken here):** whether the gate should count clean file-backed pages (`ru_maxrss`) or footprint (27b raised it; footprint at 1M is still 590–599 MiB, also over); whether order state can be positions (u32 row / commit-graph index) instead of `Oid` keys, one shared id table instead of four; whether `Topological` can drop `known` entries once emitted when generation numbers exist; why no-cg `EmittedOrder` holds a table twice the cg one (a rehash step or a second fill at promotion; unconfirmed, possibly a real bug, not fixed: out of scope); whether the 1M claim stands or the budget gets a size qualifier (spec change, user's call).

Gates: fmt, clippy, clippy `heavy-tests`, `cargo test` 816/816. Heavy tier itself not re-run (this ticket adds no heavy test).

## NOTES
- Not compacted. ~130k tokens used, over the 80k target: 1M runs and four profile stops.
- Generated repos kept under `target/fixtures/` (1M: 298 + 355 + 359 MiB, ~1 GiB total; 10k: 3–4 MiB each). Delete with `rm -rf target/fixtures/synthetic-1000000-*` if disk matters; the script rebuilds in ~30 s each.
