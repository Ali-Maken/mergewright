# detail-keys-and-focus: the pane's own keys, mouse regions, resize and its config

Status: ready
Blocked by: 06, 07

Status: draft
Build with: Sonnet — mechanical key/config wiring over signatures `detail-model-and-jobs` and `detail-pane-frame` already gave; each transition is individually testable.
Needs: `detail-model-and-jobs` (`DetailState`, `DetailFocus`, `DiffOptions`, `DetailSpec`, `Command::ReadFileDiff`/`ListFiles`, `FileListView::toggle_dir`/`move_cursor`), `detail-pane-frame` (`DetailLayout::compute`'s `override_cells`, `DetailAreas` for hit-testing)

## Read first

- `docs/design-notes/m2-details-and-diffs.md` — New UI "Detail pane" → "Keys:" bullet in full; Where I'd differ → "Keymap" row (the four new letters, `Enter`/`Tab` semantics, ADR-0023, ADR-0028)
- `ADR-0023` (Enter opens the pane; peek removed)
- `ADR-0028` (resize keys are `<`/`>`; what the focused divider is; where the keys are a no-op)
- `DESIGN.md` §2.2–2.5 (keymap conventions: how M1 keys are worded and dispatched)
- `crates/graft/MAP.md` — `update` section (`AppState` fields you touch: `detail`, `size`), `config` section in full (where `[graph]` lives, to add `[diff]` beside it)
- `crates/graft-ui/MAP.md` — `detail_pane` (after `detail-pane-frame` lands)

## Scope

In:
- `crates/graft/src/update/keys.rs` (extend the existing dispatch, don't restructure it): while `AppState.detail.shown`:
  - `Tab`/`Shift+Tab`: cycle `Graph→Files→Diff→Graph` / reverse (`DetailState`'s focus field; already typed by `detail-model-and-jobs`, this ticket writes the cycling function).
  - `Enter`: `Files→Diff`; on `Diff`/`Graph` no-op (`Graph`'s `Enter` already opens the pane per `detail-model-and-jobs`).
  - `Esc`: closes Explain first if open (`detail.explain_requested`/`view.explain` — if `explain-panel` hasn't landed yet, this arm only needs `focus = Graph; shown = false`, note the seam), else closes the pane.
  - `j`/`k`/`↑`/`↓`, `PgUp`/`PgDn`, `g`/`G`: in `Files` move `FileListView::move_cursor`/jump, issuing `ReadFileDiff` (cache hit → no command, per `detail-model-and-jobs`); in `Diff` scroll `body_top` by line/page/top/bottom, clamped to `diff_body`'s returned content length.
  - Mouse wheel: same motions, region picked by `DetailAreas` (from `detail-pane-frame`) against the click/hover row — reuse the graph's existing hover-row mapping pattern, don't invent a second one.
  - `Space`: `Files` focus, cursor on a `Dir` row → `FileListView::toggle_dir`.
  - `w` / `W` / `[` / `]`: `ignore_whitespace` toggle / `word_tint` toggle / context −1 / +1 (0..20, clamp); each re-issues `ReadFileDiff` with the new `DiffOptions` (cache key changes, so this is a normal cache miss, no new mechanism).
  - `L`: sets `max_lines = usize::MAX` for the current file, re-issues `ReadFileDiff`.
  - `y`: reuses the existing M1 `Command::CopyToClipboard` with the shown commit's full SHA (already wired for the graph row; this just makes it work with pane focus too — one dispatch condition, not new copy logic).
  - **Resize (ADR-0028):** `<` shrinks the pane, `>` grows it, 4 cells per press, acting on the **focused divider** — the one divider the current layout has: `Right` → the vertical `│` between graph and pane (cells = columns), `Below` → the horizontal `─` (cells = rows). The keys act whenever the pane is shown, whichever region (`Graph`/`Files`/`Diff`) holds focus: the pane being shown is what makes the divider exist. **No-op** (nothing changes, nothing is shown, not an error): `Tab` layout (< 100 columns — no divider), pane hidden, Map view shown. Clamp: never below the layout's minimum (48 columns / 8 rows, `detail-pane-frame`), never so large the graph keeps fewer than 40 columns (`Right`) or 6 rows (`Below`); a press already at a bound changes nothing. Divider drag (`Below`/`Right`) sets the same value directly from the pointer's column/row, same clamps. Both write into a per-session, per-orientation field (`AppState.detail.pane_size: PaneSize`), and `view::detail` passes the orientation's value to `DetailLayout::compute` as `override_cells` (`right_columns` at ≥ 160 columns, `below_rows` at 100–159, `None` below 100). Do **not** bind `+`/`-` (`+` is "unfold lanes" on the graph) or `Ctrl+arrows` (macOS takes them); `Alt+arrows` is a fallback for a later ADR, not this ticket.
  - Hints: wherever the pane's key hints already list `w W v [ ]` (`detail-pane-frame`'s footer line at ≥ 100 columns, `?` context help), add `<> resize`. Nothing at < 100 columns (no divider, no hint).
- `crates/graft/src/config.rs` (or its `types` submodule, whichever `config`'s MAP.md names): `[diff]` section — `context: u8` (default 3), `ignore_whitespace: bool` (default false), `word_tint: bool` (default true) — loaded into `DetailState::options`/`word_tint` at startup, same pattern as `[graph]`.
- `crates/graft/src/layout.rs` (new, small): `load_split(common_dir: &Path) -> Option<PaneSize>` / `save_split(common_dir: &Path, split: &PaneSize)` — file `<common_dir>/graft/layout`, two lines `right=<columns>` / `below=<rows>` (a line absent when its value is `None`); read at startup into `AppState.detail.pane_size`, written on every resize (key or drag). Failure to read/write is not fatal — log nothing that reads as an error, fall back to the percentage default, and say so in the ticket's report (not swallowed: a missing/unwritable file is an expected, not-exceptional case here, distinct from "never swallow an error"). A line that doesn't parse reads as `None` for that orientation; the other line still counts.
- Tests: one per key transition above (fixture `AppState` with `detail.shown = true`), config parse test for `[diff]`, `layout.rs` round-trip test in a temp dir. MAP.md lines.
- Capture: a short buffer dump showing a resized `Right` pane (narrower than default, after three `<`) plus the narrow `[Graph] Detail` tab strip with `Detail` active, to `docs/shots/m2-detail-keys-resize.txt`, ≤ 200 KB.

Out: `v` (`side-by-side-view`), `x` (`compare-two-commits`), `.`/right-click (`context-actions-menu`), the Explain panel's own keys (`explain-panel`), radar wording, `Alt+arrows` (not bound; ADR-0028 names it as a fallback only).

## Signatures

```rust
// crates/graft/src/update/detail.rs (extends DetailState from detail-model-and-jobs)
pub struct PaneSize { pub right_columns: Option<u16>, pub below_rows: Option<u16> }   // Default: both None → percentage defaults
impl PaneSize { pub fn for_layout(&self, layout: &DetailLayout) -> Option<u16>; }     // the orientation's override_cells; None for Tab
impl DetailState {
    pub fn cycle_focus(&mut self, back: bool);
    pub fn set_options(&mut self, options: DiffOptions, next_id: &mut impl FnMut() -> JobId) -> Vec<Command>;
    pub fn load_all(&mut self, next_id: &mut impl FnMut() -> JobId) -> Vec<Command>;
    pub fn scroll_body(&mut self, delta: isize, content_len: usize);
    /// `<` (grow = false) / `>` (grow = true) on the focused divider; `size` is the terminal (columns, rows).
    /// Returns true when `pane_size` changed (the loop then saves it). False for Tab, hidden pane, or a press at a bound.
    pub fn resize_pane(&mut self, layout: &DetailLayout, grow: bool, size: (u16, u16)) -> bool;
    /// Divider drag: set the orientation's cells from the pointer, same clamps as `resize_pane`. Same return.
    pub fn set_pane_cells(&mut self, layout: &DetailLayout, cells: u16, size: (u16, u16)) -> bool;
}
// crates/graft/src/config.rs (or config::types)
pub struct Diff { pub context: u8, pub ignore_whitespace: bool, pub word_tint: bool }   // Config::diff field
// crates/graft/src/layout.rs
pub fn load_split(common_dir: &std::path::Path) -> Option<PaneSize>;                 // None: no file
pub fn save_split(common_dir: &std::path::Path, split: &PaneSize) -> std::io::Result<()>;
```

## Acceptance

- [ ] `Tab`/`Shift+Tab` cycle in order and wrap; `Enter` moves `Files→Diff` only.
- [ ] `Esc` closes the pane (no Explain in this ticket's scope, or closes Explain first if that seam is already there — state which).
- [ ] `w`/`W`/`[`/`]`/`L` each change exactly the intended `DiffOptions` field and issue one `ReadFileDiff`; repeating the same toggle is a normal cache hit (no command) after the first round-trip.
- [ ] `Space` on a `Dir` row toggles it and only it.
- [ ] `<`/`>` in `Right` move `right_columns` by −4/+4 and leave `below_rows` alone; in `Below` move `below_rows` by −4/+4 and leave `right_columns` alone; both clamped to the layout's minimum (48/8) and to the graph's minimum (40 columns / 6 rows); a press at a bound returns false and changes nothing.
- [ ] `<`/`>` are a no-op (false, state unchanged, no command, no footer notice) in the `Tab` layout, with the pane hidden, and with the Map view shown; they work with focus on `Graph` as well as `Files`/`Diff`.
- [ ] `+`/`-` and `Ctrl+arrows` do not resize anything (`+` still unfolds lanes with the pane shown).
- [ ] `PaneSize::for_layout` returns the matching orientation's value and `None` for `Tab`; `view::detail` passes it as `override_cells`.
- [ ] `[diff]` config: unset uses defaults (3, false, true); set values parse; MAP.md/config tests updated.
- [ ] `layout.rs` round-trips both values in a temp dir; a missing file returns `None` without error; a file with only `right=` leaves `below_rows` `None`.
- [ ] Pane hints at ≥ 100 columns include `<> resize`; `?` help lists it.
- [ ] MAP.md: `graft` (`update/detail` methods and `PaneSize`, `config::Diff`, `layout`).
- [ ] Capture at `docs/shots/m2-detail-keys-resize.txt`, ≤ 200 KB; user judges by eye.

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet` · purity guard on `update/detail.rs` (I/O — `layout.rs` load/save — stays in the event loop / startup, called from outside `update`, never from the pure function itself).

## Size

This is three concerns in one crate (keys/focus/scroll; resize+persistence; `[diff]` config) and is the pass's next-largest ticket after `detail-model-and-jobs`. Named seam: if the movement/focus/toggle keys (first bullet group) plus `[diff]` config pass 70% of budget, stop there, commit, and report `partial` with resize (`<`/`>`, drag, `PaneSize`) + `layout.rs` persistence as the remainder — `DetailLayout::compute` already accepts `override_cells`, so the split is clean. Target ≤ 30 tool calls for the full ticket, ~25 for the seam above.
