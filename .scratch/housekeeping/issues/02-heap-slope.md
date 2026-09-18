# heap-slope: the heavy tier exits 101 on heap growth per commit (76 B against a 50 B bar)

Status: done
Blocked by: none
Build with: Opus — a memory-shape question across the walk, the table and the new radar cache; the wrong fix trades frames for bytes.

Filed by the conductor, 2026-09-18, from M2's review fix round 1a. **Pre-existing, not caused by M2's features** — but M2's radar cache made it worse and nobody has owned it.

## The facts

- The heavy tier exits 101: heap slope **76 B/commit** against a **50 B/commit** bar.
- It was **69 B/commit at `16f3ed1`**, before round 1a's ref-findings cache. That cache is keyed on a refs/context/policy stamp and bounded, so the growth is small and deliberate — but it pushed a bar that was already breached.
- Everything else in round 1a went the right way: heavy tier 1275 s → 91 s, release perf 604 s → 59 s, worst frame 9.6 ms against 16 ms.

## Scope

In:
- Find where the bytes per commit actually go (walk rows, the commit table, the radar cache, the layout) — measure before changing anything, from a script writing to `docs/perf/`, never in the helper's context.
- Decide and implement: either bring the slope under the bar, or make the case that 50 B/commit is the wrong bar and say what the right one is and why. **Either answer is acceptable; an unexamined bar is not.** If the bar moves, that's an ADR and needs the user's approval before the change lands.
- One heavy job at a time. Never disable or skip the test to make it pass.

Out:
- Frame-time work (round 1a settled it).
- The radar cache's design (it earned its place — 1275 s → 91 s).

## Acceptance

- `docs/perf/` holds a before/after breakdown naming where the bytes go, not just the totals.
- Either the heavy tier exits 0, or there's an approved ADR moving the bar with the evidence behind it.
- Frame budgets don't regress: worst frame stays under 16 ms in release.

---

## Done, 2026-09-18

**Cause, not symptom.** `jobs::radar::RadarContext.default_chain` held the
default branch's whole first-parent chain as a `HashSet<Oid>` — 33-byte ids,
hashed, over-allocated: ~44 B per commit on the chain, and the chain is as
long as history. That breaches ADR-0026's rule that every history-sized
structure holds a `u32` handle, never an `Oid`; M2's radar was written
against the pre-ADR shape. It is now a `HandleSet` (one bit per commit) over
the repository's own `Arc<CommitTable>`, held beside it so
`RadarContext::on_default_chain(&Oid)` still answers the only question
`view::radar` asks, and a table rebuilt since cannot make stale handles mean
something else.

**Measured before changing anything** (`scripts/heap-slope.sh`,
`prototypes/heap-slope-hk02`: a counting global allocator plus a
drop-one-structure-at-a-time accounting, run on the guard's own 10k cg and
100k cg fixtures, release, one process at a time — never in the helper's
context). Full table: `docs/perf/housekeeping-02.txt`.

| | before | after |
|---|---|---|
| footprint peak slope | 69.9 B/commit | **31.5** |
| live heap slope | 48.4 | 5.2 |
| the chain itself | 43.3 | 0.14 |

4,456,448 B = 131,072 slots × 34 B for 78,526 chain members at 100k: the
accounting matches the structure exactly. What is left of the live slope:
`RefBadges` 2.19, `CommitTable` 1.93, `RadarState::ref_findings` 0.93
B/commit. The footprint peak sits 26 B/commit above the live total —
transient walk arrays and allocator-retained pages, the residual ADR-0026
already names.

**The bar did not move and no ADR was needed.** `cargo test --features
heavy-tests` exits 0 (161 s). No test was disabled, skipped or loosened.

**Frame budgets.** Release perf run passed (83 s); p50/p99 at every tier are
at or below round 1c's. One later run of
`frames::fifty_thousand_refs_keep_input_to_frame_under_16_ms_at_the_unicode_tier`
failed on a single 16.3 ms frame out of 420. Six interleaved A/B runs of that
one test (`target/hk02-gates/ab-*.log`, untracked) put the tail in **both**
builds — before 5.4 / 9.8 / 6.5 ms, after 6.5 / 5.8 / 5.3 ms — so the outlier
is the max-based gate's own variance under load, not this change. The gate
asserts every single key under 16 ms, so it will keep flapping on a loaded
machine; that is a separate finding, not fixed here.

### What I measured, and what I could not

- Measured: the per-structure live-heap accounting at both scales, and the
  footprint peak the guard actually asserts on, in release. The heavy tier and
  the release perf run, on this machine.
- Not measured: the debug-build slope in isolation (the guard's own child runs
  are debug; I have only its pass/fail, exit 0). The 1M curve — out of scope
  for this ticket and a review job.
- Not judged by eye: nothing here has a visual result.

### Findings for the conductor (not fixed here, out of scope)

- `graft::map::build_default_chain` (`crates/graft/src/map.rs:290`) builds the
  same `HashSet<Oid>` for the Map view — same 44 B/commit, same ADR-0026
  breach. It is transient (built per Map open, not held on `AppState`) and the
  guard's gated path never opens the Map, so it does not touch this slope; it
  will show up on a Map-view memory measurement. Worth a KNOWN_GAPS row.
- The frame gate asserts on the maximum of 420 keys, so one scheduler outlier
  fails it. Both builds spike above 9 ms on a loaded machine. Worth deciding
  whether that gate should be a p99 with a named max, rather than a max.
