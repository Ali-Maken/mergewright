# Scout 47 — picker `⏎` on the current branch

**State** `PickerState` `crates/graft/src/update/picker.rs:34-50`: `query`, `list: Loading<Arc<SwitchList>>`, `selected`, `under_preview`, `note: Option<String>` (bottom-line message, cleared on next key :138), `last_click` (double-click).

**Keys** (`shown_key` :134-158): `Esc` close, `↑/↓` rows, `Backspace`, any char typed, `⏎` → `enter`, `Tab` → `show_on_graph`; everything else swallowed; `Ctrl+C` passed through (:103). Opens on `C` (:122-131).

**`⏎`** `enter` :170-226. Current check is `row.current` on `SwitchRow` (graft_ops) at :183; then `say(..."you're on {short}")` :184 and return — picker stays up. Otherwise refusal → `switch::switch_into` :211. `say` :228 writes `picker.note`.

**Message drawn**: `view/picker.rs:59` passes `note` → `graft-ui/src/picker.rs:206-213` draws it on the box's bottom line (Warning colour) in place of the footer.

**Bar clause**: `graft-ui/src/graph_screen/hints/lists/editors.rs:43-51` `picker_list()` — static `PICKER_SWITCH "switch to it"`; context chosen `graft/src/view/rows.rs:778` → `HintContext::Picker` (`hints/context.rs:124`, a unit variant, no row data).

**Mouse**: `picker_mouse` :301-340 — single click selects; double-click same row ≤500 ms calls `enter` (:335). Same path.

**Tab precedent** for cursor landing: `show_on_graph` :234-245 = `close` + `map::seek_to(tip)`.

**Test today**: `update/picker/tests.rs:418-424` `enter_on_here_writes_nothing` asserts note `"you're on main"`.

## Contradictions
- Bar clause is not in `view::picker` but graft-ui hints; `HintContext::Picker` carries no per-row info, so `⏎ close` vs `switch to it` cannot vary per row without changing it (rows.rs:778, context.rs:124).
- Signature names `PickerRow`; `enter` works on `graft_ops::switch_list::SwitchRow` (:20, :172). `PickerRow` is the graft-ui view type.
- Existing test :423 asserts the message being removed.
- Palette "switch branch" opening the same picker: could not confirm (not checked).
