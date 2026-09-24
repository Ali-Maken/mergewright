# Scout 14: resolver three panes (main 75ee76c)

## What exists
- `layout(doc, columns)` `graft-ui/src/conflict_resolver/layout.rs:89`; one arm per region; `block_row` gets the `Header` row index (:123). Open ≥`SIDE_BY_SIDE_MIN`(100) → `Labels`+`Pair`, else stacked `Side`; taken → `Side` lines only (:138–202). `row_at` = `rows.get(top+r)` (:243).
- `ScreenRow` variants: `Plain, Header, Labels, Pair, Side, End, WholeFile, Note` (layout.rs:16–71). There is no `PaneTitles`/`Aligned`/`Cell`/`LineRef` yet.
- `THREE_PANE_MIN=180`, `Arrangement`, `arrangement()` `conflict_resolver.rs:63–82`. Only callers: `placement` (files.rs:133) and a test (tests.rs:712). **Nothing in `graft` calls `arrangement`.**
- `Placement::Column{width}` = `clamp(longest,22,32)` (files.rs:532–542). `resolver_areas`: list, then one `│` column, then doc.
- Draw: `ResolverScreen::render` splits with `resolver_areas`, 2 header rows, then `draw_row` per `row_at` (conflict_resolver.rs:275–305). `GUTTER=2` (:91). `content_area` strips the gutter (draw.rs:88). `draw_gutter` puts `cursor_mark` (`▏`) at `area.x` for the cursor block (draw.rs:96–108). Only `draw_header` reverses (draw.rs:115–128, 185). `draw_row` match: draw.rs:129–180.
- Line numbers: `BlockNumbers{ours,theirs,result}` names.rs:67; `number_width` names.rs:76; `Projection` graft-git conflict.rs:197; `first_lines` document.rs:118.

## Callers of `layout(doc, columns)` (Q1)
- `graft/src/view/frame/conflicts.rs:59` (draw; placement on `document.width` :56).
- `graft/src/view/resolver.rs:159` `resolver_layout`, used by `update/resolver/geometry.rs:70` `body` → `max_top`, `scrolled_to`, `click`.
- `graft-ui/tests/resolver_layout.rs:129`; `conflict_resolver/tests.rs` :126 :174 :217 :232 :251 :306 :343 :379 :490 :599 :616 :630 :663 :701.

## Goldens (Q3)
- `graft-ui/tests/resolver_layout.rs:164–173` renders 120×40 and 80×24 into `tests/shots/resolver-{120,80}.txt` via `graft_fixtures::capture` (capture.rs:145). A rewrite happens only when an env-driven `updating` flag is set (capture.rs:163–169). Could not confirm the variable name.

## App: mouse and scroll
- `resolver_mouse` scroll.rs:27; wheel → `scroll_by` (one `top` shared by all rows). `click` hit-tests `split.doc`, body row = `row - doc.y - HEADER_ROWS` (:86), then `row_at` → `block_of` (:99–108). **`block_of` is an exhaustive match**, so new variants must be added to it.
- Geometry: `areas`→`view::resolver::resolver_split` uses `state.size.columns` (view/resolver.rs:223). The draw uses `document.width`.
- Keys: `scroll_key` claims j/k/↓/↑/PgUp/PgDn/g/G/Home/End (scroll.rs:113–135). 14 adds no keys.

## Contradictions
1. **Plain per-pane numbers.** Scope 2 says "numbers per projection… can differ between panes". `RegionView::Plain` carries only `first_line` "in the result projection" (conflict_resolver.rs:141–145). `DocView` carries no ours/theirs start for plain regions.
2. **Pane width at 180.** ADR-0043 §2 and acceptance say 46. Code gives 180 − column(22..32) − 1 `│` = 147..157, and then three per-pane gutters. The ADR does not state that arithmetic. Could not confirm that 46 follows from the code.
3. **j/k in the keymap versus the code.** `02-keymap.md` §2.6 says j/k = next/previous file (P7). The code scrolls (scroll.rs:113–114). This predates 14 and is out of 14's scope, but it contradicts the keymap doc.
4. **Taken-block row count versus today.** Scope 5 says `scroll_block_into_view` works "unchanged". It uses `block_row` = the `Header` index (geometry.rs:15–24). The ThreePane rule row must fill `block_row`, or scroll breaks.
