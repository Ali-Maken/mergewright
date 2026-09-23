# fix-resolver-file-list: every file the operation conflicted on stays on screen, with its state and the current file marked

Status: ready (filed 2026-09-23; P1–P8 approved by the user, P9 by the conductor — see docs/field-notes.md)
Blocked by: 09-ops-conflict-set-and-summary.md, 10-resolver-line-numbers.md
Was: draft · new 2026-09-23 (Amendment 2, ADR-0043 §1). Not gated: the list's display and mouse selection are in-spec. `j`/`k` on it are P7 (`fix-resolver-file-keys`).
Build with: blank — brownfield (ADR-0035): **scout first**. Expected **Opus**: new resolver state across a file switch, and a strip and a column with shared hit-testing.
Needs: `fix-ops-conflict-set-and-summary` (`operation_conflicts`, `ConflictSet`); `fix-resolver-line-numbers` (`DocView.names`, which lands first in the same files).
Crates: graft-ui, graft.

## Read first
- `docs/adr/0043-resolver-file-list-and-three-panes-when-wide.md` §1.
- `docs/design-notes/m6-conflicts.md`, Amendment 2: Journeys J4, J8, J9 and "Where each event lands him", Interfaces (the `fix-resolver-file-list` blocks), New UI "Below 180 columns" (the strip, the many-files grid, the states).
- `crates/graft-ui/MAP.md` `conflict_resolver` (`DocView`, `layout`, `row_at`, `MIN_COLUMNS`/`MIN_ROWS`); `crates/graft/MAP.md` §`update::resolver` (`ResolverState`, `open_resolver`, `resolved_advance`, `resolver_mouse`, `banner_rows`, `fits`), §`update` `Command`/`Product`, §`view` (the resolver's draw site).
- **Scout questions:**
  1. How `resolved_advance` picks the next file today.
  2. How `resolver_mouse` maps a screen row through `row_at` (what offset it subtracts).
  3. Where the resolver's `Rect` is split under the header.
  4. Whether `OpenConflict` re-reads the working-tree status before opening.

## Scope
In:
1. **The set.**
   - On `open_resolver`, and again after every `Stage`/`ResolveWholeFile` answer, issue `Command::ReadConflictSet(JobId)`. It runs `graft_ops::conflict::operation_conflicts`.
   - The answer is `Product::ConflictSet` → `ResolverState.files`. Until it lands, the list shows the unmerged paths from `AppState::working_tree`.
   - `file_cursor` stays on the same path across re-reads.
2. **Row states.** Each file shows one state:
   - `Open { open, of }`: from the document when it is the open file. Other unmerged text files show `…` (`Unread`) until visited, then their last-known counts.
   - `AllTaken`: every block is taken but the file is still unmerged. Drawn `⏎ marks done`.
   - `Done`: the file is no longer unmerged. Drawn `done ✓`.
   - `WholeFile { how }`: a modify/delete-style shape. Drawn `keep or delete`; the full `how` words go on the file identity row.
   - `Binary`.
3. **Placement.** `Column` when `arrangement(frame_columns)` is `ThreePane` (≥180), else `Strip`. The strip is:
   - one row when every `name · state` fits;
   - otherwise, first a compact pass (`1/2 open`, `✓`);
   - failing that, a grid of equal cells, row-major, at most 3 rows, keeping the cursor's row visible, with `… N more` in the last cell.

   Below `SIDE_BY_SIDE_MIN` the strip is always compact. The document area shrinks by the strip's rows plus one rule row, and `resolver::fits` counts them. Below the floor, the existing too-small line stands.
4. **File identity row.** The document's first row replaces the old title:
   - a text file: `▸ <path> · <n> conflicts · <k> open`;
   - a whole-file shape: `▸ <path> · <how words with SideNames>`, e.g. `modified on yours · main, deleted on theirs · feature`.
5. **Selection.** A click on a list cell → `select_file(index)`: `OpenConflict` for that path, cursor block 0, `top` 0.
   - While a take, stage or base job is in flight, the click is refused with today's `still writing the last take`.
   - `Done` files open read-only, like today's reopened file (`0 open`); `d` starts one over, as today.
6. **Landing after a file is done.**
   - `resolved_advance` goes to the next file in **list order** that is not `Done`, wrapping, with the notice `<file> done ✓ · ▸ <next>`.
   - None left → today's close to the banner, with the notice `every file done ✓ · c finishes the merge` for a merge. For the others it is `… · c continues the rebase` (or `the cherry-pick`, `the revert`).
   - A stash pop has no banner; the pane gets `every file done ✓`.
7. **`FileList` widget.** It draws only inside its `Rect` and puts `▸` and the `▏` gutter on the cursor row. In ASCII it uses `>` and `|`.

Out:
- `j`/`k` (P7) and `O`/`T` (P6).
- The three panes. The column is placed here but drawn beside the card until `fix-resolver-three-panes` lands, so a 200-column capture from this ticket shows column + card.
- The summary.

## Signatures
```rust
// graft-ui/src/conflict_resolver/files.rs (new; re-exported from conflict_resolver)
pub struct FileRow { pub name: SanitizedText, pub state: FileState }
pub enum FileState { Open { open: usize, of: usize }, AllTaken, Done, WholeFile { how: SanitizedText }, Binary, Unread }
pub struct FileListView { pub rows: Vec<FileRow>, pub cursor: usize }
pub enum Placement { Column { width: u16 }, Strip { rows: u16 } }
pub fn placement(view: &FileListView, frame_columns: u16) -> Placement;
pub fn file_at(view: &FileListView, placement: Placement, width: u16, dx: u16, dy: u16) -> Option<usize>;
pub struct FileList<'a> { pub view: &'a FileListView, pub placement: Placement, pub theme: &'a Theme, pub ascii: bool }
pub struct DocView { /* existing */ pub files: FileListView }
// graft
pub struct ResolverState { /* existing */ pub files: Loading<Arc<ConflictSet>>, pub file_cursor: usize }
pub fn select_file(state: &mut AppState, commands: &mut Vec<Command>, index: usize, next_id: /* the resolver's existing type */) -> bool;
pub enum Command { /* existing */ ReadConflictSet(JobId) }
pub enum Product { /* existing */ ConflictSet(Result<Arc<ConflictSet>, String>) }
```
`ARCHITECTURE.md`: `ui` never imports `ops`. `ConflictSet` becomes `FileListView` in `view::resolver`, the one sanitising site.

## Acceptance (name the verifying test when you tick)
- [ ] graft-ui `conflict_resolver::files::tests::placement_by_width`:
  - `Column` at 180 and 200; `Strip` at 179, 160, 120 and 80.
  - A one-row strip for 2 files at 120.
  - A grid of at most 3 rows, ending `… N more`, for 12 files at 120 and at 80.
- [ ] graft-ui `…::file_at_matches_what_is_drawn`: render into a buffer and read each cell's name back at the offset `file_at` returns, for both placements.
- [ ] graft `update::resolver::tests::a_done_file_stays_listed`: two-file merge, `o` on `gone.txt` → the list reads `gone.txt done ✓`, `▸ notes.txt …`.
- [ ] graft `…::the_list_survives_a_relaunch`: the same after `Relaunch` in the walk harness (`recorded` from git).
- [ ] graft `…::a_click_on_a_file_opens_it` and `…::a_click_mid_take_is_refused_in_words`.
- [ ] graft `…::after_the_last_file_the_banner_says_c_finishes` (merge) and `…::after_the_last_file_a_rebase_says_c_continues`.
- [ ] Captures `docs/shots/m6/fix-file-list-{120x40,160x48,80x24}.txt` (two-file merge, after `o` on `gone.txt`) and `…-many-120x40.txt` (12 files). **What right looks like:**
  - At 120, the second row reads `files   gone.txt · done ✓    ▸ notes.txt · 2 of 2 open`. A rule follows, then `▸ notes.txt · 2 conflicts · 2 open`.
  - At 80, the compact strip reads `✓ gone.txt   ▸ notes.txt 2/2 open`.
  - Many: 3 grid rows, the last cell `… N more`.
  - The user judges by eye.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Medium: one widget + placement, one job wire-up, a resolver state change, ~9 tests, 4 captures. One context. If it will not fit, split at the seam: the graft-ui widget first, then the app.
