# 12e final-perf-and-docs — the rest of 12 (from 12c)

Status: ready
Blocked by: 12d-perf-script-and-frames-split
Crates: graft (`tests/it/frames/*` scenarios, `examples/budgets`), scripts, `docs/perf/m7-12-idle-advance.txt`,
TESTING.md "Performance", graft MAP.md.
Build with: Opus (unchanged from 12).
Split by: conductor, 2026-09-26. 12c's "Remains" steps 3–5. No new decision.

## Scope
- 12c NOTES "Remains" steps 3–5: `g_after_idle`, `idle_gap_wakes` scenarios; final perf rev, heap slope before/after,
  heavy peak memory, budgets `G` on git/git — **all through scripts into the perf file; read SUMMARY lines only**;
  TESTING.md "Performance"; MAP.md (`Wake::label` check). The DESIGN §3.6 sentence is already drafted in 12c's NOTES —
  don't edit DESIGN.md; the conductor brings it to the user.
- 12's remaining Acceptance boxes (Release, heap slope, captures byte-identical, MAP.md), ticked in place.
- If the machine is loaded by other builders during a perf run, say so in NOTES with the run's timestamp; don't
  rerun into green.
