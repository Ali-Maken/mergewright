# slow-key-cause — find why one release key takes 17–18 ms, measured first, then fix the cause; the frame gate asserts p99

Status: ready
Blocked by: none
Crates: graft (the `frames` perf tests, the phase timing hooks in `event_loop`/`jobs` behind a test-only trace), scripts
(`scripts/m7-perf-key.sh`), `docs/perf/`. Another crate only if the named cause lives there. Then stop and report
`blocked: needs <crate>`, with the cause and the numbers.
Source: note "Pass 2a" R8.
- PROGRESS.md :102 (the user, 2026-09-17): "a ticket for background layout advance while idle … whose scope includes
  **finding the cause** of the unexplained 17–18 ms release key. Not a documented limitation; don't push it a fifth
  time."
- `docs/log/m1.md:73`: "key 37 (a j) misses every run, release 17–18 ms, cause unknown".
- `docs/perf/m6-review.txt:30-46`: `idle_wake_us` (first key after 600 ms idle) 5–15 ms, even at Unicode, where
  `idle_prefetch_us` p99 is ~1 µs.
- KNOWN_GAPS :447: "the frame gate asserts on the max of 420 keys, so one outlier fails it. Fix: assert on p99, record
  the max." KNOWN_GAPS :211: "p99 too noisy to trust: `max_us` 15772 vs 9185 on consecutive runs."

Build with: Opus. Its correctness is judged by measurement, not the diff. It decides nothing structural (the idle
advance is `idle-layout-advance`'s), so it is not Fable.
Scout: `scout` first (ADR-0035). The frames harness, `examples/budgets --phases`, and where each phase could be timed.
Needs: none. `idle-layout-advance` needs its report.

## Decided — the order is the ticket
1. **Measure first**, from a script, never in the builder's context (CLAUDE.md Gates). Write
   `scripts/m7-perf-key.sh` → `docs/perf/m7-key.txt`. The builder reads only its summary lines.
   - Runs the release `frames` tests **3 times** (`--release`, the heavy/perf features as `m4-review-perf.sh` does) and
     records each run's p50/p99/max per scenario, plus the run-to-run spread (:211).
   - With a test-only env `GRAFT_FRAME_TRACE=<file>`, it records a **phase breakdown** for every key over 8 ms and for
     every first key after idle. Phases, each in µs: input read → update → layout advance (rows added) → commit reads
     (count) → view build → render/rasterise → terminal write → any job wait.
   - **Control A:** the same key script with no idle gaps.
   - **Control B:** idle gaps with the CPU held busy by a spinning thread in the harness only. This separates machine
     wake-up (frequency, core migration, timer coalescing) from Graft's own work after idle.
   - **Control C:** idle gaps with the idle prefetch turned off (the existing knob, or a test-only one).
   - Reproduce M1's key 37 if the scenario still exists. If it doesn't, the idle-wake keys are the current form of the
     same symptom; say which was measured.
2. **Name the cause** in the perf file and NOTES: the phase that grows, by how many µs, under which control it
   vanishes.
   - "Probably" is not a cause. If the phases don't account for the time, add finer timing and re-run.
3. **Fix the cause** if it is Graft's: a red-first test that fails on the slow key and passes after.
   - If it is outside Graft (e.g. control B removes it and nothing in Graft's phases grows), write the evidence and
     **stop and ask** (rule 2). No warm-up spin, sleep or retry ships without the user's OK.
4. **The gate asserts p99 ≤ 16 ms and records the max** (:447), per scenario, in the test output line the script
   greps. The max is still printed on every run.
5. Leave the trace hooks in, behind the test-only env: zero cost when unset. A test proves it:
   `trace_off_adds_no_work`.

## Read first
- Note "Pass 2a" R8.
- `docs/perf/m6-review.txt` (summary lines :30-46 only); `scripts/m4-review-perf.sh`.
- ADR-0021 (frame budgets under `--release`).
- `crates/graft/MAP.md`: `event_loop` (`pump`, idle strips, `Wakes::waiting`), `jobs`, `history` (`History::outstanding`,
  `RowRequest`), the frames tests. `crates/graft-graph/MAP.md` `StreamingLayout::advance`.

## Scout brief (≤ 1 page)
1. Where the `frames::fifty_thousand_refs_keep_input_to_frame_under_16_ms_*` tests live, their key scripts, how
   `idle_wake_us` / `idle_prefetch_us` are measured, and the assert line (max).
2. `examples/budgets --phases`: what phases it reports, and whether it can run the frames key script.
3. What runs on the first key after idle: `pump`'s wake path, `Wakes::waiting`, any refresh or stale-check that
   triggers on a wake (fs watcher? status poll?). Name each with its line.
4. `History::outstanding`: when a key triggers a request, and whether one fires on the first `j` after idle.

## Signatures
```rust
// graft (test-only, cfg or env-gated): a FrameTrace sink recording per-key phases; name it in NOTES.
// no product API change unless the cause's fix needs one (then name it).
```

## Acceptance
- [ ] `scripts/m7-perf-key.sh` exists. `docs/perf/m7-key.txt` has 3 runs, the spread, and the phase table for the slow
      keys under all three controls.
- [ ] NOTES: **the cause**, in one line with numbers (e.g. "`<phase>` +N µs on the first key after idle; gone under
      control X"). "Could not confirm" is a stop, not a premise (rule 7).
- [ ] If the cause is Graft's: `the_first_key_after_idle_is_under_budget` (red first; release tier), plus the fix.
      Re-run the script → `docs/perf/m7-key-after.txt`, which shows the slow key under 16 ms in all 3 runs.
- [ ] If the cause is outside Graft: the evidence in the perf file, NOTES says `stopped: cause outside Graft`, and
      nothing is shipped to hide it.
- [ ] `frames_gate_asserts_p99_and_prints_max`: the gate's assert line changed, and the old max assert removed.
- [ ] `trace_off_adds_no_work`.
- [ ] MAP.md (graft): the trace hook and the gate. TESTING.md "Performance" row: how to run the script and what right
      looks like (p99 ≤ 16 ms per scenario over 3 runs; the slow key named, with its phase).

## Touches (check each, report how)
- **The fast tier:** the trace is off. Check: `cargo nextest run` time within noise of before (recorded in NOTES).
- **Every milestone review's release perf run:** now uses the p99 gate. Check: the script runs green once end to end.
- **`idle-layout-advance`:** reads this ticket's perf file. The file names what it must not regress.
- **Keymap, captures, journal:** none.

## Rules / Gates
Worktree only; `CARGO_TARGET_DIR` in the worktree. The release runs go through the script only. The four gates run
workspace-wide.

## Size
Medium (~50k). If the cause is found in another crate, stop at the report; the fix becomes its own ticket.
