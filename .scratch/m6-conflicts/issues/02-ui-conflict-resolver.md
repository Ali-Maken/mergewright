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

**Note on the data shape actually built**: the signatures block above (`Layout { rows, block_row, columns }`, `ScreenRow::Labels`) is what this ticket specified; the built module uses an equivalent but not identical shape (`ScreenRow` folds the labels row into `Side { side: SideKind::Label(_), .. }` rather than a separate `Labels` variant, and `Layout` carries `side_by_side: bool` rather than `block_row`/`columns` — `block_row_points_at_headers` below is satisfied by a helper over `rows`, not a stored field). This is a real deviation from the ticket's literal signatures, not called out by the scout's four corrections — flagging it here rather than silently complying or silently deviating (CLAUDE.md "Challenge the spec"). The public contract (`layout`/`row_at`/`ResolverScreen` totality, side-by-side ≥100, BASE always stacked, taken block hides its other sides) holds.

- [x] `layout` at 120 columns on a three-block doc (one open, one `Took(Theirs)`, one open with base, `base_shown: true`) — checked against the built `ScreenRow` shape (Header/Label+Side rows/Pair rows/End in the same order the ticket describes, not the literal `Labels`/`block_row` field names) — `conflict_resolver::tests::layout_wide_rows_in_order`.
- [x] Same doc at 80 columns: no `Pair` rows, stacked `Side` rows only — `layout_narrow_stacks`.
- [x] The row index of each block's header is discoverable for every block — `block_row_points_at_headers` (over `rows`, not a stored `block_row` field — see the note above).
- [x] `row_at(layout, top, r)` is offset lookup, `None` past the end — `row_at_is_offset`.
- [ ] **Not built.** Pixel/cell-exact 120×40 and 80×24 snapshots against the design note's mockup doc — `resolver_matches_mockup_120`, `resolver_matches_mockup_80` do not exist. Left unticked, not marked broken (CLAUDE.md rule 4: unverified is not broken). Structural layout is covered by the tests above; visual match to the mockup is not.
- [x] ASCII tier: widget-owned glyphs (`·` separators) respect `ascii: bool`; app-supplied text draws verbatim (sanitization is `term::sanitize`'s job) — `resolver_ascii_tier_is_ascii`, rewritten mid-build to an ASCII-safe fixture after finding the widget itself hardcoded `·` with no fallback (fixed).
- [x] Structural "only the window is touched": `resolver_draws_only_the_window` asserts cells outside `area` are untouched and rows past `top+height` are never read. **The <5ms timing claim is not asserted** — no `Instant` bound in the test. Left unticked as a sub-claim; the touched-cells property is ticked.
- [x] `columns < 60 || rows < 10` → the single too-small line, ASCII-aware — `resolver_tiny_frame_says_so`.
- [x] `HintContext::Resolver { base: false, restart: false }` whole-list wording and order match the design note; `. more` never appears; `every_context()` includes the four `Resolver` combinations — `hints::tests::resolver_hint_ladder`, `hints::tests::every_context_lists_resolver`. **The 80-column drop order is not what this ticket assumed** (`N prev`/`M mergetool` do not drop verbs at 80 cols): observed via a temporary `eprintln!` probe, the 9-entry list is 85 cells at its full rungs and only needs to drop `esc close`'s verb (`close`) to reach 79 cells at 80 columns — nothing else moves. The test now asserts the *observed* string, not the assumed one. `every_key_the_bar_draws_is_answered` still passes (499 `graft` tests) because every `Resolver` hint ships `Planned`, so the sweep finds nothing live to check here — it is not evidence this specific ladder claim was independently exercised by that test.
- [x] `bound_actions()` needed **no edit**: it is not an exhaustive match over `HintContext` (compiled clean with zero `Resolver`-related changes to `graft/src/update/keys.rs`) — contrary to what the ticket's item 2 implies.
- [~] `ChangesPane { unmerged: true }` and `Banner { conflicted: true }` gained a leading `planned("⏎", "resolve")`, consistent with the design note and Proposed change 2a. **Not ticked as exact-string-verified**: no `unmerged_rows_read_as_designed` test was written; the pre-existing `hints_text` assertion for `ChangesPane{unmerged:true}` (`" e edit  c commit  ⇥ graph  . more"`, Live-filtered) still passes unchanged, since `⏎ resolve`/`M`/`space resolved` all stay `Planned` and are filtered out of that string either way. `every_planned_hint_is_still_unbound` passes.
- [x] MAP.md updated (`§conflict_resolver` new section, `§graph_screen::hints` `Resolver` addition) — see NOTES.

## NOTES

- Scout (`https://.scratch/m6-conflicts/scout-ui.md`) used; its four corrections applied as given: (1) crate set graft-ui + exactly `graft/src/update/tests.rs`'s `classified_form`/`screen_for`, both exhaustive matches with no wildcard — confirmed by the compiler forcing both when `Resolver` was added; `keys.rs`/`bound_actions()` was **not** touched (no wildcard match there over `HintContext`). (2) Only `ChangesPane{unmerged}`'s `M mergetool`/`space resolved` were `planned()` going in; banner's `M`/`e` were already `Live` — confirmed in the pre-existing code before editing, held unchanged. (3) `ResolverScreen` modeled on `map_screen.rs`: neither it nor `journal_screen.rs`/`stash_screen.rs` draw a border — the widget draws only title/header/body content, no box chars; the app's outer chrome draws the frame. (4) The modal `whole_bar` arm's shrink ladder had never been exercised past 2-5 hints before; the resolver's 9-entry list is the first to actually hit `fit_list`'s width math at 80 columns — the drop order documented above (`esc close`→`esc`, nothing else) was observed, not assumed, after an initial wrong guess failed a test (see acceptance row above).
- `graft/src/update/tests.rs`'s two `Resolver` arms were folded into existing arms (`classified_form`: joined `Preview`'s `None` arm; `screen_for`: joined the `BranchRow|PlainRow|Marked|Preview` arm) rather than kept separate, because `clippy::match_same_arms` (`-D warnings`) rejected two arms with identical bodies even with distinct comments.
- Colour ships unverified (CLAUDE.md rule 5) — no colour capture taken.
- Status: **done**, with the two boxes above left unticked (mockup pixel-match, timing) rather than faked, and the exact-wording box for the changes-pane/banner rows marked `[~]` (behavior present and consistent, but not pinned by a named test this session wrote).

## Gates

fmt / clippy / nextest / doc, workspace-wide. `graft-ui` still compiles with no `graft-git`/`graft-ops` dependency (existing `tests/it/crate_rules` or equivalent — name it in the report).

## Size

One context; `conflict_resolver.rs` ~700 lines with tests, `hints.rs` +80. Reads MAP.md sections and the scout page, then only `hints.rs` and the frame helpers.
