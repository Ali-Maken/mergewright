# 01b error-rows-app — 01's rows whose change reaches graft: `Drift` direction, one stop shape, `worktree_lines`

Status: partial (:572 blocked)
Build with: Sonnet. 01 calls these rows mechanical; the words and shapes are decided in 01 "Decided".
Needs: none.
Blocked by: **01a** (both hold graft-ops `restore.rs`). Wave: see `.scratch/m7-power-tools/parallel-plan.md`.
Split by: planner, 2026-09-27, from `01-error-rows-resume-restore.md` (ADR-0051 §1). No new decision.
Crates: graft-ops, graft.
Files: crates/graft-ops/src/{restore.rs, restore/rollback.rs, restore/steps.rs, undo.rs, undo/execute.rs,
undo/tests.rs, apply/error.rs}; crates/graft/src/{jobs/conflict.rs, jobs/undo.rs, update/undo.rs,
update/undo/consequence.rs}; crates/graft/tests/it/{undo_over_drift.rs, undo_by_key.rs}; every other
`RestoreError` / `UndoError::Drift` match site the change breaks (grep; each named in NOTES — outside this list →
`blocked: needs <file>`); undo walk captures that change (re-taken, listed); MAP.md (graft-ops, graft).
Scout: 01's scout, brief item 2 (the three stop shapes and every match site).

## Scope
01's rows, as 01 "Decided" words them: **:494, :495, :572**.

## Acceptance (tick 01's boxes in 01's file, each naming its test)
- [ ] 01 Acceptance rows :572, :495, :494 (red first where 01 says so).
- [ ] 01's last two boxes (NOTES row → test for all eight rows, 01a's included; MAP.md graft-ops and graft).
- [ ] Gates workspace-wide.

## Touches
01 "Touches": `RestoreError` callers; the M6 resolver cards (m6 resolver captures byte-identical); undo/redo status
words (`U again` on redo; list every undo walk capture that changes).

## Size
~35k.

## NOTES (builder, 2026-09-27) — partial: :494 and :495 done, :572 blocked
Scout used: `.scratch/m7-power-tools/scouts/01.md` (brief item 2). No compaction.

- **:494** `undo::tests::drift_on_redo_says_capital_u`, red first, quoted: `the working tree changed — u again to go
  ahead` (for a `Direction::Redo`). Fix (cause): `UndoError::Drift { drift, direction }`, new pub
  `undo::Direction { Undo, Redo }`; `undo` passes `Undo`, `redo` passes `Redo`, each `revert_to` step `Undo`;
  `explain` names `u`/`U`.
- **:495** `undo::tests::every_stop_is_one_shape`. Scout could not confirm "the two others"; my pick: the shared shape is
  git's own `RunError::Stopped`, i.e. `RestoreError::Run(RunError::Stopped)` (const `RestoreError::STOPPED`). The
  `Stopped` variant is removed. Snapshot, merge-state, prepare and facts stops fold into it where they enter
  (`as_snapshot_error`, `From<PrepareError>`, new crate-private `RestoreError::from_facts`, used in `rollback.rs`).
  The cancel path in `run_plan` matches that one arm. Behaviour change: a `PrepareError::Facts(FactsError::Stopped)`
  used to arrive as `Facts(Stopped)` (a failure, not a cancel); it is now the stop. Not red-first (compile-level).
- **:572 blocked: needs `crates/graft/src/update/resolver/answers.rs`** (sets the resolver notice from the open;
  `worktree_glimpse`'s failure must reach `open.notice` as `can't read <path>: <reason>`), plus the `OpenedFile`
  constructors a new field breaks: `update/tests.rs:5003`, `update/resolver/tests/{advance.rs:383, file_list.rs:199,
  file_list.rs:904, cross.rs:63, take_all.rs:185}`, `view/resolver/tests.rs:{266,311}`. Next step: add
  `OpenedFile::unreadable: Option<String>`; `worktree_glimpse` returns `Result<Option<OnDisk>, String>` (`NotFound` →
  `Ok(None)`); answers.rs puts `unreadable` into `open.notice` as a problem. Relative path: already joined onto the
  workdir at `jobs/conflict.rs:209`; only a bare repo (no workdir) keeps it as given.

Changed signatures: `UndoError::Drift` tuple → struct variant; new `undo::Direction`; `RestoreError::Stopped` removed,
`RestoreError::STOPPED` const added.

Touches:
- `UndoError::Drift` match sites (grep `UndoError::Drift`): `graft/src/jobs/undo.rs:223,262,455`; graft-ops tests
  `history_ops.rs:2217`, `ops_execution.rs:343`, `ops_execution_hard.rs:442,479,625`, `undo_replaced.rs:343`,
  `undo_replaced_whole.rs:86` — pattern-only edits.
- `RestoreError::Stopped` sites (grep): `restore.rs`, `restore/steps.rs`, `restore/rollback.rs`, `undo/execute.rs:108`;
  none in graft.
- The app's drift box draws its own keys from `Undone::Drift { redo }`, not `explain`; `explain`'s text reaches the app
  via `operate.rs` / `error_line` only. No undo walk capture changed (no graft view code touched); m6 resolver captures
  untouched (no resolver file changed). Workspace nextest 4257/4257.
- MAP.md graft-ops: `RestoreError`, `MergeState`, `UndoError` lines. graft MAP: no public item changed.
