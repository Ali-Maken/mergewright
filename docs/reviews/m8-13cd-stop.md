# Review: M8 stop work — 13b review fixes, 13c, 13d (stop leaves the merge paused)

Diff reviewed: `git diff 111159e0^1 111159e0` (merge "m8 13b review fixes + 13c/13d stop leaves merge paused", 96
files, +6257/−681). Read in a detached worktree at `111159e0`, own `CARGO_TARGET_DIR`, one cargo command at a time.
Tickets: `13b-quit-waits-for-a-write.md` ("Review fixes"), `13c-stop-leaves-merge-paused.md`, `13d-stop-part-c.md`.
User ruling: `docs/field-notes.md:91` (2026-10-04). Earlier review: `docs/reviews/m8-13b-quit.md`.

**0 Critical · 3 Major · 9 Minor · 2 Questions.** Ticks: **15 checked, 15 held.**

## Gates

| Gate | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass (rc 0) |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | **FAIL: 12 compile errors in `graft` test `it`** (Major 2) |
| `cargo nextest run` (workspace) | **5692/5692 pass**, 10 skipped |
| `cargo test --doc` (workspace) | pass |
| Heavy tier, release perf run, fuzz, bench | **not run, on the conductor's instruction** ("Fast tier only"). The heavy tier would not build anyway (Major 2) |

- Fast suite: nextest 582 s, 635 s wall. Peak RSS 1.47 GB (`/usr/bin/time -l`). `target/` 5.0 GB.
- Slowest tests: the whole-suite top 5 is not recorded, because nextest's `status-level = "fail"` prints no names.
  Instead, the 122 stop-related tests (`stop_hooks`, `quit_waits`, `run::tests`, `jobs::tests`, `quit_offer`,
  `quit::tests`) ran alone with `--final-status-level all` in 12.9 s. Slowest:
  - `quit_waits::signal::stop_now_after_the_real_offer_…`: 11.4 s (the real 10 s offer)
  - `…a_signal_restores_then_waits_and_a_second_stops_git`: 1.8 s
  - `run::tests::stop::hooks_first_kills_a_hook_that_ignores_term`: 1.4 s
  - `stop_hooks::a_paused_merge_continues_…`: 1.2 s
  - `…a_resume_stopped_in_its_hook_stays_paused`: 1.1 s
- After both runs, `ps` showed no `sleep 600` or `sleep 30` left behind.

## The M3 promise, stop case by stop case

What is on disk after each stop, and where relaunch lands. "Test" is the test that shows it; "read" means I read the
code and did not observe it.

| Stop case | HEAD | git's state | index / tree | locks | journal | relaunch | shown by |
|---|---|---|---|---|---|---|---|
| Merge, stuck in `pre-merge-commit` / `prepare-commit-msg` / `commit-msg` (hook-first) | unmoved | `MERGE_HEAD` = source tip, `MERGE_MSG`, `MERGE_MODE` | merge result staged; his edit byte-equal | none | `Pending`, commands held | `MERGING … c finish  a abort`; `a` → HEAD, status, his file as before | `it::quit_waits::stuck::stop_now_mid_merge_leaves_it_paused`, `…::relaunch_after_a_stopped_merge_offers_continue_and_abort`, graft-ops `it::stop_hooks::{assert_merge_paused callers, a_paused_merge_aborted_by_resume_gives_his_edits_back, a_paused_merge_continues_to_the_merge_git_would_have_made, a_paused_merge_is_live_at_relaunch}` |
| Merge, stuck in gpg signing | unmoved | `MERGE_HEAD`, `MERGE_MSG`, `MERGE_MODE`, `AUTO_MERGE` | merge result staged | none | read: same as above (disk decides) | read: same | **observed by me** (scratch repo, git 2.54 Apple, fake `gpg.program` killed: `fatal: failed to write commit object`, exit 128, merge state written). No test. |
| Merge, stuck in `post-merge` | the merge | none | = the merge + his edit | none | `Committed`, `stopped_at: Landed`; `u` undoes | graph | `it::stop_hooks::a_merge_stopped_in_its_post_merge_hook_has_landed` |
| Merge, git itself stuck / hook outside git's group (residual) | unmoved | none | merge's files in the tree | none in the test; **SIGKILL after 2 s can strand one** (graft-git MAP `run`, m3/05: 6 in 288 SIGTERMs) | `Aborted "stopped before git could pause it"`, in force | graph; `u` → HEAD, index, tree, his file as before | `it::stop_hooks::a_merge_stopped_before_git_paused_it_is_aborted_and_u_puts_it_back` |
| Rebase (group stop, not hook-first: 13d Q1) | detached at its step | `rebase-merge/` | his edits set aside in the stash | none (asserted) | `Pending` + `put_back` | banner `… from a previous run · edits aside · c continue … a abort` | `it::quit_waits::stuck::stop_now_mid_rebase_with_edits_set_aside` |
| Cherry-pick (one commit), `prepare-commit-msg` / `post-commit` | unmoved / moved | `CHERRY_PICK_HEAD` / none | — | — | `Pending` Paused / `Committed` Landed | banner / graph | `it::stop_hooks::a_cherry_pick_stopped_in_its_hook_is_paused_if_git_pauses_it`, `…_post_commit_hook_has_landed` |
| Revert, `post-commit` | moved | none | — | — | `Committed` Landed | graph | `it::stop_hooks::a_revert_stopped_in_its_post_commit_hook_has_landed`. **Revert paused in `prepare-commit-msg`: no test** (read: same path as the pick). |
| `c` from the banner on a shell-begun merge (`FinishMerge`) | unmoved | `MERGE_HEAD` still live | as before `c` | none | `Aborted "git was stopped"`, not in force | shell-merge banner (read) | `it::stop_hooks::a_finish_merge_stopped_in_its_hook_leaves_the_merge_paused` |
| `c` on a paused merge, stuck again | unmoved | `MERGE_HEAD` | — | — | re-held `Pending` | banner | `it::stop_hooks::a_resume_stopped_in_its_hook_stays_paused` |
| Discard (never pauses) | — | — | half-done | — | `Aborted "git was stopped"`, `Stopped{changed:true}`; `u` restores | graph | `it::ops_execution_hard::a_plan_stopped_between_invocations_is_aborted_recoverable_and_narrow` |
| Hang-up (SIGHUP) mid-write | — | git **keeps running, orphaned** | — | — | `Pending` until git ends (read) | recovery (read) | filed: `M8/hangup-exit-101-orphans-git` (confirmed below) |

Nothing in the diff writes git's merge files: `merge_state` is untouched, and every paused state above is git's own.
I found no path that loses his edits. The two soft spots are a lock stranded by the SIGKILL fallback (pre-existing,
m3/05) and the hang-up gap (filed).

## Process groups: can a child survive, can Graft signal someone else's pid?

**Signalling a pid Graft does not own.**
- `killpg` (`exec.rs` `group_stop`, :317-345) is sent only while git is unreaped. Only `try_wait`/`wait` reap, and
  each `try_wait` that returns `Some` returns from the function. An unreaped leader keeps the group id reserved, so
  this path is safe.
- **Per-pid helper signals are a narrow exception** (Minor 3). `Helpers::pass` (:437-455) lists the group by
  `ps`/`/proc`, then `kill`s each listed pid. Two windows exist:
  - the time between the listing and the `kill`, if a helper has exited and been reaped in between;
  - after git has been reaped (`ended = Some`, :398). Then the loop goes on listing pgid = git's old pid every
    250 ms (:409-419). Once the last helper dies that pgid is free, and a new process that took the pid and called
    `setpgid` would be listed and sent `SIGTERM` + `SIGCONT`.

  Both need pid reuse within milliseconds. That is very unlikely on macOS, whose pids are sequential up to 99999, but
  it is not impossible.

**Children that can survive a stop** (all by reading; the first is by design and tested):
1. A hook that leaves git's group (`setsid`, `setpgrp`, a daemon). It is never signalled. 13c D2 relies on this, and
   `it::stop_hooks::a_merge_stopped_before_git_paused_it_is_aborted_and_u_puts_it_back` shows it.
2. Under the **default group stop**, a helper that ignores or handles `SIGTERM` and does not exit (Minor 2). git dies
   of the `SIGTERM`, so `group_stop` returns at once (:327-334). `SIGKILL` goes to the group only when git itself sat
   through the grace (:340-345). This is the stop a rebase gets (13d Q1), and also remote ops and the hook-first
   fallback. `hooks_first_kills_a_hook_that_ignores_term` covers only the hook-first path.
3. Hook-first, after git has ended on its own, if the member read then fails: `(Err(_) | Ok(false), Some(status)) =>
   return` (:413-415) leaves any listed helper unsignalled, with no comment saying why that is safe (Minor 4).
4. Under the default group stop, a helper held by `SIGTTIN`/`SIGSTOP`: it is never sent `SIGCONT` (Major 3). Only the
   kernel's orphaned-group `SIGHUP` + `SIGCONT` may end it (read).
5. Hang-up: no stop runs at all (filed gap, confirmed below).

## Known gaps: confirmed or refuted

**`M8/hangup-exit-101-orphans-git`: confirmed in part.**
- The orphaning follows from the code: git leads its own group (`graft-git/src/run.rs:478`
  `process_group(0)`), so the terminal's SIGHUP, which goes to the foreground group, no longer reaches it.
- Nothing else stops git once Graft exits 101 before `wind_down` → `Pool::stop` (`startup.rs`).
- It is a **regression that 13c introduced**. Before 13c, git shared Graft's group and died of the SIGHUP (13b
  NOTES: "git and the hook then died"). Since 13c it runs on unattended (13d NOTES).
- I did not observe it, and I did not find the cause of the 101.
- Two doc rows describe the hang-up path as working. `docs/design-notes/m8-polish-and-ship.md:141` says "terminal
  given back at once; the write lands within 60 s → exits; else it is stopped half-way". Keymap §2.2 "Quit from
  outside" says the same. Both are false for SIGHUP today. The gap row should say so.

**`M8/13d-open-points`: each point confirmed.**
- `stopped_on_conflict` still reads `op.starts_in_progress()` (`graft-ops/src/apply.rs:769`), so a `FinishMerge`
  never reaches it. The consequence I can see: a `FinishMerge` whose commit fails without a stop answers `Git`
  (failed), not `Conflicted`. The merge stays live, so the banner still offers `c`/`a`. Nothing is lost. It remains a
  design question.
- Banner words: the capture `docs/shots/m8-quit-stopped-merge-relaunch-80x24.txt:27` reads `c finish  a abort`, but
  design note `:140` says `c continue · a abort`.
- The 80×24 capture has only `MERGING` on its top line (`:5`). "from a previous run" is not drawn.
- A forced undo jump always gets the `None` line: `Cancel::left_at` is called only from `jobs/operate.rs:348`
  (grep), never from `jobs/undo.rs`.
- The 13b box: see the next section.

## 13b's unticked merge-half box

It is **met in substance** by two 13d tests. Nobody ticked it, so it is a missing tick, not a false one. I recommend
the conductor ticks it, naming those two tests and the two deviations below.
- `it::quit_waits::stuck::stop_now_mid_merge_leaves_it_paused` covers:
  - git and the hook gone (`kill -0`)
  - `MERGE_HEAD` = the source tip
  - `HEAD` unmoved
  - no `*.lock` under `.git`
  - one merge entry, `Pending`
  - the exact exit line
  - `quit-stuck-write 4` walked
- `it::quit_waits::stuck::relaunch_after_a_stopped_merge_offers_continue_and_abort` covers the relaunch banner and
  `a`: `HEAD`, `git status` and his file come back as before.

Two deviations from the box's literal words:
1. **The bound.** The box says the app quits within `SHUTDOWN_GRACE`. The walk asserts `FORCED_WAIT` (`stuck.rs:291`),
   which 13c item 11 introduced. The measured time is 54 ms, so it meets either bound.
2. **The banner words.** The box says `c continue · a abort`. The real banner reads `c finish  a abort` (`stuck.rs:347`).
   This is the open point already filed.

## Spot-checked ticks: 15 checked, 15 held

I read each named test against its tick's claim.
1. 13c `run::tests::hooks_first_lets_git_pause_a_merge`: asserts `MERGE_HEAD` = source, `MERGE_MSG`, no lock,
   `git_signalled == false`, exit 1. Held.
2. `run::tests::a_stop_leaves_no_process_of_the_group`: the hook is gone, the group is empty, and the stop ends within
   `STOP_WITHIN`. Held.
3. `run::tests::hooks_first_falls_back_to_the_group_stop_when_members_cannot_be_listed`: an injected `Err`,
   `git_signalled`, the group empty. Held.
4. `it::stop_hooks::a_merge_stopped_before_git_paused_it_is_aborted_and_u_puts_it_back`: the exact reason,
   `BeforePause{changed:true}`, no `MERGE_HEAD`, and `u` → `state_of` equal, his file byte-equal. Held.
5. `it::stop_hooks::a_paused_merge_continues_to_the_merge_git_would_have_made`: `Committed`, `HEAD^2`, and the twin
   fixture's message equal. Held.
6. Walk `stop_now_mid_merge_leaves_it_paused`: all four bullets of the box. Held.
7. Relaunch `relaunch_after_a_stopped_merge_offers_continue_and_abort`: the banner quoted, `a` restores. Held, with
   the wording deviation the tick states.
8. `update::quit::tests::exit_line_says_where_git_left_it`: 7 distinct lines (8 asserts), plus sanitising. Held.
9. `jobs::tests::a_forced_job_is_waited_for_its_answer_up_to_its_bound`: the answer inside the bound; past it,
   `at: None`, `stalled: 1` and the `None` line. Held.
10. `jobs::tests::an_unforced_cancel_is_still_job_cancelled`: `JobCancelled`, `Shutdown::CLEAN`. Held.
11. 13d Q1 `stop_now_mid_rebase_with_edits_set_aside`: no lock, the rebase dir present, `Pending` + `put_back`, the
    exact `Paused(Rebase)` line. Held.
12. 13d item 12 `jobs::operate::tests::answer_paused_lands_as_conflicted`. Held.
13. 13d Q3 `it::ops_execution_hard::a_plan_stopped_between_invocations_is_aborted_recoverable_and_narrow`:
    `Stopped{changed:true}`, `git was stopped`, `u` → `state_of` equal. Held.
14. 13d Q4 `it::stop_hooks::a_finish_merge_stopped_in_its_hook_leaves_the_merge_paused`: `Paused(Merge)`, `Aborted
    "git was stopped"`, not in force, `MERGE_HEAD`, `HEAD`, no lock, his file. Held.
15. Real binary `it::quit_waits::signal::stop_now_after_the_real_offer_restores_then_says_where_git_stopped`: restore
    before the line, the `PAUSED` words, exit 0, git and hook gone, `MERGE_HEAD`. Held.

## Feature pipeline

- The design note's Prior art row cites `docs/research/quit-while-git-runs.md`, which exists and has sources.
- Journeys 24/25 are in the note.
- The fence `quit-stuck-write 4` is walked by both stop walks (`within_the_note`). `quit-while-write 2` is walked by
  `quit_while_write_is_two_keys`.
- No new screen, so no prototype was owed.

## Critical

None.

## Major

### 1. Readability: four source files grew past the 400-line limit, and no gap row names them
PROMPT.md 6.2 sets files at about 400 lines, and the review brief makes this a Major.
- `crates/graft/src/jobs/pool.rs`: 249 → **451** (no tests in the file).
- `crates/graft-git/src/run/exec.rs`: the production part (before `#[cfg(test)]` at :530) went 319 → **529**.
- `crates/graft-ops/src/apply.rs`: 717 → **777**. It was already over and grew.
- `crates/graft-ops/src/apply/resume.rs`: 488 → **543**. Same.
- New test files over the limit: `crates/graft-ops/tests/it/stop_hooks.rs` 720, `crates/graft-git/src/run/tests.rs`
  567, `crates/graft/tests/it/quit_waits/stuck.rs` 488.
- `run.rs` shrank 885 → 678 when its tests moved out. Good, but still over.

**Why it matters:** the stop logic is the hardest code in the milestone to follow, and it is spread across files a
reader cannot hold in their head.

**Fix:**
- `exec.rs`: move the stop (`StopOrder`, `Graces`, `group_stop`, `hooks_first`, `Helpers`, `signal_*`) into
  `run/stop.rs`. That module is cohesive and about 230 lines.
- `pool.rs`: move the forced-answer bookkeeping (`ForcedJob`, `Answer`, `force_*`, `wait_for_forced`, `Forced`) into
  `jobs/forced.rs`.
- `stop_hooks.rs`: split into paused / landed / residual.
- Or file a size row naming each file.

### 2. The heavy tier no longer compiles: `Shutdown` and `JobRunner::shutdown` changed, and the heavy-gated callers were not updated
This merge changed two signatures:
- `Shutdown` went from an enum (`Clean`, `Stalled { workers }`) to a struct (`jobs/pool.rs`).
- `JobRunner::shutdown(grace)` went from one argument to three, `(grace, writes, force)` (`jobs/runner.rs:522`).

Five heavy-only files still use the old shapes: `crates/graft/tests/it/frames/after_idle.rs:163-165`,
`frames/journal_find.rs:289`, `frames/run.rs:115-117`, `ghostty_shot.rs:124` and `held_key.rs:419`.
`cargo clippy --all-targets --features heavy-tests` fails with 12 errors (E0061, E0599, E0223). `main` at `63522d17`
has the same code (`git show main:crates/graft/tests/it/frames/run.rs:115`).

**Why it matters:**
- The milestone review's heavy tier and its release perf run (`frames::`, `heavy::budgets_hold`, ADR-0021) cannot
  build, so no frame or first-paint budget can be measured until this is fixed.
- This is exactly the blind spot in KNOWN_GAPS "The heavy code is not in the standard clippy gate". Builders run the
  fast tier only, so nobody saw it.

**Fix:**
- Mechanical: `jobs.shutdown(Duration::from_secs(10), EXIT_WRITE_WAIT, &())` and
  `if shutdown != Shutdown::CLEAN { return Err(…) }` at each site.
- Then add the heavy clippy line to the builders' gate for any ticket that changes a `pub` signature in `graft`.

### 3. The default group stop never sends `SIGCONT`, although the spec, its doc comment and MAP say it does
- 13c Part A item 2 specifies "`killpg(SIGTERM)` then `killpg(SIGCONT)`".
- The doc comment on `group_stop` (`crates/graft-git/src/run/exec.rs:314-316`) says the same. So does the graft-git
  MAP `run` row ("`killpg(SIGTERM)` + `killpg(SIGCONT)`").
- The code sends `SIGTERM` alone (`exec.rs:326`). `grep SIGCONT exec.rs` finds it only in the hook-first helper pass
  (:447).
- Ticket 13c is marked done with this unbuilt, and two docs state it as fact. No box ticks it by name:
  `a_stopped_hook_is_continued_and_ended` exercises only the hook-first path.

**Why it matters:**
- The default stop is what a **rebase** gets (13d Q1), and also every remote op and the hook-first fallback (members
  unreadable).
- A member held by `SIGTTIN` — the `/dev/tty`-reading hook that 13c decision 2 cites as the reason for the group —
  or by `SIGSTOP` does not act on the `SIGTERM`. git dies of it, `group_stop` returns, and the stopped hook is left.
- What may still end it (**read, not observed**): when git dies the group becomes orphaned with a stopped member, and
  POSIX then has the kernel send `SIGHUP` + `SIGCONT` to it. A hook that ignores `SIGHUP` would stay.

**Fix:** `signal_group(group, Signal::SIGCONT)` straight after the `SIGTERM` at :326. Add a test under
`StopOrder::Group` with a `kill -STOP $$` hook, mirroring `a_stopped_hook_is_continued_and_ended`.

## Minor

### 1. Multi-commit cherry-pick stopped after its first commit is misread as the residual (latent)
- `graft-ops/src/apply/stopped.rs:116`: `live == Some(reads) && (reads == InProgress::Rebase || unmoved)` treats
  `HEAD` moved as "not paused" for a pick. A pick of several commits moves `HEAD` as it goes.
- **Observed** (scratch repo, git 2.54): `git cherry-pick A B`, B's `prepare-commit-msg` killed. Result: exit 128,
  `CHERRY_PICK_HEAD` and `sequencer/` present, `HEAD` moved.
- `where_stopped` then returns `Residual` (:129). The entry closes `Aborted "stopped before git could pause it"`
  while git's pick is live, and the exit line says "before git could pause it · u … puts your files back".
- It is latent: every app site builds `PickParams` with one commit (`update/drag.rs:287`,
  `update/context_menu/verbs.rs:127`). But `PickParams::commits` is a `Vec` "for M7" (`operation/history.rs:124`).
- **Fix:** treat a sequencer pick like a rebase (live alone says paused). Or assert `commits.len() == 1` where the
  plan is built, until the multi-pick lands.

### 2. The default group stop leaves a `SIGTERM`-ignoring helper alive
- `graft-git/src/run/exec.rs:317-345`: once git dies of the group `SIGTERM`, `group_stop` returns. `SIGKILL` reaches
  the group only if git outlived the grace.
- A hook with `trap '' TERM` (or one that handles TERM and carries on) outlives a rebase's stop (13d Q1 uses this
  stop), and also a remote op's stop and the hook-first fallback.
- This contradicts the module doc (`run.rs` "a stop signals the group, so a hook and what it started end with git").
  It is the same defect 13b's review found ("the hook outlives the stop"), narrowed to TERM-proof hooks.
- A surviving hook that runs git (a `post-checkout` running `git submodule update`) could take `index.lock` after Graft
  reports the rebase paused.
- Untested: `a_stop_leaves_no_process_of_the_group` uses a hook that dies on TERM.
- **Fix:** after git ends in `group_stop`, run the same bounded helper pass `hooks_first` runs after git ends
  (`SIGKILL` anything still listed after `TERM_GRACE`). Add a test with a `trap '' TERM` hook under the default stop.

### 3. Helper signals by pid can, in a narrow window, reach a stranger
- `exec.rs:398-419` together with `Helpers::pass` (:437-455): after git is reaped, the loop keeps listing pgid = git's
  old pid. Once its last member exits, that pgid is no longer reserved. Separately, every `kill(helper)` acts on a
  `ps` snapshot.
- Both cases need pid reuse within milliseconds to hit an unrelated process. That is extremely unlikely, but it is
  the one way Graft can signal a pid it does not own.
- **Fix:** stop listing once a pass after git's end finds no helper, and never start a pass after git is reaped and
  the previous pass was empty. Or detect git's exit without reaping it (`waitid(WNOWAIT)` where nix offers it), so
  the leader holds the pgid until the helpers are done.

### 4. A member read that fails after git ended is dropped without a comment (rule 3)
- `exec.rs:413`: `(Err(_) | Ok(false), Some(status)) => return Ok(own(status))`. The `Err` is dropped, and any
  helper still alive is left unsignalled. The `Err` arm above it (:412) has its safety comment; this one does not.
- **Fix:** give it its own arm with a comment saying why it is safe. Or fall back to `killpg(SIGKILL)` while a member
  is known to be alive, since the group is then still reserved.

### 5. Stale numbers and words in docs and comments
- `crates/graft/MAP.md:1045`: `SHUTDOWN_GRACE` "(2.55 s)". It is 4.55 s since `STOP_WITHIN` became 4.05 s.
- `crates/graft/src/jobs/mod.rs:106`: "2.55 s".
- `crates/graft/MAP.md:1065` and `jobs/runner.rs:480`: `force_stop` "(SIGTERM, SIGKILL after 2 s, reaped)". A write
  is now stopped hook-first.
- `crates/graft/tests/it/quit_waits/stuck.rs:196-197`: `reap` says "the pool signals git's pid, not its group". That
  has been false since 13c.
- `docs/design/02-keymap.md:241`: "stopping git is pending the user's answer". He answered on 2026-10-04
  (`field-notes.md:91`).
- `docs/design-notes/m8-polish-and-ship.md:72` (Prior art row): still says the merge does **not** pause.
  `:140`: still opens with the 13b line `stopped <op> half-way … here shows where it stopped` (13d deviation 2).
- **Fix:** update each line.

### 6. A non-signal exit with a write running waits up to 60 s and says nothing
- `event_loop/exit_wait.rs:88`: the waiting line is printed only when `signalled`.
- `startup.rs` `wind_down` sets `signalled` only for `Exit::Signal`. An `Exit::Fatal` while a merge runs (a terminal
  read error, for example the EIO 13b's NOTES suspected on hang-up) restores the terminal, then sits up to
  `EXIT_WRITE_WAIT` with no line. That is the frozen-shell look 13b's review fix set out to remove.
- **Fix:** print the waiting line whenever `writes_running()` is non-empty, not only after a signal.

### 7. A forced write that never started gets a "stopped" line
- `jobs/pool.rs` `ForcedJob::new` answers a queued, never-started write `Given(None)`.
- `exit_wait.rs:119-133` then prints `<bin>: stopped <op> after <n> s · <bin> here shows where git left it` for an
  operation git never ran. This is reachable on the signal path with a write queued.
- **Fix:** carry `never_ran` on `Forced` and print `<op> did not start`, or skip the line.

### 8. A failing graft-git stop test leaves `sleep 600` processes behind
- `crates/graft-git/src/run/tests.rs:440-552`: the hooks run `sleep 600`. If an assert fails before `gone`/
  `group_empty` hold, nothing kills them. The graft-ops fixture has a `Drop` that does (`tests/it/stop_hooks.rs:138`),
  and `quit_waits` has `reap`. These do not.
- This is laptop safety (PROMPT.md 7.1).
- **Fix:** a `Drop` guard that `SIGKILL`s the pidfile's pid, as in `Hooked`.

### 9. Revert paused in its hook is not tested
13d Q2 tests a revert that lands. The paused revert (`prepare-commit-msg` killed → `REVERT_HEAD`) has no test. The
cherry-pick one stands in for it by reading the same code path (`stopped.rs:116`, `opens_in_progress(Revert)`).
**Fix:** one more `it::stop_hooks` case.

## Questions (rule 7: no line proves a defect)

1. **Abandoned at `FORCED_WAIT` while writing the journal?**
   - Past `FORCED_WAIT`, `wait_for_forced` gives up (`pool.rs`), `shutdown` returns `stalled`, and `main` exits while
     the worker may still be inside `finish`, appending the entry's outcome line.
   - 13d's safety comment says the entry is "`Pending` or closed". A third state, "being written", is not covered.
   - Does the journal reader tolerate a torn last line, and is the append one `write`?
   - Reaching it needs after-state reads slower than 3 s after the stop (a very large worktree). Not observed.
2. **Hang-up's 101.** Where does the panic come from? 13b NOTES suspect ratatui `Terminal::drop`'s `eprintln!`.
   Unverified by them and by me. Whatever it is, it skips `Pool::stop`, and since 13c that is what orphans git.

## Not findings (checked)
- **gpg signing pauses a merge** (13c's "What is on disk" row 1): observed, see the table. git writes the merge state
  on a signing failure too.
- **`c` after a paused merge skips `pre-merge-commit`.** This is git's design (githooks(5)), and 13d Q4b's comment
  states it in `error.rs`. git's own "Not committing merge; use 'git commit'" makes the same offer.
- **No new `unwrap`/`expect` outside tests.** Both `Err(_) => {}` in `exit_wait.rs` (:95, :115) carry their safety
  comments. `signal_one`'s ignore is commented (:477-485).
- **`Exit::Open` waiting up to 60 s with the screen still Graft's** (`startup.rs` `stop_for_the_next_place`): no code
  path produces `Exit::Open` from `run_repo` (grep), so this is not reachable today.
- **Exit-line sanitising:** `<op>` goes through `sanitize_line` (`quit.rs:300`), and a test feeds it `ESC[2J`.
- **Foreground-group grep (13d Touches):** the editor, mergetool and `--continue` handovers spawn with plain `Command`,
  not `run` (`event_loop/outside.rs`), so they stay in Graft's group. `Ctrl+Z` signals only Graft's own pid
  (`outside.rs:601`).
