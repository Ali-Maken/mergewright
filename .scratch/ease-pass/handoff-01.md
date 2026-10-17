# ease-pass 01 roles-and-route: builder ticks and handoff

Status: **partial**. Blocked: needs one graft-git read. The conductor folds this into `issues/01-roles-and-route.md`. It is a separate file because the ticket lives on main.

## Acceptance
Tests: `crates/graft-graph/src/role/tests.rs` (R:), `crates/graft-ops/src/branch_status/tests.rs` + `tests/status.rs` (B:). The module is split for the 400-line rule: `branch_status.rs` (base, route) + private `branch_status/status.rs` (the four answers, re-exported).
- [x] `Route::role` table test. R: `role_table`, `focus_on_the_trunk_is_this`, `no_focus`, `unread_has_no_chain`, `trunk_in_built_on_is_dropped`.
- [x] `made_on` test. R: `made_on_reads_the_rows_own_lane`, `made_on_an_unnamed_lane_is_none`, `made_on_out_of_range_is_none`.
- [x] `short_name` table test. B: `short_name_table`.
- [x] `choose_base` property test. B: `choose_base_picks_the_nearest` (proptest), plus worked examples `choose_base_worked_example`, `choose_base_ties_go_trunk_then_integration_then_name`, `choose_base_none_when_every_candidate_holds_the_branch`.
- [x] Fixture `main ← wallet-release ← sara/fix-toc` + `ops/ci`. B: `route_of_a_stacked_branch_ends_at_the_trunk`, `route_of_the_trunk_is_empty`, `route_skips_a_candidate_at_the_same_tip`.
- [x] Cycle and depth cap. B: `route_does_not_cycle` (two branches at one tip + a diverged third, no trunk), `route_stops_at_depth_eight`; also `route_does_not_walk_back_through_the_branch`.
- [ ] `branch_status` fixtures. **Not ticked: the trunk half is blocked.** Done and passing: each `Pushed` (`pushed_each_way`), `Merged` Into/NotYet (`merged_into_its_base_after_a_merge_commit`), a branch stacked on top is not a merge (`a_branch_stacked_on_top_is_not_a_merge`), `Behind` 3 (`behind_counts_the_bases_new_commits`), `Who` 1/2/4 (`who_one_author`, `who_two_authors`, `who_four_authors`). **Blocked:** trunk → `everyone`.
- [x] `stop` honoured. B: `route_and_status_honour_stop` (both return `Err(RunError::Stopped)`).
- [x] MAP.md entries. `crates/graft-graph/MAP.md` `## role`, `crates/graft-ops/MAP.md` `## branch_status` (checked by reading; no test).

## Handoff (2026-10-17, builder)
**What's blocked.** Two cases need "the newest `cap` commits from a tip, with no lower bound":
- the trunk's `Who.last` (its tip's `author.when`);
- `Who` for a non-trunk branch with an empty route (spec: "the capped range from the branch tip").

No existing graft-git read does this with a stop:
- `commits_between` always takes an `exclude` (`graft-git/src/history_reads.rs:88`).
- `rev_list` is crate-private (`:149`).
- `read_commit` has no stop and fails with `WalkError` (`graft-git/src/history/read.rs:30`).

Today both cases call `range_from_tip` (`graft-ops/src/branch_status/status.rs`, at the bottom). It **returns `Err(RunError::Unparsable(..))`, so these cases are broken, not just unverified.** `branch_status` always fails on the trunk.

**Exact next step.** Add this to graft-git `history_reads.rs` (same shape as `commits_between`):
`pub fn commits_from(repo: &RepoHandle, include: Oid, cap: usize, stop: &dyn Stop) -> Result<Range, RunError> { rev_list(repo, cap, vec![GitArg::Oid(include)], stop) }`
and add its MAP.md line. Then make `range_from_tip` call it, or call it directly at the two call sites and delete `range_from_tip`. Then put these tests back into `branch_status/tests/status.rs`. They are already written and failed only because this read is missing:
- `the_trunk_is_the_main_line`: stack, `main` → `Merged::MainLine`, empty route, `Behind::NoBase`.
- `the_trunk_is_behind_its_upstream`: two commits on `upstream-main`, `upstream(&repo, "main", "upstream-main")` → `Behind::Behind { base: "refs/remotes/origin/main", count: 2 }`, `Pushed::ToPull(2)`.
- `who_on_the_trunk_is_everyone`: `feature_by(&[BEA])`, then `CAL` commits `m2` on `main` → `everyone`, `first` empty, `last.seconds == 1_700_000_000 + 180`.
- Add one more for `Who` on an empty-route branch (for example a branch fully merged into `main`).

Alternative if graft-git stays closed: graft-ops already runs its own reads through `GitInvocation` (`objects.rs:11`, `rewrite/progress.rs:266`). A local `rev-list` plus the public `parse_rev_list` would work. The ticket says "no new git read", so that is the conductor's call.

**Decisions not in the spec (say if any is wrong).**
- **The route never measures a candidate at the tip of the branch, or at the tip of a base already on the route.** The spec's cycle guard stops on a repeat, which drops `main` from `sara/fix-toc`'s route once `wallet-release` gets a commit after `sara` branched off it: `sara` becomes `wallet-release`'s nearest candidate, the walk stops there, and `main` is never reached. Test: `route_does_not_walk_back_through_the_branch`.
- **`Merged` when nothing else applies:** no route, no trunk among the candidates, no container → `MainLine` (nothing lies under the branch). In that case the spec's `into` has no name to give.
- **Read caps.** The stacked check reads up to 10 000 commits (`STACK_CAP`). Past that, the holder counts as stacked (the spec's "capped → not a container"), so a branch merged into `main` more than 10 000 commits ago reads `NotYet`. `Who` reads up to 2 000 commits (`WHO_CAP`).
- **Fast-forward merges look like "made on top".** If `wallet-release` fast-forwards to `sara` and then gets more commits, `sara` reads `NotYet`. This follows from the first-parent rule.
- **`who_of` tie order:** count, then newest commit, then email. The spec says count, then newest; the email step only makes the order fixed.

**For B (unverified, not built here).**
- `candidates` must not include the branch's own upstream, or the upstream reads as the base. The `route` doc says so.
- Cost: 2 `rev-list --count` spawns per candidate per route step (at most 8 steps), plus one `rev-list` per container checked. Not measured; B's per-tips cache keeps it off the hot path.

**Name check** (re-grepped at the end): `Who` also exists at `graft-ui/src/blame.rs:34`. `Apart` is also a variant at `graft-ui/src/graph_screen/badges.rs:78` and in `graft::refs::Upstream`. `Merged` is a variant at `graft-core/src/journal.rs:229`. None of these is in the same crate.
