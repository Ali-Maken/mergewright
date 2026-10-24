# 29q — are pixel-tier nodes clipped? (scout, read-only, no run)

## 1. Size and placement
- Cell/band: `raster/pen.rs:28` `cell_height = look.cell_height` px; row i spans `top..top+cell_height` (`pen.rs:175-179`), node centre `mid = top + cell_height/2` (`:178`). Node is in the commit row's own cell row.
- Radius: `raster/node.rs:56-64` `room = min(cell_h/2, cell_w) - gap`; `radius = max(room/1.4, 1.0)`; head halo reaches 1.4 radii. Punched disc = `reach + gap` (`node.rs:97-100`) <= cell_h/2 by construction, except the `.max(1.0)` floor on a tiny cell (not checked). So by this code a sprite cannot exceed its own cell row vertically.
- Tall row: `event_loop/graphics/tall.rs:32-51` `make_room` splits the cursor's strip: part 1 = strip rows `..=cursor` at `at`; part 2 = rows after, at `line+2`; band (continuation, `:49`) at `line+1`. Crop is by whole cell rows (`rows: Range<usize>`, `layers.rs:128,142`; iTerm2 `iterm2.rs:205-215` crops by `cell_height` multiples). The cursor row is the LAST row of part 1, so the node is a whole cell row, cut on cell boundaries only. Merge node sits in the first of the two screen lines (subject line), not the second.
- Pane top edge: could not confirm any code that offsets/crops the first graph row. Graph area = `graph_area` (`graph_screen/layout.rs:92-100`), body rows. For a 3-row history the merge is line 0 of the body; nothing found that crops it.
- Could not confirm: cell_height px vs the terminal's real cell height (probe/rounding); if they differ by a pixel, Kitty/Ghostty scale the image, but I did not trace it. Not runnable here.

## 2. Test/capture that a node is whole
None found. `raster/node/tests.rs` has only `a_merge_whose_side_is_not_drawn_is_still_a_merge` (:23) and `one_parent_is_a_commit_and_none_a_root` (:33): style choice, no pixel bounds. `view/node/tests.rs:212` compares ring vs Unicode kind per row, not extents. Captures `docs/shots/22-raster-*.png` (rings-on-dark etc.) are eyeball artefacts; no assertion on them found. No grep hit for "clip/crop/cut" tied to nodes in `raster/` or `view/`.

## 3. Root hollow
`Ring::of` (`node.rs:32-42`): no parents -> `Ring::Root`, stroked ring, no fill (`:126-134`); doc comment `node.rs:18` says "A hollow ring: nothing came before it (`◇`)". Merge = ring + dot. Note the doc names `◇`, not `○`. Design: grep of `docs/design/01-view-mockups.md`, `DESIGN.md`, `docs/design/*.md` for hollow/○/◇/root-node meaning found only `01-view-mockups.md:44` (`◇` = practice repo header, unrelated) and layout doc lane-ending (03:132,177). No design text defining the root's hollow node found: could not confirm it is specified.

## 4. Clipping "intended"
None found in code comments or docs/design. Comments say the opposite: "stays inside its band" (`node.rs:55`).

## Contradiction
Root glyph in code is documented as `◇` (hollow ring); user sees `○`. Whether the Unicode tier's root is `○` or `◇`: not checked here.

## 5. Horizontal (coordinator follow-up): FOUND, by arithmetic from the code, not run
- Lane x centre = `(column*CELLS_PER_LANE + 0.5) * cell_width` (`raster/pen.rs:86-89`); `CELLS_PER_LANE = 2` (`raster.rs:60`). Column 0 centre is half a cell from the image's left edge (x=0).
- Node radius `room/1.4`, `room = min(cell_h/2, cell_w) - gap` (`node.rs:56-64`), gap = 0.6*stroke, stroke = clamp(cell_w/8, 1.5, 2.5) (`raster.rs:72-79`). Any cell with h/2 >= w (normal ~1:2 cells) gives room ~ w - gap, radius ~0.7*w - so the node reaches ~0.7w left of a centre only 0.5w from the edge.
- Example cell 9x20 px: stroke 1.5, gap 0.9, room 8.1, radius 5.79 vs 4.5 available: ~1.3 px cut flat on the left of every column-0 node (outer ring included; punched eraser radius 6.69 reaches 2.2 px past). Right side has the lane's second cell, so only the left is cropped; top/bottom fit (room <= h/2). Fits his picture: all three nodes cut flat on the left.
- The vertical bound (`node.rs:55` "stays inside its band") was the only bound written; the comment promises "short of the next lane" (`:54-55`) but nothing bounds the left by the centre's 0.5w. Node-in-lane x extent is untested (see 2).
- Not confirmed: his actual cell px; the image's left edge being exactly the lane's x=0 (`local()` `pen.rs:162-175` translates by `left - origin`, column 0 left = 0, origin 0 for a whole strip); Kitty placement offset (not traced).

### Horizontal addendum (follow-up asks)
- Section 5 above is the horizontal answer (centre 0.5 cell from left edge, `pen.rs:86-89`; radius up to ~0.7 cell_w, `node.rs:56-64`). Image width = lane columns * CELLS_PER_LANE cells (`raster.rs:288`; `tall.rs` Band `columns`), so no extra left margin exists to absorb the overhang.
- cell_width source: `Capabilities.cell_size` with a `CellSizeSource` (`session.rs:198,348-351`, update on resize `session.rs:274-279`); `CellSizeSource` variants (Reported vs fallback) not read. Whether it equals Ghostty's true pixel cell: could not confirm. If it is wider than real, the cut would differ; not traced.
- Kitty placement x offset / crop: not traced; `kitty.rs` is not at `mergewright-term/src/kitty.rs` (path in `tall.rs` imports is `mergewright_term::kitty`, location not found by my grep). Could not confirm no x offset applied.
- Test that a node's x extent fits its image: none found.
