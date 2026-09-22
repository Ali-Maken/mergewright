# ui-conflict-resolver: the resolver screen as a pure widget — document layout with conflict cards (paired ≥100 columns, stacked below), base section, whole-file card, cursor block, notice line, its modal hint list, and the `⏎ resolve` hints on the changes pane and banner

Status: done
Blocked by: **none** — starts immediately.
Build with: (blank — brownfield in `graft-ui/src/graph_screen/hints.rs` (`HintContext`, `every_context()`, `bound_actions()`, the `planned()` entries) and the shared `frame_box` helpers; recommendation: **Sonnet** — a pure layout function with a fixed data shape and mockups to match; every row is asserted by a test, nothing to judge.)
Crates: graft-ui
Needs: none (the `SanitizedText` seam is `graft-term`; no `graft-git` type crosses — `TakeView`/`BlockState` are UI-local mirrors)

## Scout first (Sonnet `Explore`, ≤ 1 page)

- `crates/graft-ui/src/graph_screen/hints.rs`: the exact `HintContext` variants and their rows today; where `ChangesPane { unmerged }` and `Banner { skippable, conflicted }` draw `M mergetool` / `space resolved` as `planned()` (KNOWN_GAPS 558, lines ~838–843); how a **modal** context draws its whole list (ADR-0039 amendment 2 — which existing variant is the model: `Map`? `Journal`?); `every_context()` and `bound_actions()` (hand-written lists, KNOWN_GAPS 556).
- `crates/graft-ui/src/detail/`: how `WorkingTreeSection::Unmerged` rows are drawn (`FileStatusView::Unmerged { how }`) — the `how` string's source today, to be replaced by `UnmergedHow::words()` from the app in G4.
- `crates/graft-ui/src/graph_screen/frame_box.rs` (or wherever `frame_box` lives): the border/title/footer helpers the Map/Journal/Stash overlays use, and `snap_to_columns`/`column_boundaries` (ADR-0040).
- The `▏` gutter convention ticket m5/35 introduced for the cursor row — which helper draws it.
- Report which test asserts `every_key_the_bar_draws_is_answered` and how a new context registers with it.

## Read first

- `docs/design-notes/m6-conflicts.md` — Decision 1, Interfaces (the `graft-ui` block), New UI (both mockups **are** the acceptance), Proposed changes 2(a)(b)(c) (`⏎ resolve`, `B`, `d`: draw them only when `HintContext` says `base`/`restart` are bound — the app passes `false` until the keymap names them).
- `docs/design/02-keymap.md` §2.1 (modal panes draw their complete list), §2.6 (the resolver keys), §2.4 (CONFLICTS row), §2.6a (banner).
- ADR-0039 (ladder; modal = whole list, no `. more`) with its three amendments; ADR-0041 (no colour work; ASCII tier must still read).
- `docs/design/08-architecture.md` §8.2 (`ui` never imports `git`/`ops`; every repo string is `SanitizedText`).
- `crates/graft-ui/MAP.md` §graph_screen (`BannerView`, `hints`, `frame_box`), §detail, §theme; `crates/graft-term/MAP.md` §sanitize (`sanitize`, `sanitize_line`, `sanitize_path`, width rules).
- `docs/field-notes.md` — "cursor hard to see" (the `▏` rule + reversed header is the answer here, not colour).

## Scope

In:

1. **`graft_ui::conflict_resolver`** (new module): `DocView`, `RegionView`, `BlockView`, `BlockState`, `TakeView`, `ScreenRow`, `Layout`, `layout()`, `row_at()`, `ResolverScreen` (ratatui `Widget`), `SIDE_BY_SIDE_MIN = 100`. `layout` is pure and total: for every `columns ≥ 1` it returns rows; `ResolverScreen` draws **only** `layout.rows[top..top+height]` and nothing outside its `Rect`. Row shapes:
   - `Plain { line_no, text }`: `{line_no:>5} {text}` — line numbers are the document's current numbers (the app supplies `first_line` per region).
   - `Header { block }`: `── conflict {i+1} of {of} · {open|took ours|took theirs|took both} ── {keys} ──…` padded with the rule char to the width; `keys` = the takes still available (`o ours  t theirs  b both` when open; the other two when taken). The **cursor block's** header is drawn reversed and every row of the cursor block carries the `▏` gutter (ASCII tier: `|`).
   - `Pair { block, ours, theirs }` (≥100): two halves split at `(columns − gutter) / 2`, a `│` divider (ASCII `|`), each half `{text}` truncated with `…` (ASCII `~`) by display width; the first `Pair` row of a block is the labels row `OURS · {ours_label}` / `THEIRS · {theirs_label}`; a taken block shows **only** the taken side's lines, full width, as `Side` rows.
   - `Side { block, side, i }` (<100 and for BASE at every width): `OURS · {label}` label row, then the lines; then `THEIRS · {label}`, its lines; BASE (`base_shown` and `base.is_some()`) between them as `BASE · merged common ancestor`.
   - `End { block }`: the closing rule (`──…` / `┴` under the divider at ≥100).
   - `WholeFile { how, keys }`: one card at the top `{how} · {keys}` (e.g. `deleted by them · o keeps the file  t deletes it`) then the file's lines as `Plain`.
   - Title row (drawn by the widget from `doc.title`, not a `ScreenRow`): `{path} · file {i} of {n} · {open} of {total} conflicts open` at ≥100; `{path} · {i}/{n} · {open} of {total} open` below — the app builds the string, the widget draws it. `explainer` on the second row when `Some`. `notice` (`Some((text, is_error))`) replaces the explainer row while set; `is_error` is not drawn differently (ADR-0041 — the words carry it).
   - Empty/tiny: `columns < 60 || rows < 10` → the widget draws one line `resolver needs 60x10 · esc closes` and `layout` still returns rows (the app decides that no key acts).
2. **`HintContext::Resolver { base: bool, restart: bool }`** — modal: the whole list, no `. more`: `o ours  t theirs  b both  e edit  n next  N prev  ⏎ resolved  M mergetool  esc close` plus `B base` after `b both` when `base`, `d start over` before `esc` when `restart`. Ladder: at 80 columns drop `N prev` then `M mergetool` (still answered — §2.1 says modal panes answer every key; the ladder drops words, never keys), then shorten to prefixes (`ours`→`o`, per TESTING.md's rule that short forms are prefixes). `every_context()` gains the four combinations; `bound_actions()` gains the keys.
3. **Changes pane and banner hints**: `ChangesPane { unmerged: true, .. }` row becomes `⏎ resolve  M mergetool  e edit  space resolved`; `Banner { conflicted: true, .. }` gains `⏎ resolve` first. **These stay `planned()` in this group** — G4 flips them to `Live` in the commit that binds `⏎` (KNOWN_GAPS 558; `every_planned_hint_is_still_unbound` keeps the order honest). `⏎ resolve` is a `planned()` entry until the keymap names it (Proposed change 2a).
4. `crates/graft-ui/MAP.md` §conflict_resolver, §hints updated.

Out: any key handling, scrolling state, hit-testing beyond `row_at` (app, G4); colour/theme entries beyond reusing `theme.cursor`/`theme.frame` (M8); the three-column §1.5 layout (superseded — Proposed change 1; if the user rejects it, this group's `layout` gets a second shape, not a rewrite of the data).

## Signatures

```rust
// graft-ui/src/conflict_resolver.rs
pub const SIDE_BY_SIDE_MIN: u16 = 100;
pub struct DocView { pub title: SanitizedText, pub explainer: Option<SanitizedText>, pub regions: Vec<RegionView>, pub cursor_block: Option<usize>, pub base_shown: bool, pub notice: Option<(SanitizedText, bool)> }
pub enum RegionView { Plain { first_line: usize, lines: Vec<SanitizedText> }, Block(BlockView), WholeFile { how: SanitizedText, keys: SanitizedText, lines: Vec<SanitizedText> } }
pub struct BlockView { pub index: usize, pub of: usize, pub state: BlockState, pub ours_label: SanitizedText, pub theirs_label: SanitizedText, pub ours: Vec<SanitizedText>, pub base: Option<(SanitizedText, Vec<SanitizedText>)>, pub theirs: Vec<SanitizedText>, pub first_line: usize }
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum BlockState { Open, Took(TakeView) }
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum TakeView { Ours, Theirs, Both }
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum SideKind { Ours, Base, Theirs, Label(LabelOf) }
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum LabelOf { Ours, Base, Theirs }
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScreenRow { Plain { line_no: usize, text: SanitizedText }, Header { block: usize }, Labels { block: usize }, Pair { block: usize, ours: Option<usize>, theirs: Option<usize> }, Side { block: usize, side: SideKind, i: usize }, End { block: usize }, WholeFile { how: SanitizedText, keys: SanitizedText } }
pub struct Layout { pub rows: Vec<ScreenRow>, pub block_row: Vec<usize>, pub columns: u16 }
pub fn layout(doc: &DocView, columns: u16) -> Layout;
pub fn row_at(layout: &Layout, top: usize, screen_row: u16) -> Option<&ScreenRow>;   // screen_row counted from the first document row (title/explainer rows excluded)
pub const HEADER_ROWS: u16 = 2;   // title + explainer/notice
pub struct ResolverScreen<'a> { pub doc: &'a DocView, pub layout: &'a Layout, pub top: usize, pub theme: &'a Theme, pub ascii: bool }
impl Widget for ResolverScreen<'_> { fn render(self, area: Rect, buf: &mut Buffer); }

// graft-ui/src/graph_screen/hints.rs
pub enum HintContext { /* existing */ Resolver { base: bool, restart: bool } }
```

## Acceptance

Every box names its test; captures are `graft_fixtures::capture` snapshots under `crates/graft-ui/tests/shots/resolver-*.txt` (the app-level `docs/shots/` ones are G7's).

**Correction to a note this ticket carried before the continuation below**: an earlier pass of this file claimed the built `Layout`/`ScreenRow` shape diverged from the ticket's literal signatures (no `block_row` field, no `Labels` variant). Re-reading `crates/graft-ui/src/conflict_resolver.rs` during the continuation below shows that claim was **wrong** — `Layout` does carry `pub block_row: Vec<usize>` and `ScreenRow::Labels { block }` does exist, matching the signatures block above exactly. Struck here rather than left to mislead G4.

- [x] `layout` at 120 columns on a three-block doc (one open, one `Took(Theirs)`, one open with base, `base_shown: true`) yields the ticket's row order (`Plain…`, `Header 0`, `Labels 0`, `Pair` rows, `End 0`, `Plain…`, `Header 1`, `Side` rows, `End 1`, `Header 2`, `Labels`, `Pair…`, `Side{Base}` rows, `End 2`) — `conflict_resolver::tests::layout_wide_rows_in_order`.
- [x] Same doc at 80 columns: no `Pair` rows, stacked `Side` rows only — `layout_narrow_stacks`.
- [x] `block_row[i]` is the row index of block `i`'s `Header`, for every block — `block_row_points_at_headers`.
- [x] `row_at(layout, top, r)` is offset lookup, `None` past the end — `row_at_is_offset`.
- [x] **Both named defects fixed and independently pinned; the render is pinned against drift by two golden files, not against §1.5 (third continuation, 2026-09-22).**
  - **Fix 1**: `draw_header` (`conflict_resolver.rs`, was 531–551) now leads its rule with the block's `first_line` (`" {first_line} "` before the rule chars) — matches the mockup's `39`/`57`. Pinned by `conflict_resolver::tests::header_row_carries_its_block_line_number`.
  - **Fix 2**: `draw_side`'s `SideKind::Theirs` arm (`conflict_resolver.rs`, was line 631) now carries a real line number (`first_line + 1 + i`, or `first_line + 1 + ours.len() + i` after a `Took(Both)`'s own OURS lines) once the block is `Took(Theirs)` or `Took(Both)` — matches the mockup's `58`. Stays blank while `Open` (unaffected — the module doc's reason still holds: THEIRS isn't a real file line until taken). Pinned by `conflict_resolver::tests::taken_theirs_carries_its_real_line_number_open_theirs_stays_blank`, which checks both halves.
  - **The full-grid box is now a self-referential pin, per the conductor's fourth-round instruction, not a mockup match.** Two golden files, generated from the widget's own real render on the same token.rs fixture used throughout this ticket's diffing, asserted with `graft_fixtures::capture`: `crates/graft-ui/tests/shots/resolver-120.txt` (120×40) and `resolver-80.txt` (80×24) — pinned by `resolver_layout_is_pinned_120`/`resolver_layout_is_pinned_80` in `crates/graft-ui/tests/resolver_layout.rs` (renamed from the earlier `resolver_matches_mockup_*` working name, which the coordinator judged misleading once the box stopped claiming a §1.5 match). **What this box claims**: the render doesn't drift silently — a change to `layout`/`ResolverScreen`'s output touches a committed file, same contract as every other capture in the workspace. **What it does not claim**: that the render matches §1.5's ASCII art — see the "mockup measurement" NOTES entry below for why that's not a reachable target. Whether the resolver *looks* right is CLAUDE.md rule 6's call, by eye, on G7's acceptance capture; nobody ticked that here.
  - **By-eye comparison of the two golden files against §1.5's structure** (`docs/design/01-view-mockups.md` lines 145–169): row order matches exactly at both widths — title, explainer, plain line `38`, block-0 header carrying `39`, labels row (120: side-by-side `OURS · HEAD │ THEIRS · …`; 80: stacked `OURS · HEAD` then later `THEIRS · …`), pair/stacked content rows (`40`/`41` with real numbers on OURS, blank-numbered on THEIRS while open, matching the mockup's blank THEIRS margin), end rule, plain line `42`, block-1 header carrying `57` and `took theirs`, its taken-THEIRS row now carrying `58` (the fix this ticket made). Text content (labels, keys, wording) matches verbatim where the same title string is used. Two differences beyond raw column width, neither a defect: (1) the mockup's frame is cut for the page and never shows block 1's closing `End` rule before the footer; my full 40/24-row render does draw it, since nothing cuts it off. (2) the 80-column mockup uses a shorter, hand-abbreviated title (`1/2 · 2 of 3 open`) and explainer than the 120-column one; my fixture reuses the identical `DocView` (and so the identical full title/explainer text) at both widths, since that string is the app's to build, not this widget's, and picking two different strings would test the fixture, not the widget. Structure and content otherwise consistent; no wrong-logic or wrong-order finding here (CLAUDE.md rule 7 — a claim like that would need its own line citation, and none of this rises to one).
- [x] ASCII tier: widget-owned glyphs (`·` separators) respect `ascii: bool`; app-supplied text draws verbatim (sanitization is `term::sanitize`'s job) — `resolver_ascii_tier_is_ascii`, rewritten mid-build to an ASCII-safe fixture after finding the widget itself hardcoded `·` with no fallback (fixed).
- [x] Structural "only the window is touched": `resolver_draws_only_the_window` asserts cells outside `area` are untouched and rows past `top+height` are never read. **The <5ms timing claim is not asserted** — no `Instant` bound in the test. Left unticked as a sub-claim; the touched-cells property is ticked.
- [x] `columns < 60 || rows < 10` → the single too-small line, ASCII-aware — `resolver_tiny_frame_says_so`.
- [x] `HintContext::Resolver { base: false, restart: false }` whole-list wording and order match the design note; `. more` never appears; `every_context()` includes the four `Resolver` combinations — `hints::tests::resolver_hint_ladder`, `hints::tests::every_context_lists_resolver`. **The 80-column drop order is not what this ticket assumed** (`N prev`/`M mergetool` do not drop verbs at 80 cols): observed via a temporary `eprintln!` probe, the 9-entry list is 85 cells at its full rungs and only needs to drop `esc close`'s verb (`close`) to reach 79 cells at 80 columns — nothing else moves. The test now asserts the *observed* string, not the assumed one. `every_key_the_bar_draws_is_answered` still passes (499 `graft` tests) because every `Resolver` hint ships `Planned`, so the sweep finds nothing live to check here — it is not evidence this specific ladder claim was independently exercised by that test.
- [x] `bound_actions()` needed **no edit**: it is not an exhaustive match over `HintContext` (compiled clean with zero `Resolver`-related changes to `graft/src/update/keys.rs`) — contrary to what the ticket's item 2 implies.
- [x] `ChangesPane { unmerged: true }` and `Banner { conflicted: true }` rows carry a leading `⏎ resolve`, pinned by exact string on `context_list` (before the `Live` filter, since all three of `⏎`/`M`/`space` stay `Planned`) — `hints::tests::unmerged_rows_lead_with_resolve` (continuation session, 2026-09-22): `ChangesPane{unmerged:true}` → `" ⏎ resolve  M mergetool  e edit  space resolved  c commit  ⇥ graph"`; `Banner{conflicted:true, skippable:false}` → `" ⏎ resolve  c continue  a abort  M mergetool  e edit  ⇥ graph"`; `skippable:true` inserts `s skip` after `c continue`; `conflicted:false` drops `resolve` entirely. Note the exact wording differs from this ticket's own prose above (`⇥ leave` was never in the code; it has always been `⇥ graph`) — the test pins what the code says, not the ticket's paraphrase. `every_planned_hint_is_still_unbound` still passes.
- [x] MAP.md updated (`§conflict_resolver` new section, `§graph_screen::hints` `Resolver` addition) — see NOTES.

## Superseded by the fix round (conductor, 2026-09-22)

The review (`docs/reviews/m6-02-03-resolver-widget-and-operations.md`, g02
section) found 1 Critical and 5 Major here, and the fix round merged them. Three
boxes above describe code that no longer exists — read the fix round's commit,
not them:

- The **`resolver_draws_only_the_window`** box carried a **false tick**: the test
  rendered into the whole buffer, so "cells outside `area` are untouched" was
  never asserted. It now renders into a sub-`Rect` of a sentinel-filled buffer.
- The **taken-block line number** boxes pin `first_line + 1 + i` arithmetic and
  goldens that encode an off-by-one. A resolved block's `first_line` already *is*
  its first taken line, so it is `first_line + i` now and both goldens were
  regenerated. **That error came from a fix the conductor ordered**, not from the
  builder's own work.
- The cursor block no longer reverses whole — only its header does, with the
  gutter mark carrying the rest (m5/44's precedent).

Left open on purpose: Minor m2 (the `┴` join under a taken block with no divider
above it), out of the fix round's file fence.

## NOTES

- Scout (`https://.scratch/m6-conflicts/scout-ui.md`) used; its four corrections applied as given: (1) crate set graft-ui + exactly `graft/src/update/tests.rs`'s `classified_form`/`screen_for`, both exhaustive matches with no wildcard — confirmed by the compiler forcing both when `Resolver` was added; `keys.rs`/`bound_actions()` was **not** touched (no wildcard match there over `HintContext`). (2) Only `ChangesPane{unmerged}`'s `M mergetool`/`space resolved` were `planned()` going in; banner's `M`/`e` were already `Live` — confirmed in the pre-existing code before editing, held unchanged. (3) `ResolverScreen` modeled on `map_screen.rs`: neither it nor `journal_screen.rs`/`stash_screen.rs` draw a border — the widget draws only title/header/body content, no box chars; the app's outer chrome draws the frame. (4) The modal `whole_bar` arm's shrink ladder had never been exercised past 2-5 hints before; the resolver's 9-entry list is the first to actually hit `fit_list`'s width math at 80 columns — the drop order documented above (`esc close`→`esc`, nothing else) was observed, not assumed, after an initial wrong guess failed a test (see acceptance row above).
- `graft/src/update/tests.rs`'s two `Resolver` arms were folded into existing arms (`classified_form`: joined `Preview`'s `None` arm; `screen_for`: joined the `BranchRow|PlainRow|Marked|Preview` arm) rather than kept separate, because `clippy::match_same_arms` (`-D warnings`) rejected two arms with identical bodies even with distinct comments.
- Colour ships unverified (CLAUDE.md rule 5) — no colour capture taken.
- **§1.5's mockup frames are illustrative, not literal fixed-width grids — for whoever next builds a snapshot against them.** Measured with `python3`, counting exact character lengths of `docs/design/01-view-mockups.md` lines 145–156 with the outer `│`/box-drawing border columns stripped: under the "120x40" label (which implies 118 interior columns, `120 − 2` border cols), interior row widths actually run 77–79 cells and disagree with each other row to row — `38 fn refresh…` measures 78, the `39 ── conflict…` header rule measures 77, the OURS/THEIRS labels and pair rows measure 79. No single column width reproduces every row's rule-fill length exactly; the art is hand-drawn/compressed to fit the markdown page, not a captured terminal grid at the stated size. Confirmed independently across two rounds (this session's second and third continuations) before the coordinator concurred and took ownership of fixing the label. Consequence for future work: don't diff a renderer's output against §1.5's characters expecting a byte-exact match at any width — pin against the renderer's own output instead (see the acceptance box above) and judge the mockup's *structure* (row order, labelling, numbering) by eye, not its column count.
- Status: **done**. The mockup-pixel-match box is now ticked, narrowed to a self-pin per the coordinator's fourth-round instruction (see acceptance box above) rather than a §1.5 match, which this round's measurement confirmed is not a reachable target. The timing sub-claim stays unticked (no `Instant` bound, out of scope per the coordinator's second-round note). The exact-wording box for the changes-pane/banner rows stays `[~]` (behavior present and consistent, but not pinned by a named test this session wrote).

## Gates

fmt / clippy / nextest / doc, workspace-wide. `graft-ui` still compiles with no `graft-git`/`graft-ops` dependency (existing `tests/it/crate_rules` or equivalent — name it in the report).

## Size

One context; `conflict_resolver.rs` ~700 lines with tests, `hints.rs` +80. Reads MAP.md sections and the scout page, then only `hints.rs` and the frame helpers.
