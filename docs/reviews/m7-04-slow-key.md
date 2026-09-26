# Review — M7 group 04: slow-key cause (04) and harness input thread (04b)

Reviewer, 2026-09-26. Read-only against HEAD `0e989561`. Merges reviewed: `2398bc58` (04) and `4209676f` (04b),
each as `git diff <m>^1 <m>`. Sources: tickets 04 and 04b, note R8 (`docs/design-notes/m7-power-tools.md:778-807`),
ADR-0021, `docs/perf/m7-04-slow-key.txt` (SUMMARY lines, rev 1 and rev 2, and the raw rev-2 rows over 10 ms).

**Counts: Critical 0 · Major 3 · Minor 11 · Questions 2.**

## Does the measurement support "the slow key is the harness's timer slack, not Graft"?

**For the first key after idle: yes.** Three independent lines agree:
- Rev 1: the idle keys' `queued` equals `oversleep` to within 1–2 µs in every scenario (perf `:209-215`, e.g. Pixel/50k
  `oversleep=10009 queued=10009`). The loop took each key the moment the harness woke. All of the extra time was the
  harness's wake.
- Control D, with no Graft code, shows the same ceiling: `oversleep_us` up to 10.0 ms after 600 ms (perf `:64-65`, `:90-91`,
  `:116-117`). The loop's real shape, a thread send into `recv`, hands over in 6–36 µs (`handoff_us`).
- Rev 2 takes the timed wait out of the latency. Idle latency median falls from 8.9–12.4 ms to 1.1–3.2 ms (perf `:549-555`).
  `queued` drops to 18–29 µs. Graft's own frame, `draw`, stays where it was.
- The real loop has no timed wait: `ChannelWakes::next_wake` is `recv()` (`event_loop/mod.rs:184`), and the input reader
  waits in `poll(-1)` unless it holds a partial sequence (`graft-term/src/input/mux.rs:250-253`).
  `the_ui_thread_never_waits_on_a_job` (`event_loop/tests.rs:907`) forbids `recv_timeout` in the loop.

**For M1's 17–18 ms key 37, which the user asked about: no.** See Major 2. It does not reproduce, and its cause is not named.

**What is not verified:**
- The mechanism, "OS timer coalescing" (Minor 7).
- Esc, the one real-app key that goes through a timed wait (Q2).
- Key 1's 10–23 ms (carried to 12, not re-reported). The rev-2 raw rows show it is in **every** scenario and every control,
  not only the 4 over 16 ms. DiffScroll and Open key 1 read 10.7–13.6 ms (perf rows after `:858`). Ticket 12 Part A should
  expect all 7 scenarios.

## Acceptance spot-check

- 04: all 6 ticks checked (one `[n/a]` skipped); **6 held**.
  - The file name differs (`m7-04-slow-key.txt`, not `m7-key.txt`); the tick says so.
  - The `trace_off_adds_no_work` tick holds as worded (see Minor 3).
- 04b: 3 of 3 checked; **3 held**.
  - The release `send -> recv` is 23–32 µs (perf `:578-713`).
  - The rev-2 SUMMARY lines in NOTES match perf `:549-562` exactly.
  - The mirrored lines `input.rs:39`/`:44` are correct.
  - MAP `:745` and TESTING `:175-190` are updated.
- No false tick.

## Critical

None.

## Major

### 1. The perf script loses run failures: SUMMARY says `gate=ok` on a failed run, and the script exits 0
`scripts/m7-perf-key.sh:37-45` records each run's exit status only in the `## runs` block. The spread verdict at `:90`
reads only the `frames:` p99 lines. The script ends with `exit 0` (`:146`) or falls through (`:196`) whatever the runs did.
- `check_scale` prints the p99 line (`frames.rs:342-348`) **before** the other release assertions: pixel images drawn
  (`:352`) and rows on hand (`judge`, `:395-400`).
- A run that fails either one still yields `SUMMARY spread … gate(p99<=16ms)=ok`.
- A run that panics before its line (e.g. `:321-325`, "every key got a frame") just drops out of the spread. That scenario
  then has 2 cells instead of 3, and its verdict still says `ok`.

**Why it matters:** CLAUDE.md Gates says a helper reads only the summary lines of a measurement. This is rule 3, an
error swallowed into a green-looking line, in the instrument ticket 12 and every later review will read.

**Fix:**
- Emit a `SUMMARY runs: N of 6 exit 0` line; any non-zero run makes it `FAILED` and names the run.
- Mark any spread with fewer than 3 cells as `MISSING`.
- Exit non-zero when any run failed.

### 2. "The slow key was timer slack" answers the idle keys, not the user's 17–18 ms key
The user asked for "**finding the cause** of the unexplained 17–18 ms release key … don't push it a fifth time" (ticket
04 `:9-11`, PROGRESS `:102`). That key is M1's key 37. ADR-0021 `:35-38` says of it: "the first key that sends a row
request … **It is not the first key after idle**." 04's own finding for it is "does not reproduce … asked=0" (perf `:22`,
NOTES `:109`). The idle-wake keys that 04 did explain are a different symptom.
- The ticket allowed treating them as "the current form of the same symptom" (`:35-36`), but ADR-0021 contradicts that.
- The log (`docs/log/m7.md:32`: "Cause measured outside Graft … Key 37 gone") and this review's brief both carry the merged
  version.

A likely account exists but was not measured. The old harness counted the 16.7 ms tick's slack too, up to 8.35 ms
(`tick_oversleep_us`, perf `:64`). The M1 harness paced the same way: `recv_timeout(wait)` with latency from the due tick (`git show 10138177:crates/graft/tests/it/frames.rs`, `:456`, `:466-468`). Adding that to M1's three back-to-back frames would reach 17 ms.

**Why it matters:** rule 1. The user will read "cause found" for the key he named, when the true status is "gone, cause
unproven".

**Fix (wording, no code):** tell him what was found for each symptom separately.
- Idle keys: harness timer slack, measured.
- Key 37: no longer happens, because it no longer fetches rows. What it was is not proven. The tick slack is the likely
  share, and it was never measured on the M1 code.

If he wants proof, re-run the rev-1 harness on the M1 commit. Otherwise record it in KNOWN_GAPS as **unverified**, not
closed.

### 3. `frames.rs` grew 397 lines to 1560; `scroll` is 80 lines (PROMPT.md 6.2)
`crates/graft/tests/it/frames.rs` was 1163 lines before 04 and is 1560 at HEAD. The limit is about 400 per file.
`scroll` (`:498-573`) is 80 lines against about 60. KNOWN_GAPS has no row for this file's size. The oversize was already
there, but this group added a third to it.

**Why it matters:** ticket 12 edits the same file next (04b `:25-26`). Every perf ticket after it reads a 1.5k-line harness
to find the part it needs.

**Fix:** split `frames.rs` into three parts:
- `frames/paced.rs`: `Paced`, `Handed`, `Kind`, `slot`, the trace and breakdown.
- `frames/controls.rs`: `Control`, `Spinner`, control D `the_machine_alone_after_idle_and_warm`.
- the tests themselves.

In `scroll`, move building the `Run` after the pump into a function of its own.

## Minor

1. **Stale rationale for not asserting idle keys.**
   - `frames.rs:277-280` still says "most of their cost is the CPU waking up (ADR-0021)". 04 measured that most of it was
     the harness's timer slack.
   - After 04b the idle keys read median 1.1–3.2 ms, max 7.7 ms (perf `:549-555`), inside the budget.
   - Fix: correct the comment. Put asserting them to the user as a **Proposed change**, because not asserting them was
     his decision (2026-09-13, ADR-0021 `:49-54`).
2. **ADR-0021's "Measurement caveat: CPU power state" (`:29-47`) is not amended.**
   - Since M1 (the harness at `10138177`, `frames.rs:456,466-468`: `recv_timeout` pacing, latency from the due tick) every latency it cites included up to 8–10 ms of the harness's own sleep. The ADR now explains numbers that
     were mostly a harness artifact.
   - Fix: add a short "Amended (m7/04, 04b)" paragraph: latency now runs from the send; CPU power state accounts for
     about 1 ms of Graft's share after idle.
3. **`trace_off_adds_no_work` claims more than it tests.**
   - The test covers `FrameTrace::from_var` and `WriteClock`. It does not cover `Paced::mark` (`frames.rs:1387-1398`).
   - `Paced::mark` reads the clock on every phase and, on `Drawn`, locks `asked_so_far` and reads `write_spent`, traced or
     not. That cost lands inside the gate's latency: sub-microsecond, but not "no work".
   - The product side is zero-cost by construction: `W: Wakes` is static dispatch (`running.rs:96`) and the default
     `mark` is empty.
   - Fix: make `mark` a no-op when untraced, or rename the test for what it checks.
4. **Two public `Phase` enums in one crate:** `graft::event_loop::Phase` (`event_loop/mod.rs:110`) and
   `graft::history::Phase` (`history/state.rs:183`). `budget_run.rs:20` imports the history one, and `frames.rs` the
   event-loop one. Fix: rename the new one `FramePhase`.
5. **Misplaced comment.** `frames.rs:312-313` ("Poisoned only when another measuring test panicked…") now sits above
   `Control::from_env`, three lines from the `MEASURING.lock()` at `:316` that it explains. Fix: move it down.
6. **The release assertion of `a_key_sent_after_idle_is_received_within_1ms_of_its_send` runs only in the script.**
   - The standing release perf command, TESTING.md `:166` and the reviewer's, filters `frames:: heavy::budgets_hold`.
     `frame_trace::` is not in it.
   - So a milestone review's release run never asserts the 1 ms hand-over that the whole rev-2 latency definition rests on.
   - Fix: add `frame_trace::` to that command.
7. **The mechanism is asserted, not measured, and the named primitive is too narrow.**
   - 04 NOTES `:103` says "OS timer coalescing"; TESTING `:186` says "OS timer slack". The measured fact is: a timed wait
     on this machine wakes up to 10 ms late, with no Graft code running.
   - NOTES `:101` blames `recv_timeout`, but rev 2's `thread::sleep` in `KeyFeed` hits the same ceiling:
     `oversleep … after=…(10040)` at perf `:549-555`.
   - Fix: say "a timed wait (any: `recv_timeout`, `thread::sleep`) wakes ≤ 10 ms late; mechanism not isolated".
8. **Gate boundary mismatch.**
   - `frame_trace.rs:63` counts a key `>= budget` as over, while `:72` holds when `p99 <= budget`. A p99 of exactly 16 ms
     passes while each 16 ms key is listed as over.
   - PROMPT.md §6.3 says "under 16 ms".
   - Fix: `p99 < budget`, or state the rule once and use it in both places.
9. **An empty measurement passes.**
   - `Gate::of(&[])` holds (`frame_trace.rs:54,59` `unwrap_or_default()`), and the test asserts it (`:295-296`).
   - The only caller guards the count (`frames.rs:321-325`), so this is latent.
   - Fix: return `None` or not-held for no samples. "Nothing measured" is not a pass (rule 3).
10. **The ticket's phases are not all split.**
    - Ticket 04 `:29-30` asked for view build, render/rasterise and write separately. The trace has one `draw`.
    - At Pixel, `draw` is Graft's largest share: the idle median rose from 2.2 to 3.1 ms and the max stays near 7.4 ms
      (perf `:209` vs rev 2 idle Pixel/50k). One plain-run PageUp drew in 9.9 ms (`key=394`, asked=1). Under control C,
      Pixel PageUp/PageDown drew in 10–12 ms.
    - Ticket 12 is told not to regress numbers it cannot break down.
    - Fix: split `draw` into view, render and rasterise before 12's Part B.
11. **KNOWN_GAPS bookkeeping (conductor).**
    - Rows `:211` (p99 too noisy) and `:447` (gate asserts the max) are still open. 04 fixed `:447` and measured `:211`:
      p99 spread 0.1–1.0 ms over 3 runs (perf `:199-205`, `:556-562`).
    - `M6/18` (a queue-time miss, "M7 (next release perf run)") is not updated. 04's twelve runs are that run.
    - Fix: close or update the three rows.

## Questions

- **Q1. Was the user asked before 04b ran?**
  - 04 stopped with "Decision for the user (rule 2, not done)" (ticket 04 `:117-119`), as its ticket and the note require
    (ticket `:41-42`; note `:799-800`).
  - 04b's source is "Conductor's ruling (2026-09-26)" (04b `:10-11`; log `m7.md:32`).
  - 04b ships no sleep or spin and fixes the measurement at its cause, so it may be within the conductor's remit. But it
    changes where latency starts, in the measurement whose treatment ADR-0021 `:49-51` records as the user's decision.
  - If he was not told, tell him in one line.
- **Q2. Does Esc pay the timer slack in the real app?**
  - A lone Esc is delivered only after a timed `poll`: `graft-term/src/input/mux.rs:250-253`, with `SEQUENCE_TIMEOUT`
    50 ms at `:25`.
  - If timed waits wake up to 10 ms late, Esc may land at about 60 ms. 04 NOTES `:112-113`, "the shipped app does not pay
    the timer slack", holds for plain keys only.
  - This is unmeasured. It is a question, not a defect.

## Checked and fine

- Product diff is 4 `wakes.mark(..)` calls plus `Phase` (`running.rs:115,138,141,152`).
  - No `unwrap`/`expect`, no Git on the UI thread, no terminal output.
  - The four calls cost nothing for `ChannelWakes`: static dispatch and an empty default.
  - 04b has no product diff: its stat is `tests/`, `scripts/`, docs only.
- 04b's harness:
  - It is FIFO like `ChannelWakes`: `recv` to wait, `try_recv` to look (`frames.rs` `next_wake`/`looked`).
  - `KeyFeed` keeps the script's clock (`frame_trace.rs:217-225`), so falling behind still queues.
  - After the script, the harness waits with `recv_timeout(PATIENCE)`, so it cannot hang.
  - `scroll` joins the feed before reporting (`frames.rs:536-538`).
- The reason the max rose in rev 2 is confirmed from the diff. The old `next_wake` handed an overdue key out ahead of queued
  job events (the removed `if !wait.is_zero()` branch).
- `budget_run::open`/`pump_until` keep their signatures. The examples compile (the clippy runs below include
  `--all-targets`).
- Feature pipeline: R8 is a no-UI perf item. It has no journey, and prior art does not apply.

## Gates (this review, main tree at `0e989561`, load average 4.6 at start, other builders running)

- `cargo fmt --check`: ok.
- `cargo clippy --workspace --all-targets -- -D warnings`: ok. The same with `--features graft/heavy-tests`: ok.
- `cargo nextest run --workspace`: 3648 run, 3647 passed, **1 failed**, 1 skipped, 230.5 s (re-run 246.0 s).
  - The failure is `m7_walk::restart_after_done`, already filed as KNOWN_GAPS M7/17, not re-reported.
  - Peak RSS 842 MB.
- `cargo test --workspace --doc`: ok.
- Slowest 5:
  - `undo_cancel::cancel_at_every_step_boundary_is_clean` 14.6 s
  - `diff_fixture::parse_unified_matches_numstat_for_every_shape` 10.1 s
  - `conflict_ops::whole_file_shapes_resolve_like_git` 9.4 s
  - `m5_walk::every_scenario::merge_resolved_with_the_keys` 9.4 s
  - `set_aside::a_stop_before_each_step_loses_nothing` 9.1 s
- This group's fast tests: `frames_gate_asserts_p99_and_prints_max` 0.006 s, `trace_off_adds_no_work` 0.006 s,
  `a_key_sent_after_idle_is_received_within_1ms_of_its_send` 0.62 s.
- `target/`: 11 GB.
- **Not run, on the conductor's instruction:**
  - The heavy tier.
  - The release perf run. Its evidence is the builder's script output: 04 had 2 runs of 6 invocations, 04b 1 run of 6, all
    `exit 0`, every spread `gate(p99<=16ms)=ok` (perf `:35-40`, `:570-575`, `:556-562`).
  - Major 1 limits how much those `ok` lines prove on their own.
