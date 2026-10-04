# Scout 09c: which keys each place claims (from the code)
Paths are relative to crates/graft/src/update/. Order in keys.rs: place_claim :203 → context_menu_key :218 → undo / Esc-cancel / Esc-dismiss-failure :250-268 → pane_and_overlay_key :269 (journal/stash go first only when detail.focus==Graph :291-293; then detail_key, map_key, journal/stash, strip) → commit_graph_key → graph_key.

## Menu: context_menu.rs:124, only when `context_menu.is_some()` (:129)
- j/Down → app.down :133; k/Up → app.up :134; Enter → app.open :135; Esc and `.` → app.back / app.menu :161
- Any other key closes the menu and returns false, so it falls through (:162-165). Not claimed: g, G, Home, End, PgUp, PgDn, `/`, n, N, Tab, BackTab, q, ?.

## Changes / Diff: detail/keys.rs:99 detail_key
- `<`/`>` (Diff row): claimed whenever the pane is shown, in any focus, Graph included :104
- Esc → app.back, claimed in three cases: escape_priority (Explain open, or compare_base set) :26-41; place show_again (trail hidden and esc_goes_back) :84 / place/trail.rs:113; pane close when shown and the trail is not hidden :155
- B, F (plain): claimed when claims_b / blamable holds and a diff cursor exists :85-92, place.rs:233, place/history/open.rs:140. Otherwise F falls through to graph Focus.
- Changes only (spec==WorkingTree, focus Files/Diff) :205-282: Enter on a conflict row (app.open) :219; M and e on a conflict row; e when there is an edit target; Space, s, d, Ctrl+A.
- Enter → app.open: always claimed when shown with focus Diff (no-op) :123; with focus Files it moves focus to Diff :128; from Graph only when selected_spec is Some :133-141.
- Tab → app.next_pane, BackTab → app.prev_pane, when shown :160-167
- Focus Files :285: j/Down, k/Up, PgDn, PgUp, g, G → app.down/up/page_down/page_up/top/bottom; Space (fold)
- Focus Diff :326: j/Down, k/Up, PgDn, PgUp, g, G (line cursor)
- Focus Files or Diff, diff_option_key :361: Space (no-op), v, w, W, [, ], L.
- Not claimed: Home and End (the second keys of app.top/bottom), `.`, `/`, n, N, q, ?. These reach graph_key even with the pane focused.
- Graph-context keys the pane claims itself: Space (graph fold), M/e/s/d (Changes), F/B, L, v, w, W, [, ].

## Map: map.rs:136 (plain keys only, :137)
- Closed: M opens it, only when graph_has_focus :144.
- Shown: M/Esc → app.back :159; k/Up :163; j/Down :167; Enter :172; `_ => true` :177 swallows every other plain key (g, G, Home, End, PgUp, PgDn, `.`, `/`, n, N, Tab, BackTab, q, ?). Ctrl/Alt keys are not claimed.

## Journal: journal/keys.rs:13 (Ctrl/Alt not claimed, :19)
- Closed: J only :29.
- Shown (shown_key :67): Tab/BackTab not claimed :76; J/Esc :83; k/Up :86; j/Down :90; n → app.next_match, N → app.prev_match (both jump to the next lost row) :95-96; Space; b; Enter :108; `_ => {}` then true :112 swallows the rest (g, G, Home, End, PgUp, PgDn, `.`, `/`, q, ?).

## Stash: stash.rs:150
- Prompt open: claims every key except Ctrl+C :158-163. With no prompt, Ctrl/Alt keys are not claimed :165.
- Closed: S only :175.
- Shown (shown_key :187): Tab/BackTab not claimed :199; S/Esc :204; k/Up :205; j/Down :208; Space, s, Enter :218, p, d, b; `_ => {}` then true :226 swallows the rest (g, G, Home, End, PgUp, PgDn, `.`, `/`, n, N, q, ?).

## Blame / History: keys.rs:124 place_claim, active when trail.place_shown() (place.rs:429)
- Passes on only `?`, `q`, Ctrl+C and Ctrl+R (place/keys.rs:23-29). Every other key returns true (keys.rs:128): Tab, BackTab, Home, End, `.` and all unbound keys are swallowed, before the menu and before undo (u/U).
- Blame (place/keys.rs:33): Ctrl/Alt are no-ops :46; j/Down, k/Up, PgDn, PgUp, g, G :54-59; Space (heatmap) :60; `/` :61; n, N :62-63; w (Diff row) :64; i :68; Enter :74; ^; L (Diff row) :76; Esc :77. With the `/` prompt open, typed characters go to the prompt :87.
- History (place/history_keys.rs:24): j/Down, k/Up, PgDn, PgUp :29-32; Tab (List↔File) :33; n, N :41-42; v, w, L, B (Diff row) :43-50; Enter :51; Esc :52. g and G have no handler (swallowed, no-op). BackTab is swallowed.

## Could not confirm
- The full conditions inside claims_b and blamable (place.rs) were not read.
- Not read whether strip_key or commit_graph_key claim any of these rows.
