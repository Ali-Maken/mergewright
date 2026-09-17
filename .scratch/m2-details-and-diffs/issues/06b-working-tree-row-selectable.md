# working-tree-row-selectable: the cursor can rest on the uncommitted-changes row, so `Enter` there opens `DetailSpec::WorkingTree`

Status: ready
Blocked by: 06
Build with: Sonnet — the selection rules are fixed below; every transition is a unit test on `update`. Nothing here decides an approach.
Needs: `detail-model-and-jobs` (`DetailSpec::WorkingTree`, `DetailState::open`, `DetailState::follow`, `AppState::detail`)

## Why

`DetailSpec::WorkingTree` and its `ListFiles` path exist (06), but the uncommitted row is synthesised by `view` (m1/12b) and `History::cursor` indexes commit rows only, so nothing can select it and `Enter` there is unreachable (KNOWN_GAPS m2/06). The fix is a one-bit selection state beside `History`, not a change to `History`'s cursor model: the virtual row is not a history row, and making it one would shift every row index (windows, requests, search hits, promotion anchors).

## Read first

- `docs/design-notes/m2-details-and-diffs.md` — New UI "Detail pane" → "Working tree row selected" bullet (the reach rules live there) and the "Spec fix (2026-09-17, working-tree row)" paragraph at the end; Sizing from M1
- `.scratch/m1-the-graph/issues/12b-uncommitted-row-wiring.md` — "Notes (built)" only: the sentinel `Row` (zero `Oid`, never a selection target) and the attachment rule (shown while the top visible history row is HEAD, or HEAD is unborn; not before refs answer)
- `KNOWN_GAPS.md` — the m2/06 section (two rows)
- `crates/graft/MAP.md` — `update` section: `AppState::hover`, `AppState::working_tree`, `AppState::refs`, `AppState::detail`, `Product::WorkingTree`; `history` section: `History::top`, `History::cursor`, `History::selected`, `History::row_at`, `History::select_at`, `History::move_cursor`, `History::scroll`, `Motion`; `view` section: the internal split line naming `entries` (`uncommitted_shown`)
- `crates/graft/MAP.md` `update::detail` section: `DetailState::open`, `DetailState::follow`
- `crates/graft-ui/MAP.md` — `RowPaint`, `GraphLine::owned`, `GraphView::rows` (the highlighted cursor row)
- `crates/graft-git/MAP.md` line 141 (`WorkingTreeCounts`)

Scout (haiku): one page on (a) where `update/keys.rs` dispatches `Motion`s to `History::move_cursor`, `Enter`, `y`, and the click-to-row arm that calls `select_at`; (b) `view/entries.rs`: how `uncommitted_shown` is computed and how the cursor row gets `RowPaint::Highlighted`. Never read those files whole.

## Scope

In:
- **`AppState`** (`crates/graft/src/update/mod.rs`): `pub on_working_tree: bool` (default false). Invariant: true only while `uncommitted_row_shown()`.
  - `pub fn uncommitted_row_shown(&self) -> bool` — m1/12b's attachment rule moved out of `view::entries::uncommitted_shown` (which becomes a call to this; its tests stay green): refs answered, `working_tree` counts on hand and not clean, and the row at `history.top` is `refs.head()` (or HEAD unborn).
  - `pub fn selected_spec(&self) -> Option<DetailSpec>` — `Some(WorkingTree)` when `on_working_tree`, else `Some(Commit(id))` for `history.selected()`, else `None`.
- **`crates/graft/src/update/selection.rs`** (new submodule; `mod.rs` gains one `mod` line): the rules, pure.
  - `move_from_graph(state, motion) -> Option<Motion>` — called before `History::move_cursor` for every graph motion key; returns the motion `History` should still apply (`None` when nothing more happens):
    - not on the row, `Up(1)`, `history.cursor == history.top`, `uncommitted_row_shown()` → `on_working_tree = true`, `None`.
    - not on the row, anything else → unchanged, `Some(motion)` (a `g`/`PageUp` that lands on row 0 stops there; one more `k` reaches the virtual row).
    - on the row, `Down(n)` → flag off, `Some(Down(n − 1))` (`Down(0)` → `None`): `j` lands on HEAD's row, which is where the history cursor already is.
    - on the row, `PageDown`/`Bottom` → flag off, `Some(motion)`.
    - on the row, `Up(_)`/`PageUp`/`Top`: `history.top == 0` → `None` (already the top of everything); else flag off, `Some(motion)` (the viewport scrolls up, the row goes with HEAD, no trap above a scrolled HEAD).
  - `reconcile(state, cursor_before) -> bool` — called once at the end of `update` for every event, with `History::cursor`'s value from the start of the event: clears `on_working_tree` when `!uncommitted_row_shown()` (the tree went clean on `Ctrl+R`, refs moved HEAD, the wheel scrolled HEAD off the top, a resize) or when `history.cursor != cursor_before` (a search hit, a click, a `G` landing). Returns true when it cleared.
  - After `move_from_graph` flips the flag, and after `reconcile` clears it, while `detail.shown`: the pane follows `selected_spec()` through the existing `DetailState::follow` path (06's "selection moves while shown" arm), keeping focus. One helper, `follow_selection(state, next_id) -> Vec<Command>`, if 06's arm isn't already reusable as-is.
- **Keys** (`crates/graft/src/update/keys.rs`, the existing arms, not a restructure): motion keys go through `move_from_graph` first; `Enter` opens `state.selected_spec()` (was `Commit(selected id)`); `y` on the row → no command, status unchanged (nothing to copy; a hint is not needed); the click-to-row arm treats the first body screen row as the virtual row while `uncommitted_row_shown()` (flag on, no `select_at`) and offsets history rows by one below it — if m1/12b's mapping already offsets (it does at the pixel tier: `PixelRows::uncommitted_on_top`), only the row-0 case is new; the search-hit arm and the click arm clear the flag before `seek`/`select_at`.
- **View** (`crates/graft/src/view/entries.rs`): when `on_working_tree`, the virtual row's `GraphLine` gets `RowPaint::Highlighted` and the history cursor's row `RowPaint::Normal`; Focus mode (`F`) treats the virtual row as on HEAD's thread (never fades it); the pixel tier's strips use the same rule (`view::strips` reads the flag the same way `entries` does — one predicate, cite it).
- Tests (`update` tests, no jobs; a state with refs answered, HEAD = row 0's id, `working_tree` = 2 staged / 1 unstaged / 1 untracked): the transition table above, one test per line; `Enter` on the row → `detail.shown` and one `Command::ListFiles` with `DiffSpec::WorkingTreeVsHead` and no `ContainedIn`; `j` back with the pane shown → the follow emits `ListFiles` for `Commit(row 0)`; `Product::WorkingTree(clean)` → flag off, pane follows to the commit; click on body row 0 → flag on; `/` hit landing elsewhere → flag off; view test: exactly one `Highlighted` row and it is the virtual one. m1/12b's eight state→view tests stay as they are.
- Capture: a `TestBackend` render at 120×30 (Unicode) of a dirty temp copy of `feature-branches` (one staged, one unstaged, one untracked file) — two frames, before and after `k` from row 0 — trimmed, to `docs/shots/m2-working-tree-row.txt`, ≤ 200 KB. The pane isn't drawn yet (`detail-pane-frame`); the capture shows the highlight moving onto the row.
- KNOWN_GAPS: close the m2/06 `Enter` row; add one row, owner **M3 working tree**: "the uncommitted row appears only while HEAD is the top visible history row (m1/12b); with a newer branch above HEAD the user scrolls HEAD to the top to reach it — M3 decides whether to pin the row above HEAD wherever HEAD sits." MAP.md lines (`graft`: `AppState::on_working_tree`, the two methods, `update::selection`).

Out: the pane's `UNCOMMITTED` header (`detail-pane-frame` builds it from `selected_spec()` + `working_tree` counts), staging/discard on the row (M3), `x` on the row (no-op; `compare-two-commits`), the context menu's one-item list on the row (`context-actions-menu`), changing where the row sits (kept as m1/12b; gap row above), the hover reveal on the row.

## Signatures

```rust
// crates/graft/src/update/mod.rs
// AppState: + pub on_working_tree: bool   // default false; true only while uncommitted_row_shown()
impl AppState {
    pub fn uncommitted_row_shown(&self) -> bool;          // m1/12b's rule, moved here; view::entries::uncommitted_shown calls it
    pub fn selected_spec(&self) -> Option<DetailSpec>;    // WorkingTree on the row, else Commit(selected id), else None
}
// crates/graft/src/update/selection.rs
pub fn move_from_graph(state: &mut AppState, motion: Motion) -> Option<Motion>;   // the motion History still applies; None = fully handled
pub fn reconcile(state: &mut AppState, cursor_before: usize) -> bool;            // end of every update; cursor_before is History::cursor's value at entry (use its real type if not usize)
pub fn follow_selection(state: &mut AppState, next_id: &mut impl FnMut() -> JobId) -> Vec<Command>;  // pane follows selected_spec() while shown; no-op otherwise
```

## Acceptance

- [ ] `k` at `cursor == top` with the row shown selects it and asks `History` for nothing; `k` at `cursor == top` with the row hidden (clean tree, or HEAD not at the top) scrolls as in M1.
- [ ] From the row: `j` → HEAD's row selected, no history request; `Down(3)` → row `top + 2`; `PageDown`/`G` → flag off and the motion applied; `k`/`PageUp`/`g` at `top == 0` → nothing; `k` at `top > 0` → flag off and the viewport scrolls up one.
- [ ] `Enter` on the row → `detail.shown`, `focus == Files`, exactly one `Command::ListFiles(_, DiffSpec::WorkingTreeVsHead, _)`, no `ContainedIn`.
- [ ] Pane shown, `j` off the row → one `ListFiles` for `Commit(row 0's id)`; pane shown, `Product::WorkingTree(Ok(clean))` → flag off and the same follow.
- [ ] `y` on the row → no command; a search hit, a click on another row, and a refs product with a different HEAD each clear the flag.
- [ ] Click on the first body screen row while the row is shown selects it; a click one row lower selects history row `top`.
- [ ] View: `RowPaint::Highlighted` on the virtual row only while `on_working_tree`; m1/12b's tests unchanged.
- [ ] `view::entries::uncommitted_shown` and `AppState::uncommitted_row_shown` are one predicate (the former calls the latter).
- [ ] Purity guard passes for `update/selection.rs`; MAP.md updated; KNOWN_GAPS rows closed/added as above.
- [ ] Capture at `docs/shots/m2-working-tree-row.txt`, ≤ 200 KB; user judges by eye.

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet` · purity guard.

## Size

One crate. `update/selection.rs` ~90 lines, `AppState` +~25, `keys.rs` +~25 (arms, not a restructure), `view/entries.rs` +~15, tests ~150. ≤ 22 tool calls. Seam for a `partial`: ship `AppState` fields/methods + `selection.rs` + the key arms with tests, and leave the click arm + view highlight + capture as the remainder.
