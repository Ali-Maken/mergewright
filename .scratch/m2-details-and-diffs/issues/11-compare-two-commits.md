# compare-two-commits: `x` marks a base, the pane follows the cursor as a live compare

Status: ready
Blocked by: 06, 07

Status: draft
Build with: Sonnet — a state machine over signatures already typed (`DetailSpec::Compare`, `DetailState::open`); testable transition table.
Needs: `detail-model-and-jobs` (`DetailSpec::Compare`, `DetailState::open`, `Subject::Compare` already rendered by `detail-pane-frame`), `detail-pane-frame` (header renders `Subject::Compare`, files/body reuse unchanged)

## Read first

- `docs/design-notes/m2-details-and-diffs.md` — New UI "Compare (`x`)" bullet in full
- `PROMPT.md` §4 Comparing
- `crates/graft-ui/MAP.md` — `graph_screen` (`GraphView::search`/`GraphView::notice` as the pattern for a new footer line; `GraphEntry` fields)
- `crates/graft/MAP.md` — `update` section (`AppState`, `History::selected`/`select_at`, mouse/click handling pattern already used for row selection)

## Scope

In:
- `crates/graft-ui/src/graph_screen.rs`-equivalent addition (cite the exact section from its MAP.md entry, don't touch unrelated widget code): `GraphEntry` gains `pub compare_marked: bool` (accent left rule `▏` when true); `GraphView` gains `pub compare_footer: Option<SanitizedText>` drawn where `search`/`notice` already draw their lines (same slot pattern, not a new row when one of those is also present — stack them as the existing footer already stacks `problem`/`search`/`notice`, cite that order).
- `crates/graft/src/update/detail.rs` (extends `detail-model-and-jobs`'s module): `AppState` gains `pub compare_base: Option<CompareBase>`. `toggle_compare(state, row, oid, next_id) -> Vec<Command>`:
  - `compare_base == None` → set `Some(CompareBase { oid, row })`, no command (footer/mark now show).
  - `compare_base == Some(b)` and `oid == b.oid` → clear it, no command (row un-marks).
  - `compare_base == Some(b)` and `oid != b.oid` → call `detail.open(DetailSpec::Compare { base: b.oid, head: oid }, next_id)`, **keep** `compare_base` set (the compare stays live as the cursor moves).
- Selection-move handling in `update/detail.rs` (the "selection moves while shown" arm `detail-model-and-jobs` wrote): while `compare_base.is_some()`, moving the cursor calls `toggle_compare`'s third branch (open `Compare` against the new row) instead of `open(Commit(new_id))`. This is one conditional added to that existing arm — cite its exact location as the seam if it has grown to depend on more than `compare_base`.
- `crates/graft/src/update/keys.rs`: `x` on a graph row → `toggle_compare`. `Shift+click`/`Ctrl+click` on a second row → same as `x` on that row (both branches share `toggle_compare`, no separate click-only path). `Esc` while `compare_base.is_some()` clears it before anything else `Esc` already does (closing the pane, ADR-0023) — cite the exact `Esc` arm.
- `view::rows` (or wherever `GraphEntry` is assembled): `compare_marked` true for the row at `compare_base.row`; `compare_footer` worded exactly as the New UI bullet: `compare base a1b2c3d — move, x again clears`.
- Tests: `toggle_compare`'s three branches; selection-move override while a base is set; `Esc` clearing it; footer wording; `compare_marked` on exactly one row. MAP.md lines (`graft-ui`, `graft`).
- Capture: two rows, one marked, footer showing, pane header reading `COMPARE a1b2c3d → 7f6e5d4 · N files +A −B`, to `docs/shots/m2-compare.txt`, ≤ 200 KB.

Out: the working-tree row's own diff-vs-HEAD (already `DetailSpec::WorkingTree`, `detail-model-and-jobs`), the context-menu's "Compare with this" entry (`context-actions-menu`, calls `toggle_compare` too — that ticket's job, not this one's).

## Signatures

```rust
// crates/graft-ui (GraphEntry / GraphView additions — exact module per its MAP.md)
pub struct CompareBase { pub oid: Oid, pub row: usize }   // lives in crates/graft/src/update/detail.rs; graft-ui only gets the booleans/text below
// GraphEntry: + pub compare_marked: bool
// GraphView: + pub compare_footer: Option<SanitizedText>

// crates/graft/src/update/detail.rs
pub struct CompareBase { pub oid: Oid, pub row: usize }
pub fn toggle_compare(state: &mut AppState, row: usize, oid: Oid, next_id: &mut impl FnMut() -> JobId) -> Vec<Command>;
```

## Acceptance

- [ ] `x` on an unmarked row sets `compare_base`, emits no command; `x` again on the same row clears it, emits no command.
- [ ] `x` on a different row while a base is set opens `DetailSpec::Compare { base, head }` and keeps `compare_base` set.
- [ ] Moving the cursor (`j`/`k`) while a base is set opens a fresh `Compare` against the new row each time, not `Commit`.
- [ ] `Shift+click`/`Ctrl+click` on a second row behaves exactly as `x` would on it.
- [ ] `Esc` clears `compare_base` and closes the pane in one press, matching ADR-0023's existing `Esc` order.
- [ ] `compare_marked` is true on exactly the base row; `compare_footer` reads `compare base <sha> — move, x again clears`.
- [ ] MAP.md: `graft-ui` (`GraphEntry`/`GraphView` lines amended), `graft` (`update/detail` additions).
- [ ] Capture at `docs/shots/m2-compare.txt`, ≤ 200 KB; user judges by eye.

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet` · purity guard.

## Size

Two crates: `graft-ui` (~20 lines, two fields + footer draw), `graft` (~90 lines: `toggle_compare`, the selection-move conditional, key wiring). Tests ~120. ≤ 22 tool calls.
