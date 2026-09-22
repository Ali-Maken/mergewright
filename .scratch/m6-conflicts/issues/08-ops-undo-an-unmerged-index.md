# ops-undo-an-unmerged-index: the journal records index stages 1/2/3 as a tree, so `u` inverts a resolution made with no operation live, `U` puts a conflict back, and `d` starts a file over after it was already staged — without re-running anything

Status: ready
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

- [ ] A JSONL line written by M5 with a bare-oid `index_before` parses to `Merged`; `"index_before": null` parses to `None`; an `Unmerged` entry round-trips; `render_pending` of a `Merged` entry is byte-identical to the pre-change renderer's output for the same entry (a stored golden line) — `journal::tests::index_state_serde_is_backward_compatible` (graft-core), `journal::tests::merged_lines_render_unchanged` (graft-ops).
- [ ] On `Shape::StoppedMerge` (11 unmerged files) `take` returns `Unmerged(t)`; `ls-tree -r t` holds `1/`, `2/`, `3/` entries for exactly the conflicted paths (the `UnmergedHow` table decides which stages each has) and `0/` for every other path; `RestoreIndex { Unmerged(t), Everything }` on a scratch index reproduces `git ls-files --stage` **byte-for-byte** — `snapshot::tests::unmerged_index_round_trips`.
- [ ] A clean index still gives `Merged(write-tree)` and a backup commit whose tree has `index/` and `worktree/` and no `unmerged/`; the `index/` tree id equals the pre-change layout's — `snapshot::tests::merged_layout_unchanged`.
- [ ] 03's `whole_file_is_its_own_entry_when_nothing_is_live` passes un-ignored: after `u`, `ls-files -u` lists the path with the **same stage oids and modes** as before the resolution and the file's bytes equal the pre-resolution bytes — that test, plus `apply::tests::undo_of_a_resolution_restores_the_stages`.
- [ ] Stash-pop conflict (m5's shape, no operation live): `Stage p` → `u` (stages back, bytes back) → `U` (stage 0 again, `ls-files -u` empty) → `u`, `u` (the pop undone: tree clean, `stash list` still has it) → `U` (markers back **and** `ls-files -u` lists `p`; `git status` reads unmerged, not modified — row 229's redo half) — `apply::tests::pop_conflict_undo_redo_walks_both_ways`.
- [ ] Undoing a conflicted pop with nothing touched since shows **no** index drift (`Drift::is_clean`), no `force` needed — `undo::tests::conflicted_pop_undo_is_clean_without_force` (replaces the "forcing past it still works" behaviour; the old test is updated, not deleted).
- [ ] `Shape::StoppedRebase`: a folded `Stage a.txt` moves the entry's backup ref to a commit whose first parent is the previous ref target, whose `unmerged/` has `1/a.txt`…`3/a.txt`; the journal file's line count is unchanged; `Applied.step_backup == Some(that commit)` — `apply::tests::folded_step_chains_a_backup`.
- [ ] `StoppedMerge`: `Stage both.txt` (folded) then `RestartConflict both.txt` → `ls-files -u` lists it with the **original** stage oids and the file holds markers; the same with a **fresh `Context`** between the two (the chain lives in the ref, not in memory) — `apply::tests::restart_after_stage_brings_the_conflict_back`, `apply::tests::restart_survives_a_relaunch`. NOTES records what bare `git checkout --merge -- both.txt` did on the stage-0 path, with the git version (Scope 5's question).
- [ ] `StoppedMerge`, no chain commit holding the path (a scratch repo whose backup ref was reset to the pre-op commit) → `RestartConflict` refuses `NoStagesKept { path }`, nothing written — `apply::tests::restart_refuses_without_stages`.
- [ ] `resume(Continue)` chains a backup first; the closing after-commit's first parent is the chain tip; `u` after the rebase finishes restores the pre-rebase tip, index and tree exactly as before this group (m5's `history_ops` tests unchanged and green) — `apply::tests::resume_chains_then_after_snapshot_parents_the_chain`.
- [ ] `next_undo` with a live entry and three chained commits is the same entry as with none (the chain adds no entries; the app's refusal is untouched) — `journal::tests::chain_adds_no_entries`.
- [ ] `RestoreIndex(Unmerged)` stdin bytes: for each path the mode-0 line precedes its stage lines; one process per step — `restore::tests::unmerged_index_info_lines` (a unit test on the generated bytes).
- [ ] 300 conflicted paths (generated in-test with plain `git`, not a fixture): `take` → restore round trip byte-for-byte; the test prints `stage tree: <n> ms, restore: <n> ms` and the builder's report quotes them; the whole test under 5 s on a laptop, fast tier — `snapshot::tests::three_hundred_conflicted_paths`.
- [ ] Stop raised before the chained snapshot → `Stopped`, the backup ref not moved, the step not run — `apply::tests::stop_before_chain_writes_nothing`.
- [ ] `tests/it/seam_rule.rs` passes unchanged (no new writer exposed); `cargo test --doc` covers the `IndexState` doc example.
- [ ] MAP.md entries per Scope 7; NOTES lists the four scout answers (Size) and what was **not** observed (rule 1).

## Gates

fmt / clippy / nextest / doc, workspace-wide.

## Size

One context; ~600 lines of code across `graft-core::journal`, `graft-ops::{snapshot, facts, undo, restore, apply}` plus ~450 of tests. **If the context compacts, split at the seam**: piece A = Scope 1–4 + 6 (the schema and the round trip: fixes the reported defect and row 229), piece B = Scope 5 (the chain and `stage_source`: `d` after `⏎`); A stands alone.

Scout questions (brownfield; answer in NOTES before writing, from MAP first, source only where MAP is silent): (1) who reads the backup ref's *target commit* as the before-state rather than the entry's `index_before`/`worktree_before` oids — anything that does must be pointed at the chain root or the fields (`AfterState::BackupGone`, `Unsealed`, `rollback::begin`); (2) which parents the closing after-snapshot passes today, so (b) changes one argument; (3) every match on `Snapshot.index_tree`/`RepoStateNow.index_tree`/`index_before` as `Option<Oid>` — the compiler lists them, the scout says which are logic (drift, `state_at`, rollback) versus plumbing; (4) the m5 test corpus of journal lines, so the golden for `merged_lines_render_unchanged` is real.

## For group 07 (the conductor adds this; not an edit to 07 from here)

- New scenario `undo-a-resolution-in-a-stash-pop-conflict`: m5's stash-pop shape; `⏎` from the pane; `o`, `⏎` → `all conflicts resolved`; `u` → the pane's `CONFLICTS` row is back, `ls-files -u` lists the path, the file bytes equal the bytes before `⏎`; `u` → the pop is undone (clean tree, stash still listed); `U` → the conflict is back, markers on disk and `ls-files -u` non-empty. Truth from git, never from `AppState`. What right looks like: `git ls-files -u` prints three lines for `p` after the first `u`, none after the second, three again after `U`.
- `start-over-brings-the-markers-back` (already in 07) depends on this group for its "after `⏎`" step; its `what right looks like` line gains `git ls-files -u` lists the path with the same stage oids as before the take.
