# heap-slope: the heavy tier exits 101 on heap growth per commit (76 B against a 50 B bar)

Status: ready
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
