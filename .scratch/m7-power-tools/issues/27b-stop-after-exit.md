# 27b stop-after-exit — 27's item 1a, across the crates it touches

Status: blocked (needs `crates/graft-git/tests/it/compare_and_stash.rs`; everything else done — see NOTES)
Blocked by: 23-recover-op (graft-ops), 12d (graft)
Crates: graft-git (`run.rs`), graft-ops (`apply/remote.rs`, the undo_cancel / set_aside paths), graft (tests; the
prompt-cancel mapping if it lives there).
Build with: Opus. Concurrency in the git runner; undo.
Filed by: conductor, 2026-09-26. Not a design change.

## The bug (reproduced by 27)
`run::a_stop_after_git_has_exited_reports_the_exit`: `update-ref` wrote the ref, and the run said `Err(Stopped)`. A
write that landed is reported as not having happened. Journal and undo then reason from a false premise.
27's fix is on branch `m7-27-1a-stop-after-exit`: on a stop, `try_wait` first; if git has already exited, return its
real exit. Read 27's NOTES "1a" in full (the residual window, the pipe-held-open case).

## Knock-on (measured by 27: 7 tests red with 1a)
- graft `credential_by_key::esc_in_prompt_cancels_push`, `m4_walk::every_scenario::password_prompt_esc_cancels`: now
  `Aborted "authentication failed"`. **Ruling (conductor): Esc on the prompt still reads as cancelled, exactly as
  today.** The cancel is his act, not git's failure. Trace why git exits first (27 suspects askpass refusing), then map
  "he cancelled the prompt" to Cancelled at the cause. `graft-ops/src/apply/remote.rs:179` maps only
  `RunError::Stopped` today. Not by string-matching git's words, if there's a structural signal.
- graft-ops `undo_cancel` ×3 and `set_aside` ×2 (names in 27's NOTES). For each, decide with evidence whether the test
  encodes the wrong premise (a landed write treated as not landed). If so, correct the test and say why. Or whether
  the code relies on it, then fix the code. **No test weakened to pass.** Undo after a part-way cancel must still
  restore everything: `after_a_part_way_cancel_the_named_key_restores_everything` must hold in substance.

## Acceptance (name the verifying test at tick time)
- [x] 1a merged: `run::a_stop_after_git_has_exited_reports_the_exit` green (cherry-picked 0fab3a10; red without the
  run.rs hunk, re-measured here: `panicked at crates/graft-git/tests/it/run.rs:164:5: Err(Stopped)`).
- [x] Esc on the prompt: `credential_by_key::esc_in_prompt_cancels_push`, `m4_walk::every_scenario::password_prompt_esc_cancels`
  green, test files untouched (words unchanged: `Aborted "cancelled"`). Red with 1a alone: `Aborted "authentication
  failed for http://tester@127.0.0.1:…"`. Fixed at the cause — code, `apply/remote.rs` `Refusals` +
  `cancelled_at_the_prompt`; unit tests `apply::remote::tests::{refusals_remember_an_unanswered_ask,
  an_exit_after_a_refused_ask_with_the_stop_asked_is_the_cancel}`. 5/5 repeat runs green.
- [x] Each of the 5 graft-ops tests (all green, 5/5 repeat runs):
  - `set_aside::a_stop_before_each_step_loses_nothing` — **code fixed**: `set_aside::put_aside` asks the stop once more
    after `reset --hard`, before the operation; it relied on the reset's run reporting a post-exit stop as `Stopped`.
  - `set_aside::undo_after_a_stop_at_4_or_5_restores_everything` — same code fix; test untouched.
  - `undo_cancel::rollback_refused_by_a_moved_ref_is_reported_not_hidden` — **test corrected**: the stop keyed on the
    *last* step (working tree) writing, so with 1a the undo truthfully finished (`Applied`). Part-way cut now keyed on
    the index step landing (`Trigger::InIndex`); asserts the index step reversed and the working tree untouched.
  - `undo_cancel::after_a_part_way_cancel_the_named_key_restores_everything` — same corrected setup
    (`cancel_part_way`); assertions untouched; `U` still restores everything (`state_of == before`).
  - `undo_cancel::a_killed_step_whose_ref_is_back_where_it_started_is_not_reversed` — **test corrected**: it needs a
    step genuinely killed; now git is held in its `reference-transaction` hook at `prepared` until the stop kills it.
    Mutation-checked: forcing `settled` → `Applied` makes it fail ("refs/heads/main is not where the undo left it").
- [x] Captures unchanged: every `check_shot` test green in the full run (38 files in graft/tests use it); `git status`
  shows no `docs/shots` change.

## Gates
Workspace-wide, plus the heavy clippy line.

## NOTES (builder, 2026-09-27)
- Status: **blocked** on one file outside the granted set. Scout 27b used. No compaction.
- **Blocked: needs `crates/graft-git/tests/it/compare_and_stash.rs`.** Two more tests 27 did not list (they landed with
  14b after 27's measurement) are red with 1a, deterministically (3/3 runs, alone):
  `compare_and_stash::{only_in_stash_stopped_at_any_check_is_never_partial, compare_span_stopped_at_any_check_is_never_partial}`
  — `k=7 of N=9: Ok after a stop` / `k=6 of N=8`. Wrong premise: they assert any stop check answered `true` ⇒ `Stopped`.
  With 1a a stop polled after git exited returns git's full result. Proposed correction (lines :372, :399): drop
  `assert!(stop.made() <= k, "…Ok after a stop")`; keep `assert_eq!(got…, full…, "partial")` — the substance, "never
  partial", stands. Not applied: outside my Files.
- Scout's "could not confirm" (why later undo steps ignore a latched stop): checked — there were no later steps. The
  `PathExists("a.txt")` trigger fires in the working-tree step, the plan's last (`restore.rs` `run_plan` :448); with 1a
  that step returns `Ok`, the loop ends, `Ran::Done`. A latched stop *does* cancel the next step (`restore.rs:460`), which
  is what the corrected `cancel_part_way` relies on.
- Esc trace (verified by the red→green, not by instrumenting): Esc sets the job's cancel (`update/credential.rs:352-374`)
  → `ChannelAskPass::ask` returns `None` (`jobs/askpass.rs:149`) → git exits non-zero → runner, polling the stop, finds
  git exited → `Exited` → `classify_failure` → `AuthFailed`. Structural signal: the `AskPass` contract says `None` is
  "the user cancels" (`graft-git/src/run/askpass.rs:57-59`). `remote::run` wraps the app's `AskPass` to record a refusal;
  `Exited` + refusal + stop asked → `RunError::Stopped` → `RemoteFailure::Cancelled`. A refusal without a stop (the
  disconnected case) stays git's failure, as before. No string matching.
- Also corrected (not in the 5, same wrong premise, was passing): `undo_cancel::a_ref_moved_under_a_killed_step_is_reported_not_overwritten`
  — passed only via the roll-back's compare-and-swap once 1a made its step `Applied`, no longer exercising `settled`'s
  `Unknown(RefMoved)` path. Now held at `committed` in the same hook, so the step is truly killed after writing.
- Residual (27's NOTES, unchanged): git exiting between `try_wait` and the SIGTERM is still `Stopped`.
- Other `Stopped` mappings (scout: `resume.rs:281`, `finish_merge.rs:67`, `jobs/operate.rs:242`, `restore/steps.rs`):
  not changed; they now see `Ok` for a post-exit stop, which is the true report. Checked only by the full suite
  (4316/4318 green; the 2 red are the blocked ones).
