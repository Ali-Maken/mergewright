# 12e final-perf-and-docs — the rest of 12 (from 12c)

Status: ready
Blocked by: 12d-perf-script-and-frames-split, **12f-review-fixes** (heavy tier green and `history/views/` split before
the perf rev; planner, 2026-09-27)
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
- Not here: key 37 (M1's slow key) is `12g-key-37-cause`. Don't run 12e's perf rev while 12g is measuring.

## Carried from 12d (conductor)
- TESTING.md :175 and graft MAP.md :756 don't yet cover the script's exit status (exit 1 on FAIL/MISSING/OVER) or the six `frames/` submodules (run, paced, readings, sink, controls, verdict). Update both.
- 12d never ran the script for real in release; the first real run in this ticket is also its check — say whether exit status and SUMMARY agree.

## heavy.rs:217 — moved to 12f (planner, 2026-09-27)
- Was added here by the conductor 2026-09-26 (KNOWN_GAPS M7/heavy217). Review m7-12 Major 1 found the cause: the test
  predates 12c Part C (first window provisional by design); the fix is in the test. 12f owns it and runs first.
- Here, only: the heavy peak-memory run must show `heavy.rs:217` green — name it in that tick.
- MAP.md history lines: 12f already updates them (`views/promotion.rs`, `LaidOut`, stale `serve` line). Don't redo.
- Review m7-12 Minor 5 (`Views::promote` survives only for `examples/budgets`, so its swap line times the old
  synchronous path): you run that example here — label its promotion line "synchronous path, not the app's", or drive
  it through `begin_promotion` + `idle_step`. Say which.
