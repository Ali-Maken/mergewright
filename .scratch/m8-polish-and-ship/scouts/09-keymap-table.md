# Scout 09 — keymap table and translate (code at eeaf6a19)

## Entry point (one, in a repo)
- `crates/graft/src/update/mod.rs:1101` `core_event` `Event::Key` arm. Order **before** `keys::key_event`: `credential::credential_key` (:1106), `name_prompt_claimed` (:1109), `journal::find::find_key` (:1115), harness `hand_back` check (:1118 → `Quit::HandedBack` :1131), `search::search_key` (:1127). Then `keys::key_event` (:1128) — its only non-test caller.
- `crates/graft/src/update/keys.rs:161` `key_event`: claim chain `drag_key` → `boxes_key` (confirm, preview, :61) → `rewrite_key` :77 → `finish_key` :88 → `resolver_key` :103 → picker (shown) → `banner_key` :128 → `place_claim` (blame/history) → `context_menu_key` → `picker_key` → `editor_key` :292 → Ctrl+Z suspend (:217, reads `suspend_on_ctrl_z`) → `is_undo_key` (guarded by `text_field_open`) → Esc cancel/dismiss → `pane_and_overlay_key` :280 (journal/stash/detail/map/strip) → `commit_graph_key` → `graph_key` :318.
- So "one call before any `*_key` dispatcher" must sit at mod.rs:1101, ahead of credential/name-prompt/J-find/hand_back, or those text fields bypass it (they should Pass anyway).
- **Outside a repo** (08, merged): `event_loop/places.rs:279` → `empty_state.rs:66 outside_key` — separate loop, no `AppState`, never reaches `update`. `ContextId::EmptyState` has no `context_of(&AppState)` path.
- Mouse: `update/mouse.rs`; menu acts via `context_menu::apply_context_action` (context_menu.rs:204), not synthesized keys — not translated. Comment at mouse.rs:91 says letters are "handed back down key_event"; code shows no such call (only tests call key_event: context_menu/tests.rs:70,1040).

## Text fields today
`AppState::text_field_open` state.rs:847-860: search, place prompt, J find, commit editor, stash prompt, credential, name prompt, shown picker, rewrite field, finish non-Files focus. Ticket's Pass list omits stash prompt, name prompt, J find, rewrite field, finish message.

## Existing tables
- `update/keys/bound.rs:32` `bound_actions()` → `&[(&str,&str)]`, ~139 rows, `(hint spelling, verb)` e.g. `("q","quit")`, `j/k`, `⌃A`. **Not a dispatch record and has no context** — a claim tests compare with hint tables. "Every row is answered" is not mechanically derivable from it (contradiction with acceptance 1's premise).
- `update/tests.rs:5733` `NEVER_HINTED` (key, verb, reason); test at :5766/5782; also `every_key_the_bar_draws_is_answered`, `every_planned_hint_is_still_unbound`.
- Hints: `graft-ui/src/graph_screen/hints/context.rs:47` `HintContext` (~25 variants: WorkingTreeRow, BranchRow, PlainRow, Marked, Banner, Preview, ChangesPane, Journal, Stash, Map, Picker, PaneOverOverlay, Resolver, Finish, Dragging, ConfirmBox, NamePrompt, Credential, PaneFocused, Blame, BackToBlame, History, BackToHistory, OperationLive, Rewritable, RewriteEditor, RewriteMessage); `catalogue.rs:37 every_context`.
- No `keymap` module in graft-ui (`ls crates/graft-ui/src`). No palette or tour code exists (grep "Palette" hits paint/strips only) — `ContextId::Palette/Tour` have no state.

## HintContext → ContextId (no clean target)
Dragging, ConfirmBox, NamePrompt, Credential, OperationLive, PaneOverOverlay, Marked/Rewritable/Back* wrappers have no ContextId; ContextId `Diff`, `Menu`, `Changes` vs `PaneFocused`/`ChangesPane` split unclear. ContextId has no Confirm, Credential, NamePrompt, Search, CommitEditor — text-field Pass must key off state, not ctx.

## Keys/modifiers
`graft-core/src/event.rs:94` `KeyCode` (has `Insert`, `BackTab`; no `Space` — space is `Char(' ')`), `Modifiers` :138 has `command` (no field in `KeyChord`). Shift on chars already applied (`Char('M')`). Ctrl+Z vs suspend: keys.rs:217.

## Config
`toml` 1.1.6 (graft/Cargo.toml:55). `config/types.rs:40 Keys { suspend_on_ctrl_z }` serde-typed; `config/load.rs:177` typed parse + raw `toml::Table`; `SCHEMA` :195 `("keys", &["suspend_on_ctrl_z"])`; `unknown_keys` :216 warns per entry → `[keys.graph]` today warns `UnknownKey keys.graph`. Would typed serde reject a sub-table? could not confirm (depends on `deny_unknown_fields`, not checked). Startup copies at `event_loop/startup.rs:550`; test `keys_suspend_on_ctrl_z_reaches_app_state` :640.

## Recent nearby
06/07: bar/header/cursor (no key dispatch changes seen in log subjects). 08: `outside_key`, `places.rs` loop (above).

## Could not confirm
Code-vs-doc key diff list (doc §2.3–§2.18 vs 139 bound rows) — not walked; builder/conductor must.
