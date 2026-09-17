# map-view-screen: the Map view widget, its view assembly, frame wiring and capture

Status: draft
Build with: Sonnet — a widget over data `map-view-data` already produces, with every glyph, column and state worded in the note; judged by snapshots and the capture.
Needs: `map-view-data` (`MapData`, `MapBranch`, `MapMark`, `MapState`, `AppState::map`), `mistake-radar-detection` (`Severity`), `detail-pane-frame` (`view::frame`'s split of graph/pane areas — the map replaces the graph area, the pane stays where it is)

## Read first

- `docs/design-notes/m2-details-and-diffs.md` — New UI "Map view (`M`)" in full (row order, glyph ramps, colour, elbow, marks, `▸`/`NOW`, keys, empty/loading states); Proposed changes row "Map view draws text glyphs at every tier"; Interfaces `app::map` line
- `DESIGN.md` §5.1 lines 1148–1176 (the mockup and the Unicode/ASCII ramps — the frame's box in the mockup is *not* drawn: chrome is rules, §2.15)
- `crates/graft-ui/MAP.md` lines 35–70 (`graph_screen`: `GraphView::banner`, `ascii`, `CHROME_ROWS`, `GraphScreen::new` — the pattern to mirror), 14–16 (`fit`), 86–88 (`paint`), 119–137 (`theme`: `Theme::branch`, `Theme::token`, `color_depth`)
- `crates/graft/MAP.md` lines 202–213 (`view`: `frame`, `graph_view`, `status_view`), 44 (`Clock`), and `.scratch/m2-details-and-diffs/issues/detail-pane-frame.md` Scope's "Wire `DetailPane` into `view::frame`" bullet (where the graph area is decided)
- `crates/graft-term/MAP.md` lines 71–72 (`sanitize*`)

## Scope

In:
- `crates/graft-ui/src/map_screen.rs` (new; `pub mod map_screen`):
  - View types below; every string `SanitizedText`. `MapRowView::counts` are the raw bucket counts; the widget maps `count / max` to the ramp (`▁▂▃▄▅▆▇█` Unicode, ` .:-=+*#%@` ASCII when `ascii`), a zero bucket blank, `count > 0` never below the first non-blank glyph.
  - `MapScreen { view, theme }`, `impl Widget`. Layout per row of the area: header line 1 `MAP · <current or default branch> · <span>` (span worded `last N days` / `last N months`, `N` from `from..to`), header line 2 `BRANCH` left and `NOW` right-aligned over the last bucket column; then rows: `▸ ` or `  ` (2 cells), name fitted to 18 cells, one space, the bar cells (`buckets` wide, each bucket one cell, coloured `Theme::branch(identity)`), then the current branch's row ends with ` ●` in the accent; other rows: ` (active)` in `text-muted` when the tip is in the newest bucket and not merged. The elbow: `●╮` at `merged_into.1` in the row's colour (ASCII `o\`), overriding the bucket glyph. Marks over the bucket: the mark replaces the bucket cell — `⚠`/`ⓘ` (`!`/`i` ASCII) in the severity colour, a tag as `┴` in `text-muted` with `[<tag>]` written right of the bar when there is room. Selected row: cursor highlight (same style the graph screen uses for its cursor row). Footer (last line): `↑↓ select  ⏎ zoom to commits  M/Esc back`, plus ` · +N more` when `more > 0`, plus ` · partial` in `warning` when `!complete`.
  - States: `MapStatus::Loading { done, total }` → the header, then one centred line `building map… <done>/<total> branches` and `Esc cancels` in the footer; `Empty` → `no local branches`; `Failed(line)` → the line in `warning`; `Ready` → rows.
  - 16-colour/`NO_COLOR`: bars in the default fg, current row bold, marks as glyphs only (m1 12-colour rule).
- `crates/graft/src/view/map.rs` (new submodule of `view`): `pub fn map_view(state: &AppState, now_seconds: i64) -> MapView` — from `state.map` (`Loading` → `MapStatus`), names sanitised with `sanitize_line`, `ascii` from the tier in force (same source `GraphView::ascii` uses), `selected` from `MapState::selected`.
- `view::frame`: when `state.map.shown`, draw `MapScreen` in the area the graph screen would take (the detail pane, when shown, keeps its own area; the narrow tab strip shows `Map` in place of `Graph`); the Overview strip column is not reserved (see `overview-strip`, which checks `map.shown`). Nothing else in `frame` changes.
- Tests: `TestBackend` snapshots of `MapScreen` for a hand-built `MapView` at 80×24 (Unicode and ASCII) and 120×40 (Unicode), one each for `Loading`/`Empty`/`Failed`; a `map_view` test on `Shape::FeatureBranches` after a hand-set `Ready` `MapData` (names sanitised, `selected` carried). MAP.md lines (`graft-ui`, `graft`).
- Capture: `Shape::FeatureBranches` after `M`, via the gallery example's harness (`Harness` keys/banner as `35` used them — if the harness can't pre-press a key yet, render `MapScreen` on a `TestBackend` from the fixture's real `build_map` output and dump the buffer), 80×24 Unicode and ASCII, plus 120×40 Unicode, to `docs/shots/m2-map-view.txt`, ≤ 200 KB. Also `docs/shots/m2-map-view-loading.txt` from the `Loading` snapshot buffer.

Out: pixel-tier density curves; mouse on the map (`M2` keeps keys only — say so in the module doc, it is a gap row for M8 polish); any change to `MapData`.

## Signatures

```rust
// crates/graft-ui/src/map_screen.rs
pub enum MapMarkView { Tag(SanitizedText), Radar(Severity) }
pub struct MapRowView { pub name: SanitizedText, pub identity: BranchIdentity, pub counts: Vec<u32>, pub is_current: bool, pub merged_into: Option<(usize, usize)>, pub marks: Vec<(usize, MapMarkView)>, pub active: bool, pub complete: bool }
pub enum MapStatus { Loading { done: u32, total: u32 }, Empty, Failed(SanitizedText), Ready }
pub struct MapView { pub title_branch: SanitizedText, pub span_days: u32, pub rows: Vec<MapRowView>, pub max_count: u32, pub buckets: usize, pub selected: usize, pub more: usize, pub complete: bool, pub status: MapStatus, pub ascii: bool }
pub struct MapScreen<'a> { pub view: &'a MapView, pub theme: &'a Theme }
impl<'a> ratatui::widgets::Widget for MapScreen<'a> { fn render(self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer); }
pub fn ramp_glyph(count: u32, max: u32, ascii: bool) -> char;   // pure; blank for 0

// crates/graft/src/view/map.rs
pub fn map_view(state: &AppState, now_seconds: i64) -> MapView;
```

## Acceptance

- [ ] `ramp_glyph`: `0 → ' '`, `max → '█'` (`'@'` ASCII), `1` with `max = 1000 → '▁'` (`'.'`), never a blank for a non-zero count.
- [ ] Snapshot 80×24 Unicode: header two lines, `▸ main` row ending ` ●`, a merged row showing `●╮` at its `merged_into` bucket, a `⚠` mark cell, `[v1.0]` right of a bar, footer with `⏎ zoom to commits`.
- [ ] Snapshot 80×24 ASCII: same rows with ` .:-=+*#%@`, `o\`, `!`.
- [ ] Snapshots for `Loading { 2, 5 }` (`building map… 2/5 branches`, `Esc cancels`), `Empty` (`no local branches`), `Failed`.
- [ ] `view::frame` with `map.shown` draws no graph rows and no strip column; with it hidden, the frame is byte-identical to before this ticket (a snapshot pair).
- [ ] MAP.md: `graft-ui` (`map_screen`), `graft` (`view::map`, `frame` line).
- [ ] Captures at `docs/shots/m2-map-view.txt` and `docs/shots/m2-map-view-loading.txt`, each ≤ 200 KB; user judges by eye.

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet`.

## Size

Two crates: `graft-ui` (`map_screen.rs` ~240 lines), `graft` (`view/map.rs` ~70, `frame` edit ~20). Snapshots ≤ 6, tests ~120. ≤ 24 tool calls. Named seam: commit `map_screen.rs` with its snapshots first; if the frame wiring + capture then passes 70 % of budget, stop and report `partial` with the capture as the remainder.
