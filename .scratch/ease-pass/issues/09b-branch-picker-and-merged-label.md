# branch-picker-and-merged-label — `C` opens the picker on the row's branch; `not merged` on branch tips

Status: partial (builder 2026-10-19: 5/5 ticks; gate red on `m7_walk::picker::switch_back`, a spec conflict, Question 1 in NOTES). Was: ready (split from 09, planner spec-only pass 2026-10-18, after scout `.scratch/ease-pass/scouts/09-branch-pane.md` M2; `not merged` handed to 09 by 07c's ruling, `07c-graph-rows-rest.md:12`)
Blocked by: 09-branch-pane (focus candidate, status cache, `StatusKey`). 07b (the bar's `C switch to <b>`) and 07c (labels) are already ahead of 09.
Build with: Sonnet (builder-sonnet). The decisions are below and each is pinned by a state test. The one async case (list lands after open) has its rule written out.
Needs: from 09: the focus `Candidate` (full ref name), the status cache (`StatusKey` → `BranchStatus`); `PickerState`, `open_picker` (`update/picker.rs:104`), `picker::listed` (`answer.rs:68`, resets `selected = 0` at `picker.rs:~374`), `SwitchRow.name: RefName` (full ref, `mergewright-ops/src/switch_list.rs:54`); 07c's `RowLabel` and label order; `Merged::NotYet` (`status.rs:166-169`).

## Read first
- `docs/design-notes/ease-pass.md`: Question 1 (answered a), Journeys "Teammate's status", Mock behaviour G7.
- `docs/design/02-keymap.md` §2.3 `C` (:256, ADR-0063 note :283).
- `crates/mergewright/MAP.md`: `update::picker`.
- `09-branch-pane.md` Decided; `07b-graph-rows-cursor.md` Decided (Transient `C`); review 03 Minor 10.
- The 09 scout's file, items "Picker", "`Merged` reachable".

## Decided here
- **`open_on` holds a wanted name until the list lands** (scout M2). Today the list is `Pending` at open (`picker.rs:109`), and `listed` resets `selected = 0`, so the cursor can't be placed at open.
  - `PickerState` gains `wanted: Option<RefName>`. `listed` puts `selected` on the row whose `name == wanted`, else 0, then clears `wanted`.
  - Any key that moves the selection or edits the query before the list lands clears `wanted` too: the user's move wins.
  - Both sides are full refs (`Candidate.name`, `SwitchRow.name`), so no conversion.
- **`C` on a row whose bar is `Elsewhere`** (07b) → `open_on(focus candidate)`. On HEAD's own row, and on a `PlainRow`, `C` opens as today. This closes 07b's transient (review 03 Minor 10) and its KNOWN_GAPS row, if one was filed.
- **`not merged` row label:**
  - It is drawn on a branch's tip row, after `<branch> is here`, **only from a landed `branch_status` result in 09's cache**, and only when `Merged::NotYet { .. }`. Every other `Merged` draws no label.
  - Absent while unread; never `reading…` on a row.
  - Consequence: the label shows for branches the cursor has rested on since the last refs read.
  - Reading every tip in view is **not built**. 09's `docs/perf/branch-pane.txt` has the per-window figure; widening is a conductor/user call with that number, listed in NOTES for the usability walk.

## Scope
In: app `update::picker` (`wanted`, `open_on`, `listed`), the `C` key handler's branch, `view::entries` labels (`not merged` from the cache).
Out: the pane, the cache, the job (09); hint words; picker drawing.

## Signatures
```rust
// mergewright app, update::picker
pub(crate) fn open_on(state: &mut AppState, branch: &Candidate);   // opens the picker; `wanted = Some(branch.name)`
// PickerState gains: pub(crate) wanted: Option<RefName>
// view::entries: RowLabel::NotMerged drawn from the 09 cache when Merged::NotYet
```

## Touches
The picker's tests and captures; `C`'s dead-key sweep rows (unchanged binding); 07b's bar (words unchanged; `C` now lands on the branch); 07c's label order; KNOWN_GAPS (07b's transient row closed, if filed); 14's journey count for Q1.

## Acceptance
Each tick names its test inline.
- [x] List lands after `open_on`: `selected` is the branch's row (state test with `Pending` then `listed`). — `update::picker::tests::open_on::the_list_landing_after_open_on_selects_the_branch`
- [x] List lacks the branch → `selected` 0, no error. A `j` before the list lands → `wanted` cleared, the user's position kept. — `open_on::a_list_without_the_branch_selects_the_first_row`; `open_on::a_move_before_the_list_lands_clears_the_wanted_branch` (`j` is typed in the picker, so it edits the query and clears `wanted`; `↓` before landing keeps position 0, not sara's row)
- [x] Q1: `C` on a teammate's row → picker on that branch; `⏎` → switched: 2 keys (journey test). On HEAD's row `C` opens as today. — `tests/it/branch_pane/picker_on.rs` `c_on_a_teammates_row_then_enter_switches_to_it_in_two_keys` (keys `C`, `⏎`; asserts the selected row is `refs/heads/sara/fix-toc` and `sara/fix-toc (you)` is drawn afterwards); `c_on_heads_own_row_opens_on_the_first_row_as_before`. **But** see Question 1: m7's `switch_back` is now red.
- [x] `not merged`: after the cursor rests on sara's row and the read lands, sara's tip row shows `not merged`. A merged branch shows none. Before the read, none. After a refs read clears the cache, none until re-read (state tests). — landed: `tests/it/branch_pane/merged_label.rs` `once_saras_read_lands_her_tip_row_says_not_merged` (160 wide, see NOTES); before: `before_the_read_lands_saras_row_has_no_label`; failed: `a_failed_read_draws_no_label`; merged (`MainLine`/`Into`/`NothingToMerge`), reading, failed, and cleared by `refs_landed`: `update::branch_pane::tests::not_merged_only_from_a_landed_not_yet_read`. That last one is a state test on `Reads`, not a drawn row.
- [x] Capture `docs/shots/ease-pass/picker-on-branch-120.txt`; colour unverified (rule 5). — `picker_on::capture_the_picker_on_a_teammates_branch`: the picker cursor is on `sara/fix-toc` (second row), not on `↩ before`. The look is the user's to judge.

## NOTES (builder 2026-10-19)

Scout: none. The spec named the anchors, so I read `update/picker.rs`, `update/branch_pane.rs`, `view/rows/row_bar.rs` and ui `row_text.rs` directly. Not compacted.

**Built**
- ui: `BadgeKind::NotMerged { of }`. It is not a ref. It reads `not merged`, with no Nerd icon. It sorts right after branch badge `of`'s label (`row_text.rs` `labels`). Under ruling W it is the right-most label, so the first to drop.
- app `view/entries/cells.rs` `push_not_merged`: the badge is pushed last, so refs keep the indices `geometry::badge_at` maps by. It is skipped at the badge cap. It applies to local and remote-only branches (ruling R: a teammate's branch is often remote-only).
- `Reads::not_merged(branch)` counts only `Read` with `Merged::NotYet`. Matching by branch alone is exact because `refs_landed` clears the whole map.
- `update/picker/open_on.rs`: `open_on` and `opens_on`. `PickerState.wanted` is placed and cleared by `listed`. `↑`, `↓`, Backspace or a typed character clears it.
- Ratchet: the mouse code moved out of `picker.rs` into `picker/mouse.rs`. The C-gate tests moved to `picker/tests/open_on.rs`. The `over_400` row went from 535 to 503.

**Deviation**: `C`'s rule is "the focused branch (`branch_pane::focused`) is not HEAD's", not "the bar is `Elsewhere`". It agrees with the spec on `Own`, `Elsewhere` and `PlainRow` rows. On a teammate's folded or opened merge row it opens on the focus too, where the spec says "as today". Reason: the bar needs the drawn entry's fold mark. Revert if wrong.

**Question 1: spec conflict, gate red: `m7_walk::picker::switch_back`.**
- m7's `switch-back: C ⏎` (2 keys, `docs/design-notes/m7-power-tools.md:839`) relies on `↩ before` being selected at open.
- The fixture opens with the cursor on the top row, `origin/sara/login-page`, which is a teammate's row. Q1 now lands `C` there, so `⏎` tracks sara's branch.
- `tests/it/m7_walk/picker.rs:319`: expected `refs/heads/feature-a`, got `refs/heads/sara/login-page`.
- In daily use the cursor often opens on a teammate's newest row after a fetch, so `C ⏎` no longer goes back from there.
- The assertion is not changed, and `docs/shots/m7/picker-empty-query-187x50.txt` is not redrawn.
- Options:
  - (a) Q1 stands; the walk puts the cursor on HEAD's row first (more keys).
  - (b) Only after the cursor has moved since open does `C` land on the row's branch.
  - (c) `↩ before` stays selected and the teammate is only scrolled into view.

  This is the planner's or conductor's call.

**Question 2: the label depends on timing in captures.**
- Three captures drew no label in the loaded full-suite update run: `m5-ui-banner-conflict-120x40`, `m6/fix-landing-after-continue-rebase-120x40` and `m6/fix-undo-drift-way-back-120x40`. Run alone, they draw it every time (4 of 4).
- I committed the solo output (read landed, so the label is right). In a loaded full run they may go red. My last full run had them red; the solo reruns pass.
- A non-cursor row's label depends on whether a rest fell between the `j`s. That is the spec's own consequence ("branches the cursor has rested on").
- I found no line for the drift capture's `▸` row, although `settled_frame` waits for `branch_pane::settled`. So this is a question, not a defect.
- Lead for a debugger: `settled` is true whenever `focused()` is `None` (`update/branch_pane.rs` `settled`, `is_none_or`), for example before walk facts land.

**120 with the pane**: sara's label does not fit in `with_sibling`. W drops labels first, so the landed test runs at 160. For the usability walk: reading every tip in view is not built (spec; the cost is in `docs/perf/branch-pane.txt`).

**Captures redrawn**: 21 existing files under `docs/shots/` (m5, m6, m7, m8, `ease-pass/main-90`), plus the new one.
- Each diff adds only `  not merged` after a branch label, on a branch that test's cursor rested on. `git diff -U0 docs/shots | grep '^[-+][^-+]' | grep -v 'not merged'` lists only the removed old lines.
- No contact sheet was edited.

Touches:
- ui `badges`/`row_text`: every exhaustive match compiles.
- `geometry::badge_at`: indices are kept by pushing last; the `name_and_drag` captures pass.
- `update::picker`: mouse and tests.
- 07b's bar words: unchanged.
- 14's Q1 count = 2 (`picker_on`).
- KNOWN_GAPS: not read (builder rule). Closing 07b's transient row is the conductor's.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~30k.
