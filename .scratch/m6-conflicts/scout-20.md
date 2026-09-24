# Scout 20: continue a merge begun in the shell (main 09db5433)

## graft-core journal
- `OperationKind` `crates/graft-core/src/journal.rs:117-185`: `Merge` (:144, "one entry start to finish, Pending while conflicts open"), `Pull` (:139, ff-only), `ResolveConflict`, `RestartConflict`. No `FinishMerge`.
- `JournalEntry` :318-358: `head_before/after`, `refs_before/after`, `index_before/after: Option<IndexState>`, `worktree_before/after: Option<Oid>`, `backup_ref`, `pins`, `commands`, `reverses`. No merge-state field. `MergeState` does not exist.

## Git merge files as state today
- **Nothing writes** `MERGE_HEAD`, `MERGE_MSG`, `MERGE_MODE` or `AUTO_MERGE` (grep of `crates/`). `MERGE_MODE`/`AUTO_MERGE` are never mentioned.
- Reads: `MERGE_HEAD` presence in `graft-git/src/state.rs:86` (`in_progress`) and its commit as `InProgressDetail::theirs`. `MERGE_MSG` read-only in `graft-ops/src/conflict/merge_msg.rs:26`. It is used by `finish.rs:159-163` (summary) and `conflict/set.rs` (`Conflicts:`).

## graft-ops resume
- `resume` `apply/resume.rs:167`. `resume_target` :227-253 returns `NotInProgress` when `live_entry` finds no Pending entry (:238-239). That check runs **before** the unmerged check (:245-248). So a shell merge with unmerged paths gets `NotInProgress`, not `Unmerged`.
- `live_entry` :81-127: the newest Pending entry of a matching kind, with the same `head_before` and the branch tip unmoved.
- `ContinueWith` :182-190: `git commit --cleanup=whitespace -F -`, `GIT_EDITOR=true` :200. `Continue` = `git merge --continue`.
- `Abort` with no live entry: also `NotInProgress` (:238). Ops runs no git.
- `Refusal::NotInProgress` text `operation.rs:581-585`: "nothing Graft started is in progress; an operation begun in the shell is finished there".
- The "no live entry → its own entry" rule lives in `apply` (MAP.md:155). It covers only working-tree steps such as `ResolveWholeFile`/`RestartConflict`.

## Undo / restore
- `undo::Step` `undo.rs:34-…`: `UpdateRef`, `SetHead`, `RestoreIndex`, `RestoreWorktree`, `StashStore`, `StashDrop`. No merge-state step. `inverse` :178, `plan_undo` :407, `plan_redo` :416. Executor: `undo/execute.rs:57/62`; `restore/{steps,rollback}.rs`.
- App `u` refuses while an operation is live: `graft/src/update/undo.rs:103-109` ("finish or abort the merge first").

## App
- `LiveOrigin::{ThisRun,PreviousRun,OutsideGraft}` is in the **app**, `graft/src/update/relaunch.rs:29`. It is not in ops.
- Banner `c` `graft/src/update/banner.rs:395-420`: after `continue_refusal` (:201, counts unmerged only) it already calls `open_finish(ReturnTo::Banner)` for **any** merge (:414-417), with no shell check. The refusal comes at `⏎`, from ops `NotInProgress` (ticket 35 NOTES :30, "observed").
- Commit-editor refusal: `update/commit_editor.rs:210-214` `"{kind} begun in the shell — finish it there: {cmd}"`. `Begun::InShell` means `state.live_entries.is_empty()` (:57-60).
- `d` on a done file (M6/25): `apply/restart.rs` `restart_source` :120 returns `NoStagesKept` (:127-131). `chain_states` :219-235 returns an empty list when `live_entry` finds no entry or backup. The gap row cites `restart.rs:178,:201`; those lines are a doc comment and `journal_entries`.

## Contradictions with ticket 20
1. The ticket says F5's refusal is at `operation.rs:574-575`. It is actually at `:581-585`; `:574-575` is `MarkersRemain`.
2. The ticket says the `-F` path uses `--cleanup=strip`. The code uses `--cleanup=whitespace` (`resume.rs:34`, `:185`; changed in m6/22). Acceptance tick 2 says "after strip".
3. Scope 4: "banner's `c` … no longer refuses". `c` does not refuse today (`banner.rs:414-417`). The refusal is ops' at `⏎`.
4. Scope 1: "refused in words while unmerged (`continue_refusal`)". In ops, a shell merge hits `NotInProgress` before `Unmerged` (`resume.rs:238` before `:245`). The app's `continue_refusal` does precede it.
5. `LiveOrigin::OutsideGraft` is listed under the needs from ticket 12 as if it were ops state. It lives in the app (`relaunch.rs:29`).
6. KNOWN_GAPS M6/25 line citations (`restart.rs:178,:201`) do not point at the refusal. The refusal is at `:127-131` / `:230-232`.
7. "Pull … the same way": `OperationKind::Pull` is ff-only (`journal.rs:138`). A shell `git pull` that stops is only `MERGE_HEAD` to Graft. Could not confirm there is any pull-specific path.

Could not confirm: whether `index_before`/`worktree_before` are captured correctly while `MERGE_HEAD` is live, for a non-folded entry (not traced).
