# pre-publish: 100k Unicode frame-budget flake. Why were 0 of 34 rows on hand?

Status: ready
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
- [ ] Reproduced under fixed load: fails/runs and the configuration ← `docs/perf/pp-03-100k-unicode.txt` `== before ==`
- [ ] Questions 1–4 answered, each with file:line ← NOTES
- [ ] Before-19 vs main under the same load, both counts ← perf file `== e4beef81^1 ==` and `== main ==`
- [ ] Cause named, with a deterministic red test (or the stop condition reached and reported) ← test name
- [ ] Fix at the cause: red test green, and 0 fails in at least as many runs as `before` needed ← test + `== after ==`
- [ ] Idle wakes unchanged: `idle_gap_wakes=0` ← 19's idle test by name

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
