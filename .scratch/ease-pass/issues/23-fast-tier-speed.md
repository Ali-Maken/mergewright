# fix round: fast tier under 180 s — measure, fix slow causes, move the walk tier to its own run

Status: ready
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
- [ ] Baseline, before any filter: `build_s`, `run_wall_s`, tests, load, slowest 20 by binary. ← `scripts/23-fast-tier.sh baseline` → `docs/perf/23-fast-tier.txt` `== baseline ==`
- [ ] Each of the 20 has a cause class and its line (or "cause not found") in NOTES. ← NOTES table vs the baseline block

Fix:
- [ ] Each D2 fix: the tests still pass, before/after seconds per test are in `== after-fix ==`, and the diff keeps every assert. ← the named tests + `scripts/23-fast-tier.sh after-fix`
- [ ] The opt-level change (if adopted): cold and warm `build_s + run_wall_s` before and after are recorded. ← `== opt-level ==` sections

Tier:
- [ ] Partition holds: overlap 0, gap 0, and `default + walks == --ignore-default-filter`, counts printed. ← `scripts/walks.sh --check`
- [ ] The check catches a gap. Broken once by hand (one module dropped from `walks` only): it exits 1 naming the tests. Output quoted in NOTES, then reverted. ← `scripts/walks.sh --check`
- [ ] No test deleted, no `#[ignore]` added. The `#[ignore]` count is the same before and after (grep in NOTES), and the baseline test count == default + walks (renamed tests listed). ← grep + `--check` counts
- [ ] Walks green. ← `scripts/walks.sh` SUMMARY `failed=0`, the line quoted

Pass:
- [ ] Fast tier `run_wall_s ≤ 180`, twice, `slow_over_10s=0`. Load and `build_s` stated beside each. ← `scripts/23-fast-tier.sh after-1`, `after-2`
- [ ] Captures regenerate across both tiers with no drift: run once, `git diff --stat docs/shots/` empty. ← `MWR_UPDATE_SHOTS=1 cargo nextest run --ignore-default-filter` (red by design, TESTING.md:47) then a green `cargo nextest run` + `scripts/walks.sh`

Docs:
- [ ] TESTING.md has:
  - a Walk row in the tier table, with its measured figure;
  - the Fast row's figure from `after-1`/`after-2`;
  - "The three gates" naming `scripts/walks.sh` and when a builder runs it (D5);
  - the Captures paragraph saying fast + walk, with the new regeneration command;
  - the `*_walk` convention. ← read in the diff
- [ ] Every `scripts/*` that calls nextest is listed with a verdict: it still selects what it meant to, or it gained `--profile walks` / `--ignore-default-filter`. One acceptance-walk command was run, and its selected count is > 0. ← grep `nextest` in `scripts/`, the run quoted

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
