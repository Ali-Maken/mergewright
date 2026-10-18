# Scout 06i — graph mode in `?` (read-only, main checkout, 2026-10-18)

## What exists
- **Groups:** `update/palette/groups.rs:14-77` `groups()`. `YOUR CONFIG` first if problems (:21-37, rows `RowKind::App`, `PaletteTarget::Note`). `PaletteMode::Here(_)` pushes `IN PROGRESS`, `HERE`, `EVERYWHERE` (:58-62) — insertion point for a group after `HERE`. `Everything` (`⌃P`): `ACTIONS` + `RECENT_HEADER`, order flips on empty query (:63-74). App rows (themes, tour) appended inside `everything()` :100-136.
- **Hidden words:** `Entry { row, target, search: &'static [&'static str] }` (`palette/mod.rs` ~:290) — already the slot for `GRAPH_MODE_WORDS`; all app rows today pass `&[]`.
- **Targets:** `PaletteTarget` `mod.rs:240-262` {Action, App(AppAction), Note, Seek, File, More}; `AppAction` :228-237 {Theme, Tour}. Theme → `Command::Retheme` (`command.rs:387`), "for this run only".
- **RowKind** `graft-ui/src/palette.rs:56-74`: Action, App, Branch, Tag, File, Commit, Recent, More. No Setting.
- **PaletteRow** `palette.rs:78-89`: `label`, `key: Option<KeyLabel>`, `config: Option<ActionId>`, `kind`. **No detail field.** Nearest: `muted_prefix` (`palette/row.rs:93-119`) mutes a *prefix* of label for File/Commit only.
- **Row draw** `row.rs:60-103`: key column fit to `KEY_CELLS-1`, always accent+bold; config id right, muted, only when `wide` and room; label cut first to make room for id (:79-83).
- **Commit-editor "keyless" rows** (`editor_tests.rs:147`) are `ActionId` rows (`commit.toggle_amend`), not a keyless kind.
- **Config path:** `Loaded.path` `config/load.rs:26` is `None` when no file exists (:159-160) or no home (:183-188). Held after startup in `Session.config: Loaded` (`session.rs:27`); not found in `AppState` (grep `config_path` none). `config::default_path()` `load.rs:177` names the path regardless; `recent.rs:82` calls it at runtime.
- **Commands:** `update/events/command.rs:18`; events `AppEvent` `update/events.rs:27`. Disk writes by the app today (`SaveLayout` :94, `FirstLaunchDone` :101) are **carried by the loop, not jobs** (`jobs/runner/dispatch.rs:98` `not_a_job`).
- **Bar:** `Status` `update/state.rs:1188`; `Done{what, also, keys}` drawn `✓ what` (`Status::done` :1282); `also` drawn after the tick. Failure variants `Failed`/`Sticky` — whether they draw `✗` *could not confirm*.
- **Deps:** `toml` 1.1.6 no `display` (`graft/Cargo.toml:63-77`); `toml_edit` absent from Cargo.lock (has `toml_parser`, `toml_writer`, `toml_datetime` :2776-2809); `tempfile` dev-only in graft (:99). `tests/it/dependency_edges.rs` keeps a declared-deps table (:65) — must gain entries. No `deny.toml`.
- **Captures with `?` on graph:** `docs/shots/m8-help-graph-120x36.txt`, `m8-help-80x24.txt`, `m8-look-help-graph-187x50.txt`, `m8-keymap-config-problem-120x36.txt`, `m8-contact-sheet-{1,2,3}.txt`, proto `m8-proto-2*`; `m8/help-at-map-120x36.txt` (map, not graph). No insta `.snap` contains `EVERYWHERE`.

## Mismatches with the ticket
1. **`config` column is `Option<ActionId>`** (`palette.rs:86`); `graph.merged_branches` is a config key, not an action. Ticket's "config id column `graph.merged_branches`" needs either a fake `ActionId` or a type change — not in Signatures.
2. **`Loaded.path` is `None` with no file** (`load.rs:159`) — ticket "Needs: `config::Loaded` (its path)" fails exactly in the pty-walk case (no file). `default_path()` works.
3. **Writes are loop-carried, not worker jobs** (`dispatch.rs:98`, `command.rs:99-101`); ticket says worker + job event. New precedent, not a match.
4. **Key column is accent+bold for any key** (`row.rs:68-73`); ticket wants accent on `●` only, `○` not.
5. **No muted-detail field**; the only muted-in-label is a prefix (`row.rs:93`), ticket wants a suffix → `detail` field is new (as Signatures allow).
6. Ticket's "keyless rows over the commit editor" precedent: those carry `ActionId`s (`editor_tests.rs:147`); the real keyless precedent is `RowKind::App` (themes/tour/notes).
7. `?`-on-graph captures list incomplete in ticket (names only m8-keymap-config-problem + 06h's); 7+ more above.

## 06h assumptions (unbuilt; not checked)
`MergedBranches::{KEY, as_config}`, `History::merged_branches`, `switch_graph_mode`, `change_view` re-ask — none exist yet (grep `merged_branches` in `config/` shows only tests of `renderer`).

## Build with
Sonnet holds: mismatches 1–3 need a conductor ruling first (ActionId vs new type; path source; loop vs job).
