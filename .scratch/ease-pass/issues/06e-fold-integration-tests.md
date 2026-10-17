# fold-integration-tests — the integration tier agrees with fold by default; boxes 5, 7, 8

Status: done — 5/5 (box 5 closed by 06f)
Blocked by: 06d
Build with: Opus (`builder-opus-xhigh`) — several timeouts are undiagnosed, and each needs a cause with a line (rule 7) before any change.
Needs: from 06d: `Row::is_merge`, `Row::drawn_parents`, `View::folded(head)`. From 06: `space` handler, `Window.fold`, `FoldMark`.

## Read first
- `.scratch/ease-pass/issues/06-graph-fold-and-lanes.md`: Handoff 2 items 3, 4, 6; **Decisions for continuation 2, D3**; Acceptance boxes 5, 7, 8.
- 06d's NOTES: which tests still fail after the parents and HEAD fixes.
- `docs/design-notes/ease-pass.md` Journeys (the counts the m8 journeys assert).
- `crates/graft/MAP.md`: `history`, `update::keys`, `view::rows`, `event_loop::*`.

## Scope
In: graft integration tests (`crates/graft/tests/**`) that **write no capture**, plus their fixtures. The fold-related fast-tier budget tests. The tour's repository or steps, if the tour is the cause.
- Every test in Handoff 2.3 that is not a `capture_*` test, and not in `lane_cap`:
  - `m8_walk::journeys::{find_key_by_name, run_unrecalled, what_does_key, light_theme}`;
  - `m8_walk::teaches::*`;
  - `look_chrome::every_drawn_row_is_the_row_a_click_selects_at_every_designed_size` — run it both folded and with one merge opened, since click mapping across hidden rows is the risk;
  - `context_menu::the_menus_band_stops_at_the_detail_pane_instead_of_clearing_it`.
  Fix each by D3's route order. **Journey step counts stay as they are.**
- The undiagnosed tests: `m7_walk::pr_contents::pr_contents`, the `tour*` tests, `preview::the_branch_badge_still_lands_on_the_newest_predicted_row`, `radar_not_checked`. NOTES gives each one's cause, file and line first. Fold hid a row → D3. A ghost row folded, or the tour asking for a hidden row → a product fix, per D3. Anything else → stop and report.
- `m8_walk::journeys::fold_merge_count` rewritten: the tail reads `▸ N commits from …`; `space` → `▾` and the side's rows appear. Its step count comes from the design note's journey.
- **Box 5:** a real fixture repo (`main ← wallet-release ← sara/fix-toc`, PR merges with the branch deleted, one named merge) driven through the worker. First paint shows no merged-branch commits, and its tails read as 06 Scope says.
- **Box 7:** app-side Q10 row shape. After `space`, the opened side's rows sit on a lane that is not the merge's column and that joins the merge row.
- **Box 8:** a fast-tier first-paint test and a held-`j` test over a walked repo whose window carries folding merges (`fold` non-empty). Release numbers go to the milestone review.

Out: every `capture_*` test, `lane_cap.rs`, `look_polish` and re-shooting (06f).

## Signatures
None new. Fixture helpers are the builder's to name; NOTES names them.

## Touches
Shared fixtures (`merge_on_screen`, the m8 fixture repo). An added unmerged branch changes what the capture tests draw, so NOTES lists every capture test using that fixture, for 06f. The tour's repository and text. Journey counts in `TESTING.md` (unchanged). `over_400.txt` if test files grow.

## Acceptance
- [x] Every test above passes, or NOTES names a cause with a line and the conductor's call. Each tick names its test. — pass in the full run: `m8_walk::journeys::{find_key_by_name, run_unrecalled, what_does_key, light_theme, fold_merge_count}`, `m8_walk::teaches::{create_a_branch_is_found, merge_is_found_on_a_branch_row, push_is_found}`, `look_chrome::every_drawn_row_is_the_row_a_click_selects_at_every_designed_size` (folded and with one merge opened), `context_menu::the_menus_band_stops_at_the_detail_pane_instead_of_clearing_it`, `m7_walk::pr_contents::pr_contents`, `tour::{an_unwritable_temp_folder_stays_in_his_repo_and_says_why, the_palette_starts_the_tour_and_esc_returns_to_his_commit}`, `tour_tasks::task_7_ticks_by_resolving_clash_and_finishing`, `preview::the_branch_badge_still_lands_on_the_newest_predicted_row`, `outside_repo::launch_and_open_land_alike`. `tour_walk::the_whole_tour…` and `radar_not_checked::capture_…` now fail on their capture only (06f). Causes in NOTES.
- [x] No m8 journey's asserted step count changed (diff of the assertions, quoted). — `git diff -- crates/graft/tests/it/m8_walk/ | grep -E '^[+-].*(within_the_note|let route)'` gives only the moved pair `let route = ["Space"]; within_the_note("fold-merge-count", &route);` (− and + identical). The other journeys' routes are untouched; only their setup click moved to `topic-4`.
- [x] Box 5 and box 7 in 06 ticked, each naming its test. — `fold_by_default::first_paint_folds_merged_branches_and_the_tails_name_them`, `fold_by_default::space_opens_the_merged_side_on_its_own_lane_joining_the_merge`.
- [x] Box 8 in 06 ticked: the test shows the window really carries fold facts (an assertion on `fold`, so the tick is not vacuous). — `event_loop::tests::settle::fold::a_held_j_over_a_folded_walk_follows_and_counts_only_at_rest` asserts 61 rows, 61 fold facts, 60 `folds == Some(true)` before the hold; first paint: `fold_by_default::first_paint_…` asserts one fold fact per row and both merges `Some(true)`.
- [ ] Workspace nextest: the only failures left are `capture_*` / `lane_cap` / `look_polish`, listed for 06f. — **not ticked:** 17 fail (6644 run, 6627 pass). Ten are `capture_*` / `lane_cap` / `look_polish`; seven others fail **only** in `graft_fixtures::check` (capture.rs:130/147), i.e. on a capture: listed in NOTES.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~60k. The tour diagnosis is the unknown. If it turns into a product change bigger than one step, stop and report.

## NOTES (06e builder, 2026-10-18)

Not compacted. No scout: the failing list and messages came from a full workspace run (run 1 after `git merge` of 06d @ dda0a592 and `main`: 6641 run, 30 failed); each test read directly. Run 2 after the changes: 6644 run, 6627 passed, 17 failed.

**Causes, one per group (rule 7):**
- **Cursor found by `contains('▸')` — knock-on, test helper.** A folded merge's tail draws `▸` (graft-ui `graph_screen/fold_mark.rs:39`), so `tour.rs:103` `cursor_row` found HEAD's merge row (tail) instead of the cursor's: `tour::an_unwritable_…` waited 20 s for `landed + 1`. Same helper in `tour.rs`, `tour_tasks.rs:294/301`, `tour_walk.rs:239`, `outside_repo.rs:89/352/358`: now `starts_with('▸')` (the cursor leads its line). The tour itself was not the cause: every tour task still ticks; no product change.
- **Merged branch rows folded — knock-on, D3 route 2.** `m8_walk::{journeys, teaches}::on_a_branch_row` clicked `[topic-1]`, now folded into the octopus. New helper `m8_walk::octopus_with_an_unmerged_branch` (m8_walk/mod.rs) adds `topic-4`, one commit on `main` dated `CORPUS_NOW − 1h`, corpus refs untouched; the walks click it. Setup only, routes unchanged.
- **`look_chrome::every_drawn_row…` — knock-on.** Its fixed subject list held folded commits (only 4 drawn, needs ≥ 5). Rewritten: every row ending in a short id is clicked and must select that id; swept folded (6 rows) and with `feature/one`'s merge opened by `space` (8 rows). Stronger than before, not weaker.
- **`outside_repo::launch_and_open_land_alike` — knock-on, race.** The fold count lands on its own job, so one side drew `▸ commits from` and the other `▸ 2 commits`; it failed in run 1 only. Added `COUNTED` to its settle lines (outside_repo.rs).
- **`fold_merge_count` — superseded journey, rewritten:** opens folded, no topic rows, tail `▸ 3 commits` (octopus: no ` from …`), `Space` → `▾ 3 commits` + the 3 topic rows, Ctrl+R keeps the count. Route `["Space"]` = 1, as the note.
- `pr_contents`, `preview::the_branch_badge…`, `context_menu::…` already passed after 06d + main; not touched, not diagnosed by me.

**New tests:** `tests/it/fold_by_default.rs` (fixture `wallet_release()`: `main ← wallet-release ← sara/fix-toc`, PR 16021 merged with its branch deleted, `sara/fix-toc` kept and merged by name; built with `graft_fixtures::git::run_at`). `src/event_loop/tests/settle/fold.rs` (fixture `folded_prs`: 60 PR merges via `commit-tree`; opened through `update::opening` + the real `JobRunner`, then held `j` × 50 through the loop's `Timed` wakes). Release numbers: none taken (milestone review).

**Left for 06f (capture only; each panics in `graft_fixtures::check`, or is `capture_*`):**
- not named `capture_*`: `m7_walk::picker::take_refused_on_a_merge`, `resolver::shell_begun::c_y_finishes_a_shell_begun_merge` (05r's area), `search_hint::{after_enter_the_bar_is_the_rows_keys_and_the_search_is_in_the_header, the_search_hint_draws_his_esc_spelling}`, `tour_walk::the_whole_tour_in_a_pty_ticks_all_eight_and_comes_home` (`m8-tour-finished-120x36`: header `10 commits` → `6 commits` at first diff), `update::tests::keymap_edge::palette::at_80_the_bar_ends_with_keys_and_help_lists_what_it_dropped`, `update::tests::keymap_edge::remaps::j8_a_keys_table_beside_suspend_moves_merge_and_says_one_problem`;
- `capture_*`: `hint_bar::capture_the_doorway_on_a_marked_row_for_docs_shots`, `look_chrome::capture_the_chrome_at_every_designed_size`, `m8_walk::look::capture_help_over_the_graph_at_187x50`, `radar_not_checked::capture_radar_not_checked_with_an_evil_merge`, `update::tests::keymap_edge::remaps::capture_merge_remapped_to_shift_m`;
- **need a D3 route, not a re-shoot** (their subject `[feature/two]` is folded, they panic before the capture): `look_chrome::capture_the_cursor_row_in_dark_and_light` (look_chrome.rs:242), `m8_walk::look::capture_the_cursor_row_at_80x24_and_187x50` (look.rs:27);
- `lane_cap::{capture_lane_cap_for_docs_shots, the_pane_narrows_the_graph_so_the_graph_folds_more}`, `look_polish::capture_folded_merge_count_for_docs_shots` (retire per D3).
I did not classify these diffs as fold-legit vs regression (D3's rule): that is 06f's read.

**Touches:** shared fixtures — the corpus OctopusMerge is copied, not changed (new ref in the scratch copy only); `look_chrome::opened` unchanged, so its captures draw as before; no capture uses `octopus_with_an_unmerged_branch` (grep: journeys.rs, teaches.rs only). `over_400.txt` rows: settle.rs 708→710, outside_repo.rs 577→582. TESTING.md journey counts untouched. No product source changed; no MAP.md change (test-only items).

**Question (not a defect, no line proves it wrong):** the cursor and the fold tail share `▸` on one row (`▸ ◆ … ▸ 2 commits from …`). Tests now tell them apart by column; whether a person does is 07's look.
