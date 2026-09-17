# context-actions-menu: `.` / right-click popup — Copy SHA, Compare, Open details, (Explain stub)

Status: done
Blocked by: 06, 06b, 11
Build with: Sonnet — a small popup widget plus dispatch to actions other tickets already implement; judged by the capture and the transition tests.
Needs: `detail-model-and-jobs` (`DetailSpec::Commit`, `DetailState::open`), `compare-two-commits` (`toggle_compare`), `working-tree-row-selectable` (`AppState::on_working_tree`, `AppState::selected_spec`), M1's existing `Command::CopyToClipboard`/`Status::Copied` (see `crates/graft/MAP.md` `update` section)

## Read first

- `docs/design-notes/m2-details-and-diffs.md` — New UI "Context actions" bullet in full; "Marker column" bullet (for the conditional `Explain` item only — the item never appears until `radar-context-and-markers` sets `GraphEntry::marker`, so this ticket's tests never see it, and that's expected, not a bug); "Working tree row selected" bullet (the one-item menu on that row); coverage row "Explain | context-actions-menu, explain-panel" (this ticket's share is the menu item and the request flag only)
- `PROMPT.md` §2 (interaction conventions)
- `crates/graft/MAP.md` — `update` section: `Command::CopyToClipboard`, `Status::Copied`, `AppState::status`, `AppState::on_working_tree`, `AppState::selected_spec` (KG 326: Copy SHA already exists under `y`; this reuses the same command, not new clipboard logic)
- `crates/graft-ui/MAP.md` — `theme` (`surface-raised` token if present, else nearest chrome token), `fit`

## Scope

In:
- `crates/graft-ui/src/context_menu.rs` (new; `pub mod context_menu`): `ContextAction`, `MenuItem`, `ContextMenu` — a small popup widget: width = longest item's label + 4, `surface-raised` background, accent cursor row, anchored bottom-left of the row that opened it (or the graph's own bottom-left when there's no room below, per the New UI bullet).
- `crates/graft/src/update/keys.rs`: `.` and right-click open the menu for the row under cursor/pointer (`AppState.context_menu: Option<(usize, ContextMenu)>`, built via `ContextMenu::for_row(marked)` where `marked` is `GraphEntry::marker.is_some()` for that row — `false` until `radar-context-and-markers` lands, which is fine: the `Explain` item just doesn't show yet). On the working-tree row (`state.on_working_tree`, or a right-click on the first body screen row while it is shown — the click-to-row mapping `working-tree-row-selectable` made row-0 aware) the menu is `ContextMenu::for_working_tree()`: `Open details` only. `j`/`k`/`Enter`/`Esc`/click inside the menu; `Esc` or a click outside closes without acting.
- `apply_context_action(state, action, row, oid, next_id) -> Vec<Command>` (`oid` is `None` on the working-tree row):
  - `CopySha` → `Command::CopyToClipboard(full_sha)` (same command `y` already uses).
  - `CompareWithThis` → `compare_two_commits::toggle_compare(state, row, oid, next_id)`.
  - `OpenDetails` → `state.detail.open(state.selected_spec(), next_id)` (`Commit(oid)` on a commit row, `WorkingTree` on the working-tree row).
  - `Explain` → sets `state.detail.explain_requested = Some(oid)` (new field, default `None`, added to `DetailState`) and returns no command; **not otherwise wired** — `explain-panel` reads this field, builds `DetailView.explain`, and clears it. Say this plainly in the ticket's report, it is not a hidden failure: the item cannot appear in this ticket's own tests (no marked row exists yet), so nothing here claims Explain works end to end.
  - `CopySha`/`CompareWithThis`/`Explain` with `oid == None` cannot be reached (`for_working_tree` never lists them): `debug_assert!` and no command — a routing bug, not an error to surface.
- `view`: draw `ContextMenu` on top of the graph when `context_menu.is_some()`.
- Tests: menu open/close, cursor movement, each of `CopySha`/`CompareWithThis`/`OpenDetails` dispatched correctly, `Explain` sets the flag and nothing else, menu never lists `Explain` when `marked == false`, `for_working_tree` lists `Open details` only and `OpenDetails` there opens `WorkingTree`. MAP.md lines (`graft-ui`, `graft`).
- Capture: menu open over a row, both with and without an `Explain` item (fake `marked: true` directly in the test fixture, since the real marker isn't wired yet), to `docs/shots/m2-context-menu.txt`, ≤ 200 KB.

Out: the real `Explain` panel content (`explain-panel`), the marker column and real `marked` source (`radar-context-and-markers`), right-click hit-testing beyond "which row" (reuses the graph's existing click-to-row mapping, not a new one).

## Signatures

```rust
// crates/graft-ui/src/context_menu.rs
pub enum ContextAction { Explain, CopySha, CompareWithThis, OpenDetails }
pub struct MenuItem { pub action: ContextAction, pub label: &'static str }
pub struct ContextMenu { pub items: Vec<MenuItem>, pub cursor: usize }
impl ContextMenu {
    pub fn for_row(marked: bool) -> ContextMenu;   // Explain first when marked, then Copy SHA, Compare with this, Open details
    pub fn for_working_tree() -> ContextMenu;      // Open details only
    pub fn move_cursor(&mut self, delta: isize);
    pub fn selected(&self) -> ContextAction;
    pub fn width(&self) -> u16;
}
impl ratatui::widgets::Widget for &ContextMenu { fn render(self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer); }

// crates/graft/src/update/detail.rs (extends DetailState)
// DetailState: + pub explain_requested: Option<Oid>   (default None; explain-panel consumes and clears it)
// crates/graft/src/update/keys.rs (or update/context.rs)
pub fn apply_context_action(state: &mut AppState, action: ContextAction, row: usize, oid: Option<Oid>, next_id: &mut impl FnMut() -> JobId) -> Vec<Command>;   // oid None only on the working-tree row
```

## Acceptance

- [x] `ContextMenu::for_row(false)` omits `Explain`; `for_row(true)` lists it first; `for_working_tree()` is `Open details` alone.
- [x] `.`/right-click opens the menu anchored at the row; `Esc`/outside-click closes without dispatching.
- [x] `j`/`k` move the cursor — **clamped**, not wrapped, matching the codebase's existing convention (`FileListView::move_cursor`); `Enter` dispatches `selected()`'s action and closes the menu.
- [x] `apply_context_action(CopySha, …)` emits the same `Command::CopyToClipboard` shape as `y`; `CompareWithThis` calls `toggle_compare` (assert its first-branch effect); `OpenDetails` opens the commit, or `WorkingTree` on the working-tree row; `Explain` sets `explain_requested` and no command.
- [x] `width()` is the longest label's cell width plus 4.
- [x] MAP.md: `graft-ui` (`context_menu`), `graft` (`AppState::context_menu`, `update::context_menu`, `DetailState::explain_requested`, `apply_context_action`).
- [x] Capture at `docs/shots/m2-context-menu.txt` (1.9 KB), ≤ 200 KB; user judges by eye.

## Notes (build)

- Spec fix applied: `apply_context_action` takes `oid: Option<Oid>` for the working-tree row (per 2026-09-17 note), and `ContextMenu::for_working_tree()` exists as specified.
- `compare-two-commits` doesn't exist as its own module — `toggle_compare` actually lives in `update::detail`; called as `super::detail::toggle_compare` from `update::context_menu::apply_context_action`.
- Deviation: `OpenDetails` builds `DetailSpec` from the `oid` parameter directly (`Some(oid) => Commit(oid)`, `None => WorkingTree`) rather than calling `state.selected_spec()` as literally written in Scope — `selected_spec()` returns `Option<DetailSpec>` (type mismatch with `open`'s `DetailSpec`), and using `oid` is also correct for a right-clicked non-selected row, whereas `selected_spec()` would silently act on the cursor's row instead of the row the menu was opened on. Flagging per "challenge the spec" — recommend the spec wording be corrected for future reference.
- Deviation: `ContextMenu`'s `Widget for &ContextMenu` impl carries no theme (per the exact signature given). It draws only labels and the reversed-video cursor row; the `SurfaceRaised` background is painted by the caller (`view/frame.rs::draw_context_menu`) directly on the buffer before the widget renders, matching the existing pattern in `graph_screen/render.rs`'s highlighted-row paint.
- `update/mod.rs` gained `pub(crate) fn context_menu_area(...) -> Option<(u16,u16,u16,u16)>` (a plain-tuple wrapper) alongside the private `Bounds`-returning `context_menu_bounds`, so `view/frame.rs` (outside `update`) can get the menu's rectangle without the private `Bounds` type crossing the module boundary.
- Session note: this ticket ran across a context compaction (a conversation summary was generated mid-build). All work after resuming was verified against gates from a clean state (fmt/clippy/build/test all rerun green), not merely assumed correct from the summary.

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet` · purity guard.

## Size

Two crates: `graft-ui` (~95 lines), `graft` (~75 lines). Tests ~140. ≤ 20 tool calls.
