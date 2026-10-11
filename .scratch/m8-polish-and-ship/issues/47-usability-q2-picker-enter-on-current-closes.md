# Usability Q2: `⏎` on the branch he is on closes the branch picker

Status: ready
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
- [ ] `update::picker::tests::enter_on_the_current_branch_closes` (rewrites `enter_on_here_writes_nothing`,
      tests.rs:418-424, whose `"you're on main"` assertion is the behaviour removed; NOTES says so): picker closed,
      focus graph, cursor on HEAD's row, `note` None, no switch command.
- [ ] `…::enter_on_the_current_branch_lands_as_tab_does`: same resulting state and command as `Tab` from the same state.
- [ ] `…::enter_on_the_current_branch_with_a_dirty_tree_closes_too` (no preview opened).
- [ ] `…::double_click_on_the_current_row_closes`.
- [ ] `…::enter_on_another_branch_still_switches`; `…::detached_head_enter_still_switches` (no row current).
- [ ] graft `view` test `picker_bar_context_follows_the_row`: `PickerOnCurrent` on the current row, `Picker` on
      another, `Picker` while loading.
- [ ] graft-ui `hints` test `picker_on_current_offers_close`: `footer_hints` for `PickerOnCurrent` has `⏎ close`, not
      `switch to it`; for `Picker`, unchanged.
- [ ] Grep quoted in NOTES: `you're on` in `crates/*/src`: no hit.
- [ ] TestBackend capture (rule 6, his eye; colour unverified, rule 5):
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
