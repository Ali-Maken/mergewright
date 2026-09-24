# first-paint-perf: ticket 21's perf step

Status: ready
Blocked by: none.
Was: split by the conductor 2026-09-24 from ticket 21 (Remaining step 3).
Build with: **Sonnet**.
Needs: none.
Crates: none (a script, and `docs/perf/`).

## Scope
In: write `.scratch/m6-conflicts/perf-m6-21.sh`, modelled on `.scratch/m6-conflicts/perf-m6-07.sh`. It runs the release `heavy::budgets` and writes the `first_paint_us` lines and the summary to `docs/perf/m6-21.txt`. Measurement never runs in the helper's context (CLAUDE.md): run the script and read only its summary lines.

Out: any code change. A budget that fails → report it with the number; don't fix.

## Acceptance (each tick names its test, at tick time)
- [ ] `docs/perf/m6-21.txt` written by the script; the summary lines quoted in NOTES, with pass/fail against each budget — test: the release `heavy::budgets` run named there.

## NOTES
