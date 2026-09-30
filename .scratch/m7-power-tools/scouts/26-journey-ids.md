# Scout 26: M7 journey ids → walks

Counting mechanisms (all read the fence via `m7_walk::noted_keys`, `m7_walk.rs:27`, which calls `m5_walk::parse_counts`):
- **A** `noted_keys(id)` direct assert (`m7_walk.rs:76`, `picker.rs`, `pr_contents.rs:105`)
- **B** `rebase_line::within(journey, &route)` `rebase_line.rs:238` (also used by rewrite_editor/rewrite_stops via `use super::rebase_line::within`)
- **C** `amend_into::within` `amend_into.rs:189`
- **D** `blame::Route::within_count` `blame.rs:266` (`self.id`; used by history.rs, journal.rs)

Landings listed only where seen in the grep; "not read" = not confirmed.

| id | test (nextest name) | file:line | ≤ noted_keys? | landing asserted |
|---|---|---|---|---|
| squash-last-three | m7_walk::rewrite_editor::squash_last_three | rewrite_editor.rs:203 (within :218) | yes, B | feature-a message/tree (:225,:233) |
| swap-adjacent | m7_walk::rewrite_editor::swap_adjacent | :253 (:270) | yes, B | not read |
| fix-into-older | m7_walk::amend_into::fix_into_older | amend_into.rs:219 (:230) | yes, C | replayed subject (:235) |
| absorb-staged | m7_walk::amend_into::absorb_staged | :249 (:274) | yes, C | feature-a tokens.rs content (:278) |
| amend-with-dependents | m7_walk::amend_into::amend_with_dependents | :298 (:313) | yes, C | feature-a content (:315) |
| rebase-line-onto-main | m7_walk::rebase_line::r_y_moves_the_line | rebase_line.rs:250 (:272) | yes, B | not read (`lands_as_drawn` :195 exists) |
| squash-merged-bottom | m7_walk::rebase_line::r_y_drops_the_squash_merged_branch | :283 (:298) | yes, B | feature-a == main (:300) |
| keep-a-suggested-drop | m7_walk::rebase_line::keep_a_suggested_drop | :448 (:475) | yes, B | feature-b^ == feature-a (:493) |
| rewrite-stop-to-first-file | m7_walk::rewrite_stops::rewrite_stop_to_first_file | rewrite_stops.rs:72 (:83) | yes, B | not read |
| edit-stop-lands | m7_walk::rewrite_stops::edit_stop_lands | :100 (:104) | yes, B | followers b,c (:119) |
| edit-stop-amend-continue | m7_walk::rewrite_stops::edit_stop_amend_continue | :302 (:353) | yes, B | amended in place (`assert_amended_in_place` :278) |
| abort-from-a-chain-stop | m7_walk::rewrite_stops::abort_from_a_chain_stop | :374 (:410) | yes, B | refs unchanged, HEAD on feature-a (:412–419) |
| restart-after-done | m7_walk::restart_after_done | m7_walk.rs:53 (:76) | yes, A | not read |
| switch-named-branch | m7_walk::picker::switch_named_branch | picker.rs:244 (:266) | yes, A | not read |
| switch-back | m7_walk::picker::switch_back | :304 (:323) | yes, A | not read |
| switch-with-edits | m7_walk::picker::switch_with_edits | :346 (:371) | yes, A | not read |
| cherry-pick-in-view | m7_walk::picker::cherry_pick_in_view | :492 (:535) | yes, A | new commit (`y_takes_it_and_lands_on_the_new_commit` :556) |
| cherry-pick-other-branch | m7_walk::picker::cherry_pick_other_branch | :575 (:605) | yes, A | not read |
| pr-contents | m7_walk::pr_contents::pr_contents | pr_contents.rs:100 (:105) | yes, A | not read |
| **pull-current-branch** | **MISSING as fence walk.** Nearest: m7_walk::pull::pull_ff_over_edits | pull.rs:33 | **no**: local `const NOTED_KEYS: usize = 1` (pull.rs:26, "03 Decided"), never calls noted_keys; id string appears nowhere in tests | cursor on new HEAD, `pulled origin/main ✓ · your edits are back` |
| blame-line-to-commit | m7_walk::blame::blame_line_to_commit | blame.rs:284 (:297) | yes, D | not read |
| commit-back-to-blame | m7_walk::blame::commit_back_to_blame (+ `_with_a_base_pinned` :320, same id) | :313 via back_to_blame :324 (:347) | yes, D | not read |
| blame-who-at-line | m7_walk::blame::blame_who_at_line | :377 (:386) | yes, D | not read |
| blame-line-before | m7_walk::blame::blame_line_before | :411 (:422) | yes, D | not read |
| function-history | m7_walk::history::function_history | history.rs:53 (:65) | yes, D | not read |
| file-history | m7_walk::history::file_history | :93 (:101) | yes, D | not read |
| lost-to-shell-reset | m7_walk::journal::the_three_lost_work_journeys (+ `_git_clock_pinned` :223) | journal.rs:200 → route :268 (:280) | yes, D | `j_closed_and_pane_shown` :161 (not confirmed which) |
| deleted-branch-back | same test | :305 (:331) | yes, D | not read |
| edit-in-undo-backup | same test | :375 (:399) | yes, D | not read |

Also `Route::new("lost-to-shell-reset")` in capture tests journal.rs:450,:492 (captures, not the walk).

Contradiction: the fence (:842) counts `pull-current-branch` = 1, but no test reads it; pull.rs:26 hardcodes 1.

## Nextest
No `[[test]]` in crates/graft/Cargo.toml; `tests/it/main.rs` = one binary named `it` (main.rs:1–6). Test paths omit `it::`:
`cargo nextest run -E 'binary(it) & test(=m7_walk::blame::blame_line_to_commit)'`
(nextest id: `graft::it m7_walk::blame::blame_line_to_commit`). Could not confirm by running (read-only).
