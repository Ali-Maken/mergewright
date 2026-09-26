# 27b stop-after-exit — 27's item 1a, across the crates it touches

Status: ready
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
- [ ] 1a merged: `run::a_stop_after_git_has_exited_reports_the_exit` green.
- [ ] Esc on the prompt: both tests above green, with the words unchanged.
- [ ] Each of the 5 graft-ops tests: fixed code, or corrected test with the reason, one line each.
- [ ] Captures unchanged (`check_shot`); any change is a defect unless named.

## Gates
Workspace-wide, plus the heavy clippy line.
