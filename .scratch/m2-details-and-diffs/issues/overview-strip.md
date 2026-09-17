# overview-strip: the one-column whole-history strip — marks assembly, widget, `O`

Status: draft
Build with: Sonnet — a bucketed lookup over rows already on hand and a one-column widget; the priority order and glyphs are fixed in the note, judged by tests and the capture.
Needs: `radar-context-and-markers` (`marker_for`, `MarkerView`), `mistake-radar-detection` (`Severity`), `detail-pane-frame` (`view::frame`'s graph/pane split — the strip takes the graph area's last column), `map-view-data` (`AppState::map.shown` — the strip is not drawn while it is true)

## Read first

- `docs/design-notes/m2-details-and-diffs.md` — New UI "Overview strip (`O`)" in full; Interfaces `graft-ui::overview_strip` line (as amended in pass 2c: `StripMark::Head` carries nothing); Proposed changes row 1 (markers are a paint-time lookup: the strip sees findings for resident rows only)
- `DESIGN.md` §5.3 lines 1246–1268 (Shows / Toggle / Cost) and §2.3 line 436 (`O`)
- `crates/graft/MAP.md` lines 264–308 (`history`: `History::rows`, `window`, `top`, `visible`, `select_at`, `Window::laid_out`, `Window::first`), 133–138 (`search`: `Search::hits`/`hit` — learn whether a hit carries a row number or only a commit id; one scout summary of `search.rs`'s hit type if the MAP line doesn't say, never the file), 56–60 (`RefBadges::head`), 202–213 (`view`)
- `crates/graft-graph/MAP.md` lines 17–24 (`Row::commit`, `Row::marker`)
- `crates/graft-ui/MAP.md` lines 35–70 (`graph_screen`: `GraphScreen::graph_area`, `CHROME_ROWS`), 119–137 (`theme`)
- `.scratch/m2-details-and-diffs/issues/radar-context-and-markers.md` — `marker_for` signature and the `History::rows` eviction note (its Read-first, `history` line)

## Scope

In:
- `crates/graft-ui/src/overview_strip.rs` (new; `pub mod overview_strip`):
  - Types below. `StripMarks::rows_total` is how many rows exist as far as the app knows (`Window::laid_out` — honestly incomplete for a repo still streaming, per DESIGN §5.3 "Cost"); `viewport` is the rows on screen; `marks` are absolute row numbers, unsorted allowed.
  - `OverviewStrip { marks, theme, ascii }`, `impl Widget`, one column wide, `area.height` cells: bucket = `ceil(rows_total / height)` rows (min 1). Each cell draws the highest-priority mark of any row in its bucket: `⚠` (warning colour) > `ⓘ` (info) > `◆` search hit (accent) > `●` HEAD (accent) > `◉` merge (`text-muted`) > `█` viewport (accent, only where nothing else drew) > `┆` (`text-muted`). ASCII: `!`, `i`, `*`, `o`, `+`, `#`, `:`. Below `rows_total`'s last bucket (when `rows_total < height`) the column is blank. `pub fn cell_for(marks: &StripMarks, height: u16, cell: u16) -> StripCell` is the pure half the widget and the tests share.
  - `pub fn row_for_cell(marks: &StripMarks, height: u16, cell: u16) -> usize` — the first row of a cell's bucket (for the click).
- `crates/graft/src/view/strip.rs` (new submodule of `view`): `pub fn strip_marks(state: &AppState) -> StripMarks` — over the rows `History` holds now (`History::rows`, whatever window that is): `Merge` where `row.commit.parents.len() >= 2`; `Radar(sev)` from `marker_for(state, id, parents, subject)` (only once `state.radar.context` is `Ready`, as `marker_for` already guarantees); `Head` on the row whose commit is `RefBadges::head()`; `SearchHit` for each search hit — at its row number if `Search` hits carry one, else for hits whose commit is a resident row (state which in the report; both are honest per §5.3's "fills in as background streaming catches up"). `viewport = top..top + graph_rows`. Empty history → `rows_total = 0`, no marks.
- `crates/graft/src/update/strip.rs` (new, tiny): `AppState::strip_shown: bool` (`Default` false — DESIGN §5.3 makes it a toggle, off until asked; a `[ui] overview_strip = true` config key is **out**, M8 polish). `O` on the graph screen (search prompt closed, detail focus on `Graph`) flips it. Nothing else changes state; the map's hide/restore needs no state: the strip is drawn only while `strip_shown && !map.shown`.
- `view::frame`: while the strip draws, the graph screen's area loses its last column to the strip — the rightmost column of the graph area in every layout (narrow: the screen's right edge; wide/medium with the pane shown: the column just left of the detail divider). `GraphScreen::graph_area` and the pixel-tier image rect follow from the narrower area, nothing else moves.
- Mouse: a left click on the strip column → `History::select_at(row_for_cell(...))` (the same clamp a graph click uses). Wheel on the strip: as on the graph.
- Tests: `cell_for` priority (a bucket holding a merge, a search hit and a warning draws `⚠`; viewport-only draws `█`; empty draws `┆`; beyond `rows_total` blank); bucket math (`rows_total = 100, height = 24` → 5 rows per cell, last cell covers `96..100`); `strip_marks` on `Shape::FeatureBranches` (two merges at their rows, `Head` on `main`'s tip row, viewport `0..graph_rows`); `O` toggles; strip absent while `map.shown`; frame snapshot pair with the strip on/off at 80×24 (the graph column loses one cell, the text columns don't move); click test. MAP.md lines (`graft-ui`, `graft`).
- Capture: `Shape::FeatureBranches` at 80×24, `O` pressed, Unicode and ASCII (same route as `map-view-screen`'s capture: gallery harness, else `TestBackend` dump of the real frame), to `docs/shots/m2-overview-strip.txt`, ≤ 200 KB.

Out: a config key for the strip's default; lane letters beside `●` (M1 draws none — DESIGN's `●h` has nothing to carry, hence `StripMark::Head` without a payload); marks for rows not on hand (by design, note Proposed changes row 1); the pixel tier (none, per DESIGN §5.3).

## Signatures

```rust
// crates/graft-ui/src/overview_strip.rs
pub enum StripMark { Merge, Radar(Severity), SearchHit, Head }
pub struct StripMarks { pub rows_total: usize, pub viewport: std::ops::Range<usize>, pub marks: Vec<(usize, StripMark)> }
pub enum StripCell { Blank, Rail, Viewport, Merge, Head, SearchHit, Radar(Severity) }
pub fn cell_for(marks: &StripMarks, height: u16, cell: u16) -> StripCell;
pub fn row_for_cell(marks: &StripMarks, height: u16, cell: u16) -> usize;
pub struct OverviewStrip<'a> { pub marks: &'a StripMarks, pub theme: &'a Theme, pub ascii: bool }
impl<'a> ratatui::widgets::Widget for OverviewStrip<'a> { fn render(self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer); }

// crates/graft/src/view/strip.rs
pub fn strip_marks(state: &AppState) -> StripMarks;

// crates/graft/src/update/strip.rs
// AppState::strip_shown: bool   (`O` flips it; drawn only while `strip_shown && !map.shown`)
```

## Acceptance

- [ ] `cell_for` priority and bucket tests as listed; `row_for_cell(100 rows, 24 cells, cell 23) == 96`.
- [ ] `strip_marks` on `Shape::FeatureBranches`: `Merge` at both merge rows, `Head` at row 0, `viewport == 0..graph_rows`, `rows_total == 9`; after the radar context is `Ready` on `Shape::FoxtrotMerge`, one `Radar(Warning)` at the merge's row.
- [ ] `O` toggles `strip_shown`; the frame reserves the last graph column while on and none while `map.shown`.
- [ ] Frame snapshot pair: with the strip on, the graph column is one cell narrower and the subject/author/date/SHA columns start in the same screen column as with it off.
- [ ] Click on the strip selects the bucket's first row.
- [ ] MAP.md: `graft-ui` (`overview_strip`), `graft` (`view::strip`, `update::strip`, `AppState::strip_shown`, `frame` line).
- [ ] Capture at `docs/shots/m2-overview-strip.txt`, ≤ 200 KB; user judges by eye.

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet` · purity guard (`update/strip.rs`).

## Size

Two crates: `graft-ui` (`overview_strip.rs` ~130 lines), `graft` (`view/strip.rs` ~90, `update/strip.rs` ~25, `frame` + mouse edits ~40). Tests ~140, snapshots ≤ 4. ≤ 22 tool calls. Named seam: commit `overview_strip.rs` + `strip_marks` with their tests first; if the frame column reservation + mouse + capture then passes 70 % of budget, stop and report `partial` with the mouse click and the capture as the remainder.
