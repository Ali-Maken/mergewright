# Scout 21 findings: two drawn-frame observations

## A. `notes.txt` cut to `…tes.txt` in the 27-cell list column

- The column width is set by `column_width`, `crates/graft-ui/src/conflict_resolver/files/column.rs:19-29`. It takes the **widest single row**: `3 + name + 2 + state` for each row (`:24`), plus the header, clamped to 22..=32 (`:28`).
  - `gone.txt` row: 3+8+2+14 (`keep or delete`) = **27**.
  - `notes.txt` row: 3+9+2+1 (`…`) = 15.
  - Header `FILES · 2 conflicted` = 20.
  - Result: 27.
- The column is placed at that width: `files.rs:134`, split at `files.rs:177-186`.
- `column_plan` (`column.rs:40-54`) lays out the rows differently. It aligns states, so it takes the **longest name** (9, `notes.txt`, `:48-53`) and the **longest state** (14, `:47`) **from different rows**. It then sets `name_w = min(9, 27 - 5 - 14) = 8` (`:54`).
- `fit_left` (`files.rs:330`) then cuts `notes.txt` to 8 cells: `…tes.txt`.
- So 8 is not taken from `gone.txt`'s width. It is `27 - 3 - 2 - 14`. The two functions add up widths in different ways (per row vs. per column), so the width chosen is 1 cell short of the plan.
- On purpose? No comment says this cut is intended. The `column_width` doc (`:18`) says "its longest row". The plan's doc (`:39`) says "states aligned". The two rules do not agree once states are aligned.
- Tests that cover a mixed-length name and state: *could not confirm* (I did not search `files/tests.rs`).

## B. `begun outside Graft` dropped at 187 columns

- The wording ladder is `stop_wordings`, `crates/graft/src/update/banner/stop.rs:197-215`. Rung 2 drops the origin (`:260` vs `:261`). It reaches the UI through `view/rows.rs:343-352` as `StopWording`.
- The footer picks a wording in `stopped_row`, `crates/graft-ui/src/graph_screen/hints/fitting.rs:224-252`:
  - The room is `columns - bar - 3` (`:229-231`).
  - It takes the first rung that fits (`:235-238`).
- `columns` is `width16 = area.width` (`graph_screen/footer.rs:87,104`). Here `area` is the **graph screen's rectangle**, not the frame's (`graph_screen/render.rs:26,68`).
- With the detail pane open on the Right, that rectangle is `frame width - pane columns - gutter` (`crates/graft-ui/src/detail_pane/layout.rs:96-111`, called from `crates/graft/src/view/frame/layout.rs:46-68`).
- A fresh launch on a live operation "open[s] the changes pane on the work" (`crates/graft/src/update/relaunch.rs:32-33`). So the footer's budget is the graph side of the split, not the 187-column frame.
- Arithmetic (not observed): the full wording is about 61 cells, the bar about 60, plus 3. That is about 124 cells, which fits 187 but not a graph side much narrower than that.
- *Could not confirm*:
  - The pane's actual column count at 187x50.
  - That the pane was on the Right, not Below or Tab, in that capture.
  - The code does not state anywhere whether the footer should size to the graph side on purpose. The `stopped_row` doc (`fitting.rs:211-222`) only accepts the reason dropping "beside a laddered row bar".
