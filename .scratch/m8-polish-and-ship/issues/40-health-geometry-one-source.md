# health — one source for layout arithmetic: `update` calls graft-ui, the mirrors go

Status: ready (planner, from `docs/reviews/m8-architecture.md` Major M3; re-specified 2026-10-10 after scout `scouts/40-geometry.md`)

Build with: **Opus** (medium; scout and planner agree). This adds one new public seam, the vertical carve in plain numbers, and deletes two mirrors. A wrong carve puts a click one row off. New render-and-read-back tests judge that, so it doesn't pass Fable test 3.

Needs: 27 merged. It doesn't collide with the `health-split-*` tickets. If `health-split-update-hub` lands first, find `update::geometry`, `update::tour` and `update::banner` by name.
Blocked by: 27 (conductor, from Needs)
Seam for 38: 38 still needs this merged first. Its public set gains `Cells`, `graph_screen::{CarveInputs, RowCarve, row_carve}` and `preview_box::{MIN_BOX_ROWS, dock_cells}` (plus `DockInputs` if that gets added). `split_cells` and a public `tour_rows` are **dropped**.

## What changed from the draft (scout findings)
- **`detail_split` is kept, under the stop rule.** `Right` is one column wider on purpose: the gutter routes to the region it came out of (`graft-ui/src/detail_pane/layout.rs:81-84`; `update/tests/tab_strip_geometry.rs:202-207`). `Tab` follows graft's own `view::frame::detail_rects` (banner and hint rows), not `split_area`, so `split_cells` can't replace it. `split_cells` is dropped.
- **Tour card and pending block: one carve, not two fixes.** Both copies rebuild the same `left` (`tour.rs:59-64`, `banner.rs:325-331`) from graft-ui `screen_rows` (`graph_screen/layout.rs:308`). Their sources, `tour_rows` (:349) and `pending_rect` (:357), are private, and graft-ui is the only possible home for them. Graft-ui cannot call graft, so the draft's "`view::frame` calls `pending_rows_taken`" was impossible. The fix is one plain-number carve in graft-ui that both copies read. Making `tour_rows` public would remove one line, and it takes a `&GraphView` that update cannot build.
- **`dock_cells` takes the view's inputs as plain values.** The source `preview_box_area(frame, view)` reads `&PreviewBox`.
- **Three more copies the draft never named are out**, with a gap line for the conductor:
  - `detail_hit_areas` and `percent_of` (`geometry.rs:151-158`, copying `detail_pane::detail_areas`)
  - `choice_at` (`preview/rect.rs:63`, copying private `frame_box::buttons_row`)
  - `strip_bounds` (`geometry.rs:384`, copying `render_graph_screen`'s carve-out)
- **ARCHITECTURE.md:267-270 is already false.** Production `update` imports `view` at `geometry.rs:315`, `:432` and `:468`, and at `mouse.rs:118` and `:153`. The test-only exception at :271-276 is still needed by `detail_split`'s equality test.

## Read first
- `docs/reviews/m8-architecture.md` M3 (lines 78-95).
- `scouts/40-geometry.md`, all of it.
- ARCHITECTURE.md:260-280.
- ui MAP entries for `picker_box`, `screen_rows`, `chrome_rows` and `preview_box` (grep them). graft MAP:352, :496, :590-591, :957.
- ADR-0058 §4 (grep `mirror`).

## Scope
In:
1. **graft-ui crate root:** `pub type Cells`.
2. **graft-ui `graph_screen::layout`:**
   - `screen_rows`'s arithmetic, including the private `pending_rect` and `tour_rows`, moves into `row_carve`, which works on `Cells`.
   - `screen_rows` becomes a thin wrapper: `CarveInputs::of(view)`, then `row_carve`, then `Cells` → `Rect`. Exactly one body remains.
   - `CarveInputs` holds exactly the values `screen_rows` reads from `&GraphView` today, as `u16`/`bool`. Re-export them from `graph_screen`.
3. **graft-ui `preview_box`:**
   - `MIN_BOX_ROWS` becomes `pub`.
   - `dock_cells` holds `preview_box_area`'s arithmetic, and `preview_box_area` becomes a thin wrapper.
   - Values read from `&PreviewBox` are passed as plain args, or as `DockInputs` if there are more than three.
   - **Stop rule:** if a value isn't plain, keep the copy and name it in NOTES.
4. **graft `update`:**
   - `AppState::carve_inputs()` builds `CarveInputs` from state. It uses the same reads the two copies use today.
   - `tour::card_rows` and `banner::pending_rows_taken` each become one line reading `row_carve`. Callers are unchanged (`geometry::rows_above_graph` :37, `AppState::graph_rows` state.rs:584).
   - `preview/rect.rs` `preview_box_rect` calls `dock_cells`. Its 0×0 → `None` guard stays as a caller-side line; NOTES says it isn't arithmetic.
   - `update`'s own `MIN_BOX_ROWS` is deleted. `BoxRect` stays as update's type, gaining `From<Cells>`.
5. **New behavioural test first**, which must pass on the **unchanged** code. NOTES names the commit where it first passed. It covers the pending-block geometry the scout couldn't find a test for.
6. **ARCHITECTURE.md:267-276:**
   - Rewrite :267-270 to match the code: `update` may call ratatui-free `view` fns so that a hit-test uses the renderer's own answer. List each one with its call site, from `rg 'crate::view' crates/graft/src/update` excluding tests.
   - Keep the test-only exception (:271-276), and name `the_geometry_mirror_matches_the_rects_the_frame_hands_out` as its remaining user.
7. **Both MAPs**, for the items added, changed and deleted.

Out:
- `detail_split` (stop rule, deliberate gutter column) and its equality test, which stays.
- `detail_hit_areas`/`percent_of`, `choice_at`, `strip_bounds`. NOTES gives one KNOWN_GAPS line for the conductor: "health: four unnamed geometry mirrors left (…file:line…)".
- `Bounds`, badge packing (`view::hit`) and `chrome_rows`, which are already shared.
- Any change to what is drawn.

## Signatures
```rust
// graft-ui, crate root
pub type Cells = (u16, u16, u16, u16); // x, y, width, height

// graft-ui, graph_screen::layout, re-exported from graph_screen
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CarveInputs { /* pub fields: exactly what `screen_rows` reads from `&GraphView` today; u16 / bool only */ }
impl CarveInputs { pub fn of(view: &GraphView) -> CarveInputs; }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowCarve { /* one pub field per band `screen_rows` returns today: same name, same Option-ness, Rect -> Cells */ }
pub fn row_carve(area: Cells, inputs: CarveInputs) -> RowCarve;

// graft-ui, preview_box
pub const MIN_BOX_ROWS: u16;
pub fn dock_cells(area: Cells, /* plain values preview_box_area reads from &PreviewBox, or `inputs: DockInputs` */) -> /* preview_box_area's return type, Rect -> Cells */;

// graft, update
impl AppState { pub(crate) fn carve_inputs(&self) -> graft_ui::graph_screen::CarveInputs; }
impl From<graft_ui::Cells> for BoxRect;
```
Removed from graft: the `left` arithmetic in `tour::card_rows` and in `banner::pending_rows_taken` (each fn stays as a one-line read), and `update`'s `MIN_BOX_ROWS`.

## Acceptance
- [ ] New test `a_click_on_the_first_graph_row_lands_on_the_commit_drawn_there`, in `crates/graft/src/update/tests/carve_geometry.rs`. It sweeps widths 40/80/120, heights 12/24/48, a pending block of 0/2/5 rows, and tour on/off. For each case it renders the frame, checks that the first graph row drawn sits at `rows_above_graph`, and checks that a click there selects that row's commit. It passes before any deletion (commit named) and after.
- [ ] `the_card_takes_three_rows_off_the_graph_while_the_tour_is_on` (`update/tour/tests.rs:276`) passes, unchanged.
- [ ] `a_click_lands_on_the_button_the_widget_drew` (`update/preview/tests.rs:676`) passes, and its cases cover widths 40, 80 and 120. Add the missing widths; NOTES quotes the case list. `the_click_targets_match_what_the_widget_draws`: NOTES gives its location from `rg`, or says it doesn't exist.
- [ ] `a_click_below_one_hundred_columns_still_lands_on_the_row_it_named` and `the_geometry_mirror_matches_the_rects_the_frame_hands_out` (`tab_strip_geometry.rs:93`, :221) pass, unchanged.
- [ ] `update::tests::purity::update_cannot_reach_the_world` passes, unchanged.
- [ ] One body each. NOTES quotes all three `rg` outputs:
  - `rg -n 'chrome_rows' crates/graft/src/update/tour.rs crates/graft/src/update/banner.rs` → no hits.
  - `rg -n 'MIN_BOX_ROWS' crates/graft/src` → only `graft_ui::preview_box::MIN_BOX_ROWS` uses.
  - `rg -n 'fn pending_rect|fn tour_rows' crates/graft-ui` → none, or called only from `row_carve`.
- [ ] NOTES lists each deleted copy next to the graft-ui fn that replaced it, each deleted copy==source test with why it can no longer fail (the scout found none for mirrors 2-4), and `detail_split`'s kept difference.
- [ ] ARCHITECTURE.md:267-276 rewritten as in Scope 6. NOTES quotes the `rg 'crate::view'` output it was written from.
- [ ] NOTES gives the conductor:
  - an ADR-0058 mirror-list line: which mirrors are gone and which remain
  - the KNOWN_GAPS line from Out
  - the new `pub` items for 38

## Touches
How each was checked goes on the report's **Touches:** line.
- graft-ui: crate root (`Cells`), `graph_screen::layout` (`screen_rows` and its callers, by `rg screen_rows`), `graph_screen` re-exports, `preview_box` (`preview_box_area` callers, including `view/frame/boxes.rs:112`).
- graft: `update::tour`, `update::banner`, `update::preview`, `preview/rect.rs`, `update::state` (`carve_inputs`), and the callers of `card_rows`/`pending_rows_taken`/`preview_box_rect` (by `rg`).
- Mouse hit-tests for the graph and the preview buttons, covered by the tests above.
- ARCHITECTURE.md:267-276, ui MAP, graft MAP.
- ADR-0058 and KNOWN_GAPS: through NOTES, for the conductor. The builder doesn't edit them.
- Ticket 38: new public items, in NOTES.
- Keymap, journal/undo, captures: none, since nothing drawn moves. NOTES says the captures need no re-shoot, backed by the full nextest run, which includes the capture tests.

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`.

## Size
About 65k tokens.
