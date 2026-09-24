# fix-resolver-file-list: every file this step conflicted on stays on screen, with its state and the current file marked

Status: ready (filed 2026-09-23; P1–P8 approved by the user, P9 by the conductor — see docs/field-notes.md)
Blocked by: 09-ops-conflict-set-and-summary.md, 10-resolver-line-numbers.md, 22-ops-finish-message-and-step-bound.md, 24-review-fix-amend-11-12.md (same crate set, ADR-0036: not concurrent)
Was: draft · new 2026-09-23 (Amendment 2, ADR-0043 §1). Not gated: the list's display and mouse selection are in-spec. `j`/`k` on it are P7 (`fix-resolver-file-keys`).
**Revised 2026-09-23 (review `docs/reviews/m6-amend-09.md` Minor 3, ruled by the planner).** For a rebase, cherry-pick or revert, the list shows **the current step's files only**: what is unmerged now, plus what was resolved since this step stopped. Files an earlier step conflicted on are already inside a commit, so they are not listed. A merge is one step, so it keeps its whole set. The bound is built in graft-ops (`ops-finish-message-and-step-bound`). This ticket proves it end to end.
**Revised 2026-09-24 (spec-only pass on `.scratch/m6-conflicts/scout-13.md`).** Rulings:
- R1. This ticket **owns** `THREE_PANE_MIN`, `Arrangement`, `arrangement()` (pure, three lines). `layout`'s signature is **not** changed here; `fix-resolver-three-panes` changes it and builds the `ThreePane` rows. Breaks the 13 ↔ 14 cycle.
- R2. `ConflictSet` is order only (`graft-ops/src/conflict/set.rs:27`). Row state is derived app-side by one pure fn, `listed_files` (Signatures). `file_cursor` is a **function** of the open path, not a field: it cannot drift from `ResolverState::path`.
- R3. `fits` (`update/resolver/geometry.rs:60`) is **unchanged**. The strip lives inside the widget's `Rect`, like `HEADER_ROWS` does today; the floor stays `MIN_ROWS` on the widget's whole area. Draw and hit-test both split that area through one fn, `resolver_areas`.
- R4. After the last file, this ticket keeps today's **close to the banner** (`02-keymap.md:270`, `⏎` row). "Stays open" (`02-keymap.md:269`, P8) is `fix-finish-summary-screen` Part B's, which also binds `c` in the resolver; staying open before `c` works there would strand him.
- R5. The "every file done" words go on the **status line**, today's site (`update/resolver/answers.rs:245-251`), for every kind, stash pop included. P8 moves them into the resolver's notice row (ticket 19 Part B).
- Keys: this ticket binds **none** (checked `02-keymap.md` §2.6 :258-273: `j`/`k` P7, `O`/`T` P6, `c` P8 are other tickets'). Mouse only.

Build with: **Opus** (scout: new resolver state across a file switch, a strip and a column with shared hit-testing, the document's origin moving under every existing click).
Needs: `fix-ops-conflict-set-and-summary` (`operation_conflicts`, `ConflictSet`); `ops-finish-message-and-step-bound` (the step-bounded set, `Shape::TwoStopRebase`); `fix-resolver-line-numbers` (`DocView.names`).
Crates: graft-ui, graft.

## Read first
- `.scratch/m6-conflicts/scout-13.md` (whole, one page).
- `docs/adr/0043-resolver-file-list-and-three-panes-when-wide.md` §1.
- `docs/design-notes/m6-conflicts.md` Amendment 2: Journeys J4, J8, J9; "Where each event lands him"; New UI "≥180 columns" (list column only) and "Below 180 columns" (strip, grid, states).
- `crates/graft-ui/MAP.md` `conflict_resolver`; `crates/graft/MAP.md` §`update::resolver`, §`update::banner` (`unmerged_paths`), §`update` (`Loading`, `Command`, `Product`), §`view` (`resolver_context`, the resolver draw site).
- `crates/graft-ops/MAP.md` `ConflictSet`, `operation_conflicts` (stash pop: no `MERGE_MSG`, so its set is the unmerged paths only).

## Scope
In:
1. **The set.**
   - On `open_resolver`, and after every `Stage` / `ResolveWholeFile` answer, push `Command::ReadConflictSet(id)`; the job runs `graft_ops::conflict::operation_conflicts`.
   - `Product::ConflictSet` → `ResolverState::files`. A re-read keeps the list it refreshes (`set_job` holds the id, `JournalState::reading`'s pattern). An answer whose id is not `set_job` is dropped: comment why it is safe (a newer read is coming).
   - Opening the resolver on a new step reads the set afresh: no rows carry over from the previous step. The app takes `ConflictSet` as given, no filtering.
2. **Row states** — `listed_files(&AppState)` is the one source of order and state; the view, the mouse and `resolved_advance` all read it.
   - Order: `ConflictSet::paths` when `Ready`; then every path in `banner::unmerged_paths` not already listed, in git's order. `Idle` / `Pending` with no earlier set / `Failed` → `unmerged_paths` only.
   - State, first match wins:
     - status not arrived (`working_tree` `None`) → `Unread` for every row but the open one;
     - not in `unmerged_paths` → `Done`;
     - the open path, document loaded → `Open { open, of }`, or `AllTaken` when `open == 0`;
     - the open path, whole-file shape → `Binary` if binary, else `WholeFile { how }`;
     - otherwise `seen[path]`, else `Unread`.
   - `seen` records each file's last derived state when its read or take lands, while this resolver is open.
   - `Done` comes from the working-tree status only. If the walk harness shows the status lagging a `Stage` answer, **stop and report**; no staged-path special case (rule 2).
   - A stash pop's set has no record (graft-ops MAP): a done file leaves its list at the next re-read. Say so in the ticket NOTES for the conductor's gap row.
3. **Arrangement and placement** (graft-ui).
   - `arrangement(frame_columns)`: `ThreePane` iff `>= THREE_PANE_MIN` (180), else `Card`. Nothing but `placement` calls it in this ticket.
   - `Column { width }` when `ThreePane`, `width = clamp(longest row, 22, 32)`; else `Strip { rows }`.
   - The strip: one row when every `name · state` cell fits. Otherwise the compact pass (`1/2 open`, `✓`). Failing that, a grid of equal cells, row-major, at most 3 rows, keeping the cursor's row visible, `… N more` in the last cell. Below `SIDE_BY_SIDE_MIN` the strip is always compact.
   - A name too long for its cell or the column is cut from the **left** with `…`: the file name survives.
4. **Areas.** `resolver_areas(area, placement)`:
   - Strip: `list` = top `rows`; one rule row; `doc` = the rest.
   - Column: `list` = left `width`; one `│` column; `doc` = the rest, full height.
   - `ResolverScreen` draws `FileList` in `list` and today's title/notice/body in `doc`.
   - The app's `layout(&doc, …)` width, document height and `body_y` all come from `doc`: `body_y = doc.y + HEADER_ROWS`, the card's x from `doc.x`. Every site that computes them today (scout: `view/frame/conflicts.rs:34-43`, `update/resolver/geometry.rs:40-42`, `scroll.rs:43-60`) goes through `resolver_areas`.
   - Floor: `MIN_ROWS` is unchanged and still measured on the whole widget area; `fits` is unchanged.
5. **File identity row** replaces the title (`view::resolver::resolver_title`):
   - a text file: `▸ <path> · <n> conflicts · <k> open`;
   - a whole-file shape: `▸ <path> · <how words with SideNames>`, e.g. `modified on yours · main, deleted on theirs · feature`.
6. **Selection.** A click in `list` → `file_at` → `select_file(index)`.
   - `select_file` sets `ResolverState` to what `open_resolver` builds for that path, keeping `opened_from`, `files`, `set_job` and `seen`, and pushes `OpenConflict`. Cursor block 0, `top` 0.
   - The index of the open file → `true`, nothing sent.
   - While `job`, `staging`, `restarting` or `base_job` is in flight → refused with today's words at today's site (`still reading the file` / `still writing the last take` / `still loading the base`).
   - `Done` files open read-only, like today's reopened file (`0 open`); `d` starts one over, as today.
   - A click on the rule row or the `│` column does nothing.
7. **Landing after a file is done** (`resolved_advance`).
   - Next file in `listed_files` order after the one just staged, skipping `Done` rows and that file itself, wrapping. Notice (resolver's notice row) `<file> done ✓ · ▸ <next>`.
   - None left → resolver closes to the banner, as today (R4). Status line (R5): merge `every file done ✓ · c finishes the merge`; rebase `every file done ✓ · c continues the rebase`; cherry-pick `… · c continues the cherry-pick`; revert `… · c continues the revert`. Stash pop (no banner) → the graph, status line `every file done ✓`.
8. **`FileList` widget** draws only inside its `Rect`: `▸` on the cursor cell or row, `▏` gutter on the cursor row in the column. ASCII: `>` and `|`.
   - Column rows: `FILES · <n> conflicted`, a blank row (where ticket 14's pane titles sit), then one file per row, `name` then its state. Overflow → last row `… N more`, cursor kept visible.
   - Set `Failed` → `unavailable`, and the strip's label reads `files: git's list unavailable · the unmerged ones below` (note :662).
   - State words, full / compact: `Open` `k of n open` / `k/n open`; `AllTaken` `⏎ marks done` / `⏎`; `Done` `done ✓` / `✓`; `WholeFile` `keep or delete` / `keep?`; `Binary` `binary` / `bin`; `Unread` `…` / `…`.

Out:
- `j`/`k` (P7), `O`/`T` (P6), `c` in the resolver and "stays open after the last file" (P8, ticket 19 Part B).
- `layout`'s `Arrangement` parameter and the three panes. At ≥180 this ticket draws the column beside the **card** (at `doc.width`).
- The summary. Bounding the set (graft-ops).

## Signatures
```rust
// graft-ui/src/conflict_resolver.rs (R1; ticket 14 consumes, does not redefine)
pub const THREE_PANE_MIN: u16 = 180;
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Arrangement { ThreePane, Card }
pub fn arrangement(frame_columns: u16) -> Arrangement;
pub struct DocView { /* existing */ pub files: FileListView }

// graft-ui/src/conflict_resolver/files.rs (new; re-exported from conflict_resolver)
pub struct FileRow { pub name: SanitizedText, pub state: FileState }
pub enum FileState { Open { open: usize, of: usize }, AllTaken, Done, WholeFile { how: SanitizedText }, Binary, Unread }
pub struct FileListView { pub rows: Vec<FileRow>, pub cursor: usize, pub unavailable: bool }
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Placement { Column { width: u16 }, Strip { rows: u16 } }
pub fn placement(view: &FileListView, frame_columns: u16) -> Placement;
pub struct ResolverAreas { pub list: Rect, pub doc: Rect }
pub fn resolver_areas(area: Rect, placement: Placement) -> ResolverAreas;
pub fn file_at(view: &FileListView, placement: Placement, width: u16, dx: u16, dy: u16) -> Option<usize>; // offsets from `list`'s origin; shares the draw's cell geometry
pub struct FileList<'a> { pub view: &'a FileListView, pub placement: Placement, pub theme: &'a Theme, pub ascii: bool } // Widget

// graft/src/update/resolver/files.rs (new private child, re-exported as update::resolver::…)
#[derive(Clone, Debug, PartialEq, Eq)] pub enum RowState { Open { open: usize, of: usize }, AllTaken, Done, WholeFile { how: UnmergedHow }, Binary, Unread }
#[derive(Clone, Debug, PartialEq, Eq)] pub struct ListedFile { pub path: PathBuf, pub state: RowState }
pub fn listed_files(state: &AppState) -> Vec<ListedFile>;
pub fn file_cursor(state: &AppState) -> usize;   // index of ResolverState::path in listed_files; 0 when absent
pub fn select_file(state: &mut AppState, commands: &mut Vec<Command>, index: usize, next_id: &mut JobId) -> bool;
pub struct ResolverState { /* existing */ pub files: Loading<Arc<ConflictSet>>, pub set_job: Option<JobId>, pub seen: BTreeMap<PathBuf, RowState> }
pub enum Command { /* existing */ ReadConflictSet(JobId) }
pub enum Product { /* existing */ ConflictSet(Result<Arc<ConflictSet>, String>) }
```
`ARCHITECTURE.md`: `ui` never imports `ops`. `view::resolver` turns `listed_files` into `FileListView` (the one sanitising site; `WholeFile { how }` words through `SideNames`). The mouse builds the `FileListView` through that same function, as it builds the `Layout` today.

## Acceptance (name the verifying test when you tick)
- [ ] graft-ui `conflict_resolver::tests::arrangement_turns_at_180`: `Card` at 179, `ThreePane` at 180.
- [ ] graft-ui `conflict_resolver::files::tests::placement_by_width`: `Column` at 180 and 200; `Strip` at 179, 160, 120, 80. A one-row strip for 2 files at 120. At most 3 rows ending `… N more` for 12 files at 120 and 80.
- [ ] graft-ui `…::file_at_matches_what_is_drawn`: render into a buffer, read each cell's name back at the offset `file_at` returns, both placements; the rule row and `│` return `None`.
- [ ] graft-ui `…::areas_put_the_list_above_or_beside_the_doc`: strip → `doc.y == list.y + rows + 1`; column → `doc.x == list.x + width + 1`.
- [ ] graft-ui `…::the_floor_still_leaves_a_document`: 12 files at `MIN_COLUMNS × MIN_ROWS` → document rows `== MIN_ROWS − HEADER_ROWS − 4`, and `> 0`.
- [ ] graft `update::resolver::tests::row_states_follow_the_document_and_the_status`: one fixture per `RowState`, including `seen` counts for a visited, closed file.
- [ ] graft `…::the_cursor_stays_on_its_path_across_a_reread`: a set re-read that reorders rows moves `file_cursor` with the path.
- [ ] graft `…::a_click_on_the_document_still_picks_its_block`: with a strip of 1 and of 3 rows, a click on block 1's header row makes it the cursor block (the origin moved; nothing else may).
- [ ] graft `…::a_done_file_stays_listed`: two-file merge, `o` on `gone.txt` → the list reads `gone.txt done ✓`, `▸ notes.txt …`.
- [ ] graft `…::the_list_survives_a_relaunch`: the same after `Relaunch` in the walk harness (`recorded` from git).
- [ ] graft `…::a_rebase_lists_only_this_steps_files`, walk harness, `Shape::TwoStopRebase`, real git: resolve `a.txt`, `⏎`, `c` (stops at step 2); if the resolver is not open, `⏎` on the banner (P5 may have opened it; cope with either); open on `b.txt`; the drawn list has exactly one row, `▸ b.txt …`, no row names `a.txt`; resolve `b.txt` → `b.txt done ✓`, `a.txt` still absent.
- [ ] graft `…::a_click_on_a_file_opens_it` and `…::a_click_mid_take_is_refused_in_words`.
- [ ] graft `…::after_the_last_file_the_banner_says_c_finishes` (merge: resolver closed, banner focused, status line `every file done ✓ · c finishes the merge`), `…::after_the_last_file_a_rebase_says_c_continues`, `…::after_the_last_stash_file_the_status_says_every_file_done`. Ticket 19 Part B rewrites the first two.
- [ ] graft `…::an_unreadable_set_says_so`: `Product::ConflictSet(Err)` → the strip reads `files: git's list unavailable · the unmerged ones below`, rows from the status.
- [ ] Captures `docs/shots/m6/fix-file-list-{120x40,160x48,80x24,200x50}.txt` (two-file merge, after `o` on `gone.txt`), `…-many-120x40.txt` (12 files), `…-rebase-step2-120x40.txt` (`TwoStopRebase` at step 2). **What right looks like:**
  - At 120, the row under the header reads `files   gone.txt · done ✓    ▸ notes.txt · 2 of 2 open`. A rule follows, then `▸ notes.txt · 2 conflicts · 2 open`.
  - At 80, the compact strip reads `✓ gone.txt   ▸ notes.txt 2/2 open`.
  - At 200, a left column headed `FILES · 2 conflicted`, `▸ notes.txt` in it, `│`, then the card (three panes are ticket 14's).
  - Many: 3 grid rows, the last cell `… N more`.
  - Rebase step 2: the strip names `b.txt` only; `a.txt` appears nowhere on screen.
  - The user judges by eye.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Medium–large: one widget + placement + areas, one job wire-up, one derivation, a moved document origin, ~15 tests, 6 captures. One context. If it won't fit, split at the seam and say so: graft-ui (`arrangement`, `files.rs`, `resolver_areas`, widget, its tests) first, then the app.
