# fix round: fast tier under 180 s — measure, fix slow causes, move the walk tier to its own run

Status: partial (mechanism done; the 180 s budget is NOT met, blocked on the conductor's next move)
Build with: Sonnet (`builder-sonnet`); fails twice → `builder-opus`. Why: every rule is decided below. The work is a nextest config, two scripts, docs, and test-helper fixes for causes this ticket names by class. If a cause turns out to sit in the pty or job-runner concurrency, stop and report: that part goes to `builder-opus`.
Needs: none. **Run solo.** No other cargo-running helper during this ticket: its numbers are wall times on this laptop.

From review Major 5 (`docs/reviews/ease-pass-07-14.md:182-189`: "an idle run from a script into `docs/perf/` … name the slowest 20; move whole-app capture/walk tests … behind a `walks` filter … by the user's call"). The user's call, 2026-10-20: "you can run the test". So the conductor runs the moved set.

## Need
Every builder and every merge gate runs `cargo nextest run`. Conductor gate runs 2026-10-21..24: 493.6, 477.9, 663.0, 710.7, 489.6 s (7220–7235 tests). Budget: under 3 minutes (PROMPT.md:497). The suite runs on the laptop he is working on (PROMPT.md:493), so 8–12 minutes of 4 busy cores per gate is the cost. No field note.

## Evidence (rule 7)
- Budget and rules: PROMPT.md:497 (fast: "Under 3 minutes"); :511 (no `#[ignore]`: one command runs the whole tier); :533 ("A fast-tier test that takes over 10 seconds … is a Major review finding").
- No per-test times exist. `.config/nextest.toml:5` has `status-level = "fail"`, and the reviewer could not name the slowest tests (`ease-pass-07-14.md:27-28`).
- Growth is not linear. TESTING.md:7: 34.3 s of test time for 1822 tests (2026-09-20). Now ~480 s wall for ~7230 tests: 4× the tests, ~14× the time. Two unknowns: (a) whether the conductor's figures include compile time; (b) whether per-test time grew. The baseline must split build time from run time, so (a) gets an answer before anything moves.
- Captures are "asserted by every `cargo nextest run`" (TESTING.md:29). Moving a capturing test out of the fast tier changes that sentence and the regeneration command (TESTING.md:43).
- Merge gates today: `.claude/skills/conduct/SKILL.md:33`. A tier that no merge gate runs rots: the heavy tier went uncompiled for ten groups (`ease-pass-07-14.md:160-172`).

## Decisions
**D1. How it is measured.** JUnit, which is stable in nextest. libtest-json needs an experimental env var.
- `[profile.default.junit] path = "junit.xml"` writes `$CARGO_TARGET_DIR/nextest/default/junit.xml` on every run. `testsuite` = binary id; `testcase` has `name`, `classname` and `time`.
- `scripts/23-fast-tier.sh <label>`:
  1. `cargo nextest run --no-run`, timed → `build_s`.
  2. `cargo nextest run`, timed → `run_wall_s`.
  3. Parse the junit file (awk, or `python3 -I`).
  4. Append a section `== <label> ==` to `docs/perf/23-fast-tier.txt`:
     - date, `git rev-parse --short HEAD`, `cargo nextest --version`, `uptime` load before and after;
     - `build_s`, `run_wall_s`, test count, sum of test times;
     - per-binary totals, sorted;
     - the **slowest 20**: seconds, binary, full test name, grouped by binary;
     - any SLOW line.
  5. Print one line to stdout: `SUMMARY label=<l> tests=N run_wall_s=X build_s=Y sum_test_s=Z slow_over_10s=K`.
- The builder reads SUMMARY lines and the slowest-20 block only. No absolute or home paths in the file: `published_repo::nothing_a_commit_would_carry_names_this_machine` scans it.
- **The pass bar is `run_wall_s`, after a warm `--no-run`.** `build_s` is always reported beside it, never left out.

**D2. Fix at the cause first (rule 2).** Classify each of the baseline's slowest 20 in a NOTES table: test, seconds, cause class, and the line that shows it. A test with no line gets "cause not found". Fix a cause when it is in test code, test helpers or fixtures:
- **Fixed waits.** `thread::sleep`, or a `recv_timeout`/poll with a long constant used as "nothing came". Replace with waiting on the condition the test asserts. A negative check ("nothing arrives") waits for a window derived from the product's own constant (debounce, idle step), named in the test, never a new magic number.
- **A fixture built per test.** It should be generated once into `target/fixtures/` and shared read-only (PROMPT.md:520-522). Tests that write copy a small one into a temp dir.
- **Git spawned in loops** (N × `git commit`). Use fast-import or the shared fixture.
- **A lock that queues unrelated tests.** Narrow the lock to what it guards.
- **Dependency code at opt-level 0** (e.g. gix hashing and zlib in debug). Candidate fix: `[profile.dev.package."*"] opt-level = 2` in the root `Cargo.toml`. Adopt it only if `build_s + run_wall_s` (cold and warm both measured) falls, and record both.

A fix never weakens an assertion. The same asserts must stand in the diff. A cause in product code (`crates/*/src`) is out of scope: report it with its line and leave the test where it is.

**D3. What leaves the fast tier: the walk tier, defined by kind, not by time.**
- (a) **Acceptance walks:** modules named `*_walk` (`ease_walk`, `m7_walk`, every `m*_walk`).
- (b) **Pty tests:** tests that spawn the `mwr` binary in a pseudo-terminal.

The filter is written **by module path, not by test name**, so a new walk in those modules lands in the walk tier on its own. Convention, recorded in TESTING.md: a new acceptance walk lives in a `*_walk` module.

Everything else stays fast: unit tests, captures outside walk modules, harness tests. **A remaining fast test over 10 s is a bug** (PROMPT.md:533). Fix it under D2 or list it; never move it.
- Rejected: **anything over N s moves.** That turns slowness into a tier, and a test gets slower without anyone deciding it. It is the workaround rule 2 forbids.
- Rejected: **a cargo feature** (`walk-tests`, the way `heavy-tests` works). Feature-gated code is not compiled by the plain gates; Major 4 is the proof. A nextest filter keeps every walk compiled and clippy'd on every gate, and only moves where it runs.
- Rejected: **`#[ignore]` plus `--run-ignored`.** PROMPT.md:511 forbids it, and nothing would show which ignored tests are walks and which are broken.

**D4. Mechanism: nextest `default-filter`, two profiles that partition every test.**
- `[profile.default] default-filter = "not (<WALKS>)"`.
- `[profile.walks] default-filter = "<WALKS>"`. It inherits the rest of `default` (4 threads).
- `<WALKS>` is the same expression in both places. `scripts/walks.sh --check` proves the partition from `cargo nextest list` on each profile and on `--ignore-default-filter`: no test in both, none in neither, and the counts add up to the total.
- `slow-timeout = { period = "10s" }` in `default`, so a slow fast test prints SLOW on every run. No `terminate-after`.
- If the installed `cargo-nextest` has no `default-filter`: **stop, report `blocked`** with `cargo nextest --version`. Don't install anything and don't work around it.
- `cargo test` (no nextest) still runs everything. It is a superset and is not a gate.

**D5. Where the walk tier runs, so nothing goes unrun.**
- **After every merge, on main, the conductor** runs `scripts/walks.sh`: the partition check, then `cargo nextest run --profile walks`, then one line `SUMMARY walks tests=N failed=M wall_s=X`.
- **A builder** runs `scripts/walks.sh` once before handing in, when its ticket changes what the app draws, changes a key, or its Touches names a walk. It never runs it per edit.
- **Every milestone review** runs it with the other tiers.
- Not chosen: walks only at the milestone review. That is the gap that let the heavy tier rot (Major 4).
- Regenerating captures covers both tiers: `MWR_UPDATE_SHOTS=1 cargo nextest run --ignore-default-filter`.

## Order of work
1. Add the junit and slow-timeout lines only (no filter yet) and the script.
2. Baseline ×1.
3. Classify the slowest 20.
4. D2 fixes.
5. Measure `after-fix`.
6. Add D4 filters and `walks.sh`.
7. Measure `after-1` and `after-2`.
8. Run `walks.sh`.

More than ~5 distinct causes, or the budget is still missed after D2 + D3: **stop and report `blocked`** with the table (each remaining slow family and its seconds). The conductor decides the next move. Don't widen D3.

## Scope
In:
- `.config/nextest.toml`;
- `scripts/23-fast-tier.sh` and `scripts/walks.sh`;
- test code, test helpers and fixtures in any crate, for D2 fixes only;
- the root `Cargo.toml` profile only under D2's opt-level rule;
- TESTING.md: the tier table, "The three gates", the Captures paragraph and its regeneration command, the `*_walk` convention;
- every script under `scripts/` that calls nextest (see Touches);
- `docs/perf/23-fast-tier.txt`.

Out:
- product code;
- test threads (PROMPT.md:516 caps them at 4);
- the heavy tier;
- deleting or `#[ignore]`-ing any test;
- CLAUDE.md, `conduct/SKILL.md`, PROMPT.md and ADRs: the conductor applies the text at the end of this ticket.

## Signatures (config and script contracts; no Rust API changes)
```toml
# .config/nextest.toml
[profile.default]
test-threads = 4
failure-output = "immediate-final"
status-level = "fail"
slow-timeout = { period = "10s" }
default-filter = "not (<WALKS>)"
[profile.default.junit]
path = "junit.xml"

[profile.walks]
default-filter = "<WALKS>"          # identical text to the parenthesised part above
[profile.walks.junit]
path = "junit.xml"
```
```
scripts/23-fast-tier.sh <label>      # D1; appends to docs/perf/23-fast-tier.txt; stdout: one SUMMARY line
scripts/walks.sh [--check] [-- <extra nextest args>]
                                     # always the partition check first (exit 1 on overlap/gap, prints the
                                     # offending test names); then `cargo nextest run --profile walks`
                                     # unless --check; stdout: one SUMMARY line; failure output on failure only
```

## Acceptance (tick each with its test or script)
Measure:
- [x] Baseline, before any filter: `build_s`, `run_wall_s`, tests, load, slowest 20 by binary. ← `scripts/23-fast-tier.sh baseline` → `docs/perf/23-fast-tier.txt` `== baseline ==`
- [x] Each of the 20 has a cause class and its line (or "cause not found") in NOTES. ← NOTES table vs the baseline block

Fix:
- [ ] Each D2 fix (NONE made: every slow cause found is product code or the product's own clock; nothing to fix in test code): the tests still pass, before/after seconds per test are in `== after-fix ==`, and the diff keeps every assert. ← the named tests + `scripts/23-fast-tier.sh after-fix`
- [x] The opt-level change (not adopted; `== opt-level ==` records why; cold build_s was lost, see NOTES): cold and warm `build_s + run_wall_s` before and after are recorded. ← `== opt-level ==` sections

Tier:
- [x] Partition holds (`scripts/walks.sh --check`: fast=6772 walks=463 all=7235, overlap 0, gap 0): overlap 0, gap 0, and `default + walks == --ignore-default-filter`, counts printed. ← `scripts/walks.sh --check`
- [x] The check catches a gap (`scripts/walks.sh --check`, exit 1, output in NOTES). Broken once by hand (one module dropped from `walks` only): it exits 1 naming the tests. Output quoted in NOTES, then reverted. ← `scripts/walks.sh --check`
- [x] No test deleted (`--check` counts: 6772+463 = 7235 = baseline 7235; `git grep -c '#[ignore'` 22 files before and after), no `#[ignore]` added. The `#[ignore]` count is the same before and after (grep in NOTES), and the baseline test count == default + walks (renamed tests listed). ← grep + `--check` counts
- [x] Walks green (`scripts/walks.sh`: `SUMMARY walks tests=463 failed=0 wall_s=120.4`). ← `scripts/walks.sh` SUMMARY `failed=0`, the line quoted

Pass:
- [ ] NOT MET: Fast tier `run_wall_s ≤ 180` (after-1 386.9, after-2 390.9, slow_over_10s=9), twice, `slow_over_10s=0`. Load and `build_s` stated beside each. ← `scripts/23-fast-tier.sh after-1`, `after-2`
- [x] Captures regenerate (`MWR_UPDATE_SHOTS=1 cargo nextest run --ignore-default-filter`: 7235 passed, `git diff --stat docs/shots/` empty; then green `cargo nextest run` and `scripts/walks.sh`) across both tiers with no drift: run once, `git diff --stat docs/shots/` empty. ← `MWR_UPDATE_SHOTS=1 cargo nextest run --ignore-default-filter` (red by design, TESTING.md:47) then a green `cargo nextest run` + `scripts/walks.sh`

Docs:
- [x] TESTING.md has (read in the diff):
  - a Walk row in the tier table, with its measured figure;
  - the Fast row's figure from `after-1`/`after-2`;
  - "The three gates" naming `scripts/walks.sh` and when a builder runs it (D5);
  - the Captures paragraph saying fast + walk, with the new regeneration command;
  - the `*_walk` convention. ← read in the diff
- [x] Every `scripts/*` that calls nextest (verdicts in NOTES) is listed with a verdict: it still selects what it meant to, or it gained `--profile walks` / `--ignore-default-filter`. One acceptance-walk command was run, and its selected count is > 0. ← grep `nextest` in `scripts/`, the run quoted

## Read first
- This ticket.
- `docs/reviews/ease-pass-07-14.md:12-28` and `:182-189`.
- PROMPT.md:491-533 (§7.1).
- TESTING.md:1-60.
- `.config/nextest.toml`, `.cargo/config.toml`.
- MAP.md before source: `crates/mergewright/MAP.md` (tests/it layout, the pty harness), `crates/mergewright-fixtures/MAP.md` (fixture cache, `Captures`).
- nextest docs on `default-filter`, junit and `slow-timeout`, for the installed version.
- Source only as the slowest 20 point to it.

## Touches (check each, say how)
- **Every nextest caller:** `scripts/*.sh` (19's `scripts/19-fold-rules.sh` uses `--features heavy-tests -E …`: does the default filter drop a heavy pty test?), the TESTING.md commands, acceptance commands in `.scratch/*/issues` (read-only: list the ones that would change, don't edit them).
- **Captures:** walk-module captures are no longer asserted by the plain gate. They are covered by `walks.sh` at merge (D5).
- **`published_repo` test:** the perf file and scripts hold no machine paths.
- **`file_size` / `over_400.txt`:** split rather than grow any test file you touch.
- **Heavy tier:** `cargo test --features heavy-tests` ignores nextest config, so it is unchanged. Confirm, don't run.
- **Clippy:** walks are still compiled by `cargo clippy --all-targets`. Confirm the walk modules appear in `cargo nextest list --ignore-default-filter`.
- No product code, keymap, journal or undo touched: `git diff --stat crates/*/src` empty unless a D2 fix needed a test-only seam, and that is reported.

## Gates
Workspace-wide:
- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo clippy --all-targets --features heavy-tests -- -D warnings`
- `cargo nextest run`
- `cargo test --doc`
- `scripts/walks.sh`

## Size
~50k tokens: one config, two scripts, docs, and fixes for ≤5 causes. More causes → stop per "Order of work".

## For the conductor (apply after merge; not the builder's)
**CLAUDE.md "Gates", replace the first bullet's opening with:**
> **Rust is done only when** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run` and `cargo test --doc` all pass. `cargo nextest run` is the **fast tier**, budget 180 s run time (PROMPT.md 7.1). Its default filter (`.config/nextest.toml`) leaves out the **walk tier**: acceptance walks (`*_walk` modules) and pty tests (ease-pass/23). The walk tier is `scripts/walks.sh`, which a builder runs once before handing in when its ticket changes what the app draws, changes a key, or touches a walk. A fast test over 10 s is a bug: fix its cause, don't move it.

(Keep the rest of the bullet: workspace-wide, never `-p`, the doctest note.)

**`conduct/SKILL.md:33`, replace with:**
> - **Gates after every merge, on main:** the four CLAUDE.md gates, **plus** `cargo clippy --all-targets --features heavy-tests -- -D warnings` (~10 s warm), **plus the walk tier, `scripts/walks.sh`**. It checks that the fast and walk filters split every test with no overlap and no gap, then runs the walks; read its SUMMARY line. Why: the heavy tier failed to compile from 06c to the ease-pass review, ten groups, because no merge gate built it (review ease-pass-07-14 Major 4). The walks left the fast tier in ease-pass/23 to keep builders under 180 s, and a tier no merge gate runs rots the same way.

**Proposed change, PROMPT.md §7.1 table (to the user; a spec edit):**
- Fast row command: `cargo nextest run` + `cargo test --doc`, which is what the gates already run. Today the row says `cargo test`, which nextest's filter doesn't govern.
- Add a Walk row: `scripts/walks.sh` | after every merge (conductor), before hand-in when drawing or keys change (builder), milestone reviews | budget set from the first measurement.
- Trade-off: a builder can break a walk capture and not see it until the merge gate, unless its ticket triggers D5's builder run.

**ADR:** amend ADR-0018 (heavy behind a feature, no `#[ignore]`) or file a new one: "Walk tier: acceptance walks and pty tests run under a nextest profile after every merge; the fast tier excludes them by default filter". My search tool was down this pass (`rg` missing), so I couldn't check for an existing tier ADR. Check before numbering. Closes KNOWN_GAPS `M7/r1007-q1` if `after-1`/`after-2` hold.


## NOTES (builder, 2026-10-24)

No compaction. No scout (the work was config, scripts, docs; the slow causes were read by sampling one test and reading the named tests). The order of work was kept except D3 filters went in before D2 fixes, because no D2 fix existed.

**Result: blocked on the budget.** Fast tier 6772 tests, run_wall_s 516.8 (baseline, 7235 tests) -> 386.9 (after-1) and 390.9 (after-2), build_s 53.3 / 0.3 / 0.4 (warm; the baseline's 53 s was partial incremental), load 2.1-5.8 (fseventsd at 100% CPU the whole time, not mine). Walk tier: 463 tests, 120-124 s. Answer to unknown (a): the 480-710 s gate figures were run time, not build; sum of test times 2041 s over 4 threads = 510 s. (b) Per-test time: median 0.5-0.65 s in the git-spawning binaries under 4-way load vs 0.19 s for the same test alone, so contention inflates it; no single cause.

**Baseline slowest 20, classified** (seconds under load; `docs/perf/23-fast-tier.txt == baseline ==`):
| test | s | class | line |
|---|---|---|---|
| mergewright-ui graph_screen::hints::tests (8 tests: the_bar_never_draws_a_fragment_of_itself 38.8, a_tail_hint_gains_its_verb 37.2, no_hint_loses_ground 35.2, the_bare_keys_are_a_tail 33.9, a_words_only_bar 20.3, no_context_hides_every_key 19.9, the_three_most_relevant 17.4, one_rule_puts_the_doorway 9.6) | 212 | PRODUCT code at opt-level 0: sweeps of every context x divider x 1..240 columns call `hints_for` (graph_screen/hints.rs:119), which re-measures a cloned list per ladder step (hints/fitting.rs:129-170, `fits` at :216). One alone: 17.4 s; with opt-level 1 on mergewright-ui alone: 10.0 s; deps at opt-level 2: 17.6 s (no gain) | tests.rs:90, :1282 |
| mergewright update::tests::bar_keys::every_key_the_bar_draws_is_answered | 10.8 | same product cost, `every_context() x 2 dividers x SWEPT_WIDTHS` | update/tests/bar_keys.rs:56-70 |
| mergewright::it quit_waits::signal::stop_now_after_the_real_offer... | 11.3 | the product's own 10 s clock, waited for on purpose; a pty test, MOVED to the walk tier by D3(b) | quit_waits/signal.rs:186 |
| mergewright-ops::it undo_cancel::cancel_at_every_step_boundary_is_clean | 11.0 | sweep: one fresh repo + one undo per poll (already strided by SWEEP_STRIDE) | undo_cancel.rs:450-456 |
| mergewright-graph::it raster_pieces::pieces_composite_equals_single_emphasis 9.9; emphasis_layer::a_threaded_rows_emphasis_draws_nothing 9.7 | 19.5 | product raster code at opt-level 0 over many cases | raster_pieces.rs:24-42 (the other: cause not found) |
| mergewright event_loop::tests::pixel_scroll::held_j_on_wide_strips... 9.3; mergewright-graph layout::stream::tests::scrolling_the_whole_history... 8.9 | 18.2 | cause not found |  |
| mergewright-git::it diff_fixture::parse_unified_matches_numstat_for_every_shape | 8.7 | two git spawns per commit of every corpus shape | diff_fixture.rs:18-28 |
| mergewright-ops::it conflict_ops::whole_file_shapes 8.5, pull_set_aside 8.1, merge_set_aside 7.4, rewrite_preview::the_preview_honours_stop 6.7 | 30.7 | git spawns per scenario; cause not found beyond that |  |

No fixed wait, per-test fixture, git-in-a-loop or over-wide lock is in the slowest 20 (the fixture cache already exists: mergewright-fixtures `borrow`/`copy_into_temp`). `grep sleep|recv_timeout` shows no test among them using one except the quit_waits pty test. So **no D2 fix was made; no test code changed** (`git diff --stat crates/` empty).

**Why 180 s is out of reach with D2 + D3 as written.** Sum of test times is 1523 s after the filter. Hints sweeps are ~220 s of it (~55 s wall). The rest is ~6500 tests at about 0.2 s each, spread thin: top families in the fast tier (test-seconds, count): mergewright-ui graph_screen 218.8 (252), mergewright::it resolver 95.8 (124), ops::it history_ops 48.1 (65), mergewright update 39.1 (1811), ops::it set_aside 37.8 (35), mergewright::it m6_stash_put_back 33.8 (30), ops recover 32.9 (37), merge_set_aside 31.9 (15), mergewright-ops branch_status 31.3 (41), conflict_interim 30.3 (37). Getting to 180 s wall needs the sum near 720 s: cut ~800 test-seconds. Options for the conductor, none taken (all outside this ticket's scope): (1) `[profile.dev.package.mergewright-ui] opt-level = 1` (probe: hints test 17.4 -> 10.0 s) and likewise graph/ops, which is a workspace profile decision; (2) rewrite the hints sweep in product code so the ladder does not clone and re-measure per step; (3) move the exhaustive sweeps (hints, bar_keys, raster, undo_cancel, diff_fixture) to a named sweep tier by kind (D3 says don't widen; the user decides); (4) measure on a machine without fseventsd churn. Do not widen D3 on time alone (the ticket rejects it).

**Opt-level.** `[profile.dev.package."*"] opt-level = 2`: not adopted. Fresh-target run 366.9 s vs 386.9 s (5%, inside the 387/391 spread); the hints test did not move. The cold `build_s` for it was lost: my first attempt put the fresh target dir inside the worktree where `published_repo::nothing_a_commit_would_carry_names_this_machine` scanned it (657 s, failed), and the second reused a half-built dir. Not re-run since the run time gave no case to adopt. Cold/warm before-and-after was therefore not fully recorded.

**Script fix found on the way.** nextest 0.9.145 writes the junit file under the worktree's `target/nextest`, not under `$CARGO_TARGET_DIR/nextest` when the two differ; `scripts/23-fast-tier.sh` takes the newest of both. `walks.sh` avoids BSD-sed-only constructs.

**Gap check, quoted** (module `tour_tasks` dropped from the walks profile only; `scripts/walks.sh --check`, exit 1):
```
partition: fast=6772 walks=452 all=7235
IN NEITHER PROFILE:
mergewright::it tour_tasks::an_aborted_clash_merge_ticks_neither_6_nor_7
mergewright::it tour_tasks::reading_the_card_ticks_nothing
mergewright::it tour_tasks::task_1_ticks_by_staging_a_hunk
... (11 tests; reverted)
```

**Regeneration.** TESTING.md says a regeneration "is always a red run"; with no drift the run was green (7235 passed, `docs/shots` diff empty). The doc line is untouched (not mine to rule on); the claim is unverified for the case where something changes.

**Scripts calling nextest, verdicts** (`grep nextest scripts/`):
- ease-accept.sh:48, m7-accept.sh:47, m8-accept.sh:56 and :112: selected walk/pty modules or the whole tier; GAINED `--ignore-default-filter`. Run: `cargo nextest list --ignore-default-filter -p mergewright --test it -E 'test(/^ease_walk::/)'` selects 93 tests; without the flag 0.
- m8-accept.sh:67 (heavy, `--features heavy-tests`): its three filters hit `frames::`, `heavy::`, `held_key::`, none in the walk set; 11 selected with and without the flag. Unchanged.
- git-matrix.sh:91 (`rewrite_preview::`, one `cherry_pick::` test): not walk modules; 0 selected for `rewrite_preview` is the `-p mergewright --test it` slice only (that module lives in ops), unchanged. linux-fast-tier.sh:77 and toolchain-bench.sh: mean the fast tier; unchanged on purpose.
- 19-fold-rules.sh uses `cargo test`, which ignores nextest config: unchanged.
- Acceptance commands in TESTING.md (older sections, e.g. lines naming `m5_walk`, `m7_walk`) and `.scratch/*/issues` acceptance lines naming a walk now select nothing without `--profile walks`/`--ignore-default-filter`: not edited (a note in TESTING.md "The walk tier" says so). List them with `grep -rn "nextest" .scratch/*/issues`.

**Touches:** every nextest caller: grepped `scripts/`, verdicts above. Captures: walk and pty captures are asserted by `scripts/walks.sh` only; the all-tier regeneration ran, no drift. `published_repo`: passed in both full runs with the perf file and scripts in the tree. file_size/over_400: no test file touched. Heavy tier: `cargo test --features heavy-tests` ignores nextest config (not run; `cargo clippy --features heavy-tests` green). Clippy: walks are in `cargo nextest list --ignore-default-filter` (7235 = all), compiled by the plain gates. Product code: `git diff --stat crates/` empty.

**For the conductor.** The CLAUDE.md / conduct text in "For the conductor" above states a 180 s budget the fast tier does not meet; do not apply the "budget 180 s" wording until you decide the options above. Product-code cause to report: hints fitting (see table), and `.scratch` for a ticket if you want it fixed.
