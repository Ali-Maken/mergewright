# fix round: pixel nodes cut flat on the left — a lane's centre moves to the middle of its two cells

Status: reopened by D5 (planner, 2026-10-24): D1 reverted, the cut fixed by a lead cell. Boxes D5 touches are un-ticked below.
Blocked by: none
Build with: Sonnet (`builder-sonnet`, model sonnet); fails twice → `builder-opus`. Why: the cause is found and read at its lines (conductor), the fix is one formula moved into one function, and a pixel-extent test judges it, so correctness does not rest on the diff or on an eye. No Fable test is met: nothing structural, no new interface.
Needs: none.

From KNOWN_GAPS `ease/29` (broken), scout `.scratch/ease-pass/scouts/29q-node-clipping.md`.

## Need
User's Ghostty screenshot, dark theme, pixel tier, 2026-10-24: every graph node (head halo, plain `●`, root ring) is cut flat on its **left**. Top and bottom are whole. A cut node reads as a different shape, and different shapes mean different things to him (memory "different things must look different").

## Evidence (rule 7)
- Lane centre x = `(lane * CELLS_PER_LANE + 0.5) * cell_width`: `crates/mergewright-graph/src/raster/pen.rs:88` (conductor read it; scout :23). `CELLS_PER_LANE = 2` (`raster.rs:60`). So lane 0's centre is **0.5 cell** from the image's left edge and **1.5 cells** from its lane's right edge.
- Node size: `room = min(cell_h / 2, cell_w) - gap`, `radius = max(room / 1.4, 1.0)`; head halo reaches `room`, punched disc `room + gap`: `raster/node.rs:54-64`, `:97-100` (conductor read it). Horizontal room is **one full cell** each side of the centre. The size assumes a centred node; the centre is not centred.
- At 9×20 (stroke 1.5, gap 0.9): halo reach 8.1 px, plain radius 5.79 px, punched 9.0 px, against 4.5 px left of the centre. That cuts 1.3 px off a plain node and 3.6 px off the halo (scout :25).
- The image has no left margin to absorb this: width = lanes × `CELLS_PER_LANE` cells (`raster.rs:288`, scout :30).
- Lanes ≥ 1 overhang into the previous lane's second cell rather than past the image edge. Whether any crop cuts them there is not confirmed. The test below covers every lane position.
- No test bounds a node's x extent (scout :11, :33).

## Decision
**D1. Fix the centre, not the size.** *Superseded by D5 (2026-10-24): the centre goes back to main's; read D5 first. Kept for the record.* A lane's centre is the boundary between its two cells: `(lane * CELLS_PER_LANE) * cell_width + (CELLS_PER_LANE * cell_width) / 2`, which is `(2·lane + 1) · cell_width`. `node.rs`'s size rule is unchanged: it already allows one cell each side, so with the move every node style lies inside its own lane, `gap` short of both lane edges.
- Rejected: **shrink the node to fit 0.5 cell.** At 9×20 the plain dot falls from ~11.6 px to ~5 px across, and the halo from 16 px to 7 px. That is a visible loss on the tier he uses in Ghostty, and it leaves the lane's right cell empty.
- Rejected: **a half-cell left margin** (an image one cell wider, or a Kitty `X=` pixel offset). A wider image breaks `CELLS_PER_LANE` parity with the Unicode tier, so graph width and the lane cap (ADR-0022) would differ by tier. iTerm2 has no pixel offset. *(D5 overturns this rejection: the extra cell is the existing gutter, so graph width and the lane cap do not change.)*
- **Unicode and ASCII tiers: unchanged.** Their node glyph stays in the lane's first cell, and the lane is still two cells, so every column and text start is the same. The pixel node now sits half a cell right of where the Unicode glyph sits. That only matters if a pixel-tier frame also draws Unicode graph glyphs; Touches checks for that. *(It does: Ghost Preview. Hence D5.)*
- **Edges and arcs:** every lane x moves by the same +0.5 cell, so arcs between lanes and the lane-to-lane spacing (2 cells) are unchanged. The gap between adjacent nodes stays `2·gap`, as today.
- **Last lane / right edge:** the centre is 1 cell from the image's right edge, and reach ≤ `cell_w − gap`, so it fits.

**D2. One place.** Every raster site that computes a lane's x calls one function (signature below). The builder greps `raster/` and the app's graphics code for every lane-x computation (`CELLS_PER_LANE`, `0.5`, `cell_width` products) and lists each in NOTES, with its verdict: now calls it, or not a lane x.

**D3. Stubs (the folded-edge tail, `pen.rs` `tail`) and the rail.** *Ruled 2026-10-24 (planner), after the builder's stop; replaces the earlier "inside the lane, clear of the punched disc, at current size" rule. D5 amends its lane bounds; the shape rule stands.*
- **What a stub means** (DESIGN `03-graph-layout-algorithm.md` §3.5, ADR-0022): an edge into a folded or dropped column, marked beside its node — `Below` = the branch continues below, `Above` = it forked here. Nothing crosses a kept lane to reach the fold; the folded thread stays legible by its colour. In the Unicode tier it is `▾`/`▴` in the **gap cell right of the node**, inside the node's own lane.
- **Rule: the stub is a corner tail, in its own lane, attached to its node.** A straight stroke on the ray from the node's centre to its lane's corner on the stub's side (`Below` → bottom-right, `Above` → top-right). It starts at the node's outer rim (its reach: halo for `Head`, ring/dot otherwise), crossing the punched gap so it reads as leaving this node, and ends where its stroke (cap and anti-alias included) is still inside `[lane_left + 1, next_lane_left − 1]` in x and inside the row's cell band in y. Length = whatever that leaves; colour and stroke width unchanged. **Same rule in every column**: lane 0, inner lanes, the last drawn column, the rail, a one-column strip. Its x extent (≈ `centre + 0.4w` to `2w − 1` from lane left) is the Unicode gap cell's, so both tiers put the mark in the same cell.
- **Why not the two options asked:**
  - *Shorten the side tail to fit the lane:* 2.3 px at 9×20 (builder's numbers), i.e. invisible, and the thread is lost against §3.5.
  - *Relax to "left of the next lane's centre":* there is no next lane on the **last drawn column**, the **rail** (folded nodes draw on it, §3.5) or a **one-column strip** (06b's `◉▾ … ●▴`, `lanes drawn: 1 of 2`). There the room right of the punched disc is `w − min(h/2, w)`, which is **0 at every listed size** (all have `w ≤ h/2`), so the tail is cut flat at the image edge — the defect this ticket fixes, moved to the stub. It would also draw two-thirds of a cell into the next lane, a cell right of the Unicode `▾`, and share that lane's left cell with its arcs.
  - It never overlaps a next-lane **node** in either form (one node per row); the relaxed form could overlap that lane's arcs on the row, the corner form cannot.
- Rejected: **shrink the node on stub rows** (two node sizes = two meanings); **shrink every node** (D1's rejection stands).
- **Weak spot, stated:** the corner is smallest for the head halo at small cells — about 1.8 px past the halo at 8×16 (the fallback), about 3 px at 9×20; plain/merge about 3.8 px at 8×16 and 5 px at 9×20; ≥ 7 px at 17×37 and up. NOTES gives the measured table. If he finds it too small in the PNG, the next lever is node size (a new decision, not this ticket).
- **Out:** a stub sharing its corner with a kept arc from the same node (an octopus merge's second kept parent, or a second fork) — precedence unchanged; NOTES say whether the test fixtures hit it and what draws on top.
- **Capture:** `29-nodes-after.png` adds one row of stubs: `Commit` and `Head` with `▾` and `▴`, at lane 0 of a one-column strip and on the rail of a 3-column strip.
- The rail draws at its column's centre via the same function.

**D4. Size change only if pixels demand it.** If a style's drawn pixels still cross a lane edge after D1 (the anti-alias fringe, or a stroke's half-width past `reach`), subtract that overhang inside `node.rs`'s `room`. It is the one size change allowed; NOTES gives before/after radius at 9×20. Vertical has the same rule. *(D5: keep the builder's D4 change; the node box is the same width.)*

**D5. Revert D1; fix the cut with a lead cell taken from the gutter.** *Ruled 2026-10-24 (planner), on the builder's correction (NOTES, last paragraph).*
- **Why D1 cannot stand** (read at the lines): on main the pixel centre was `(2c + 0.5)·w` (`pen.rs:88` on main, read this pass), the centre of the lane's own cell, which is where the Unicode node glyph sits (`mergewright-ui/src/graph_column/mod.rs:11-12`: "its own (`2 × column`), where the lane's vertical line or node sits, and a gap to its right"). At the pixel tier an open Ghost Preview draws the whole graph column as Unicode text (ADR-0060; `view/tests/pixel_preview.rs:1-3`, `:65-111`). With D1, every node and lane line moves half a cell sideways when a preview opens or closes. A text glyph cannot sit on a cell boundary, so only the pixel tier can move to match.
- **Why not accept the jump:** the preview exists so he can see what the operation changes (UX 2 #5, M5's promise "you see the result before you confirm"). If every node shifts half a cell when it opens, the whole graph looks changed, and the change he is meant to spot is hidden in the noise. The same swap happens on fallback after a graphics error and on `F3`, so the jump would show up there too (UX 2 #4, consistency).
- **Why not keep pixels in the preview:** that undoes ADR-0060, which is a whole feature, not a fix round.
- **Rule.**
  1. **The lane centre in graph coordinates is main's again:** `(2c + 0.5)·w` from the graph column's left edge, the Unicode node cell's centre. Edges, arcs, the rail and the continuation band are drawn exactly as on main, relative to the graph column.
  2. **Every strip image starts one cell left of the graph column**, in the gutter's gap cell (`GUTTER` = cursor slot + gap, `graph_screen/mod.rs:142-144`; the body is inset by it, `carve.rs:176-183`, `layout.rs:296-302`; "the gutter and the margin are blank cells beside the rows", `layout.rs:122`). The image is `STRIP_LEAD_CELLS + lanes × CELLS_PER_LANE` cells wide. Its right edge does not move. Graph width, `Columns`, the lane cap and hit-tests do not change: the lead cell is space the layout already gives up.
  3. **So, in strip pixels:** `lane_centre_x(c, w) = (STRIP_LEAD_CELLS + 2c + 0.5)·w = (2c + 1.5)·w`. Lane 0 gets 1.5 cells of room on its left and the last lane gets 1.5 on its right. Node reach is ≤ `w − gap`, so no node is cut on either side.
  4. **A lane's box** (the node bounds in the extent test and the stub bounds in D3) is the two cells centred on its centre: strip x `[centre − w, centre + w]`, i.e. `[(LEAD + 2c − 0.5)·w, (LEAD + 2c + 1.5)·w]`. The boxes tile with no gap and no overlap. D3's `lane_left` / `next_lane_left` mean this box's edges. The stub's geometry relative to its node is unchanged, so the builder's length table should still hold (re-measure it). The stub now ends at about the centre of the Unicode gap cell, where `▾`/`▴` is drawn, which replaces D3's "x extent is the Unicode gap cell's".
  5. **One constant, one function.** Crops, anchors and widths read `STRIP_LEAD_CELLS`. Nobody writes `- 1` or `+ 1` by hand. `pieces/extent.rs:73`'s `.max(0)` clamp stays, but lane 0 never hits it any more (`LEAD + 2c − 1 ≥ 0`).
  6. **No gutter, no strip.** Below 4 columns the body has no gutter (`layout.rs:297`). Strips are not placed there; the column draws as Unicode text, the same path the preview takes. Do not place an image whose lead cell falls outside the graph screen's area.
  7. **Same placement for Kitty and iTerm2.** It is a whole cell, so no Kitty `X=` offset is needed and iTerm2's lack of one doesn't matter.
- **Stop conditions (report, don't work around):** (a) any row, whether a cursor row, its second line, the pinned uncommitted row, the tall row or a banner/tour/pending band beside the body, draws text or a background in the gutter's gap cell that differs from what the strip's backdrop paints there; (b) any screen other than the graph screen places strips whose left neighbour is not a blank gutter cell.
- **Rejected again: shrink the node** (D1's rejection stands); **move the Unicode glyph** (a glyph can't sit on a cell boundary).

## Scope
In: crate `mergewright-graph` (`raster/pen.rs`, `raster/node.rs` only if D4 applies, the stub draw (D3), and tests). Crate `mergewright`: any lane-x site D2 finds in graphics code, the shot tests, and re-shot captures. MAP.md lines for both.
D5 adds: graph `raster.rs` (strip width, `STRIP_LEAD_CELLS`), `pen.rs` `local()` (lane box), `pen/continuation.rs` width, `emphasis/cells.rs`, `emphasis/pieces.rs`, `emphasis/pieces/extent.rs` (crop cells shift by the lead); app `event_loop/graphics/layers.rs` (anchor and crop), `graphics/tall.rs` (band width), the iTerm2 band crop and the Kitty piece crop (`mergewright-term` only if the crop arithmetic lives there; then it reads the re-export).
Out: the Unicode and ASCII renderers; node styles and colours; `Ring::of`; lane cap and fold; the root glyph question (below, NOTES only); `Columns`, `GUTTER`, graph width, ADR-0060.

## Signatures
```rust
// crates/mergewright-graph/src/raster/pen.rs (crate-private; numeric types as pen.rs:88 uses today)
/// x of a lane column's centre in strip pixels: the centre of the lane's own
/// cell — where the Unicode tier draws the node glyph — counted from the
/// strip's left edge, which is STRIP_LEAD_CELLS cells left of the graph column (D5).
pub(crate) fn lane_centre_x(column: usize, cell_width: f32) -> f32; // (STRIP_LEAD_CELLS + 2·column)·w + w/2

// crates/mergewright-graph/src/raster.rs (pub, re-exported where CELLS_PER_LANE is; MAP line)
/// Cells a strip image carries left of the graph column: the gutter's gap cell
/// (ADR-0058 §4), so lane 0's node has a full cell of room on its left.
pub const STRIP_LEAD_CELLS: u32 = 1; // same integer type as raster's CELLS_PER_LANE
```
If the app crate computes a lane x itself, D2 makes it call a `pub` re-export of this function instead (MAP line added). Don't copy the formula. Every anchor, crop and width that offsets by the lead reads `STRIP_LEAD_CELLS`.

## Acceptance (tick each with its test, inline)
graph, fast tier:
- [ ] **Every node style is whole and inside its lane.** *(D5: the lane is D5.4's box; re-tick.)* Rasterise one-row strips at cell sizes 9×20, 8×16 (the fallback), 10×22, 17×37, 24×52 and narrow-tall 6×24 and 7×28. For each `Ring` style (`Commit`, `Merge`, `Root`, `Head`, `Stash`, `Uncommitted`) at lane 0, lane 1, the last column of a 3-lane strip, and on the rail column:
  - every pixel with alpha > 0 that belongs to the node lies in `[lane_left + 1, next_lane_left − 1]` in x and inside the row's cell band in y;
  - its left and right margins to the lane edges differ by ≤ 1 px, and so do its top and bottom margins to the band. A flat cut breaks the symmetry, so it fails here.
  - Node pixels are isolated: draw the row with no edges, or subtract the same row drawn without the node.
  - On failure, name the style, cell size and lane. <- `raster::node::extent_tests::every_node_style_lies_whole_inside_its_lane` (test lives in `node/extent_tests.rs`)
- [x] **Red first:** the box above fails on main before D1. Record the failing style, size and lane in NOTES. <- same test, run before the fix (red seen, see NOTES)
- [ ] Emphasis path: a `PieceKind::Node` piece from `rasterise_piece` for the same styles and sizes holds the node whole: same bounds, measured in strip coordinates through the piece's crop. *(D5: lane 0's piece now starts in the lead cell; re-tick.)* <- `raster::node::extent_tests::a_node_piece_holds_its_node_whole`
- [ ] **Stubs (D3) are corner tails inside their lane, attached to their node.** *(D5: bounds are D5.4's box; re-tick, table re-measured.)* At every cell size above, for each `Ring` style and each `Toward`, at lane 0 of a one-column strip, lane 1 and the last column of a 3-column strip, and on the rail column. Stub pixels (alpha > 0, isolated as in the node box: the row with and without the folded edge, subtracted):
  - all lie in `[lane_left + 1, next_lane_left − 1]` in x and inside the row's cell band in y — none in the next lane, none past the image edge;
  - all lie right of `lane_centre_x` and on the direction's side of the row's middle (`Below` below it, `Above` above it);
  - none lies more than 1 px inside the node's reach (no overdraw of the node); the nearest lies within the node's punched gap (attached, not floating);
  - the farthest lies at least one stroke width (≥ 1.5 px) past the node's reach.
  - On failure, name style, cell size, lane and direction. NOTES: a table of stub length past reach, style × size. <- `raster::tests::a_stub_leaves_its_node_into_its_lane_corner` (where the stub tests live)
- [ ] ~~Lane geometry: `lane_centre_x(k, w) == (2k + 1) · w`~~ *Replaced by D5's lane-geometry box below.* <- `raster::pen::tests::a_lane_centres_on_the_boundary_of_its_two_cells` (rename or delete)

graph, fast tier — D5:
- [ ] **The pixel node sits on the Unicode node cell.** For k in 0..4 and every width above: `lane_centre_x(k, w) − STRIP_LEAD_CELLS·w == (2k + 0.5)·w`, the centre of graph-column cell `2k` (the Unicode node cell, `graph_column/mod.rs:11`). Strip width in cells `== STRIP_LEAD_CELLS + lanes × CELLS_PER_LANE` for 1..4 lanes. An arc between lanes 0 and 2 draws, relative to the graph column, the same pixels as on main (shifted by exactly `STRIP_LEAD_CELLS·w` in strip pixels), or: its endpoints sit on both lane centres. <- `raster::pen::tests::a_lane_centres_on_its_unicode_node_cell`
- [ ] **The lead cell holds no ink but lane 0's node.** At every size above, strip pixels in `[0, STRIP_LEAD_CELLS·w − 0.5w)` carry alpha 0 on rows with no lane-0 node, and only node pixels on rows with one (no edge, arc, rail or stub reaches left of lane 0's box). <- `raster::node::extent_tests::the_lead_cell_holds_only_lane_zeros_node`

app, fast + walk tier:
- [ ] Unicode and ASCII captures unchanged: no `.txt` under `docs/shots/` changes bytes in the full nextest + `scripts/walks.sh`. *(D5: re-run.)* <- the full runs, named in NOTES
- [ ] **Placement starts in the gutter's gap cell.** At the pixel tier (Kitty and iTerm2), every strip, the pinned row, the tall row and every emphasis piece is placed with its left column at `graph_area.x − STRIP_LEAD_CELLS` (the piece at its lane's cell offset from there), and its right edge at `graph_area.right()` as before. Assert on the emitted bytes or the placement records, per protocol. <- `event_loop::graphics::tests::a_strip_starts_in_the_gutter_gap_cell` (builder names the exact path)
- [ ] **The cell under the lead is blank.** At the pixel tier on a 100×30 fixture with the cursor row, its second line, the pinned uncommitted row and a tall row in view: the text layer's cell at `graph_area.x − 1` is a space on every body row, and its background equals the backdrop the strip paints there on that row (stop condition (a) otherwise). <- `view::tests::pixel_preview::the_gutter_gap_cell_under_a_strip_is_blank`
- [ ] **A preview doesn't move the nodes sideways.** Same fixture, preview closed then open (`pixel_preview.rs` `previewing_at`): for each real row drawn in both frames, the closed frame's pixel node centre in screen pixels (placement column × w + `lane_centre_x`) equals the open frame's Unicode node glyph cell centre (`(glyph column + 0.5) × w`) for every cell width above. Same check for the fallback frame after a graphics failure and for `F3` to Unicode. <- `view::tests::pixel_preview::preview_nodes_sit_where_the_pixel_nodes_were`
- [ ] **Boxes cover the lead cell too.** A confirm or preview box whose left edge snaps to the frame edge (`layout.rs:128`) leaves no image pixel in any cell it covers, the lead cell included (crop or clear, per `layers.rs`'s rule today). <- the existing layers crop test extended, named in NOTES
- [ ] **No gutter, no strip.** At 3 columns the pixel tier places no image and the column draws as text. <- `view::tests::pixel_preview::no_strip_without_a_gutter`
- [ ] **Bytes that change, listed.** Every `docs/shots/` PNG and every pixel golden or hash test whose bytes change (expected: the `ghostty_shot` PNG, the `pieces_shot` PNG, `docs/shots/22-raster-*.png` if a test writes them, any raster golden in `mergewright-graph`). For each: path, old → new size, why. Historic "before" files (e.g. `docs/shots/ease-pass/20-pinned-row-before.png`) are **not** re-shot; list them as left. *(D5: re-list against main, not against D1's build.)* <- the shot tests by name, in NOTES
- [ ] **For his eye:** the Ghostty shot re-taken (`ghostty_shot`, path in NOTES, ≤ 200 KB). Also a new `docs/shots/ease-pass/29-nodes-{before,after}.png` (≤ 200 KB each): every `Ring` style at lanes 0 and 1, 9×20 cells, scaled ×4 nearest-neighbour (after: plus D3's stub row). "Before" comes from the new test's capture run before D1; "after" from after it. *(D5: "after" includes the lead cell, drawn with a 1 px cell-grid tick at the graph column's left edge so he can see lane 0's node centred on its own cell, half in the gutter.)* What right looks like: every node a whole circle or ring; the left side as round as the right; lane 0's node centred over the graph column's first cell, its left half reaching into the gutter cell with a clear gap before the image's left edge; each stub a short line leaving its node toward the lane's lower-right (`▾`) or upper-right (`▴`) corner, ending inside the lane, never cut at the image edge. Field check (not a test, stays unverified until he does it): in Ghostty, open and close a rebase preview, and the nodes should only change shape, never shift sideways. Colour unverified (rule 5). <- `node_shots::capture_ease_29_nodes`, plus `ghostty_shot` by name

## Read first
- This ticket, then the scout `.scratch/ease-pass/scouts/29q-node-clipping.md`.
- `crates/mergewright-graph/MAP.md` § raster; `crates/mergewright/MAP.md` § event_loop graphics only.
- Source: `raster/pen.rs:80-180`, `raster/node.rs` (whole, small), `raster.rs:55-80` and `:280-295`, the stub draw (`pen.rs` `tail`), `event_loop/graphics/tall.rs`, `tests/it/ghostty_shot.rs`.
- D5 adds: `mergewright-ui/src/graph_column/mod.rs:9-22` (cell geometry), `graph_screen/mod.rs:142-147` (`GUTTER`), `graph_screen/carve.rs:173-183`, `graph_screen/layout.rs:92-132`, `:293-308`, `raster/emphasis/pieces/extent.rs:63-76`, `event_loop/graphics/layers.rs` (anchor, crop), `view/tests/pixel_preview.rs`.

## Touches (check each, say how)
- **Every lane-x site** (D2): the grep list in NOTES. Covers edges, arcs, tails, stubs, rail, nodes and the continuation band (`rasterise_continuation`), which all move together.
- **Stub shape** (D3): piece keys hash each element's geometry (MAP § raster `Piece`), so stub-row `Node` pieces get new keys; say so. Any test or golden that pinned the old tail's pixels is listed under "Bytes that change".
- **Crops:** tall row `make_room` and `layers.rs` crop by whole cell rows (scout :6), unaffected in x. Check the iTerm2 band crop (`iterm2.rs:205-215`) and the Kitty piece crop to whole cells touched: a lane ≥ 1 node now touches only its own two cells, so piece widths and keys change. Say so.
- **Kitty images that outlive a run:** piece keys hash anchor-relative geometry (ADR-0053). Find out whether that geometry is in pixels or in lane units. If lane units, a terminal still holding a piece from a pre-fix run could be asked for the old pixels by an equal key. Show with file:line that images are deleted at start or exit, or that the key changes; if neither, **stop and report** before working around it.
- **Strip cache:** keys carry cell size (§3.8), with no persistence across runs. Confirm.
- **Pixel-tier frames that draw Unicode graph glyphs** (a pending upload, the pinned uncommitted row, a lane name in a column): any found means the node shifts half a cell when the image lands. List with file:line; don't change. *(D5: now asserted aligned by the preview box; a pending upload or any other such frame found gets the same check.)*
- **Mouse / hit tests** on the graph: confirm they map by cell or lane, not by the pixel centre (grep, file:line). *(D5: a click in the gutter cell still hits what it hit before; images take no clicks.)*
- **Perf:** one formula, no new pass. No run needed; say so. *(D5: strips one cell wider, ≈ +1/(2L) pixels; no run needed.)*
- **No change:** Unicode/ASCII renderers, `mergewright-ui`, keymap, journal/undo, `-ops`, `-git`, `-term` (except a lane-x site D2 finds there, which then calls the re-export).
- **D5:** the gutter's gap cell on every row kind (stop condition (a)); boxes and overlays that cover it (ADR-0040 snap to the frame edge); the branch pane and the detail pane layouts (`Right`/`Below`/`Tab`), where the graph's left neighbour must still be the gutter; ADR-0058 §4 gets no edit, but the MAP line for `STRIP_LEAD_CELLS` cites it.
- **Docs:** graph MAP `CELLS_PER_LANE` line gains "a lane's centre is its own cell's centre, where the Unicode node glyph sits (`lane_centre_x`); strips start `STRIP_LEAD_CELLS` left of the graph column, in the gutter"; MAP `Stub` line gains "pixel tier: a corner tail from the node's rim, inside its lane"; app MAP graphics line on placement says the anchor is `graph_area.x − STRIP_LEAD_CELLS`.

## Question (NOTES only, not to fix)
`raster/node.rs:18` documents the root as "a hollow ring (`◇`)". The pixel tier draws a stroked circle; he sees `○`. The scout found no design text that defines the root node's glyph (scout :14). This pass couldn't confirm that, because search tools were down. In NOTES, record the glyph the Unicode tier's root `NodeKind` draws, with file:line. The conductor then asks: is `◇` in the comment stale, or should the tiers' root marks match? Don't edit the comment.

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`; then `scripts/walks.sh` (this changes what the app draws).

## Size
~40k tokens: one function and its callers, one stub reshape, four tests, a capture test, re-shots. D5 adds ~30k: one constant, about eight anchor/crop/width sites, five tests, re-shots. If the resumed context can't hold it, hand off after the graph half (D5.1–5.5 and the graph boxes), and the app half becomes 29b.

## Planner's note to the conductor
Search tools (rg) were down this pass, so Glob and Grep failed. I could not list `issues/` (28's file name unknown; format taken from 26), grep DESIGN for a pixel node-placement rule (none found in 03 §3.x or 04, read by hand), or list `docs/shots/`. The shot list above is from the scout and the log, so the builder's NOTES list is the real one.

D3 ruling (2026-10-24): rg still down; read DESIGN 03 §3.5 and the ease-pass note by hand, no source. The corner lengths in D3's "weak spot" are my arithmetic from the node formula above (far end inset ~1.25 px for cap + margin), not measured; the builder's table is the real one.

D5 ruling (2026-10-24): rg still down; read by hand at the lines cited in D5 (main's `pen.rs:88`, `graph_column/mod.rs:9-22`, `pixel_preview.rs`, `graph_screen/{mod,carve,layout,row_columns}.rs`, `pieces/extent.rs:63-76`). **Not verified:** that the gutter's gap cell is blank on every row kind. The comments say so (`layout.rs:122`, `mod.rs:142-144`), but I didn't trace the cursor row's background paint. That is why stop condition (a) and the blank-cell box exist. I also didn't read `layers.rs`, `tall.rs` or `iterm2.rs`; their sites come from the builder's D2 list.

## NOTES (builder, 2026-10-24) — done

Did not compact. Scout 29q used (conductor's). Red seen on main, before D1: `every_node_style_lies_whole_inside_its_lane` -> `Commit at 9x20 lane 0: x 0..=10 leaves lane 0..18 with a pixel to spare`. The stub test was also red first: `a_stub_leaves_its_node_into_its_lane_corner` -> `Commit Below at 9x20 lane 1 of 3: pixel (36,9) outside lane 18..36`.

Boxes and tests (all in `mergewright-graph`): node whole `raster::node::extent_tests::every_node_style_lies_whole_inside_its_lane`; piece `raster::node::extent_tests::a_node_piece_holds_its_node_whole`; stubs `raster::tests::a_stub_leaves_its_node_into_its_lane_corner` (4 strip cases x 7 sizes x 6 styles x 2 directions); lane geometry `raster::pen::tests::a_lane_centres_on_the_boundary_of_its_two_cells` (the arc-shift half of that box is covered by the unchanged-relative-geometry argument below and by `a_fork_is_an_antialiased_curve_not_an_elbow`, whose x bounds moved by exactly +0.5 cell); eye box `node_shots::capture_ease_29_nodes` (`tests/it/node_shots.rs`) + `ghostty_shot::fix_scroll_ghostty_png`.

Changes: `lane_centre_x` (`raster/pen.rs`), `Pen::x` calls it. D2 grep: other `CELLS_PER_LANE` uses are cell-range arithmetic, not lane x: `raster.rs:288`, `pen.rs` `local()` (lane's left edge), `pen/continuation.rs:160` (width), `emphasis/cells.rs:16`, `emphasis/pieces.rs:241`, `emphasis/pieces/extent.rs:71` (piece crops, left edge +/- whole cells; the node's cells `2c-1 .. 2c+4` still hold a node now spanning `2c .. 2c+2`), app `graphics/layers.rs:36` (crop anchor), `graphics/tall.rs:97` (band width). No lane x in the app crate. Rail: `pen.rs` `rail` uses `Pen::x`; the 1 px rail takes the pixel right of the boundary (x = 3w for lane 1; was centred at 2.5w).
D3: `tail` rewritten: straight stroke on the ray centre -> lane corner, from the node's reach to where stroke + 1 px margin still fits; drawn after the node (the punch disc would clear its start); solid at the lane's alpha, no fade (a 3-6 px tail has nothing to fade over; ADR-0022's fade existed because it ran to the next lane). `NodeSize::reach(ring)` added, `clearance` and `stroke_with` removed (dead).
D4: `node.rs` `room` reserves `max(gap, 1.0)` (was `gap`; at 9x20 gap is 0.9 so room 8.1 -> 8.0, radius 5.79 -> 5.71); the head halo is drawn at least 1 px thick (a thinner stroke is a 1 px hairline in tiny-skia, which put the halo's fringe on the lane-edge pixel at 8x16). At 8x16: gap 0.9, room 7.1 -> 7.0.

Stub length past reach (px, pixel-centre distance, min over the 4 strip cases and both directions):
| style | 6x24 | 7x28 | 8x16 | 9x20 | 10x22 | 17x37 | 24x52 |
| Commit/Merge/Root/Stash/Uncommitted | 6.0 | 7.6 | 4.2 | 5.6 | 6.3 | 11.1 | 16.5 |
| Head | 4.6 | 5.9 | 2.2 | 3.3 | 3.7 | 6.6 | 10.1 |
Smallest is the head at 8x16 (2.2 px), as the ruling predicted.
Octopus/second-fork sharing the corner with a kept arc: not checked by a test; by code order the tail draws last, over any arc (unverified by eye).

Test replaced: `tests/it/raster.rs` `a_folded_edge_is_a_tail_that_fades_out_before_the_next_column` -> `a_folded_edge_is_a_corner_tail_inside_its_lane` (the old fade/length assertions describe the removed shape). Hard-coded x in `raster.rs`, `raster_continuation.rs`, `raster_roles.rs` moved by +0.5 cell (centre = boundary): same checks, new coordinates.

Bytes that change (all regenerated; the sizes are the old -> new):
- Goldens/snapshots: `crates/mergewright/src/view/strips/pinned_row.golden` (1.3k, same length, pixels moved); insta `it__raster__raster-{criss-cross-merge,folded-merge,foxtrot-merge,octopus-merge}.snap`, `it__raster_pixels__raster-pixel-digests.snap`. Why: every lane x +0.5 cell, nodes resized (D4), stub reshaped. The committed `*.snap.new` files from the earlier partial commit are deleted.
- PNGs: `docs/shots/22-raster-criss-cross-merge.png` 7919->7905, `-on-dark` 7731->7892, `22-raster-foxtrot-merge.png` 2112->2207, `-on-dark` 2193->2258, `22-raster-octopus-merge.png` 5127->5332, `-on-dark` 5046->5334, `22-raster-rings-on-dark.png` 3382->3567, `40-raster-folded-on-dark.png` 2034->1766, `42-raster-rings-on-dark.png` 3382->3567, `42-raster-thread-on-dark.png` 6832->7075, `m7/fix-scroll-thread-composite.png` 27889->27354, `m7/fix-scroll-ghostty.png` (ghostty_shot, heavy tier) -> 1.6k, `m8-05-emphasis-pieces.png` (pieces_shot) -> 29k, `ease-pass/20-pinned-row-after.png` (pinned_row_shot; its `-before` left). New: `ease-pass/29-nodes-before.png` 7.2k (taken on b5ae7541 with the same test), `29-nodes-after.png` 6.8k. All under 200 KB. No `.txt` under `docs/shots/` changed (git status; full nextest and walks green after the regenerate). Colour unverified (rule 5): the PNGs are for his eye; I looked at before/after: before, left halves are cut flat; after, all whole.

Touches: Unicode/ASCII renderers untouched (`mergewright-ui` not edited; `git diff --stat` shows none). Piece keys: they hash kind tag, cell size, backdrop and per-row ink/edge data (`emphasis/pieces.rs:174-180`), not pixel geometry, so equal keys now mean new pixels vs an old build. Stop condition cleared: the Kitty upload table is per process (`mergewright-term/src/kitty/mod.rs` `uploads`, starts empty each run, so a fresh run transmits everything and replaces ids from `FIRST_IMAGE_ID`) and `free_all` runs on exit (`mergewright/src/event_loop/graphics.rs:342`). Strip cache keys carry cell size and are in-memory only. iTerm2 band crop and Kitty piece crop go by whole cells; lane >= 1 nodes touch only their own two cells now (extent unchanged: still `2c-1 .. 2c+4`, so no crop change needed, the tail now stays in the lane so the extra cell is slack, not removed). Pixel-tier frame drawing Unicode graph glyphs: Ghost Preview draws the whole graph column at the Unicode tier (ADR-0060, `view/tests/pixel_preview.rs`), so the nodes shift half a cell when a preview opens or closes; not changed. Mouse hit tests map by cell and lane (per `view/hit.rs` header: it reads the drawn cells; I did not trace a pixel-centre use, none found by grep). Perf: one formula, no new pass; tail is one line instead of a gradient path; none measured.

Question (root glyph): the Unicode tier's root `NodeKind::Root` draws `◇` (`crates/mergewright-ui/src/graph_column/kinds.rs:63`). The pixel root is a stroked circle; so the `◇` in `node.rs:18` matches the Unicode tier's glyph name but not the pixel shape. Not edited.

**Correction (Ghost Preview shift): NEW with this fix.** On main the pixel centre was `(2c + 0.5) * w` (`pen.rs:88` at b5ae7541), the centre of the lane's first cell, where the Unicode glyph sits (ticket D1: node glyph in the lane's first cell), so the tiers coincided. Now it is `(2c + 1) * w`: opening or closing a Ghost Preview (ADR-0060, `view/tests/pixel_preview.rs:1-3`) moves nodes half a cell. Not fixed; conductor to decide. Both boxes re-checked at 45bd720c: extent_tests + a_stub_leaves 3/3 pass; full nextest 6820/6820.
