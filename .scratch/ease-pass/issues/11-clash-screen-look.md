# clash-screen-look — the clash screen as the mock: inline file, only clashes split, `?` vs `◀ ▶ ◆`

Status: ready (ids revised 2026-10-17 after ease-vocabulary's scout; this group's own scout not yet run)
Blocked by: 05-resolver-picks, 03-ease-vocabulary
Build with: (blank — brownfield; conductor sets after the scout)
Needs: `Picks`, `BlockAt`, `next_open_in_file`, resolver state after a pick (resolver-picks); the clash screen's three bar states — open clash, solved file, all solved — as V added them to `HintContext::Resolver` (or as new variants; names in V's NOTES; `resolver_list`, `graft-ui/src/graph_screen/hints/lists/resolver.rs:76`); `resolver.review` (`y`, new) for the list's review row.

## Read first
- `docs/design-notes/ease-pass.md`: Prior art rows "Clash layout" → "Undo a take", Journeys merge-* (the landing on the first clash), Mock behaviours M5–M9, M11, M13, M14, New UI (clash screen), Questions 7, 12 (answered a).
- `docs/design/01-view-mockups.md` §1.5, §1.13. `docs/design/02-keymap.md` §2.6. ADR-0043 §1, §4; ADR-0044 §1.
- `crates/graft-ui/MAP.md`: `conflict_resolver` (all), `theme` (conflict tokens), `graph_screen` (`BannerView`, `banner_line`). `crates/graft/MAP.md`: `view::resolver`, `update::banner`.
- The merge mock's visible text (clash screen only).
- The scout's file.

## Scout questions (≤1 page)
- Which arrangement code (`Arrangement`, `three_panes`, `THREE_PANE_MIN`) can be retired and which tests/captures pin it.
- How a taken block is drawn today and what `RegionView`/`BlockView` hold (result text, side texts, take).
- Where the resolver's header and the banner words (`merging …`) come from; what the list column shows for files git combined.
- **Same kind of mismatch as V's scout:** whether any of the clash screen's words are drawn by screen code in `conflict_resolver` rather than `resolver_list` (the finish row `✓ every file done — c …`, notices); those stay this group's screen code, keys spelled through the keymap. What a click on the finish row (`finish_row_at`) dispatches today — an id or a direct call.

## Scope
In (graft-ui `conflict_resolver`, `graph_screen` banner words; graft `view::resolver`):
- **Layout (New UI):** list column 30 at ≥100 cols, the file pane beside it; below 100 the list is today's strip and the sides stack (today's). **Inline at every width ≥100**: the three-pane arrangement is retired (§1.5 Proposed edit).
- **M5:** header `merging <from> into <into>` (and the banner's words, §1.13 Proposed edit). Rebase/cherry-pick/revert/stash keep their own header words (Q12 a: same look, own names).
- **M6:** list column heading `Clashing files`; each row `?` open or `✓` solved with `N open` / `done`; last row `✓ Review the merge y` once all are solved (merge; `y` spelled from `resolver.review`); `Combined by git (N)` section listing the files the merge changed without clashing. Q12 a, non-merge operations: the last row keeps today's finish words with `c` (go on), and no `Combined by git` section unless the scout shows the read is the same.
- **M7:** file title; plain lines numbered; an open clash marked `?` and drawn as two columns `yours · <into>` (left, `Ours` token) | `theirs · <from>` (right, `Theirs` token), its lines unnumbered; the clash under the cursor outlined (box drawing).
- **M8:** a taken block shrinks to its result lines, marked `◀` (kept yours), `▶` (took theirs), `◆` (kept both), with the words `kept yours` / `took theirs` / `kept both`.
- **M9:** a solved file stays on screen with `✓ Every clash in <f> is solved.` and the next-step hint (mock words).
- **M11:** bars from V's three clash states (picked by the view from resolver state, not edited here). **M13:** `e` hint shown; `$EDITOR` at the cursor block (today's handler). **M14:** `esc` (`app.back`) spelled "pause" by V's bar; P's handler.
- Sides named by `SideNames` as today (`yours · main`, `picked · 3f2a1b7 …` in a rebase).
Out: what keys do (P), the review and look (Fr), keymap and hint lists (V).

## Signatures
graft-ui `conflict_resolver`:
```rust
pub enum Mark { Open, KeptYours, TookTheirs, KeptBoth }            // `?` `◀` `▶` `◆`
impl Mark { pub fn glyph(self, ascii: bool) -> char; pub fn words(self) -> &'static str; }
pub enum ListTail { Review, Finish /* today's c row */, None }
pub struct FileListView { /* existing */ pub combined: Vec<SanitizedText>, pub tail: ListTail }
pub enum ScreenRow { /* existing variants trimmed to the inline arrangement */ Solved { path: SanitizedText } }
pub const LIST_COLUMN: u16 = 30;
pub const INLINE_FROM: u16 = 100;
```
Retired if unused after the change: `Arrangement::ThreePane`, `three_panes`, `THREE_PANE_MIN`, `FinishRow` (or kept for non-merge; say which).

## Touches
Resolver hit-tests (`file_at`, `finish_row_at` → the review row), mouse clicks on the list's last row (now `y`'s review: dispatch what `resolver.review` dispatches); the banner line on the graph (`banner_line`) and its captures; `SideRoles` words for every operation; `B` (BASE toggle) — keep working inline; ASCII glyphs for marks; every resolver capture (recaptured here); `docs/design/01-view-mockups.md` §1.5, §1.13 (Proposed edits, NOTES only).

## Acceptance
- [ ] Fixture merge `sara/fix-toc` into `wallet-release`, `report.md` 2 clashes + `notes.md` 1 clash + 2 combined files, 120 cols: header `merging sara/fix-toc into wallet-release`; list heading `Clashing files`, `report.md` `? 2 open`, `notes.md` `? 1 open`; `Combined by git (2)` with both names.
- [ ] The open clash under the cursor shows `yours · wallet-release` left and `theirs · sara/fix-toc` right, a box outline around it, no line numbers inside; plain lines numbered.
- [ ] After `o`: block shows only the kept lines with `◀` and `kept yours`; after `t`, `▶ took theirs`; after `b`, `◆ kept both`.
- [ ] Solving `notes.md`: it stays on screen with `✓ Every clash in notes.md is solved.`; list row `✓ done`.
- [ ] All solved: last list row `✓ Review the merge y`; a click on it does what `y` does.
- [ ] Width 150 and 200: inline (no three panes). Width 90: today's strip and stacked sides.
- [ ] Q12: rebase stop fixture: same marks and advance; side names `picked · <sha7> …`; last row today's `c` words.
- [ ] Captures next to the merge mock: `docs/shots/ease-pass/merge-clash-open-120.txt`, `merge-clash-taken-120.txt`, `merge-file-solved-120.txt`, `merge-all-solved-120.txt`, `merge-clash-90.txt`, `rebase-clash-120.txt`; colour unverified. Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
Large (~70k). Split point: list column + marks + header here; inline-at-every-width retirement as a follow-on.
