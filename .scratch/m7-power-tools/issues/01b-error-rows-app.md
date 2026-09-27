# 01b error-rows-app — 01's rows whose change reaches graft: `Drift` direction, one stop shape, `worktree_lines`

Status: done
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

## Ruling (planner, 2026-09-27) — :495's shared shape

**Change the pick.** The shared shape is `RestoreError::Cancelled { rolled_back }`, not `Run(RunError::Stopped)`.

**What "the two others" are** (the scout could not confirm; the row's own history does):
- `KNOWN_GAPS.md:495`: "`RestoreError::Stopped` is a third stop-shape alongside the existing two", filed by "M5/06 fix".
- `.scratch/m5-history-and-ghost-preview/issues/06-undo-cancel-rollback.md:201-202` (that fix round): "`RestoreError::Stopped`
  (a stop before `begin`) is a **third** cancel-ish shape reaching group 08 and needs `Done("undo cancelled")` too."
- `docs/reviews/m5-06-undo-cancel-rollback.md:163-165`: "Three cancel-ish error shapes reach group 08, NOTES lists two …
  The `undone` arm needs `Stopped` → `Done("undo cancelled")` too."
- So the two are `Cancelled { rolled_back }` and `CancelledPartway` (`crates/graft-ops/MAP.md:223`). `RunError::Stopped` is
  not one of them, and the app does not treat it as a cancel: `jobs::undo::undone_from` maps `Cancelled` /
  `CancelledPartway` only (`docs/reviews/m5-08-app-preview-and-keys.md:137-139`; scout `01.md:23`). A `Run(Stopped)`
  reaches some other arm. Which one, and its words, I did not verify (no source read). The test below settles it.

**Stop during facts or prepare = cancel: yes, and the user must see it as one.** The only source of that stop is `Esc`
on the undo job (ADR-0037 §6, `docs/adr/0037-m4-checkpoint-decisions.md:50-52`). Before any step runs, nothing has
changed, so `undo cancelled · nothing changed` (`Status::Done`) is the true sentence. A failure line there breaks
`crates/graft/MAP.md:851` ("a cancel is not a failure") and CLAUDE.md's status rule (never read as an error when nothing
is wrong). The builder's direction (a prepare stop stops being a `Facts(Stopped)` failure) is right. Only the shape
it lands in is wrong.

**Exactly what to do:**
1. **Public contract.** Every stop that leaves `restore::execute`, `execute_over_drift`, and the undo path (the
   `undo/execute.rs:108` site the scout names) is **`RestoreError::Cancelled { rolled_back }`** (`rolled_back: 0` when
   the stop came before any step ran, including before or inside `begin`) or **`CancelledPartway`**. No other
   variant ever carries a stop out of restore. `RestoreError::Stopped` stays removed.
2. **Journal.** If `begin` wrote an entry before the stop, it closes as today's cancel closes it (`Aborted("cancelled")`,
   after-state read fresh). If nothing was written, nothing is journaled. Don't invent an entry.
3. **Internal folding: keep it.** Folding the snapshot, merge-state, prepare and facts stops into one recogniser, so that
   `run_plan`'s cancel path catches every stop, is right. It also closes the M6 flaw where `scoped_paths` turned a cancel
   into `Snapshot(Stopped)`, which the roll-back missed (`docs/log/m6.md:58`). Its representation is your call, as long
   as it stays crate-private. **`RestoreError::STOPPED` must not be `pub`**: `pub(crate)` or gone. No caller outside
   `restore` may match `Run(RunError::Stopped)` as a meaning.
4. **Callers match one variant.** The app's existing `Cancelled` arm (`Undone::Cancelled { clean: true }`) covers it,
   so no graft match change is expected. If one is needed, it's inside this ticket's file list.
5. **Docs.** graft-ops MAP.md `RestoreError`: `Cancelled` covers a stop before any step (`rolled_back: 0`, entry only if
   `begin` wrote one). graft MAP.md:851: "the journal gained an `Aborted` entry either way" is no longer always true.
   Reword it. `refresh` stays `Everything` (a spare re-read, harmless).

**Tests (replace the NOTES' :495 test; the tick names them):**
- graft-ops `every_stop_is_one_shape`: the m5/06 counting stop, fired at **every** poll index of a 3-step undo (the
  `cancel_at_every_step_boundary_is_clean` sweep). Each result is `Ok`, `Cancelled { .. }` or `CancelledPartway { .. }`,
  never another variant. Every index before the first step gives `Cancelled { rolled_back: 0 }`, with refs, index
  and worktree read back unchanged. Red first against your current branch (a pre-`begin` stop yields `Run(Stopped)`).
- graft `a_stop_before_any_step_reads_as_cancelled` (in `tests/it/undo_by_key.rs` or `jobs/undo.rs`): a stop before
  the first step gives `Undone::Cancelled { clean: true, .. }`, and the status line is `Done`, not a failure.

**Not in this ruling:** on `U`, `Cancelled`'s words say "undo cancelled" (`crates/graft-ops/MAP.md:223`). Whether a redo

## NOTES (builder, 2026-09-27) — done: :494, :495 (per ruling), :572
Scout used: `.scratch/m7-power-tools/scouts/01.md` (brief item 2). No compaction.

- **:494** `undo::tests::drift_on_redo_says_capital_u`, red first, quoted: `the working tree changed — u again to go
  ahead` (for a `Direction::Redo`). Fix (cause): `UndoError::Drift { drift, direction }`, new pub
  `undo::Direction { Undo, Redo }`; `undo` passes `Undo`, `redo` passes `Redo`, each `revert_to` step `Undo`;
  `explain` names `u`/`U`.
- **:495, per the planner's ruling above.** `undo_cancel::every_stop_is_one_shape` (graft-ops it; every poll, stride
  1, through `restore::execute` and through `undo::undo`), red first, quoted: `poll 1 (through undo: false) left as
  another shape: Run(Stopped)`; after the pre-`begin` fix, `poll 3 … Snapshot(Stopped)` (a stop inside `begin`), then
  `a stopped undo left as Facts(Stopped)` and `Facts(Snapshot(Stopped))` (the rails). graft
  `jobs::undo::tests::a_stop_before_any_step_reads_as_cancelled`, red first against the pre-ruling branch, quoted:
  `a stop before any step is a cancel, got Failed { error: "stopped", refresh: None }`. Fix (cause): `STOPPED` is
  `pub(crate)`; a stop before `begin`, inside `begin` (`cancel_if_stopped`, and `From<PrepareError>` folds
  `Snapshot(Stopped)`), or in `undo`'s rails (`facts_or_cancel`) is `Cancelled { rolled_back: 0 }`; `revert_to`'s
  between-step stop too. No journal change: `begin` journals what it wrote; nothing new invented.
  **Judgement to check:** `facts_or_cancel` also asks the stop (a rail failure while the stop is raised = cancel),
  because stopped reads surface nested in several `FactsError` variants. A real read failure that coincides with
  `Esc` reads as a clean cancel; nothing was written either way. Cost: the sweep takes ~110 s alone (stride 1, two
  paths); nextest runs it in parallel (workspace 393 s). The old unit test is renamed
  `undo::tests::inner_stops_fold_to_one_recogniser`.
- **:572 done (granted).** `jobs::conflict::tests::a_permission_error_shows_in_the_card_not_empty`, red first, quoted:
  `no words: OpenedFile { file: None, how: DeletedByThem, binary: false, on_disk: None, unreadable: None, … }`. Fix
  (cause): `worktree_glimpse` returns `Result<Option<OnDisk>, String>`; `NotFound` → `Ok(None)`, any other error →
  `can't read <path>: <reason>` in new `OpenedFile::unreadable`; `answers.rs` lands it as the notice, a problem, over
  the landing words (`cross::an_unreadable_card_lands_its_words_as_the_notice`, written after the wiring: not red
  first). `a_relative_path_reads_from_the_workdir` was green before: the join existed. Constructor sites edited:
  `update/resolver/tests/{advance.rs:383, cross.rs:63, file_list.rs:904, take_all.rs:185}` (none in 16's or 13b's
  lists); `update/tests.rs:5003`, `file_list.rs:199` and `view/resolver/tests.rs` build `WholeFileShape`, not
  `OpenedFile`, so they were not touched. The new tests are in `jobs/conflict/tests.rs` (conflict.rs's own test
  module) and `cross.rs`. Not seen in the app: no capture taken.

Changed signatures: `UndoError::Drift` tuple → struct variant; new `undo::Direction`; `RestoreError::Stopped` removed (no pub
replacement); new graft `OpenedFile::unreadable`.

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
