# Stop review fixes, part 1: the stop in graft-git, the splits in graft-ops

Status: done
Build with: **Opus** (high, not xhigh). This is the code that signals git's process group during a write. Whether a
stopped hook really ends can't be judged from the diff, only from a process test. It changes no journal or undo
semantics, so it doesn't need xhigh. Not Fable: the shape stays as it is.
Needs: none (no new signatures).
Blocked by: 10e (conductor 2026-10-07: 10e is editing graft-ops refusal callers now; 13e's move-only splits go after it)
run concurrently (at most 2 helpers).
Scout: none needed. The review (`docs/reviews/m8-13cd-stop.md`) names every line. Say this in the report.
Source: review m8-13cd Major 1 (in part), Major 3, Minors 2, 4 and 8; open point `M8/13d-open-points`
(`stopped_on_conflict`). Siblings: 13f (graft: heavy tier, pool split, forced lines, docs) and 13g (graft: hang-up).

## Rulings (planner, 2026-10-07; against the user's ruling, field notes :91, and M3)
These cover all of 13e–13g. None needs the user.
- **R1 Banner words: `c finish  a abort` on a merge is right; the doc is wrong.**
  - Keymap §2.6a `c` row: on a merge, `c` opens the finish summary.
  - The shipped label is `c finish` since m6-conflicts/70 (`docs/design-notes/m6-conflicts.md:258-259`), and the
    m6 usability review asked for exactly that word.
  - A rebase, cherry-pick or revert keeps `c continue`.
  - Design note :140 is fixed in 13f. No code change. The 13d test that quotes the real banner stands.
- **R2 `stopped_on_conflict` stays on `starts_in_progress()`.**
  - A `FinishMerge` whose commit fails without a stop answers `Git`: entry `Aborted`, not in force, merge still live,
    banner still `c`/`a`. Observed: `graft-ops it::finish_merge::a_hook_refusal_leaves_the_merge_in_progress`.
  - Switching it would hold a `Pending` entry for a merge Graft never began, which 13d Q4 forbids. Nothing is lost,
    and the words are git's own.
  - A comment at the site and the MAP row say why (this ticket). The row closes as decided.
- **R3 The 80x24 capture without "from a previous run":** treated as the width ladder (ADR-0039) until shown
  otherwise. 13g adds the 120x40 relaunch capture and asserts the words are there. If they're missing at 120, that's a
  graft-ui defect: report it, don't fix it in 13g.
- **R4 A forced undo jump's exit line stays the `None` line.** It claims nothing false: "here shows where git left
  it". A more specific line would need the undo jump's roll-back answer carried through `Cancel`, and that is
  undo-semantics work. The MAP `Forced` row says so (13f).
- **R5 Hang-up:** see 13g.
- **R6** 13b's unticked merge-half box goes to the conductor (the review recommends ticking it, naming its two
  tests). It isn't a builder job.

## Scope
**In:**
1. **Move-only splits, first, one commit per crate.** Record the `cargo nextest run` total in NOTES before and after,
   and check it is unchanged. Public paths stay unchanged (re-export). `git diff -M --stat` shows moves only.
   - graft-git `run/exec.rs` (529 production lines): move the stop (`StopOrder`, `Graces`, `group_stop`,
     `hooks_first`, `Helpers`, `signal_*`) to `run/stop.rs`. Move the `stop` test module from `run/tests.rs` (567)
     to `run/tests/stop.rs`.
   - graft-ops `apply.rs` (777) and `apply/resume.rs` (543): cut them into cohesive children until each is ≤ 400
     production lines. Suggested cuts, to be confirmed by reading:
     - `stopped_on_conflict` with the put-back failure branch (`clashed_put_back`) → `apply/failed.rs`;
     - `live_entry`/`live_entry_facts` → `apply/resume/live.rs`.
     The builder picks the cut by cohesion and names it in the MAP's File layout row.
2. **Major 3:** `group_stop` sends `killpg(SIGCONT)` straight after its `SIGTERM` (`exec.rs:326`; the doc at :314 and
   the MAP already say it does).
3. **Minor 2:** once git has ended inside `group_stop` and members are still listed, run the same bounded helper pass
   `hooks_first` runs after git ends (`SIGTERM` + `SIGCONT`, then `SIGKILL` after `HELPER_TERM_GRACE`). Use one
   helper path, not a second copy. The default stop must still end within `STOP_WITHIN`. If the "default stop
   ends within 2.05 s" figure no longer holds, write the new figure in the MAP `STOP_WITHIN` row.
4. **Minor 4:** the `(Err(_) | Ok(false), Some(status))` arm (`exec.rs:413`) splits.
   - `Ok(false)`: return, as now.
   - `Err`: `killpg(SIGKILL)` to the group. Safe: git is reaped, but the group's id stays reserved while any member
     lives, and on an empty group the kill is `ESRCH`. Say this in a comment, and handle `ESRCH` explicitly.
   - Rule 3: no silent drop.
5. **Minor 8:** a `Drop` guard in the graft-git stop tests that `SIGKILL`s every pid a hook wrote to its pidfile, as
   graft-ops' `Hooked` does (`tests/it/stop_hooks.rs:138`).
6. **R2:** a comment at `stopped_on_conflict`'s `starts_in_progress()` read, plus one sentence in graft-ops MAP
   `## apply`.

**Out (the conductor files rows):**
- Minor 1 (multi-commit pick read as residual, latent).
- Minor 3 (pid reuse window).
- Minor 9 (paused revert test).
- Question 1 (torn journal line at `FORCED_WAIT`).
- Size of `tests/it/stop_hooks.rs` (720) and `graft-git run.rs` (678).
- Everything in the graft crate (13f, 13g).

## Signatures
No public item changes. `GitInvocation`, `StopOrder` visibility, `STOP_WITHIN`, `HOOKS_GRACE` and the
`apply::`/`apply::resume` paths all stay as they are. New private modules only: `run/stop.rs`, `run/tests/stop.rs`,
and the graft-ops children.

## Read first
- `docs/reviews/m8-13cd-stop.md`: "Process groups", Majors 1 and 3, Minors 2, 4 and 8
- `crates/graft-git/MAP.md` `## run`; `crates/graft-ops/MAP.md` `## apply`, the File layout rows (:393-406)
- `.scratch/m8-polish-and-ship/issues/13c-stop-leaves-merge-paused.md` "Decisions" (decision 2: why a group)
- Then source: `run/exec.rs`, `run/tests.rs`, `apply.rs`, `apply/resume.rs`

## Touches (check each; say how)
- graft-ops tests that read `stops_hooks_first` and `it::stop_hooks`: run with the workspace nextest.
- graft's `it::quit_waits` (the real stop walks): they go through `run`, so the same workspace run covers them. No
  graft edits.
- graft-git MAP `run` row: SIGCONT is now true; add the helper pass after git ends. File layout row for `stop.rs`.
- graft-ops MAP File layout (m8/13e) row and the `## apply` file list (`apply.rs holds …`).
- No captures, keymap, journal or undo change. Show it: `grep -rn "group_stop\|hooks_first" crates --include=*.rs`
  lists only graft-git.

## Acceptance (each tick names its test or check inline)
- [x] Splits (`git diff -M --stat` of 21f2db0c and 30e75ce2 shows moves only; numbers in NOTES): the move commits come first. nextest total N before = N after (both numbers in NOTES). `exec.rs`,
      `apply.rs` and `resume.rs` are each ≤ 400 production lines (`wc -l` up to `#[cfg(test)]`, in NOTES).
- [x] Major 3, SIGSTOP (passes; **not seen red**, see NOTES):
      `run::tests::stop::the_group_stop_continues_and_ends_a_hook_stopped_by_sigstop`.
      - Hook `kill -STOP $$` under `StopOrder::Group`.
      - Before the stop, assert the hook's `ps -o stat=` starts with `T`.
      - After it: the hook's pid is gone (`kill -0` → ESRCH), the group is empty, and the stop ends within
        `STOP_WITHIN`.
      - Seen red without the SIGCONT line (NOTES).
- [x] Major 3, SIGTTIN (passes; the hook does reach `T`, the test waits for it; not seen red, see NOTES): `…::the_group_stop_continues_and_ends_a_hook_stopped_by_sigttin`. Same asserts, with
      `kill -TTIN $$`. If the hook never reaches `T` (the kernel discards TTIN in an orphaned group), say so in NOTES
      with the `ps` line. Don't fake it.
- [x] Minor 2 (seen red before the fix: `gone` failed): `…::the_group_stop_ends_a_hook_that_ignores_term`. Hook `trap '' TERM; sleep 600`, default stop: the
      hook is gone, the group is empty, the stop ends within `STOP_WITHIN`. Seen red first.
- [x] Minor 4 (seen red before the fix: `gone(helper)` failed): `…::hooks_first_ends_helpers_when_members_cannot_be_read_after_git_ended`. Inject `Err` on the member
      read after git's exit (the existing injection seam of
      `hooks_first_falls_back_to_the_group_stop_when_members_cannot_be_listed`): the helper is gone.
- [x] Minor 8 (`the_hook_guard_kills_its_pid_on_drop` passes; grep in NOTES): `…::the_hook_guard_kills_its_pid_on_drop`. Spawn `sleep 600`, write its pid, drop the guard: the pid
      is gone. Every stop test that runs `sleep 600` holds a guard (grep in NOTES).
- [x] R2 (passed unchanged in the final workspace run; comment `apply/close.rs` above `starts_in_progress()`, MAP `## apply` sentence): `graft-ops it::finish_merge::a_hook_refusal_leaves_the_merge_in_progress` passes unchanged. The comment
      and the MAP sentence exist (grep in NOTES).
- [x] (`ps -A | grep -c "[s]leep 600"` read 0 after the final run) After all of this, `ps -A | grep -c "sleep 600"` reads 0 (NOTES).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo clippy --all-targets --features heavy-tests -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

**Known:** heavy clippy fails in graft tests until 13f lands (review Major 2). If 13f hasn't merged, record that
the failures are only those 5 graft files, and that graft-git and graft-ops are clean.

## Size
One context, about 70k tokens. Most of it is reading the four files. If it runs long, split at the crate seam:
graft-git items 1–5, then graft-ops items 1 and 6.

## NOTES (builder, 2026-10-07)
- Scout: none, as the ticket says; the review named every line. Scouts 13b-review-fix / 13c-app cover the graft crate; not used.
- Not compacted.
- **Splits.** nextest before: 5708 passed / 10 skipped (main 923275f8). After both move commits: 5708 / 10, unchanged. Final, with the 5 new tests: 5713 / 10.
  Production lines (up to `#[cfg(test)]`): `exec.rs` 241, `stop.rs` 381, `apply.rs` 343, `apply/close.rs` 362, `apply/guard.rs` 113, `apply/resume.rs` 307, `resume/live.rs` 121, `resume/close.rs` 142. Test files: `run/tests.rs` 257, `run/tests/stop.rs` 247, `run/tests/stop/fixture.rs` 246 (fixture split out in the build commit to keep the test file under 400; test paths unchanged).
- **Major 3 not seen red.** STOP and TTIN tests both passed with neither the SIGCONT line nor the helper pass in place (the same run where Minor 2 and 4 were red). Read, not observed: once git dies, the group is orphaned with a stopped member, and the kernel sends it SIGHUP + SIGCONT, which ends the `sh` hook. So on macOS the SIGCONT is a backstop, not the only cure. I found no test setup where the group stays un-orphaned after git dies. TTIN: the hook does reach `T`, because git's group is not orphaned while git lives.
- **Minor 2 / 4 design.** One helper path, `stop.rs` `after_git_ended`, used by both stops once git is reaped. It reads at once, then every 250 ms; each helper gets TERM + CONT, then SIGKILL 1 s later; anything still listed at `until` gets SIGKILL. A failed read → `kill_reaped_group` (`killpg(SIGKILL)`, `ESRCH` matched, `EPERM` commented). `group_stop` runs it inside its own `term` grace, so the 2.05 s default bound and `STOP_WITHIN` (4.05 s) are unchanged. Measured: a TERM-ignoring hook under the default stop took 1.33 s; STOP/TTIN hooks took 58–74 ms. The final SIGKILL at `until` also now applies to hook-first's own-end phase (before, helpers still listed at its bound were left). A small widening on the same path.
- **Minor 8 guard.** `Reap` (fixture.rs) kills the hook's pid and every member still in its group (never the test's own group). grep: every `sleep 600` test holds one. `stop_at_the_hook` / `stop_once_stopped` return it; `Reap::hook` is used in the two direct `stop::stop` tests, the fallback test and the background-job test.
- **Gates.** fmt, clippy `--all-targets`, nextest 5713/5713 and doctests all pass, workspace-wide. Heavy clippy fails only in graft: `tests/it/frames/{after_idle,journal_find,run}.rs`, `ghostty_shot.rs`, `held_key.rs` (+ `jobs/runner.rs`, cited as the callee). graft-git and graft-ops are clean (13f).
- **Touches.** graft-ops `stops_hooks_first` / `it::stop_hooks` and graft `it::quit_waits`: all ran in the final workspace run and passed. `grep -rnw "group_stop|hooks_first" crates` → only `crates/graft-git/src/run/`. Both MAPs updated (run row, layout rows, `## apply` file list, R2 sentence). No capture, keymap, journal or undo change.
