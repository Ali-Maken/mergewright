# pre-publish: `fork_rows` walk flake. Find why the test's repository vanishes mid-test

Status: done (cause fixed)
Blocked by: none
Build with: **debugger** (`debugger`, Opus high, skill `diagnosing-bugs`). Why: an intermittent failure seen three times with no cause found. A flake gets its root cause, never a retry (rule 2).
Scout: no. Why: finding the cause is the ticket.

## Closes
- `ease/17-fork-rows-flake` (**broken**, intermittent): "`m7_walk::fork_rows::a_stop_inside_a_fork_names_it_on_the_banner` failed once: `git add -A` exit 128 at `rebase_line.rs:23:31`, tempdir vanished; passed on rerun, and in the full run on main 2026-10-21. Cause unknown. Again 2026-10-24 in a full `scripts/walks.sh` run (ticket 25), passed alone and on rerun. Source: 17 continuation NOTES 2."
- `ease/06h-fork-rows-flaky` (**unverified**): "`fork_rows` test failed once then passed; code untouched by 06h". Close it with the row above only if the cause is shown to be the same.

## Evidence (rule 7)
- Sightings: 06h (2026-10-18), 17 continuation (2026-10-21), and ticket 25's full `scripts/walks.sh` (2026-10-24). Each passed alone and on rerun.
- The failing line, `rebase_line.rs:23:31`, is in a **helper module of `m7_walk`**, not in `fork_rows`. Other walks share it.
- The test is in the walk tier (`*_walk` module, `scripts/walks.sh`, 4 threads).
- Related rows. Not in scope, but say whether they share the cause: `M8/56-flake-outside-repo`, `ease/10-flaky-undo-cancel` ("leftover lock"), `M8/flaky-http-remote-drop`, `M8/long-tmpdir-socket` (unix-socket path length under a long `$TMPDIR`), `M8/53-unexplained-flakes` (a builder ran `pkill -f cargo-nextest` at the time).

## Hypotheses (to test, not facts; rule 7 binds the reader too)
1. The `TempDir` guard is dropped, or moved into a scope that ends, while the app thread or a spawned `git` still uses the path.
2. Two tests derive the same temp path (time- or name-based, not unique across nextest's per-test processes), and one test's cleanup deletes the other's repository.
3. A fixture meant to be shared read-only is written to (`git add -A`) or cleaned by another test.
4. Something outside the test deletes it (another helper's `pkill` or `cargo clean`, the macOS tmp cleaner). Rule this out **last**, with evidence, never first.

## Rules
- **Reproduce first.** Repeat from a script writing `docs/perf/pp-02-fork-rows.txt`: runs, fails, and the first failure's stderr tail. Example runs: `m7_walk` alone at 4 threads; the single test beside a fixed load generator; the full walk tier. The script prints one SUMMARY line, and you read that line.
- **Never** a retry, a sleep, a longer timeout, `#[ignore]` or a skip. If the fix is in test code, the test still asserts the same thing.
- Then a **deterministic red test**: one that fails every time on main by forcing the race or the drop order (an injected barrier, a forced early drop, two tests given the same name source). A test that fails only by chance doesn't count.

## Stop conditions (report, don't work around)
- (a) No reproduction within the script's budget (≥300 module runs under load, or 2 h wall): report what ran and each hypothesis ruled out, with its evidence. The row stays broken (intermittent), with that note.
- (b) The cause is product code and needs a design decision: report the line.
- (c) The cause is outside the repo (another process): report the evidence. The conductor decides.

## Scope
In: the m7 walk helpers (`rebase_line.rs` and whatever creates its temp repository), the fixture-copy helpers in `mergewright-fixtures` if the cause is there, the repetition script, and the perf file. Product code only for a single-site defect proven by the red test.
Out: other flakes' fixes (named in NOTES only); nextest config; tier membership.

## Signatures
None expected. A helper whose signature changes (e.g. it now returns the `TempDir` guard to its caller) lists every caller in NOTES.

## Acceptance (tick each with its test, inline)
- [x] Reproduced: fails/runs before any fix ← `docs/perf/pp-02-fork-rows.txt` `== before fork_rows x300 beside a load generator ==`: `SUMMARY … runs=300 fails=3 tests_run=900 first_fail_run=91` (`scripts/repeat-walk.sh`); plain-git A/B `scripts/pp-02-maintenance-ab.sh`: arm on 7/2400 fail, arm off 0/2400
- [x] Cause named with file:line, and a red test that fails deterministically on main (red output quoted) ← `mergewright-fixtures::it maintenance::a_commit_through_the_fixture_runner_starts_no_background_maintenance`, 3/3 FAIL with `git.rs` at HEAD: `a fixture commit started background maintenance: [… "argv":["git","maintenance","run","--auto","--quiet","--detach"]]`. Cause in NOTES.
- [x] Fix at the cause; the red test is green; the same script shows 0 fails in at least as many runs as `before` needed to fail ← same test PASS; `== after fork_rows x300 beside a load generator ==`: `runs=300 fails=0 tests_run=900` (before first failed at run 91)
- [x] Every other user of the same helper or pattern checked (grep shown), each with a verdict ← NOTES 3
- [x] Related rows: same cause yes or no, each with its reason ← NOTES 4

## Touches (check each, say how)
- Every walk that uses the helper (grep).
- `mergewright-fixtures` temp/copy helpers, and their other callers in ops and git tests.
- `over_400.txt`: `m7_walk/rebase_line.rs` is a 662-line row. Split, don't raise.
- Walk partition: `scripts/walks.sh --check` counts unchanged.
- `published_repo`: no machine path in the perf file.

## Read first
- This ticket.
- `crates/mergewright/MAP.md` (tests/it layout, pty harness); `crates/mergewright-fixtures/MAP.md` (fixture cache, copy into temp).
- Then source: `tests/it/m7_walk/rebase_line.rs`, `tests/it/m7_walk/fork_rows*`, and the temp-repository helpers they call.

## Gates
Workspace-wide: the four CLAUDE.md gates, heavy clippy, then `scripts/walks.sh`.

## Size
~50k tokens. If the reproduction budget runs out, stop at (a). Don't widen the search to the related rows.

## NOTES (debugger, 2026-11-10)

Scout: no (ticket says finding the cause is the ticket). Picked up an interrupted debugger's WIP; reviewed it, re-ran the red test, confirmed the trigger by trace, removed nothing further (its `[DEBUG-pp02]` probes were already gone: `grep -rn DEBUG-pp02 crates scripts` empty).

1. **Cause (hypotheses 1–4 all wrong; the tempdir never vanished).** `crates/mergewright-fixtures/src/git.rs:293` `isolated()` (pre-fix: `Command::new("git")` with no `maintenance.auto`) (under `run_at`, called by `m7_walk/rebase_line.rs:22`) ran plain `git commit` with auto maintenance on. Every commit ends by starting a detached `git maintenance run --auto`. Git 2.54's auto check samples `objects/17/`; fork_rows' fixed dates give fixed object ids, and after commit p3 one lands in `17/`, so the detached child runs `git repack -d -l --cruft --write-midx` **every time**, deleting all loose objects and their fan-out folders (traced: `5b 6f … fd info pack` → `info pack` 3 s later). The fixture's next `git add -A` (`rebase_line.rs:30`, panic at `:23:31`) writes its temp object into a fan-out folder being removed: `unable to create temporary file: No such file or directory` (or `Invalid argument`). The ticket's "tempdir vanished" was that ENOENT misread; the probe at the failure saw tempdir, `.git`, `index` present, `objects = [pack, info]`. Trigger deterministic, race timing load-dependent → intermittent.
2. **Fix (cause, not symptom):** `isolated()` now passes `-c maintenance.auto=false`, so no fixture command leaves git working in the background; `-c` reaches children too. No retry, sleep or skip; the walks assert what they did. `remote/tests.rs` `hermetic_env_blocks_user_config` now accepts that one `command line:` origin (file kept at 643 lines, `over_400` ratchet unchanged). Precedent: `remote/local.rs:92-105` already turns it off per repository for the same git 2.54 behaviour (m8/30). Cost to the user: none, test-only; the product's own git (`mergewright-git/src/run.rs:466`) is untouched and keeps the user's maintenance settings.
3. **Other users.** `grep -rln 'mergewright_fixtures::git::(run_at|isolated|run)' crates` → 42 files (all walks and tests building repos through the fixture runner, incl. `m7_walk/rebase_line.rs`, `amend_into.rs`, `pr_contents.rs`, `picker/take.rs`, `m5_walk/fixture.rs`, `ease_walk/fixture.rs`, `hermetic` corpus builds): **fixed** by the same change, no signature change. `grep -rn 'Command::new("git")' crates` → ~90 other sites that bypass the runner: **not fixed, same exposure** wherever they commit then write again — notably `mergewright/tests/it/app_harness.rs:79` `git()` (plain `Shell::new("git")`), `mergewright-git/src/fixture.rs:105` `run_git`, `mergewright-ops/tests/it/support.rs:127,145`. Out of scope (ticket: other flakes named only); the cheap fix is to route them through `isolated` or add the same `-c`.
4. **Related rows.**
   - `ease/06h-fork-rows-flaky`: same test and helper, but no stderr was kept, so the cause is **not shown** to be the same. Likely; leave for the conductor (ticket: close only if shown).
   - `M8/56-flake-outside-repo`: **possibly same family, not shown.** Seen here during `before` (run 2): `outside_repo::launch_and_open_land_alike` `fatal: could not parse HEAD` at `app_harness.rs:86`, in a 27+-commit loop through `app_harness::git`, which keeps auto maintenance on. Needs its own red test.
   - `ease/10-flaky-undo-cancel` (leftover lock): **not shown.** `mergewright-ops/tests/it/undo_cancel.rs:194` uses bare git with maintenance on, and maintenance takes locks, but no evidence links them.
   - `M8/flaky-http-remote-drop`: **no** — network transport, no fixture write after a commit.
   - `M8/long-tmpdir-socket`: **no** — socket path length, deterministic per `$TMPDIR`.
   - `M8/53-unexplained-flakes`: **no evidence either way** — `pkill` of nextest kills tests, not object folders.
5. **Touches.** Walks using the helper: grep in 3. `mergewright-fixtures` other callers: covered by the fast tier (all pass). `over_400.txt`: `rebase_line.rs` not edited. `scripts/walks.sh --check`: counts unchanged (see report). `published_repo`: perf file paths redacted (`<tmp>`), `grep -n '/Users\|/var/folders' docs/perf/pp-02-fork-rows.txt` empty. Walk tier not run here (conductor: fast tier only); the 300-run `after` covers fork_rows.
6. **Gates (2026-11-10, this worktree):** fmt ok; clippy `-D warnings` ok; heavy clippy ok; `cargo nextest run --no-fail-fast` 6845 passed, 0 failed (653 s wall, machine under other load; budget 6 min); `cargo test --doc` ok; `scripts/walks.sh --check` `fast=6845 walks=463 all=7308`.
