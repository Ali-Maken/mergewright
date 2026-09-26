# ops-undo-an-unmerged-index: the journal records index stages 1/2/3 as a tree, so `u` inverts a resolution made with no operation live, `U` puts a conflict back, and `d` starts a file over after it was already staged — without re-running anything

Status: done — piece A (first builder) and piece B / Scope 5(c) (second builder) both built. The `crates/graft` seam (see "Seam") closed in `1aadc4d`.
Review fixes (m6-04-08): M1 and M3 done in graft-ops; the `graft` seam test is updated and M1 is proven through the app — see "Review fixes, part 2" at the end. **Spec question open:** in a shell-begun merge the edit is recoverable only after leaving the app.
Blocked by: **03** (its `ResolveWholeFile`/`RestartConflict` are what become undoable; same crates). Groups 04–07 do **not** wait on this one; 07 (acceptance) gains one scenario from it — see "For group 07" at the end. The conductor adds **08** to 07's `Blocked by`.
Build with: Opus — undo and git index writes (CLAUDE.md routing). Not Fable: the schema, the tree layout, the restore order and the fold rule are all decided below, and every claim is checkable byte-for-byte against `git ls-files --stage` in a throwaway repo — a diff and a test can judge it. Fails Fable tests 2 and 3. Brownfield in `graft-ops` (`snapshot`, `undo`, `restore`, `apply`) and `graft-core` (`journal`): scout questions listed under Size.
Crates: graft-ops, graft-core
Needs:
- `ops-conflict-operations` (03) — `Operation::{ResolveWholeFile, RestartConflict}`, `RestartParams`, `Refusal::NotUnmerged`, and its acceptance tests `whole_file_is_its_own_entry_when_nothing_is_live` / `restart_brings_the_conflict_back` (whatever 03 shipped in their place — an `#[ignore]`, a KNOWN_GAPS row — is un-ignored/closed here).
- Existing (m3/m5): `snapshot::{take, commit_trees, move_backup_ref, index_tree, worktree_tree}`, `undo::{Step, PathScope, InversePlan, inverse, drift}`, `restore::execute`, `apply::{apply, resume, Applied}`, `journal::{Line, parse, render_pending, render_outcome, state_at}`.

## Answered before the build — Scope 5's question (conductor, 2026-09-22, git 2.54.0)

The spec files this as a question, correctly (rule 7: no line, so not a defect).
**It is now measured, so don't reason from the reading — build from this.**

**Bare `git checkout --merge -- <p>` DOES re-conflict a path already at stage 0**,
and in *both* states that matter:

- mid-merge (`MERGE_HEAD` present): stages 1/2/3 came back and the file on disk
  held markers again;
- **conflicted stash pop, no `MERGE_HEAD` and no operation in progress** — the
  case this group exists for (row 229) — same result, exit 0.

**The mechanism, which the spec does not name and should**: git's index carries a
**resolve-undo (REUC) extension**. `git add` on a conflicted path moves it to
stage 0 *and* records the three stages in REUC; `checkout -m` restores from
there. `git ls-files --resolve-undo` prints them, and they survive the commit
that follows.

Two things follow, and the second is the one that matters:

1. The spec's plan is right and its fallback is **not** belt-and-braces to delete.
   `stage_source` returning `Backup { tree }` is still needed.
2. **REUC is not a substitute for the stage tree.** It is one slot per path, it is
   overwritten by the next resolution of that path, and nothing in git promises it
   across a relaunch or a pruned index. The snapshot in Decision 1 is what makes
   this durable; REUC is what makes the *immediate* `d`-after-`⏎` work without one.
   Say which of the two answered each test in NOTES, with the git version — they
   are indistinguishable from a green test otherwise, and that is exactly the kind
   of agreement M5 taught us nothing was checking.

This also means **03's `restart_brings_the_conflict_back` tick is sound**, not
accidentally passing.

## Read first

- `docs/design-notes/m6-conflicts.md` — Hard-to-reverse choice 2 (a take is an fs write inside the live span, not journaled — **kept**), choice 3, Proposed change 2(c) (`d`).
- `docs/design/06-undo-journal-design.md` §6.1 (entry shape: `index_before`/`index_after` are tree ids; the backup ref), §6.2 (before → `Pending` → run → after), §6.3 (the combined snapshot), §6.4 (undo snapshots before it restores; a mid-flight operation is not an entry yet).
- ADR-0031 (the journal answers whole-repo state at a time — an unmerged index is a state like any other).
- `crates/graft-ops/MAP.md` §snapshot (`Snapshot.index_tree: Option<Oid>`, `SnapshotError::Unmerged`, `take`, `commit_trees`, `move_backup_ref`), §undo (`Step::RestoreIndex`, `PathScope`, `NotInvertible::NoBackup`, `RepoStateNow.index_tree`, `Drift`), §restore (`RestoreIndex` is `update-index --index-info`; `NoBefore`'s "real case"), §apply (`Applied`'s folded-step exception; the fold gate; the after-snapshot's parents), §undo (execution half) (the "reads as index drift; forcing past it still works" sentence — retired here).
- `crates/graft-core/MAP.md` §journal (`JournalEntry`, serde).
- `.scratch/m6-conflicts/issues/03-ops-conflict-operations.md` Scope items 2–3 (the fold rule for `ResolveWholeFile`; `RestartConflict` = `checkout --merge`).
- `KNOWN_GAPS.md` line 229 (redo of an undone conflicted stash pop loses the unmerged index).

## Decisions (the four the dispatch asked; the builder does not reopen them)

1. **What a snapshot of an unmerged index is: one tree, the stage tree.** `git ls-files --stage` on the index *copy* the snapshot already makes (never the real index), every line's path rewritten to `<stage>/<path>` at stage 0, fed to `git update-index --index-info` on a **second** temp index, then `write-tree`. Layout `{ 0/<path>…, 1/<path>…, 2/<path>…, 3/<path>… }`. No blob is written: every stage's blob is already an object (ours = `HEAD`'s, theirs = `MERGE_HEAD`/`REBASE_HEAD`/the stash commit's, base = the merge base's), and the tree makes them reachable from the backup ref regardless. Replay is the same channel in reverse: `ls-tree -r` → `update-index --index-info` on the real index, **a mode-0 removal line per path before its stage lines** (git's documented rule for placing higher-stage entries; without it stale stages survive). Working-tree bytes are **not** part of it: the existing worktree tree (index copy + `add -A`) already captures a conflicted file, markers and all (MAP §undo execution half). The journal records the tree's *shape* with its id — `IndexState::{Merged, Unmerged}` — so a stage tree can never be handed to `restore --source` or read as a plain tree by mistake. Bare-oid lines from M3–M5 parse unchanged.
2. **Steps inside a live operation do not get entries.** The one `Pending` entry stays one entry (m5/07; design note choice 2). But git's own `add` drops stages 1–3 for the path, and nothing else keeps them (ours/theirs/base blobs stay reachable; *which* blob was which stage does not). So **every folded step and every `resume` takes a before-snapshot commit chained under the live entry's backup ref** — `commit_trees(now)` with the ref's current commit as parent, then `move_backup_ref`. No journal line, no entry. The chain is what `d` reads after `⏎`, and it is a recovery point for a hand edit that `checkout --ours`/`rm` would otherwise overwrite mid-operation. The closing after-snapshot parents the chain tip, so the ref stays one linear chain: pre-op → before step 1 → … → after.
3. **`u` stays refused while an operation is live. No change to `02-keymap.md` §2.6a, no change to `in_force`.** The in-operation undo of one file's resolution is `d` (`RestartConflict`), which this group makes work *after* `⏎` — the case where git alone cannot (see the question under Scope 5). After the operation finishes, `u` undoes it whole, as today. With **no** operation live (a conflicted stash pop/apply — the resolver owns the state, KNOWN_GAPS 229), every resolution is its own entry (03's rule) and `u`/`U` walk them one at a time — that is where the reported `no backup was taken` refusal bites, and where it stops biting.
4. **Cost is linear in the index and there is no size refusal.** A stage tree is `ls-files --stage` + `update-index --index-info` + `write-tree` over the copy: the same order of work git already does to hold the index, no bytes copied; the worktree side hashes only stat-dirty files (15 ms / 5,000 files warm, MAP §snapshot). A conflict on 300 paths adds ~900 index lines. The only refusal is the existing `Stopped` (the stop token is checked before the snapshot and the ref move, as everywhere). The wide test prints its wall-clock into the report; it is not asserted (laptop-safe, PROMPT.md 7.1).

**Constraint kept:** nothing here re-runs the operation. The stages come from the recorded tree, never from replaying a merge.

## Scope

In:

1. **`graft_core::journal::IndexState`** — `Merged(Oid)` (serde: the bare oid string, exactly what every existing line holds) / `Unmerged { unmerged: Oid }` (serde: `{"unmerged":"<oid>"}`); `#[serde(untagged)]`, `Merged` listed first. `JournalEntry.index_before`/`index_after: Option<IndexState>`; `None` now means only *never recorded*. `PointInTime.index` follows. `render_pending`/`render_outcome` of a `Merged` entry produce **byte-identical** lines to before (no field renamed, no map where a string was).
2. **`snapshot`**: `index_tree` → `index_state` (the copy `write-tree`s → `Merged`; it refuses as unmerged → build the stage tree per Decision 1 → `Unmerged`). `SnapshotError::Unmerged` is retired (unreachable). `Snapshot.index: IndexState`. `commit_trees` takes `Option<IndexState>`; the commit's tree is `{ index/ }` for `Merged` (unchanged, so every backup ref written since M3 reads as before) and `{ unmerged/ }` for `Unmerged`, plus `worktree/` — never both index subtrees. `chain_backup(ctx, entry)`: Decision 2's chained commit. `stage_entries(index, path)`: the `(mode, oid, stage)` triples one path has in an `IndexState` (from `ls-tree` on `<stage>/<path>`, or stage 0 from a `Merged` tree); empty when absent. `index_paths_changed(from, to)`: the set of paths whose entry set differs — `diff-tree -r --name-only` with the stage prefix stripped and de-duplicated when both sides share a shape, one `ls-tree -r` per side otherwise.
3. **`facts::read_state_now`** → `RepoStateNow.index: IndexState` (never `None`, never an error for unmerged). `drift` compares `IndexState`s (`PartialEq`); `explain` says `the index changed` as today, `the index is unmerged now` when recorded `Merged` and now `Unmerged`, `the index is merged now` for the reverse.
4. **`undo`**: `Step::RestoreIndex { index: IndexState, paths: IndexPaths }` with `IndexPaths::{ChangedBetween { from, to }, Exactly(Vec<PathBuf>), Everything}`; `inverse` emits it from `index_before`/`index_after` whatever their shapes. `NotInvertible::NoBackup` no longer fires for an entry whose before-index was unmerged (the defect: `snapshot.rs:147` → `undo.rs:238`). **`restore::execute`** for `Unmerged`: `ls-tree -r <tree>` scoped to the paths, one `update-index --index-info` process, stdin = per path a `0 0000000000000000000000000000000000000000\t<path>` line then one `<mode> <oid> <stage>\t<path>` per entry (stage 0 alone when the target holds the path merged). Never `read-tree`, never `checkout --merge` here. The rollback half (`reverse_of`) restores over the same `IndexPaths`; `NoBefore { field: "the index" }` can no longer arise from an unmerged before-state (the variant stays for refs). A `Merged` target keeps today's path exactly.
5. **`apply`**: (a) the folding path calls `chain_backup` **before** the step runs (`Stage`/`Unstage`/`Discard` on any path, `ResolveWholeFile`, `RestartConflict` — anything whose `touches` names the index or the worktree) and `resume` calls it before `--continue`/`--skip`/`--abort`; `Applied.step_backup: Option<Oid>` carries the chained commit for a folded step; the journal file is untouched by it. (b) The closing after-snapshot's first parent is the ref's **current** commit, not the pre-op one. (c) **`RestartConflict` after `⏎`**: `stage_source(ctx, path)` → `Index` when `ls-files -u` still lists the path (plan as 03 wrote it: `checkout --merge -- <p>`), else `Backup { tree }` = the newest `Unmerged` stage tree holding a stage ≥ 1 for the path — the live entry's ref chain newest-first when an operation is live, else the journal's in-force entries newest-first (`index_after`, then `index_before`); plan = `RestoreIndex { Unmerged(tree), Exactly([p]) }` then `checkout --merge -- <p>` (git regenerates the markers from the restored stages: its own labels, its own `merge.conflictStyle`). `None` → `Refusal::NoStagesKept { path }`, words `<path>: the conflict's sides were not kept`. **Verify first, in a throwaway repo, and write the answer in NOTES with the git version:** does bare `git checkout --merge -- <p>` re-conflict a path already at stage 0 (my reading of `builtin/checkout.c::checkout_paths` is that stage-0 matches `continue` past the merge branch and the resolved bytes are simply checked out — unverified, so a question, rule 7). If git *does* re-conflict a stage-0 path, `stage_source` still returns `Backup` when the stages are gone from the index and the plan still restores them first (a relaunch or a pruned ref changes nothing then) — the one difference is that NOTES says the extra step was belt-and-braces. Either way the acceptance below holds.
6. **Row 229 closes here**, both halves: undoing a conflicted pop is clean without `force` when nothing moved (its `index_after` is now `Unmerged(t)` and `read_state_now` gives the same), and `U` after that undo puts the unmerged index back with its markers. The conductor moves the row.
7. **MAP.md**: `graft-core` §journal (`IndexState`); `graft-ops` §snapshot (`unmerged/` layout, `chain_backup`, `stage_entries`, `index_paths_changed`; retire `SnapshotError::Unmerged`), §undo (`IndexPaths`; retire the `NoBackup`-for-unmerged reading), §restore (the mode-0-first order; retire `NoBefore`'s index sentence), §apply (the chain rule and the after-snapshot's parent; `stage_source`), §undo execution half (retire "reads as index drift; forcing past it still works" — replace with what is now true).

Out: any key or view (the app's `u`-while-live refusal is G04/G06 territory and is not touched); journal lines for folded steps or for takes (design note choice 2 stands); showing the chain in the journal browser (M7); rerere; retention of chained commits beyond what the backup ref already gives (§6.8 applies to the ref as a whole); a "too large" refusal (Decision 4: none exists).

## Signatures

```rust
// graft-core/src/journal.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IndexState {
    Merged(Oid),                    // the plain tree `write-tree` gives; serde: the bare oid string (every pre-M6 line parses unchanged)
    Unmerged { unmerged: Oid },     // the stage tree `{ 0/<path>, 1/<path>, 2/<path>, 3/<path> }`; serde: {"unmerged":"<oid>"}
}
impl IndexState { pub fn tree(&self) -> Oid; pub fn is_unmerged(&self) -> bool; }
pub struct JournalEntry { /* … */ pub index_before: Option<IndexState>, pub index_after: Option<IndexState>, /* … */ }

// graft-ops/src/journal.rs
pub struct PointInTime { /* … */ pub index: Option<IndexState>, /* … */ }

// graft-ops/src/snapshot.rs   (parameter order per the crate's convention in MAP §snapshot; the types are what is fixed)
pub struct Snapshot { pub index: IndexState, pub worktree_tree: Oid, pub commit: Oid }
pub fn index_state(repo: &RepoHandle, git: &Git, index_copy: &Path, scratch: &Path) -> Result<IndexState, SnapshotError>;
pub fn commit_trees(repo: &RepoHandle, git: &Git, index: Option<IndexState>, worktree: Option<Oid>, parents: &[Oid]) -> Result<Oid, SnapshotError>;
pub fn chain_backup(ctx: &Context, entry: &JournalEntry) -> Result<Oid, SnapshotError>;   // commit_trees(now) parented on the ref's current commit, then move_backup_ref; the commit id
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub struct StageEntry { pub mode: u32, pub oid: Oid, pub stage: u8 }
pub fn stage_entries(repo: &RepoHandle, git: &Git, index: IndexState, path: &Path) -> Result<Vec<StageEntry>, SnapshotError>;
pub fn index_paths_changed(repo: &RepoHandle, git: &Git, from: IndexState, to: IndexState) -> Result<BTreeSet<PathBuf>, SnapshotError>;
pub enum SnapshotError { Run, Io, NotAnOid { what, printed }, Stopped }   // `Unmerged` retired

// graft-ops/src/facts.rs
pub struct RepoStateNow { pub head: HeadRef, pub refs: BTreeMap<String, Option<Oid>>, pub index: IndexState, pub worktree_tree: Oid }

// graft-ops/src/undo.rs
pub enum Step { /* … */ RestoreIndex { index: IndexState, paths: IndexPaths }, /* RestoreWorktree unchanged */ }
pub enum IndexPaths { ChangedBetween { from: IndexState, to: IndexState }, Exactly(Vec<PathBuf>), Everything }
pub struct Drift { /* … */ pub index: Option<(IndexState, IndexState)>, /* … */ }

// graft-ops/src/apply.rs
pub struct Applied { /* … */ pub step_backup: Option<Oid> }   // Some only for a folded step: the chained commit taken before it ran
pub enum StageSource { Index, Backup { tree: Oid } }
pub fn stage_source(ctx: &Context, path: &Path) -> Result<Option<StageSource>, FactsError>;

// graft-ops/src/operation.rs
pub enum Refusal { /* … */ NoStagesKept { path: PathBuf } }
```

## Acceptance

Ticks name the test that verifies them. Anything not observed stays unticked
with the reason, per rule 1.

- [x] A JSONL line written by M5 with a bare-oid `index_before` parses to `Merged`; `"index_before": null` parses to `None`; an `Unmerged` entry round-trips; a **real 6-line golden captured from the pre-change renderer** re-renders byte-identically — `graft-core` `journal::tests::index_state_serde_is_backward_compatible`; `graft-ops` `tests/it/unmerged_index.rs::merged_lines_render_unchanged` against `tests/it/goldens/m5-journal.jsonl`.
- [x] `take` over an unmerged index returns `Unmerged(t)`; `ls-tree -r t` holds `1/`, `2/`, `3/` for exactly the conflicted path and `0/` for every other; `RestoreIndex { Unmerged(t), Everything }` reproduces `git ls-files --stage` **byte-for-byte** after the index has been wrecked with `add -A` — `tests/it/unmerged_index.rs::unmerged_index_round_trips`. **Deviation, stated:** the fixture is a merge built in-test with plain `git` (1 conflicted path + 2 clean), not `Shape::StoppedMerge`'s 11 files; the 900-line case is `three_hundred_conflicted_paths` and all seven `UnmergedHow` shapes are exercised by `conflict_ops.rs`'s own fixture.
- [x] A clean index still gives `Merged(write-tree)` and a commit whose tree is `index/` + `worktree/` with no `unmerged/`, the `index/` id equal to `write-tree`'s — `tests/it/unmerged_index.rs::merged_layout_unchanged`.
- [x] A whole-file resolution with nothing live is undoable: after `u`, `ls-files -u` lists the path with the same stage oids and modes and the bytes equal the pre-resolution bytes — `tests/it/conflict_ops.rs::undo_of_a_resolution_restores_the_stages` (this **replaces** `undo_after_a_whole_file_resolution_with_nothing_live`, which pinned the defect).
- [x] Stash-pop conflict, nothing live: resolve → `u` → `U` → `u`, `u` → `U`, with `ls-files -u`, the file bytes and `status --porcelain` read from `git` at every step — `tests/it/unmerged_index.rs::pop_conflict_undo_redo_walks_both_ways`.
- [x] Undoing a conflicted pop with nothing touched since shows **no** drift and needs no `force` — `tests/it/unmerged_index.rs::conflicted_pop_undo_is_clean_without_force`; `src/undo/tests.rs::drift_says_which_way_the_index_shape_went` for the two new words.
- [x] A folded step moves the entry's backup ref to a commit whose first parent is the old target and whose `unmerged/` holds the path's three stages; the journal file's line count is unchanged; `Applied.step_backup == Some(that commit)` — `tests/it/chained_backup.rs::folded_step_chains_a_backup`. **Deviation:** a stopped merge Graft ran, not `Shape::StoppedRebase`.
- [x] **The g03 review's M2 data-loss path**: hand-edit a `UD` file, then take theirs (`git rm --quiet`, no confirm, no journal line) → the edit's bytes are in the chained commit's `worktree/` tree — `tests/it/chained_backup.rs::a_hand_edit_is_recoverable_after_the_other_side_is_taken`. The false comment at `apply/resume.rs:64-66` is replaced by what is now true.
- [x] `StoppedMerge`: `Stage shared.txt` (folded) then `RestartConflict shared.txt` → `ls-files -u` lists it with the **original** stage oids and modes byte for byte, and the file holds markers — `tests/it/chained_backup.rs::restart_after_stage_brings_the_conflict_back`. **This test's outcome does not discriminate and says so in its own doc comment**: measured by disabling the restore and re-running, git's REUC answers it. Its last assertion (the `update-index --index-info` line in `Applied.entry.commands`) is what catches that, and does fail without the restore.
- [x] The same with the chain read fresh **and REUC cleared** — the index rebuilt from its own tree, the control (`git ls-files --resolve-undo` empty, and bare `git checkout --merge -- shared.txt` then exits 0 and leaves `ls-files -u` empty) asserted inside the test — `tests/it/chained_backup.rs::restart_survives_a_relaunch`. Checked by disabling the restore and re-running: `ls-files -u` comes back **empty**, so the recorded stage tree and nothing else answers this one. Every `step` opens a fresh `Context` over a freshly read journal, which is the relaunch.
- [x] `StoppedMerge`, no chain commit holding the path (the entry's backup ref reset to the pre-operation commit) → `RestartConflict` refuses `NoStagesKept { path }`; the index line, the file's bytes and the journal's line count are all unchanged — `tests/it/chained_backup.rs::restart_refuses_without_stages`.
- [x] `stage_source`'s `Index` arm and the two-place gate: a restart of a path resolved inside a **shell-started** merge (no entry of Graft's, so no chain) is refused `NoStagesKept`, while a whole-file resolution of the same path is still `NotUnmerged` — `tests/it/conflict_ops.rs::conflict_operations_refuse_a_path_that_is_not_conflicted`. The stage-set → `UnmergedHow` mapping the runtime half of the gate rests on: `src/operation/tests.rs::a_stage_set_spells_the_shape_it_came_from` (all seven shapes plus the empty set, and `has_markers` over each). The pure half unchanged for an unmerged path: `src/operation/tests.rs::conflict_operations_check_against_the_status`, `tests/it/conflict_ops.rs::restart_refuses_the_shapes_checkout_merge_cannot_merge`.
- [x] `Refusal::NoStagesKept`'s words — `src/operation/tests.rs::conflict_refusals_in_words`.
- [x] `resume(Continue)` chains a backup of its own first and the closing after-commit's first parent is that chain tip, so every chained commit stays reachable (`merge-base --is-ancestor`) — `tests/it/chained_backup.rs::resume_chains_then_after_snapshot_parents_the_chain`; m5's `history_ops` tests unchanged and green.
- [x] Chained commits add no entries: same line count, same entry count, same `next_undo` — `tests/it/chained_backup.rs::chain_adds_no_entries`.
- [x] `RestoreIndex(Unmerged)` stdin bytes, mode-0 record first, one process per step — `src/restore/steps.rs::tests::unmerged_index_info_lines`, plus `a_path_with_no_stages_is_only_removed` and `a_merged_path_is_one_stage_zero_line`.
- [x] 300 conflicted paths, generated in-test with plain `git`: round trip byte-for-byte over 900 index lines. **Measured: `stage tree: 85 ms, restore: 172 ms`**, whole test 0.65 s — `tests/it/unmerged_index.rs::three_hundred_conflicted_paths`.
- [x] Stop raised before the chained snapshot → `Stopped`, the ref not moved, the step not run — `tests/it/chained_backup.rs::stop_before_chain_writes_nothing`.
- [x] `tests/it/seam_rule.rs` unchanged and green — **run by the conductor after closing the seam**, not by the builder, which does not hold `crates/graft`: `cargo nextest run -E 'test(/seam_rule|app_never_builds/)'` on the merged tree, 2 tests, 2 passed. The file itself is unchanged by this group. No new writer was exposed (`chain_backup` and `stage_entries` read; `index_paths_changed` reads).
- [x] `cargo test --doc` covers the `IndexState` doc example — 1 doctest, passing.
- [x] MAP.md per Scope 7, for what was built — `graft-core` §journal (`IndexState`); `graft-ops` §snapshot (`index_state`, the `unmerged/` layout, `chain_backup`, `StageEntry`/`stage_entries`, `index_paths_changed`, `SnapshotError::Unmerged` retired), §undo (`IndexPaths`, the `NoBackup` reading retired), §restore (mode-0-first, `NoBefore`'s index sentence retired), §apply (`step_backup`), §undo execution half ("forcing past it still works" retired), and `RestartParams`' gate. Completed for 5(c): §apply (`StageSource`/`stage_source`, `prepare_restart`/`run_restart_prep`), §snapshot (`recorded_index_state`, `backup_chain`), §restore (`restore_stages`), §operation (`NoStagesKept`, `shape_of_stages`, and `RestartParams`' two-place gate).

### The g03 review's M1, taken as instructed

- [x] `RestartConflict` is gated on `UnmergedHow::has_markers`, not on "is it unmerged", and refuses `NotMarkerShape { path, how }` otherwise — `src/operation/conflict.rs::check_restart`; `src/operation/tests.rs::conflict_operations_check_against_the_status` (the `Ok(())` it pinned is flipped); `tests/it/conflict_ops.rs::restart_refuses_the_shapes_checkout_merge_cannot_merge`, which runs the **same `git` the plan would have run** on each shape first, so the refusal is measured against git's own answer.
- [x] **Measured, git 2.54.0 (Apple Git-157), all seven shapes** from the rename/rename fixture: `UU` exit 0 (3 stages back), `AA` exit 0 (2 stages), and `UD` `DU` `AU` `UA` `DD` all **exit 1, `does not have all necessary versions`**. The reviewer's "`AA` may stay allowed" is confirmed. MAP.md §`RestartParams` corrected.

## Gates

fmt / clippy / nextest / doc, workspace-wide.

## Size

One context; ~600 lines of code across `graft-core::journal`, `graft-ops::{snapshot, facts, undo, restore, apply}` plus ~450 of tests. **If the context compacts, split at the seam**: piece A = Scope 1–4 + 6 (the schema and the round trip: fixes the reported defect and row 229), piece B = Scope 5 (the chain and `stage_source`: `d` after `⏎`); A stands alone.

Scout questions (brownfield; answer in NOTES before writing, from MAP first, source only where MAP is silent): (1) who reads the backup ref's *target commit* as the before-state rather than the entry's `index_before`/`worktree_before` oids — anything that does must be pointed at the chain root or the fields (`AfterState::BackupGone`, `Unsealed`, `rollback::begin`); (2) which parents the closing after-snapshot passes today, so (b) changes one argument; (3) every match on `Snapshot.index_tree`/`RepoStateNow.index_tree`/`index_before` as `Option<Oid>` — the compiler lists them, the scout says which are logic (drift, `state_at`, rollback) versus plumbing; (4) the m5 test corpus of journal lines, so the golden for `merged_lines_render_unchanged` is real.

## For group 07 (the conductor adds this; not an edit to 07 from here)

- New scenario `undo-a-resolution-in-a-stash-pop-conflict`: m5's stash-pop shape; `⏎` from the pane; `o`, `⏎` → `all conflicts resolved`; `u` → the pane's `CONFLICTS` row is back, `ls-files -u` lists the path, the file bytes equal the bytes before `⏎`; `u` → the pop is undone (clean tree, stash still listed); `U` → the conflict is back, markers on disk and `ls-files -u` non-empty. Truth from git, never from `AppState`. What right looks like: `git ls-files -u` prints three lines for `p` after the first `u`, none after the second, three again after `U`.
- `start-over-brings-the-markers-back` (already in 07) depends on this group for its "after `⏎`" step; its `what right looks like` line gains `git ls-files -u` lists the path with the same stage oids as before the take.


## NOTES (builder, 2026-09-22)

**Scout: none.** The conductor judged the spec's "Read first" list sufficient
and asked for a verdict. It was sufficient for `snapshot`, `undo`, `restore`
and `journal` — MAP.md named every type the compiler then listed. It was
**not** sufficient for `apply/resume.rs`: the fold path's shape (where the
live entry is looked up, and that it was looked up *after* the plan ran) had
to be read from source, and that reading is what made the g03 review's M2 fix
a three-line move rather than a redesign. Cost: one file read. Not worth a
scout round-trip in hindsight.

**Scout questions, answered:**

1. *Who reads the backup ref's target commit as the before-state rather than
   the entry's fields?* `apply::seal_after` (via `Pending.before_commit`) and
   `resume::close_resume`, which already re-reads the ref. `rollback::begin`
   and `AfterState::BackupGone` use the entry's own fields. `seal_after` now
   reads the ref's **current** target as the first parent (Scope 5(b)), so the
   chain cannot be orphaned; it falls back to the pre-op commit when the ref
   cannot be read, which is exactly today's behaviour.
2. *Which parents does the closing after-snapshot pass?* `[before_commit] +
   anchors(head_after, refs_after)`. One argument changed.
3. *Every match on the `Option<Oid>` index fields.* All plumbing except three:
   `undo::drift` (now compares `IndexState`s and says which way the shape
   went), `undo::generic_steps` (the defect itself — `index_before` being
   `None` was `NotInvertible::NoBackup`), and `restore::rollback::reverse_of`.
4. *The m5 golden.* Captured from the **pre-change** renderer into
   `crates/graft-ops/tests/it/goldens/m5-journal.jsonl`: 6 real lines, 3
   entries (stage / commit / stash_push), bare-oid `index_before`.

**REUC or the stage tree — which answered each test, git 2.54.0 (Apple
Git-157).** This is the line the dispatch asked for.

- Every restore test in `unmerged_index.rs` and `conflict_ops.rs` is answered
  by **the stage tree**, and REUC cannot be confounding them: the restore is
  one `update-index -z --index-info` reading `ls-tree` records out of the
  recorded tree. No `checkout --merge` runs, and `checkout -m` is the only
  thing that consults resolve-undo. `pop_conflict_undo_redo_walks_both_ways`
  and `undo_of_a_resolution_restores_the_stages` compare `git ls-files -u`
  byte-for-byte, so a wrong stage would show.
- `conflict_ops::restart_brings_the_conflict_back` (g03's, untouched) is
  answered by **neither**: I re-read it rather than assuming. It writes ours'
  bytes to disk and never stages them, so the path is still at stages 1/2/3
  and `checkout --merge` merges straight from the **index**. REUC is not
  consulted. My first draft of this note said "REUC", which was wrong; the
  test's own body disproves it.
- **REUC is what would answer the case 5(c) is for**, and it was measured on
  its own, outside the repo, git 2.54.0: after `git add` on a conflicted path
  (stage 0), bare `checkout --merge -- <p>` re-conflicts it — mid-merge and in
  a conflicted stash pop with no `MERGE_HEAD` alike, exit 0 both times, and
  `git ls-files --resolve-undo` prints the three stages. Control: with REUC
  cleared (`rm .git/index; git read-tree HEAD; git checkout-index -a -f`) the
  same command exits **0 and silently does nothing** — `ls-files -u` stays
  empty. That control is the only thing that tells the two apart, and it is
  why the missing `restart_survives_a_relaunch` has to clear REUC to mean
  anything.
- `restart_refuses_the_shapes_checkout_merge_cannot_merge` is answered by
  **git itself**: it runs the raw command on each shape before asserting the
  refusal.
- The chain tests are answered by **the stage tree**: they read `unmerged/`
  and `worktree/` out of the chained commit with `ls-tree` and `cat-file`.
- The distinguishing experiment for Scope 5(c) is therefore: clear REUC (write
  the merged index's tree, `rm .git/index`, `read-tree <that tree>`), then
  restart. That is the test `restart_survives_a_relaunch` should be, and it is
  **not written**, because 5(c) is not built.

**What was NOT observed:**

- `AU`/`UA`/`DD` restart behaviour was measured (exit 1) but only from the
  rename/rename fixture; no other merge shape was tried.
- Nothing was run under `--release`, on Linux, or on any git but 2.54.0.
- Colour, rendering and every app-crate behaviour: out of scope and
  uncompilable here (see the seam).
- `graft`'s `tests/it/seam_rule.rs` was not run.

**Deviations from the spec's signatures**, all deliberate:

- `chain_backup` returns `Result<Option<Oid>, _>`, not `Result<Oid, _>`: an
  entry with no backup ref (an operation begun in the shell) is a legitimate
  "nothing to chain onto", and inventing an error variant for it would make
  the caller swallow it.
- The spec's `snapshot::tests::*` / `apply::tests::*` names live in
  `tests/it/` per this crate's rule that everything spawning `git` is an
  integration test; the test names are the spec's.
- `stage_entries` sorts by **stage**, not by `StageEntry`'s derived `Ord`
  (mode, then oid) — the derived order puts three stages of one path in blob
  order, which is to say at random. Found by a test, not by reading.

**A latent flaw left alone, named here:** `restore/steps.rs::scoped_paths`
maps the worktree side's `snapshot::worktree_tree` failure with
`.map_err(RestoreError::Snapshot)`, which turns a cancel into
`Snapshot(Stopped)`. The cancel/rollback path (m5/06) keys on
`RestoreError::Stopped` alone, so a cancel there would be left half-applied.
The index side had the same flaw and is fixed (`as_snapshot_error`); the
worktree side is pre-existing and outside this ticket.

**Also noted:** index-side path scoping now goes through `snapshot::*`, so its
`diff-tree`/`ls-tree` command lines no longer reach the Command Log. The
worktree side still records its own. Not a regression this ticket introduced
in substance — the old code scoped from trees the same way — but the lines
that used to appear no longer do.

## Handoff — what remains (Scope 5(c))

Everything else is done and committed. What is left is one coherent piece:

1. `apply::StageSource { Index, Backup { tree: Oid } }` and
   `stage_source(ctx, path) -> Result<Option<StageSource>, FactsError>`:
   `Index` when `ls-files -u` still lists the path; else walk the live entry's
   backup ref chain newest-first (`rev-list <ref>`), then the journal's
   in-force entries newest-first (`index_after`, then `index_before`), and
   take the first `IndexState::Unmerged` whose `stage_entries(path)` has an
   entry with `stage >= 1`. All the reading blocks exist:
   `snapshot::stage_entries` takes an `IndexState` and a path.
2. `Refusal::NoStagesKept { path }`, words `<path>: the conflict's sides were
   not kept`, for `stage_source` → `None`.
3. The plan for `RestartConflict` when the path is **not** unmerged now:
   `RestoreIndex { Unmerged(tree), Exactly([p]) }` then `checkout --merge --
   <p>`. This is the only real design work left, and it is real: `plan()`
   returns `Vec<Invocation>`, which cannot carry an index restore (it needs
   stdin and a recorded tree). Either `inside_live_operation` special-cases
   `RestartConflict` and calls a new narrow `restore::restore_stages(ctx,
   index, path)` before running the plan, or `Invocation` grows stdin. The
   first is smaller and keeps `plan()` pure.
4. **Watch the interaction with M1's new gate.** `check_restart` now requires
   the path to be unmerged *and* `has_markers()`. After `⏎` the path is at
   stage 0, so there is no `UnmergedHow` to read at all — the check must
   admit a stage-0 path exactly when `stage_source` returns `Backup`, and
   must still refuse a one-sided shape. Deciding that gate is the first thing
   the next builder should settle, before writing code.
5. Tests: `restart_after_stage_brings_the_conflict_back`,
   `restart_survives_a_relaunch` (**with REUC cleared**, per the experiment
   above — otherwise it proves nothing), `restart_refuses_without_stages`.
6. MAP.md §apply gains `stage_source`; §operation gains `NoStagesKept`.

## Seam — `blocked: needs graft`

`crates/graft` does not compile against this branch. `crates/graft-ui` is
unaffected. Every site and its fix:

| File:line | Fix |
|---|---|
| `src/update/journal.rs:366-367` | `Some(oid(1))` → `Some(IndexState::Merged(oid(1)))` (and `oid(2)`) |
| `src/view/journal.rs:261-262` | same |
| `src/view/tests.rs:2537-2538` | same |
| `src/jobs/undo.rs:530`, `:569` | add `step_backup: None,` to the `Applied` literal |
| `src/update/banner/tests.rs:54` | same |
| `src/update/operate.rs:1008` | same |
| `src/update/undo.rs:525` | same |
| `src/update/undo.rs:552`, `:794` | `Drift { index: true }` → `Drift { index: Some((IndexState::Merged(a), IndexState::Merged(b))) }`, two different oids |
| `src/view/tests.rs:2951` | same |

New public shapes the app can now use: `IndexState::{Merged, Unmerged}` with
`tree()` / `is_unmerged()`; `Drift.index: Option<(IndexState, IndexState)>`;
`RepoStateNow.index: IndexState`; `Applied.step_backup: Option<Oid>`;
`Refusal::NotMarkerShape` is now also returned for `RestartConflict`, so `d`'s
refusal path in the resolver must expect it.

## NOTES (second builder, Scope 5(c), 2026-09-22)

**Scout: none.** The handoff above named every file and every open question,
and the compiler named the rest. A scout would have re-read what it already
said. Cost of not having one: nothing found late.

### The gate, settled before the code (the first thing the dispatch asked)

`check_restart` is pure over `RepoFacts` and **cannot spawn**, by the four-
properties contract. After `⏎` the path is at stage 0, so there is no
`UnmergedHow` to read at all, and whether its sides were kept lives in a
recorded stage tree that only a spawn reaches. So the gate splits along what
each half can see, and stays **one rule** — *`checkout --merge` needs both
sides*:

- **`check_restart`** (`src/operation/conflict.rs`): the status calls the path
  unmerged → decide exactly as the g03 review's M1 left it,
  `UnmergedHow::has_markers()` or `NotMarkerShape { path, how }`. It does
  **not** → `Ok(())`. `NotUnmerged` is no longer a restart's refusal.
- **`apply::prepare_restart`** (`src/apply.rs`), before a byte is written:
  `stage_source` → `None` is `NoStagesKept`; `Backup { tree }` has the same
  test applied to the stages actually kept, via `operation::shape_of_stages`
  (1+2+3 `UU`, 2+3 `AA`, 1+2 `UD`, 1+3 `DU`, 2 `AU`, 3 `UA`, 1 `DD`) →
  `has_markers()`, else `NotMarkerShape` with the derived shape.

So the stage-0 path is admitted by `check` **exactly when** `stage_source`
returns `Backup` *and* what it found is a marker shape — decided one step
later, where the answer exists, and still before any git that writes. A
one-sided shape is refused wherever its stages live.

**Order matters and is not arbitrary:** the restore runs **after** the
before-snapshot (`begin`) or the chained backup, never before. Snapshot first
means `u` on the restart puts back the *resolution* the user is starting over
from. Restore first would have recorded the stages as the before-state and
put the stage-0 resolution out of undo's reach. Both call sites do it in that
order; `prepare_restart`/`run_restart_prep` is the split that makes it
possible without a refusal leaving a `Pending` line behind.

### The design call held up

`inside_live_operation` (and `apply`'s non-live path) special-case
`RestartConflict` and call `restore::restore_stages(ctx, index, path)` before
running the plan. **`plan()` stayed pure and still returns
`Vec<Invocation>`.** Cost: one `RestartPrep` enum, two small functions, one
extra match arm at each of two call sites — about 40 lines. Nothing pushed
back. `Invocation` was not touched.

**Both** call sites are wired, not only the live one, and that is not scope
creep: relaxing `check_restart` means a stage-0 path now reaches `apply` in
the no-operation case too (a conflicted stash pop), and without
`prepare_restart` there it would have run `checkout --merge` on a stage-0
path — exit 0, nothing done, no word to the user. The refusal has to be where
the admission is.

### Which mechanism answered each test — git 2.54.0 (Apple Git-157), macOS

Measured, not reasoned: each test was re-run with `run_restart_prep`'s restore
arm disabled.

| test | answered by | how that is known |
|---|---|---|
| `restart_after_stage_brings_the_conflict_back` | **git's REUC** | With the restore disabled the stages and markers still came back. Only the `update-index --index-info` assertion on `Applied.entry.commands` failed. Recorded in the test's own doc comment so nobody reads it as proof. |
| `restart_survives_a_relaunch` | **the recorded stage tree, and nothing else** | REUC cleared inside the test (`write-tree`, `rm .git/index`, `read-tree <tree>`); `ls-files --resolve-undo` asserted empty and bare `checkout --merge` asserted to exit 0 and leave `ls-files -u` empty. With the restore disabled, `ls-files -u` after the operation came back **empty** — the outcome itself fails, not just the mechanism assertion. |
| `restart_refuses_without_stages` | **neither** | It refuses in `prepare_restart` before any git that writes; `stage_source` walks the chain (reset to the pre-operation commit, whose index is `Merged`) and finds nothing. Index line, file bytes and journal line count all asserted unchanged. |
| `conflict_operations_refuse_a_path_that_is_not_conflicted` | **neither** | Same refusal path, for a shell-started merge with no entry of Graft's. |
| `a_stage_set_spells_the_shape_it_came_from` | **neither** | Pure, no repository. |

### A trade-off this makes, named

`stage_source` deliberately does not consult REUC. So there is a case Graft
now refuses that bare git would have done: a path staged outside Graft, or
inside a merge Graft did not start, where git still holds resolve-undo. The
user gets `<path>: the conflict's sides were not kept` instead of the conflict
back. That is the ticket's decision (REUC is one slot per path, overwritten by
the next resolution and gone with a rebuilt index — Decision under "Answered
before the build", point 2), and the refusal is in words rather than a silent
no-op, but it is a real narrowing and the conductor should know it.

### What was NOT observed

- Nothing run under `--release`, on Linux, or on any git but 2.54.0.
- `restart_survives_a_relaunch` simulates the relaunch with a fresh `Context`
  per step over a freshly read journal. **No process was actually restarted.**
- The **app** side of any of this: `crates/graft` does not compile against
  this branch (the seam below, unchanged by this session — still 14 errors,
  all the first builder's, and `Refusal::NoStagesKept` broke no exhaustive
  match, so the seam table needs no new row). `tests/it/seam_rule.rs` still
  not run.
- `stage_source`'s **journal** arm (no operation live) has no integration test
  of its own reaching `Backup`: the tests that exercise the no-live path
  (`conflict_operations_refuse_a_path_that_is_not_conflicted`) reach `None`.
  The code path is the same walk over `IndexState`s that the chain arm uses,
  but that is a reading, not an observation. Filed here rather than ticked.

### Deviation from the spec's signatures

`stage_source(ctx, facts, path)`, not `stage_source(ctx, path)`. `apply` has
already read the status one line earlier; a second `ls-files -u` would be a
wasted spawn that could disagree with the `check` that just ran. The return
type is the spec's.

### Gates

`cargo fmt --check`, `cargo clippy --workspace --exclude graft --all-targets
-D warnings`, `cargo nextest run --workspace --exclude graft` (1436 passed, 1
skipped), `cargo test --doc --workspace --exclude graft` — all green. **Not
workspace-wide**: `graft` is held by another builder and does not compile
against this branch. That is a green I did not get, and this says so.

## Review fixes (m6-04-08) — builder, 2026-09-23, branch `m6-08-fix`

Findings: `docs/reviews/m6-04-08.md`, Majors **M1** and **M3** (the g08 ones).
A scout was used (the fold path and the file layout). This session
**compacted once**, between M1's commit and M3's; nothing of M3 was lost —
it was gated after the compaction.

### M1 — a whole-file take or `d` in a merge begun in the shell took no backup

**Cause, not symptom.** `apply::resume::folded_step` (now
`apply/fold.rs`) sent every conflict step taken while git had an operation
live into `inside_live_operation`, and chained a backup only onto a live entry
of Graft's. A merge begun in the shell has none (`live_entry` → `None`), so a
`t` (`git rm` / `checkout --ours|--theirs`) or a `d` (`checkout --merge`)
wrote over the working tree with no journal line and no backup ref. A hand
edit in the file was gone.

**Fix.** Ticket 03's scope 2 rule, which the fold had skipped: "folds into the
live entry **when one is live**; with no live entry … it is its own entry,
undoable through the normal backup". `fold::folded_step` now returns `None`
when there is no live entry *and* the step can write the working tree
(`Operation::touches(facts).worktree` — `ResolveWholeFile`, `RestartConflict`),
and `apply` runs it as an ordinary entry: `Pending` line, full snapshot under
`refs/graft/backup/<wt>/<ulid>`, outcome line. Index-only steps (`Stage`,
`Unstage`) still fold into nothing — they move no bytes, and the index they
change is git's own record of the merge.

This is **not** the review's suggested fix (a standalone backup ref with no
journal line, or a refusal). A ref with no line has no way back that the app
knows about; an entry is reachable by `u`. It is the dispatch's "stash-pop
precedent" (a `ResolveWholeFile` with nothing live is its own entry) and
03's written rule, so it was built, not asked.

**Proving tests** (red before the fix — journal ids `[]` against `[id]` —
green after, run through a real repository, `nextest`):
- `graft-ops` `tests/it/shell_begun_merge.rs::a_hand_edit_survives_taking_the_other_side`
  — hand edit on `theirs-deleted.txt`, take theirs, file gone; one
  `ResolveConflict` entry, `Committed`, with a backup ref; `undo::undo` gives
  the edited bytes back and the same `ls-files -u` stages; `MERGE_HEAD` still
  there.
- `…::a_hand_edit_survives_starting_the_file_over` — hand edit on `both.txt`,
  `RestartConflict`, markers back; one `RestartConflict` entry with a backup
  ref; undo gives the edit and the stages back.
- `…::a_stage_writes_no_line` — `Stage` in the same merge: journal still empty,
  index resolved, bytes unchanged.

**Tests changed, with the reason:**
- `conflict_ops::whole_file_folds_into_live_entry` **removed**. It asserted
  the defect (no entry in a shell-begun merge). Its index-only half is
  `a_stage_writes_no_line`; its whole-file half is inverted by
  `a_hand_edit_survives_taking_the_other_side`.
- `conflict_ops::restart_refuses_the_shapes_checkout_merge_cannot_merge` —
  its two final restarts are now two entries, so they take distinct
  timestamps (`NOW`, `NOW + 1`); same `NOW` opened one ULID twice.
- `conflict_ops::conflict_operations_refuse_a_path_that_is_not_conflicted` —
  doc only, below.

**Cost to the user.**
- In the app, `u` is refused while any operation is live
  (`crates/graft/src/update/undo.rs:103 in_progress_refusal`). The edit is
  **safe but not reachable** until the merge ends in the shell (`git commit` /
  `git merge --abort`); then `u` takes it back, possibly behind the drift
  confirm. Observed only at the ops level (`undo::undo` has no in-progress
  gate); the app path is **unverified** — `graft` is not this group's.
- Each `t` / `d` in a shell-begun merge adds one journal row.
- **Follow-up (refusal, not loss):** `d` after a `t`/`⏎` in a shell-begun merge
  still refuses `NoStagesKept`. `restart::stage_source` reads the **chain**
  while an operation is live and ignores the take's own entry, which now holds
  the stages. Read, not observed by a dedicated test. Not fixed here — scope.

**Seam — `blocked: needs graft`.** One app test asserts the defect and now
fails (the only failure workspace-wide):
`crates/graft/tests/it/resolver/restart.rs:151 no_entry_added_in_a_shell_begun_merge`.
Observed: `restart.rs:166` left 1, right 0. The change it needs:
- line 168 `before` → `before + 1` (observed);
- line 206 `before` → `before + 2` (**expected, not observed** — the run
  stops at 166);
- rename, e.g. `each_whole_file_step_is_its_own_entry_in_a_shell_begun_merge`;
- doc (143-149): the step is its own entry with its own backup (m6 g04/g08
  review, M1); `apply::resume::inside_live_operation` is now
  `apply::fold::inside_live_operation`.

### M3 — `snapshot.rs` 976 and `apply.rs` 1230 lines, against PROMPT §6.2's ~400

**Cause, not symptom:** split by concern (`codebase-design`), private
submodules, every public item re-exported so no path outside the module moved.
No behaviour change: bodies moved whole; the edits are `use` lines,
`pub(super)` visibility and one call path (`resume::folded_step` →
`fold::folded_step`).

| File | Lines | Holds |
|---|---|---|
| `snapshot.rs` | 300 | types, `take`, `index_state`, `worktree_tree`, `commit_trees`, `stage_entries` |
| `snapshot/stage_tree.rs` | 210 | the unmerged index as one tree, and back |
| `snapshot/chain.rs` | 163 | `chain_backup`, `backup_chain`, `recorded_index_state` |
| `snapshot/index_paths.rs` | 93 | `index_paths_changed` and friends |
| `snapshot/plumbing.rs` | 297 | temporary index, `git` helpers |
| `apply.rs` | 373 | `Context`, `apply`, `Applied`, `Refresh` |
| `apply/error.rs` | 232 | `ApplyError`, `AfterState`, `reason_for` |
| `apply/record.rs` | 339 | `begin`, `finish`, after-state, `Pending` |
| `apply/run.rs` | 127 | running a plan |
| `apply/restart.rs` | 247 | `stage_source`, `prepare_restart` |
| `apply/fold.rs` | 183 | `folded_step`, `inside_live_operation` |
| `apply/resume.rs` | 314 | `resume`, `live_entry` |
| `apply/recover.rs` | 142 | `recover_interrupted` |

`apply/resume.rs` was split too although the review did not name it: 573
lines at `d74a016`, 614 after M1. `apply/remote.rs` (281) untouched.
Suite count identical: **2676** before the split and after, same single seam
failure. `crates/graft-ops/MAP.md` §snapshot and §apply gained one layout line
each; no public item changed.

### Gates (workspace-wide)

`cargo fmt --check` ok · `cargo clippy --all-targets -- -D warnings` ok ·
`cargo test --doc` ok · `cargo nextest run` **2675/2676** — the one failure is
the seam above. Not run: `--release`, Linux, any git but 2.54.0.

## Review fixes, part 2 (m6-04-08 M1) — builder, 2026-09-23, branch `m6-08-fix2`

Branch from `67c2dbb`, `git merge main` (g04 race fix): **clean, no
conflict** — the two touched disjoint files. No scout: the seam and the
walk were named in the dispatch; the refusals were found by running a
throwaway probe test (deleted before commit), not by reading source.

### The seam test (observed)

`crates/graft/tests/it/resolver/restart.rs`
`no_entry_added_in_a_shell_begun_merge` renamed
**`each_working_tree_step_is_its_own_entry_in_a_shell_begun_merge`**; doc
rewritten (each step is its own entry with its own backup;
`apply::fold::folded_step`). Both counts **observed** passing: whole-file
`o` → `before + 1`, the `d` restart → `before + 2`. Added: every new entry
has a `backup_ref`.

### M1 through the app — what the user must do to recover

`crates/graft/tests/it/resolver/shell_begun.rs::a_hand_edit_taken_by_t_comes_back_after_the_merge_ends`
(`Shape::StoppedMerge`, a merge built with raw git). Observed, in order:

1. Hand edit `theirs-deleted.txt`, `⏎` its row, `t`: file gone; one new
   `Committed` entry with a backup ref.
2. `Esc`, `u` → refused, `finish or abort the merge first · ⇥ banner`.
3. `⇥` to the banner, `a`, `→`, `⏎` → refused, `nothing Graft started is in
   progress; an operation begun in the shell is finished there`. MERGE_HEAD
   still there.
4. **In the shell:** `git merge --abort`.
5. Back in the app: `Esc` (the banner is modal and eats `Ctrl+R`), `Ctrl+R`
   → the banner goes.
6. `u` → drift box `Undo “resolved theirs-deleted.txt (theirs)”?`, `→`, `⏎`
   → `undo: resolved …`. **The hand edit's bytes are back on disk.**

**So: there is no in-app path.** The edit is safe (backed up, journaled)
but reachable only after the user ends the merge **outside the app**.
Nothing invented here. **Spec question for the conductor/user:** should
`u` (or the banner) reach a whole-file step's entry while a shell-begun
merge is still open, or should the banner's `a` abort a shell-begun merge?
Both are spec changes (keymap §2.6a; the abort refusal).

Not tested: ending the merge with `git commit` instead of `--abort` (the
file would have to be resolved first, so the drift box differs).

**Observed side effect (pinned in the test):** the undo restores the index
the take found — `theirs-deleted.txt` is back as `UD` in `ls-files -u` with
**no MERGE_HEAD**: conflict stages with no operation around them. The user
has their bytes but a file git calls unmerged; `git add` clears it. Whether
the app draws that state sensibly was **not checked**. Question, not defect.

### `d` after `t` in a shell-begun merge (follow-up, refusal not loss)

Still open, **read, not observed**: `restart::stage_source` reads the
**chain** while an operation is live and ignores the take's own entry, which
now holds the stages, so `d` after a `t`/`⏎` refuses `NoStagesKept`. No
bytes lost. Not fixed here.

### Gates (workspace-wide, own `CARGO_TARGET_DIR`)

`cargo fmt --check` ok · `cargo clippy --all-targets -- -D warnings` ok ·
`cargo nextest run` **2679/2679** (1 skipped) · `cargo test --doc` ok.
Not run: `--release`, Linux, any git but 2.54.0.
