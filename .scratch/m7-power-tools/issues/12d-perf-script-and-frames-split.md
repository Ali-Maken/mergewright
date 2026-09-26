# 12d perf-script-and-frames-split — review 04 Majors 1 and 3 (from 12c)

Status: done
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

## NOTES (builder, 2026-09-26) — done, no compaction
- Scout: none (conductor: the review gives the lines, 12c left exact steps).
- **Major 1, cause fix.** `scripts/m7-perf-key.sh` now reads every run's exit (`SUMMARY runs:`), puts
  `FAIL(<plain runs> failed)` / `MISSING(k of 3)` ahead of `OVER`/`ok`, and exits 1 on any failed run, any spread not
  `ok`, or a crashed summariser. It exits after writing the perf file, so the evidence is kept. The rev comparison now
  reads the whole verdict (`FAIL(plain2 failed)` has a space). Test hook: `M7_PERF_KEY_CANNED=<dir>` replays
  `<label>.out`/`<label>.exit` and skips cargo. Selftest `scripts/m7-perf-key-selftest.sh`: red 17/21 on the old
  logic, green 21/21.
- **p99 print order.** `check_scale` builds the `frames:` line; `verdict::judge` prints it after the rows-on-hand
  assert (release) and before the gate assert, so an over-budget p99 still prints. Debug prints it first, as before.
  The pixel-images assert already ran before `judge`.
- **Major 3.** Split as 12's tick says. The review asked for 3 parts; 3 parts left two files over 400 lines, so there
  are 6 submodules plus the tests. Only `Sink`, `Written`'s totals and `Counts` cross the sink seam; the parser stays
  private.
- **Not verified:** any real release run of the changed script or the scroll tests (out of scope; 12e's perf rev
  is the first). TESTING.md :175 and graft MAP.md :756 still describe the script and harness correctly, but they
  don't mention the exit status, the selftest or the submodules. Those files are outside this ticket; left for 12e.
- Gates: fmt ok; clippy workspace `--all-targets -D warnings` ok, and with `-p graft --features heavy-1m`; nextest
  3734/3734 (1 skipped); `test --doc` ok.
