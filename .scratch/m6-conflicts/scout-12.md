# Scout 12 — landing after a stop

(Start paths: see scout-stuck-after-merge.md.)

## Footer draw path
- `graph_view` `crates/graft/src/view/rows.rs:40`: `problem = status::graph_problem(&state.status)` (:45); hint context = `HintContext::Preview` if preview open, else `footer_context` (:59-63).
- `footer_context` `view/rows.rs:343-359`: box context first; resolver → row context; `banner.focused` → `banner_context` → `HintContext::Banner{skippable, conflicted}` (:364-369); else `hint_context`.
- `graph_problem` `view/status.rs:65-80`: `Failed` → keys `esc dismiss`; `Sticky{reason,keys}` → keys or `esc dismiss` (DISMISS `:55`). `Done/Busy/Copied/Ready` → None.
- Precedence `crates/graft-ui/src/graph_screen/footer.rs:61-100`: **problem wins the whole row** (fit_status_line, hints dropped) > search > notice > compare > reveal > `hints_for` (`hints.rs:406`). No composite "reason · bar" today; `fit_status_line` `hints.rs:1211`.
- Coverage fns: `every_context` `hints.rs:1023`, `marked_form` `:1096`, `pane_focused_form` `:1135`, `coverage_slot` `graph_screen/tests.rs:1944`; sweep `update/tests.rs:3446`.

## Stop → Status today
- `operate::operated` Conflicted `update/operate.rs:786-815`: `live_entries=[entry]`, `Status::sticky(conflicted_line(..))` (:797, fn `:935`), `banner_focus = banner_focusable(kind)` (:806), refresh Everything.
- `banner::resumed` `update/banner.rs:561`: `Done` → `undo::operation_ended` + `Status::Done` + refresh (:573-576), **no focus/cursor change**; stop again → `Sticky{"stopped again: …", STUCK_KEYS}` (:591-611); Failed → `Sticky{…,"a abort"}`.
- `undo::operation_ended` `update/undo.rs:120-124`: clears `in_progress`, `banner_focus`; does not touch status.
- Esc: `key_event` clears any `Failed|Sticky` before anything else (`update/keys.rs:159-168`). Banner `Tab|BackTab|Esc` → `hand_the_keyboard_back` (`banner.rs:291`, fn `:265-281`) which **also resets `Sticky` → Ready** (:278-280), on Tab too.
- `Done` cleared by next key (`keys.rs:83`).

## Launch
- `launch_into_live` `update/relaunch.rs:38-66`: returns if `live_entries.is_empty()` (:43), if no `in_progress`, or not focusable; sets `banner_focus`, `Sticky{"<kind> still in progress from a previous run", LIVE_KEYS[_CONFLICTED]}`; `land_on_the_work` (:78) opens changes pane with `KeptCursor` on Unmerged/Staged. No modal/text-field check.
- Callers: first working-tree read `update/mod.rs:546-568` (only when `in_progress` Some); first journal read `update/journal.rs:104,130-132`. `next_id` type: `&mut impl FnMut() -> JobId`.

## Contradictions / unconfirmed
1. Ticket 3: "lands the same way today's relaunch does: banner focused" — keymap `02-keymap.md:280-284` says a stop with unmerged opens the resolver (P5); ticket defers that to P5. Consistent, flagged only.
2. No `HEAD`-row cursor helper found for item 5; could not confirm one exists.
3. `hand_the_keyboard_back` Sticky-clear (banner.rs:278) will not hit `Stopped` — intended by ticket, but that code's comment assumes the banner's line is Sticky.
4. `Status` lives in `update/state.rs:648`, not `update/mod.rs` as Signatures say.

## Build with
Opus — spans `graft` + `graft-ui`, footer precedence, launch timing, exhaustive-match sweeps.
