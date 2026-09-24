# fix-continue-a-merge-begun-in-the-shell: `c` finishes a merge Graft did not start — through the same finish summary — journaled as its own entry whose `u` puts the repository back mid-merge, exactly

Status: ready — respecified 2026-09-24 (spec-only pass) against `scout-20.md` (main `09db5433`), which found 7 contradictions with the previous spec. Each is resolved below and marked **(S-n)** for scout item n. P4 approved by the user 2026-09-23 ("keep all 5"). It reverses ruling F5 (`Refusal::NotInProgress`, `graft-ops/src/operation.rs:581-585` — **S-1**; the old `:574-575` is `MarkersRemain`) and pulls TESTING.md:108 ("Times I left") from M7. The keymap already carries P4 (`02-keymap.md:294`), so there is no keymap edit.
Blocked by: 09, 12, 34 + 35, 36 (all merged). **Part B also waits for 31** (same crate `graft`, ADR-0036). Part A may build while 31 does, because its crates are disjoint from 31's.
Build with: **Fable**. It meets all three tests:
1. **Structural:** a new journal kind, a merge-state field on every entry, and a new inverse step. M7's shell-begun rebase and cherry-pick build on all three.
2. **Not readable from a diff:** whether `u` truly restores a *merging* repository shows only in git's state afterwards.
3. **The builder decides:** whether `AUTO_MERGE` is state, how git's merge files are written back safely, and which of the two allowed routes the edited message takes to git (Scope A1).

Needs: `graft_ops::apply::{resume, Resume::ContinueWith}` (09); `graft_ops::finish::finish_summary`, and the private rule that computes `FinishSummary::from` (09); `graft::update::finish::{open_finish, resume_answered, FinishAnswer}` (34/35); `graft::update::banner::continue_pressed` (31 gives it a `from: ReturnTo` parameter — use whatever signature is merged). It does **not** need `LiveOrigin`: that is app state (`graft/src/update/relaunch.rs:29`), used only by the landing's words, and nothing here reads it **(S-5)**.
Crates: Part A — graft-core, graft-ops. Part B — graft.

## Read first
- `.scratch/m6-conflicts/scout-20.md`, all of it (1 page). It answers the old scout questions: F5's check, and `Abort` with no live entry (`NotInProgress`, and ops runs no git).
- `docs/design-notes/m6-conflicts.md`:
  - Amendment 1: Hard choice 3 (:345-347).
  - Amendment 2: Journey J6b (:461); "Where each event lands him" (:467-481); Interfaces `Resume` (:559-562); Hard choice 3 (:610-612 — its `--cleanup=strip` is stale, see S-2).
- TESTING.md:108 and :2121-2123.
- `crates/graft-ops/MAP.md`:
  - §apply: the `Resume::ContinueWith` bullet, the `Resume` bullet, the `Applied` bullet, the `apply` bullet (the "no live entry → its own entry" rule), `stage_source`, `recover_interrupted`.
  - §undo: `Step`, `IndexPaths`, `InversePlan`, `inverse`.
  - §restore: `execute` and its two "the stop" / "killed inside" bullets.
  - §snapshot: `take` (the parents it anchors).
  - §finish: `FinishSummary`.
- `crates/graft-core/MAP.md` §journal: `OperationKind`, `JournalEntry`, `IndexState`.
- Part B only: `crates/graft/MAP.md` §`update::banner` (`continue_refusal`, `banner_key`, `resumed`), §`update::finish` (`open_finish`, `resume_answered`, `FinishAnswer`), §`update::commit_editor` (`CommitEditor::refusal`, `Begun`, `live_operation`), §`update::undo` (`in_progress_refusal`).

## Scope

### Part A — graft-core, graft-ops

**A0. Write this test first: `tests/it/finish_merge.rs::undo_after_c_puts_back_exactly_what_was_there_before_c`.** It fails until A1–A4 exist.
- Setup: a scratch repo where `git merge feature` stops. Resolve it with `checkout --theirs` + `add`.
- Record, before `c`:
  - `git status --porcelain=v2 --branch` (bytes);
  - `git ls-files --stage` (bytes);
  - the bytes of `MERGE_HEAD`, `MERGE_MSG` and `MERGE_MODE`, each read at `git rev-parse --git-path <name>`;
  - every tracked file's bytes.
- Then run `resume(Continue)`, then `undo`.
- Assert every one of those records is byte-equal to its before-`c` value.

The scout could not confirm that `index_before` and `worktree_before` are captured correctly while `MERGE_HEAD` is live, for an entry that is not folded. **Don't reason forward from it; this test is the check.** If the index or worktree half differs, and the cause is the before-state capture (`apply`'s `begin`, or `snapshot::take`), you must **stop**. Report `blocked: before-state while merging: <function> — <the differing record>`. Don't compensate in the restore. A fix that changes the backup commit's layout is hard to reverse; that's the conductor's call.

**A1. `resume` with no live entry, while git is merging (S-4).** The routing lives in `resume_target`. New order:
1. `ContinueWith` on a non-merge → `MessageNotHere`. Unchanged, still first.
2. A live entry is found → the path as today, unchanged.
3. No live entry, the action is `Continue`/`ContinueWith`, and git is merging with **one** `MERGE_HEAD` line:
   1. Run the **unmerged check first**. The refusal is `Unmerged`, with the same variant and words as the live-entry path. Today `resume_target` returns `NotInProgress` for a shell merge before it ever reaches its unmerged check. That's the bug.
   2. Then run `apply(ctx, Operation::FinishMerge(FinishMergeParams { merge_head, message }))`. `message` is `None` for `Continue` and `Some` for `ContinueWith`.
4. Everything else → `NotInProgress`, **with the words unchanged**: `Abort`/`Skip` with no live entry, any shell-begun rebase, cherry-pick or revert, an octopus merge, and nothing in progress. The words stay because they are still true for every case that reaches them. So the old Scope 5 ("their words name M7") is dropped.

`FinishMerge` in `apply`:
- **Always its own entry, with its own backup.** It touches the branch (or detached `HEAD`), the index and the worktree, so the "no live entry → its own entry" rule covers it.
- `check` refuses it with `NotInProgress` unless git is merging with `MERGE_HEAD` equal to `params.merge_head`.
- `opens_in_progress(FinishMerge)` is `None`. So `live_entry` never matches it, and `recover_interrupted` closes a crashed one as interrupted: it is not in force, because nothing changed.
- `journal::undoable` says yes. Serde name: `finish_merge`.
- The description is `finish merge of <from>`, where `<from>` is exactly `FinishSummary::from`'s rule. **Share that function; don't copy it.**

The plan:
- `message: None` → `git merge --continue` with `GIT_EDITOR=true`. This is git's own `MERGE_MSG`, byte for byte, the same as today's `Continue`.
- `Some(m)` → `git commit --cleanup=whitespace -F …` with `GIT_EDITOR=true`. **(S-2)** The mode is `whitespace`, not `strip`: it's the mode the live-entry `ContinueWith` runs (`resume`, since m6-conflicts/22). `finish_summary` has already removed git's comment lines, so a `#` line in `m` is the user's and must survive. Both routes to git use one cleanup rule.
- **`Invocation` doesn't grow stdin.** `restore_stages` in the MAP is the precedent: plans stay pure. The message reaches git by one of two routes, and you pick:
  - `-F <file under ctx.scratch>`, removed afterwards; or
  - a crate-private run step, like `run_restart_prep`.

  Either way, its command line goes into the entry's Command Log, and NOTES say which you chose. **`MERGE_MSG` is never written by the forward path.**
- A hook refusing, or git failing: the entry is `Aborted`, not in force, and the repo is still merging. The error is the one `apply` returns today for a failed git step.

**`MERGE_AUTOSTASH` — stop condition.** First check in a scratch repo (`git merge --autostash` with a dirty tree that stops) whether `git commit` / `git merge --continue` apply `MERGE_AUTOSTASH`. The belief that they do is **unverified**.
- If they do: `FinishMerge` refuses while `MERGE_AUTOSTASH` exists, as `NotInProgress` (its words hold). Write a test, and put the observation in NOTES.
- If they don't: say so in NOTES, and no refusal.

**A2. Merge state on every entry.**
- `begin` records `merge_before` and `finish` records `merge_after`, for **every** entry. Each is `None` when git is not merging.
- Absent on an old line → `None`. That's safe, because A3 only acts when the two sides differ.
- Reading the state fails → `AfterState::MergeState` (new), never swallowed.
- `snapshot::take` anchors `merge_before.merge_head` (and `merge_after`'s) among the backup commit's parents. The reason: a `git pull <url> <branch>`'s `MERGE_HEAD` may be reachable only from `FETCH_HEAD`, and `u` must still be able to write it back after `gc`.
- `MergeState` holds:
  - `MERGE_HEAD`'s single oid;
  - `MERGE_MSG`'s exact bytes;
  - `MERGE_MODE`'s bytes when the file exists. An empty file is `Some(vec![])`.
- You decide whether `AUTO_MERGE` belongs. Either add `pub auto_merge: Option<Oid>` and anchor it (it names a tree, so work out how to anchor one), or leave it out and say why in `MergeState`'s doc comment.
- The JSONL encoding of the bytes is yours to choose. It must round-trip non-UTF-8 bytes exactly, which needs a test.

**A3. The inverse: `Step::SetMergeState(Option<MergeState>)`.**
- It replaces the note's `RestoreMergeState(MergeState)`. The reason: the undo entry's own inverse (the redo) has to **remove** the files, and a roll-back of either direction is the same step aimed at the entry's own `merge_before`. So one step serves both ways, like `SetHead`.
- `inverse` emits `SetMergeState(entry.merge_before)` when `merge_before != merge_after`. It goes **last** in `steps`, after the worktree.
  - `Some` writes each file at `git rev-parse --git-path <name>`: temp file, then rename. Afterwards `git status` must report merging; if it doesn't, the step fails.
  - `None` removes them. Afterwards `git status` must not report merging; if it does, the step fails.
- **Undo** (from the generic steps): `UpdateRef` puts the branch (or `SetHead` puts detached `HEAD`) back on the pre-merge tip; the index and worktree come back from the backup over `ChangedBetween`; then `SetMergeState(Some)`.
- **Redo** is the generic redo: the branch goes back to **the same merge-commit oid**, then `SetMergeState(None)`. This replaces the old "re-runs the commit". A state-based redo (ADR-0031) runs no hook and makes no new commit.
- **Drift (§6.7):** `undo`/`redo` compare the merge state now against `merge_after`. A difference is drift, like any other field. Example: after `c`, a new `git merge` was started in the shell. Put it in the existing `Drift` type as a new field, and name it in NOTES.
- **`execute`'s stop:** a stop before `SetMergeState` rolls back what ran. The step's own reversal is `SetMergeState(<this entry's merge_before>)`. `NoBefore` doesn't apply, because `None` is a recorded state here.
- `git`'s resolve-undo (REUC) extension is **not** restored. `stage_source` rejects REUC (MAP). Test after `u` whether `git checkout -m <path>` still rebuilds the conflict, and record what you see in NOTES.

**A4. Pull (S-7): in scope, with no pull path.** A `git pull` (merge mode) that stops is only `MERGE_HEAD` + `MERGE_MSG` (`Merge branch 'x' of <url>`) to git. `OperationKind::Pull` is ff-only and isn't involved. The fetch's `refs/remotes/*` move belongs to the shell and isn't in the `FinishMerge` entry, so `u` leaves it alone. A `git pull --rebase` that stops is a rebase: `NotInProgress`, M7.

**A5. KNOWN_GAPS M6/25 (`d` on a done file of a shell-begun merge): not fixed here.**
- The refusals are at `apply/restart.rs:127-131` (`restart_source` → `NoStagesKept`) and `:230-232` (`chain_states`: no live entry → empty).
- Why it's not fixed here: in a shell merge, `⏎`'s stages are kept **nowhere**. An index-only step there folds into nothing (MAP §apply `apply`). Fixing it means:
  1. journal every index step of a shell-begun operation as its own entry;
  2. bound `chain_states`/`restart_source` to *this* merge's entries;
  3. change 28's `restart_gate`.

  That's J11's journey, not finishing.
- What A2 contributes: `merge_before` on every entry **is** that bound (entries whose `merge_before.merge_head` is the current `MERGE_HEAD` and whose `head_before` is `HEAD`). Say so in the `JournalEntry` doc comment. Don't build the fix.

### Part B — graft (after 31 merges)

**B1. Banner `c` (S-3): no code change.** `banner::continue_pressed` already calls `open_finish(.., ReturnTo::Banner, ..)` for every merge, with no shell check (scout-20). The refusal came from ops at `⏎` (`resume_target`), and A1 removes it. Pin it with tests: `c` → the summary, then `⏎` → `Resume::Continue`. A1 routes that.

**B2. `CommitEditor::refusal`, `Begun::InShell` with a merge:**
- The words become `merge begun in the shell — esc, then c finishes it`.
- Rebase, pick and revert `InShell` keep `{kind} begun in the shell — finish it there: {cmd}`.
- No `a abort`: `a` still refuses on a shell merge (KNOWN_GAPS M6/08, question, M7).
- **The words must be true.** From the state J3 leaves (a launch into a shell merge, resolved, the commit editor opened from the changes pane), pressing exactly `Esc` then `c` must reach `finish.is_some()`. If it doesn't, **stop**: report `blocked: <the words name keys that don't reach open_finish>` with the function that swallows `c`. Don't reword to fit.

**B3. Landings (design note :477-481; the additions go to the conductor below).**
- `⏎` on a shell-begun merge lands like J6: graph focused, cursor on the merge commit, status `merged feature into main ✓`, and `finish` is `None`.
- A hook refusing → the summary stays, git's words go above the bar, the draft is kept, and git is still merging. That's `resume_answered`'s `Failed` path.
- `u` after that →
  - status `Done` with the undo entry's description (`undo: finish merge of feature`);
  - `in_progress` is `Some(Merge)` after the refresh;
  - the header reads `MERGING` and the banner is drawn;
  - focus is unchanged;
  - `stopped` isn't set (KNOWN_GAPS M6/12 row :637, M7).

  If what the build shows differs, record it in NOTES and stop; don't special-case it.
- `U` while merging keeps `in_progress_refusal`: the gate is unchanged. Finishing again is `c ⏎`.

**Out:**
- `Abort` of a shell-begun merge: `NotInProgress`, and ops runs no git (scout-20). That's gap M6/08.
- Shell-begun rebase, cherry-pick and revert (M7).
- Octopus merges (`NotInProgress`, as today).
- The relaunch.
- M6/25 (A5).
- The app's `U` gate while merging.
- REUC (A3, observed only).

## Signatures
```rust
// graft-core/src/journal.rs
pub enum OperationKind { /* existing */ FinishMerge }              // serde "finish_merge"
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeState {
    pub merge_head: Oid,              // MERGE_HEAD's one line
    pub merge_msg: Vec<u8>,           // exact bytes
    pub merge_mode: Option<Vec<u8>>,  // None = no file; Some(vec![]) = empty file
    // AUTO_MERGE: `pub auto_merge: Option<Oid>` or a doc comment saying why not (builder, A2)
}
pub struct JournalEntry { /* existing */
    #[serde(default)] pub merge_before: Option<MergeState>,   // None = not merging, or a line older than this field
    #[serde(default)] pub merge_after: Option<MergeState>,
}

// graft-ops/src/operation.rs
pub enum Operation { /* existing */ FinishMerge(FinishMergeParams) }
pub struct FinishMergeParams { pub merge_head: Oid, pub message: Option<Vec<u8>> }

// graft-ops/src/undo.rs
pub enum Step { /* existing */ SetMergeState(Option<MergeState>) }   // always last in InversePlan::steps
// graft-ops/src/apply/error.rs
pub enum AfterState { /* existing */ MergeState }
// the Drift type (§6.7 compare) gains one merge-state field; name it in NOTES
// crate-private, graft-ops (snapshot or apply/record — builder's choice):
pub(crate) fn read_merge_state(repo: &RepoHandle) -> Result<Option<MergeState>, /* the module's error */>;

// graft-ops/src/apply/resume.rs — `resume` signature unchanged; `resume_target`'s order per A1
// graft/src/update/commit_editor.rs — `CommitEditor::refusal` signature unchanged; words per B2
```

## Acceptance (name the verifying test when you tick; truth from git, never `AppState`, except B's landings)
Part A, graft-ops `tests/it/finish_merge.rs`:
- [ ] `undo_after_c_puts_back_exactly_what_was_there_before_c` (A0): every record byte-equal. **First written, last to go green.**
- [ ] `continue_finishes_a_merge_begun_in_the_shell`: `HEAD^2` = `feature`'s tip, `MERGE_HEAD` absent, exactly one new entry, kind `FinishMerge`, `Committed`, with `merge_before` = the state read before and `merge_after` = `None`.
- [ ] `an_unmerged_shell_merge_is_refused_unmerged_not_not_in_progress` (S-4): `Refused(Unmerged)`, no entry, no git run.
- [ ] `an_edited_message_finishes_a_shell_begun_merge` (S-2): send `"Subject  \n\n# kept\nbody\n\n\n"`. `git log -1 --format=%B` must equal git's `whitespace` cleanup of it: trailing spaces and trailing blank lines gone, and `# kept` **present**.
- [ ] `an_unedited_finish_keeps_gits_message`: `%B` equals `MERGE_MSG`'s non-comment lines, as `git merge --continue` writes them.
- [ ] `redo_after_undo_lands_on_the_same_merge_commit`: the branch is at the **same oid**, and `MERGE_HEAD`/`MERGE_MSG`/`MERGE_MODE` are absent.
- [ ] `c_again_after_undo_makes_an_equal_merge`: same tree and parents.
- [ ] `a_stop_before_set_merge_state_rolls_back`: `HEAD` = the merge commit, no `MERGE_HEAD`, entry `Aborted("cancelled")`, not in force.
- [ ] `undo_refuses_drift_when_a_new_merge_was_begun`: `Drift`, nothing written.
- [ ] `a_hook_refusal_leaves_the_merge_in_progress`: `MERGE_HEAD` present, entry `Aborted`, not in force.
- [ ] `a_pull_begun_in_the_shell_finishes_the_same_way` (LocalRemote, `pull.rebase=false`): `HEAD^2` = the fetched tip. After `u`: `MERGE_HEAD` back, `refs/remotes/origin/main` unmoved by `u`.
- [ ] `merge_state_round_trips_non_utf8_bytes` (the journal line).
- [ ] `the_merge_head_commit_is_anchored`: the `FinishMerge` backup commit's parents include `MERGE_HEAD`.
- [ ] The `MERGE_AUTOSTASH` test or the NOTES observation (A1), whichever the check decided.

Part B, graft:
- [ ] `update::banner::tests::c_on_a_shell_begun_merge_opens_the_summary` (B1): `finish` is `Some`, `return_to == Banner`, no `Resume` pushed.
- [ ] `update::commit_editor::tests::a_merge_in_progress_points_at_c` replaces `a_merge_begun_in_the_shell_names_no_key_that_refuses_it`: the words are exact, and they contain neither `git commit` nor `a abort`.
- [ ] `tests/it/resolver/shell_begun.rs::the_keys_the_commit_editor_names_reach_the_summary` (B2).
- [ ] `tests/it/resolver/shell_begun.rs::c_enter_finishes_a_shell_begun_merge`: two key presses from the resolved banner. git: `HEAD^2` = `feature`. App: B3's first landing. This is the `b-finish` count of 2.
- [ ] `…::u_after_a_shell_begun_finish_puts_the_merge_back`: git has `MERGE_HEAD` back. App: B3's `u` landing.
- [ ] `…::a_hook_refusal_keeps_the_summary_on_a_shell_begun_merge`.
- [ ] Captures (rule 6; colour **unverified**, rule 5):
  - `docs/shots/m6/fix-finish-shell-merge-120x40.txt`, the frame after `⏎`. **What right looks like:** the cursor row is the merge commit, the bottom row ends `merged feature into main ✓`, and the header doesn't read `MERGING`.
  - `docs/shots/m6/fix-undo-shell-merge-120x40.txt`, the frame after `u`. **What right looks like:** the header reads `MERGING`, the status reads `undo: finish merge of feature`, and the graph has no merge commit above `main`'s pre-merge tip.

  The user judges by eye.
- [ ] NOTES: the message route (A1); `AUTO_MERGE` (A2); the drift field (A3); REUC (A3); `MERGE_AUTOSTASH` (A1); and B3's landings as observed.
- [ ] MAP.md: graft-core §journal; graft-ops §apply, §undo, §restore, §snapshot; graft §`update::commit_editor`.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Large. Split at the seam: **Part A** (graft-core, graft-ops) is one context, **Part B** (graft) another. If adding `OperationKind::FinishMerge` or `Operation::FinishMerge` breaks a `match` in `graft` while 31 holds it, commit Part A at the seam, report `blocked: needs graft — match arms for OperationKind::FinishMerge / Operation::FinishMerge`, and stop. Part B's first job is then those arms.

## For the conductor (exact text, after this ticket merges; fill `<test>` from the tick)
- `docs/design-notes/m6-conflicts.md` :335-336, replace with:
  `pub struct MergeState { pub merge_head: Oid, pub merge_msg: Vec<u8>, pub merge_mode: Option<Vec<u8>> }  // JournalEntry gains merge_before / merge_after, recorded on every entry`
  `// undo: Step::SetMergeState(Option<MergeState>) — last step; Some writes MERGE_HEAD/MERGE_MSG/MERGE_MODE back, None removes them (redo) (ticket 20, spec pass 2026-09-24)`
- The same note, :611, replace `` `git commit --cleanup=strip -F` `` with `` `git commit --cleanup=whitespace -F` (m6-conflicts/22) ``.
- The same note, after :481, add:
  `- \`⏎\` on a merge begun in the shell → as J6; one \`FinishMerge\` entry. \`u\` after it → the merge is back in progress, resolved; header \`MERGING\`, status \`undo: finish merge of <from>\`, focus unchanged; \`c ⏎\` finishes it again (ticket 20).`
- `KNOWN_GAPS.md` row M6/25: replace `Ticket 20 (continue a shell-begun merge) is the natural owner.` with `Not fixed by ticket 20: a shell merge's \`⏎\` keeps its stages nowhere (an index-only step folds into nothing). The fix journals those steps and bounds \`chain_states\` by \`merge_before\` (ticket 20 records it on every entry).`, and the target column `M6 (ticket 20)` with `M7`.
- `TESTING.md:108`: status `open (M7)` → `closed (m6-conflicts, <test>)`. Replace the text after the task with `M6: **0** — \`c ⏎\` on the banner (P4; \`crates/graft/tests/it/resolver/shell_begun.rs::c_enter_finishes_a_shell_begun_merge\`). Was: the shell, ruling F5 (\`graft-ops/src/operation.rs:581-585\`).`
- `TESTING.md:2121-2123`, replace `A merge **begun in the shell** is not finished in the app — neither the banner's \`c\` nor the changes pane's commit editor will (Times I left).` with `A merge **begun in the shell** finishes through the banner's \`c\` and the summary, journaled as its own entry that \`u\` puts back mid-merge; the commit editor still refuses, naming \`esc, then c\`.`
