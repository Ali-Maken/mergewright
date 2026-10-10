# Scout: 42 health-map-app (HEAD 523f6c59)

## What exists
- Guard: `crates/graft/tests/it/map_coverage.rs` (+ `map_coverage/public_set.rs`). `CHECKED: &[&str]` at :22-29 = six libraries; **graft not in it**. No `Mode` enum, no `declared_items`.
- Check (a) `every_pub_item_is_in_its_map` :110 — item counts as named if it appears as **any word** in any `##` heading or any backtick span on a `- ` line (`backticked_idents` :75-98). Check (c) `every_map_item_still_exists` :187 uses `listed_item` :101 (first backtick span of a `- \`` line, last `::` segment).
- `public_items` (public_set.rs:32): walks from `lib.rs` through `pub mod` only. graft `lib.rs:12-35` declares 23 mods: 18 `pub`, `empty_state`/`first_launch`/`recent`/`trace` `pub(crate)`, `spelled` `#[cfg(test)]`. Nested `pub mod` only in `update/mod.rs`, `update/context_menu.rs`, `update/preview.rs`, `event_loop/mod.rs`, `view/mod.rs`. `main.rs` declares no mods.
- graft public-set **count: could not confirm** (needs running the test; scout does not run cargo). Raw `pub (fn|struct|enum|trait|type|const|static)` lines in src: 785.

## Module count (non-`tests` `mod` decls, grep)
- 427 total; 129 directly under `#[cfg(test)]`; 196 .rs files under `tests` dirs. Non-test module count therefore roughly 230-300 — exact figure **could not confirm** (needs cfg(test)-ancestor walk).
- Per dir: update 268, jobs 37, view 37, event_loop 31, history 12, config 6, rest 0 (flat files).

## MAP state
- `crates/graft/MAP.md`: **1237 lines**, 67 `##` headings. `^## history` matches 4 (:58, :1148, :1197 `history::source`, :1202 `history::seeds`); `^## event_loop` matches 5 (:95, :1001, :1044, :1063, :1069).
- Moved-code coverage: `hub` 0 hits; no `## jobs::runner` or `## update::geometry`/`update::dispatch` heading (src has `update/geometry.rs`, `update/dispatch.rs`, `jobs/runner/`); `rename` only as bullet :20 under `## lib.rs`; three `## File layout` sections :1209/:1213/:1232. Headings carry ticket ids (e.g. :134, :352, :359, :362) — (b) would fail them.
- 21 files under docs/.scratch/KNOWN_GAPS cite `graft/MAP.md:`.

## Ticket vs code
1. Ticket says MAP 1158 lines, dup sections :57/:1141 and :94/:995 — actually 1237, :58/:1148, :95/:1001.
2. Acceptance `rg -c '^## history'` / `'^## event_loop'` == 1 is unsatisfiable if submodule headings `## history::source`, `## event_loop::input` etc. exist under its own "one `##` per module" scope. Contradiction in the ticket.
3. One heading per module (~230-300) + purpose line(s) cannot meet "about 250 lines or fewer".
4. "Read `map_coverage.rs`" — it is now `map_coverage.rs` + `map_coverage/public_set.rs`; fine, but signature change touches both if `declared_items` lives beside `public_items`.
5. Mutation acceptance: deleting a heading fails (a) only if the module name appears nowhere else as a heading word or backtick word (`backticked_idents` :75). Names like `history`, `map`, `search` recur in backticks, so the mutation may pass silently unless the builder picks a unique name or (a) is changed to match headings only.

## Size
update/ alone has 268 mod decls; one context is tight. Ticket's own pass-2 split (update/+view/ first) is likely needed.
