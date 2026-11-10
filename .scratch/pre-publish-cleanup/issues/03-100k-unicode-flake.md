# pre-publish: 100k Unicode frame-budget flake. Why were 0 of 34 rows on hand?

Status: stopped (part fixed, part needs a ruling)
Blocked by: none
Build with: **debugger** (`debugger`, Opus high, skill `diagnosing-bugs`). Why: an intermittent budget failure with no established cause. It may be a product regression (rows late under load) or a test measuring something the budget doesn't promise. Telling those apart is the job, and a retry is forbidden (rule 2).
Scout: no. Why: finding the cause is the ticket.

## Closes
- `ease/19-100k-unicode-flake` (**broken**, intermittent): "`frames::scrolling_the_100k_fixture_keeps_input_to_frame_under_16_ms_at_the_unicode_tier` failed once at `frames/verdict.rs:62` ("0 of 34 rows on hand": layout behind keys), load 3.5-7; builder reports two passing reruns (p99 2.5, 3.6 ms) not recorded in `docs/perf/19-fold-rules.txt` (only the failure is, :15). Cause not established. Source: 19 NOTES."

## Evidence (rule 7)
- The failure is recorded at `docs/perf/19-fold-rules.txt:15`. The passing reruns are not recorded anywhere, so treat them as unverified.
- Heavy tier: `frames::` runs under `--features heavy-tests` (23 NOTES, `m8-accept.sh` heavy filters).
- It failed inside ticket 19, the fold rules (ADR-0065, merged at `e4beef81`). 19's D1: "at rest only an input's own update/Rested/first window may ask reads, each answered once".
- ADR-0021: frame budgets are judged under `--release`. Whether the failing run was debug or release is unknown.

## Questions the debugger answers, each with its line
1. What does `verdict.rs:62` assert? Is "rows on hand" part of the 16 ms budget as DESIGN/ADR-0021 define it, or an extra condition that depends on CPU share?
2. Did 19 change when layout is asked for, so rows arrive later under load than before? Run the same load script on `e4beef81^1` (before 19) and on main.
3. Debug vs release: does it reproduce under `--release` (the budget's own mode)?
4. What does the user see in that state? A frame drawn with no rows means blank rows on screen while he scrolls. Say whether the product draws that, from its lines.

## Rules
- Reproduce under **deliberate, fixed load**: a load-generator script with set concurrency. Never "a busy machine". Repetitions are scripted and write `docs/perf/pp-03-100k-unicode.txt` (runs, fails, load, p99, rows-on-hand at failure, before/after 19). Read the SUMMARY line.
- Never a retry, a relaxed threshold, a longer wait or a skip.

## Stop conditions (report, don't work around)
- The cause is a budget-definition question (the test asserts something the budget doesn't promise): stop with the lines on both sides. Don't change the assertion; the planner rules.
- The cause is product code and the fix is not one clear site: report the line, and whether 19 introduced it.
- No reproduction within budget (≥100 runs per configuration, or 2 h wall): report what ran.

## Scope
In: the repetition and load scripts, the perf file, and either a test-side fix that keeps the assertion's meaning or a single-site product fix proven by a red test.
Out: ADR-0065's rules, idle-wake behaviour (19's `idle_gap_wakes=0` must stay 0), other frame tests' thresholds.

## Signatures
None expected.

## Acceptance (tick each with its test, inline)
- [x] Reproduced under fixed load: fails/runs and the configuration ← `frames::scrolling_the_100k_fixture_keeps_input_to_frame_under_16_ms_at_the_unicode_tier`, 30/30 fail on main with the test clamped to E-cores (`taskpolicy -c utility`, script `POLICY`); perf file block 1 (`== main ==`, the before; INDEX at the end). **Not** reproduced at default QoS under spinner load: 0/30 (x10) + 0/20 (x4). 19's own config (default QoS, other builders) is not reproduced.
- [x] Questions 1–4 answered, each with file:line ← NOTES
- [x] Before-19 vs main under the same load, both counts ← perf file blocks 1–4 (`== e4beef81^1 ==`, `== main ==`), clamp and spin x10
- [x] Cause named, with a deterministic red test (or the stop condition reached and reported) ← two parts: (a) harness defect, red→green `frames::readings::tests::rows_on_hand_are_read_at_the_frame_each_turn_drew`; (b) the failures that remain are real frames drawn without rows on a slow CPU: **stop condition** (budget-definition question, not one product site, not 19's). NOTES.
- [ ] Fix at the cause: red test green, and 0 fails in at least as many runs as `before` needed ← **not met.** Red test is green, but `== after-utility ==` is still 30/30 fail (7–28 real blank frames per run). Needs the planner's ruling (NOTES, stop).
- [x] Idle wakes unchanged: `idle_gap_wakes=0` ← `frames::scrolling_the_100k_fixture_*`, `frames::fifty_thousand_refs_*` (closed, open, diff, map) and `frames::after_idle::g_after_idle`: all `idle_gap_wakes=0` in the `== after: frames:: heavy ==` block. One E-core-clamped run (after-utility run 19) showed 1 `FoldedCounted`: a question, below.

## Touches (check each, say how)
- The pixel-tier sibling test, and any other test sharing `frames/verdict.rs`.
- ADR-0065's idle-wake tests; the heavy tier (heavy clippy, and the frames heavy run from the script).
- `docs/perf/19-fold-rules.txt`: quote it, don't edit it.
- `over_400.txt` (`frames/paced.rs` is a row): split, don't raise.
- Product layout or history code if the fix lands there: the MAP line, and the frame-budget run under `--release`.

## Read first
- This ticket.
- ADR-0065, ADR-0021.
- `crates/mergewright/MAP.md` (history, frames harness).
- `.scratch/ease-pass/issues/19-*.md`: Decisions D1–D3 and NOTES.
- Then source: `tests/it/frames/verdict.rs`, the failing test, and what it calls.

## Gates
Workspace-wide: the four CLAUDE.md gates, heavy clippy, and the frames heavy tests from the script (not in the helper's context).

## Size
~50k tokens.

## NOTES (debugger)
No compaction. Scout: no (ticket says so). Skill: `diagnosing-bugs`. Took over an interrupted debugger's WIP (instrumentation, script, the red test, 4 perf blocks); reviewed, kept the script and test, removed the `[DEBUG-pp03]` lines.

**Q1. What `verdict.rs:62` asserts.** `run.waited.is_empty()` (`frames/verdict.rs:62-67`): no frame drawn with the cursor's row or part of the viewport missing. It is **not** the 16 ms budget (that is `verdict.rs:75-82`, the p99 gate). It is a separate pace check that ADR-0021 puts under the same release-only rule (`docs/adr/0021-…:71-80`, "Applied: rows on hand": whether layout stays two screens ahead of a held `PageDown` at 60 Hz "is a question of how fast the worker runs"; `frames.rs:25-29` says the same). So yes: it depends on CPU share. And the product's own doc says it may fail on a slow CPU: `src/history/scroll/query.rs:214-216` "the next two keys are drawn from rows on hand — the answer may land a key late on a slow CPU". The test asserts never; the code says sometimes. **That is the budget-definition question.**

**Q2. Did 19 move when layout is asked?** No. 19 (`e4beef81`) does not touch `History::wanted` (`query.rs:221-251`, the ask-ahead rule); its only `query.rs` change is `:300` (a clone), and in `serve.rs` it adds two counters to the answer (`walk_counts`, O(1), ADR-0065 D3). Measured, same clamp, 30 runs each: before 19, 18.4 drawn-without-rows frames/run, every run; main 16.8/run, every run. At spin x10: 0 misses in 30/30 on both. Before-19 runs fail first on the pre-19 idle-wake bug (`frames.rs:370` runs before `:371`), so their rows count is the instrumented one (perf INDEX).

**Q3. Debug vs release.** Every run here is release. Debug never asserts it (`verdict.rs:40-58` prints it). Reproduces in release under the E-core clamp, 30/30.

**Q4. What he sees.** A frame drawn from a state with 0 of 34 rows on hand: `view/entries.rs:127` draws `history.visible(height)`, which returns only the rows inside the window on hand (`history/scroll/query.rs:172-183`; `&[]` when the viewport is past it). So that frame's graph area has no commit rows, until the window's answer lands and redraws (the next frame). By the lines, not by eye: no capture taken.

**Cause, part (a): harness defect (fixed, test side).** `Paced::waited` checked the state after **every** key, but since m7-power-tools/43 (`ff761542`, 2026-10-01) the loop applies a key and any wake already waiting in one turn and draws once (`event_loop/running/turn.rs:156`, `joins_frame` `event_loop/mod.rs:343`). A key whose window joined its turn was never drawn without rows, yet was named. The check dates from M1 (`14f54ed1`) and was never moved to the turn. Instrumented: 57% of named keys in block 1 were never drawn. Fix: `frames/readings.rs:291-324` reads rows once per drawn frame, naming that turn's newest key. Keeps the assertion's meaning (ADR-0021: "no key drawn before its viewport rows arrived"); it cannot hide a frame drawn without rows. Red before (named `key 3 … 0 of 9 rows on hand` on a joined turn), green after. Deterministic: hand-written turns over a 2,000-commit fixture, no timing.

**Cause, part (b): real frames without rows on a slow CPU (stop).** With (a) fixed, the E-core clamp still fails 30/30, 7–28 frames per run with 0 of 34 rows. Layout throughput on E-cores falls behind a held `PageDown` (2,400 rows/s) with a two-screen margin (`query.rs:229`). Pre-19. Not one product site (margin, layout chunk size, worker priority are all candidates; none tested). Stop per the ticket: **the planner rules** whether "rows on hand in release" means on any core (then product work) or on the machine at default QoS (then the clamp is out of the budget and the test stands).

**Not established.** Whether 19's single failure (key 262, load 3.5-7) was part (a) or (b): its log did not record whether that key was drawn. Default-QoS spinner load never reproduced it (50 runs, under the 100-run threshold).

**Question (no line, rule 7):** after-utility run 19 failed first on one `Produced::FoldedCounted` idle wake (main, E-core clamp). ADR-0065 says 0. Not investigated (out of scope).

**Touches.** `waited` callers: `frames/run.rs:142` only (grep), feeding `verdict.rs:62` for every tier: the pixel sibling and the 50k/pane tests all use it, and all can only lose false names, never gain one. `paced.rs` unchanged (over_400 row 419 untouched); `readings.rs` 328 lines, `readings/tests.rs` 315 (no new over_400 rows). Idle tests: `idle_gap_wakes=0` across the frames heavy run. `docs/perf/19-fold-rules.txt` quoted, not edited. No product code changed, so no MAP line. Heavy clippy clean. Frames heavy run (`== after: frames:: heavy ==`): 16 pass, 3 time-budget fails (50k pixel/Unicode p99, `g_after_idle` 21.6 ms) under load ~10 from another tree's nextest. None of them at `verdict.rs:62`. Not rerun quiet.

**Gates (workspace):** fmt ok, clippy ok, heavy clippy ok, doctests ok. Nextest: the first run failed `published_repo::nothing_a_commit_would_carry_names_this_machine` on this ticket's perf file (scratch paths in the SUMMARY lines). Fixed: the script no longer writes the logs path into the perf file, and the file was redacted with `mergewright-fixtures redact`. Rerun: 6844/6844 pass (716 s wall, over the 6 min budget, while another tree ran nextest at ~590% CPU: not a measurement).
