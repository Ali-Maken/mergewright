# Scout: 13 pixel-roles-and-tall-row (at c021e6a6)

## What exists
- **Strip key**: `StripKey{first_row, rows, layout, theme, cell_width, cell_height}` `mergewright-graph/src/raster/cache/key.rs:~14`; `layout = plain_layout_hash` `:123` (lane_columns, marker, per row id/edges/identities/marker/ring/stubs, row above); `theme = theme_id(palette, muted)` (layer_key). Paint key adds `lit_hash` `:146` (backdrop + thread/emphasis). `first_row` is a **history row index**, aligned to `strip_rows` (`aligned()`).
- **On `j`**: layout/theme unchanged → backdrop hits; only `lit_hash` changes if thread/emphasis changes. No role in any key today (grep `LaneRole|roles|look.` in `view/strips.rs`, `event_loop/graphics.rs`: none).
- **`colours_off`**: `Look.colours_off` `mergewright/src/update/state/look.rs:10`. Strip palette from `session.theme.raster_lanes()` `view/strips.rs:49` (session theme, not `Theme::colourless`) — could not confirm the pixel tier sees `colours_off` today. If palette changes, `theme_id` already re-keys.
- **Fold in key**: rows are `fold.view(row)` before hashing (`view/strips.rs:254,398`), `lane_columns = fold.drawn_columns()` `:86`, one fold per frame `plan` `:327`. So a fold change re-keys through `layout`; yes, key depends on the window's Fold indirectly.
- **Placement**: by screen row: `CellRect.row = graph.y + strip.at` `event_loop/graphics/layers.rs:152`; `Drawn.at` = graph row the first shown row lands on `:142`; `rows: Range` crops a strip. Emphasis pieces placed from `piece.rows` within the strip `:170-175`.
- **Strokes**: one stroke width per strip, `stroke_width(cell_width)` `raster/pen.rs:30`; "never varies with emphasis" `:112`. Lanes drawn with `dash: None` (`raster/geometry.rs:167`); tiny-skia dash support exists `:154-160`. Only dotted thing is the fold rail, rect dashes `pen.rs:336-357`. Same-hue lanes: no shape distinction (`raster.rs:26-28`: pixel tier never dashes).
- `LaneRole` `mergewright-graph/src/role.rs:14` (This/Under/Trunk/Other); `lane_roles` app-side `view/entries/roles.rs:16`.

## Mismatches with the ticket
1. Signature `PlainStrip { …, pub roles: Vec<…> }`: `PlainStrip<'a>` is `#[derive(Clone, Copy)]` with borrowed fields (`raster/emphasis.rs:164-165`) — a `Vec` breaks `Copy`; needs a slice.
2. `pub enum Stroke` name clashes with `tiny_skia::Stroke` imported in `raster/geometry.rs:5`.
3. Tall row "strips below placed one row lower": the cursor row sits **inside** a strip (strips are `strip_rows` history rows, `view/strips.rs:279-318`); rows of that same strip below the cursor need a crop+second placement, not only an offset. Placement offset itself is possible (`at` is per strip).
4. Needs `SecondLine`, `commit_rows` (07b): absent (grep no match). `History::visible`/`graph_rows` heights (`strips.rs:215`) do not count a second line yet.
5. Needs `AppState.route`: actually `state.look.route` (`look.rs:15`), focus-matched by `frame_route` (`roles.rs:27`, `pub(in crate::view)` — strips.rs is in view, OK).

## Depends on 07b/07c
07b second line + scroll (`commit_rows`) is the tall row's input; 07c not needed by the raster. 06b done (status line 3 of its ticket).

## Build with
Opus (two commits; raster + placement + perf script; mismatch 3 needs judgement).
