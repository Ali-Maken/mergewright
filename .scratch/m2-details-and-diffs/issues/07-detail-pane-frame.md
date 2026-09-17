# detail-pane-frame: the detail pane's regions, breakpoints and states — no diff rendering yet

Status: ready
Blocked by: 06

Status: draft
Build with: Sonnet — layout math and widget composition; judged by the capture, not a design decision.
Needs: `detail-model-and-jobs` (`DetailView`, `DetailHeader`, `Subject`, `ContainedIn`, `RadarLine`, `FileListView`, `FileRow`, `FileRowKind`, `FileStatusView`, `BodyView`, `HunkView`, `LineView`, `LineKindView`, `DetailFocus`, `DiffMode`, `DiffOptionsView`, `AppState::detail`, `AppState::repository`/history row access)

## Read first

- `docs/design-notes/m2-details-and-diffs.md` — Sizing from M1; Interfaces (`graft-ui::detail`, the `DetailLayout` line); New UI "Detail pane" in full (all three breakpoints, states, diff-line chrome minus tints — this ticket draws diff lines plainly, `diff-widget` adds tints/gutter/pin); "Compare" bullet, header line only (render `Subject::Compare`'s header text; the `x` mechanic is `compare-two-commits`)
- `DESIGN.md` §1.1–1.3 (pane position at each breakpoint) and §2.15 (chrome is rules, not boxes)
- `crates/graft-ui/MAP.md` — `graph_screen` (`CHROME_ROWS`, `AUTHOR_BREAKPOINT`, `GraphScreen::graph_area`, breakpoint pattern to mirror), `theme`, `fit`
- `crates/graft/MAP.md` — `view` section (`view`, `view_with_graphics`, `AppState::graph_rows`), `update` section lines for `AppState::screen`, `AppState::size`, `History::selected`/`row_at`

## Scope

In:
- `crates/graft-ui/src/detail_pane.rs` (new; `pub mod detail_pane` in lib.rs):
  - `DetailLayout::compute(width, height, focus, override_cells) -> DetailLayout` — `width >= 160` → `Right { columns: 42% of width, min 48 }`; `100..160` → `Below { rows: 40% of height, min 8 }`; `< 100` → `Tab { shown: focus != DetailFocus::Graph }` (the narrow tab strip reuses `focus`: no new "which tab" state). `override_cells` (from `detail-keys-and-focus`'s resize) replaces the percentage figure, still clamped to the min.
  - `detail_areas(pane_area, layout, files_count) -> DetailAreas { header, files, body }` — header rows fixed per variant (5 `Right`, 2 `Below`, 3 `Tab`); `Below` splits files (30 cols) beside body horizontally; `Right`/`Tab` stack files (≤ 40 % of what's left after the header, capped by `files_count`) above body.
  - `DetailPane<'a> { view: &'a DetailView, layout: &'a DetailLayout }`, `impl Widget for DetailPane<'a>`: draws header text exactly as the New UI breakpoint bullets word it (accent title when `view.focus` is that region, `text-muted` otherwise); files as a flat list of `FileRow`s indented by `depth`, cursor row highlighted, `M/A/D/R/C/T` + `+N −N` right-aligned, `bin` for binary, first `✉ message (N lines)` row when present; body dispatch: `BodyView::Empty/Message/Loading/Failed` fully worded per the New UI "States" bullet (`select a commit`, `reading…`, `git diff-tree: <line>` in `warning`, `untracked · no diff yet (M3)`, `binary · … `); `BodyView::Diff` calls `pub fn diff_body(hunks: &[HunkView], top: usize, area: Rect, buf: &mut Buffer, theme: &Theme, focused: bool) -> usize` — **this ticket's version is plain**: one screen row per `LineView`, sign glyph (`+`/`-`/` `) and kind soft-bg only, text fit to width, no gutter numbers, no pinned heading, no word/syntax tint, no `␍`, no truncation footer line (all of that is `diff-widget`, which replaces this function's body under the same signature). Returns total content rows for scroll clamping.
  - 16-colour/`NO_COLOR`: sign + bold/dim only (no soft bg), per the m1 12-colour rule.
- `crates/graft/src/view/detail.rs` (new submodule of `view`): `pub fn detail_view(state: &AppState) -> DetailView` — assembles `DetailHeader` from the selected row's already-hydrated commit facts (`History::selected`/`row_at`, the same source `GraphEntry`'s author/date/subject come from) plus `state.detail.contained.get(&oid)` for `Contained`; `header.radar` stays `vec![]` (wired by `radar-context-and-markers`); `files`/`body`/`focus`/`mode`/`options`/`explain` come straight from `AppState::detail`. If the commit facts available to the graph don't carry parents/committer/co-author trailers in full (only subject/author/date), build what's available and leave the missing header lines blank rather than guessing at a source you haven't read — report the exact gap as your ticket's `partial` seam, don't invent a git call.
  - Wire `DetailPane` into `view::frame`/`view::rows`: when `state.detail.shown`, split the graph area using `DetailLayout::compute`, draw `GraphScreen` in the remaining area, `DetailPane` in the rest; narrow `Tab` variant draws whichever of graph/pane `shown` names, full width, plus the `[Graph] Detail` strip (active side per `focus != Graph`).
- Tests: unit tests on `DetailLayout::compute` (breakpoints, override, tab shown-by-focus), `detail_areas` (percentages, min clamps, `Below`'s horizontal split), `DetailPane` snapshot tests on a `TestBackend` for a hand-built `DetailView` at each breakpoint and each `BodyView` state. MAP.md lines (`graft-ui`, `graft`).
- Capture: render a hand-built `DetailView` (one commit, three files, a two-hunk diff, one radar-less header) with `DetailPane` on a `TestBackend` at 160×40, 120×30, 80×24; dump each buffer as plain text, trim to the pane's own rect, ≤ 200 KB combined, to `docs/shots/m2-detail-pane-breakpoints.txt`.

Out: `diff-widget`'s tinted/gutter rendering, `side-by-side-view`, all keys (`detail-keys-and-focus`), `x` compare mechanics (`compare-two-commits`), the marker column and radar line content (`radar-context-and-markers`), the Explain panel body (`explain-panel`), resize persistence.

## Signatures

```rust
// crates/graft-ui/src/detail_pane.rs
pub enum DetailLayout { Right { columns: u16 }, Below { rows: u16 }, Tab { shown: bool } }
impl DetailLayout { pub fn compute(width: u16, height: u16, focus: DetailFocus, override_cells: Option<u16>) -> DetailLayout; }
pub struct DetailAreas { pub header: ratatui::layout::Rect, pub files: ratatui::layout::Rect, pub body: ratatui::layout::Rect }
pub fn detail_areas(pane_area: ratatui::layout::Rect, layout: &DetailLayout, files_count: usize) -> DetailAreas;
pub fn diff_body(hunks: &[HunkView], top: usize, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer, theme: &Theme, focused: bool) -> usize;
pub struct DetailPane<'a> { pub view: &'a DetailView, pub layout: &'a DetailLayout }
impl<'a> ratatui::widgets::Widget for DetailPane<'a> { fn render(self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer); }

// crates/graft/src/view/detail.rs
pub fn detail_view(state: &AppState) -> DetailView;
```

## Acceptance

- [ ] `DetailLayout::compute`: 160/159/99 wide give `Right`/`Below`/`Tab`; `Tab`'s `shown` follows `focus != Graph`; `override_cells` replaces the percentage, still clamped to the min (48/8).
- [ ] `detail_areas`: `Right`/`Tab` stack (files above body, header above both); `Below` splits files|body horizontally beside a 2-line header; files height/width never exceeds its cap even with `files_count` huge, never negative with `files_count == 0`.
- [ ] Snapshot: each `BodyView` state (`Empty`, `Message`, `Loading`, `Failed`, `Diff`) renders its exact wording from the New UI "States" bullet.
- [ ] Snapshot: focused region's title paints accent, the other two `text-muted`.
- [ ] `diff_body` on a two-hunk fixture draws one row per line with the right sign glyph and soft bg per `LineKindView`, no gutter numbers (this ticket's plain version).
- [ ] MAP.md: `graft-ui` (`detail_pane`), `graft` (`view::detail`).
- [ ] Capture at `docs/shots/m2-detail-pane-breakpoints.txt`, ≤ 200 KB; user judges by eye.

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet`.

## Size

Two crates: `graft-ui` (`detail_pane.rs` ~260 lines: layout math ~60, widget ~200), `graft` (`view/detail.rs` ~90). Tests + snapshots ~220. ≤ 30 tool calls. If `detail_view`'s header assembly hits a data gap (commit facts the graph doesn't already carry), stop there, ship the widget + layout with a hand-built `DetailView` in tests, and report `partial` naming exactly which header field is missing and where it would come from.
