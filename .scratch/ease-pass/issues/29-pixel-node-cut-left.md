# fix round: pixel nodes cut flat on the left — a lane's centre moves to the middle of its two cells

Status: ready
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
**D1. Fix the centre, not the size.** A lane's centre is the boundary between its two cells: `(lane * CELLS_PER_LANE) * cell_width + (CELLS_PER_LANE * cell_width) / 2`, which is `(2·lane + 1) · cell_width`. `node.rs`'s size rule is unchanged: it already allows one cell each side, so with the move every node style lies inside its own lane, `gap` short of both lane edges.
- Rejected: **shrink the node to fit 0.5 cell.** At 9×20 the plain dot falls from ~11.6 px to ~5 px across, and the halo from 16 px to 7 px. That is a visible loss on the tier he uses in Ghostty, and it leaves the lane's right cell empty.
- Rejected: **a half-cell left margin** (an image one cell wider, or a Kitty `X=` pixel offset). A wider image breaks `CELLS_PER_LANE` parity with the Unicode tier, so graph width and the lane cap (ADR-0022) would differ by tier. iTerm2 has no pixel offset.
- **Unicode and ASCII tiers: unchanged.** Their node glyph stays in the lane's first cell, and the lane is still two cells, so every column and text start is the same. The pixel node now sits half a cell right of where the Unicode glyph sits. That only matters if a pixel-tier frame also draws Unicode graph glyphs; Touches checks for that.
- **Edges and arcs:** every lane x moves by the same +0.5 cell, so arcs between lanes and the lane-to-lane spacing (2 cells) are unchanged. The gap between adjacent nodes stays `2·gap`, as today.
- **Last lane / right edge:** the centre is 1 cell from the image's right edge, and reach ≤ `cell_w − gap`, so it fits.

**D2. One place.** Every raster site that computes a lane's x calls one function (signature below). The builder greps `raster/` and the app's graphics code for every lane-x computation (`CELLS_PER_LANE`, `0.5`, `cell_width` products) and lists each in NOTES, with its verdict: now calls it, or not a lane x.

**D3. Stubs (`▾`/`▴`) and the rail.** Read where pixel stubs and the rail are drawn.
- Rule: a stub lies wholly inside its own lane's two cells and does not touch the node's punched disc.
- If the uniform shift keeps that, keep it.
- If not (e.g. a stub placed one cell right of the centre would now sit on the next lane's edge), place it in its lane's corner toward its direction (`Below` → bottom-right, `Above` → top-right) at its current size.
- If no corner holds it at some listed cell size, **stop and report `blocked`**, giving the numbers.
- The rail draws at its column's centre via the same function.

**D4. Size change only if pixels demand it.** If a style's drawn pixels still cross a lane edge after D1 (the anti-alias fringe, or a stroke's half-width past `reach`), subtract that overhang inside `node.rs`'s `room`. It is the one size change allowed; NOTES gives before/after radius at 9×20. Vertical has the same rule.

## Scope
In: crate `mergewright-graph` (`raster/pen.rs`, `raster/node.rs` only if D4 applies, the stub draw if D3 applies, and tests). Crate `mergewright`: any lane-x site D2 finds in graphics code, the shot tests, and re-shot captures. MAP.md lines for both.
Out: the Unicode and ASCII renderers; node styles and colours; `Ring::of`; lane cap and fold; the root glyph question (below, NOTES only).

## Signatures
```rust
// crates/mergewright-graph/src/raster/pen.rs (crate-private; numeric types as pen.rs:88 uses today)
/// x of a lane column's centre in strip pixels: the boundary between its two cells,
/// so a node sized by `node.rs` (one cell of room each side) stays inside its lane.
pub(crate) fn lane_centre_x(column: usize, cell_width: f32) -> f32;
```
If the app crate computes a lane x itself, D2 makes it call a `pub` re-export of this function instead (MAP line added). Don't copy the formula.

## Acceptance (tick each with its test, inline)
graph, fast tier:
- [ ] **Every node style is whole and inside its lane.** Rasterise one-row strips at cell sizes 9×20, 8×16 (the fallback), 10×22, 17×37, 24×52 and narrow-tall 6×24 and 7×28. For each `Ring` style (`Commit`, `Merge`, `Root`, `Head`, `Stash`, `Uncommitted`) at lane 0, lane 1, the last column of a 3-lane strip, and on the rail column:
  - every pixel with alpha > 0 that belongs to the node lies in `[lane_left + 1, next_lane_left − 1]` in x and inside the row's cell band in y;
  - its left and right margins to the lane edges differ by ≤ 1 px, and so do its top and bottom margins to the band. A flat cut breaks the symmetry, so it fails here.
  - Node pixels are isolated: draw the row with no edges, or subtract the same row drawn without the node.
  - On failure, name the style, cell size and lane. <- `raster::node::tests::every_node_style_lies_whole_inside_its_lane`
- [ ] **Red first:** the box above fails on main before D1. Record the failing style, size and lane in NOTES. <- same test, run before the fix
- [ ] Emphasis path: a `PieceKind::Node` piece from `rasterise_piece` for the same styles and sizes holds the node whole: same bounds, measured in strip coordinates through the piece's crop. <- `raster::node::tests::a_node_piece_holds_its_node_whole`
- [ ] Stubs (D3): at the same cell sizes, each stub's pixels lie inside its lane and do not touch the node's punched disc. <- `raster::tests::a_stub_lies_inside_its_lane_clear_of_the_node` (place it where the stub tests live)
- [ ] Lane geometry: `lane_centre_x(k, w) == (2k + 1) · w` for k in 0..4 and the widths above. An arc between lanes 0 and 2 draws the same pixels shifted by `w / 2` as before (or: its endpoints sit on both lane centres). <- `raster::pen::tests::a_lane_centres_on_the_boundary_of_its_two_cells`

app, fast + walk tier:
- [ ] Unicode and ASCII captures unchanged: no `.txt` under `docs/shots/` changes bytes in the full nextest + `scripts/walks.sh`. <- the full runs, named in NOTES
- [ ] **Bytes that change, listed.** Every `docs/shots/` PNG and every pixel golden or hash test whose bytes change (expected: the `ghostty_shot` PNG, the `pieces_shot` PNG, `docs/shots/22-raster-*.png` if a test writes them, any raster golden in `mergewright-graph`). For each: path, old → new size, why. Historic "before" files (e.g. `docs/shots/ease-pass/20-pinned-row-before.png`) are **not** re-shot; list them as left. <- the shot tests by name, in NOTES
- [ ] **For his eye:** the Ghostty shot re-taken (`ghostty_shot`, path in NOTES, ≤ 200 KB). Also a new `docs/shots/ease-pass/29-nodes-{before,after}.png` (≤ 200 KB each): every `Ring` style at lanes 0 and 1, 9×20 cells, scaled ×4 nearest-neighbour. "Before" comes from the new test's capture run before D1; "after" from after it. What right looks like: every node a whole circle or ring; the left side as round as the right; a clear gap between the node and the image's left edge. Colour unverified (rule 5). <- `raster::node::tests::capture_ease_29_nodes` (or wherever shots are written), plus `ghostty_shot` by name

## Read first
- This ticket, then the scout `.scratch/ease-pass/scouts/29q-node-clipping.md`.
- `crates/mergewright-graph/MAP.md` § raster; `crates/mergewright/MAP.md` § event_loop graphics only.
- Source: `raster/pen.rs:80-180`, `raster/node.rs` (whole, small), `raster.rs:55-80` and `:280-295`, the stub draw, `event_loop/graphics/tall.rs`, `tests/it/ghostty_shot.rs`.

## Touches (check each, say how)
- **Every lane-x site** (D2): the grep list in NOTES. Covers edges, arcs, tails, stubs, rail, nodes and the continuation band (`rasterise_continuation`), which all move together.
- **Crops:** tall row `make_room` and `layers.rs` crop by whole cell rows (scout :6), unaffected in x. Check the iTerm2 band crop (`iterm2.rs:205-215`) and the Kitty piece crop to whole cells touched: a lane ≥ 1 node now touches only its own two cells, so piece widths and keys change. Say so.
- **Kitty images that outlive a run:** piece keys hash anchor-relative geometry (ADR-0053). Find out whether that geometry is in pixels or in lane units. If lane units, a terminal still holding a piece from a pre-fix run could be asked for the old pixels by an equal key. Show with file:line that images are deleted at start or exit, or that the key changes; if neither, **stop and report** before working around it.
- **Strip cache:** keys carry cell size (§3.8), with no persistence across runs. Confirm.
- **Pixel-tier frames that draw Unicode graph glyphs** (a pending upload, the pinned uncommitted row, a lane name in a column): any found means the node shifts half a cell when the image lands. List with file:line; don't change.
- **Mouse / hit tests** on the graph: confirm they map by cell or lane, not by the pixel centre (grep, file:line).
- **Perf:** one formula, no new pass. No run needed; say so.
- **No change:** Unicode/ASCII renderers, `mergewright-ui`, keymap, journal/undo, `-ops`, `-git`, `-term` (except a lane-x site D2 finds there, which then calls the re-export).
- **Docs:** graph MAP `CELLS_PER_LANE` line gains "a lane's centre is the boundary between its two cells (`lane_centre_x`)".

## Question (NOTES only, not to fix)
`raster/node.rs:18` documents the root as "a hollow ring (`◇`)". The pixel tier draws a stroked circle; he sees `○`. The scout found no design text that defines the root node's glyph (scout :14). This pass couldn't confirm that, because search tools were down. In NOTES, record the glyph the Unicode tier's root `NodeKind` draws, with file:line. The conductor then asks: is `◇` in the comment stale, or should the tiers' root marks match? Don't edit the comment.

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`; then `scripts/walks.sh` (this changes what the app draws).

## Size
~40k tokens: one function and its callers, one possible stub move, four tests, a capture test, re-shots.

## Planner's note to the conductor
Search tools (rg) were down this pass, so Glob and Grep failed. I could not list `issues/` (28's file name unknown; format taken from 26), grep DESIGN for a pixel node-placement rule (none found in 03 §3.x or 04, read by hand), or list `docs/shots/`. The shot list above is from the scout and the log, so the builder's NOTES list is the real one.

## NOTES (builder, 2026-10-24) — BLOCKED on D3, partial

Red seen on main: `every_node_style_lies_whole_inside_its_lane` -> "Commit at 9x20 lane 0: x 0..=10 leaves lane 0..18 with a pixel to spare".

Done (committed): `lane_centre_x` in `raster/pen.rs` (Pen::x calls it; the only lane-x formula, other CELLS_PER_LANE sites are cell-range crops, not lane x); D4 in `raster/node.rs` (room reserves `gap.max(1.0)`; head halo never thinner than 1 px hairline, else the fringe hit the lane edge pixel at 8x16, 9x20). Tests green: `node::extent_tests::{every_node_style_lies_whole_inside_its_lane, a_node_piece_holds_its_node_whole}`, `pen::tests::a_lane_centres_on_the_boundary_of_its_two_cells`.

BLOCKED (D3 stop condition): the pixel "stub" is the folded-edge tail (`pen.rs` `tail`), which runs from node edge toward the next lane's centre. It already left its lane on main (pixel x=18 at 9x20, lane edge 18). Max tail x vs lane right edge, after D1 (both Below/Above): 6x24 15/12, 7x28 18/14, 8x16 21/16, 9x20 24/18, 10x22 27/20, 17x37 48/34, 24x52 68/48. Room between node edge and lane edge is w - clearance ~ 0.3w - gap (9x20: 18 - 15.7 = 2.3 px), so no corner holds the tail at its current size. The stub test (strict: tail alpha>0 pixels x < 2w) was removed from the commit because it cannot pass. Needs a conductor decision: shorten tail to fit, or relax the rule to "tail stays left of the next lane's centre" (ADR-0022).

Not done: integration tests/snapshots in `tests/it` (raster*.rs, raster_continuation) fail because pixels moved +0.5 cell; no re-shots, no capture test, no app crate checks, no gates beyond fmt/clippy on graph (clean) and graph lib tests (167 pass).
