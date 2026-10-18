# clash-screen-look — the clash screen as the mock: inline file, only clashes split, `?` vs `◀ ▶ ◆`

Status: ready (ids revised 2026-10-17 after ease-vocabulary's scout; clash bar states named after 03b, 2026-10-17; Needs fitted to the 05/05b split, 2026-10-17; `ClashBar { at, roles }` and file-name shortening after 03r, Q16 acceptance line, 2026-10-17; amended 2026-10-18 after review ease-pass-05-10: Minor 8 question for the user, Minor 11 split-first, 05s/05t knock-ons; this group's own scout not yet run)
Blocked by: 05-resolver-picks, 05b-merge-review-at-y, 03-ease-vocabulary, 03b-ease-vocabulary-rest, 03r-vocabulary-fixes, 05s-merge-screens-fixes (`open_clashes`, the review's door the review row's click goes through), 05t-merge-screens-ratchet (renames the type this group draws rows from); **the user's answer to the question below** (the M6 row word)
Build with: (blank — brownfield; conductor sets after the scout)
Needs: from 05: `Picks`, `BlockAt`, `next_open_in_file`, `next_open`, resolver state after a pick; from 05b: `resolver.review` (`y`) for the list's review row, `FileState::{Open { open }, Solved, Done}` and `file_state` (graft-ops) — the list's `?`/`✓` and the review row come from it, never from "no markers"; from 05s: `FileState::open_clashes` (a row's `N open`; a whole-file card counts 1); from 05t: graft-ui `FileRowLook` (was `conflict_resolver::FileState`); from 03b/03r: `HintContext::Clash(ClashBar { at: ClashAt, roles: SideRoles })` — `ClashAt::{Open, Picked, FileSolved { next: SanitizedText }, AllSolved}`, `ClashBar::is_merge()` (merge-ness read from `roles`; there is no `ClashOp`); the merge `AllSolved` words are 05b's `y review the merge …`; `FileSolved { next }` takes the **full path** and the bar draws its last segment (03r); hint verbs are `Cow<'static, str>`.

## Question for the user (review 05-10 Minor 8 — conductor asks before this group builds)
Words he accepted in the mock change in the build, so they go back to him:

> Clash screen, file list: a file with every clash picked (not yet committed) reads `✓ done` in your mock, `✓ solved` in the build — which keeps `done` for a file git already took whole. Which?
> **a)** `done` for both, as the mock · **b)** `solved` for picked, `done` for taken whole, as built

Planner's lean: **a** — in a merge both commit at the same `y`, and nothing he does differs between them. Whichever he picks: the M6 bullet and the acceptance line below use it; `CONTEXT.md` `Solved (file)` (05t) gets its "Shown" line edited to match.

## Read first
- `docs/design-notes/ease-pass.md`: Prior art rows "Clash layout" → "Undo a take", Journeys merge-* (the landing on the first clash), Mock behaviours M5–M9, M11, M13, M14, New UI (clash screen), Questions 7, 12, 16, 17 (all answered a; :254).
- `docs/design/01-view-mockups.md` §1.5, §1.13. `docs/design/02-keymap.md` §2.6. ADR-0043 §1, §4; ADR-0044 §1.
- `crates/graft-ui/MAP.md`: `conflict_resolver` (all, `SideRoles` :288), `theme` (conflict tokens), `graph_screen` (`BannerView`, `banner_line`, `::hints`). `crates/graft/MAP.md`: `view::resolver`, `update::banner`.
- `.scratch/ease-pass/issues/03b-ease-vocabulary-rest.md` "Clash screen" and NOTES deviations 2–4; `03r-vocabulary-fixes.md` Signatures; `05-resolver-picks.md` and `05b-merge-review-at-y.md` "Decided here"; `05c-merge-review-rest.md` NOTES (the live bar, `view/rows/clash_bars.rs`, `view/resolver/file_list.rs`); `05s-merge-screens-fixes.md` Decided (the door).
- `docs/reviews/ease-pass-05-10.md` Minors 8, 11.
- The merge mock's visible text (clash screen only).
- The scout's file.

## Scout questions (≤1 page)
- Which arrangement code (`Arrangement`, `three_panes`, `THREE_PANE_MIN`) can be retired and which tests/captures pin it.
- How a taken block is drawn today and what `RegionView`/`BlockView` hold (result text, side texts, take).
- Where the resolver's header and the banner words (`merging …`) come from; what the list column shows for files git combined.
- **Same kind of mismatch as V's scout:** whether any of the clash screen's words are drawn by screen code in `conflict_resolver` rather than `resolver_list` (the finish row `✓ every file done — c …`, notices); those stay this group's screen code, keys spelled through the keymap. What a click on the finish row (`finish_row_at`) dispatches today — an id or a direct call. Where `view::resolver` picks today's `HintContext::Resolver { .. }`, and which of its states `Clash` replaces (05c already picks `Clash { FileSolved | AllSolved }` in `view/rows/clash_bars.rs`). Where the resolver already holds the operation's `SideRoles` (the value `ClashBar.roles` takes).
- **Size (review 05-10 Minor 11):** which of the files at the 400-line ceiling this group edits — graft-ui `conflict_resolver/files.rs` (397), graft `update/resolver.rs` (398) — and the split line in each.

## Scope
In (graft-ui `conflict_resolver`, `graph_screen` banner words; graft `view::resolver`, including the bar's `HintContext` choice). No edits to hints files.
- **Split first, in its own commit (review 05-10 Minor 11; split, don't raise):** graft-ui `conflict_resolver/files.rs` (397) — move `FileRowLook` and its glyph/word mapping into `conflict_resolver/files/row_look.rs` before the list's new rows go in. graft `update/resolver.rs` (398), only if the review row's click lands there — move the mouse/click arms into `update/resolver/click.rs` first. Each file ≤400 after; no `over_400.txt` row raised or added.
- **Layout (New UI):** list column 30 at ≥100 cols, the file pane beside it; below 100 the list is today's strip and the sides stack (today's). **Inline at every width ≥100**: the three-pane arrangement is retired (§1.5 edit, approved 2026-10-17 — text in NOTES for the conductor).
- **M5:** header `merging <from> into <into>` (and the banner's words, §1.13 edit). Rebase/cherry-pick/revert/stash keep their own header words (Q12 a: same look, own names). Every clash stop lands here (Q17 a, 05b).
- **M6:** list column heading `Clashing files`; each row from `FileState`: `Open { open }` → `?` with `N open` (N = `open_clashes`; a whole-file card: `?` with today's card words); `Solved` and `Done` → `✓` with `done` (**the word is the user's answer above**: a → `done` for both; b → `solved` for `Solved`, `done` for `Done`). Last row `✓ Review the merge y` once no file is `Open` (merge; `y` spelled from `resolver.review`); `Combined by git (N)` section listing the files the merge changed without clashing. Q12 a, non-merge operations: the last row keeps today's finish words with `c` (go on), and no `Combined by git` section unless the scout shows the read is the same.
- **M7:** file title; plain lines numbered; an open clash marked `?` and drawn as two columns `yours · <into>` (left, `Ours` token) | `theirs · <from>` (right, `Theirs` token), its lines unnumbered; the clash under the cursor outlined (box drawing).
- **M8:** a taken block shrinks to its result lines, marked `◀` (kept yours), `▶` (took theirs), `◆` (kept both), with the words `kept yours` / `took theirs` / `kept both`. A re-take **replaces** the take (05, Q16 a — a departure from the mock, which refused with `already taken · u undoes it`) and redraws with the new mark.
- **M9:** a solved file stays on screen with `✓ Every clash in <f> is solved.` and the next-step hint (mock words).
- **M11 — the bar:** `view::resolver` picks `HintContext::Clash(ClashBar { at, roles })` from resolver state: `Open` (cursor on an open clash), `Picked` (cursor on a taken block, the file still has open clashes), `FileSolved { next }` (this file solved, `next` = the first file with an open clash, **full path** — the bar draws its last segment, 03r), `AllSolved` (no `Open` file in the operation); `roles` = the operation's `SideRoles`, the same value the resolver names its sides with (merge, rebase, cherry-pick, revert, stash). Words are 03b's/03r's (and 05b's for merge `AllSolved`); never edited here. NOTES says which `Resolver { .. }` states anything still picks (e.g. list focus); if none, list the variant for retirement (conductor's readability item). **M13:** `e` hint shown; `$EDITOR` at the cursor block (today's handler). **M14:** `esc` (`app.back`) spelled "pause" by the bar at every width (03r); 05b's handler.
- Sides named by `SideNames` as today (`yours · main`, `picked · 3f2a1b7 …` in a rebase).
Out: what keys do (05, 05b, 05s), the review and look (Fr), keymap rows and hint list words (03, 03b, 03r, 05b).

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
graft app (`view::resolver`):
```rust
/// The clash screen's bar state, from the resolver's own state.
fn clash_bar(state: &ResolverState) -> ClashBar;   // name the resolver state type from the scout
```
Retired if unused after the change: `Arrangement::ThreePane`, `three_panes`, `THREE_PANE_MIN`, `FinishRow` (or kept for non-merge; say which).

## Touches
Resolver hit-tests (`file_at`, `finish_row_at` → the review row), mouse clicks on the list's last row (now `y`'s review: dispatch what `resolver.review` dispatches — 05s's door: the review opens when its read lands); the banner line on the graph (`banner_line`) and its captures; `SideRoles` words for every operation; `B` (BASE toggle) — keep working inline; ASCII glyphs for marks; the bar's choice site (`view::resolver`, 05c's `view/rows/clash_bars.rs`) and graft sweeps that build a resolver state (`sweep_screens.rs` — the `Clash` arms must still reach every `ClashAt`); every resolver capture (recaptured here, including the rebase landing 05b moved onto this screen, and 05t's renamed `fix-solved-row-120x40.txt`); `docs/design/01-view-mockups.md` §1.5, §1.13 (approved edits, text in NOTES); `CONTEXT.md` `Solved (file)` "Shown" line (the answer above); `crates/graft/tests/it/over_400.txt` (no row raised).

## Acceptance
- [ ] Split first: `conflict_resolver/files.rs` and (if edited) `update/resolver.rs` ≤400, no `over_400.txt` row raised or added — graft's ratchet test (name it).
- [ ] Fixture merge `sara/fix-toc` into `wallet-release`, `report.md` 2 clashes + `notes.md` 1 clash + 2 combined files, 120 cols: header `merging sara/fix-toc into wallet-release`; list heading `Clashing files`, `report.md` `? 2 open`, `notes.md` `? 1 open`; `Combined by git (2)` with both names.
- [ ] The open clash under the cursor shows `yours · wallet-release` left and `theirs · sara/fix-toc` right, a box outline around it, no line numbers inside; plain lines numbered.
- [ ] After `o`: block shows only the kept lines with `◀` and `kept yours`; after `t`, `▶ took theirs`; after `b`, `◆ kept both`.
- [ ] **Q16 a (where the mock refused):** cursor on a block taken with `o`, press `t` → the block redraws `▶ took theirs` with theirs' lines, and no `already taken` words appear anywhere on screen; `u` → `◀ kept yours` back with yours' lines (state + buffer test).
- [ ] Solving `notes.md`: it stays on screen with `✓ Every clash in notes.md is solved.`; list row `✓` with the word the user picked (a: `done`; b: `solved`) (from `FileState::Solved`, while git still lists it unmerged); a whole-file choice's row reads `✓ done` either way.
- [ ] All solved: last list row `✓ Review the merge y`; a click on it does what `y` does.
- [ ] Bar by state (state tests through `clash_bar`): open clash → starts `o take yours (left)`; on a taken block with clashes left → `n next open clash · u undo pick`; `docs/report.md` solved, `docs/notes.md` open → `n next file: notes.md` (`next` passed as the full path); all solved (merge) → `y review the merge`; all solved (rebase) → today's `c` words, no `y`; a rebase's `o` reads its own side's word (from `roles`, not `yours`).
- [ ] Width 150 and 200: inline (no three panes). Width 90: today's strip and stacked sides.
- [ ] Q12: rebase stop fixture: lands here (Q17 a), same marks and advance; side names `picked · <sha7> …`; last row today's `c` words.
- [ ] Captures next to the merge mock: `docs/shots/ease-pass/merge-clash-open-120.txt`, `merge-clash-taken-120.txt`, `merge-file-solved-120.txt`, `merge-all-solved-120.txt`, `merge-clash-90.txt`, `rebase-clash-120.txt`; colour unverified. Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
Large (~70k). Split point: list column + marks + header + bar choice here; inline-at-every-width retirement as a follow-on.
