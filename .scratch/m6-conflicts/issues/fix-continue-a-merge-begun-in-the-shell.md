# fix-continue-a-merge-begun-in-the-shell: `c` finishes a merge Graft did not start — through the same finish summary — journaled as its own entry whose `u` puts the repository back mid-merge, exactly

Status: draft · **P4 approved** (user, 2026-09-23: "keep all 5"). It reverses ruling F5 and pulls TESTING.md:108 ("Times I left") from M7. The conductor writes the §2.6a row into `02-keymap.md`.
Build with: **Fable**. It meets all three tests:
1. **Structural:** a new journal kind and a new inverse step that M7's shell-begun rebase and cherry-pick will build on.
2. **Not readable from a diff:** whether `u` truly restores a *merging* repository shows only in git's state afterwards.
3. **The builder decides:** which of git's merge files are state (`MERGE_HEAD`, `MERGE_MSG`, `MERGE_MODE`, `AUTO_MERGE`?) and how they are written back safely.

Needs: `fix-landing-after-a-stop` (`LiveOrigin::OutsideGraft`, the launch landing); `fix-ops-conflict-set-and-summary` (`Resume::ContinueWith`, the `commit --cleanup=strip -F` path, `finish_summary`); `fix-finish-summary-screen` (`open_finish`, the `⏎` that sends `Continue`/`ContinueWith`).
Crates: graft-core (journal kind, before-state field), graft-ops (operation, apply/resume, restore), graft (banner `c` path, commit-editor refusal words).

## Read first
- `docs/design-notes/m6-conflicts.md`:
  - Amendment 1: Approved P4; Interfaces (the P4 block); Hard choice 3; Genuinely risky tickets.
  - Amendment 2: Journey J6b; Interfaces (`FinishMergeParams` gains `message`); Hard choice 3 (`commit -F`).
- TESTING.md:108 (the row this ticket closes) and :2122-2123.
- `crates/graft-ops/MAP.md`:
  - §apply: `resume`, `live_entry`, and the "operation begun in the shell … journaled as its own entry" rule at the `apply` bullet.
  - §restore: `execute`, the roll-back, `Step`s.
  - §journal: `undoable`, `next_undo`.
  - §operation: `starts_in_progress`.

  Also `crates/graft-core/MAP.md` §journal (entry before/after state).
- `crates/graft/MAP.md` §`update::banner` (`resume`, `resumed`), §`update::finish` (after the summary ticket), §`update::commit_editor` (`CommitEditor::refusal`, F12's words).
- **Scout questions:**
  1. What ruling F5's refusal at `graft-ops/src/operation.rs:574-575` checks, and why (the conductor has `rulings-07.md`).
  2. What `Resume::Abort` does today with no live entry.

## Scope
In:
1. **Ops: `resume` with no live Graft entry and git merging.** This covers a `git merge` or `git pull` stopped in the shell.
   - `Continue` and `ContinueWith { message }` both run `apply(Operation::FinishMerge(FinishMergeParams { merge_head, message }))`.
   - The operation is journaled as its **own** entry, with its own backup: the same "no live entry → its own entry" rule `ResolveWholeFile` follows.
   - Plan:
     - `message: None` → `git merge --continue` with `GIT_EDITOR=true` (git's own `MERGE_MSG`);
     - `Some` → the `git commit --cleanup=strip -F` path from `fix-ops-conflict-set-and-summary`.
   - Refused in words while anything is unmerged (today's `continue_refusal`).
2. **Before-state records the merge.** `MergeState` holds the bytes of `MERGE_HEAD`, `MERGE_MSG` and `MERGE_MODE` when present, on the entry. The builder decides whether `AUTO_MERGE` belongs, and documents it.
3. **Exact inverse.** `u` puts `HEAD` and the branch back on the pre-merge tip and restores the index and worktree from the backup (the resolved tree). Then `RestoreMergeState` writes git's merge files back.
   - Afterwards: `git status` says merging, `ls-files -u` is empty, the files are resolved, and `c` finishes again.
   - Redo (`U`) re-runs the commit with the same message.
   - A stop during the roll-back follows `execute`'s rule.
4. **App.**
   - The banner's `c` on a shell-begun merge with nothing unmerged now does what it does for any merge: `open_finish(ReturnTo::Banner)`. It no longer refuses.
   - The summary's `⏎` is unchanged; ops routes it.
   - F12's commit-editor refusal (`merge begun in the shell — finish it there: git commit`) becomes `a merge is in progress · c finishes it`.
   - TESTING.md:108 closes with the test name.
5. Rebase, cherry-pick and revert begun in the shell are unchanged (M7); their refusal words name that.

Out: `Abort` of a shell-begun merge (whatever it does today stays; say what in NOTES); rebase; the relaunch.

## Signatures
```rust
// graft-core/src/journal.rs
pub enum OperationKind { /* existing */ FinishMerge }
pub struct MergeState { pub merge_head: Oid, pub merge_msg: Vec<u8>, pub merge_mode: Option<Vec<u8>> }
// the entry's before-state gains: pub merge_before: Option<MergeState>
// graft-ops/src/operation.rs
pub enum Operation { /* existing */ FinishMerge(FinishMergeParams) }
pub struct FinishMergeParams { pub merge_head: Oid, pub message: Option<Vec<u8>> }
// graft-ops restore: the InversePlan step enum gains RestoreMergeState(MergeState)
// graft-ops apply::resume — signature unchanged (Resume::ContinueWith from fix-ops-conflict-set-and-summary)
```

## Acceptance (name the verifying test when you tick; truth from git, never `AppState`)
- [ ] graft-ops `tests/it/finish_merge.rs::continue_finishes_a_merge_begun_in_the_shell`: a scratch repo where `git merge feature` stops and is resolved by `checkout --theirs` + `add`. `resume(Continue)` → `HEAD^2` = `feature`'s tip, and there is one `FinishMerge` entry, `Committed`.
- [ ] `…::an_edited_message_finishes_a_shell_begun_merge`: `ContinueWith` → `git log -1 --format=%B` equals the message after strip.
- [ ] `…::undo_puts_the_merge_back_in_progress`: `u` → `git rev-parse -q --verify MERGE_HEAD` = `feature`'s tip, `MERGE_MSG` byte-equal, `HEAD` = the pre-merge tip, `ls-files -u` empty, and the worktree equals the resolved tree. `c` again → a merge commit equal in tree and parents.
- [ ] `…::a_stop_mid_undo_rolls_back` (the `execute` rule).
- [ ] `…::a_pull_begun_in_the_shell_finishes_the_same_way` (LocalRemote).
- [ ] graft `update::banner::tests::c_on_a_shell_begun_merge_opens_the_summary` and `update::commit_editor::tests::a_merge_in_progress_points_at_c` (the latter replaces F12's test).
- [ ] Journey `b-finish` at 2 keys (`c ⏎`; acceptance ticket).
- [ ] Capture `docs/shots/m6/fix-finish-shell-merge-120x40.txt`, the frame after `⏎`. **What right looks like:**
  - The cursor row is the merge commit.
  - The bottom row ends `✓`.
  - The header no longer reads `MERGING`.
  - The user judges by eye.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Medium–large; three crates. If it will not fit one context, split at the seam (ops and core first, the app second) and say so.
