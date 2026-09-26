# 12c promotion-and-final-perf — Part C of 12, the final measurements, plus review 04's harness fixes

Status: partial (Part C + 04b done; see NOTES "Remains")
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

## Carried from 12b (conductor, 2026-09-26)
- **M7/04b remainder — the history phase can stay short of `Complete`.**
  - 12b fixed the `ⓘ` cause: 26/40 runs failed before, 0/120 after.
  - Still 5/120 runs time out in `ready()` (`tests/it/two_boxes_over_content.rs:119`). The same 5/120 with the idle
    advance disabled, so this predates 12b.
  - 12b's one trace: the promotion lands before the first `Window`, and the phase never reaches `Complete`
    (`crates/graft/src/history/scroll/apply.rs:41`). Unverified whether every timeout takes this path; verify it first.
- **Ruling (conductor):** reaching `Complete` when the whole history is laid out is correctness, not a design change.
  - In the race, the header must read exactly what the normal path reads once complete.
  - Every capture of the normal path stays byte-identical (12's "no visual change" rule still holds for them).
  - If the fix would change any normal-path capture, stop and report.
- **Test first:** a deterministic test that orders the promotion before the first `Window` and fails today. Then the
  20-parallel loop: 0/120, with the counts stated.
- Tick 12's open "(12b, M7/04b)" box in place when it holds.

## NOTES (builder, 2026-09-26) — PARTIAL, context compacted mid-ticket

**Compacted.** Stopped at the seam after Part C + 04b; everything below "Remains" is untouched.

### Done (committed)
- **Part C** (`history/views.rs`, new `history/serve.rs` — `serve`/`next_request`/`answer` moved out of views.rs,
  which was over size). `Views::begin_promotion` keeps a pending promoted layout; a request carrying `promote` is
  answered from the provisional layout with `promote` cleared; `idle_step` advances the pending layout one chunk per
  step (`IdleWork::Promoting`), swaps once ready and re-answers the last provisional request as a promoted window.
  A newer answered request drops a stale pending ask. `Views::promote` (synchronous) kept for `examples/budgets`.
  UI side: `History::reaim_promotion` (scroll/apply.rs) re-aims the Swapping anchor to the current selection on each
  provisional window, so a `G`/jump during the swap is where the cursor lands.
- **04b race — cause fix.** Swap only when `ready_to_swap`: layout done, or ≥ one chunk (`chunk_rows`) laid out and
  the anchor has a row. Before, a swap with only the anchor's rows (40) left the race at `Streaming` forever; the
  normal path always had a chunk. Test red → green: `a_promotion_before_the_first_window_ends_as_the_normal_path_does`
  compares (phase, rows, promotion, walk order, `order_notice`) race vs normal: both `(Complete, 200, None,
  Topological, None)`. Loop after: 0 failed / 120 (6×20, `--exact`), 0 `ⓘ`, 0 timeouts.
  **Not verified:** that every one of 12b's 5/120 base timeouts took this path — I did not trace base runs. The 0/120
  after is the evidence, not proof of a single cause.
- **Tried and rejected:** marking `Complete` on `LaidOut` when it follows the asked request (a `request` field on
  `LaidOut`). It changed 6 normal-path captures (e.g. committed `148 rows so far date order, sorting…` vs drawn
  `200 commits`): `status_under_a_box`, `m6_stash_put_back`, `radar_bookkeeping`, `remote_by_key` (×3 capture
  tests), `box_owns_the_bar`, and `idle_advance::g_after_laid_out_lands_without_laying_out`. Per the ruling, reverted.
- Gates at commit: fmt ok, clippy `--all-targets -D warnings` ok, nextest 3687/3687 (1 skipped), `test --doc` ok.
  No `docs/shots` file changed.
- Ticked in 12: "(12b, M7/04b)", "Part C". MAP.md: serve, idle_step, begin_promotion, IdleWork, WindowOrder lines.

### Touches (checked)
- `examples/budgets/no_cg.rs:166` calls `Views::promote` — kept, builds (clippy all-targets).
- `history::tests::served` now drives `idle_step` while promoting — all history tests green.
- `idle_tests` match gains the `Promoting` arm. Every capture via nextest `check_shot` — unchanged.

### Remains (exact next steps)
1. **Review 04 Major 1:** `scripts/m7-perf-key.sh` — count run exits; `SUMMARY runs: N of M exit 0` / `FAILED …`;
   spread verdict `FAIL(plainN failed)` / `MISSING(k of 3)` before `OVER`/`ok`; `exit 1` on any failed run. Selftest
   script with canned outputs (a failing canned run → non-zero, `gate=FAIL`). Write it with Write/Edit (the shell tool
   refuses long heredocs). Then move the p99 print in `frames.rs` after the other asserts.
2. **Review 04 Major 3:** split `tests/it/frames.rs` (no behaviour change).
3. `g_after_idle`, `idle_gap_wakes` scenarios in `frames`; then final perf rev, heap slope before/after, heavy
   peak-memory test, budgets `G` on git/git — all via scripts into `docs/perf/m7-12-idle-advance.txt`.
4. TESTING.md "Performance"; MAP.md: `HistoryUpdate::LaidOut`/`History::laid_out` lines already exist,
   `Wake::label` (event_loop) still to check; graft-graph `truncate` line is outside this crate set.
5. DESIGN §3.6 proposal (for the conductor): "The history worker lays history out a chunk at a time whenever no
   request is waiting, to the end of history, then parks; a promotion is laid out the same way and swaps in once
   it has a chunk and the selected commit's row."
