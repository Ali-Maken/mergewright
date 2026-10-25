# pre-publish: `fork_rows` walk flake. Find why the test's repository vanishes mid-test

Status: ready
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
- [ ] Reproduced: fails/runs before any fix ← `docs/perf/pp-02-fork-rows.txt` `== before ==`, SUMMARY quoted
- [ ] Cause named with file:line, and a red test that fails deterministically on main (red output quoted) ← test name
- [ ] Fix at the cause; the red test is green; the same script shows 0 fails in at least as many runs as `before` needed to fail ← test name + `== after ==`
- [ ] Every other user of the same helper or pattern checked (grep shown), each with a verdict ← NOTES
- [ ] Related rows: same cause yes or no, each with its reason ← NOTES

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
