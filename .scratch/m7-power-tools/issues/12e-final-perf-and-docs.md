# 12e final-perf-and-docs — the rest of 12 (from 12c)

Status: partial — heap slope blocked (needs prototypes/heap-slope-hk02); p99 vs m7-04 rev 2 not met on a slow machine (see NOTES)
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

## NOTES (builder, 2026-09-30) — PARTIAL

No compaction. No scout: the files edited were read directly (12d's split notes named each `frames/` part); the
code seam was small (one new test module, two readings).

### Done (commits 7f9c42c9 + the ticket commit)
- `frames::after_idle::g_after_idle` (new `tests/it/frames/after_idle.rs`): 100k cg, 50k refs, Unicode; settles
  until `LaidOut` for every row in the final order and startup's jobs answered, 2 s idle, `G`; latency to the frame
  whose replayed state has the cursor on the last row with the row on hand. Release asserts ≤ 16 ms and 0 gap wakes.
  `GRAFT_FRAME_NO_IDLE_ADVANCE` = the "before" build (`scripts/m7-12-no-idle-advance.patch`, applied only in a
  throwaway worktree by `scripts/m7-12-final-perf.sh`; nothing under `src` is committed with it) — records only.
- `idle_gap_wakes` in every scroll run: `Paced::unasked_wakes` / `idle_gaps` (readings.rs), `check_idle_gaps`
  (frames.rs), asserted 0 in both builds except control A (no idle pauses). Unit test
  `frames::readings::idle_gaps_count_what_nothing_asked_for_after_laid_out` — **written after the code, green on
  first run, never red.**
- `scripts/m7-perf-key.sh`: keeps the new lines, `SUMMARY idle_gap` / `SUMMARY g_after_idle`; selftest still green.
  `scripts/heap-slope.sh`: `HEAP_SLOPE_OUT`. New `scripts/m7-12-final-perf.sh` (before/after, quiet check per step,
  exit 1 on a failed step).
- Minor 5: **labelled**, not re-driven — `examples/budgets/no_cg.rs` lap reads "promote swap (synchronous path, not
  the app's)". Only prints on a repository without a commit-graph; git/git has one, so the run above never printed it.
- TESTING.md heavy tier, graft MAP.md (frames line, budgets line). 12's boxes updated in place.

### Perf (read from SUMMARY lines only)
- Rev 4 (`m7-perf-key.sh`, 17:40–17:50Z): every spread `ok`, 6 of 6 runs exit 0, script exit 0 — exit status and
  SUMMARY agree. **p99 is worse than m7-04 rev 2 beyond its spread in 5 of 7 scenarios**, and 3–5× rev 3's. Control
  D (`machine:` fixed work, no Graft code) was 2–5× slower than in rev 3, so the machine was slow; not rerun. Whether
  any of the rise is code (12b/12c/12f) is **unverified**. A fair check: rev 3's HEAD (5981cac3) and 7f9c42c9
  interleaved in one script, as 12g did.
- Final script (18:01–18:04Z, 16 of 16 steps quiet): `g_after_idle` before 51.1 ms (201 wakes: the layout's progress)
  → after 8.7 ms (1 wake: the window). budgets `G` on git/git ≈ 141–149 ms both sides (G is pressed at first paint,
  before the idle advance has run far — expected no change). Heavy `heavy::` 12/12 incl. heavy.rs:217, release
  `heavy::budgets_hold` 5/5.
- Heap slope: **blocked: needs `prototypes/heap-slope-hk02/src/main.rs`** — it no longer compiles
  (`Command::ReadRefs` takes a `JobId` now; 2 errors at :106). The script's step failed both sides; the perf
  section says `FAILED … heap_slope`. Next step: fix the prototype's `Command` calls, then rerun
  `scripts/m7-12-final-perf.sh` (it re-runs all steps, ~5 min).

### DESIGN §3.6 sentence (for the conductor; DESIGN.md not edited)
"The history worker lays history out a chunk at a time whenever no request is waiting, to the end of history, then
parks; a promotion is laid out the same way and swaps in once it has a chunk and the selected commit's row."

### Touches (checked)
- `frames/run.rs`: three fns made `pub(super)`, `Run.idle_gaps` added — only `frames` reads `Run` (grep).
- `scripts/m7-perf-key.sh`: selftest 7 cases green after the edit.
- `examples/budgets/no_cg.rs` label: grep "promote swap" → only MAP.md (updated) and the example.
- No `src` change; no key, no capture (nextest 4979/4979, `docs/shots` untouched).
