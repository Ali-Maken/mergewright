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
