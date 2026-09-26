# 27 graft-git-run-fixes — gap fixes in graft-git found during M7

Status: partial (1a blocked: needs graft-ops, graft)
Blocked by: none
Crates: graft-git only.
Build with: Opus. Concurrency in the git runner.
Filed by: conductor, 2026-09-26, from KNOWN_GAPS rows with lines. Not a design change.

## Scope
1. **KNOWN_GAPS M7/dbg (unverified):** the debugger read two races in the run loop:
   - `crates/graft-git/src/run.rs:478` checks the stop before `try_wait`. A git that already finished is reported as
     stopped.
   - `run.rs:331` spawns git before the first stop check.
   **Reproduce first**, with a test that fails today (e.g. a stop requested just after git exits). If a race doesn't
   reproduce, say so, with the attempt, and don't change that code. Fix at the cause. Check every caller that reads
   "stopped" (grep, list them in Touches).
2. **KNOWN_GAPS M7/19b (broken):** heavy-feature clippy fails on `examples/trace_perf.rs` (m7/18). Make
   `cargo clippy --workspace --all-targets --features heavy-tests -- -D warnings` clean for graft-git. (Use the
   feature name the repo uses.)
3. **KNOWN_GAPS M7/18c (question):** the 18 builder reported two parse failures:
   - `diff/unified/primitives.rs:54` can't parse a mixed-quoted `diff --git` header;
   - `log -L` across a non-ASCII rename returns `Parse`.
   **Verify each with a real-git test first.** Reproduced → fix at the cause. Not reproduced → say so.

## Acceptance (name the verifying test at tick time)
- [x] 1b: reproduced, fixed — `run::a_stop_asked_before_run_spawns_nothing` (tests/it/run.rs; red before: a stop
  already asked still let `git update-ref` write the ref). On this branch.
- [ ] 1a: reproduced, fixed, **NOT merged here** — `run::a_stop_after_git_has_exited_reports_the_exit` (red before:
  `update-ref` wrote the ref, run said `Err(Stopped)`). Fix lives on branch `m7-27-1a-stop-after-exit` only: it breaks 7
  tests in graft-ops/graft (see NOTES). Blocked: needs graft-ops + graft.
- [x] 2: `cargo clippy --workspace --all-targets --features heavy-tests -- -D warnings` clean (examples/trace_perf.rs
  split into `measure` + `one_run`; `BlameOptions` by value).
- [x] 3a: reproduced, fixed — `trace::a_mixed_quoted_diff_git_header_parses` (tests/it/trace.rs; real `git diff -M`,
  header `a/plain.txt "b/caf\303\251.txt"`; was `ParseError line 1 malformed diff --git header`).
- [x] 3b: reproduced, same cause, fixed by 3a — `trace::line_history_crosses_a_non_ascii_rename` (was `Parse` at line 3).

## NOTES (builder, 2026-09-26)
- No scout: ticket gave file:line; run.rs/exec.rs/primitives.rs read directly. No compaction.
- 1a/1b are both real. Attribution measured by running the downstream tests with 1b only (all green) vs 1b+1a
  (7 red): **every downstream failure comes from 1a**.
- 1a fix (branch `m7-27-1a-stop-after-exit`, commit on top of this branch): on stop, `try_wait` first; git already
  exited → drain its pipes (one poll each) and return its real exit; a pipe still held open by something git left
  running → `Stopped` (no whole result). Residual window: git exiting between `try_wait` and the SIGTERM landing is
  still reported `Stopped` (ms, `kill(1)` spawn time).
- 1a knock-on (all rely on "stop observed after git's exit ⇒ Stopped"):
  - graft `credential_by_key::esc_in_prompt_cancels_push`, `m4_walk::every_scenario::password_prompt_esc_cancels`:
    observed: now `Aborted "authentication failed"`; suspected (not traced): Esc → askpass refuses → git exits first →
    reported, not cancelled. **User-visible if so**: the app would need to map "user cancelled the prompt" to Cancelled itself
    (`graft-ops/src/apply/remote.rs:179` maps only `RunError::Stopped`).
  - graft-ops `undo_cancel::{after_a_part_way_cancel_the_named_key_restores_everything,
    rollback_refused_by_a_moved_ref_is_reported_not_hidden, a_killed_step_whose_ref_is_back_where_it_started_is_not_reversed}`:
    `StopOnce` fires when the ref step's ref has landed; git has exited, so the step now succeeds and the undo
    completes (`Applied`) — later steps don't observe the stop.
  - graft-ops `set_aside::{a_stop_before_each_step_loses_nothing, undo_after_a_stop_at_4_or_5_restores_everything}`
    (Rebase, step 5): stop keyed to `reset --hard` having written `base.txt`; reset now reports done, the rebase is
    stopped before spawning, put-back runs → `stash drop`, edits back in tree, vs ADR-0045 §5's expected `reset --hard`
    / S listed / tree clean.
  - Decision needed (conductor/planner): whether ops want the true report (tests + Esc mapping change) or keep 1a open.
- 1b changes poll counts by one per run (the pre-spawn poll). Full workspace suite green with it (3733/3733).

## Gates
Workspace-wide, plus the heavy clippy line.
