# health — one source for layout arithmetic: `update` calls graft-ui, the mirrors go

Status: done (builder 2026-10-10; was ready — planner, from `docs/reviews/m8-architecture.md` Major M3; re-specified 2026-10-10 after scout `scouts/40-geometry.md`)

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
- [x] New test `a_click_on_the_first_graph_row_lands_on_the_commit_drawn_there`, in `crates/graft/src/update/tests/carve_geometry.rs`. It sweeps widths 40/80/120, heights 12/24/48, a pending block of 0/2/5 rows, and tour on/off. For each case it renders the frame, checks that the first graph row drawn sits at `rows_above_graph`, and checks that a click there selects that row's commit. It passes before any deletion (commit named) and after. — **verified by** `update::tests::carve_geometry::a_click_on_the_first_graph_row_lands_on_the_commit_drawn_there`: green at `705e8cb3` (unchanged code), green after. "0/2/5" = picks still to apply (blocks of 0, 4, 7 rows). Mutation checked: `+1` on `pending_rows_taken` fails at `40x12, 2 pending, tour false`; `+1` on `card_rows` fails at `40x12, 0 pending, tour true`.
- [x] `the_card_takes_three_rows_off_the_graph_while_the_tour_is_on` (`update/tour/tests.rs:276`) passes, unchanged. — **verified by** that test, untouched file, full nextest.
- [x] `a_click_lands_on_the_button_the_widget_drew` (`update/preview/tests.rs:676`) passes, and its cases cover widths 40, 80 and 120. Add the missing widths; NOTES quotes the case list. `the_click_targets_match_what_the_widget_draws`: NOTES gives its location from `rg`, or says it doesn't exist. — **verified by** `update::preview::tests::a_click_lands_on_the_button_the_widget_drew` (now `for columns in [40, 80, 120]`).
- [x] `a_click_below_one_hundred_columns_still_lands_on_the_row_it_named` and `the_geometry_mirror_matches_the_rects_the_frame_hands_out` (`tab_strip_geometry.rs:93`, :221) pass, unchanged. — **verified by** both tests, file untouched.
- [x] `update::tests::purity::update_cannot_reach_the_world` passes, unchanged. — **verified by** that test.
- [x] One body each. NOTES quotes all three `rg` outputs: — **verified by** the greps in NOTES, plus `a_click_on_the_first_graph_row…` (behaviour).
  - `rg -n 'chrome_rows' crates/graft/src/update/tour.rs crates/graft/src/update/banner.rs` → no hits.
  - `rg -n 'MIN_BOX_ROWS' crates/graft/src` → only `graft_ui::preview_box::MIN_BOX_ROWS` uses.
  - `rg -n 'fn pending_rect|fn tour_rows' crates/graft-ui` → none, or called only from `row_carve`.
- [x] NOTES lists each deleted copy next to the graft-ui fn that replaced it, each deleted copy==source test with why it can no longer fail (the scout found none for mirrors 2-4), and `detail_split`'s kept difference. — NOTES below (doc item; behaviour backed by the tests above).
- [x] ARCHITECTURE.md:267-276 rewritten as in Scope 6. NOTES quotes the `rg 'crate::view'` output it was written from. — doc item; table checked line by line against the grep below.
- [x] NOTES gives the conductor: — NOTES below.
  - an ADR-0058 mirror-list line: which mirrors are gone and which remain
  - the KNOWN_GAPS line from Out
  - the new `pub` items for 38

## NOTES (builder, 2026-10-10)
Status: done. No compaction. Scout used: `scouts/40-geometry.md` (no new scout). Skills: implement (SKILL.md read), tdd approach (test first).

**Test-first commit:** `705e8cb3` — `carve_geometry.rs` added, green on unchanged code.

**Deleted copies → replacement**
- `tour::card_rows`'s `left` rebuild (`chrome_rows` + harness banner + commit-graph + tab rows) → `graft_ui::graph_screen::row_carve(..).tour` via `update::geometry::graph_carve` (one line).
- `banner::pending_rows_taken`'s `left` rebuild + `wanted.min(left / 2)`, `>= 3` → `row_carve(..).pending` via `graph_carve` (one line).
- `preview/rect.rs` `preview_box_rect` dock arithmetic → `graft_ui::preview_box::dock_cells`. The `width == 0 || height == 0 → None` guard stays: a caller-side "no box on an empty frame", not arithmetic.
- `update::preview::MIN_BOX_ROWS` → `graft_ui::preview_box::MIN_BOX_ROWS` (now pub; graft names it nowhere, dock_cells uses it).
- graft-ui itself: `screen_rows` is now a wrapper over `row_carve` (one body, in new private file `graph_screen/carve.rs` with `tour_rows`/`pending_rect`/`tour_rect`/`body_rect`/`banner_rows`); `preview_box_area` is a wrapper over `dock_cells`.
- Copy==source tests deleted: none existed for mirrors 2-4 (scout; confirmed — the only copy==source test is `the_geometry_mirror_matches…`, for `detail_split`, kept). None could fail now anyway: there is no second copy to diverge.
- Behaviour note: the carve clips the banner to free rows and drops the offer when no row is spare; the old copies didn't. Differs only at tiny heights; the update side now matches what is drawn there.
- `detail_split` kept: `Right` graph region is one column wider than `split_area`'s (gutter routes to its source region, `graft-ui/src/detail_pane/layout.rs:81-84`); `Tab` follows graft's `view::frame::detail_rects` (banner/hint rows), not `split_area`.

**Greps** (`rg` is a shell alias here; plain `grep -rn` equivalents):
- `grep -n 'chrome_rows' update/tour.rs update/banner.rs` → no hits.
- `grep -rn 'MIN_BOX_ROWS' crates/graft/src` → no hits (graft names it nowhere; graft-ui's `dock_cells` is the only user).
- `grep -rnE 'fn pending_rect|fn tour_rows|pending_rect\(|tour_rows\(' crates/graft-ui` → `graph_screen/carve.rs`: `fn tour_rows`, `fn pending_rect`, called only from `row_carve` (pending_rect) and `tour_rect` (tour_rows, itself called only from `row_carve`).

**`crate::view` in production `update`** (ARCHITECTURE table written from this; test-module hits `confirm.rs:729`, `mouse.rs:293` excluded): `context_menu.rs:61`; `finish.rs:185,192,195`; `mouse.rs:118,131,153`; `geometry.rs:332,449,485`; `operate/failure.rs:233`; `detail/control.rs:138,155`; `operate/outcome/staging_not_kept.rs:60`; `finish/keys.rs:112,116`; `finish/mouse.rs:35,38,58,63`; `rewrite_editor/keys.rs:99,101`; `resolver/scroll.rs:89,90`; `resolver/geometry.rs:45,73`; `palette/input.rs:220-228`. Broader than the scout's five: not only hit-tests (also `radar::findings_for`, `status::dismiss`/`done_line`, `build_explain_view`). The rewrite says so rather than "hit-tests only".

**Preview test cases:** `for columns in [40, 80, 120]` at 40 rows (fixture's height). Button cells now found by cell, not byte offset (the old `find` returned a byte index; `▸`/`│` are multi-byte). `the_click_targets_match_what_the_widget_draws`: **does not exist** — `grep -rn` finds it only in the comment at `update/preview/rect.rs` (`choice_at` doc, out of scope; stale name left for the `choice_at` ticket).

**For the conductor**
- ADR-0058 mirror list: gone — `tour::card_rows` left-carve, `banner::pending_rows_taken` left-carve, `preview_box_rect` dock + `MIN_BOX_ROWS`. Remain — `detail_split` (deliberate), `detail_hit_areas`/`percent_of`, `choice_at`, `strip_bounds`.
- KNOWN_GAPS line: "health: four unnamed geometry mirrors left (`update/geometry.rs:158,165` `percent_of`/`detail_hit_areas` copy `graft_ui::detail_pane::detail_areas`; `update/preview/rect.rs:58` `choice_at` copies private `frame_box::buttons_row`; `update/geometry.rs:397` `strip_bounds` copies `render_graph_screen`'s carve-out) — unverified, not broken; the `choice_at` doc names a test that doesn't exist."
- File-size ratchet (`crates/graft/tests/it/over_400.txt`) rows changed: `graph_screen/layout.rs` 765→650, `graph_screen/mod.rs` 601→603, `update/geometry.rs` 501→518, `update/preview.rs` 469→464, `update/preview/tests.rs` 2114→2141, `update/state.rs` 1287→1304. Four raised (visible in diff).
- Captures: none re-shot; nothing drawn moves (graft-ui arithmetic unchanged, only moved). Backed by full nextest (6298/6298, capture tests included).

**Public items for 38 (final signatures)**
```rust
// graft_ui (crate root)
pub type Cells = (u16, u16, u16, u16); // x, y, width, height
// graft_ui::graph_screen (defined in private graph_screen/carve.rs, re-exported)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CarveInputs { pub spaced: bool, pub banner_rows: u16, pub commit_graph: bool, pub tour: bool, pub pending_rows: u16 }
impl CarveInputs { pub fn of(view: &GraphView<'_>) -> Self; }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowCarve { pub header: Cells, pub gap: Cells, pub banner: Cells, pub tour: Cells, pub pending: Cells, pub body: Cells, pub rule: Cells, pub offer: Cells, pub footer: Cells }
pub fn row_carve(area: Cells, inputs: CarveInputs) -> RowCarve;
// graft_ui::preview_box
pub const MIN_BOX_ROWS: u16 = 4;
pub fn dock_cells(frame: Cells, wanted: u16) -> Cells; // wanted = preview_rows(view, frame.width)
// graft::update (crate-internal)
impl AppState { pub(crate) fn carve_inputs(&self) -> CarveInputs; }
pub(super) fn geometry::graph_carve(state: &AppState) -> RowCarve; // new, not in spec: the one place the update-side area is built
impl From<graft_ui::Cells> for BoxRect; // in update/preview/rect.rs
```
No `DockInputs`: `preview_box_area` reads one value from `&PreviewBox` (`preview_rows(view, width)`), passed as `wanted`.

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
