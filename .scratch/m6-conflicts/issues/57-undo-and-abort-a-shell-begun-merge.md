# undo-and-abort-a-shell-begun-merge: `a` aborts, and `u`/`U` undo Graft's own steps, in a merge begun in the shell

Status: ready
From: user picks 2026-09-24 item 10 (batch 3, "both"); KNOWN_GAPS M6/08 :611 (the question), :612; TESTING.md
"Times I left" row :113.
Build with: *(blank: brownfield; the conductor sets it after the scout, ADR-0035)*
Scout brief: (1) graft-ops: `resume(Abort)` with no live entry (`apply/resume.rs`, `apply/resume/shell.rs`), the
`FinishMerge` gate (`apply/finish_merge.rs`: `live_entry`, octopus, `MERGE_HEAD`, `MERGE_AUTOSTASH`), how a shell-merge
working-tree step becomes its own entry (`apply.rs`), and the undo executor's entry points (`undo`, `redo`,
`revert_to`) up to where they call `drift` — name the function that reads `RepoStateNow` first. Does `MergeState` hold
every head of an octopus (m6-conflicts/38)? (2) graft: `update::undo::in_progress_refusal` and its callers (the global
`u`/`U`, `banner_key`, the journal's revert), the banner's `a` path and its box (`ConfirmThen`), how
`NotInvertible` becomes words in `jobs::undo` / `update::undo::undone`, and the test
`it/resolver/shell_begun.rs::a_hand_edit_taken_by_t_comes_back_after_the_merge_ends`.
Needs: none.
Crates: graft-ops, graft.
Lane: wave 1, lane O, first. Runs in parallel with lane U (`graft-ui` only).

## Read first
- `docs/design/02-keymap.md` §2.6a (the `a` row, and the `u`/`U` paragraph with its "One exception", edited
  2026-09-24).
- `.scratch/m6-conflicts/spec-pass-edits-2026-09-24.md` E6, J15 and J16 (the journeys and landings).
- `crates/graft-ops/MAP.md` §undo (:100-114), §apply (:157-174), `Refusal` (:48-49).
- `crates/graft/MAP.md` §update::undo (:275-282), §update::banner (:284-313).
- `docs/research/conflict-resolution.md` (the study's recovery section, for the words only).

## Scope
**In — graft-ops:**
- **`Operation::AbortMerge(AbortMergeParams { merge_head })`.** `resume(Abort)` with **no live entry** while git is
  merging → `apply(Operation::AbortMerge)`, mirroring `Continue`'s shell route. Plan: `git merge --abort`. Scope `Full`
  backup (the unmerged index, the worktree with his hand edits). `begin` records `merge_before = Some(state)`, `finish`
  `merge_after = None`, as on every entry. Description `abort merge of <from>`.
- **Gates, before `begin`:** a Graft entry owns the merge (`live_entry`) → the existing `resume(Abort)` path, unchanged;
  `MERGE_HEAD` not `params.merge_head` → `NotInProgress`; `MERGE_AUTOSTASH` present → a new refusal
  `AutostashWaits`, words `git set your changes aside for this merge (autostash); abort it in the shell: git merge --abort`
  (git would apply the stash on abort, which `u` could not give back — the same reason `FinishMerge` refuses). An
  octopus merge: allowed **if** `MergeState` round-trips every head (scout); else refused, words
  `this merges <n> branches at once (an octopus merge); abort it in the shell: git merge --abort`. Shell rebase,
  cherry-pick, revert: `NotInProgress`, unchanged (M7).
- **`u` restores the merge:** the generic inverse already restores the index (unmerged `IndexState`), the worktree, and
  emits `SetMergeState(merge_before)` last. Prove it; add nothing unless a test shows a hole.
- **The undo gate while an operation is live** (the safety rule; ops owns it, the app only words it): when git reports
  an operation in progress, an entry may be undone, redone or reverted to **only if** the operation is a merge **and**
  the entry's `merge_after` equals the merge state now. Otherwise `NotInvertible::OperationLive { kind }`. Checked by the
  executor after it reads `RepoStateNow` and **before** `drift`, so a live merge is never offered the drift box's force.
  - This admits: a whole-file `t`/`o` or `d` step Graft journaled inside this shell merge; the redo of an undone
    `AbortMerge`. It refuses: anything from before the merge (its `merge_after` is `None`), anything while a rebase,
    cherry-pick or revert is live.

**In — graft:**
- `in_progress_refusal` returns `None` for a merge begun in the shell (no Graft live entry), so `u`/`U` there — global,
  and on the focused banner — send the undo; ops answers. Every other live operation refuses as today.
- `NotInvertible::OperationLive` → the sticky words today's refusal uses (`finish or abort the merge first · ⇥ banner`,
  keys by context). One source for both, so they cannot drift.
- The banner's `a` on a shell-begun merge opens the existing abort box (the keymap's cursor-on-Cancel rule); its words
  name what happens: `<into> goes back to before the merge · u puts the merge back`. Accept → `Resume::Abort` → ops.
- Landing after the abort: the existing abort landing (graph focused, `merge aborted ✓` or the words the abort of a
  Graft merge already shows; the scout names them), `choices` and `finish_draft` cleared (`operation_ended`).
- Landing after `u` of a step: the banner keeps focus if it had it; status `undo: <entry description> ✓`.

**Out:**
- `u` inside the resolver: keymap §2.6 does not bind it (the resolver is modal). `Esc u` from the resolver is the route.
- `d` after `t` in a shell merge (`NoStagesKept`, KNOWN_GAPS :613).
- A block take (`o`/`t`/`b` on a marker block) is an fs write, not a journal entry (graft-ops MAP §conflict :57): it is
  not what `u` restores. Only whole-file choices and `d` are.
- Shell-begun rebase, cherry-pick, revert (M7).

## The failing test first
- graft-ops `tests/it/finish.rs` (or a sibling `abort.rs`): `abort_of_a_shell_merge_is_journaled_and_undoable`.
  Setup: `git merge feature` in the shell stops on `notes.txt`; hand-edit `notes.txt`. `resume(Abort)`.
  Assert: no `MERGE_HEAD`; one `AbortMerge` entry. Then `undo`: `MERGE_HEAD` = `feature`, `git ls-files -u` lists
  `notes.txt`'s three stages, the file holds his hand edit byte-for-byte.
  Red today: `resume(Abort)` → `NotInProgress`.
- graft-ops `undo` tests: `undo_refuses_an_entry_from_before_a_live_merge`. A commit, then a shell merge stops; `undo`
  → `NotInvertible::OperationLive`, nothing written, no drift box. Red today: it plans the commit's undo (and `drift`
  says `a merge is in progress now`).
- graft `tests/it/resolver/shell_begun.rs`: `a_on_the_banner_aborts_a_shell_merge` and
  `u_on_the_banner_restores_a_whole_file_take_in_a_shell_merge`.

## Signatures
```rust
// graft-ops/src/operation.rs (+ its child the scout names)
pub enum Operation { /* … */ AbortMerge(AbortMergeParams) }
pub struct AbortMergeParams { pub merge_head: Oid }
pub enum Refusal { /* … */ AutostashWaits }       // + OctopusAbort { heads: usize } only if the scout says MergeState cannot hold them
// graft-ops/src/undo.rs
pub enum NotInvertible { /* … */ OperationLive { kind: InProgress } }
// graft-ops/src/apply/resume.rs: `resume(ctx, Resume::Abort)` with no live entry and a one-head merge → apply(AbortMerge)
// graft/src/update/undo.rs: signature unchanged
pub(super) fn in_progress_refusal(state: &AppState) -> Option<String>;   // None for a shell-begun merge
```

## Touches (each entry, and how the builder checks it)
- **Every exhaustive match on `Operation`, `Refusal`, `NotInvertible`** (describe, touches, plan, check, explain,
  journal kind names, the app's word tables).
  - Check: workspace-wide clippy; `grep -rn "NotInvertible::" crates` — each arm handles `OperationLive`.
- **Journal readers** — a new kind name in the journal (`kind_name`), the journal browser `J`, `interrupted_notice`,
  `plan_revert_to` (an `AbortMerge` is an operation: revertible like any other).
  - Check: `journal_round_trips_an_abort_merge_entry`; `J` shows `abort merge of feature` (a row test).
- **The undo gate reaches every executor entry point** (`undo`, `redo`, `revert_to`).
  - Check: `redo_refuses_while_a_rebase_is_live`, `revert_to_refuses_past_a_live_merge`,
    `redo_of_an_undone_abort_is_allowed_while_the_merge_is_back`.
- **Graft-begun merges and every other kind:** unchanged.
  - Check: the existing `u` refusal tests (`update::undo` tests, `it/resolver/*`) green unedited;
    `u_still_refuses_in_a_merge_begun_in_graft`.
- **The existing test** `a_hand_edit_taken_by_t_comes_back_after_the_merge_ends` still passes (the shell route still
  works); its NOTES-worthy change is that the shell step is no longer needed. Check: green.
- **Banner hints:** `a abort` already shows on a shell merge's banner? If it was hidden because it refused, it now shows.
  - Check: `every_key_the_bar_draws_is_answered`; the `m6-18-launch-shell-begun-*` captures regenerate if the bar
    changed (named in NOTES).
- **`u` in the changes pane or graph on a shell merge** — now an undo. Check: `u_in_the_graph_undoes_a_step_of_a_shell_merge`.
- **The walk** (`crates/graft/tests/it/m5_walk/`): two scenarios, `b-abort-shell-merge` (`a → ⏎`, 3) and
  `b-undo-a-take-in-the-shell` (start: the banner focused after whole-file `t` on `gone.txt` and `Esc`; keys `u`, 1).
  - Check: `journey_counts_match_the_note` with the rows below applied; `every_scenario::journey`.
- **TESTING.md "Times I left" :113** — the acceptance ticket closes it; the builder names the test that proves it.
- **KNOWN_GAPS :611, :612** — the conductor closes :611; :612 (the `UD` drawn with no merge) is now reachable only after
  an undo past the abort, and stays open unless a test here draws it.
- **Docs:** `crates/graft-ops/MAP.md` §apply (`AbortMerge`, the shell route), §undo (`OperationLive`, the gate);
  `crates/graft/MAP.md` §update::undo (`in_progress_refusal`), §update::banner (`a` on a shell merge).

## Acceptance (name the verifying test at tick time)
- [ ] `abort_of_a_shell_merge_is_journaled_and_undoable`: red first, then green.
- [ ] `undo_refuses_an_entry_from_before_a_live_merge`: red first, then green.
- [ ] `abort_refuses_while_git_holds_an_autostash`
- [ ] `abort_of_an_octopus_shell_merge` (allowed and undone, or refused in the words above: say which)
- [ ] `redo_refuses_while_a_rebase_is_live`, `revert_to_refuses_past_a_live_merge`,
      `redo_of_an_undone_abort_is_allowed_while_the_merge_is_back`
- [ ] `a_on_the_banner_aborts_a_shell_merge`, `u_on_the_banner_restores_a_whole_file_take_in_a_shell_merge`,
      `u_still_refuses_in_a_merge_begun_in_graft`
- [ ] `journey_counts_match_the_note` with `b-abort-shell-merge` = 3 and `b-undo-a-take-in-the-shell` = 1.
- [ ] Capture (rule 6; colour **unverified**, rule 5): `docs/shots/m6/fix-shell-merge-abort-and-undo-120x40.txt`,
      three frames: the abort box over the banner; the graph after `→ ⏎`; after `u`, the banner back.
  - **What right looks like:** frame 1 names `<into> goes back to before the merge · u puts the merge back`, cursor on
    `[ Cancel ]`; frame 2 has no `MERGING` and the bottom row names the abort; frame 3 shows `MERGING` and
    `CONFLICTS (2)` again and the status line starts `undo: abort merge of feature`.
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
Medium-large: a new operation kind, one safety gate, and the app's routing. One context; if it grows past it, split at
the seam: ops (operation + gate) first, app second.

## NOTES
