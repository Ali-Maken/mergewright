# 01a error-rows-ops — 01's graft-ops rows that change no signature graft calls

Status: draft
Build with: Opus. :496's roll-back must be designed against the recording protocol (01 "Build with").
Needs: none.
Blocked by: none. Wave: see `.scratch/m7-power-tools/parallel-plan.md`.
Split by: planner, 2026-09-27, from `01-error-rows-resume-restore.md` (ADR-0051 §1: the graft-ops part stands alone).
No new decision; 01's "Decided" binds.
Crates: graft-ops only.
Files: crates/graft-ops/src/{apply.rs, apply/resume.rs, apply/resume/shell.rs, apply/resume/rewrite.rs,
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
- [ ] 01 Acceptance rows :601, :613, :640, :496, :497 (red first where 01 says so).
- [ ] NOTES: each row → its test. MAP.md (graft-ops).
- [ ] Gates workspace-wide: fmt, clippy `--all-targets -D warnings`, nextest, `test --doc`.

## Touches
01 "Touches", first bullet: every caller of `live_entry` and `restore::execute` (grep, listed in NOTES); undo and resume
tests green, graft's included.

## Size
~35k.
