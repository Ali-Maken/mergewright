# fold-captures — every capture the fold default changes, re-shot or fixed by D3's rule; the folded and opened main screen

Status: done — 5/5 (box 5 closed by the debugger, 068b98c3)
Blocked by: 06e
Build with: Opus (`builder-opus-xhigh`) — sorting each diff into fold-only or regression is the step where a regression gets shot in as "expected". It needs judgment, not a script.
Needs: from 06d/06e: the fixed parents, the HEAD keep, fixtures (including any unmerged branch 06e added).

## Read first
- `.scratch/ease-pass/issues/06-graph-fold-and-lanes.md`: Handoff 2 items 3–5; **Decisions for continuation 2, D3**; Acceptance box 9.
- 06e's NOTES: the capture tests whose fixture changed.
- `TESTING.md`: the "what right looks like" line for each capture below. ADR-0037 (contact sheet).
- `crates/graft/MAP.md`, `crates/graft-ui/MAP.md`, `crates/graft-graph/MAP.md`: the sections 06/06d changed.

## Scope
In: the `capture_*` tests, `lane_cap.rs`, `look_polish`, `docs/shots/**`, the ease-pass contact sheet, capability M1-09's citation, and the MAP.md files.
- **Diff each capture first, then classify it by D3.** Fold-only → re-shoot. Anything else → fix the cause or stop.
- **Expected fold-only** (background graph): m8-look-chrome-80x24, m8-look-help-graph-187x50, m8-help-80x24, m8-keymap-config-problem-120x36, m8-keymap-remapped-120x36, m8-quit-stop-now-80x24, m8-quit-waits-80x24, m8-bar-after-search-120x36, m8/search-hint-120x36, m5-ui-name-prompt-80x24.
- **Check the subject is still shown first:** m7/rebase-line-preview-187x50, m7/rewrite-refused-fork-120x40, m7/refused-box-single-close-120x40, m7/rebase-stop-forecast-80x24, m5-ui-preview-rebase-80x24, m5-ui-preview-scrolled-80x24 — predicted rows never fold. Also `look_chrome::capture_the_cursor_row_in_dark_and_light` and `m8_walk::look::capture_the_cursor_row_at_80x24_and_187x50`.
- **Re-check after 06d** (the Fix menu was the parents bug): m6/fix-preview-verb-120x40, m6/fix-finish-shell-merge-120x40. Their diffs must be fold-only.
- **Lane cap:** `lane_cap::*` and m5-lane-cap-160x24. Make the fixture's topics unmerged branches (D3 route 2). 06b will re-shoot it again.
- **Superseded:** `look_polish::capture_folded_merge_count_for_docs_shots` → retired. `15-merges-folded.txt` / `15-merges-unfolded.txt` → deleted if `grep` finds no other citer. M1-09 is repointed to box 9's captures.
- `m7_walk::pr_contents::capture_compare_since_split` — diagnose it by 06e's finding for `pr_contents`.
- **Box 9:** `docs/shots/ease-pass/main-folded-120.txt` and `main-opened-120.txt`, from 06e's box 5 fixture. Opened = `space` on the PR 16021 merge.
- MAP.md entries for `Row::drawn_parents` / `is_merge`, `View::folded`, `Window.fold`, `FoldMark` / `FoldCount` (`map_coverage` passes).

Out: lane drawing and header words (06b); the opened rows' look (07).

## Signatures
None new.

## Touches
`docs/shots/**` (≤200 KB each), the contact sheet, `TESTING.md` capture lines (only if a path changed, never "what right looks like"), the capability index (M1-09), and MAP.md files.

## Acceptance
- [x] NOTES has a table: capture → fold-only / regression-fixed / retired, a one-line diff summary, and the test that writes it. Every row on Handoff 2.5's list appears. — NOTES table below; each row names its writer test.
- [x] No re-shot capture's diff has anything outside D3's allowed set (checked by the builder per row; the user judges the look by eye — rule 6; colour unverified — rule 5). — Every row checked by `git diff` per file. The one non-D3 diff found (folded merges drew `●`, not `◉`) was fixed in code, red first: `graft-ui graph_column::tests::a_merge_whose_side_is_not_drawn_is_still_a_merge`. Route-2 rows (fixture changed, D3's own route) are marked as such. **Not re-shot:** `m8-tour-finished-120x36.txt` (blocked, see NOTES).
- [x] Box 9 in 06 ticked: both captures exist, are written by named tests, and are in the contact sheet next to `prototypes/ease-mocks/mergewright-main-screen.html`. — `fold_by_default::capture_main_folded_and_opened_at_120`; sheet `docs/shots/ease-pass/ease-pass-contact-sheet.txt` (captions name the mock).
- [x] M1-09 cites the new captures; the `graft-capabilities` checker passes. — `docs/capabilities/m1-the-graph.md` M1-09; `gen-capabilities` regenerated; checker: graft-capabilities tests in the workspace run (see NOTES).
- [x] The workspace is fully green. — After the debugger's fix (NOTES, "Fixed by the debugger"): `tour_walk::the_whole_tour_in_a_pty_ticks_all_eight_and_comes_home` (re-shot `m8-tour-finished-120x36.txt`) and `tour_walk::a_screen_still_sorting_is_not_ready_to_shoot` green, 3 runs of 3; the bug's own tests `event_loop::tests::settle::fold::a_count_answered_mid_burst_asks_the_next_at_rest` and `outside_repo::launch_and_open_land_alike` (now waits for both counts). Full workspace nextest + `cargo test --doc` green.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~50k. Most rows are a diff and a re-shoot. Any regression found → fix it if it is small, or stop and report it with its line.

## NOTES (06f builder, 2026-10-18)

Not compacted. **Partial:** one capture blocked by a product bug (below). No scout: 06e's NOTES listed every failing capture and its cause; each diff was read directly (`git diff` per file).

### Regression fixed (code, red first)
- **A folded merge drew `●`, not `◉`.** `crates/graft-ui/src/graph_column/kinds.rs:44` read `row.drawn_parents.len()` (06d's D1 change), and a folded merge draws one parent. The accepted mock draws every folded row `◉` (`prototypes/ease-mocks/mergewright-main-screen.html:115-129`, `g:"◉"`). Seen in the first re-shoot (`m8-look-chrome-80x24`: `◉─│─│─╮ Merge branch 'feature/one'` → `● │ …`). Now reads `row.commit.parents.len()` (git's commit, 06 D1). Test: `graph_column::tests::a_merge_whose_side_is_not_drawn_is_still_a_merge` (failed before: `left: Commit, right: Merge`). Knock-on: the first-parent view's merges also draw `◉` now. No checked capture draws it (full run: no other test changed); the frozen `15-first-parent.txt` (no producer) still shows `●`. 07 owns the G3 glyphs, so it can change this back.

### Fixed by the debugger (2026-10-18): the second count now lands
- **Cause, confirmed** by a tagged log in the real binary (pty run of `launch_and_open_land_alike`, log removed): the window landing moved the cursor to `HEAD` (a motion, `Rest::moved`); every answer after it came within `BURST_GAP`, so `defer_follow` was `true` for each, `FoldedCounted` for `feature/two`'s merge included, and `fold_count::at_rest` was skipped. No follow was owed, so `rest_deadline` gave no deadline and no `Rested` wake came. The candidate below was right.
- **Fix (cause):** the loop's rest wake is owed for every kind of work a burst defers, not only the follow. `update::rest_owed` = `follow_behind || fold_count::owed` (a folding merge on screen with no count asked); `Run::rest_deadline` (`event_loop/running/turn.rs`) uses it. No sleep, no polling: one `Rested` at the burst's own quiet deadline, as for a follow. A held `j` still reads no count mid-burst (`a_held_j_over_a_folded_walk_follows_and_counts_only_at_rest` green).
- Cost to him: none seen; one extra `Rested` update (and frame) per burst that left a count owed.
- Tests: `event_loop::tests::settle::fold::a_count_answered_mid_burst_asks_the_next_at_rest` (red before: no ask at rest); `outside_repo::launch_and_open_land_alike` now waits for `▸ 2 commits from feature/one` too (red before at 20 s; green in ~2 s).
- Re-shot `m8-tour-finished-120x36.txt`: fold-only (feature branches folded, both tails counted, 10→6 commits, cursor two rows under `HEAD` = `Add b.txt`). The m8 contact sheets are the milestone's record and were not regenerated (as for the other 06f re-shoots).
- Touches: `rest_deadline` is the only caller of `rest_owed` (grep); `follow_behind` keeps its own callers (`rest_first`, `follow_waits`). Settle loop tests (`event_loop::tests::settle::*`) and `frames::paced` green in the full run; the release perf run is the milestone review's. `over_400.txt`: `outside_repo.rs` 582→587. MAP `update::fold_count`.

### Regression found, NOT fixed: the second folding merge's count never lands (real binary)
- Seen: the pty tour home (`tour_walk`) drew `◉ │ Merge branch 'feature/one' ▸ commits from feature/one` (no number) for **90 s** (temporary PATIENCE 90 s, reverted). On a plain launch too: `outside_repo::launch_and_open_land_alike` with `▸ 2 commits from feature/one` added to its wait (temporary, reverted) timed out at 20 s, `feature/two`'s count landed, `feature/one`'s did not. The in-process harness (`App::pump`) lands both every time (look_chrome captures).
- **Cause not confirmed.** Candidate, with lines but **unverified**: `fold_count::at_rest` asks one merge per rest, gated on `!next.defer_follow` (`crates/graft/src/update/mod.rs:251`). The loop sets `defer_follow = rest.in_burst(now)` for every wake, job answers included (`crates/graft/src/event_loop/running/turn.rs:331`), and schedules a `Rested` wake only while a follow is owed (`rest_deadline`, same file, ~line 340). If the first count's answer lands inside a burst, nothing wakes the loop afterwards to ask the second. **Could not confirm**. Needs the debugger. A fix would be in the G-settle loop, which the perf tests guard, so it is not small.
- Effect for him: on a launch, every folding merge after the first on screen can show `▸ commits from …` with no number until he presses a key.
- Knock-on here: the `m8-tour-finished-120x36.txt` re-shoot is **blocked**. Not re-shot, because that would shoot a pending count in. `tour_walk::ready_to_shoot` now treats a tail with no number as still in flight (`▸ commits` / `▾ commits` in `IN_FLIGHT`), and `a_screen_still_sorting_is_not_ready_to_shoot` asserts that (it also expects the folded header `6 commits`). Both tour tests stay **red** until the count lands and the capture is re-shot. Earlier runs had shot the pending tail in, unseen: the tour shot the screen before counts landed.

### Captures: classification (D3)
"Fold-only" = only rows gone or moved up, `▸ N commit(s)` tails, header count, lanes gone. "Route 2" = D3's fixture route (new unmerged ref only). "Unchanged" = its test passed against the committed file after 06d+06e+main.

| Capture | Class | Diff in one line | Written by |
|---|---|---|---|
| m8-look-chrome-80x24 | fold-only | feature/one, feature/two rows gone; `▸ 2 commits from …` tails; 10→6 commits; 2 lanes gone | `look_chrome::capture_the_chrome_at_every_designed_size` |
| m8-look-chrome-120x36, -187x50 | fold-only | as 80x24 | same |
| m8-look-help-graph-187x50 | fold-only | the graph behind the palette as above; palette unchanged | `m8_walk::look::capture_help_over_the_graph_at_187x50` |
| m8-help-80x24 | fold-only | hand-built merge row gains `▸ 1 commit` | `update::tests::keymap_edge::palette::at_80_the_bar_ends_with_keys_and_help_lists_what_it_dropped` |
| m8-keymap-config-problem-120x36 | fold-only | `▸ 1 commit` on the merge row | `update::tests::keymap_edge::remaps::j8_a_keys_table_beside_suspend_moves_merge_and_says_one_problem` |
| m8-keymap-remapped-120x36 | fold-only | `▸ 1 commit` on the merge row, 3 frames | `update::tests::keymap_edge::remaps::capture_merge_remapped_to_shift_m` |
| m8-quit-stop-now-80x24, m8-quit-waits-80x24 | unchanged | — | `quit_waits.rs` |
| m8-bar-after-search-120x36 | fold-only | octopus topics gone, `▸ 3 commits`, 5→2 commits; search words and bar unchanged | `search_hint::after_enter_the_bar_is_the_rows_keys_and_the_search_is_in_the_header` |
| m8/search-hint-120x36 | fold-only | as above, 3 frames | `search_hint::the_search_hint_draws_his_esc_spelling` |
| m7/rebase-line-preview-187x50 | unchanged (subject shown) | — | `m7_walk::rebase_line` |
| m7/rewrite-refused-fork-120x40 | unchanged (subject shown) | — | `m7_walk::rewrite_editor` |
| m7/refused-box-single-close-120x40 | unchanged (subject shown) | — | `offer_and_blame.rs` |
| m7/rebase-stop-forecast-80x24 | unchanged (subject shown) | — | `rebase_stop_forecast.rs` |
| m5-ui-preview-rebase-80x24, m5-ui-preview-scrolled-80x24 | unchanged: predicted rows did not fold | — | `preview.rs` |
| m5-ui-name-prompt-80x24 | unchanged | — | `name_and_drag.rs` |
| m6/fix-preview-verb-120x40 | unchanged after 06d | — | `preview.rs` |
| m6/fix-finish-shell-merge-120x40 | fold-only | feature's row gone, `▸ 1 commit from feature`, 7→6, its lane gone; cursor on the merge as before | `resolver::shell_begun::c_y_finishes_a_shell_begun_merge` (05r's file: capture only, test untouched) |
| m7/radar-not-checked-evil-120x40 | fold-only | alpha's row gone, `▸ 1 commit from alpha`, 4→3; `⚠` unchanged | `radar_not_checked::capture_radar_not_checked_with_an_evil_merge` |
| m7/take-refused-merge-120x40 | fold-only | login-page's row folded into `merged`'s merge (its two badges go with the row), `▸ 1 commit`, 7→6; refusal box unchanged | `m7_walk::picker::take_refused_on_a_merge` |
| m8/doorway-marked-120x36, -140x40 | fold-only | alpha folded as radar; `⚠` and doorway bar unchanged | `hint_bar::capture_the_doorway_on_a_marked_row_for_docs_shots` |
| m8-look-cursor-row-dark/light-120x36 | route 2 | new `feature/two-next` (one commit on `feature/two`) keeps `[feature/two] two: finish` drawn; cursor row `▸ │ │ ●─╯` (its join added); feature/one folded; 10→9 | `look_chrome::capture_the_cursor_row_in_dark_and_light` |
| m8-look-cursor-row-80x24, -187x50 | route 2 | as above | `m8_walk::look::capture_the_cursor_row_at_80x24_and_187x50` |
| m5-lane-cap-160x24 | route 2 (fixture reshaped) | 14 unmerged topics forked from main's base, 30 main commits, scrolled to the oldest; pane open `+8 lanes` (was +6), closed `+4`; caption's fixture words and `+8` updated | `lane_cap::capture_lane_cap_for_docs_shots` |
| m8-tour-finished-120x36 | **blocked** | needs the second count to land (above); cursor change (two: start → Add b.txt) is the test's "two rows under where he landed" (`tour_walk.rs:652-655`) taking route 1 by itself | `tour_walk::the_whole_tour_in_a_pty_ticks_all_eight_and_comes_home` |
| m8/polish-fold-120x36 | retired | `look_polish::capture_folded_merge_count_for_docs_shots` removed; file kept as a frozen record (`FROZEN.md` row) because KNOWN_GAPS and the m8 contact sheet cite it | — |
| 15-merges-folded, 15-merges-unfolded | deleted | only citers were M1-09 (repointed) and `FROZEN.md` (rows removed); `.scratch/` history mentions are records | — |
| ease-pass/main-folded-120, main-opened-120 | new (box 9) | — | `fold_by_default::capture_main_folded_and_opened_at_120` |

Every re-shot capture is ≤ 3 KB except the sheet (82 KB).

### Questions (no line proves them wrong)
- **Search counts matches in folded rows.** `m8/search-hint-120x36`: `/topic 4 matches (1/4)` while 3 of the 4 matching rows are folded. Whether `n` should open the fold is 06c's seek-reveal.
- **Design-note wording vs the spec on which ref tips stay shown.** `docs/design-notes/ease-pass.md:199-200` says "refs not reachable from trunk"; the 06 ticket (`06-graph-fold-and-lanes.md:28`) and the code (`graft-graph/src/inclusion/fold_walk.rs:5-6`) say "a ref tip no shown merge reaches through a second parent". `take-refused-merge` follows the code: `login-page` (merged only into `merged`, not into `main`) folds. For his dependent-branch team work this decides whether a branch merged into a colleague's unmerged branch shows. Planner's call.
- `FROZEN.md` listed `m8-look-cursor-row-dark/light-120x36.txt` as "no producer". `look_chrome` writes them through a `format!` name, so those rows were removed.

### Touches
- `NodeKind::of`: callers `graph_column/mod.rs:121,146` only (grep). The full workspace run changed no capture beyond the table. MAP entry updated.
- Test fixtures: `look_chrome::opened` unchanged for every other caller (`opened_in` + `with_feature_two_kept` added; only the 2 cursor-row tests use it, grep). `lane_cap::many_lanes` is used only in `lane_cap.rs`.
- `over_400.txt`: `graph_column/tests.rs` 795→807. `tour_walk.rs` 934→942.
- Docs: M1-09 Looks like; `CAPABILITIES.md` and `docs/generated/capabilities.html` regenerated; `FROZEN.md`; MAP.md (`graft`: `View`/`View::folded`, `Window.fold`; `graft-ui`: `NodeKind::of`). `Row::drawn_parents`/`is_merge`, `FoldMark`/`FoldCount` were already there.
- Contact sheet: `docs/shots/ease-pass/ease-pass-contact-sheet.txt` + captions TSV (`scripts/contact-sheet.sh --captions-order ease-pass docs/shots/ease-pass`). Its header says `Promise: NOT RECORDED`: the script has no ease-pass promise. That is 14's (scripts/ is outside this ticket).
- TESTING.md: no capture path changed, so not touched.
- Keymap, journal/undo: none.
