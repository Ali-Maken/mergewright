# pre-publish: fast tier back under 6 min. Find what grew since ticket 24, fix its cause

Status: stopped at a stop condition (A/A spread > delta); see NOTES
Blocked by: none
Build with: Sonnet (`builder-sonnet`, model sonnet); fails twice → `builder-opus`. Why: a measuring script, then fixes of known classes in test code. A product-code cause is reported, not fixed here. Nothing structural, no interface. **Run solo**: no other cargo-running helper, because its numbers are wall times.
Scout: no. Why: the place to change isn't known until the growth table names it. Source is read only where the table points.

## Closes
- `ease/28-fast-tier-over-budget` (**broken**, measured once): "`cargo nextest run` on main after 28A: 381.5 s with no builder running, over the 6 min budget (PROMPT.md 7.1). 357 s after 26A, 341 s after 24 — rising with tests. One clean measurement."

## Need
Every builder and every merge gate runs `cargo nextest run` on his laptop. Budget: 6 min (PROMPT.md 7.1, user 2026-10-24). Gate figures on main (conductor log): 341.1 s after 24 (6774 tests); 357.4 s after 26A (6789); 381.5 s after 28A, alone (6815); 368.7 s after 29 (6827); 367.5 s after 30b, alone (6844). The 70 added tests should cost about 3.5 s (~0.2 s each over 4 threads), but the measured rise is about 26 s. Either an existing test got slower or a slow test was added. Which one is not yet known.

## Evidence (rule 7)
- `scripts/23-fast-tier.sh <label>` appends per-binary totals, the slowest 20 and a SUMMARY line to `docs/perf/23-fast-tier.txt` (ease-pass/23, D1). It keeps only the slowest 20, not every test's time.
- Ticket 24 (`.scratch/ease-pass/issues/24-*.md`, NOTES): hints 82–83 → 29–33 µs/call; fast tier 355.2/357.4 s; its perf file is named there.
- Load inflates per-test time: median 0.5–0.65 s under load vs 0.19 s alone (23 NOTES), with fseventsd churn seen. So a single run can't tell growth from load. Hence D1.
- Not verified: that 341.1 s and 381.5 s ran at the same load. Only 381.5 s is logged "alone".

## Decisions
**D1. A/B on one machine, interleaved.**
- Old = main at ease-pass/24's merge (find it with `git log --oneline main`; the gate after it measured 341.1 s). New = current main.
- Build each in its own worktree with its own `CARGO_TARGET_DIR`, **outside** any tree that `published_repo` scans. 23 failed that way.
- Warm `cargo nextest run --no-run` on both, then run A B A B.
- New script `scripts/pp-fast-tier-ab.sh <old-rev> <new-rev>`. It calls `23-fast-tier.sh` per run, keeps each run's junit file under the target dir (not in the repo), and writes `docs/perf/pp-01-fast-tier-ab.txt`:
  - the four run sections, as 23 writes them;
  - **per binary:** old mean, new mean, delta, sorted by delta;
  - **per test:** top 30 by delta among tests in both; top 30 tests present only in New, by time; count of tests gone;
  - **A/A spread:** old-vs-old and new-vs-new wall difference;
  - one line: `SUMMARY old_wall=… new_wall=… delta_s=… grown_s=… new_tests_s=… aa_spread_s=… load=…`.
- The builder reads the SUMMARY line and the growth tables only (CLAUDE.md: measurement never runs in a helper's context).

**D2. Classify every top grower and every top new test** in a NOTES table: test, old s, new s, class, line.
- **(a) Test-code cost.** 23's D2 classes: fixed wait, per-test fixture, git spawned in a loop, an over-wide lock, an exhaustive sweep in a test added since 24. Fix it here. The same asserts stand in the diff.
- **(b) Product slowdown.** An existing test is slower because product code got slower. Bisect from a script between old and new, for the top one or two only (`== bisect ==` section). Report the commit and the line. **Don't fix it here**: the conductor files the fix.
- **(c) Load noise.** The delta is within the A/A spread. Say so.

**D3. Not allowed:** moving tests between tiers, `#[ignore]`, deleting tests, changing test threads, opt-level changes (rejected in 23 and 24). A fast test over 10 s is a bug (ADR-0066): fix its cause, never move it.

**D4. Pass:** two New runs after the fixes at `run_wall_s` ≤ 360, load stated beside each, `slow_over_10s` = 0 or each one listed with its class.

## Stop conditions (report, don't work around)
- The growth is class (b): stop after the table and the bisect, and report the commit and line.
- The A/A spread is larger than the delta: stop. The machine can't see the growth; report both numbers.
- More than ~5 distinct causes: stop with the table.
- Another cargo-running helper ran during any measured run: those numbers don't count. Rerun; never average them in.

## Scope
In: `scripts/pp-fast-tier-ab.sh` (plus a small `python3 -I` helper for the junit diff if needed), `docs/perf/pp-01-fast-tier-ab.txt`, test code, helpers and fixtures for class (a) fixes, and the Fast row figure in TESTING.md's tier table.
Out: product code (`crates/*/src`, except a `#[cfg(test)]` seam a fix needs, reported); `.config/nextest.toml` filters; CLAUDE.md, PROMPT.md and ADRs (the conductor's); the heavy tier.

## Signatures (script contracts; no Rust API change)
```
scripts/pp-fast-tier-ab.sh <old-rev> <new-rev>
    # D1; appends to docs/perf/pp-01-fast-tier-ab.txt; stdout: the SUMMARY line only; exit 1 on a failed run
scripts/23-fast-tier.sh <label>     # unchanged contract; may gain an option to keep its junit file at a given path
```

## Acceptance (tick each with its test or script, inline)
- [x] A/B recorded: four run sections, both growth tables, A/A spread, SUMMARY ← `scripts/pp-fast-tier-ab.sh <old> <new>` → `docs/perf/pp-01-fast-tier-ab.txt`
- [x] Every top-30 grower and top-30 new test classified, each with its line or "cause not found" ← NOTES table vs the growth tables
- [ ] Each class (a) fix keeps every assert (read in the diff), with before/after seconds per test ← the test names + `scripts/23-fast-tier.sh pp-01-after-fix`
- [ ] Class (b), if any: the bisect names the commit, the test and its seconds before/after ← `== bisect ==` section
- [ ] Fast tier ≤ 360 s twice, load beside each, or **NOT MET** with the remaining causes listed ← `scripts/23-fast-tier.sh pp-01-after-1` and `pp-01-after-2`, SUMMARY lines quoted
- [x] The perf file names no path or account on this machine ← `published_repo::nothing_a_commit_would_carry_names_this_machine` green in the full run
- [x] No test deleted, no `#[ignore]` added: tier counts before = after, `#[ignore]` grep count before = after ← `scripts/walks.sh --check` before and after, the grep quoted

## Touches (check each, say how)
- `published_repo` scanner: perf file content, and where the A/B worktrees and target dirs live.
- Walk partition: `scripts/walks.sh --check`.
- `over_400.txt`: split, don't raise, any test file touched.
- TESTING.md Fast row (figure only).
- Other scripts that call `23-fast-tier.sh` (grep `scripts/`): still work if it gains an option.
- Ticket 24's numbers: quote them, don't edit its file.
- No product code, keymap, journal or captures: `git diff --stat crates/*/src docs/shots` empty, or each line explained.

## Read first
- This ticket.
- `.scratch/ease-pass/issues/23-fast-tier-speed.md`: Decisions D1–D3 and NOTES (the slowest-20 table and the classes).
- `.scratch/ease-pass/issues/24-*.md`: NOTES only.
- PROMPT.md §7.1 (the tier table).
- `scripts/23-fast-tier.sh`, `.config/nextest.toml`.
- Source only where the growth table points.

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo clippy --all-targets --features heavy-tests -- -D warnings`, `cargo nextest run`, `cargo test --doc`, `scripts/walks.sh --check`.

## Size
~40k tokens: one script, one table, ≤5 test-code fixes. Wall time is long (two builds, 4+2 runs of ~6 min). That's the machine, not the context.

## NOTES (builder)
Stopped by the ticket's stop condition: **A/A spread larger than the delta.** No scout (ticket said none). Did not compact. No product or test code changed.

Built `scripts/pp-fast-tier-ab.sh` + `scripts/pp-junit-diff.py`; `scripts/23-fast-tier.sh` gained env options FT_ROOT / FT_OUT / FT_JUNIT (defaults unchanged). Old = 68eec82c (24 merged), new = e4278ff2. Both checked out under `target/` (ignored), each with its own target dir, removed afterwards. Alone: no other cargo process (ps count 0).

`SUMMARY old_wall=394.4 new_wall=448.0 delta_s=53.6 grown_s=210.4 new_tests_s=1.8 aa_spread_s=135.3` (old 407.5/381.3, new 380.3/515.6). Full file: `docs/perf/pp-01-fast-tier-ab.txt`.

What it shows:
- **Growth is not visible on this machine.** new-2 ran 515.6 s (sum_test_s 2048 vs 1511 in new-1, same code): +37% on identical code. Old-2 381.3 vs new-1 380.3: same wall with 70 more tests. The A/A spread (135 s) is 2.5x the A/B delta (53.6 s).
- **Load, not code.** Load average was 8-15 before every run on a 10-core machine, with no cargo running; `dasd` and `fseventsd` at ~100% CPU each (the churn 23 saw). The load stayed high during all four runs.
- **New tests cost 1.8 s in total** (70 tests, ~0.03 s each), against the ~3.5 s the ticket expected. None is a slow one.
- **Per-binary and per-test growth is uniform** (ops::it +17%, mergewright::it +18%, top growers +10..35% across unrelated tests, e.g. conflict_ops 9.2 -> 12.4, bar_keys 10.4 -> 12.1): the shape of a skewed run, not of one grown test.

Classification (D2): every top-30 grower is **(c) load noise** (delta within the A/A spread; line: the per-test table in the perf file, no test whose delta stands out from its binary's). No (a), no (b); no bisect run (nothing to bisect). Top-30 new-only tests: all <= small, 1.8 s total, class (c)/no cost.

Not done, because the stop condition applies: class (a) fixes, `pp-01-after-1/2` runs, D4 (<= 360 s twice). Acceptance boxes left open: class (a) fixes, class (b), Fast <= 360 s twice. Fast tier floor seen in this A/B on the quietest runs is ~380 s on this machine, so <= 360 s is **NOT MET**, and the 341.1 s of ticket 24 (different load) cannot be reproduced by old code here either (old: 407.5/381.3).

Suggested next step (conductor's call): rerun A B A B when fseventsd/dasd are quiet (a reboot or Spotlight/Time Machine off), or A/B with `RUST_TEST_THREADS` left alone but measuring `sum_test_s` per run as a load-normalised figure. The known real cost remains the six-ish tests over 10 s from ticket 24's NOTES (hints sweeps, bar_keys 10-12 s): a product-side cache (`whole_bar`, `live_labels`), outside this ticket.

Gates: this ticket changed scripts and a perf file only. Ran `cargo fmt --check`, shellcheck on both scripts, `published_repo::nothing_a_commit_would_carry_names_this_machine` (1/1 green, perf file holds no path), `scripts/walks.sh --check` before (fast=6844 walks=463 all=7307). The `#[ignore]` count in crates is 22 before and after (no Rust file touched). Full clippy/nextest/doc/walks not rerun: no Rust changed.
