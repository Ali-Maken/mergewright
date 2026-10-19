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
- **Ruling on NOTES Question 1 (planner, 2026-10-19): not decided here — it goes to the user.** Two journeys he approved collide on one row:
  - m7 "Back to the previous branch: `C ⏎`, 2" (`docs/design-notes/m7-power-tools.md:671`, `:839`; walked it 3 times in the prototype, `docs/field-notes.md:73`). The journey names no starting row.
  - ease-pass Q1 (a) "on a teammate's row … list opens on that branch, `⏎` switches (2 keys)" (`ease-pass.md:293`, Journeys `:106-107`), with the bar there reading `C switch to <branch>` (accepted mock, G13).
  - On a teammate's row the same two keys can't do both. Either answer changes one approved journey, so the planner does not pick.
  - Builder's (b) and (c) are rejected, not offered:
    - (b) "only after the cursor has moved": a hidden mode. On the launch row the bar would say `C switch to sara` while `C ⏎` goes back, so the bar would lie (ux-principles: visibility of system status; same key, same meaning).
    - (c) "`↩ before` stays selected": breaks Q1 (a)'s "`⏎` switches" and the bar's words.
  - `C` keeps one meaning (keymap §2.3 :256, "open the branch picker") under both options. Only the picker's first selected row differs.
  - **Question for the user** (his words): *"When the cursor is on a teammate's branch and you press `C ⏎`, should it switch to his branch, or take you back to the branch you were on before?"*
    - **(a) Switch to his.** The bar's `C switch to <branch>` is true on his row. To go back, it's `C ⏎` from your own row or any non-teammate row. From his row you move off it first (one or more navigation keys). **Rec a**: the bar names what `C` does on the row you're looking at, and going back still takes 2 keys from your own work.
    - **(b) Go back.** `C ⏎` always goes back, from anywhere. Q1 is reverted: on his row `C` opens as today, and the bar there stops saying `C switch to <branch>` (its words change from the accepted mock). Switching to him is `C`, type, `⏎`: 3+ keys.
  - **Exact changes per answer** (nothing changes until he answers; the gate stays red and this ticket stays `partial`; no assertion is weakened in the meantime, rule 2):
    - **(a):**
      - `tests/it/m7_walk/picker.rs` `switch_back`: before `walk.open`, a setup step (not counted, not in `walk.keys`) puts the graph cursor on HEAD's row (`main`'s tip). Every assertion stays as written: `walk.keys == ["C", "⏎"]`, `<= noted_keys("switch-back")`, `head == refs/heads/feature-a`, status `on feature-a`, cursor on `target`, undo back to `main`.
      - The doc comment reads "from your own row".
      - `docs/design-notes/m7-power-tools.md:671` row and `:839` comment gain "from your own row". The count stays 2.
      - `docs/shots/m7/picker-empty-query-187x50.txt` is redrawn only if the box differs. It shouldn't, since `↩ before` is still selected.
      - A new journey test `back_from_a_teammates_row_is_c_enter_after_moving_off_it` pins the cost. Its count, measured, goes into the m7 note beside the 2.
      - Builder's `picker_on` tests stand.
    - **(b):**
      - Drop the `Elsewhere` branch of the `C` handler: `C` never calls `open_on` from a key. `open_on`/`wanted` stay for `Tab`/palette use, or go, as the conductor rules.
      - `picker_on::c_on_a_teammates_row_then_enter_switches_to_it_in_two_keys` becomes `c_on_a_teammates_row_opens_on_before`, asserting `selected` is `↩ before`.
      - `ease-pass.md` Q1 and Journeys `:107` are amended. 07b's bar words on `Elsewhere` change: a 07b reopen, with captures.
      - `switch_back` is unchanged and goes green.

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

**Deviation (goes to review)**: `C`'s rule is "the focused branch (`branch_pane::focused`) is not HEAD's", not "the bar is `Elsewhere`". It agrees with the spec on `Own`, `Elsewhere` and `PlainRow` rows. On a teammate's folded or opened merge row it opens on the focus too, where the spec says "as today". Reason: the bar needs the drawn entry's fold mark. Revert if wrong.

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

**Q2: the label was lost under load. Fixed (continuation, 2026-10-19, conductor's order).**

Three captures drew no label in a loaded run: `m5-ui-banner-conflict-120x40`, `m6/fix-landing-after-continue-rebase-120x40` and `m6/fix-undo-drift-way-back-120x40`.

**Ruled out by observation.** A slow read is not the cause. I held every `BranchStatusRead` back 1.5 s with a temporary sleep in `jobs/runner/lists.rs`, and the three captures still passed. The waits do cover a slow read.

**Cause 1, app side: a stale read counted as settled.**
- `branch_pane::settled` held a read landed under the old refs as settled while a refs read was in flight.
- When that refs read lands, `refs_landed` clears the cache. So a frame taken in between, then pumped on (`radar_answered` after `settled_frame`), lost the label.
- Made deterministic in `update::branch_pane::tests::labels::a_read_is_not_settled_while_a_refs_read_is_in_flight`. It was red before the fix and green after.
- Fix: `settled` is false while `refs_read` is `Pending` (`update/branch_pane.rs` `settled`).

**Cause 2, test side: two capture paths never waited for the read.**
- `conflict_interim/landing_shots.rs` `settled` waited only for no `reading…`/`sorting…` text. The pane is hidden there (the detail pane is open), so the read was never awaited.
- The banner capture (`conflict_interim/banner_shot.rs`) did not wait either.
- Both now wait on `branch_pane::settled`. This one is inferred from the code, not reproduced. Before 09b the race was invisible: with the pane hidden, the read changed nothing on screen.

**Ratchet**: the banner capture moved to `conflict_interim/banner_shot.rs` (row 1931 → 1838). The label tests moved to `branch_pane/tests/labels.rs`.

**Unproven**: the original under-load failure has not been seen again since the fix, but it was never reliable under load either. It is fixed at the two mechanisms above. Not proven by a loaded rerun.

**Seen, not mine (question)**: `graph_mode_keep::typed_graph_lists_the_group_after_here_at_120_and_90` failed once under load. It is a pty capture of the real binary, `docs/shots/ease-pass/help-graph-mode-120.txt` line 13. That line holds the pane's route (`─◆ wallet-release`) under the help box, not a label. It passed alone, 2 of 2. It looks like 09's pane race in the pty path (`folded_and_rested` does not wait for the pane), but I have no line for it.

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
