# fix-continue-a-merge-begun-in-the-shell: `c` finishes a merge Graft did not start, journaled as its own entry whose `u` puts the repository back mid-merge, exactly

Status: draft · **gated on Proposed change P4** (reverses ruling F5; pulls TESTING.md:108 "Times I left" from M7). Do not file until approved.
Build with: **Fable** — meets all three tests: (1) structural — a new journal kind and a new inverse step that M7's shell-begun rebase and cherry-pick will build on; (2) whether `u` truly restores a *merging* repository is not readable from a diff, only from git's state after it; (3) the builder decides which of git's merge files are state (`MERGE_HEAD`, `MERGE_MSG`, `MERGE_MODE`, `AUTO_MERGE`?) and how they are written back safely.
Needs: `fix-landing-after-a-stop` (`LiveOrigin::OutsideGraft`, the landing J6 starts from).
Crates: graft-core (journal kind, before-state field), graft-ops (operation, apply/resume, restore), graft (banner `c` path, commit-editor refusal words).

## Read first
- `docs/design-notes/m6-conflicts.md` — Amendment: Journey J6, Interfaces (P4 block), Hard choice 3, Where I'd differ P4, Genuinely risky tickets.
- TESTING.md:108 (the row this closes) and :2122-2123.
- `crates/graft-ops/MAP.md` §apply (`resume`, `live_entry`, the "operation begun in the shell … journaled as its own entry" rule at the `apply` bullet), §restore (`execute`, the roll-back, `Step`s), §journal (`undoable`, `next_undo`), §operation (`starts_in_progress`); `crates/graft-core/MAP.md` §journal (entry before/after state).
- `crates/graft/MAP.md` §`update::banner` (`resume`, `resumed`), §`update::commit_editor` (`CommitEditor::refusal`, F12's words).
- Scout questions: what ruling F5's refusal at `graft-ops/src/operation.rs:574-575` checks, and why (the conductor has `rulings-07.md`); what `Resume::Abort` does today with no live entry.

## Scope
In:
1. **`c` on a merge with no live Graft entry** (`git merge`/`git pull` stopped in the shell): `resume(Continue)` finds no `Pending` entry and git merging → runs `apply(Operation::FinishMerge { merge_head })` — journaled as its **own** entry with its own backup, the same "no live entry → its own entry" rule `ResolveWholeFile` follows. Plan: `git merge --continue` with `GIT_EDITOR=true` (git's own `MERGE_MSG`). Refused in words while anything is unmerged (today's `continue_refusal`).
2. **Before-state records the merge**: `MergeState` (the bytes of `MERGE_HEAD`, `MERGE_MSG`, `MERGE_MODE` when present — the builder decides and documents whether `AUTO_MERGE` belongs) on the entry.
3. **Exact inverse**: `u` → `HEAD`/branch back to the pre-merge tip, index and worktree back from the backup (the resolved tree), then `RestoreMergeState` writes git's merge files back. After it: `git status` says merging, `ls-files -u` empty, files resolved; `c` again finishes. Redo (`U`) re-runs the commit. The roll-back on a stop follows `execute`'s rule.
4. **App**: the banner's `c` stops refusing for this case; F12's commit-editor refusal (`merge begun in the shell — finish it there: git commit`) is replaced by pointing at `c continue`; TESTING.md:108 closes with the test name.
5. Rebase / cherry-pick / revert begun in the shell: unchanged (M7), refusal words name that.

Out: `Abort` of a shell-begun merge (whatever it does today stays; say what in NOTES); rebase; the relaunch.

## Signatures
```rust
// graft-core/src/journal.rs
pub enum OperationKind { /* existing */ FinishMerge }
pub struct MergeState { pub merge_head: Oid, pub merge_msg: Vec<u8>, pub merge_mode: Option<Vec<u8>> }
// the entry's before-state gains: pub merge_before: Option<MergeState>
// graft-ops/src/operation.rs
pub enum Operation { /* existing */ FinishMerge(FinishMergeParams) }
pub struct FinishMergeParams { pub merge_head: Oid }
// graft-ops restore: the InversePlan step enum gains RestoreMergeState(MergeState)
// graft-ops apply::resume — signature unchanged
```

## Acceptance (name the verifying test at tick time; truth from git, never `AppState`)
- [ ] graft-ops `tests/it/finish_merge.rs::continue_finishes_a_merge_begun_in_the_shell` — scratch repo, `git merge feature` stops, resolved by `checkout --theirs`+`add`; `resume(Continue)` → `HEAD^2` = `feature`'s tip, one `FinishMerge` entry `Committed`.
- [ ] `…::undo_puts_the_merge_back_in_progress` — `u` → `git rev-parse -q --verify MERGE_HEAD` = `feature`'s tip, `MERGE_MSG` byte-equal, `HEAD` = pre-merge tip, `ls-files -u` empty, worktree = the resolved tree; `c` again → a merge commit equal in tree and parents.
- [ ] `…::a_stop_mid_undo_rolls_back` (the `execute` rule).
- [ ] `…::a_pull_begun_in_the_shell_finishes_the_same_way` (LocalRemote).
- [ ] graft `update::banner::tests::c_finishes_a_shell_begun_merge` and `update::commit_editor::tests::a_merge_begun_in_the_shell_points_at_c` (replacing F12's test).
- [ ] Journey `b-finish` at 1 key (acceptance ticket).
- [ ] Capture `docs/shots/m6/fix-finish-shell-merge-120x40.txt`. **What right looks like:** after `c`, the cursor row is the merge commit and the bottom row ends `✓`; the header no longer reads `MERGING`.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
Medium–large; three crates. If it will not fit one context, split at the seam (ops+core first, app second) and say so.
