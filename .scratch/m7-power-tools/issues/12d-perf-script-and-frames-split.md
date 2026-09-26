# 12d perf-script-and-frames-split — review 04 Majors 1 and 3 (from 12c)

Status: ready
Blocked by: 12c (merged c7cad495)
Crates: graft (`tests/it/frames.rs` only), `scripts/m7-perf-key.sh` + a selftest script.
Build with: Opus (unchanged from 12).
Split by: conductor, 2026-09-26. 12c compacted; this is its "Remains" steps 1–2, verbatim. No new decision.

## Scope
- 12c NOTES "Remains" step 1 (review 04 Major 1: the script's run-exit counting, verdicts, `exit 1`, the selftest
  with canned outputs; the p99 print moved after the other asserts) and step 2 (review 04 Major 3: split
  `tests/it/frames.rs`, no behaviour change; `codebase-design`).
- Tick 12c's two "(12c, review 04)" items in 12's Acceptance in place (add them if absent), naming the selftest and
  the frames tests.
- No perf runs in this ticket beyond the selftest's canned outputs.
