# 01b error-rows-app — 01's rows whose change reaches graft: `Drift` direction, one stop shape, `worktree_lines`

Status: draft
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
