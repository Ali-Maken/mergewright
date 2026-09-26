# 02b — rewrite engine, part b: apply, resume, record, undo, oracle

Status: done (m7-power-tools-02b). Every open box in 02 ticked there with its test; decisions, seams and gaps in 02's
"NOTES -b". Compacted once mid-ticket; nothing left over.
Blocked by: 02a (merged 66eaf8aa)
Build with: **Fable (signature)** — continuation of 02 after compaction (split along the handoff's seams; scope, signatures, model unchanged).
Crates: graft-ops, graft-core. **Not graft** (held by 04).

Spec: `02-rewrite-engine.md` — its signatures, its open acceptance items (Oracle, Record and undo, The rebase regression net,
MAP.md), and its Handoff section below, which is the plan. Every open box in 02 is ticked **in 02** with its test when
done.

Conductor rulings (2026-09-26):
- **`RebaseParams.line` / `drops` move to 05-rewrite-preview**, which already holds graft and can update the ~12 graft
  literals. 02b computes a Rebase's dependents at apply time without a new field (handoff step 3). Record this in 02 NOTES.
- **`plan` honours the app's default-branch override** (the user's configuration wins over the guess). If the override
  lives outside graft-ops/graft-core, take it as a parameter and name the signature in NOTES.
- `history_ops.rs:567-600` not inverting (lower branch inside the range): keep it, and say in NOTES which ADR-0047 case it
  is and which test covers the inverted case.

## Handoff → rewrite-engine-b

Done in -a (commit `m7-power-tools-02a`):
- **The `rewrite` module:** types, `plan`, `dependents`, `progress`, the pure functions, the saved-plan format and
  helper (`saved.rs`), and `git_todo`.
- **Refusals:** the new `Refusal` variants and their words.
- **The new kind:** `OperationKind::Rewrite`; `opens_in_progress(Rewrite)` → `Rebase`; `kind_name` → `rewrite`.
- **Errors:** the new `FactsError` variants.

Next, in order:
1. **`Operation::Rewrite(RewriteParams)`.** Wire every `operation.rs` site:
   - `kind`, `is_history`, `sets_aside`, `tips` (head + line), `facts_scope` (line names);
   - `describe` (`rewrite <branch>` / `put staged changes into <short>`);
   - `check` (on `head.name`, tip CAS → `Moved`, `validate` → `BadTodo`), `plan`, `touches` (line refs + stash if
     set aside, index, worktree).
   - `record.rs`: `pins_for` → `made`. `preview::plan` falls to `NotPreviewable`.
2. **`plan_rebase` → `git rebase -i [--no-update-refs] --no-autostash <onto> [<top short name>]`.**
   - Keep the `git rebase ` prefix: graft tests `m5_walk/preview.rs:75` and `tests/it/preview.rs:216` assert it.
   - Pass `--no-update-refs` only when git ≥ 2.38; `<top>` must be the **short** name (a full ref detaches).
   - Update `operation/history/tests.rs:322`.
   - Fix the stale comment at `history.rs:613`.
3. **`apply/engine.rs`.**
   - Build the line: `dependents` for Rebase, `params.line` for Rewrite.
   - Build the todo: `replayed_commits` picks + `line_picks` + `git_todo`.
   - `saved::save` to `saved::dir(repo, ctx.next_id())` before `begin`; remove it if `begin` fails.
   - Run env: `GIT_SEQUENCE_EDITOR=saved::sequence_editor`, `GIT_EDITOR=saved::editor`, `CLEANUP`.
   - After a finish with top ≠ home: `git switch <home short>`.
   - Exit 0 while `rebase-merge/` is still live is an edit stop → `Conflicted{paths: []}`.
   - Remove the plan dir on close; a failed removal is a new `AfterState` (e.g. `PlanLeft`).
4. **Resume.**
   - `live_entry` accepts `head-name` == the saved plan's top.
   - Continue/Skip pass `GIT_EDITOR` + `CLEANUP`.
   - Exit 0 with the rebase still live → Conflicted.
   - After a finish or abort with top ≠ home, switch home before `close_resume`.
   - Remove the plan dir.
   - `recover_interrupted` removes the plan dirs of the entries it closes.
5. **The `-c` question.** Resume needs no `-c rebase.updateRefs`: it affects only todo generation, and the todo is
   already written.
   - Prove it: `rebase.updateRefs=true` in the repo config, an edit stop, `resume(Continue)`, and a branch inside the
     range stays unmoved.
6. **Tests:** the oracle file (10 rows plus an unsaved-edit row, with a `cp -R` TempRepo helper), record/undo/abort/kill,
   the plan-dir removal test, and a `#123` reword.

Gotchas:
- In a worktree-isolated agent, inline git in temp dirs is refused. Put experiments in a script file and run `sh`.
- The helper script embeds `repo.git_dir()`. Check it is absolute.
- Check that `--continue` after a conflicted pick opens the editor (the helper must answer it).
