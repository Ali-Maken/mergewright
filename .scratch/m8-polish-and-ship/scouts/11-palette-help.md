# Scout 11 — palette, help, actions (main @ 81899a70)

## What exists
- **No palette code.** No `graft-ui/src/palette*`, no `update/palette*`, no `fuzzy_score`. Only the context: `ContextId::Palette` keymap.rs:70-71 (falls back to Global :166; text-field context :212); warning words warning.rs:190.
- Table rows: `app.palette` = `⌃P`, `:` table.rs:67-72; `app.help` = `?` table.rs:73; `app.menu` = `.` table.rs:100; `palette.run` = `⏎` table.rs:489. `Binding` has search words (keymap.rs:317-319).
- Keymap API present: `translate` resolve.rs:173, `spell` :215, `spell_keys` :227, `spell_every` :238, `in_context` :327; `KeyWarning::words` warning.rs:75.
- Edge: `update::edge::key` edge.rs:18-37 (text field → raw; else `translate(context_of)`). `context_of` doc edge.rs:58-59: "Never `Tour` or `Palette` (ticket 11's to add)". `dispatch_key` mod.rs:1081 is documented for palette use (:1077-1078).
- `AppState::config_problems: Vec<String>` state.rs:416, set :860; status line in view/frame/bars.rs (scout 16).

## What `?`, `:`, `⌃P` do today (rule 7)
- Translate passes them (default chords); no dispatcher answers any. Recorded as known: `NOT_YET_ANSWERED` update/tests/keymap_edge.rs:342-346 (`app.help`, `app.palette`, `palette.run` → "ticket 11").
- Places pass `?` up as a live global: update/place/keys.rs:22-28 — then nothing answers it.
- Test pinning `?` as a no-op: detail/landing/tests.rs:307-309 (`?` leaves the landing) — will need a non-moving key if `?` opens a modal.
- Could not confirm a single "fallthrough" line per dispatcher; resolver/banner `?` bindings: none found by grep for `'?'` in graft/src/update.

## Doorway / hints
- `MORE = hint(MENU, "more")` graph_screen/hints.rs:355; attached hints.rs:575; catalogue.rs:544; ~6 tests in hints/tests.rs (258, 981, 1053, 1068, 1137, 1204).
- `NEVER_HINTED` is in **graft** update/tests.rs:5751 (not graft-ui). `a amend` there: could not confirm (grep for amend at :5751-5780 not done line-by-line).

## Matcher
- `filter_switch_rows` graft-ops/switch_list.rs:185; `rank` :199 — substring, tier 0 segment-start / 1 other, over `SwitchRow`. Not fuzzy, not a `(query,text)->score`; the pure core of `rank` is extractable. graft-core reached by both crates.

## Theme change path
- Theme lives in **`Session`** (event loop), not `AppState`. OSC 11 re-query built: redetect.rs:136-141 → `Wake::Terminal(TerminalChange::Theme)` → turn.rs:256-262 `session.apply` (session.rs:190-201, `ui.theme` forced wins) + `graphics.flush(Flush::Theme)`. Only strip cache flushed (scout 16).
- **Contradiction with ticket:** "through the existing theme change path" — that path takes a `ThemeHint` (terminal evidence) and a forced `ui.theme` overrides it (session.rs:196-197); a chosen kind has no path. Update returns `Command`s; no theme command exists (grep `Command::` in update/mod.rs). Ticket 16 §7 (16:49-53) adds `Session::retheme(kind)` and says 11 must call it.

## Overlaps
- **16:** shares graft `session.rs` (retheme) and `event_loop/running/turn.rs`; 11 needs 16's `retheme` or adds its own → order 16 first or seam. No graft-ui theme file overlap seen.
- **09d (draft):** rewrites `update/tests/keymap_edge.rs` whole (09d:37,43) — 11 deletes `NOT_YET_ANSWERED` rows there; both touch `keymap/*` tests, graft-ui MAP.md, `empty_state*` (11 adds `? keys` to empty bar; 09d D6 `outside_edge`), `graph_screen` mod. Conflict: serialise.

## Files 11 touches
graft-ui: new `palette.rs`, `graph_screen/hints.rs`+`hints/catalogue.rs`+`hints/tests.rs`, `empty_state*`, `lib.rs`, MAP.md. graft-core: matcher. graft-ops: `switch_list.rs`. graft: new `update/palette.rs`, `update/edge.rs`, `update/mod.rs`, `update/state.rs`, `update/mouse.rs`, `update/place/keys.rs`, view/frame (draw), `session.rs`/`turn.rs` (theme), tests (`tests.rs`, `keymap_edge.rs`, landing tests), every capture with `. more`.

## Size / kind
Design-heavy-ish UI + wide mechanical sweep (doorway re-shoot, every-context `?`, 4 hint rows 163/538/539/553). No git writes. ~75k is tight: likely split — widget+state+modes+theme rows / doorway swap + hint rows + captures.
