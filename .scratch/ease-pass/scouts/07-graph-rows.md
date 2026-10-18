# Scout — 07 graph-rows (2026-10-18, main @ e9b08d12)

Paths below are post-rename: `crates/mergewright*`.

## Blockers are not all done (stop)
- 06s `split-update-files`: **Status: draft** (`.scratch/ease-pass/issues/06s-split-update-files.md:3`). 06t `split-view-files`: **Status: draft** (`06t…md:3`). The ticket says the scout reads the split layout. There is no split layout yet: `view/entries.rs` 428 lines, `view/rows.rs` 844, `update/keys.rs` 487, `update/state.rs` 1318.
- 06r: "built, 10/12 boxes; Q1, Q2 open" (`06r…md:3`). 03r: "partial" (`03r…md:3`). 06: "partial, closed for building". 06h and 06i: "ready", not built, and 06s/06t wait on them.

## What exists
- **Roles:** `LaneRole{This,Under,Trunk,Other}` at `mergewright-graph/src/role.rs:14`. `Route::new/unread` at :28–58. `made_on` at :96. Nothing in the `mergewright` app crate uses `Route` or `LaneRole` (grep returns nothing). There is no `AppState.route`.
- **`Row.lanes`** is at `mergewright-graph/src/row.rs:52`. The ticket cites `row.rs:24,45`, which is stale.
- **`Token::Role*`** exists (`mergewright-ui/src/theme/mod.rs:126+`). Lane colour today comes from `Theme::lane(usize)` (`theme/lanes.rs:86`) and `Theme::branch(identity)` (:149), with `branch_muted` at :110. No `colourless` exists.
- **Glyphs:** set in `graph_column/geometry.rs` and `graph_column/style.rs`. The ASCII tier uses `to_ascii` (`style.rs:111`). Node glyph: `NodeKind::glyph` (`kinds.rs:59`).
- **Row cells:** `GraphEntry` (`graph_screen/entry.rs:13`) holds `badges`, `author: AuthorChip`, `sha`, `when`, `fold`, `marker`, `tag`. Layout: `graph_screen/row_columns.rs` (author breakpoint, `SHA_WIDTH`, `sha_x` :39), consts in `columns.rs`. `short_sha` is at `columns.rs:214`. `badge_spans` is at `graph_screen/render.rs:547` (test at `tests.rs:1943`).
- **Row context choice site:** it has moved to `view/rows/context.rs:107` `cursor_row_context` (the ticket cites `hints/context.rs:47`). `BranchRow` is picked from `cursor.badges` `BadgeKind::Branch` (:131–139), so removing badges removes that input. `PlainRow` is also picked at `view/rows.rs:308`, `:765` and `update/commit_editor/done_row.rs:58`. There is a `Rewritable(Box<_>)` wrapper (:141), which the ticket does not mention. `GraphRow` is already picked for `JustCommitted`/`JustMerged` (:128).
- **`RowBar`** (`hints/context/row_bar.rs:12`) has `Own`, `Elsewhere`, `FoldedMerge`, `OpenedMerge`, `JustCommitted`, `JustMerged`. `bar_name` is at :47.
- **`InsideMerge`** is at `view/rows/fold.rs:27`. It is `pub(crate)` and marked dead code ("07 draws from it"). Its `label` is `Option<Vec<u8>>` taken from the subject. This scout did not find a resolver from `label` to a live branch: *could not confirm* one exists.
- **Two `FoldCount` enums:** `mergewright-ui/src/graph_screen/fold_mark.rs:9` and `mergewright/src/update/fold_count.rs:19`.
- **Keys:** `graph.journal` is at `keymap/table.rs:194` and `graph.copy_sha` (`y`) at :193. `Graph` rows are still in `table.rs`; `keymap/table/` holds only `commit_editor.rs` and `resolver.rs`. The test to flip is `keymap/tests/screen_keys.rs:59`, and its module doc at :4 already names `graph.colours_off`. App-side, a bound action is listed in `update/keys/bound.rs:78`, for example "`is_copy_key` → `copy_selected_sha`" (`update/keys/actions.rs:45`). The ticket also cites `keymap/claims.rs:48,59`.
- **`↑N to push`:** `push_words` (`update/commit_editor/landing.rs:141`) reads `counts.ahead`, which is a `usize` (`mergewright-git/src/history/divergence.rs:29`). Today it renders as the header's `N commits to push` (`graph_screen/view.rs:56`), not as a row label. `radar.rs:166` converts it to `u32`.
- **Contains read:** `jobs/detail.rs:340` `read_contained`, product `ContainedIn` (`update/events.rs:318`). `event_loop/rest.rs:3` lists it as per-row rest work. I did not measure its cost.
- **Scroll and hit tests:** `History::visible(height)` returns `height` rows (`history/scroll/query.rs:172`), which is one row per line. `geometry::graph_row_at` (`update/geometry.rs:311`, offset = line − rows_above) is used by mouse (`mouse.rs:98,134`) and drag (`drag.rs:71,90`). `History::row_at` is used by `context_menu.rs:56,178`, `drag.rs:180` and `fold_count.rs:63`.

## Mismatches
1. Blockers 06s, 06t, 06h, 06i are not done, and 06r and 03r are partial.
2. The choice site is at `view/rows/context.rs:107`, not `hints/context.rs:47`.
3. `Row.lanes` is at line 52, not 24/45.
4. The `Rewritable` wrapper and the extra `PlainRow` pickers (`rows.rs:308,765`, `done_row.rs:58`) are not in the scope list.
5. `InsideMerge.label` is bytes parsed from the subject, and this scout found no live-branch lookup (*could not confirm*).
6. Signatures put `LaneRole` in the ui crate's API; it lives in `mergewright-graph`.

## Build with
**Opus.** The ticket spans two crates and about 15 touch points, needs scroll and hit-test semantics to change, has stale anchors, and its prerequisites are unfinished. It should also be split at the ticket's (a)/(b) point.
