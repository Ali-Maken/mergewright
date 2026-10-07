# Scout — 12 palette find-anything (main @ ae74b302)

## What exists (from 11)
- `crates/graft/src/update/palette.rs` (548 lines): `PaletteState` :32 (mode, query `TextField`, cursor, opened_from); `AppAction` :46; `PaletteTarget` :53 = `Action(ActionId) | App(AppAction) | Note`, **derives `Copy`** :52; `Group` alias :63; `Source` :67 (keymap, problems, operation_live — no refs/history/files); `opener` :103, `open` :161, `rows` :176, `listing` :182, `groups` :195 (headers `YOUR CONFIG`, `IN PROGRESS`, `HERE`, `EVERYWHERE`, `ACTIONS` :214–238); `filter` :334; `edit` :412; `palette_key` :456; `run` :479; `view` :513; `bar` :539.
- `crates/graft-ui/src/palette.rs`: `RowKind` :57 already has `Branch, Tag, File, Commit, Recent` (unused, "ticket 12"); `PaletteRow` :76; `PaletteView` :91; `palette_area` :151; `palette_row_at` :192; widget `Palette` :211.
- Draw: `crates/graft/src/view/frame/boxes.rs:35` `draw_palette`.
- Keys: `Ctrl+P`, `:` → palette on everything (`docs/design/02-keymap.md:199`, :534 lists the five sources). Palette claims every key but `Ctrl+C` (:82). 12 adds no key.

## Answers to scout questions
- `/` matches without moving cursor: `search::Query::matches` (`crates/graft/src/search.rs:56`) is pure. But `search::scan` :142 is a **job walking all refs** (not laid-out history); no in-memory "query laid-out rows" fn found. Laid-out rows live in `history/scroll` window (`query.rs:74`) — only the visible window.
- ls-files read: none for tracked paths. Nearest: `graft-git/src/diff/read.rs:273–283` (`ls-files -z --others`, untracked) and `conflict/disk.rs:107` (`-u`).
- Picker `⇥`: `update/picker.rs:233 show_on_graph` → `update/map.rs:202 seek_to` → `History::seek` (`history/scroll/query.rs:71`); tip not loaded → `Command::ShowRows` request (map.rs:208). Handled.
- File history opener: `update/place/history/open.rs:9 open_history(state, HistoryFrom)`.

## Contradictions with the ticket
- Ticket's `PaletteTarget` drops `Note` (exists :59) and adds `FileHistory(PathBuf)` — breaks the `Copy` derive at :52.
- "Commits: the `/` search over laid-out history" — the `/` search scans all refs as a job (search.rs:142); laid-out history is only a window. Synchronous per-keystroke commit rows: could not confirm a source.
- "Ranking … then recency": `filter` :334 uses `graft-core` matcher; recency source absent.

## Files 12 touches
update/palette.rs (+ Source), graft-ui/palette.rs (row drawing for new kinds), boxes.rs maybe, a new job + events route (file list), graft-git ls-files read, search.rs, MAP.md ×2, new capture.

## Overlap
- 11b: bar `? keys`/`. actions`, mouse `palette_row_at`, captures — touches graph_screen hints and palette geometry; same `palette.rs` files for mouse. Serialise with 12.
- 09e: search under overlays, bar `/ search` — touches `update/search.rs`; 12 reads `search.rs`. Low conflict unless 12 edits search.
- 17: row painting/graph — no shared file found.

## Size / kind
Over one context likely (four sources + job + perf script); split candidate: refs+commits+recent / files job. Kind: design-heavy + git (job, ls-files).
