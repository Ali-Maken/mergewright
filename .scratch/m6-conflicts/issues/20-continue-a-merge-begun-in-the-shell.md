# fix-continue-a-merge-begun-in-the-shell: `c` finishes a merge Graft did not start — through the same finish summary — journaled as its own entry whose `u` puts the repository back mid-merge, exactly

Status: **Part A done** (commit `m6-conflicts-20: Part A …`); **Part B partial**: B1, B3, captures done; **B2 blocked** (its stop condition — `esc, then c` from the commit editor reopens the editor; see NOTES Part B). Respecified 2026-09-24 (spec-only pass) against `scout-20.md` (main `09db5433`), which found 7 contradictions with the previous spec. Each is resolved below and marked **(S-n)** for scout item n. P4 approved by the user 2026-09-23 ("keep all 5"). It reverses ruling F5 (`Refusal::NotInProgress`, `graft-ops/src/operation.rs:581-585` — **S-1**; the old `:574-575` is `MarkersRemain`) and pulls TESTING.md:108 ("Times I left") from M7. The keymap already carries P4 (`02-keymap.md:294`), so there is no keymap edit.
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
- [x] `undo_after_c_puts_back_exactly_what_was_there_before_c` (A0): every record byte-equal. **First written, last to go green.** — verified by `crates/graft-ops/tests/it/finish_merge.rs::undo_after_c_puts_back_exactly_what_was_there_before_c` (status v2, `ls-files --stage`, the three files at `--git-path`, every tracked file: all byte-equal). Honest order: written in the same pass as A1–A4, after the code, and green on its first run — so no before-state blocker; `begin`/`snapshot::take` capture correctly while `MERGE_HEAD` is live.
- [x] `continue_finishes_a_merge_begun_in_the_shell` (`crates/graft-ops/tests/it/finish_merge.rs::continue_finishes_a_merge_begun_in_the_shell`; also asserts description `finish merge of feature` and the Command Log line): `HEAD^2` = `feature`'s tip, `MERGE_HEAD` absent, exactly one new entry, kind `FinishMerge`, `Committed`, with `merge_before` = the state read before and `merge_after` = `None`.
- [x] `an_unmerged_shell_merge_is_refused_unmerged_not_not_in_progress` (S-4): `Refused(Unmerged)`, no entry, no git run. — `crates/graft-ops/tests/it/finish_merge.rs::an_unmerged_shell_merge_is_refused_unmerged_not_not_in_progress` (Continue and ContinueWith). "No git run" is observed as: no entry, `HEAD` unmoved, still merging, `MERGE_MSG` bytes unchanged; the refusal returns before `apply` is called (`apply/resume.rs` `shell_merge` → `refuse_unmerged`).
- [x] `an_edited_message_finishes_a_shell_begun_merge` (S-2; `crates/graft-ops/tests/it/finish_merge.rs::an_edited_message_finishes_a_shell_begun_merge`, compared with `git stripspace` and the literal `Subject\n\n# kept\nbody\n`): send `"Subject  \n\n# kept\nbody\n\n\n"`. `git log -1 --format=%B` must equal git's `whitespace` cleanup of it: trailing spaces and trailing blank lines gone, and `# kept` **present**.
- [x] `an_unedited_finish_keeps_gits_message` (`crates/graft-ops/tests/it/finish_merge.rs::an_unedited_finish_keeps_gits_message`, against `git stripspace --strip-comments` of `MERGE_MSG`): `%B` equals `MERGE_MSG`'s non-comment lines, as `git merge --continue` writes them.
- [x] `redo_after_undo_lands_on_the_same_merge_commit` (`crates/graft-ops/tests/it/finish_merge.rs::redo_after_undo_lands_on_the_same_merge_commit`): the branch is at the **same oid**, and `MERGE_HEAD`/`MERGE_MSG`/`MERGE_MODE` are absent.
- [x] `c_again_after_undo_makes_an_equal_merge` (`crates/graft-ops/tests/it/finish_merge.rs::c_again_after_undo_makes_an_equal_merge`): same tree and parents.
- [x] `a_stop_before_set_merge_state_rolls_back` (`crates/graft-ops/tests/it/finish_merge.rs::a_stop_before_set_merge_state_rolls_back`: a latched stop keyed to `refs/heads/main` reading the pre-merge tip, through `restore::execute`; also asserts `SetMergeState(Some)` is the plan's last step): `HEAD` = the merge commit, no `MERGE_HEAD`, entry `Aborted("cancelled")`, not in force.
- [x] `undo_refuses_drift_when_a_new_merge_was_begun` (`crates/graft-ops/tests/it/finish_merge.rs::undo_refuses_drift_when_a_new_merge_was_begun`: a `--no-ff --no-commit` merge of an empty commit, so only the merge state moves; `Drift::merge == Some(MergeDrift::Begun)`, every other field clean; unit: `undo::tests::drift_names_the_way_the_merge_state_moved`): `Drift`, nothing written.
- [x] `a_hook_refusal_leaves_the_merge_in_progress` (`crates/graft-ops/tests/it/finish_merge.rs::a_hook_refusal_leaves_the_merge_in_progress`: a `pre-commit` hook via `core.hooksPath`; `ApplyError::Git`): `MERGE_HEAD` present, entry `Aborted`, not in force.
- [x] `a_pull_begun_in_the_shell_finishes_the_same_way` (LocalRemote, `pull.rebase=false`): `HEAD^2` = the fetched tip. After `u`: `MERGE_HEAD` back, `refs/remotes/origin/main` unmoved by `u`. — `crates/graft-ops/tests/it/finish_merge.rs::a_pull_begun_in_the_shell_finishes_the_same_way`. **Fixture deviation:** not `graft_fixtures::LocalRemote`; the existing `conflict_fixtures::downstream_of` + `remote add origin` + `pull.rebase=false`, `git pull origin feature` — so the tracking ref is `refs/remotes/origin/feature`, and the assertion is that **every** `refs/remotes/*` is unmoved by `u`. Also byte-equal A0 records after `u`.
- [x] `merge_state_round_trips_non_utf8_bytes` (the journal line). — `crates/graft-ops/tests/it/finish_merge.rs::merge_state_round_trips_non_utf8_bytes` (`MERGE_MSG` with `\xff\xfe`, `MERGE_MODE` `no-ff`, read back by a fresh `Journal::open`, then `u` writes the bytes back exact); unit: `graft_core::journal::tests::merge_state_bytes_are_text_when_they_can_be_and_exact_always`.
- [x] `the_merge_head_commit_is_anchored` (`crates/graft-ops/tests/it/finish_merge.rs::the_merge_head_commit_is_anchored`: pull from a URL, `<backup>^1^@` holds `MERGE_HEAD`; then `FETCH_HEAD` removed, reflogs expired, `gc --prune=now`, and `u` still writes `MERGE_HEAD` back): the `FinishMerge` backup commit's parents include `MERGE_HEAD`.
- [x] The `MERGE_AUTOSTASH` test or the NOTES observation (A1), whichever the check decided. — git applies it (observed: `crates/graft-ops/tests/it/finish_merge.rs::git_applies_merge_autostash_when_the_merge_is_finished`), so the refusal: `crates/graft-ops/tests/it/finish_merge.rs::c_is_refused_while_merge_autostash_waits`.

Part B, graft:
- [x] `update::banner::tests::c_on_a_shell_begun_merge_opens_the_summary` (B1): `finish` is `Some`, `return_to == Banner`, no `Resume` pushed. — `crates/graft/src/update/banner/tests.rs::c_on_a_shell_begun_merge_opens_the_summary` (also asserts `live_operation` = `(Merge, InShell)` and one `ReadFinishSummary`). The `⏎ → Resume::Continue → FinishMerge` half is pinned end to end by `c_enter_finishes_a_shell_begun_merge` below.
- [ ] `update::commit_editor::tests::a_merge_in_progress_points_at_c` replaces `a_merge_begun_in_the_shell_names_no_key_that_refuses_it`: the words are exact, and they contain neither `git commit` nor `a abort`. — **BLOCKED (B2 stop condition hit, see NOTES Part B).** Words unchanged; `a_merge_begun_in_the_shell_names_no_key_that_refuses_it` kept and made exact (`merge begun in the shell — finish it there: git commit`); new `a_rebase_begun_in_the_shell_still_names_gits_command` pins the rebase words.
- [ ] `tests/it/resolver/shell_begun.rs::the_keys_the_commit_editor_names_reach_the_summary` (B2). — **BLOCKED**: written, run, failed on `finish.is_some()` (the editor reopened); removed from the tree, not disabled. Its body is in NOTES Part B's repro.
- [x] `tests/it/resolver/shell_begun.rs::c_enter_finishes_a_shell_begun_merge`: two key presses from the resolved banner. git: `HEAD^2` = `feature`. App: B3's first landing. This is the `b-finish` count of 2. — `crates/graft/tests/it/resolver/shell_begun.rs::c_enter_finishes_a_shell_begun_merge` (`c`, `⏎` from the focused banner; git `HEAD^2` = `feature`, no `MERGE_HEAD`, one new `FinishMerge` entry; app: `finish` `None`, graph focused, cursor on `HEAD`, `Status::Done("merged feature into main")`).
- [x] `…::u_after_a_shell_begun_finish_puts_the_merge_back`: git has `MERGE_HEAD` back. App: B3's `u` landing. — `crates/graft/tests/it/resolver/shell_begun.rs::u_after_a_shell_begun_finish_puts_the_merge_back` (`MERGE_HEAD` = `feature`, `main` = its pre-merge tip; `Status::Done("undo: finish merge of feature")`, `in_progress` = `Merge`, `banner_view` `Some`, header ends `MERGING`, `graph_has_focus`/`banner_focus` equal to before `u`, `stopped` `None`).
- [x] `…::a_hook_refusal_keeps_the_summary_on_a_shell_begun_merge`. — `crates/graft/tests/it/resolver/shell_begun.rs::a_hook_refusal_keeps_the_summary_on_a_shell_begun_merge` (`commit-msg` hook, edited subject so `ContinueWith`: notice is a problem with the hook's words, subject kept, `MERGE_HEAD` present, the new entry `FinishMerge` + `Aborted`).
- [x] Captures (rule 6; colour **unverified**, rule 5): — both written and checked by the two tests above (`check_shot`); merge commit id and author date zeroed (made now, new every run).
  - `docs/shots/m6/fix-finish-shell-merge-120x40.txt`, the frame after `⏎`. **What right looks like:** the cursor row is the merge commit, the bottom row ends `merged feature into main ✓`, and the header doesn't read `MERGING`. — **Wording drift:** the status is the graph's **footer** row, not the frame's bottom row: the landing opens the detail pane below it (as `fix-landing-after-continue-rebase`). The test asserts the footer row. The age column reads `futu…` because the capture's clock is fixed in 2021 and the merge is made now — same as the rebase capture.
  - `docs/shots/m6/fix-undo-shell-merge-120x40.txt`, the frame after `u`. **What right looks like:** the header reads `MERGING`, the status reads `undo: finish merge of feature`, and the graph has no merge commit above `main`'s pre-merge tip.

  The user judges by eye.
- [x] NOTES: the message route (A1); `AUTO_MERGE` (A2); the drift field (A3); REUC (A3); `MERGE_AUTOSTASH` (A1); and B3's landings as observed. — A items in NOTES (Part A); B3 landings in NOTES Part B.
- [x] MAP.md: graft-core §journal; graft-ops §apply, §undo, §restore, §snapshot; graft §`update::commit_editor`. — **Part A half done**: graft-core §journal; graft-ops §operation, §finish, §journal, §undo, §facts, §apply, §restore, new §merge_state. The anchoring is in `apply/record.rs` (`with_anchors`), so it is under §apply, not §snapshot (`snapshot::take` is unchanged). graft §`update::commit_editor`: the `Begun` line (Part B; the `refusal` line is unchanged because B2's words did not change).

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

## NOTES (Part A builder, 2026-09-24)
- **I compacted once mid-ticket.** Everything Part A lists is done and ticked above; what remains is Part B (handoff below) and the graft MAP line.
- **Scout used:** `.scratch/m6-conflicts/scout-20.md`.
- **Message route (A1): a third route, not one of the two offered.** The spec's premise "`Invocation` doesn't grow stdin" does not hold: `Invocation` already has `stdin: Option<Vec<u8>>` and `Invocation::with_stdin` (`graft-ops/src/operation.rs`, the `Invocation` struct; `commit -F -` and `tag -a -F -` use it since m3/02 / m5/07). So `FinishMerge`'s plan is `commit --cleanup=whitespace -F -` with the message on the invocation's stdin: pure data, no scratch file, no crate-private run step. `GIT_EDITOR=true` comes from the plan runner, as for every step. Command Log line: `git commit --cleanup=whitespace -F -`. The forward path never writes `MERGE_MSG`.
- **`AUTO_MERGE` (A2): left out.** It names a tree, not a commit. Anchoring it would mean a tree among a backup commit's parents (git refuses) or a change to the backup commit's layout (hard to reverse; the conductor's call). `git status`, `commit` and `merge --continue` don't need it, and git rewrites it on its next merge. Documented in `MergeState`'s doc comment.
- **Drift field (A3): `Drift::merge: Option<MergeDrift>`**, `MergeDrift::{Begun, Ended, Changed}`. It isn't the two states because a boxed pair of `MergeState`s pushed `UndoError` over clippy's `result_large_err` bound, and `UndoError::Drift(Box<_>)` would break `graft`, which isn't my crate. The words: `a merge is in progress now` / `the merge is no longer in progress` / `the merge in progress changed`.
- **REUC (A3): observed surviving.** `git ls-files --resolve-undo` reads the same before `c`, after `c` and after `u`. After `u`, `git checkout -m a.txt` rebuilds the same three stages it would have before `c` (`finish_merge.rs::after_u_checkout_m_still_rebuilds_the_conflict`, git 2.54.0 Apple Git-157). Graft writes no REUC. Why it survives wasn't investigated; `merge --continue` keeps it and the index restore doesn't drop it. Not checked on other git versions.
- **`MERGE_AUTOSTASH` (A1): git applies it**, on both `git merge --continue` and `git commit --no-edit` (observed, git 2.54.0; `finish_merge.rs::git_applies_merge_autostash_when_the_merge_is_finished`). So `apply` refuses `FinishMerge` as `NotInProgress` while `MERGE_AUTOSTASH` exists under `git_dir` (`merge_state::autostash_waits`; test `c_is_refused_while_merge_autostash_waits`). It is looked up as a file under `git_dir`, like `MERGE_HEAD` in `graft_git::state`. A reftable repository would not show it there; that is the same limit `graft_git::state` already has.
- **Octopus: a behaviour change beyond `c`, unverified.** `resume` gives `NotInProgress` (words unchanged). But `read_merge_state` returns `MergeStateError::Octopus`, so **`begin` of every entry fails while git has an octopus merge in progress** (`FactsError::MergeState`) rather than record a wrong state. No octopus fixture exists, so no test covers this. The conductor decides whether an octopus should instead record `None`.
- **Abort/Skip of a shell-begun merge: unchanged**, `NotInProgress`, and no git runs (`finish_merge.rs::abort_and_skip_of_a_shell_merge_stay_not_in_progress`).
- **A live entry of Graft's owning the merge:** `apply(FinishMerge)` refuses `InProgress(Merge)`. `resume` never sends it there, but a direct caller would otherwise leave that entry `Pending` for ever. No test for the direct-call case.
- **I changed one existing test, deliberately.** Leg 2 of `history_ops::a_shell_merge_is_not_adopted_as_grafts_own_entry` expected `NotInProgress` for `Continue` on an unresolved shell merge. A1 makes that `Unmerged`, which fixes the bug the ticket names. The leg now also asserts that Graft's stale entry stays `Pending` (not adopted), and binds that leg's own entry id.
- **`SetMergeState`'s after-check** is `rev-parse -q --verify MERGE_HEAD`, not a parse of `git status`, because `MERGE_HEAD` existing is what `git status` reads to say "merging".
- **Gates (Part A), what ran:**
  - `cargo fmt --check`: clean.
  - `cargo build --workspace`: clean; graft's **lib and bin compile**.
  - `cargo test --doc` workspace-wide: green.
  - `cargo nextest run --workspace --exclude graft`: 1555/1555.
  - `cargo clippy -p graft-core -p graft-ops --all-targets -D warnings`: clean.
  - **Workspace clippy and nextest fail, only in `graft`'s test code.** 13 `JournalEntry { .. }` literals lack `merge_before`/`merge_after` (E0063), at these lines in this commit:
    - `graft/src/update/`: `resolver/tests/words_and_c.rs:19`, `banner/tests.rs:57`, `undo.rs:567`, `undo.rs:969`, `remote/tests.rs:831`, `journal.rs:322`, `tests.rs:4579`, `operate.rs:1011`
    - `graft/src/view/`: `journal.rs:246`, `tests.rs:2528`
    - `graft/src/jobs/undo.rs:593`
    - `graft/tests/it/`: `undo_by_key.rs:217`, `conflict_interim.rs:926`

    No `match` arm breaks.

### Handoff: Part B
1. **First:** add `merge_before: None, merge_after: None` to the 13 literals above (or `..` a base). That makes the workspace gates green again.
2. Then B1–B3 as specified. `resume(Continue | ContinueWith)` already routes a shell-begun merge to `FinishMerge`, so the app needs no new ops call. The entry's description is `finish merge of <from>` (`from` = `FinishSummary::from`'s rule), so B3's `undo: finish merge of feature` status comes from the journal as it is.
3. The graft MAP line (§`update::commit_editor`) is Part B's.

## NOTES (Part B builder, 2026-09-24)
- **Status: partial — B1, B3, captures done; B2 blocked** (its own stop condition). No compaction. Scout: `scout-20.md` (read, not re-run); the rest read directly (banner/finish/commit-editor MAP sections, `keys.rs:252-290`).
- **Seam first:** the 13 `JournalEntry` literals gained `merge_before: None, merge_after: None` (commit `m6-conflicts-20: Part B seam …`). No `match` arm needed one. Workspace gates were green before B1–B3 began.
- **B2: `blocked: the words name keys that don't reach open_finish — update::keys::graph_key` (`crates/graft/src/update/keys.rs:273-278`, `is_commit_key` at `keys/predicates.rs:114-123`).** Observed: J3's state (the app launched into a resolved shell merge; `Tab` off the banner; changes pane; `c` opens the editor), then `Esc` → the editor closes and the **changes pane** keeps the keys; `c` → `graph_key` → `is_commit_key` → `commit_editor::open`: the editor reopens, `finish` stays `None`, status `Ready`. The banner's `c` is only reached while `banner_focus` is set (`keys.rs:89`, `banner_key`). Per the ticket I did not reword to fit: `CommitEditor::refusal` still says `merge begun in the shell — finish it there: git commit` (true). Repro test (removed, not disabled): the B2 integration test above, with `resolved_shell_merge` + `launched_into` from `tests/it/resolver/shell_begun.rs`; after `Esc`, assert `finish.is_some()` fails with `editor true, status Ready`.
  - **Question (not a defect — not observed):** the `InGraft` arm's words `merge in progress — esc, then c continue  a abort` (`commit_editor.rs`, the `Begun::InGraft` arm) go through the same key route: after `Esc` from an editor opened in the changes pane, `c` would reopen the commit editor and `a` would open the **amend** editor (`is_commit_key` `'a' → Some(true)`). I did not run that walk. The conductor decides whether the fix is a key route (e.g. `Esc` from the editor while an operation is live hands the banner the keys) or words (`esc, tab to the banner, c`); either is a keymap/spec change, not mine.
- **B3 landings, as observed** (all from the tests named in the ticks; nothing special-cased):
  - `⏎` on a shell-begun merge: `finish` `None`, graph focused, cursor on `HEAD` (the merge commit), `Status::Done("merged feature into main")`, header without `MERGING`, `in_progress` `None`. The detail pane opens below the graph (as J6's landing); the status is the graph's footer row.
  - Hook refusal: summary stays, notice `(words, true)` with the hook's stderr, subject kept, `MERGE_HEAD` present; the journal has the `FinishMerge` entry `Aborted`.
  - `u` after the finish: `Status::Done("undo: finish merge of feature")`; after the refresh `in_progress` = `Merge`; header `MERGING`; `banner_view` `Some`; focus unchanged (`graph_has_focus` and `banner_focus` each asserted equal to their values before `u`; the `⏎` landing left the graph focused); `stopped` `None`. The graph shows `main` back on `main's take`, no merge commit; `Uncommitted changes (2 staged)` on top.
- **Helpers made `pub(crate)`** in `tests/it/conflict_interim.rs` for the new tests: `CONFLICT_NOW`, `about_to_conflict`, `launched`. Test code only.
- **Captures:** the merge commit's id **and author date** are zeroed in `fix-finish-shell-merge-120x40.txt` (the app makes it now; the `COMMIT` line draws `%Y-%m-%d %H:%M %z`). Colour unverified (rule 5); the look is the user's (rule 6).
- **For the conductor's TESTING.md text:** the `TESTING.md:2121-2123` replacement says the commit editor "still refuses, naming `esc, then c`" — with B2 blocked it still names `git commit`. Adjust that sentence to whatever B2's resolution is.
