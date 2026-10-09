# health — one source for layout arithmetic: `update` calls graft-ui, the mirrors go

Status: draft (planner, from `docs/reviews/m8-architecture.md` Major M3)

Build with: _(blank: brownfield, ADR-0035.)_ The planner expects **Opus**. If a mirror is deleted, a click can land one row off from what is drawn. The render-and-read-back tests judge it, but which mirror is a pure copy and which one hides a deliberate difference needs reading.

Needs: 27 merged. It doesn't collide with the `health-split-*` tickets, but if `health-split-update-hub` lands first, re-locate `update::geometry` by name.
Blocked by: 27 (conductor, from Needs)

## Why
`graft::update` keeps hand copies of graft-ui's layout arithmetic, because the purity guard keeps `ratatui` out of `update/`:
- `update::geometry::detail_split` copies `graft_ui::detail_pane::split_area` (ui MAP:121; ARCHITECTURE.md:267–276).
- `tour::card_rows` copies `layout::tour_rows` (graft MAP:352).
- `MIN_BOX_ROWS`/`BoxRect` copy `preview_box`'s dock (graft MAP:590–591).
- `banner::pending_rows_taken` copies `view::frame`'s carve (graft MAP:496).

Sweeps keep the copies equal. The precedent for doing without copies is `graft_ui::picker::picker_box((x, y, w, h))` (ui MAP:50), which gives the same box in plain numbers. The purity guard forbids only `ratatui` itself (graft MAP:957).

## Read first
- `docs/reviews/m8-architecture.md` M3.
- ARCHITECTURE.md:260–280.
- ui MAP:50, :121–123, :160–161, and the `preview_box` entry (grep it).
- graft MAP:352, :496, :590–591, :950, :957.
- ADR-0058 §4 (grep `mirror`).
- Then the scout file.

## Scout (≤1 page)
For each of the four mirrors:
- the copy's fn and its source fn, with `file:line`
- whether the two compute the same thing on every input, or the copy differs on purpose. Quote any comment that says it differs.
- the tests whose **only** job is to assert copy == source, named
- the render-and-read-back tests that check behaviour, named. These stay.

## Scope
In:
- **graft-ui** gains plain-number forms next to each source, beside `picker_box`.
  - The type is `pub type Cells = (u16, u16, u16, u16); // x, y, width, height`, at the crate root.
  - Each rect form is a thin wrapper over the existing `Rect` fn, so there is exactly one body.
- **graft**:
  - `update` calls the new fns, and the copies are deleted.
  - `pending_rows_taken` stays the single fn, and `view::frame`'s carve calls it. `view` may read `update`, and the reverse stays forbidden.
- Delete the copy==source tests. Keep every behavioural test.
- Remove ARCHITECTURE.md:267–276's test-only exception if nothing still needs it; otherwise say what does.
- Update both MAPs.

Out:
- Badge packing (`view::hit`), which is already single-source.
- `chrome_rows`, which is already the shared source.
- Any change to what is drawn.

**Stop rule.** If the scout finds that a copy differs from its source on purpose, keep that copy, name the difference in NOTES, and go on to the next one.

## Signatures (graft-ui; parameter types as each source takes them today, `Rect` → `Cells`)
```rust
pub type Cells = (u16, u16, u16, u16);
// detail_pane
pub fn split_cells(layout: DetailLayout, area: Cells) -> (Option<Cells>, Option<Cells>);
// graph_screen::layout: visibility only, if it is crate-private today
pub fn tour_rows(/* as today */) -> u16;
// preview_box
pub const MIN_BOX_ROWS: u16;
pub fn dock_cells(area: Cells) -> Option<Cells>;
```

Removed from graft:
- `update::geometry::detail_split`
- `tour::card_rows`'s own arithmetic: the fn may remain as a one-line call
- `update`'s `MIN_BOX_ROWS` and `BoxRect` copy

## Acceptance
- [ ] Each deleted copy is named in NOTES with the graft-ui fn that replaced it, and `rg` shows no second body.
- [ ] The behavioural tests the scout named all pass, each named in NOTES. They include `a_click_lands_on_the_button_the_widget_drew`, the detail-split mouse tests and the tour-card row tests.
- [ ] `update::tests::purity::update_cannot_reach_the_world` passes unchanged.
- [ ] Every deleted copy==source test is listed in NOTES with the one-line reason it can no longer fail.
- [ ] Clicks match the drawn screen at 40, 80 and 120 columns, by the width sweeps the scout named. Name them.

## Touches
- graft-ui `detail_pane`, `graph_screen::layout`, `preview_box`
- graft `update::geometry`, `update::tour`, `update::banner`, `view::frame`
- mouse hit-tests
- ARCHITECTURE.md:267–276
- ADR-0058's list of mirrors, as a line in NOTES for the conductor
- both MAPs

No keymap or capture change: nothing drawn moves. The captures do not need re-shooting, and NOTES says so.

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`.

## Size
About 65k tokens.
