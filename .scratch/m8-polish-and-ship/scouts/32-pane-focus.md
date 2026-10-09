# Scout 32 — pane focus follows remap (root main, f64d50d2)

## Surprise: the edge already runs before the pane dispatchers
- `update/mod.rs:1210` `Event::Key` → `edge::key` (`update/edge.rs:17`). It translates **once, before any dispatcher** (`edge.rs:41`), then `dispatch(next, …, default.to_press(), pressed)` (`edge.rs:43`, `mod.rs:1143`). Dispatchers get the translated key; only `harness.hand_back` reads `raw` (`mod.rs:1167`).
- `banner::take_focus` is reached from `keys.rs:133` (`banner_key`), and `detail/keys.rs:159,163`, both inside `keys::key_event` — i.e. **after** translate.
- Context: `edge::context_of` (`edge.rs:71-105`): Banner when `banner_focus`; Changes/Diff when detail focused; else Graph. Scopes `graft-ui/src/keymap.rs:157-180`: Graph=[Graph,Diff,Global], Diff/Changes include Global, Banner=[Banner,Global]. So `app.*` is live in all three.
- `app.next_pane`/`app.prev_pane`: `keymap/table.rs:74-75`, context `Global`, `[TAB]`/`[BACKTAB]`. No Graph/Diff/Changes/Banner row holds ⇥/⇧⇥ (other ⇥ rows: Preview :230, Confirm :243, Picker :399, CommitEditor :440,:444). Translate logic `keymap/resolve.rs:139-163` would map `f6` → `BackTab` in Graph/Diff/Changes/Banner.
- **Contradiction:** ticket (and 20d NOTES :149-151) asserts F6 does not move focus because handlers match raw keys. Code read says the handlers get the translated key. *Could not confirm* the defect by code; no test drives a remapped pane key into banner/detail focus (below). Not run (scout doesn't build).

## Raw Tab/BackTab sites (crates/graft/src/update unless noted)
Pane cycle: `banner/focus_keys.rs:29-30` (enter banner), `:76-77`, `:124` (leave, with Esc); `detail/keys.rs:159,163`; `banner/stop/tab.rs:31,183,206` spell from `app.*`.
Own row: `stash.rs:219,403`; `confirm.rs:180`; `name_prompt.rs:275`; `finish/keys.rs:47,50,53`; `preview/keys.rs:61`; `journal/keys.rs:78` (passes); `commit_editor/keys.rs:58,62`; `picker.rs:155`; `rewrite_editor/keys.rs:182`; `place/history_keys.rs:33`; `empty_state/open_prompt.rs:29`; `cli/man.rs:302-303` (naming only).
Own-row places spelling `app.next_pane`: `finish/keys.rs:242`; graft-ui `finish_summary.rs:233` (`⇥ edit`), `name_prompt.rs:208`, `stash_screen.rs:121` (`scope`), `empty_state/bar.rs:38` (`complete`), `graph_screen/hints/keys.rs:132` `FIELD_TAB` (`fixed`, lists.rs:45 `toggle`). Check 2 decision-2 candidates.

## Hints spelling the way to the banner
graft-ui `hints/filters.rs:267` `⇥ banner`, `:271` `⇧⇥ banner`; `hints/lists.rs:312` `⇥ graph` (leave, from `app.next_pane` — no separate leave id); `graph_screen/banner.rs:183` `… ⇥ banner`; refusal words `banner/stop/tab.rs:31,183,206`; `commit_editor/refusal.rs:116` (unbound case).

## Tests / captures pinning Tab focus
- Sweep: `tests/keymap_edge/sweep.rs:161` walks `ContextId::ALL` but **exempts** `(Graph, app.next_pane/prev_pane)` as "pane closed" (`sweep_tables.rs:29-30`; Stash :34-35, Journal :38-39, Menu :89-90). Graph fixture has no banner, no open pane → the gap for check 4. Banner not in swallowed list for `app.*_pane` (`sweep_swallowed.rs:83-93`).
- Raw-key tests: `banner/tests.rs:184,810-905,1416-1455`; `keys/tab_words_tests.rs:106,176-178`; `banner/rewrite_stop_tests.rs:310`; `tests/detail_pane.rs`; words with remap: `tests/keymap_words/resolve.rs:191-208`, `undo_journal.rs:30-37` (spelling only, no press).
- it: `tests/it/conflict_interim/tab_walk.rs`, `hint_bar.rs`, `m5_walk/*`. Captures: `docs/shots/m5-ui-banner*.txt`, `m2-03-feature-branches-enter-tab-v-*.txt`.
