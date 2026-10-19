# branch-picker-and-merged-label — `C` opens the picker on the row's branch; `not merged` on branch tips

Status: ready (split from 09, planner spec-only pass 2026-10-18, after scout `.scratch/ease-pass/scouts/09-branch-pane.md` M2; `not merged` handed to 09 by 07c's ruling, `07c-graph-rows-rest.md:12`)
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
- [ ] List lands after `open_on`: `selected` is the branch's row (state test with `Pending` then `listed`).
- [ ] List lacks the branch → `selected` 0, no error. A `j` before the list lands → `wanted` cleared, the user's position kept.
- [ ] Q1: `C` on a teammate's row → picker on that branch; `⏎` → switched: 2 keys (journey test). On HEAD's row `C` opens as today.
- [ ] `not merged`: after the cursor rests on sara's row and the read lands, sara's tip row shows `not merged`. A merged branch shows none. Before the read, none. After a refs read clears the cache, none until re-read (state tests).
- [ ] Capture `docs/shots/ease-pass/picker-on-branch-120.txt`; colour unverified (rule 5).

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~30k.
