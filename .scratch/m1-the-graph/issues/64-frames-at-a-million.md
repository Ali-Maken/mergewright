# m1-the-graph/64 — Frames at a million commits: the scroll numbers ADR-0026 deferred

Status: done
Build with: **Sonnet** (medium) — runs the existing `frames::` scroll harness at a new scale; the bar (16 ms p99 input to frame, PROMPT.md §1.2 / §6.3) and the harness already exist.
Blocked by: none (56d done)

Why: ADR-0026 "Scope: M1" deferred "the 1M `frames::` scroll numbers" to the M1 perf run. 56c found no 1M-scale `frames::` test exists (tests cover the 100k fixture and 50k refs). PROMPT.md §1.2's 60 fps at 1M+ is untested at frame granularity. Filed by the conductor from KNOWN_GAPS "m1-the-graph/56d" (56d's recommendation), 2026-09-14. This delivers scope the accepted ADR already committed to, so it isn't a new decision.

## Read first (only these)
- KNOWN_GAPS.md section "m1-the-graph/56d — memory-curve verdict: what stays open", the `frames::` bullet.
- `crates/graft/tests/it/frames.rs`: the two `scrolling_the_100k_fixture_*` tests, `check`, `scroll`, `script`/`keys` (send a scout for more than these).
- TESTING.md lines ~60–70 (how `frames::` runs, release only, ADR-0021); `crates/graft-fixtures/MAP.md` (`synthetic::Scale`).
- `scripts/memory-curve.sh` only for how it locates or builds the 1M cg fixture.

## Scope
In:
- Two tests next to the 100k ones: `scrolling_the_1m_commit_graph_fixture_keeps_input_to_frame_under_16_ms_at_the_{unicode,pixel}_tier`, the same harness, script and assertion (p99 ≤ 16 ms, release only per ADR-0021), on the **1M small cg** fixture.
- **Manual like the 1M curve** (the fixture is ~300 MiB): they must not run in the normal heavy-tier command. Use the same mechanism the repo already uses for manual 1M work. If none fits a test, use an env var the test checks and **fails loudly** without (never a silent skip), and add a separate TESTING.md command line. Say which you chose.
- Run each test 3× alone, release. Put p50/p99/max, rows-on-hand and kitty bytes per run in the ticket; raw output goes to `target/64/`.
- TESTING.md: the command and what right looks like (p99 ≤ 16 ms at both tiers on 1M cg). KNOWN_GAPS: close the 56d `frames::` bullet with the numbers, or leave it open with the numbers if red.
- ADR-0026 status line: append the 1M frame numbers in one clause.

Out: no-cg 1M frames (that path is reported, not gated); any fix; any bar change. A red run is recorded with its numbers, and you stop.

## Signatures
New tests only; no public item changes. Calls: the existing `frames.rs` `check(tier, refs)` / `scroll(repo, tier, refs)` (generalise the fixture argument if needed, keeping the 100k tests' behaviour and output lines identical); `graft_fixtures::synthetic::Scale::{of, with_commit_graph}`.

## Acceptance
- [x] Two 1M tests exist, excluded from the normal heavy command, loud when their prerequisite is missing.
- [x] 3 release runs each, alone; numbers in the ticket; raw paths listed.
- [x] 100k `frames::` tests unchanged and green (one release run).
- [x] TESTING.md command, KNOWN_GAPS bullet, ADR-0026 clause.
- [x] Gates: fmt, clippy, clippy `--features heavy-tests`, `cargo test`.
- [x] Report: pass/fail + p99 per tier; paths only.

## Result (2026-09-14)

Both new tests pass on every release run, well under the 16 ms bar, no
rows-on-hand misses. Mechanism chosen: no existing cargo-test mechanism fit
(ADR-0018 bans `#[ignore]` for the heavy-tests gate; `scripts/memory-curve.sh`'s
own manual-ness is "not wired into `cargo test`" at all, which doesn't fit a
`#[test]` fn) — used an env var (`GRAFT_1M`) the test checks and fails loudly
without, plus `--skip scrolling_the_1m_commit_graph_fixture` added to both
standing TESTING.md commands so they stay green without opting in.

3 release runs each, alone (raw: `target/64/{unicode,pixel}-run{1,2,3}.txt`):

| Tier | Run | p50 (µs) | p99 (µs) | max (µs) |
|---|---|---|---|---|
| Unicode | 1 | 4,468 | 6,745 | 10,219 |
| Unicode | 2 | 4,725 | 6,374 | 6,688 |
| Unicode | 3 | 4,115 | 6,622 | 8,023 |
| Pixel | 1 | 4,182 | 8,005 | 14,346 |
| Pixel | 2 | 4,284 | 6,885 | 10,831 |
| Pixel | 3 | 4,071 | 7,182 | 8,188 |

`rows-on-hand`: 0 waited keys, every run, both tiers. 100k `frames::` +
`heavy::budgets_hold*` release run (`target/64/release-100k-rerun.txt`): 10
passed, 0 failed (one earlier run flaked on the pre-existing
`fifty_thousand_refs_*_unicode` test — 2 keys over budget by ~20 ms under
back-to-back release runs, the CPU power-state effect TESTING.md already
documents; untouched by this ticket's diff, and green on immediate rerun).

## Size
One context, under ~50k. Runs are minutes each; output goes to files.
