# Scout 06b — lanes on screen and header (main 0538e8c0)

## What exists
- `Fold { kept, folded, columns }` `graft-graph/src/fold.rs:53-61`. `plan` :73-110: `columns <= max_lanes` → keeps `0..columns`, **every** column, even ones open in no row (:75-81). Over cap → `scan` (:288-330): `open` = lane present or touched (:289-299); `required` = col 0 or node's column (:304-308); the rest are candidates ranked other < preferred < reserved < selected. Columns not open are already dropped over the cap (Scan doc :275-278 "dropped, not folded… never counts in +N").
- Accessors: `kept` :127, `drawn/real` :133/:139, `marker` :146, `drawn_columns` :152, `folded_count` :159. No `open_columns()` method; `open_columns` exists only as a local in `scan` (:303).
- Plan callers: graft-ui `Compaction::plan` `graph_screen/mod.rs:366-369`; pixel `view/strips.rs:362` (separate plan, `usize::MAX` when expanded :357-361). `width_through` = `width_for(drawn_columns())` `graph_column/mod.rs:211-213`.
- Applied fold: `layout.rs:290` (`None` while expanded). Header count: `render.rs:85` `folded_count` → `header(width, folded_lanes)` `layout.rs:131` → `tail_items` :196 → `left_out` `layout.rs:349-362` appends `+N lanes`.
- Tail fitting: `header_tail.rs` (module) — `TailItem`, `Drop {Gfx,Walk,Push,Filters,Search}` :19-25, `fitted_tail` :38: last group standing (not Search) is **cut with `…`**, not dropped (:56-57). `tail_room` `header.rs:64`.
- `Filters { first_parent, stash_hidden, focused }` `graph_screen/mod.rs:301-309` (`folded` already gone; `text(self, keys)` :316 — Copy). Filled in `graft/src/view/rows.rs:142-148`.
- Per-row fold: `RowFold.folds` `graft/src/history/fold.rs:28`; on screen as `GraphEntry.fold: Option<FoldMark>` `mod.rs:405`, `FoldMark.open` `fold_mark.rs:24`. `FoldWalk::folds` `inclusion/fold_walk.rs:241`; `OpenedMerges` `inclusion.rs:85`.
- 06r `view/node.rs`: `Node::of(row, refs)` (Head/Stash/Merge/Root/Commit) — node shape only; no fold or lane state. Not touched by 06b.
- Keys: 06b binds none. Lanes expand toggles `lanes_expanded` (strips.rs:357) — not changed.
- Tests: `fold/tests.rs` (kept/folded asserts :86-384, incl. proptest :345-384 asserting kept∪folded = open set); `graph_column/tests.rs:382-522`; `header.rs:295`.

## Contradictions with the spec
1. **Filters fill site:** spec fills `lanes`/`folded_in_view` in `view::rows`; but the fold is planned in graft-ui at render (`mod.rs:368`, `layout.rs:290`) and the drawn window is known only in `body_layout`. view::rows cannot know N, M, or which rows are in view. Today's count already flows render→header as a parameter (`render.rs:85`).
2. **"Never cut mid-word":** `fitted_tail` cuts the last group with `…` (`header_tail.rs:56-57`); dropping whole phrases needs Filters text split into items or a rule change.
3. **Name clash:** proposed `pub fn header_tail` vs existing module `graph_screen::header_tail`.
4. **Under cap:** today keeps even *non-open* columns (:75-81), not just node-less open ones; spec text says "node-less ones included" — true but understates.
5. **Proptest** `fold/tests.rs:355` asserts kept∪folded == open columns; dropping node-less open columns breaks it by design.
6. Frame-test refs: `settle.rs:312` is end of a helper (test starts :315); `pixel_scroll.rs:192` is inside a test starting :191; `pace.rs:220` holds.
7. `strips.rs:362` plans its own fold — strip keys/width change with the drop (spec flags; confirmed).

## Could not confirm
- Fixture numbers `5 of 8 · 3`; HEAD branch naming on the top line (not read).
