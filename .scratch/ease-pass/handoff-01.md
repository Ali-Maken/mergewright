# ease-pass 01 roles-and-route: builder ticks and notes

Status: **done**. The conductor folds this into `issues/01-roles-and-route.md`. It is a separate file because the ticket lives on main.

## Acceptance (9/9)
Tests: `crates/graft-graph/src/role/tests.rs` (R:), `crates/graft-ops/src/branch_status/tests.rs` + `tests/status.rs` (B:), `crates/graft-git/tests/it/commits_from.rs` (G:). The ops module is split for the 400-line rule: `branch_status.rs` (base, route) + private `branch_status/status.rs` (the four answers, re-exported).
- [x] `Route::role` table test. R: `role_table`, `focus_on_the_trunk_is_this`, `no_focus`, `unread_has_no_chain`, `trunk_in_built_on_is_dropped`.
- [x] `made_on` test. R: `made_on_reads_the_rows_own_lane`, `made_on_an_unnamed_lane_is_none`, `made_on_out_of_range_is_none`.
- [x] `short_name` table test. B: `short_name_table`.
- [x] `choose_base` property test. B: `choose_base_picks_the_nearest` (proptest), plus worked examples `choose_base_worked_example`, `choose_base_ties_go_trunk_then_integration_then_name`, `choose_base_none_when_every_candidate_holds_the_branch`.
- [x] Fixture `main ← wallet-release ← sara/fix-toc` + `ops/ci`. B: `route_of_a_stacked_branch_ends_at_the_trunk`, `route_of_the_trunk_is_empty`, `route_skips_a_candidate_at_the_same_tip`.
- [x] Cycle and depth cap. B: `route_does_not_cycle` (two branches at one tip + a diverged third, no trunk), `route_stops_at_depth_eight`.
  - **Deviation:** the route skips candidates at a tip it has already walked (the branch's or a base's) instead of stopping on a repeat. Otherwise `main` drops off `sara/fix-toc`'s route once `wallet-release` gets a commit after `sara` branched off it. Test: B `route_does_not_walk_back_through_the_branch`.
- [x] `branch_status` fixtures. B:
  - each `Pushed`: `pushed_each_way`;
  - `Merged` Into/NotYet: `merged_into_its_base_after_a_merge_commit`;
  - a branch stacked on top is not a merge: `a_branch_stacked_on_top_is_not_a_merge`;
  - `Behind` 3: `behind_counts_the_bases_new_commits`;
  - `Who` 1/2/4: `who_one_author`, `who_two_authors`, `who_four_authors`;
  - trunk → `everyone`: `who_on_the_trunk_is_everyone`;
  - also `the_trunk_is_the_main_line`, `the_trunk_is_behind_its_upstream`, `who_of_a_branch_with_no_base`.
- [x] `stop` honoured. B: `route_and_status_honour_stop` (both return `Err(RunError::Stopped)`); G: `commits_from_honours_stop`.
- [x] MAP.md entries. `crates/graft-graph/MAP.md` `## role`, `crates/graft-ops/MAP.md` `## branch_status`, `crates/graft-git/MAP.md` `commits_from` (checked by reading; no test).

## The graft-git read (approved by the conductor after the first partial)
`graft_git::history_reads::commits_from(repo, include, cap, stop) -> Result<Range, RunError>`: `rev-list <include>` with no lower bound, capped the same way as `commits_between`. It lives in `history_reads/compare.rs`, so `history_reads.rs` stays under 400 lines. Test: G `commits_from_reaches_the_root_and_caps_to_the_commits_nearest_the_tip`. It is used for the trunk's `Who.last` and for `Who` of a branch with no base.

## Decisions not in the spec (say if any is wrong)
- **`Merged` when nothing else applies:** no route, no trunk among the candidates, no container → `MainLine` (nothing lies under the branch). In that case the spec's `into` has no name to give.
- **Read caps.** The stacked check reads up to 10 000 commits (`STACK_CAP`). Past that, the holder counts as stacked (the spec's "capped → not a container"), so a branch merged into `main` more than 10 000 commits ago reads `NotYet`. `Who` reads up to 2 000 commits (`WHO_CAP`).
- **Fast-forward merges look like "made on top".** If `wallet-release` fast-forwards to `sara` and then gets more commits, `sara` reads `NotYet`. This follows from the first-parent rule.
- **`who_of` tie order:** count, then newest commit, then email. The spec says count, then newest; the email step only makes the order fixed.
- **A branch with no base reads its whole history from the tip, capped.** For a branch `main` has merged, that includes commits by people who never worked on it. This is what the spec says; whether the right pane shows it is B's call.

## For B (unverified, not built here)
- `candidates` must not include the branch's own upstream, or the upstream reads as the base. The `route` doc says so.
- Cost: 2 `rev-list --count` spawns per candidate per route step (at most 8 steps), plus one `rev-list` per container checked. Not measured; B's per-tips cache keeps it off the hot path.

**Name check** (re-grepped): `Who` also exists at `graft-ui/src/blame.rs:34`. `Apart` is also a variant at `graft-ui/src/graph_screen/badges.rs:78` and in `graft::refs::Upstream`. `Merged` is a variant at `graft-core/src/journal.rs:229`. None of these is in the same crate.
