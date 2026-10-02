# Scout 06 — look-chrome (header, gutter, bar)

## Header drawers
- Graph: `graft-ui/src/graph_screen/layout.rs:105` `header()` — draws literal `" graft "` brand (`:107-110`, Accent), then banner (reserved first, `:124-137`), location, walk/filters/gfx. Called from `graph_screen/render.rs:39`.
- Map: `map_screen/render.rs:116` own `header_line()`, drawn `:340`; own `footer_line()` `:125`, drawn `:391`.
- Journal: `journal_screen.rs:195` own `header()` (drawn `:400`), `footer()` `:220`.
- Stash: `stash_screen.rs:169` own `header()` (drawn `:369`), `footer()` `:174` (static str).
- History place: `graft/src/update/place/history/view.rs:42` own `self.header(now)`; `graft-ui/src/history/file_pane.rs:66`.
- Resolver: own `HEADER_ROWS = 2` `conflict_resolver.rs:95`; header `draw/block.rs:27`.
- None of map/journal/stash/history/resolver call graph_screen's `header()` — each has its own (so "shared header" = graph only; screen count for restyle = graph + 5 own headers).

## Bar drawers
- Graph footer `graph_screen/footer.rs:46` (`hints_for` `:101`, `stopped_row_before_keys` `:109`, `hints_text` `:118`); render `render.rs:73`.
- graft frame bars calling `hints_text(hints_for(..))`: `view/frame/bars.rs:84,152,226`; `overlays.rs:135,194,231`; `conflicts.rs:109`; `view/finish.rs:212`; `finish_summary/frame.rs:61`. `hints_for` `hints.rs:295`; `hints_text` `hints/fitting.rs:195`; `stopped_row_before_keys` `fitting.rs:274`.

## CHROME_ROWS readers
- Def `graph_screen/mod.rs:91` (=2); re-export `graft/src/update/state.rs:21`; `state.rs:502` (`graph_pane_rows`); `update/banner.rs:308`; `update/picker.rs:259` (with `geometry::HEADER_ROWS`); `view/strips/tests.rs:16,26`; tests `graft-ui/tests/{ascii_tier.rs:13,54,77; lanes_without_hue.rs:14,65,87,125; graph_screen.rs:12,28; selection.rs:15,62,155,233}`, `graph_screen/tests.rs:144` (asserts ==2).
- `HEADER_ROWS` (graft): `update/geometry.rs:26` `rows_above_graph`; `mouse.rs:285,318,365-371`; `update/tests.rs` ~15 sites (`:477…5931`).
- Own, not this: `commit_editor.rs:189` (6), `picker.rs:32` (4).

## Repo/path/branch source
- Path: `view/rows.rs:42` `sanitize_path(&facts.path)` → `GraphView.location`; journal `view/journal.rs:35`. `sanitize_path` `graft-term/src/sanitize/mod.rs:157`.
- `~` abbreviation: no helper found (grep for HOME/`~` in non-test src: only `crash.rs:130-163`).
- Repo name: no separate field; only full path. Branch: `GraphView` has no branch field; `facts.head_branch` exists (`update/remote.rs:487`), `preview::current_branch` (`update/preview.rs:61`). Detached-sha source: could not confirm.

## `?` on the graph (code, not defect)
- `key_event` `update/keys.rs` → `graph_key` `keys.rs:318-346`: no branch matches `'?'`; the only `'?'` key match in graft src is `place/keys.rs:27` (`live_global`, passes `?` through a place). So on the graph `?` falls through every `else if` and does nothing. Test pinning "moves nothing": `update/detail/landing/tests.rs:308`.

## Stale captures
- `grep -rl '^ graft ' docs/shots` → 345 files; `crates/graft-ui/tests` + `crates/graft/tests` → 25 files (incl. `crates/graft-ui/tests/shots/`). Any 2-cell gutter/rule change widens this to every graph capture.

## File set (expected)
graft-ui: `graph_screen/{mod,layout,render,footer,hints,hints/fitting,tests}.rs`, map/journal/stash headers if restyled, 4 tests files above. graft: `update/{state,geometry,mouse,banner,picker,tests}.rs`, `view/rows.rs`, `view/frame/{bars,overlays,conflicts}.rs`, `view/finish.rs`, `view/strips/tests.rs`.

## Overlap
- 04: `view/strips` (ticket 04:56) — `view/strips/tests.rs` reads `CHROME_ROWS`. Collides.
- 03: cursor motion reads `graph_pane_rows` (`state.rs:502`) / `graph_rows`; could not confirm 03 edits those files.
- 14: `update/preview/history.rs`, `update/clash.rs`, graft-ops — no file seen in 06's set. Disjoint as far as grep shows.

## Contradictions
- Ticket says map/journal/stash/blame/history/resolver "draw the shared header/bar": headers are each their own (lines above); only bars share `hints_for`.
- Ticket assumes branch in the view: `GraphView` has none today.
