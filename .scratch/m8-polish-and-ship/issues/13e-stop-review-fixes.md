# Stop review fixes, part 1: the stop in graft-git, the splits in graft-ops

Status: draft
Build with: **Opus** (high, not xhigh). This is the code that signals git's process group during a write. Whether a
stopped hook really ends can't be judged from the diff, only from a process test. It changes no journal or undo
semantics, so it doesn't need xhigh. Not Fable: the shape stays as it is.
Needs: none (no new signatures).
Blocked by: 10e (conductor 2026-10-07: 10e is editing graft-ops refusal callers now; 13e\'s move-only splits go after it)
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
- [ ] Splits: the move commits come first. nextest total N before = N after (both numbers in NOTES). `exec.rs`,
      `apply.rs` and `resume.rs` are each ≤ 400 production lines (`wc -l` up to `#[cfg(test)]`, in NOTES).
- [ ] Major 3, SIGSTOP:
      `run::tests::stop::the_group_stop_continues_and_ends_a_hook_stopped_by_sigstop`.
      - Hook `kill -STOP $$` under `StopOrder::Group`.
      - Before the stop, assert the hook's `ps -o stat=` starts with `T`.
      - After it: the hook's pid is gone (`kill -0` → ESRCH), the group is empty, and the stop ends within
        `STOP_WITHIN`.
      - Seen red without the SIGCONT line (NOTES).
- [ ] Major 3, SIGTTIN: `…::the_group_stop_continues_and_ends_a_hook_stopped_by_sigttin`. Same asserts, with
      `kill -TTIN $$`. If the hook never reaches `T` (the kernel discards TTIN in an orphaned group), say so in NOTES
      with the `ps` line. Don't fake it.
- [ ] Minor 2: `…::the_group_stop_ends_a_hook_that_ignores_term`. Hook `trap '' TERM; sleep 600`, default stop: the
      hook is gone, the group is empty, the stop ends within `STOP_WITHIN`. Seen red first.
- [ ] Minor 4: `…::hooks_first_ends_helpers_when_members_cannot_be_read_after_git_ended`. Inject `Err` on the member
      read after git's exit (the existing injection seam of
      `hooks_first_falls_back_to_the_group_stop_when_members_cannot_be_listed`): the helper is gone.
- [ ] Minor 8: `…::the_hook_guard_kills_its_pid_on_drop`. Spawn `sleep 600`, write its pid, drop the guard: the pid
      is gone. Every stop test that runs `sleep 600` holds a guard (grep in NOTES).
- [ ] R2: `graft-ops it::finish_merge::a_hook_refusal_leaves_the_merge_in_progress` passes unchanged. The comment
      and the MAP sentence exist (grep in NOTES).
- [ ] After all of this, `ps -A | grep -c "sleep 600"` reads 0 (NOTES).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo clippy --all-targets --features heavy-tests -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

**Known:** heavy clippy fails in graft tests until 13f lands (review Major 2). If 13f hasn't merged, record that
the failures are only those 5 graft files, and that graft-git and graft-ops are clean.

## Size
One context, about 70k tokens. Most of it is reading the four files. If it runs long, split at the crate seam:
graft-git items 1–5, then graft-ops items 1 and 6.
