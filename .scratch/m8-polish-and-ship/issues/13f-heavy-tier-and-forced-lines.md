# Stop review fixes, part 2: the heavy tier compiles again, pool split, forced-write lines, stale docs

Status: done
Build with: **Sonnet**. The work is mechanical: call-site fixes, a move-only split, one new case in the exit line, and
doc lines given verbatim. It changes no git write and no journal or undo behaviour.
Needs: none (no other ticket's signatures).
Blocked by: **11b merged**, because both are in the graft crate (`update/`, captures) and 11b's bar hints could
conflict. It can run alongside 13e (a disjoint crate set).
**Crate set: graft only**, plus the doc files listed. Files:
- `crates/graft/tests/it/frames/{after_idle,journal_find,run}.rs`, `tests/it/ghostty_shot.rs`, `tests/it/held_key.rs`
- `crates/graft/src/jobs/{pool,forced,mod,runner}.rs`, `jobs/tests*`
- `crates/graft/src/event_loop/exit_wait.rs`
- `crates/graft/src/update/quit.rs` (+ its tests)
- `crates/graft/tests/it/quit_waits/stuck.rs` (a comment only)
- `crates/graft/MAP.md`, `docs/design/02-keymap.md`, `docs/design-notes/m8-polish-and-ship.md`

Scout: none needed (the review gives each line). Say this in the report.
Source: review m8-13cd Major 2, Major 1 (`pool.rs`), Minors 5 and 7; rulings R1 and R4 in `13e-stop-review-fixes.md`.

## Scope
**In:**
1. **Major 2, heavy tier compile.** At each heavy-only call site, use the 13b/13d shapes:
   - `jobs.shutdown(Duration::from_secs(10), EXIT_WRITE_WAIT, &())`
   - `shutdown != Shutdown::CLEAN` → `Err(…)` naming `stalled` and `forced`
   Sites: `frames/after_idle.rs:163-165`, `frames/journal_find.rs:289`, `frames/run.rs:115-117`,
   `ghostty_shot.rs:124`, `held_key.rs:419`, and any other site the heavy clippy finds.
2. **Major 1, `jobs/pool.rs` (451).** Move-only commit first: the forced-answer bookkeeping (`ForcedJob`, `Answer`,
   `force_*`, `wait_for_forced`, `Forced`) goes to `jobs/forced.rs`, re-exported so paths are unchanged. nextest
   total unchanged; `pool.rs` ≤ 400.
3. **Minor 7, a forced write that never started.** `Forced` gains `started: bool`. A queued write answered at once by
   `ForcedJob::new` has `started: false`. Its line, exactly: `<bin>: <op> did not start · nothing changed`. It
   replaces the `stopped <op> after <n> s` line for that case only. How `exit_line` receives it is the builder's
   choice; name it in NOTES.
4. **R4:** the MAP `Forced` row gains: "A forced undo jump (`jobs::undo`) never sets `Cancel::left_at`, so its line
   is always the `None` one. This is decided (13e R4): the line claims nothing false, and a specific line would need
   undo's roll-back answer carried through `Cancel`."
5. **Minor 5, stale words.** Replacement text below is verbatim.
   - `crates/graft/MAP.md:1045` and `src/jobs/mod.rs:106`: `(2.55 s)` → `(4.55 s)`.
   - `crates/graft/MAP.md:1065` and `src/jobs/runner.rs:480`: replace `(SIGTERM, SIGKILL after 2 s, reaped)` with
     `(its git stopped as graft_git::run stops a write: hook-first for a local write, the group stop for a rebase or a remote op; reaped within STOP_WITHIN)`.
   - `tests/it/quit_waits/stuck.rs:196-197`: replace the `reap` comment's "the pool signals git's pid, not its
     group" with `since 13c the pool stops git's whole process group; this reaps by pidfile only what a failing test leaves behind`.
   - `docs/design/02-keymap.md:241`: replace `(**Proposed**, M8/13b review fix; stopping git is pending the user's answer)`
     with `(**Proposed**, M8/13b review fix; a stop leaves the operation paused, nothing lost: the user's ruling 2026-10-04, field notes :91)`.
   - `docs/design-notes/m8-polish-and-ship.md:72`: replace everything from `The research flagged` up to the row's
     closing `|` with the block below.
     ```
     The research flagged "leaves the merge paused" as unverified. 13b found a merge stopped in its pre-merge-commit hook did **not** pause; 13c–13d end the hook first, so git pauses it itself (graft-ops it::stop_hooks), per the user's ruling 2026-10-04 (field notes :91) |
     ```
   - Design note :140 (**R1**): replace the whole row with the block below.
     ```
     | Stuck write, `q` `y` after 10 s | shell, with one line saying where git was left (keymap §2.2 "A stuck write"; the six lines in 13d); next launch: banner `c finish · a abort` (a merge) or `c continue · a abort` (rebase, cherry-pick, revert), set-aside edits offered. Graft ends the stuck hook, not git, so git pauses a merge, cherry-pick or revert itself; a rebase stays paused at its step (13c–13d); git stuck outside a hook: no pause, `u` puts the files back, and the exit line says which. |
     ```

**Out:**
- Hang-up and Minor 6 (13g).
- Design note :141 and keymap :242 (13g writes them once they're true).
- Running the heavy tier itself (that happens in the milestone review).

## Signatures
```rust
// graft::jobs (re-exported from jobs/forced.rs)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Forced { pub job: JobId, pub what: String, pub at: Option<StoppedAt>, pub started: bool }
```
`JobRunner::shutdown(self, grace, writes, force: &dyn Stop) -> Shutdown` is unchanged. `Shutdown::CLEAN` is
unchanged. Keep `Forced`'s existing derives if they differ from the ones above.

## Read first
- `docs/reviews/m8-13cd-stop.md`: Major 2, Major 1 (the `pool.rs` bullet), Minors 5 and 7
- `crates/graft/MAP.md` `## jobs` (:1013-1069), `event_loop` `exit_wait` rows (:948-950)
- `13d-stop-part-c.md` "Exit lines"
- Then source: `jobs/pool.rs`, `event_loop/exit_wait.rs`, `update/quit.rs` `exit_line`

## Touches (check each; say how)
- Every construction of `Forced`: the compiler, and `grep -rn "Forced {" crates`.
- `exit_line` callers (`exit_wait.rs`, the `quit_waits::signal` tests): the workspace nextest.
- The graft-ui no-literal-keys prose ratchet (`crates/graft-ui/tests/no_literal_keys/prose.rs`) if the new line is
  listed there. That's graft-ui test data. If it needs a row, report `blocked: needs graft-ui` and give the row.
- Design note journey fences: unchanged. Check that `quit-stuck-write 4` still parses (its walk test passes).
- Captures: none change (no drawn text).

## Acceptance (each tick names its test or check inline)
- [x] Heavy compile: `cargo clippy --all-targets --features heavy-tests -- -D warnings` rc 0, and
      `cargo nextest run --features heavy-tests --no-run` rc 0. Both summary lines in NOTES.
- [x] Pool split: the move commit comes first. nextest total N = N (NOTES). `pool.rs` ≤ 400 lines (`wc -l`).
- [x] Minor 7: `jobs::tests::a_forced_write_that_never_started_is_answered_not_started` (`started: false`, `at: None`),
      and `update::quit::tests::exit_line_says_a_write_that_never_started_did_not_start` (the exact words).
      A started forced write still reads its 13d line: `update::quit::tests::exit_line_says_where_git_left_it`, which
      passes unchanged.
- [x] R4: the MAP row exists. `grep -rn "left_at(" crates/graft/src` lists only `jobs/operate.rs` (NOTES).
- [x] Minor 5: every line in item 5 is replaced (`grep` for the old strings finds none; NOTES).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo clippy --all-targets --features heavy-tests -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
One context, about 45k tokens.

## NOTES
- Scout: none (ticket gives each line; the named `scouts/13b-review-fix.md` does not exist in this worktree).
- Heavy: `cargo clippy --all-targets --features heavy-tests -- -D warnings` rc 0, no output; `cargo nextest run --features heavy-tests --no-run` rc 0 ("Finished `test` profile"). Only the five listed sites failed. Heavy tests not run.
- Commit order: heavy-site fix (tests only) first, then the move-only split, then behaviour. nextest after split 5713/5713 (no test file touched by the move); after Minor 7 5715/5715 (+2). `pool.rs` 325 lines. `Live`/`Jobs` fields and `lock` became `pub(super)` so `jobs/forced.rs` holds `impl Live { force_stop, force_all_unstoppable, wait_for_forced }`.
- Minor 7 delivery: `exit_line` unchanged; new `update::quit::not_started_line(bin, what)`; `event_loop/exit_wait.rs` picks it when `Forced::started` is false (boxed write and the rest). `answer_of` removed (inlined).
- R4: `grep -rn "left_at(" crates/graft/src` → non-test only `jobs/operate.rs:353` (plus its def `types.rs:66`, two test uses in `jobs/tests.rs`).
- Minor 5: grep for `2.55 s`, `not its group`, `stopping git is pending`, `13b's review-fix walk found` finds none. Still stale, outside this ticket's list: `src/update/quit.rs:174`, `src/update/events/command.rs:72` ("SIGTERM, SIGKILL after 2 s, reaped").
- Prose ratchet: new line not in graft-ui `prose.rs`; no row needed. Journey fence walk passes in workspace nextest.
