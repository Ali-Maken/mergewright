# 12c promotion-and-final-perf — Part C of 12, the final measurements, plus review 04's harness fixes

Status: ready
Blocked by: 12b-idle-advance-worker
Crates: graft (`update::history`, `tests/it/frames.rs`, `examples/budgets`), `scripts/m7-perf-key.sh`,
`scripts/heap-slope.sh`, TESTING.md "Performance", graft MAP.md.
Build with: Opus (unchanged from 12).
Split by: conductor, 2026-09-26, along the builder's seam. No new decision.

## Scope
- **12 "Decided" → Part C**, and 12's remaining Acceptance boxes: Release, heap slope, captures byte-identical, MAP.md.
  From 12's NOTES "What remains": `begin_promotion`, the three `update::history` tests, `g_after_idle` +
  `idle_gap_wakes` in `frames`, final perf rev, heap-slope before/after, heavy peak memory, `examples/budgets` `G` on
  git/git, TESTING.md "Performance", MAP.md history lines, the DESIGN §3.6 sentence **proposal** (in NOTES; the
  conductor brings it to the user — don't edit DESIGN.md).
- **Review 04 Major 1** (`docs/reviews/m7-04-slow-key.md`): `scripts/m7-perf-key.sh:37-45,90,146` reports `gate=ok`
  and exits 0 after a failed run; the p99 line prints before the other asserts (`frames.rs:342` vs `:352,:395`).
  Fix at the cause; a test or a deliberate failing run showing non-zero exit and `gate=FAIL`.
- **Review 04 Major 3:** `frames.rs` 1560 lines, `scroll` 80 lines — split (no behaviour change; `codebase-design`).
- Tick 12's boxes in 12's file, in place, each naming its test or perf line. Add ticks for the two review fixes under
  "(12c, review 04)".

## Rules
12 "Touches", "Rules / Gates". Measurement never in the builder's context: script → `docs/perf/m7-12-idle-advance.txt`,
read SUMMARY lines.
