# fix-resolver-three-panes: at 180 columns and wider, yours | result | theirs, rows level by git's own regions, the chosen side landing in the middle

Status: ready (filed 2026-09-23; P1–P8 approved by the user, P9 by the conductor — see docs/field-notes.md)
Blocked by: 10-resolver-line-numbers.md, 13-resolver-file-list.md
Was: draft · new 2026-09-23 (Amendment 2, ADR-0043 §2, §5). Not gated.
**Revised 2026-09-24 (spec-only pass on `scout-13.md`, ruling R1 in `13-resolver-file-list.md`).** `THREE_PANE_MIN`, `Arrangement` and `arrangement()` are built by `fix-resolver-file-list`; this ticket uses them and does not redefine them. The document's `Rect` comes from 13's `resolver_areas`.
**Revised again 2026-09-24 (spec-only pass on `scout-14.md`, main `75ee76c`).** Rulings S1–S5 below; each cites the scout line it answers.
Build with: **Opus** — scout done (`scout-14.md`). A second arrangement through the existing `layout` → draw → hit-test chain; every rule is fixed here and in ADR-0043, and judged by golden files, counted widths and captures. No structural choice left open (S1–S5 close them).
Needs: `fix-resolver-line-numbers` (`Projection`, `first_lines`, `BlockNumbers`, `SideNames`); `fix-resolver-file-list` (`THREE_PANE_MIN`, `Arrangement`, `arrangement`, `Placement::Column`, `resolver_areas`, `view::resolver::resolver_split`). Both merged.
Crates: graft-ui (`conflict_resolver`), graft (`view::resolver`, `view::frame::conflicts`, `update::resolver::{geometry, scroll}`).

## Rulings (scout-14)
- **S1 — plain-region numbers per pane** (scout Contradiction 1; `RegionView::Plain` holds only the Result `first_line`, conflict_resolver.rs:141–145). `RegionView::Plain` swaps `first_line: usize` for `numbers: BlockNumbers`, filled in `view::resolver` from the same `first_lines(Ours | Theirs | Result)[region]` vectors already computed for blocks (MAP: "each region is numbered by `ConflictFile::first_lines` per projection"). The card reads `numbers.result`, so the goldens stay byte-identical. `BlockNumbers` keeps its name; its doc comment becomes "this region's first line in each projection".
- **S2 — pane widths** (Contradiction 2; `Placement::Column` = `clamp(longest, 22, 32)`, files.rs:532–542, then one `│`). "46 at 180" does not follow from 13's column. The rule is: `doc` minus two 1-column `│` separators, split in three, the remainder going to the result pane. Each pane's width **includes** its own gutter. At 180 frame columns: doc = 147..=157, panes 48/49/48 (column 32) up to 51/53/51 (column 22). **Floor at 180 = 48.** The acceptance asserts this rule and the floor, not 46.
- **S3 — `j`/`k`** (Contradiction 3; `scroll_key` claims them at scroll.rs:113–114, and `02-keymap.md` §2.6 gives them to files, P7). Owned by **`17-resolver-file-keys`** (its scope 2). This ticket does not touch `scroll_key`. Its tests move by `scroll_by`, `scroll_block_into_view` or `state.cursor`, never by `j`/`k`.
- **S4 — the rule row is the block's row** (Contradiction 4; `block_row` is filled from the `Header` index at layout.rs:123; `scroll_block_into_view` reads it, geometry.rs:15–24; `block_of` is an exhaustive match, scroll.rs:99–108). In ThreePane, `block_row[i]` = the index of block `i`'s rule row. `block_of` gains `PaneTitles → None` and `Aligned { block, .. } → block`. In scope, tested (below).
- **S5 — one width source** (scout "Geometry": the mouse splits on `state.size.columns` via `resolver_split`, view/resolver.rs:223; the draw measures `document.width`, view/frame/conflicts.rs:56). `arrangement()`'s `frame_columns` is **`state.size.columns`**, and it is read once, in `resolver_split`. The arrangement is derived from the placement that split returns (`Placement::arrangement`), never measured again. The draw at conflicts.rs:56 takes its placement and areas from `resolver_split` and stops measuring `document.width`. `layout`'s `columns` = `areas.doc.width`. If the builder finds the resolver's `document` rect narrower than `state.size.columns`, **stop and report it as a question** (rule 7); do not pick one.

## Read first
- `.scratch/m6-conflicts/scout-14.md` (whole, one page).
- `docs/adr/0043-resolver-file-list-and-three-panes-when-wide.md` §2, §5, Consequences (projections, not stage files).
- `docs/design-notes/m6-conflicts.md`, Amendment 2: Interfaces (the `fix-resolver-line-numbers` and `fix-resolver-three-panes` blocks, :498–534), New UI "≥180 columns" (:627–648).
- `crates/graft-ui/MAP.md` `conflict_resolver` (`layout`, `ScreenRow`, `row_at`, `ResolverScreen`, `arrangement`, `resolver_areas`, `Placement`, goldens `tests/shots/resolver-{120,80}.txt`); `crates/graft/MAP.md` §`update::resolver` (`scroll_block_into_view`, `resolver_mouse`, `geometry`) and §`view` `resolver` (`resolver_split`, `resolver_layout`).
- Prototype v2 frames at 200x50: `docs/shots/m6/prototype-journeys.txt` :165–:230 and :583–:640 (the frames, not `walk.py`).
- Golden regeneration: `graft-fixtures` capture.rs:163–169 (the scout could not confirm the env variable's name; read it there).

## Scope
In:
1. `layout(doc, arrangement, columns)`. Every caller in the scout's Q1 list passes it: `view/frame/conflicts.rs:59`, `view/resolver.rs:159` (`resolver_layout`), `graft-ui/tests/resolver_layout.rs:129`, and the `conflict_resolver/tests.rs` sites. `Card` returns exactly today's rows, and the existing goldens stay byte-identical.
2. **ThreePane rows**: one `ScreenRow::PaneTitles` (`yours · main │ result │ theirs · feature`, from `DocView::names`), then per region:
   - `Plain`: one `Aligned { block: None, .. }` row per line, the same `LineRef::Plain` in all three cells. Numbered from `numbers.ours` / `numbers.result` / `numbers.theirs` (S1).
   - **Open block**: one rule row (`Aligned { block: Some(i), ours: Cell::Rule, result: Cell::Rule, theirs: Cell::Rule }`), then `max(ours, theirs, 1)` rows. Yours/theirs draw their lines numbered from `numbers.ours` / `numbers.theirs`, padded with `Pad`. The result pane shows `Placeholder` on the first row (`? open · o t b`) and `Pad` below it. With `base_shown` it shows `LineRef::Base` lines, unnumbered, the first headed `base`; the block then takes `max(ours, theirs, base + 1)` rows.
   - **Taken block**: one rule row, then `max(ours, theirs, taken)` rows. Result = `LineRef::Taken` numbered from `numbers.result`; the sides unchanged.
   - Rule texts: open → `── o take` / `── <i> of <n> · open` / `── t take`. Taken → `── taken ✓` or `── not taken` per side, `── <i> of <n> · took <role · name>`. Both → both sides `taken ✓`.
   - Whole-file shapes: the sides show the file or `(deleted on <name>)`; the result shows `? keep or delete · o t` until resolved.
3. **`block_row` and `block_of` (S4).** `Layout::block_row[i]` = block `i`'s rule-row index in ThreePane (the `Header` index in Card, unchanged). `update::resolver::scroll::block_of` gains `PaneTitles → None`, `Aligned { block, .. } → block`.
4. **Widths (S2).** `three_panes(doc)` splits `doc` into yours, result and theirs rects, with a 1-column `│` between them and the remainder to the result. Each pane has its own gutter: the mark column, the number sized to that pane's widest number, one space. Lines longer than a pane's text width are cut with `…` (gap row → M7: horizontal scroll).
5. **Cursor block.** `▏` on every row of it, in the yours pane's mark column. The rule row reversed across all three panes (colour, **unverified**, rule 5).
6. **One width source (S5).** `Placement::arrangement`. `resolver_split` stays the one split. The draw (`view/frame/conflicts.rs`) and the hit-test (`update::resolver::geometry`) both take `placement.arrangement()` and `areas.doc.width` from it.
7. **App.**
   - `scroll_block_into_view` and `row_at` keep working on the row list (through S4).
   - A click in any pane (or on a separator) on a block's rows makes it the cursor block; the wheel scrolls all three together. Pick by row only: the three panes share rows.
   - Resize across 180 keeps the cursor block in view.

Out:
- `j`/`k` and every other key (S3; `17-resolver-file-keys` owns `j`/`k`). `scroll_key` is not edited.
- The card arrangement's rules (`fix-resolver-line-numbers`); horizontal scroll (M7); `THREE_PANE_MIN`, `Arrangement`, `arrangement()`, `placement()` and `resolver_areas` themselves (13).
- Pinning `PaneTitles` while scrolling. It is a body row and scrolls with the document, like the card's `Labels`.

## Signatures
```rust
// graft-ui/src/conflict_resolver.rs — from fix-resolver-file-list, used unchanged:
//   THREE_PANE_MIN, Arrangement { ThreePane, Card }, arrangement(frame_columns), resolver_areas, Placement
// changed here (S1):
pub enum RegionView { Plain { numbers: BlockNumbers, /* other fields unchanged */ }, /* other variants unchanged */ }  // was first_line: usize
// new here:
impl Placement { pub const fn arrangement(self) -> Arrangement; }   // Column{..} → ThreePane, Strip{..} → Card (S5)
pub fn three_panes(doc: Rect) -> [Rect; 3];                          // [yours, result, theirs]; separators between; remainder → result (S2)
pub enum LineRef { Plain { region: usize, i: usize }, Ours { block: usize, i: usize }, Theirs { block: usize, i: usize }, Taken { block: usize, i: usize }, Base { block: usize, i: usize } }
pub enum Cell { Line { line_no: Option<usize>, text: LineRef }, Pad, Placeholder, Rule }
pub enum ScreenRow { /* existing */ PaneTitles, Aligned { block: Option<usize>, ours: Cell, result: Cell, theirs: Cell } }
pub fn layout(doc: &DocView, arrangement: Arrangement, columns: u16) -> Layout;   // was (doc, columns)

// graft — unchanged signatures, changed callers:
//   view::resolver::resolver_split(state, banner_rows) -> (FileListView, Placement, ResolverAreas)  // the one source (S5)
//   update::resolver::scroll::block_of — new arms (S4)
```

## Acceptance (name the verifying test when you tick)
- [ ] graft-ui `conflict_resolver::tests::card_rows_are_unchanged`: goldens `resolver-{120,80}.txt` are byte-identical through `Arrangement::Card` (S1's `numbers.result` included).
- [ ] graft-ui `…::three_panes_stay_level`: for every fixture file × {nothing taken, block 0 ours, all theirs, block 1 both, block 0 open with `base_shown`}, every region starts on the same screen row in all three panes, and a block spans `max` of its panes' lines + 1.
- [ ] graft-ui `…::each_pane_is_numbered_without_gaps`: the numbers read down each pane, skipping `Pad`/`Placeholder`/`Rule`/`Base`. Yours and theirs run 1..=n for their projection in every state. The result runs 1..=`render()` line count when every block is taken. With a block open, the result's next number after its placeholder = the number before it + that block's ours count + 1 (the `Result` rule, note :503–504; mockup :637–639, `3` → `5`).
- [ ] graft-ui `…::plain_regions_number_each_pane_by_its_projection` (S1): build (or pick) a fixture with a block whose theirs is longer than its ours; after `t` on it, the next plain region's row shows `first_lines(Ours)[region]`, `first_lines(Result)[region]`, `first_lines(Theirs)[region]` in the three panes, and the result's differs from the yours'.
- [ ] graft-ui `…::the_chosen_side_lands_in_the_middle`: after `t` on block 0, the result cells of that block are its theirs lines.
- [ ] graft-ui `…::the_rule_row_is_the_block_row` (S4): in ThreePane, for every block `i`, `rows[block_row[i]]` is `Aligned { block: Some(i), .. Cell::Rule }`.
- [ ] graft-ui `…::panes_split_by_the_rule` (S2): for doc widths 147..=157 and 167..=177, `three_panes` gives two separators, sides equal, result = sides + remainder, sum = width. With `resolver_areas` at 180 frame columns and `Placement::Column { width }` for every width in 22..=32, no pane is narrower than **48** columns, gutter included.
- [ ] graft-ui golden `tests/shots/resolver-three-200.txt` (with a header saying it is generated, not a mockup match).
- [ ] graft `update::resolver::tests::a_click_in_any_pane_picks_the_block`: a click on the rule row in each of the three panes and on a separator sets the same cursor block.
- [ ] graft `…::draw_and_click_agree_across_180` (S5): at 179, 180 and 181 frame columns, the arrangement the draw uses and the one `geometry` uses are equal, and a click on the first body row of block 1 picks block 1.
- [ ] graft `…::next_block_scrolls_three_panes_to_its_rule_row` (S4): with a file longer than the body, `scroll_block_into_view` on the last block puts its rule row on screen.
- [ ] graft `…::resize_across_180_keeps_the_cursor_block_in_view`.
- [ ] Captures `docs/shots/m6/fix-three-panes-200x50.txt` (two-file merge, `notes.txt`, after `t` on block 1) and `…-180x50.txt`. **What right looks like:**
  - The titles row reads `yours · main │ result │ theirs · feature`.
  - Block 1's result cell reads `2 TWO from feature` on the same row as `2 two from main` and `2 TWO from feature`.
  - Block 2's result reads `? open · o t b`. The result pane's first plain line after block 2 is numbered: the result's line before block 2's rule row + block 2's yours line count + 1.
  - The list column on the left shows `▸ notes.txt`.
  - At 180, counted from the capture: the list column + `│` + three panes + two `│` = 180, and the narrowest pane is ≥ 48 columns wide (gutter included).
  - The user judges by eye.

## Proposed doc edits (conductor applies)
- ADR-0043 §2, :44–45: "At 180 columns each pane is 46 characters; at 200 columns, 52." → "Each pane is a third of what the list column and two `│` separators leave, the remainder to the result, its line-number gutter included: 48–53 columns at 180, 55–59 at 200. The list column's clamp (22–32) is the knob the user's eye tunes."
- Design note :426: "(LA §B: 46–52 chars a pane)" → "(LA §B; 48–53 columns a pane at 180, gutter included — ADR-0043 §2)".
- Design note :512: "RegionView::Plain.first_line now = the Result projection's number" → "RegionView::Plain { numbers: BlockNumbers, .. } replaces first_line (fix-resolver-three-panes, S1); the card draws numbers.result".
- Design note :530–534 block: add `impl Placement { pub const fn arrangement(self) -> Arrangement; }` and `pub fn three_panes(doc: Rect) -> [Rect; 3];`.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Medium: one layout arm, one draw path, the `Plain` numbers field, two `block_of` arms, the draw moving onto `resolver_split`, ~11 tests, a golden, 2 captures. One context. If the conflicts.rs:56 move turns out to be more than a call-site swap, commit the layout half (scope 1–5) and report the split.
