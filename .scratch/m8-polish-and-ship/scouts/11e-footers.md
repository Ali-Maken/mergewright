# Scout 11e — J / S / Map footers (base 9a1c6b97)

## Q1 Footers
- **J**: `JournalScreen::footer(&self, width) -> String`, `crates/graft-ui/src/journal_screen.rs:243`; drawn `:391-392`. Not ready → `close` only (`:247-248`). Ready clauses in order: `⏎ <enter_words>` (`:252`), `b new branch here` if `branch_here` (`:255`), `/ find` always (`:260`), `n/N match` while a find is kept (`:261`), `⏎?/x command log` (journal.expand) on a Graft row (`:266`), `J/esc close` last (`:269-270`). Cut `:273-280`: drops from the right of the middle, first part and `close` kept; `join("  ")`. No bare-key clause. `enter_words` per row kind: `crates/graft/src/view/journal.rs:261` (Graft: undo this / redo this / revert to before this / None for live, not-undone kind, undo/redo rows; Outside: switch back to X / make X again / put X back…; Lost: continues past :300).
- **S**: `StashScreen::footer(&self) -> String`, `crates/graft-ui/src/stash_screen.rs:189`; drawn `:411-412`. **Takes no width; never cuts** — `set_text` just fits/truncates. Ready: apply, pop, save, drop, files, close (`:209-218`); Loading/Empty/Failed: save, close (`:220-221`). Built by `Keymap::key_verbs` (`:36`). No bare key.
- **Map**: `footer_line(&self) -> Line<'static>` (spans), `crates/graft-ui/src/map_screen/render.rs:126`; drawn `:416` via `buf.set_line` (truncates, no cut rule). Loading: `esc cancels` only (`:129-131`). Ready/Empty/Failed: `j/k select`, `⏎ zoom to commits`, `M/esc back` (`:137-141`), **then** ` · +N more` (`:146`) and ` · partial` in Warning ink (`:149-153`). No bare key.

## Q2 HintContext::{Journal,Stash,Map}
Constructors outside tests: only `view/rows.rs:682-684` (`overlay_context`), returned only when `state.graph_has_focus()` (`rows.rs:701`; `update/state.rs:689-690`: focus == Graph). Other mentions are match arms / lists: `hints.rs:519-521,568-570,814-816`; `catalogue.rs:41-43,344-346,449-451,523-525`; `lists.rs:82-87`.
Drawers: (a) graph footer `screen.hint_context` (`layout.rs:54`) — graph is drawn only when `graph_overlay()` is None (`frame.rs:185-188` vs `:188`) → **unreachable**. (b) `render_overlay_with_detail` → `split_for_detail` → `draw_tab_hints` with `pane_hint_context` (`overlays.rs:325-333`); `draw_tab_hints` draws only if `tab_hint_rows>0` (`bars.rs:145-148`), which needs `Tab{shown:true}` (`control.rs:103`) = focus != Graph (`detail_pane/layout.rs:59-61`) — exactly when `overlay_context` does not return these three → **unreachable**. `hit.rs:52` reads `graph_view` for badges only. Verdict: unreachable, by reading; not run.

## Q3 Fallback
`fn spelled_bar(context: &HintContext, divider: Divider, keys: &Keymap) -> (Vec<Hint>, Option<Hint>)`, graft-ui `crates/graft-ui/src/graph_screen/hints.rs:460`; fallback `:467-473` (DOORWAY → `DOORWAY_BY_PALETTE` `:359` if `?` unbound), dropped if unspelled `:474-476`. Works on `Hint`, not words.

## Q4 Sub-states
- J find prompt: bar replaced by `/typed`, `journal_screen.rs:379-390`.
- J `box_open` (= `state.preview.is_some()`, `view/journal.rs:63`): bar blank, `journal_screen.rs:391`.
- S save prompt: **not blank** — replaced by shipped-key `⏎ save  ⇥ scope  esc back`, `stash_screen.rs:195-205`. S box_open blank `:192-194`.
- Map loading: **not blank** — `esc cancels` (`render.rs:129-131`). Map failed: **draws the ready footer** (only Loading branches, `:129`).

## Q5 Shots (grep of fixed words)
J (`find  …close`): m3-08-journal-after-unstage-{160x48,80x24}, m3-journal-{160x48,80x24}, m3-journal-expanded, m4-07-journal-remote-rows-{160x48,80x24}, m7-proto-3-journal, m7-proto-4-picker, m7/journal-{100x30,140x40,187x24,187x50}, m7/journal-after-put-back-120x40, m7/journal-after-squash-187x44, m7/journal-expanded-120x40, m7/journal-find-187x50, m7/picker-empty-query-187x50, m7/picker-filtered-100x30, m8/doorway-journal-120x36, + contact sheets m3/m4/m7.
S (`apply  pop`): m3-06-stash-list-{160x48,80x24}, m3-stash-list, m3-contact-sheet. Save prompt: m3-07-stash-prompt-{80x24,160x48}, m3-stash-save.
Map (`zoom to commits|esc cancels`): m2-15-feature-branches-m-{100x30-unicode,160x40-unicode,80x24-ascii,80x24-unicode}, m2-map-view, m2-map-view-loading, m4-05-progress-footer-{160x48,80x24}, m4-remote-keys, m6/fix-box-owns-the-bar-{120x40,80x24}, contact sheets m4/m6. Could not confirm the m4-05/m4-remote-keys/m6 hits are Map footers (no "Map" word). `m8/help-at-map-120x36.txt` has no match (footer covered or absent).

## Contradictions with the ticket
- F2 "footer's words shed by their existing rule": S and Map have **no** shedding rule (`stash_screen.rs:189` takes no width; `render.rs:416` truncates).
- F2 "doorway last": Map appends ` · +N more` / ` · partial` after its keys (`render.rs:146-153`) — the doorway's place relative to them is not decided.
- Q4/F2 list "S save prompt" and "Map loading/failed" as no-doorway: S prompt and Map loading draw their own footers; Map failed draws the ready footer.
- `doorway_words` → `Option<String>`: today's fallback yields `Hint`, inside `spelled_bar` (`hints.rs:467`).
