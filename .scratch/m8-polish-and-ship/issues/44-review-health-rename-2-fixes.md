# 44 — fixes from review m8-health-rename-2

Status: done
Build with: Opus (conductor dispatch).
Source: `docs/reviews/m8-health-rename-2.md` Majors M2, M3, M4 (M1 is the conductor's, filed separately).

## Crate set

graft-ui: `lib.rs`, `picker.rs`, `preview_box.rs`, `MAP.md`.
graft: `update/{mouse,picker,geometry,banner,tour}.rs`, `update/preview/{rect,tests}.rs`, `view/frame/boxes.rs`, `view/tests.rs`, `tests/it/{map_coverage.rs,over_400.txt}`. `ARCHITECTURE.md` line refs.

## Boxes (each names its test or check when ticked)

- [x] M2 — the preview click test draws the whole frame with the box open, finds `[ Cancel ]` / `[ Confirm ]` in the drawn grid, clicks those cells through `update`, sweep `[40, 80, 120]` kept; red when the box is docked one row off. **Ticked:** `update::preview::tests::a_click_lands_on_the_button_the_frame_drew` — mutation `dock_cells(… height - 1 …)` in `preview/rect.rs:36` → red ("40 columns: a click on Confirm runs it: []"); reverted → green. **Found on the way (cause, fixed):** `preview_click` had no caller outside its tests since m5/08 — `update/mouse.rs::mouse_event` never routed to it, so a real click on `[ Confirm ]` fell through to `left_click` and selected the graph row behind the box. The new test was red without the mutation until it was wired (`mouse.rs` `preview_mouse`, left button only).
- [x] M3 — `crates/graft-ui/MAP.md`: the 19 lines with unbalanced `(` and the wrong-item lines rewritten from source. Wrong-item lines: the review's 5 (`:346` fits, `:375` ALL, `:399` with_overrides, `:451` RowKind, `:505` RowMark) plus 3 more found by checking every name listed under more than one heading against source: `:235` commit_editor `fits` (claimed a "too small" message; source says it still draws), `:491` preview_box `confirm` (described the confirm module; is `Again::confirm`), `:507` rewrite_editor `FieldView` (text_input's fields). **Ticked:** new `map_coverage::map_prose_parentheses_balance` (graft `tests/it/map_coverage.rs`) — red on HEAD's MAP (19 lines), green now; `map_coverage::*` green (160 run).
- [x] M4 — `graft_ui::Cells` is a struct with `x`, `y`, `width`, `height`; `cells_of`/`rect_of` the only conversions (now `pub`, used by `view/frame/boxes.rs`). `dock_cells`, `picker_box`, `picker_box_in` take/return `Cells`; `.3` reads gone (`banner.rs:321` `.pending.height`, `tour.rs:54` `.tour.height`). Behaviour unchanged. **Ticked:** workspace build; `update::tests::carve_geometry::a_click_on_the_first_graph_row_lands_on_the_commit_drawn_there` (pending/tour heights), `a_click_lands_on_the_button_the_frame_drew` (dock), picker mouse tests (`update::picker::tests`) all green in the full run.
- [x] Gates workspace-wide. **Ticked:** fmt clean; clippy `-D warnings` clean; nextest 6311/6311; doctests ok.

## NOTES

Touches:
- `update/mouse.rs` — new `preview_mouse` before the context-menu branch: left-down only; wheel/right-click/motion unchanged (as before, they still reach the graph under an open preview — not changed here). Order: after palette/finish/rewrite/place/resolver/picker modals. Full nextest green.
- `KNOWN_GAPS.md` row `M7/24-click` ("Mouse click on `Put back` over J untested (shared `preview_click`)") — stays unverified; it is now reachable from a real click, but no stash-specific click test was added. Conductor to re-word.
- `view/tests.rs` `draw_cells` made `pub(crate)` (test-only).
- `update/picker.rs` new `picker_bounds_in` (Bounds ↔ Cells) so `geometry::picker_bounds` and `boxes::draw_picker` shrank; over_400 rows lowered for geometry (511), boxes (460), preview/tests (2126); `mouse.rs` 399.
- `ARCHITECTURE.md` `view`-call table line numbers updated (geometry 442/478, mouse 139/152/174), checked by grep.
- MAP: `Cells` line now struct + `cells_of`/`rect_of` lines; `picker_box` line. `map_coverage` green.
- Generator rule (review M3): for later MAP generators, never reuse an old clause whose name sits under more than one old heading — the 8 wrong lines were all that case.

Scout: none — review cites lines; re-read at HEAD. No compaction. `code-review` skill not run (single-context fix ticket; reviewer follows).
