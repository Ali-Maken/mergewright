# fix-resolver-three-panes: at 180 columns and wider, yours | result | theirs, rows level by git's own regions, the chosen side landing in the middle

Status: ready (filed 2026-09-23; P1–P8 approved by the user, P9 by the conductor — see docs/field-notes.md)
Blocked by: 10-resolver-line-numbers.md, 13-resolver-file-list.md
Was: draft · new 2026-09-23 (Amendment 2, ADR-0043 §2, §5). Not gated.
Build with: blank — brownfield (ADR-0035): **scout first**. Expected **Opus**: a second arrangement through the same `layout` → draw → hit-test chain; the rules are fixed in ADR-0043 and judged by golden files and captures.
Needs: `fix-resolver-line-numbers` (`Projection`, `first_lines`, `BlockNumbers`, `SideNames`); `fix-resolver-file-list` (`Placement::Column`, `arrangement` callers).
Crates: graft-ui, graft (`view::resolver`, `update::resolver::{geometry, scroll}`).

## Read first
- `docs/adr/0043-resolver-file-list-and-three-panes-when-wide.md` §2, §5, Consequences (projections, not stage files).
- `docs/design-notes/m6-conflicts.md`, Amendment 2: Interfaces (the `fix-resolver-three-panes` block), Hard choice 1, New UI "≥180 columns".
- `docs/research/conflict-layout-audit.md` §B (pane widths).
- `crates/graft-ui/MAP.md` `conflict_resolver` (`layout`, `ScreenRow`, `row_at`, `ResolverScreen`, the golden files `tests/shots/resolver-{120,80}.txt`); `crates/graft/MAP.md` §`update::resolver` (`scroll_block_into_view`, `resolver_mouse`, `geometry`).
- Prototype v2 frames at 200x50: `docs/shots/m6/prototype-journeys.txt` :165–:230 and :583–:640 (look at the frames, not `walk.py`).
- **Scout questions:**
  1. Every caller of `layout(doc, columns)`.
  2. How `ResolverScreen` draws the gutter and `▏`.
  3. How the golden files are generated and regenerated.

## Scope
In:
1. `arrangement(frame_columns)`: `ThreePane` iff ≥ `THREE_PANE_MIN` (180). `layout(doc, arrangement, columns)`. `Card` returns exactly today's rows, and the existing goldens stay byte-identical.
2. **ThreePane rows**, one `ScreenRow::PaneTitles` (`yours · main │ result │ theirs · feature`), then per region:
   - `Plain`: one `Aligned` row per line, the same `LineRef::Plain` in all three cells. Numbers per projection, so they can differ between panes once a take changes lengths.
   - **Open block**: one rule row (`Aligned { block: Some(i), .. Cell::Rule }`), then `max(ours, theirs, 1)` rows. Yours/theirs draw their lines numbered from `numbers.ours`/`numbers.theirs`, padded with `Pad`. The result pane shows `Placeholder` on the first row (`? open · o t b`) and `Pad` below it. With `base_shown` it shows `LineRef::Base` lines, unnumbered, the first headed `base`.
   - **Taken block**: one rule row, then `max(ours, theirs, taken)` rows. Result = `LineRef::Taken` numbered from `numbers.result`; the sides unchanged.
   - Rule texts: open → `── o take` / `── <i> of <n> · open` / `── t take`. Taken → `── taken ✓` or `── not taken` per side, `── <i> of <n> · took <role · name>`. Both → both sides `taken ✓`.
   - Whole-file shapes: the sides show the file or `(deleted on <name>)`; the result shows `? keep or delete · o t` until resolved.
3. **Widths.** The list column is `Placement::Column { width }`. The rest is split into three equal panes (remainder to the result), each with its own gutter sized to its widest number. Lines longer than a pane are cut with `…` (gap row → M7: horizontal scroll).
4. **Cursor block.** `▏` on every row of it, in the yours pane's gutter. The rule row reversed (colour, **unverified**, rule 5).
5. **App.**
   - `view::resolver` and `update::resolver::{scroll, geometry}` pass the same `Arrangement`.
   - `scroll_block_into_view` and `row_at` keep working on the row list unchanged.
   - A click in any pane on a block's rows makes it the cursor block; the wheel scrolls all three together.
   - Resize across 180 keeps the cursor block in view.

Out: keys (none are new); the card arrangement's rules (`fix-resolver-line-numbers`); horizontal scroll.

## Signatures
```rust
// graft-ui/src/conflict_resolver.rs
pub const THREE_PANE_MIN: u16 = 180;
pub enum Arrangement { ThreePane, Card }
pub fn arrangement(frame_columns: u16) -> Arrangement;
pub enum LineRef { Plain { region: usize, i: usize }, Ours { block: usize, i: usize }, Theirs { block: usize, i: usize }, Taken { block: usize, i: usize }, Base { block: usize, i: usize } }
pub enum Cell { Line { line_no: Option<usize>, text: LineRef }, Pad, Placeholder, Rule }
pub enum ScreenRow { /* existing */ PaneTitles, Aligned { block: Option<usize>, ours: Cell, result: Cell, theirs: Cell } }
pub fn layout(doc: &DocView, arrangement: Arrangement, columns: u16) -> Layout;
```

## Acceptance (name the verifying test when you tick)
- [ ] graft-ui `conflict_resolver::tests::card_rows_are_unchanged`: goldens `resolver-{120,80}.txt` are byte-identical through `Arrangement::Card`.
- [ ] graft-ui `…::three_panes_stay_level`: for every fixture file × {nothing taken, block 0 ours, all theirs, block 1 both}, every region starts on the same screen row in all three panes, and a block spans `max` of its panes' lines + 1.
- [ ] graft-ui `…::each_pane_is_numbered_without_gaps`: the numbers read down each pane, skipping `Pad`/`Placeholder`/`Rule`/`Base`, run 1..=n for its projection. The result with all taken = `render()` line count.
- [ ] graft-ui `…::the_chosen_side_lands_in_the_middle`: after `t` on block 0, the result cells of that block are its theirs lines.
- [ ] graft-ui golden `tests/shots/resolver-three-200.txt` (with a header saying it is generated, not a mockup match).
- [ ] graft `update::resolver::tests::a_click_in_any_pane_picks_the_block` and `…::resize_across_180_keeps_the_cursor_block_in_view`.
- [ ] Captures `docs/shots/m6/fix-three-panes-200x50.txt` (two-file merge, `notes.txt`, after `t` on block 1) and `…-180x50.txt`. **What right looks like:**
  - The titles row reads `yours · main │ result │ theirs · feature`.
  - Block 1's result cell reads `2 TWO from feature` on the same row as `2 two from main` and `2 TWO from feature`.
  - Block 2's result reads `? open · o t b`.
  - The list column on the left shows `▸ notes.txt`.
  - At 180 no pane is narrower than 46 characters.
  - The user judges by eye.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Medium: one layout arm, one draw path, hit-test through the existing chain, ~7 tests, a golden, 2 captures. One context.
