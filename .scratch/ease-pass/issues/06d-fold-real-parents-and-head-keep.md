# fold-real-parents-and-head-keep — a folded merge is still a merge to everything but the drawing; launch lays out once

Status: ready
Blocked by: 06 continuation 1 merged (branch `worktree-agent-acb2cc0f5685a9e78` @ 7a4c11f0)
Build with: Opus (`builder-opus-xhigh`) — a public field change on `Row` with many readers, classified one by one. The shape is decided (06 D1); correctness is checkable by a property test and the m5/m7 walks, so not Fable.
Needs: from 06: `Included`, `advance_folded`, `View::folded`, `with_head_kept`, `history::fold::{lay_out, landing}`, `views/sent.rs` `refresh_sent`. Existing: `Row::assemble` (`graft-graph/src/row.rs:201`), `RepositoryFacts` (HEAD), `RevertParams { mainline, parents, .. }` (graft-ops).

## Read first
- `.scratch/ease-pass/issues/06-graph-fold-and-lanes.md`: NOTES (continuation 1) "Spec question" and "Second finding"; **Decisions for continuation 2, D1 and D2**.
- `crates/graft-graph/MAP.md`: `row`, `raster` (`Ring`), `ancestry`, `fold`, `layout` (invariants), `inclusion`. `crates/graft/MAP.md`: `history`, `history::state`, `update::tree_answers`, `update::keys`, the repository facts section (`RepositoryFacts`, `Head`). `crates/graft-ops/MAP.md`: `RevertParams`, `PickParams`.
- Brownfield: re-check the readers' line refs with grep before editing (the planner read no source).

## Scope
In: graft-graph `row` and every drawing reader of a row's parents; graft `update::*` semantic readers; `history::state` / `update::tree_answers` / `keys::view_change` (HEAD keep); graft's repository facts if they lack HEAD's commit id (graft-git `RepoHandle::head` if needed, in this crate set).
1. **D1, step 1 (mechanical).** Add `Row::drawn_parents`. `Row::assemble` fills it with what it puts in `commit.parents` today, and puts the source commit's parents in `commit.parents`. Move every current reader of a row's `commit.parents` to `drawn_parents`. **Every existing test and capture passes unchanged, with no `MWR_UPDATE_SHOTS`.** That proves the drawing did not move.
2. **D1, step 2 (semantic).** Move each of these to `commit.parents` / `is_merge()`:
   - `update/selection.rs:208`;
   - `update/context_menu.rs:59`;
   - `update/preview/take.rs:131`;
   - `view/entries.rs:340`;
   - `update/detail/control.rs:131` (stash rows keep today's output);
   - whatever builds `RevertParams` / `PickParams` from a row.
   Then grep `parents` in `crates/graft/src/**` and `crates/graft-graph/src/**`. NOTES lists every hit as **drawing** or **git truth**.
3. **D2.** The walk starts with HEAD's commit in `keep`, from the opening's facts. `with_head_kept` returns `None` when it is unchanged. Delete any HEAD write that this makes redundant.
4. **Untested paths** from continuation 1: a stale sent window is re-sent (`views/sent.rs`); a promoted anchor that the fold hides lands on its `behind` merge (`fold::landing`); the HEAD keep (D2).

Out: the integration list in 06's Handoff 2.3 and 2.4 (06e); captures (06f); the look of a folded merge's node (07).

## Signatures
graft-graph `row`:
```rust
pub struct Row {
    /// The commit as git has it: `commit.parents` is every real parent under every inclusion.
    /// Anything that reasons about the commit, or builds a git write, reads this.
    pub commit: Commit,
    /// The parents this view draws an edge to (trimmed by fold, `1`, hidden stashes). Drawing only.
    pub drawn_parents: Vec<Oid>, // element type = `Commit::parents`'s
    // lane, incoming, outgoing, lanes, marker: unchanged
}
impl Row {
    /// Two or more real parents, stash entries excluded (the same test `Ring::of` uses for `Stash`).
    pub fn is_merge(&self) -> bool;
}
```
graft `history::state` / `keys::view_change`:
```rust
impl View { pub fn folded(head: Option<Oid>) -> View; } // None: unborn, or HEAD unreadable (say which in the status, never silently)
pub(crate) fn with_head_kept(view: &View, head: Option<Oid>) -> Option<View>; // None when keep already holds `head`
```

## Touches
Every `Row` reader in graft-graph (raster, ring, ancestry, fold, made_on, invariants, chunk fingerprints) and graft (selection, menus, preview, detail, radar, entries); graft-ops params built from rows; stash rows; first-parent `1` (its latent bug is fixed by the same change, so test it); `Window` size: `Row` grows by one `Vec` per resident row (NOTES gives the bytes, and the AppState `large_stack_arrays` cliff in 06 NOTES); the history phase on launch; `m7_walk::pull`. The builder's report needs a **Touches:** line with how each was checked.

## Acceptance
- [ ] graft-graph property: under every inclusion `viewed()` generates, every row's `commit.parents` == the source commit's parents, and `drawn_parents` == today's trimmed set. Name the test.
- [ ] Step 1 left the drawing unmoved: the workspace run after step 1 alone shows no capture or raster test change. Say the run and its count.
- [ ] A folded merge, and a merge in `1`, is a merge to: `selected_merge`, the Fix menu, take refusal, radar markers and detail parents. One app test each, named. A stash entry is not a merge.
- [ ] `m5_walk::every_scenario::revert_merge_uses_mainline` and `m5_walk::every_scenario::fix_*` pass; `m7_walk::picker::take_refused_on_a_merge` passes.
- [ ] `refs_arriving_with_head_unchanged_keeps_the_view`: no `RowRequest`, the phase stays. `m7_walk::pull::pull_ff_over_edits` passes. `switching_to_a_merged_branch_shows_its_tip` (HEAD onto a folded tip → the tip is shown).
- [ ] Tests for the re-send of a stale window and for the promotion landing, named.
- [ ] Re-run Handoff 2's 47 failing tests. NOTES lists which still fail, as input to 06e.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~55k. Step 1 is wide but mechanical. If the reader count makes it larger, stop after step 2 and report: D2 is independent.
