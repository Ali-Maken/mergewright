# Scout 09e — sweep defects (code at 1d92394d + 11)

Paths under `crates/graft/src/` unless stated.

## 1. `/` opener
- Gate: `update/search.rs:20` — `is_search_key(key) && next.screen == Screen::Graph && next.history.is_some()`. No overlay check.
- Caller: `update/mod.rs:1121-1122` — search runs only if `search_prompt.is_some() || !modal_owns_keys()`; that runs **before** `keys::key_event`, so before any overlay arm.
- Overlays' "I am up" predicates: Map `next.map.shown` (`update/map.rs:141`); Stash `next.stash.shown` (`update/stash.rs:158` prompt, `:168` list). `update/strip.rs:17` also reads `next.map.shown`.
- **Contradiction:** `update/state.rs:906-909` doc says Map/journal/stash are deliberately *not* in `modal_owns_keys` because "the bar behind each offers `/ search`, which works there", citing test `update/tests.rs:4105` `the_bar_behind_an_overlay_draws_no_key_the_overlay_swallows` (also cited `view/rows.rs:664`). Ruling (a) makes `/` idle there; that test/doc and bar may draw `/ search` behind Map/Stash — could not confirm the bar's content without running.

## 2. R / C / O gates
- `R`: `update/keys.rs:409` (ticket says :405) — `is_rewrite_key(key) && next.graph_has_focus() && next.graph_overlay().is_none()`.
- `C`: `update/picker.rs:119-129` `opens()` — plain `C`, `screen == Graph`, `graph_has_focus()` (:125), `picker.is_none()`, `graph_overlay().is_none()`, `!modal_owns_keys()`.
- `O`: `update/strip.rs:17` — `!plain || next.map.shown || !next.graph_has_focus()` → false.
- `graph_has_focus` = `detail.focus == DetailFocus::Graph` (`update/state.rs:679-681`).
- S/J landing pattern: `update/stash.rs:175-176` sets `next.detail.focus = DetailFocus::Graph` then opens; J same at `update/journal/keys.rs:43`.
- Dispatch order: `update/keys.rs:299-307` — detail_key, map_key, journal/stash, then `strip_key` last in `pane_and_overlay_key`.

## 3. Init box / confirm (STOP check)
- Init arm is `empty_state.rs:254-261` (ticket says :110-116; :105-109 is the context pick `ConfirmInit → ContextId::Confirm`). `:256` `KeyCode::Char('n') | KeyCode::Escape => Stay` (box closed: `prompt.take()` at :252); other keys re-set the prompt (:257-259).
- Shared confirm handler `update/confirm.rs:159-198`: answers Left/Right/Tab/BackTab/Enter/`y`/Esc; raw `n` falls to `_ => {}` (:192-196) **swallowed**. STOP condition **not met**.
- Non-test readers of `empty_state.cancel`: none in `crates/` (grep: only `graft-ui/src/keymap/table.rs:496`, `graft-ui/tests/keymap_ids.txt:57`, `empty_state/sweep_tests.rs:43`). graft-ui `src/empty_state.rs:122` draws "cancel" from `app.back`, not this row.

## 4. Sweep tables
- `DEFECTS`: `update/tests/keymap_edge/sweep_tables.rs:178-188`; `SWALLOWED_BY_DESIGN`: `.../sweep_swallowed.rs:44`; `ACTING_AT_LANDING = 396`: `.../sweep.rs:29`. DEFECTS consumed at `sweep.rs:19,44,194`.
- Rows (ctx, id, chord, why):
  Map app.find `/` "search.rs:20 opens the graph search behind the Map, unseen"; Stash app.find `/` "…behind the list, unseen"; Changes graph.rewrite `R` "keys.rs:405 graph focus only; scope keymap.rs:149"; Changes graph.switch `C` "picker.rs:125 …:149"; Changes graph.overview `O` "strip.rs:17 …:149"; Diff graph.rewrite `R` "keys.rs:405 …:150"; Diff graph.switch `C` "picker.rs:125 …:150"; Diff graph.map `M` "map.rs:144 …:150"; Diff graph.overview `O` "strip.rs:17 …:150".
- Readers outside tests: none (grep `DEFECTS|SWALLOWED_BY_DESIGN` over `crates/`: only keymap_edge, `empty_state/sweep_tests.rs:26,163,265` (own table), `crates/graft/MAP.md:937`). **Ticket 11's `?`/palette does not read them** — contradicts ticket :25-26/:72 "11 reads that table".
- Start screen twin: `empty_state/sweep_tests.rs` `SWALLOWED_BY_DESIGN` :26, cancel row :43 (comment cites :256), `ACTING_AT_LANDING = 13` :81.

## 5. graft-ui row
- `graft-ui/src/keymap/table.rs:495-500` (`empty_state.cancel`, EmptyState, `[k('n')]`, "No — don't init"); `graft-ui/tests/keymap_ids.txt:57`. No `keymap/tests/*` names it (grep).

## 6. Gap M8/blame-find-drops-help-quit
- `KNOWN_GAPS.md:1145`: `?`/`q` dropped while blame find open; cites `update/keys.rs:125` (`place_claim`, :124-130) and `update/place/keys.rs:23-29` (`live_global`). Separate path (blame place claim); none of 1–5 touch it.
