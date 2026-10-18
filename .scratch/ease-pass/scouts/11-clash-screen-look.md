# Scout 11 — clash-screen-look (HEAD 3b734080)

## Arrangement (retirement candidates)
- `SIDE_BY_SIDE_MIN = 100` conflict_resolver.rs:69; `THREE_PANE_MIN = 180` :73; `Arrangement {ThreePane, Card}` :77; `arrangement()` :86.
- `Placement::arrangement` files.rs:128 maps Column→ThreePane, Strip→Card; `placement()` files.rs:139 gives a **Column only at ≥180**, strip below. `layout()` layout.rs:108 dispatches Card→`card_rows`, ThreePane→`panes::pane_rows`. `three_panes` panes.rs:93 (+ panes/band.rs, panes/draw.rs).
- App: `view::resolver::arrangement` view/resolver.rs:145, `resolver_layout` :135.
- Pinned by: graft-ui tests/resolver_layout.rs:133–206; conflict_resolver/tests/three_panes.rs (whole file); card tests call `Arrangement::Card` (tests.rs:137,185,228,243; card_colours.rs; tests/finish_row.rs:70,116); graft tests/it/resolver/three_panes.rs, frame_budget.rs, update/resolver/tests/three_panes.rs. Captures: docs/shots/m5-06-stop-lands-in-resolver-{80x24,120x40,160x48}.txt.

## Blocks
- `RegionView::{Plain{numbers,lines}, Block(BlockView), WholeFile{how,keys,lines,note,ours_absent,theirs_absent}}` :152. `BlockView{index,of,state,ours,base,theirs,first_line,numbers}` :189 — no result text; `BlockState::{Open, Took(TakeView)}` :215, `TakeView::{Ours,Theirs,Both}` :224. Taken drawn in draw/block.rs (not read in detail — could not confirm exact taken look).

## Header, banner, list
- Title `name · N conflicts · M open` / `done ✓` from `resolver_title` view/resolver.rs:194; widget prefixes `▸`. No `merging X into Y` anywhere in graft/graft-ui src (grep).
- Banner: `banner_line` graph_screen/banner.rs:124 = `banner.kind.as_str()` + step/rewrite; no from/into words.
- List rows: `file_list_view` view/resolver/file_list.rs:17 maps **`RowState`** (update::resolver::listed_files), not graft-ops `FileState`; merge `AllTaken`→`Solved`. Words in `FileRowLook::words` files.rs:75–92: `N of M open`, `solved ✓`, `done ✓`, `keep or delete`, `binary`. Tone in files/words.rs:28. No combined-files section in the list; "combined by git" exists only in the finish summary (view/finish.rs:142–189, update/finish/words.rs:91).

## Screen-drawn words, clicks, bar
- Finish row words built in **app** `view::resolver::finish_row` view/resolver.rs:91–129 (`✓ every file done — c review and finish the merge`, `c` spelled via `resolver.finish`), drawn by graft-ui finish_row.rs on the doc's notice row (not a list row).
- Click: update/resolver/scroll.rs:89–95 → `finish_row_at` → **direct call** `leave::finish_pressed` (leave.rs:20) → `banner::continue_pressed`. Key `c` in a merge goes instead to `review_keys` (review.rs:33, act order update/resolver.rs:374 before :375).
- Bar: `resolver_context` view/rows/clash_bars.rs:56 — `Clash` only via `merge_solved_bar` :87 (FileSolved/AllSolved, merge only, full path); everything else `HintContext::Resolver{base,restart,whole_file,done,roles,finishable}` :68. No `Open`/`Picked` picked anywhere.
- `SideRoles` from `update::resolver::side_names(state).roles` (names.rs:52–91, all ops).
- Keys in `act` update/resolver.rs:359–380: esc, j/k, B, clash_keys, ⏎, e, d, M, review_keys, c, undo_keys.

## Size
- files.rs 397: `FileRowLook` :45–93 is the natural cut (enum + `words`). update/resolver.rs 398: clicks are **not** there (scroll.rs, 166) — no split needed for the click.

## Mismatches with the ticket
1. Click on finish row calls `finish_pressed`→continue directly (scroll.rs:94), while `c` in a merge opens the review (review.rs:40) — the click today bypasses 05s's door.
2. List built from `RowState` (file_list.rs:28), not `FileState`/`file_state`; `FileState` used only in review.rs:172.
3. Column list only at ≥180 (conflict_resolver.rs:73), ticket says list column 30 at ≥100.
4. Finish row lives on the doc notice row and is worded by the app (view/resolver.rs:91), not a list "last row" nor `conflict_resolver` screen code.
5. update/resolver.rs split premise void: click arms are in scroll.rs.
6. "glyph/word mapping" partly already split: tone in files/words.rs; `words()` still in files.rs:75.
7. Merge `Solved` currently drawn `solved ✓` (files.rs:82) — user's answer `done` changes it.
8. No `Combined by git` read feeding the list; only the finish summary has it — could not confirm reuse.

Build with: Sonnet high (large, many captures; mechanical-to-moderate) — conductor decides.
