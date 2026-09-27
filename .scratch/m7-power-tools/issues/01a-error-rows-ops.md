# 01a error-rows-ops — 01's graft-ops rows that change no signature graft calls

Status: done (2026-09-27)
Build with: Opus. :496's roll-back must be designed against the recording protocol (01 "Build with").
Needs: none.
Blocked by: none. Wave: see `.scratch/m7-power-tools/parallel-plan.md`.
Split by: planner, 2026-09-27, from `01-error-rows-resume-restore.md` (ADR-0051 §1: the graft-ops part stands alone).
No new decision; 01's "Decided" binds.
Crates: graft-ops only.
Files: crates/graft-ops/src/{apply.rs, apply/record.rs, apply/restart.rs (both granted by the conductor 2026-09-27), apply/resume.rs, apply/resume/shell.rs, apply/resume/rewrite.rs,
conflict/set.rs, restore.rs, restore/rollback.rs, restore/steps.rs}; crates/graft-ops/tests/it/error_rows.rs (new,
pre-registered by the conductor's seam commit); MAP.md (graft-ops). A file outside this list → stop, report
`blocked: needs <file>` (ADR-0051 §3).
Scout: 01's scout (ADR-0035), brief items 1, 3, 4 of 01 "Scout brief". One scout serves 01a and 01b.

## Scope
01's rows, as 01 "Decided" words them: **:601, :613, :640, :496, :497**.
- :613: 01 lets the builder pick (explicit `ApplyError::Stopped` match, or `live_entry` returning `FactsError`). If
  the pick changes a signature graft calls, the graft side is outside this ticket: stop, `blocked: needs <graft file>`.
- Out, to 01b: :494 (`Drift` direction), :495 (`RestoreError::Stopped` collapse), :572 (`worktree_lines`).

## Acceptance (tick 01's boxes in 01's file, each naming its test)
- [x] 01 Acceptance rows :601, :613, :640, :496, :497 (red first where 01 says so). All five ticked in 01, each naming its `error_rows::` test.
- [x] NOTES: each row → its test (NOTES below). MAP.md (graft-ops): `Resume` line, `RestoreError`, `execute` failure line.
- [x] Gates workspace-wide: fmt, clippy `--all-targets -D warnings`, nextest, `test --doc`. fmt ok; clippy ok; nextest 4244/4244 (after :601); `test --doc` ok (2026-09-27).

## Touches
01 "Touches", first bullet: every caller of `live_entry` and `restore::execute` (grep, listed in NOTES); undo and resume
tests green, graft's included.

## Size
~35k.

## NOTES (builder, 2026-09-27) — :601 was blocked, then granted and done; see ":601 done" below
Scout used: `.scratch/m7-power-tools/scouts/01.md` (01's). No compaction.

**Blocked: needs `crates/graft-ops/src/apply/record.rs` and `crates/graft-ops/src/apply/restart.rs`.** This ticket's
`Files:` names `apply.rs` for `seal_after` / `chain_states` (01's wording), but the scout found both moved:
`seal_after` is `apply/record.rs:252` (`read_refs(..).ok()…unwrap_or(before)`), `chain_states` is
`apply/restart.rs:255` (`_ => FactsError::Stopped`). Neither file is in the list, so :601 is not done.
Exact next step for :601:
- `chain_states`: replace the `resume::live_entry(..).map_err(|why| match why { .. _ => .. })` with
  `resume::live_entry_facts(repo, entries, live, stop)?` — the error is already `FactsError`, so the arm disappears by
  type (no new signature). Test `chain_states_maps_each_error_kind` in `tests/it/error_rows.rs`.
- `seal_after`: return the `read_refs` failure instead of `unwrap_or(before)`; it returns `Option<AfterState>`, so
  report it as an `AfterState` the caller already carries (the `Unsealed` arm). Test
  `a_failed_read_refs_in_seal_after_is_reported` (red first).

Rows → tests (all in `crates/graft-ops/tests/it/error_rows.rs`):
- :640 `live_entry_stops_when_asked` — red first quoted: `expected Stopped, got nothing Graft started is in progress; an
  operation begun in the shell is finished there`. Fix: `live_entry` calls `in_progress_detail_with_stop(repo, stop)`,
  then polls the stop again (the stopped `rev-list` lands on `detail.theirs`, which `live_entry` never reads, so the
  second poll is what turns it into `Stopped`). Cause fix.
- :613 `a_facts_failure_is_not_reported_as_stopped` — **not red first**: it passed before the fix. The old catch-all
  could only ever catch `ApplyError::Stopped` (live_entry's only non-`Facts` error), so no input mislabelled a failure;
  the row was the code shape. Pick: **option 2**, as a new crate-private `live_entry_facts` returning `FactsError`;
  `live_entry` wraps it (`FactsError::Stopped` → `ApplyError::Stopped`, else `Facts`) so its six callers are unchanged.
  `conflict/set.rs` calls `live_entry_facts(..).map_err(ConflictSetError::Facts)`: no arm at all.
- :496 `a_restore_that_fails_mid_way_rolls_back` — red first quoted: `main is back where the restore found it` failed
  (main left at the first step's target). Also `a_failed_roll_back_names_what_is_left` (red: `expected FailedPartway,
  got refs/heads/main is not where it was; nothing was changed` — "nothing was changed" was false). Fix:
  `rollback::roll_back_after_failure`, uncancellable, reverses what ran newest first plus the failed step unless it is
  an `UpdateRef` (atomic in git). Clean → the step's own error, entry `Aborted`, not in force. Refused → new
  `RestoreError::FailedPartway { failure, applied, rollback }`, words name what is still changed and `U`.
- :497 `a_bad_ref_name_is_refused_before_any_write` (`refs/heads/bad..name`, `-d`, `refs/heads/a b`) — red first
  quoted: `expected BadRefName, got \`git update-ref refs/heads/bad..name … ''\` failed (128)` after step 1 had already
  written. Fix: `restore::check_ref_names` before `begin` — `RefName::new` (graft-core's check-ref-format checker) plus
  "under `refs/`" (the restore path's own rule: `-d` passes check-ref-format's byte rules but reaches git as an
  option). New `RestoreError::BadRefName { name, reason }`. No journal entry, no backup, nothing written.

Changed signatures: `RestoreError` gains `FailedPartway`, `BadRefName` (pub, additive; graft's matches have a fallback
arm, workspace builds). `started_on_its_line` (private) now returns `FactsError`. New crate-private
`live_entry_facts`. `Ran::Failed` carries the roll-back outcome (private).

Touches:
- `live_entry` callers (grep `live_entry(`): `resume.rs` `resume_target`, `restart.rs:261` `chain_states`,
  `fold.rs:54`, `recover.rs:39`, `finish_merge.rs:47` — same `ApplyError` as before, one extra stop poll after the
  state read; `conflict/set.rs:143` → `live_entry_facts`. Checked: workspace nextest 4242/4242 green (undo_cancel,
  finish_merge, conflict_set, recover, restart_* included, graft's included).
- `restore::execute` callers (grep): `undo/execute.rs:169,179` only (graft reaches it through `undo`). graft's
  `jobs/undo.rs:266–281` matches `Cancelled`/`CancelledPartway`/`AfterState`; `FailedPartway` goes to its general
  arm and shows `explain()`. Not seen in the app: no capture taken (not a visual ticket; 01b holds graft).
- Keymap / captures: none changed.

### :601 done (after the grant of record.rs and restart.rs)
- `a_failed_read_refs_in_seal_after_is_reported`, red first, quoted: `the backup ref's read failure is reported: ["the
  after-state could not be kept reachable: \`git update-ref refs/graft/backup/main/01HF7YAT… 599d8e8…\` failed (128):
  … unable to resolve reference … reference broken"]`. The read failure was dropped, and the seal went ahead on the
  guessed parent `before`, whose failure only showed up later as `Unsealed`. Fault: a `post-commit` hook breaks the
  loose refs under `refs/graft` between `begin` and `finish`. Fix (cause): `seal_after` reports
  `AfterState::Refs(<the read error>)`, drops the after-trees (the same `drop_after_trees` the `Unsealed` arm now
  shares), and seals nothing. An **absent** backup ref still falls back to `before`, because there is no chain to lose.
  Question for the conductor: should that be reported too? Not changed here.
- Used existing `AfterState::Refs(FactsError)`, not a new variant: a new variant needs `apply/error.rs`, which was not
  granted. Its words read "the refs after the operation: <error>", and the error names the backup ref. A
  `BackupUnread { name, error }` variant would say it more precisely. That needs error.rs, and is proposed, not done.
- `chain_states_maps_each_error_kind`: **green before the fix**, like :613, because the old `_ =>` arm only ever caught
  the stop and already mapped it to `FactsError::Stopped`. The fix removes the arm by type: `chain_states` calls
  `resume::live_entry_facts(..)?`. The test covers both kinds through `stage_source`: the stop comes back as
  `Stopped`, and a broken `main` comes back as a non-stop facts error.
- Touches: `seal_after` is called only by `record::finish` (grep). `finish` serves every operation, restore included.
  graft reads `AfterState` through `explain`/`to_string`, and `jobs/operate.rs:352` matches only `StagingNotKept`.
  `chain_states` is called only by `find_source`. Workspace nextest 4244/4244.
