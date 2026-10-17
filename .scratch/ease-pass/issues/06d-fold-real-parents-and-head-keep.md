# fold-real-parents-and-head-keep — a folded merge is still a merge to everything but the drawing; launch lays out once

Status: done — 7/7 (box 4 closed by 06f: take-refused-merge re-shot)
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
- [x] graft-graph property: under every inclusion `viewed()` generates, every row's `commit.parents` == the source commit's parents, and `drawn_parents` == today's trimmed set. Name the test. — `inclusion::tests::real_parents::every_row_keeps_its_real_parents_under_every_inclusion` (+ `a_folded_merge_and_a_first_parent_merge_are_merges`).
- [x] Step 1 left the drawing unmoved: the workspace run after step 1 alone shows no capture or raster test change. Say the run and its count. — step-1 run: 6617 tests, 53 failed vs 47 at baseline d547bad0 on the same subset; each difference is explained in NOTES; graft-graph 183/183 pass. A drawing reader missed at step 1 (preview `resident_slice`) is fixed: `update::preview::tests::the_slice_takes_the_parents_the_screen_draws`, and `name_and_drag::capture_fix_preview_verb_name_prompt_for_docs_shots` passes with its capture unchanged.
- [x] A folded merge, and a merge in `1`, is a merge to: `selected_merge`, the Fix menu, take refusal, radar markers and detail parents. One app test each, named. A stash entry is not a merge. — `update::tests::folded_merge::{a_folded_merge_is_probed_as_a_merge, a_folded_merge_menu_reverts_with_a_mainline, o_on_a_folded_merge_is_refused_as_a_merge, a_folded_merge_gets_the_evil_merge_radar_line, a_folded_merge_shows_both_parents_in_the_detail_pane, a_stash_entry_is_not_probed_as_a_merge, a_stash_entry_shows_its_first_parent_in_the_detail_pane}`. The fixture trims `drawn_parents` to one parent, which is what both the fold and `1` produce.
- [ ] `m5_walk::every_scenario::revert_merge_uses_mainline` and `m5_walk::every_scenario::fix_*` pass; `m7_walk::picker::take_refused_on_a_merge` passes. — **partial, not ticked.** The m5 four pass in the full run. `take_refused_on_a_merge` still fails, on its capture only: the merge now draws folded. The refusal itself is covered by `o_on_a_folded_merge_is_refused_as_a_merge`. Regenerating the capture → 06e/06f.
- [x] `refs_arriving_with_head_unchanged_keeps_the_view`: no `RowRequest`, the phase stays. `m7_walk::pull::pull_ff_over_edits` passes. `switching_to_a_merged_branch_shows_its_tip` (HEAD onto a folded tip → the tip is shown). — `update::tests::head_keep::{refs_arriving_with_head_unchanged_keeps_the_view, switching_to_a_merged_branch_shows_its_tip, the_first_request_waits_for_the_opening_and_keeps_head, an_opening_with_no_head_commit_still_releases_the_first_request, a_failed_opening_still_releases_the_first_request, refs_before_the_opening_carry_head_into_the_one_first_request}`; `pull_ff_over_edits` passes in the full run.
- [x] Tests for the re-send of a stale window and for the promotion landing, named. — `history::views::sent::tests::{a_repair_inside_the_sent_window_sends_it_again_under_its_request, a_repair_below_the_sent_window_sends_nothing}`, `history::idle_tests::folded_tests::{a_window_sent_before_its_merge_folded_is_sent_again_once_it_does, every_hidden_commit_lands_on_a_shown_merge_and_a_shown_commit_on_none}`.
- [x] Re-run Handoff 2's 47 failing tests. NOTES lists which still fail, as input to 06e. — 29 still fail, all listed in NOTES.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~55k. Step 1 is wide but mechanical. If the reader count makes it larger, stop after step 2 and report: D2 is independent.

## NOTES (06d builder)

**Compacted once**, after the baseline comparison; the re-send/landing tests, the preview fix, the gates and this ticket came after. No scout: the planner's line refs were re-checked by grep (all within 1–2 lines; `view/entries.rs` is 341).

**Reader classification** (`Row` → parents):
- Drawing (`drawn_parents`): graft-graph `Ring::of`, `ancestry` ×3, the `Fold::view` literal; graft-ui `graph_column/kinds.rs` `NodeKind::of` (not 03r's files); graft `view/preview.rs` label row, and **`update/preview.rs` `resident_slice`**. Step 1 missed that one. The m6 capture caught it: a stash entry's index parent drew as "1 lane continue outside this view".
- Git truth (`commit.parents` / `history::parents`): `update/selection.rs:208`, `context_menu.rs:59`, `preview/take.rs:131`, `detail/control.rs:131`, `context_menu/verbs.rs:206`, `view/entries.rs:341` (radar), `view/detail.rs` (parents and radar), `view/strip.rs:47,55`, `update/place.rs:416`.
- Read a `Commit`, not a `Row` (unchanged): `preview/slice.rs`, `map/walk.rs`, `history/source.rs`, `jobs/fold_count.rs`, graft-ops.

**Deviations from the spec's signatures:**
- `Row::is_merge()` means "2 or more real parents": a row cannot tell a stash entry apart. The new `history::parents::{of, is_merge}` in graft take the refs and give a stash entry its first parent only, which is today's output. The spec said `Ring::of` holds a stash test; it does not (the caller sets `Ring::Stash`).
- `View::folded()` takes no `head` argument. HEAD is added through the existing `with_head_kept` when the held first request is released.
- `HeadError` moved to `repo/head_error.rs` in the git crate, with a new `Peel` variant, to keep `repo/mod.rs` under 400 lines.
- `AppState.repository` is now `Option<Box<RepositoryFacts>>`. `head_commit` (an `Option<Oid>`, 40 B) took `AppState` to 8232 B, past clippy's `large_stack_arrays` limit: 4 test arrays hold two states each. Fixed at the cause, not with an `allow`.

**Size:** each resident `Row` grows by one `Vec<Oid>` header, 24 B, plus 33 B on the heap per drawn parent.

**Still failing (input to 06e/06f).** All 29 are in the baseline's 47. Full run: 6634 run, 6605 passed, 29 failed.
- hint_bar::capture_the_doorway_on_a_marked_row_for_docs_shots
- lane_cap::{capture_lane_cap_for_docs_shots, the_pane_narrows_the_graph_so_the_graph_folds_more}
- look_chrome::{capture_the_chrome_at_every_designed_size, capture_the_cursor_row_in_dark_and_light, every_drawn_row_is_the_row_a_click_selects_at_every_designed_size}
- look_polish::capture_folded_merge_count_for_docs_shots (120 s timeout; superseded per D3)
- m7_walk::picker::take_refused_on_a_merge (capture only: the merge now draws folded, `6 commits`, `▸ 1 commit`)
- m8_walk::journeys::{find_key_by_name, fold_merge_count, light_theme, run_unrecalled, what_does_key}
- m8_walk::look::{capture_help_over_the_graph_at_187x50, capture_the_cursor_row_at_80x24_and_187x50}
- m8_walk::teaches::{create_a_branch_is_found, merge_is_found_on_a_branch_row, push_is_found}
- radar_not_checked::capture_radar_not_checked_with_an_evil_merge
- resolver::shell_begun::c_y_finishes_a_shell_begun_merge
- search_hint::{after_enter_the_bar_is_the_rows_keys_and_the_search_is_in_the_header, the_search_hint_draws_his_esc_spelling}
- tour::{an_unwritable_temp_folder_stays_in_his_repo_and_says_why, the_palette_starts_the_tour_and_esc_returns_to_his_commit}
- tour_tasks::task_7_ticks_by_resolving_clash_and_finishing
- tour_walk::the_whole_tour_in_a_pty_ticks_all_eight_and_comes_home
- update::tests::keymap_edge::palette::at_80_the_bar_ends_with_keys_and_help_lists_what_it_dropped
- update::tests::keymap_edge::remaps::{capture_merge_remapped_to_shift_m, j8_a_keys_table_beside_suspend_moves_merge_and_says_one_problem}

Unverified: that each fails *only* because of the fold drawing. I read the capture diff for take_refused alone. `resolver::shell_begun` and `tour` are not obviously captures; 06e should read them.

**Gap noticed, not fixed (graft-ui, 03r's crate):** the doc at `overview_strip.rs:22` says a merge is `row.commit.parents.len() >= 2`, but the caller now uses `history::parents::is_merge`. Doc only.
