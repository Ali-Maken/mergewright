# undo-and-abort-a-shell-begun-merge: `a` aborts, and `u`/`U` undo Graft's own steps, in a merge begun in the shell

Status: partial — ops half done (m6-conflicts-57a); app half open (see NOTES, "What remains")
From: user picks 2026-09-24 item 10 (batch 3, "both"); KNOWN_GAPS M6/08 :611 (the question), :612; TESTING.md
"Times I left" row :113.
Amended 2026-09-24 (spec-only pass) after scout `.scratch/m6-conflicts/scouts/57-58.md`; the planner re-read the lines
it cites (`graft-git/src/state.rs:121`, `graft/src/update/state.rs:261-267`, `update/undo.rs:103`,
`update/preview.rs:295-300`, `graft-ops/src/apply/finish_merge.rs:32-49`, `apply/resume.rs:251-264`,
`undo/execute.rs:128-159`, `undo.rs:161-222`) and `docs/design/02-keymap.md` §2.6 `d` row and §2.6a (confirmed: the
`a` row and the `u`/`U` "One exception" paragraph say what this spec builds).
Build with: builder-opus-xhigh — undo and abort over git writes in the conflicts area; a wrong gate loses work or
refuses the way back, and the diff alone cannot show which.
Needs: none.
Crates: graft-ops, graft.
Lane: wave 1, lane O, first. Runs in parallel with lane U (`graft-ui` only).

## Read first
- `docs/design/02-keymap.md` §2.6a (the `a` row; the `u`/`U` paragraph with its "One exception").
- `.scratch/m6-conflicts/spec-pass-edits-2026-09-24.md` E6, J15 and J16 (the journeys and landings).
- `.scratch/m6-conflicts/scouts/57-58.md` §57 (the code as it stands, with lines).
- `crates/graft-ops/MAP.md` §undo (:100-114), §merge_state (:150-155), §apply (:157-176), `Refusal` (:48-49).
- `crates/graft/MAP.md` §update::commit_editor `Begun` / `live_operation` (:241-242), §update::undo (:271-282),
  §update::banner (:284-313), `AppState::live_entries` (:170).
- `docs/research/conflict-resolution.md` (the study's recovery section, for the words only).

## Decisions (made in this pass; build them, do not re-decide)
1. **Who began the merge — the signal already exists.** `update::commit_editor::live_operation(&AppState) ->
   Option<(InProgress, Begun)>` answers `Begun::InShell` iff `AppState::live_entries` is empty (graft MAP :242). That is
   the same fact ops' `resume_target` checks (`live_entry`). It survives a relaunch: `recover_interrupted` hands the
   still-live `Pending` entries to the app at launch (`Recovered::live`, graft-ops MAP :176) and every journal read
   refills `live_entries`. **No new field in `InProgressDetail`, and `stopped` is not the signal** (it is set on
   relaunch too). One addition: until the launch's first journal read has answered, `live_entries` is empty for a merge
   Graft began, so `live_operation` would say `InShell`. For `u`/`U` and the abort box's words, **no journal read yet
   = refuse as today** (`finish or abort the merge first`). The builder names in NOTES the line where the launch first
   fills `live_entries`, and whether `live_operation` itself should carry that rule (if every caller wants it, it goes
   there; `commit_editor`'s refusals are then re-checked green).
   The app's routing is words only. The safety rule lives in ops (decision 3): if the app sends an undo it should not
   have, ops refuses it and nothing is written.
2. **Refusal words: one source, three callers.** Today there are three copies of `finish or abort the <kind> first`:
   `update::undo::in_progress_refusal` (:103, keys `⇥ banner`), `update::preview::open_preview` (:295, keys none), and
   now ops' `OperationLive`. Add `pub(super) fn live_refusal(kind: InProgress, keys: Option<String>) -> Status` in
   `update::undo`; `in_progress_refusal`, `open_preview` and the `OperationLive` answer all build their `Status` through
   it. `open_preview` keeps refusing **every** live operation, a shell merge included: it opens a Ghost Preview or the
   journal's revert, and keymap §2.6a's exception covers `u`/`U` only. Its keys stay `None`: the journal and preview
   overlays hide the banner, so `⇥ banner` would point at nothing. Delete `preview::kind_word`.
3. **The undo gate (ops owns it).** In `undo::execute::run_plan`, after `read_state_now` and before `drift`:
   ask git's state (`graft_git::state::in_progress`, as `resume_target` does). If nothing is live, go on. If a
   **merge** is live and the entry being reversed (`plan.reverses`) has a `merge_after` that is **the same merge** as
   `now.merge`, go on. "The same merge" means `merge_head` and `other_heads` are equal; `merge_msg` and `merge_mode`
   are `drift`'s business, and the force box still covers them. Otherwise refuse with
   `UndoError::Nothing(NotInvertible::OperationLive { kind })`, nothing written and no drift box. This covers all three
   entry points (`undo`, `redo`, and each `revert_to` step), because they all go through `run_plan`.
   Redo needs no special case: a redo reverses the `Undo` entry, and that entry's `merge_after` is the merge the undo
   put back.
4. **Autostash: in scope, for both ends.** `FinishMerge` refuses an autostash as `NotInProgress` today
   (`finish_merge.rs:48-49`), and so the user is told the merge is not in progress while it is standing in front of
   him. Both gates now refuse with `Refusal::AutostashWaits { end: MergeEnd }`. The words are
   `git set your changes aside for this merge (autostash); finish it in the shell: git commit`, or
   `…; abort it in the shell: git merge --abort` for an abort. `MergeEnd` is `Finish | Abort`.
5. **Octopus: refused, reusing `resume::shell::refuse_octopus`.** `MergeState` does hold every head
   (`merge_state.rs:83-92`), but the check after the write compares only the first head (graft-ops MAP :154), and
   `FinishMerge` already refuses an octopus. `Refusal::OctopusMerge` gains `end: MergeEnd`, so the words name the right
   command: `…; abort it in the shell: git merge --abort`. The existing finish words stay the same. No
   `OctopusAbort` variant.

## What `u` of an abort puts back, and what it cannot (each with its proof)
| After `a` on a shell merge, `u`… | puts back? | proved by |
|---|---|---|
| `MERGE_HEAD`, `MERGE_MSG`, `MERGE_MODE` (byte-exact) | yes: `SetMergeState(merge_before)`, last | `abort_of_a_shell_merge_is_journaled_and_undoable` |
| the three stages of each path still unmerged at the abort | yes: backup `IndexState::Unmerged` | same test (`git ls-files -u`) |
| a path he had already resolved and staged before the abort | yes: its stage-0 entry, in the same stage tree | `undo_of_an_abort_keeps_a_file_already_resolved` |
| his hand edits in the working tree, byte for byte | yes: the `Full` backup's worktree tree | `abort_of_a_shell_merge_is_journaled_and_undoable` |
| `HEAD` and the branch | unchanged by the abort; the undo leaves them alone | `abort_of_a_shell_merge_is_journaled_and_undoable` asserts `HEAD` before = after undo |
| a stash git holds for the merge (`MERGE_AUTOSTASH`) | **never at risk**: the abort is refused first | `abort_refuses_while_git_holds_an_autostash` |
| an octopus merge | **never at risk**: refused first | `abort_of_an_octopus_shell_merge_is_refused` |
| other git files under `.git/` that the abort removed and `MergeState` does not model (for example `MERGE_RR`, which rerere uses, and `AUTO_MERGE`) | **no** | `undo_of_an_abort_restores_exactly_the_merge_files_graft_knows`: fixture with `rerere.enabled=true`; lists the merge files under `git_dir` before the abort and after the undo; asserts the difference is exactly what NOTES names. The conductor opens a KNOWN_GAPS row (a known limit, not broken) for each file that does not come back. |
| anything from before the merge, while the merge stands | refused, `OperationLive` | `undo_refuses_an_entry_from_before_a_live_merge` |

## Scope
**In: graft-ops**
- **`Operation::AbortMerge(AbortMergeParams { merge_head })`.** `resume(Abort)` with **no live entry** while git is
  merging → `apply(Operation::AbortMerge)`. This mirrors `Continue`'s shell route: `resume_target`'s
  `(InProgress::Merge, true)` arm (`resume.rs:262`) gains an `Abort` arm into `resume/shell.rs`. The plan is
  `git merge --abort`, with a `Full` backup. `begin` records `merge_before = Some(state)` and `finish` records
  `merge_after = None`, as on every entry. The description is `abort merge of <from>` (`finish::merged_from`, as
  `finishing_merge` builds it).
- **Gates before `begin`**, in the order `finishing_merge` uses (share them; do not copy): a live entry → the existing
  `resume(Abort)` path, unchanged; no `MERGE_HEAD` → `NotInProgress`; octopus → `OctopusMerge { heads, end: Abort }`;
  `MERGE_HEAD` ≠ `params.merge_head` → `NotInProgress`; autostash → `AutostashWaits { end: Abort }`.
  `finishing_merge`'s autostash arm → `AutostashWaits { end: Finish }`.
  Shell rebase, cherry-pick and revert: `NotInProgress`, unchanged (M7).
- **The undo gate**: decision 3.
- `nothing_to_say(OperationLive { kind })` → `finish or abort the <kind> first`. This is ops' own `Display`; the app
  words it through decision 2, and a graft test pins the two texts equal.

**In: graft**
- `in_progress_refusal` → `None` when `live_operation` is `(Merge, InShell)` and a journal read has landed
  (decision 1). `u`/`U` there, global or on the focused banner (`banner_key`, `banner.rs:380`), send the undo, and ops
  answers. Every other live operation refuses as today.
- `jobs::undo`: `UndoError::Nothing(NotInvertible::OperationLive { kind })` → a new `Undone::OperationLive { kind }`
  (not `Nothing(String)`). `update::undo::undone` words it with `live_refusal(kind, Some("⇥ banner"))`, as a Sticky
  status: a refusal, not `Status::Done`. `RevertedTo` keeps its `Failed` words: `open_preview` refuses before a revert
  can reach the gate, so the gate is only a backstop there.
- The banner's `a` on a shell-begun merge (`abort_pressed`) opens the existing abort box with its cursor on
  `[ Cancel ]`. Its lines are `<into> goes back to before the merge` and `u puts the merge back`, where `<into>` is
  `HEAD`'s branch (short sha when detached). Accept → `ConfirmThen::Resume(Abort)` → ops. A merge begun in Graft keeps
  today's lines.
- Landing after the abort: `banner::resumed` → `Done` → `undo::operation_ended` (clears `choices` and the finish
  draft), graph focused, and the `Done` status `resumed` already sets. **The planner did not read `resumed`'s words.**
  The builder quotes the line in NOTES. If it does not name the abort (`abort merge of feature`), make it name it: a
  `Done` that says nothing about what was done fails the capture below.
- Landing after `u` of the abort: `undo::land_after_undo`. The merge is live again, so the banner is focused and the
  status reads `undo: abort merge of feature ✓ · c finishes the merge` (ticket 41's landing, unchanged).
- Landing after `u` of a whole-file step inside the shell merge: the banner keeps focus if it had it; status
  `undo: <entry description> ✓`.

**Out:**
- `u` inside the resolver: keymap §2.6 does not bind it (the resolver is modal). `Esc u` from the resolver is the
  route.
- `d` after `t` in a shell merge (`NoStagesKept`, KNOWN_GAPS :613; keymap §2.6 `d` row: "still refused in a merge
  begun in the shell"). The §2.6a exception's "or a start-over (`d`)" can't happen in a shell merge until :613 is
  fixed; say so in NOTES, and don't build it.
- A block take (`o`/`t`/`b` on a marker block) is an fs write, not a journal entry (graft-ops MAP §conflict :57), so
  `u` does not restore it. Only whole-file choices are entries in a shell merge.
- Shell-begun rebase, cherry-pick, revert (M7).
- Removing `MergeDrift::Begun`. After the gate, `run_plan` never reaches it; note that in NOTES and leave the variant.

## The failing tests first
- graft-ops `tests/it/finish.rs` (or a sibling `abort.rs`): `abort_of_a_shell_merge_is_journaled_and_undoable`.
  Setup: `git merge feature` in the shell stops on `notes.txt` and `gone.txt`; hand-edit `notes.txt`. `resume(Abort)`.
  Assert: no `MERGE_HEAD`; one `AbortMerge` entry. Then `undo`: `MERGE_HEAD` = `feature`; `MERGE_MSG` byte-equal to
  before; `git ls-files -u` lists both paths' stages; `notes.txt` holds his hand edit byte for byte; `HEAD` unchanged.
  Red today: `resume(Abort)` → `NotInProgress`.
- graft-ops undo tests: `undo_refuses_an_entry_from_before_a_live_merge`. A commit, then a shell merge stops; `undo`
  → `NotInvertible::OperationLive { kind: Merge }`; nothing is written and no drift box opens. Red today: it plans the
  commit's undo, and `drift` says `a merge is in progress now`.
- graft `tests/it/resolver/shell_begun.rs`: `a_on_the_banner_aborts_a_shell_merge` and
  `u_on_the_banner_restores_a_whole_file_take_in_a_shell_merge`.

## Signatures
```rust
// graft-ops/src/operation.rs (+ the child holding the merge params)
pub enum Operation { /* … */ AbortMerge(AbortMergeParams) }
pub struct AbortMergeParams { pub merge_head: Oid }
pub enum MergeEnd { Finish, Abort }
pub enum Refusal {
    /* … */
    AutostashWaits { end: MergeEnd },                 // new; FinishMerge's autostash arm moves here from NotInProgress
    OctopusMerge { heads: usize, end: MergeEnd },     // `end` added; Finish keeps today's words
}
// graft-ops/src/undo.rs
pub enum NotInvertible { /* … */ OperationLive { kind: InProgress } }
// graft-ops/src/undo/execute.rs (private): the gate of decision 3, called from run_plan between read_state_now and drift
fn refuse_while_live(ctx: &Context<'_>, target: &JournalEntry, now: &RepoStateNow) -> Result<(), UndoError>;
// graft-ops/src/apply/resume.rs + resume/shell.rs: resume(ctx, Resume::Abort), no live entry, git merging → apply(AbortMerge)

// graft/src/jobs/undo.rs
pub enum Undone { /* … */ OperationLive { kind: InProgress } }
// graft/src/update/undo.rs
pub(super) fn live_refusal(kind: InProgress, keys: Option<String>) -> Status;   // the one wording
pub(super) fn in_progress_refusal(state: &AppState) -> Option<Status>;          // unchanged signature; None for (Merge, InShell) once the journal is read
// graft/src/update/commit_editor.rs (existing, read): pub fn live_operation(&AppState) -> Option<(InProgress, Begun)>
```
`Repo`/`Context` argument types follow what exists.

## Touches (each entry, and how the builder checks it)
- **Every exhaustive match on `Operation`, `Refusal`, `NotInvertible`, `Undone`**: describe, touches, plan, check,
  explain, `kind_name`, the app's word tables.
  - Check: workspace-wide clippy. `grep -rn "NotInvertible::\|Undone::\|OctopusMerge" crates`: each arm handles the
    new or changed shape.
- **`FinishMerge`'s gates**: the autostash arm's refusal changes, and octopus gains `end`.
  - Check: `grep -rn "autostash\|OctopusMerge" crates/*/tests crates/*/src`. Every test that asserted `NotInProgress`
    for an autostash now asserts `AutostashWaits { end: Finish }`, and each one is named in NOTES. The finish-octopus
    words test stays green unedited.
- **Journal readers**: the new kind name in the journal (`kind_name`), the journal browser `J`,
  `interrupted_notice`, and `plan_revert_to` (an `AbortMerge` is an operation, so it is revertible like any other).
  - Check: `journal_round_trips_an_abort_merge_entry`; `J` shows `abort merge of feature` (a row test).
- **The undo gate reaches every executor entry point**: `undo`, `redo`, `revert_to`.
  - Check: `redo_refuses_while_a_rebase_is_live`, `revert_to_refuses_past_a_live_merge`,
    `redo_of_an_undone_abort_is_allowed_while_the_merge_is_back`.
- **Existing ops tests that undid something while a merge was live**, and expected the drift box
  (`MergeDrift::Begun`, `a merge is in progress now`).
  - Check: `grep -rn "MergeDrift::Begun\|a merge is in progress now" crates`. Each such test now expects
    `OperationLive`; NOTES names each one as an intended change. A test that undoes a step **inside** the same merge
    stays green unedited.
- **The shell-merge signal** (decision 1).
  - Check: `u_still_refuses_in_a_merge_begun_in_graft`,
    `u_still_refuses_after_relaunch_into_a_merge_begun_in_graft`, and
    `u_refuses_while_the_launch_journal_read_is_out`. `commit_editor`'s refusal tests stay green unedited.
- **The three refusal copies** (decision 2).
  - Check: `grep -rn "finish or abort the" crates/graft/src`: one hit, in `live_refusal`.
    `operation_live_words_match_ops` (graft) pins the app's reason equal to ops' `UndoError` text for each kind.
    The existing preview refusal tests stay green unedited.
- **Graft-begun merges and every other kind**: unchanged.
  - Check: the existing `u` refusal tests (`update::undo` tests, `it/resolver/*`) stay green unedited.
- **The existing test** `a_hand_edit_taken_by_t_comes_back_after_the_merge_ends` (`shell_begun.rs:24`) still passes
  (scout contradiction 4: nothing is live after the shell abort, so the gate does not apply). Its NOTES-worthy change:
  the shell step is no longer needed. Check: green, unedited.
- **Banner hints**: does `a abort` show on a shell merge's banner? If it was hidden because it refused, it now shows.
  - Check: `every_key_the_bar_draws_is_answered`. If the bar changed, the `m6-18-launch-shell-begun-*` captures are
    regenerated and named in NOTES.
- **`u` in the changes pane or the graph during a shell merge** is now an undo.
  - Check: `u_in_the_graph_undoes_a_step_of_a_shell_merge`.
- **Confirm box and the `y` rule** (ADR-0044 §3, ticket 50): the abort box has one acting choice. Whether `y` takes it
  is ticket 50's business; this ticket keeps `→ ⏎`. Check: `a_on_the_banner_aborts_a_shell_merge` drives `→ ⏎`.
- **The walk** (`crates/graft/tests/it/m5_walk/`): two scenarios, `b-abort-shell-merge` (`a → ⏎`, 3) and
  `b-undo-a-take-in-the-shell` (start: the banner focused after whole-file `t` on `gone.txt` and `Esc`; keys `u`, 1).
  - Check: `journey_counts_match_the_note` with the rows below applied; `every_scenario::journey`.
- **TESTING.md "Times I left" :113**: the acceptance ticket closes it; the builder names the test that proves it.
- **KNOWN_GAPS :611, :612**: the conductor closes :611. :612 (the `UD` drawn with no merge) is now reachable only after
  an undo past the abort, and stays open unless a test here draws it. New rows for the files `u` does not put back
  (the table above) are the conductor's, from NOTES.
- **Lane U**: none of 54–56 touches `graft-ops` or `graft` source. Check: `git diff --stat` names only these two
  crates, `docs/shots`, and the MAPs.
- **Docs**: `crates/graft-ops/MAP.md` §apply (`AbortMerge`, the shell route, `AutostashWaits`, `MergeEnd`) and §undo
  (`OperationLive`, the gate); `crates/graft/MAP.md` §update::undo (`live_refusal`, `in_progress_refusal`,
  `Undone::OperationLive`), §update::banner (`a` on a shell merge), §update::preview (the refusal's source).

## Acceptance (name the verifying test at tick time)
- [x] `abort_of_a_shell_merge_is_journaled_and_undoable`: red first, then green.
      Verified by `graft-ops tests/it/abort_merge.rs::abort_of_a_shell_merge_is_journaled_and_undoable` (red: `resume(Abort)` → `NotInProgress`).
- [x] `undo_refuses_an_entry_from_before_a_live_merge`: red first, then green.
      Verified by `graft-ops tests/it/undo_gate.rs::undo_refuses_an_entry_from_before_a_live_merge` (red: the drift box `…a merge is in progress now — u again to go ahead`).
- [x] `undo_of_an_abort_keeps_a_file_already_resolved`
      Verified by `abort_merge.rs::undo_of_an_abort_keeps_a_file_already_resolved` (whole `ls-files --stage` equal; green on first run, not seen red).
- [x] `undo_of_an_abort_restores_exactly_the_merge_files_graft_knows` (NOTES lists the files that do not come back)
      Verified by `abort_merge.rs::undo_of_an_abort_restores_exactly_the_merge_files_graft_knows` (missing after the undo = `AUTO_MERGE`, `MERGE_RR`; nothing changed or added).
- [x] `abort_refuses_while_git_holds_an_autostash`, `finish_refuses_while_git_holds_an_autostash` (words, not
      `NotInProgress`)
      Verified by `finish_merge.rs::abort_refuses_while_git_holds_an_autostash` and `::finish_refuses_while_git_holds_an_autostash` (exact words; plus `::git_applies_merge_autostash_when_the_merge_is_aborted`, plain git).
- [x] `abort_of_an_octopus_shell_merge_is_refused` (words name `git merge --abort`)
      Verified by `octopus_merge.rs::abort_of_an_octopus_shell_merge_is_refused` (exact words).
- [x] `redo_refuses_while_a_rebase_is_live`, `revert_to_refuses_past_a_live_merge`,
      `redo_of_an_undone_abort_is_allowed_while_the_merge_is_back`
      Verified by `undo_gate.rs::redo_refuses_while_a_rebase_is_live`, `undo_gate.rs::revert_to_refuses_past_a_live_merge`, `abort_merge.rs::redo_of_an_undone_abort_is_allowed_while_the_merge_is_back` (the first two written after the gate, **not seen red**: see NOTES).
- [x] `journal_round_trips_an_abort_merge_entry`
      Verified by `abort_merge.rs::journal_round_trips_an_abort_merge_entry` (`"kind":"abort_merge"`, parsed equal, rendered byte-equal).
- [ ] `a_on_the_banner_aborts_a_shell_merge`, `u_on_the_banner_restores_a_whole_file_take_in_a_shell_merge`,
      `u_in_the_graph_undoes_a_step_of_a_shell_merge`
- [ ] `u_still_refuses_in_a_merge_begun_in_graft`, `u_still_refuses_after_relaunch_into_a_merge_begun_in_graft`,
      `u_refuses_while_the_launch_journal_read_is_out`
- [ ] `operation_live_words_match_ops`
- [ ] `journey_counts_match_the_note` with `b-abort-shell-merge` = 3 and `b-undo-a-take-in-the-shell` = 1.
- [ ] Capture (rule 6; colour **unverified**, rule 5): `docs/shots/m6/fix-shell-merge-abort-and-undo-120x40.txt`,
      three frames: the abort box over the banner; the graph after `→ ⏎`; after `u`, the banner back.
  - **What right looks like:** frame 1 has the lines `main goes back to before the merge` and
    `u puts the merge back`, with the cursor on `[ Cancel ]`. Frame 2 has no `MERGING`, and its status line contains
    `abort merge of feature`. Frame 3 shows `MERGING` and `CONFLICTS (2)` again, and its status line starts
    `undo: abort merge of feature ✓`.
- [ ] MAP.md updated (doc, no test).

## For the conductor (exact text, in this spec's merge commit)
Add to the `journey-counts` fence of `docs/design-notes/m6-conflicts.md`, after `b-finish-again-after-undo`:
```
b-abort-shell-merge           3                     # batch 3 (approved 2026-09-24): a → ⏎ from the banner · ticket 50 → 2 (a y) · lazygit 3 (?)
b-undo-a-take-in-the-shell    1                     # batch 3: u from the banner after a whole-file t · JetBrains ⌃Z 1
```

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Medium-large: a new operation kind, one safety gate, two refusal shapes, and the app's routing. One context. If it
outgrows that, split at the seam: ops first (the operation, the gates, `OperationLive`), then the app (routing, words,
landings). Ops ships first and on its own: with nothing else changed, the app keeps refusing as today.

## NOTES

**Compacted mid-ticket.** The builder's context compacted during the ops half. Per the Size note the ticket is split at
the seam: the ops half is done and committed (`m6-conflicts-57a`), and the app half is open, listed below under "What
remains". Scout: `.scratch/m6-conflicts/scouts/57-58.md` (the conductor's, as briefed).

**The ops seam changes one app behaviour on its own.** The Size note says "with nothing else changed, the app keeps
refusing as today". That holds for `u` inside the merge: the app still refuses it before sending. It does **not** hold
for `a`. The banner's existing `a → ⏎` box already sends `Resume::Abort`, and ops now carries it out:
- The shell merge is aborted as an `AbortMerge` entry. The status reads `Done { what: "abort merge of feature" }`,
  observed in `shell_begun.rs` before that test was edited.
- The box still says the old lines, `everything goes back to before it started` / `the journal keeps the entry`.
  Both are true of an abort, but they are not 57's words.
- After the abort nothing is live, so the app sends `u`, and `u` puts the merge back. This is tested in ops by
  `abort_of_a_shell_merge_is_journaled_and_undoable`. It is **not** tested through the app.

**Touches:**
- **journal:**
  - The new kind `abort_merge` is in graft-core (`OperationKind::AbortMerge`). `kind_name` returns `abort merge`.
    Checked by `journal_round_trips_an_abort_merge_entry`.
  - `pins_for` gives `[]`. `fold` never folds it: checked by `folded_step`'s match and the abort test, which gets
    exactly one entry.
  - `plan_revert_to` treats it as any operation. Not tested on its own.
  - The `J` row and `interrupted_notice` belong to the app half. **Not checked.**
- **undo:**
  - The gate sits in `run_plan`, so all three entry points pass it. Checked by
    `undo_refuses_an_entry_from_before_a_live_merge` (`undo`), `redo_refuses_while_a_rebase_is_live` (`redo`),
    `revert_to_refuses_past_a_live_merge` (`revert_to` step 1) and
    `redo_of_an_undone_abort_is_allowed_while_the_merge_is_back` (the "same merge" exception).
  - Steps undone **inside** a shell merge: the existing tests stay green unedited (workspace nextest).
  - `grep -rln "MergeDrift::Begun\|a merge is in progress now" crates` now matches `drift` itself and its pure unit
    tests (`undo.rs`, `undo/tests.rs`), plus doc comments that name the old behaviour (`execute.rs`,
    `finish_merge.rs`, `undo_gate.rs`, MAP). No test still asserts that drift box. The variant is kept, but `run_plan`
    can no longer reach it.
- **every refusal copy:**
  - The words for `OctopusMerge{end: Finish}` are unchanged: `c_on_an_octopus_merge_names_its_refusal` still asserts
    `git commit`.
  - The words for `OctopusMerge{end: Abort}`, `AutostashWaits{Finish}` and `AutostashWaits{Abort}` are new. Each is
    pinned exactly by its test.
  - The words for `NotInProgress` are unchanged. It is still returned for Skip, for a shell rebase, cherry-pick or
    revert, and when nothing is live.
  - `OperationLive` reads `finish or abort the <kind> first`, pinned in `undo_gate.rs`.
  - The app's single `live_refusal` is **not built**. Three copies still exist:
    - ops `nothing_to_say`
    - `update::undo::in_progress_refusal`
    - `update::preview`
- **captures:**
  - None were regenerated. The ops half changes nothing that is drawn: `every_key_the_bar_draws_is_answered` and the
    capture tests are green in the workspace run.
  - The only new user-visible result is `a` actually aborting, which today's captures do not show past the box.
- **crates:** graft-core, a crate outside the stated set, got one variant plus its MAP line. `Operation::kind()`
  needs it. graft-ui was not touched: `git diff --stat` lists graft-core, graft-ops, one graft test, the ticket and
  the MAPs.

**Intended test changes (each named):**
1. `finish_merge.rs`: `abort_and_skip_of_a_shell_merge_stay_not_in_progress` is now
   `skip_of_a_shell_merge_stays_not_in_progress`. Abort no longer returns `NotInProgress`; it aborts.
2. `finish_merge.rs`: `c_is_refused_while_merge_autostash_waits` is now
   `finish_refuses_while_git_holds_an_autostash`. It asserts `AutostashWaits { end: Finish }` and the exact words;
   it used to assert `NotInProgress`.
3. `finish_merge.rs`: `undo_refuses_drift_when_a_new_merge_was_begun` is now
   `undo_refuses_when_a_new_merge_was_begun`. It expects `OperationLive { Merge }` for both force values; it used to
   expect `MergeDrift::Begun`.
4. `history_ops.rs::a_shell_merge_is_not_adopted_as_grafts_own_entry`, leg 1: `Abort` now aborts the shell merge as
   a new `AbortMerge` entry. It still asserts that Graft's stale entry stays `Pending`, so the entry is not adopted.
5. `octopus_merge.rs`, three tests:
   - `an_unrelated_operation_begins_and_records_during_an_octopus_merge`
   - `undo_during_an_octopus_merge_keeps_merge_head_byte_exact`
   - `set_merge_state_writes_an_octopus_merge_head_back_byte_exact`

   They used to `u` a pre-merge entry through the drift box. Now they assert the gate refuses that `u` for both force
   values, then run the same inverse directly with `plan_undo` + `restore::execute`. So they still prove that `begin`
   records the two-line `MERGE_HEAD` byte-exact and that `SetMergeState` writes it back. The module doc is rewritten.
6. `octopus_merge.rs::c_on_an_octopus_merge_names_its_refusal`: the pattern gained `end: MergeEnd::Finish`, forced
   by the new field; its words are unchanged. The Touches item "stays green unedited" could not hold once the field
   was added.
7. graft `resolver/shell_begun.rs::a_hand_edit_taken_by_t_comes_back_after_the_merge_ends`: its helper no longer
   presses `a` and no longer asserts `begun in the shell is finished there`, because `a` now aborts. Both doc
   comments are updated. The Touches item "green, unedited" was wrong: scout contradiction 4 missed that the helper
   presses `a`.

**Files `u` of an abort does not put back** (observed with git 2.54.0, `rerere.enabled=true`). The conductor opens
one KNOWN_GAPS row per file, marked as a known limit, not broken:
- `MERGE_RR`: rerere's map from each conflicted path to its `rr-cache/<id>/`. After the undo, rerere will not record
  the resolution. The preimage stays in `rr-cache/`.
- `AUTO_MERGE`: the conflicted tree written by ort, used by `git diff AUTO_MERGE`.
- These stay byte-equal throughout: `ORIG_HEAD`, `COMMIT_EDITMSG`, `rr-cache/*/preimage`.

**Decisions taken in the build (question for the conductor if any is wrong):**
- The gate lets a **bisect** through, as before. It is not an operation Graft ends (no banner), and the app sends
  `u` during a bisect on purpose. Refusing it would change behaviour nobody asked to change.
- In the gate, `AbortMerge` is exempt from `Unmerged`, because a stopped merge is exactly what it ends. It is also
  exempt from the in-progress wait.
- Gate order, shared by `c` and `a` (`ending_a_shell_merge`):
  1. live entry → `InProgress`
  2. no `MERGE_HEAD` → `NotInProgress`
  3. octopus
  4. head mismatch → `NotInProgress`
  5. autostash
- Decision 1 asked for the line where launch first fills `live_entries`: `crates/graft/src/update/journal.rs:112`.
  Whether `live_operation` should carry the "no read yet" rule is **not decided**; it belongs to the app half.

**Not verified:**
- `redo_refuses_while_a_rebase_is_live` and `revert_to_refuses_past_a_live_merge` were written after the gate and
  never seen red. A mutation run with the gate call disabled was refused by the sandbox's permission classifier, so
  the gate was put back unchanged.
- `undo_of_an_abort_keeps_a_file_already_resolved` passed on its first run. It tests backup capability that existed
  before this ticket.
- No app-level test covers `a` then `u` on a shell merge yet.

**What remains (app half, graft only; ops needs nothing more):**
- `update::undo::live_refusal(kind, keys) -> Status`. `in_progress_refusal` and `preview::open_preview` build
  through it, and `preview::kind_word` is deleted. Afterwards `grep "finish or abort the" crates/graft/src` must
  give one hit.
- `in_progress_refusal` → `None` for `(Merge, InShell)`, but only once the launch journal read has answered
  (`Loading::Ready`).
- `jobs/undo.rs`: `Undone::OperationLive { kind }` plus its arm in `undone_from`, which today maps every
  `UndoError::Nothing` to `Undone::Nothing`. `update::undo::undone` words it `Sticky` via
  `live_refusal(kind, Some("⇥ banner"))`.
- `banner_key` `u`/`U` sends the undo when `in_progress_refusal` is `None`.
- `abort_pressed` on a shell merge shows two lines:
  - `<into> goes back to before the merge`
  - `u puts the merge back`

  The cursor starts on `[ Cancel ]`. `<into>` is HEAD's branch, or the short sha when detached.
- Tests:
  - `a_on_the_banner_aborts_a_shell_merge`, driven with `→ ⏎`
  - `u_on_the_banner_restores_a_whole_file_take_in_a_shell_merge`
  - `u_in_the_graph_undoes_a_step_of_a_shell_merge`
  - `u_still_refuses_in_a_merge_begun_in_graft`
  - `u_still_refuses_after_relaunch_into_a_merge_begun_in_graft`
  - `u_refuses_while_the_launch_journal_read_is_out`
  - `operation_live_words_match_ops`
  - `J` shows `abort merge of feature` (a row test)
- Walk scenarios `b-abort-shell-merge` (3) and `b-undo-a-take-in-the-shell` (1), checked by
  `journey_counts_match_the_note`. The note's fence edit is the conductor's.
- Capture `docs/shots/m6/fix-shell-merge-abort-and-undo-120x40.txt`, three frames. Re-check
  `every_key_the_bar_draws_is_answered` and the `m6-18-launch-shell-begun-*` captures.
- `crates/graft/MAP.md`: §update::undo, §update::banner, §update::preview.
