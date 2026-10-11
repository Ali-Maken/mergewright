# Usability Q2: `⏎` on the branch he is on closes the branch picker

Status: done
Blocked by: none
Build with: **Sonnet**. One key's effect on one row plus one hint variant; reuses `Tab`'s path; held by state tests;
no git write, no new rule.
Brownfield: scout done, `.scratch/m8-polish-and-ship/scouts/47-q2-picker.md` (code at 8c7b46df). Revised against it
2026-10-11 (spec-only pass): the bar clause lives in graft-ui hints (`HintContext::Picker`, no row data), so graft-ui
gains a variant; the row type is `graft_ops::switch_list::SwitchRow`; double-click is the same path; the test asserting
the removed message is rewritten; the pty walk and its capture moved to the acceptance ticket (57 owns
`m8_walk::mergewright::picker_enter_current`).
Needs: none. Beside: `usability-q1-replaced-when-undoable`, runnable side by side (file sets disjoint; shared only
`docs/design/02-keymap.md` §2.16 vs §2.11 and MAP.md files, different sections; the conductor merges those hunks).

## Read first
- `docs/reviews/m8-usability.md` Q2 (:159-161); `docs/field-notes.md:97` ("close the picker")
- `docs/design/02-keymap.md` §2.16 (:447-457), :174 (the switch rule, `already on <b>`)
- `docs/design-notes/m8-polish-and-ship/pass-3-mergewright.md` (Journeys `picker-enter-current`; Lands)
- `crates/graft/MAP.md` `## update::picker`, `## update::switch`, `## view::picker`; `crates/graft-ui/MAP.md`
  `HintContext`, `footer_hints`, `## picker`
- The scout's file. Then source, only the files it names.

## Decisions
- **`⏎` on the current row** (`SwitchRow.current`; `update/picker.rs:183`): `enter` takes `Tab`'s path
  (`show_on_graph`, :234-245: `close` + `map::seek_to(tip)`): picker closed, graph focused, cursor on HEAD's row,
  detail pane follows. The `say(… "you're on {short}")` at :184 goes. No switch, no journal entry, no status line,
  no note (a status must not read as a problem when nothing is wrong). Dirty tree or not: the check at :183 runs
  before refusal/preview, so no preview.
- **Mouse:** double-click on a row calls `enter` (`picker_mouse` :335), so it follows with no extra code; tested.
  Single click still selects.
- **Bar (graft-ui):** new unit variant `HintContext::PickerOnCurrent` (MAP: one variant per screen and state). Its list
  is `picker_list()`'s with the `⏎` clause `close` (new const `PICKER_CLOSE: &str = "close"` beside `PICKER_SWITCH`,
  `hints/lists/editors.rs:43-51`). The chooser (`graft/src/view/rows.rs:778`) picks it when the selected row is
  current; `Picker` otherwise, and while the list is loading.
- **`PickerState.note` stays** for every other message (refusals); only the `you're on` call goes.
- **Unchanged:** every other row; `Tab`; `Esc`; detached HEAD (no row has `current`); `.` Checkout's
  `already on <b>` (keymap :174); the action id (`⏎` stays the picker's switch action, never renamed, ADR-0055).
- **Keymap §2.16 `⏎` row** (this ticket edits it; the user decided): append "; on the branch you are on, closes the
  picker, cursor on HEAD, nothing written (amended 2026-10-11, usability Q2, user: 'close the picker')".

## Scope
In: graft `update/picker.rs` (`enter`, a `selected_row` lift if needed), `update/picker/tests.rs`,
`view/rows.rs` (chooser line), `view/picker.rs` only if the note plumbing needs it; graft-ui
`graph_screen/hints/context.rs`, `hints/lists/editors.rs` and the hint-list dispatch that matches `HintContext`;
keymap §2.16; TestBackend captures of the picker; MAP lines.
Out: the switch rule; other surfaces; the pty walk (`mergewright-accept`, S6); journal files (ticket 46's).

## Signatures
```rust
// graft-ui::graph_screen::hints (context.rs)
pub enum HintContext { /* … */ Picker, PickerOnCurrent, /* … */ } // + PickerOnCurrent: picker cursor on HEAD's branch
// graft-ui hints/lists/editors.rs
const PICKER_CLOSE: &str = "close";
// graft::update::picker — lift only if `enter` (:170-183) finds its row inline
impl PickerState {
    pub(crate) fn selected_row(&self) -> Option<&graft_ops::switch_list::SwitchRow>;
}
```
`enter`'s and `show_on_graph`'s signatures unchanged; `enter` calls `show_on_graph` on the current row.

## Acceptance (each box: the test named at tick time)
- [x] `update::picker::tests::enter_on_the_current_branch_closes` (rewrites `enter_on_here_writes_nothing`,
      tests.rs:418-424, whose `"you're on main"` assertion is the behaviour removed; NOTES says so): picker closed,
      focus graph, cursor on HEAD's row, `note` None, no switch command.
- [x] `…::enter_on_the_current_branch_lands_as_tab_does`: same resulting state and command as `Tab` from the same state.
- [x] `…::enter_on_the_current_branch_with_a_dirty_tree_closes_too` (no preview opened).
- [x] `…::double_click_on_the_current_row_closes`.
- [x] `…::enter_on_another_branch_still_switches`; `…::detached_head_enter_still_switches` (no row current).
- [x] graft `view` test `picker_bar_context_follows_the_row`: `PickerOnCurrent` on the current row, `Picker` on
      another, `Picker` while loading.
- [x] graft-ui `hints` test `picker_on_current_offers_close`: `footer_hints` for `PickerOnCurrent` has `⏎ close`, not
      `switch to it`; for `Picker`, unchanged. (Superseded by 46r: the clause is now `⏎ show it on the graph`,
      test renamed `picker_on_current_enter_says_what_tab_says`.)
- [x] Grep quoted in NOTES: the removed `you're on <b>` message in `crates/*/src`: no hit. (Reworded by 46r,
      review m8 g1 minor 4: the `· here` row's own `you're on it` subject remains, by design.)
- [x] TestBackend capture (rule 6, his eye; colour unverified, rule 5):
      `docs/shots/m8-q2-picker-current-row-ui-120x36.txt`: picker open on `main`, cursor on `main`'s row, bar
      `⏎ close`, no bottom-line message.

## Touches
- **Callers:** every `match` on `HintContext` (grep `HintContext::Picker` workspace-wide; exhaustive matches must take
  the new variant); `switch::switch_into` (no longer reached from the current row: tests above); `picker_mouse`
  double-click; palette's "switch branch" if it opens this picker (grep; same `enter`, so covered).
- **Screens:** the picker; the graph it lands on. Any test enumerating hint contexts (hints ladder) gains the variant.
- **Keymap / help:** §2.16 text; no key added or moved; action id unchanged. `?`/palette description of the picker's
  `⏎`, if it says "switch" only: grep, update to match §2.16.
- **Journal / undo:** nothing written (no switch command; test above).
- **Captures:** picker and hints TestBackend captures regenerate; drift summary
  `.scratch/m8-polish-and-ship/q2-shot-drift.txt`.
- **Later tickets:** `mergewright-accept` S6 walks it (3 steps, reflog unchanged).
- **Docs:** keymap §2.16; graft MAP `update::picker` line; graft-ui MAP `HintContext` line.
- **Crates:** graft, graft-ui. Files disjoint from ticket 46's (its: graft-ops `journal_list*`, graft
  `view/journal.rs`, `update/journal*`, graft-ui `journal_screen*`).
- Each checked by the named grep or test in the report.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`,
workspace-wide.

## Size
One context, ~30k tokens.

## NOTES (builder, 2026-10-11)
- Ticks, each its test: the six `update::picker::tests::current_row::*` (enter_on_the_current_branch_closes,
  …_lands_as_tab_does, …_with_a_dirty_tree_closes_too, double_click_on_the_current_row_closes,
  enter_on_another_branch_still_switches, detached_head_enter_still_switches);
  `view::rows::tests::picker_bar_context_follows_the_row`;
  graft-ui `graph_screen::hints::tests::typed_fields::picker_on_current_offers_close`;
  capture `update::picker::tests::current_row::capture_the_picker_on_the_current_row`
  → `docs/shots/m8-q2-picker-current-row-ui-120x36.txt` (his eye; colour unverified, rule 5).
- `enter_on_here_writes_nothing` removed: its `"you're on main"` note is the removed behaviour.
- Grep `you're on` in `crates/*/src`: no hit for the removed message. Remaining hits are the picker
  row's own subject `you're on it` (graft-ui `picker.rs:83,119,284`, `picker/tests.rs:106`) — the `· here`
  row label, not the `⏎` message; left as is (not this ticket's). Doc at `picker.rs:119` still lists
  `you're on` among `note` reasons: stale wording, graft-ui, flagged not fixed.
- Chooser lives in `update::picker::hint_context` (new pub fn), called from `view/rows.rs`, not inline:
  the file-size ratchet forbids `rows.rs` growing. Tests moved to new files for the same reason
  (`update/picker/tests/current_row.rs`, `hints/tests/typed_fields.rs` — the latter also took the
  existing `a_fixed_key_is_drawn_only_where_keys_go_on_as_typed`, move-only). Ratchet rows lowered.
- Coverage slot 119 for `PickerOnCurrent` (COVERAGE_SLOTS 120); sweep_screens gains its state.
- Caption row added to `docs/shots/m8-contact-sheet-captions.tsv`; contact sheet file itself not
  regenerated by any test I found — the acceptance ticket assembles it.
- Not done: `keymap/table.rs:398` description "Switch to it" and `docs/man/trib.1` unchanged (help/man
  text for `picker.switch`); updating it drifts the man page — left for the conductor.
- No compaction.
- Capture drift (added by 46r, review m8 g1 minor 10): **no existing picker or hints capture drifted.**
  `git diff --stat 5191a559~1 5191a559 -- docs/shots` lists only the new
  `m8-q2-picker-current-row-ui-120x36.txt` and one new row in `m8-contact-sheet-captions.tsv`; every capture
  test passed at the merge (`cf2b9919`, gates green). No `q2-shot-drift.txt` was written because there was
  nothing to list. 46r later changed the Q2 capture's bar (`⏎ show it on the graph`), see 46r NOTES.
