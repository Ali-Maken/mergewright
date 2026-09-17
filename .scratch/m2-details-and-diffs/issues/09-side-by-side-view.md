# side-by-side-view: `v` toggles a two-column diff over the same LineView data

Status: ready
Blocked by: 02, 06, 08

Status: draft
Build with: Sonnet — a second renderer over data `diff-widget` already produces; judged by the capture.
Needs: `diff-widget` (`diff_body`'s look and helpers to mirror: gutter, tints, syntax, `␍`), `detail-model-and-jobs` (`LineView::pair`, `DiffMode`, `DetailState.mode`), `diff-parser` (`Pair`, `pairs` — for the shape `pair` indices key on)

## Read first

- `docs/design-notes/m2-details-and-diffs.md` — New UI "Diff lines: Side-by-side" bullet; Interfaces `Pair`/`pairs()` doc comment ("side-by-side and word diff both key on it")
- `PROMPT.md` §5 Diffs (side-by-side row)
- `crates/graft-ui/MAP.md` — `detail_pane` (after `diff-widget` lands)

## Scope

In:
- `crates/graft-ui/src/detail_pane.rs`: `pub fn diff_body_side_by_side(hunks: &[HunkView], top: usize, area: Rect, buf: &mut Buffer, theme: &Theme, focused: bool) -> usize` — two halves split by a `│` (or plain rule at 16 colours), each with its own gutter (old on the left half, new on the right), `LineView.pair` groups removed/added runs so a pair's rows line up; an unpaired removed line leaves the right half blank on that row and vice versa; word tint, syntax and `␍` render exactly as `diff_body` does, mirrored per half; hunk-heading pin spans the full width above both halves.
- `crates/graft/src/update/keys.rs` (or wherever `w`/`W` live once `detail-keys-and-focus` adds them — if that ticket hasn't landed yet, add the `v` arm directly to the existing key dispatch, not inside a module that doesn't exist yet): `v` while `focus != Graph` toggles `AppState.detail.mode` between `Unified`/`SideBySide`; pure state flip, no job (the cached `DiffView` already has everything both renderers need).
- `DetailPane`'s body dispatch (`detail-pane-frame`) picks `diff_body` vs `diff_body_side_by_side` on `view.mode` — a one-line edit to that `match`, cite it if it has moved.
- Tests: fixture hunk with one 2-removed/2-added pair and one unpaired context line — assert row alignment (paired rows share a screen row, unpaired blank half). Key test: `v` flips `mode`, a second `v` flips back, `Graph` focus ignores it. MAP.md line.
- Capture: the same two-hunk fixture in both modes side by side, to `docs/shots/m2-side-by-side.txt`, ≤ 200 KB.

Out: word/syntax computation (already in `LineView`), resize, anything but the `v` key.

## Signatures

```rust
// crates/graft-ui/src/detail_pane.rs
pub fn diff_body_side_by_side(hunks: &[HunkView], top: usize, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer, theme: &Theme, focused: bool) -> usize;
```

## Acceptance

- [ ] A paired removed/added run renders on the same screen rows, left/right; an unpaired context or added-only line leaves the other half blank on that row, not shifted.
- [ ] Word tint, syntax fg, `␍`, 16-colour fallback all match `diff_body`'s look on the same fixture (assert same style at the equivalent cell).
- [ ] `v` toggles `AppState.detail.mode`; no `Command` emitted; ignored when `focus == Graph`.
- [ ] MAP.md: `graft-ui` (`detail_pane` line amended).
- [ ] Capture at `docs/shots/m2-side-by-side.txt`, ≤ 200 KB; user judges by eye.

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet` · purity guard (the `v` arm touches `update`).

## Size

Two crates: `graft-ui` (~150 lines), `graft` (~15 lines, one key arm). Tests ~120. ≤ 20 tool calls.
