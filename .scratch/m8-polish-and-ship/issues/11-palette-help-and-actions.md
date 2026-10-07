# G-palette 1 — `?` and `⌃P` / `:` open one palette; actions, theme rows, your config

Status: draft (planner spec-only split, 2026-10-07, from scout `.scratch/m8-polish-and-ship/scouts/11-palette-help.md`;
part 2 is `11b-palette-doorway-and-hint-rows.md`)
Build with: **Opus** — a new modal screen plus the theme command across graft-ui, graft-core and graft; every choice is
decided below and each is held by a named test. Not Fable: nothing left structural or unjudgeable from tests.
Blocked by: 08, 09, 09b, 10, 10c, **09d** (09d rewrites and splits `update/tests/keymap_edge.rs` and edits
`empty_state*`; this ticket deletes rows there and moves a landing test — serial, 09d first), **16** (`Session::retheme`
and the overrides it keeps; this ticket calls it and edits `session.rs`/`turn.rs` after it)
Needs: from 09/09b/09c — `Keymap::{reachable, in_context, translate}`, `KeyChord::{to_press, label}`, `KeyLabel`,
`Binding.search`, `ContextId::{Palette, is_text_field}`, `update::edge::context_of`, `AppState::config_problems`,
`KeyWarning::words`; from 09b — `update::dispatch_key(&mut AppState, &mut Vec<Command>, KeyPress)`; from 16 —
`Session::retheme(&mut self, ThemeKind) -> bool`; from 09d — the per-context sweep fixtures and `SWALLOWED_BY_DESIGN`.
**Crate set:** graft-core (new matcher module), graft-ops (`switch_list.rs` only), graft-ui (new `palette.rs`, `lib.rs`,
MAP.md), graft (new `update/palette.rs`, `update/edge.rs`, `update/mod.rs`, `update/state.rs`, `update/place/keys.rs`,
the `Command` enum's file, `view/frame` draw site, `session.rs`, `event_loop/running/turn.rs`, tests, MAP.md).
**Not here (11b):** `graph_screen/hints*`, `empty_state*` bar, `update/mouse.rs`, `NEVER_HINTED`, doorway captures.

## Read first
- `docs/design/01-view-mockups.md` §1.16 (:471–498), §1.6 (palette rows); `docs/design/02-keymap.md` §2.12
  (:526–541), §2.1 (modal bullet, :82), §2.3c (:296 — confirm boxes swallow)
- `docs/adr/0056-help-is-the-palette-on-this-place-and-the-tour-is-a-practice-repo.md` §1–2
- `docs/design-notes/m8-polish-and-ship.md`: Prior art rows :47, :54, :56; Journeys J4–J7, J9; Interfaces
  `graft-ui::palette` (:276–279)
- The scout file above (whole, 1 page)
- `09d-keymap-sweeps-and-literals.md` **Question 2 / D7** (option (a) is ruled here, decision 3)
- `crates/graft-ui/MAP.md` `## keymap` (:389–406), `text_input` (`TextField`, `FieldView` :381–383), `picker`;
  `crates/graft/MAP.md` `## session` (:77–84), `update::dispatch_key` (:209), `update::picker`
- Source after the maps: `update/edge.rs` (`key`, `context_of`), `update/place/keys.rs:22-28`,
  `graft-ops/src/switch_list.rs:185-230`, `session.rs` (`apply`, `retheme`), `turn.rs` theme arm

## Decisions (planner; not the builder's)
1. **One matcher, in graft-core.** `fuzzy_score(query, text)`: case-insensitive; the query splits on whitespace; every
   word must occur in `text` as a substring, else `None`. Per word, tier 0 when the hit starts `text` or follows a
   non-alphanumeric char, else 1; score = Σ tiers × 1024 + first hit's char offset (capped 1023). **Lower ranks
   first.** This is `switch_list::rank`'s core (scout: substring, tier 0 segment-start / 1 other) generalised to words;
   `rank` calls it. The only behaviour change for switch: a query with a space now can match (NOTES says so); its
   existing tests stay green **unedited**. A palette row matches if its label or any `Binding.search` word scores; its
   score is the best of those. (`theme light` finds `Theme: light`.)
2. **Groups and their rows.** `opened_from = context_of(state)` at open (never `Palette`).
   - `YOUR CONFIG` (both modes, first, only when `config_problems` is non-empty): one row per problem, its words as
     label, `kind: App`, no key, `⏎` closes (nothing to run).
   - `IN PROGRESS` (`Here` only; an operation is live): `ContextId::Banner`'s rows with a live chord.
   - `HERE` (`Here`): every `reachable(opened_from)` row whose `binding.context != Global`, minus IN PROGRESS's ids,
     then `in_context(opened_from)` rows with no chord there (key `—`). Table order.
   - `EVERYWHERE` (`Here`): the `Global` rows, same rule.
   - `ACTIONS` (`Everything`): HERE ∪ EVERYWHERE ∪ app rows, deduped by id, then the three app rows.
   - A binding with an empty `default` (e.g. `app.suspend`, switch off) is not listed: nothing can run it.
   - `Palette`, `Picker`, `Confirm`, `Credential`, `CommitEditor` text-field rows are never listed (their keys are fixed
     and only mean something inside them).
3. **Per-place spelling, in the palette only** (09d D7 option (a), scoped). A row's key = `KeyLabel` of the first chord
   `reachable(opened_from)` gives that id — what pressing it **there** does. Bars and footers stay on `live_labels`
   (D7); this ticket changes no bar.
4. **Filter.** Empty query: all groups, cursor on the first runnable row. Typed: rows scored per decision 1, groups keep
   their order, rows within a group by score then table order; empty groups dropped. **A query that parses as a
   `KeyChord` (`KeyChord::parse`) bound in `opened_from`** (`translate(opened_from, chord)` → that row's default) puts
   that row first: its group is drawn first and the row heads it; cursor on it (J5).
5. **`⏎` runs as the key would.** Close the palette (restoring `opened_from` focus), then
   `dispatch_key(state, commands, binding.default[0].to_press())` — the default chord, post-translation, so an unbound
   row runs too (J6's point). A refusal reads as from the key (same path). IN PROGRESS rows: same call; if J7's `⏎`
   on `c continue` does not continue from `opened_from` = Graph, **stop** — NOTES names the arm's `file:line` that
   answers `c` at a stop and where `dispatch_key` sent it; report, don't route around it.
6. **Theme rows** are `PaletteTarget::App(AppAction::Theme(kind))` for `Dark`, `Light`, `HighContrast`; labels
   `Theme: dark`, `Theme: light`, `Theme: high contrast`; no key, no config id. `⏎` → `Command::Retheme(kind)`. The
   event loop's command handling calls `session.retheme(kind)`; if `true`, `graphics.flush(Flush::Theme)` and redraw —
   the same two steps as the `TerminalChange::Theme` arm (turn.rs). **A chosen kind sticks for the run:** `Session`
   gains `chosen: Option<ThemeKind>`, set by this command; `apply(TerminalChange::Theme)` keeps it over terminal
   evidence, as a forced `ui.theme` already wins (session.rs:196-197). Not an operation, not journaled, not undoable
   (this run only, §2.12 :533).
7. **Where `?` answers.** Every screen: graph, every pane and place, overlays, previews, focused banner, resolver,
   empty state (§2.2 "`?` is help on every screen"). **Not** over a live confirm box (§2.3c :296 swallows; the box's
   bar already shows its keys), and inside any text field `?` types `?` (credential, name prompt, `/`, commit editor,
   the palette itself). If 09d's `SWALLOWED_BY_DESIGN` lists `app.help`/`app.palette` for `Preview` or `Banner`, delete
   those entries citing this decision.
8. **Modal.** While open, `context_of` returns `ContextId::Palette` (a text-field context: translate passes every key);
   the palette's handler takes `↑`/`↓` (move, clamped), `⏎` (run), `esc` (close, nothing run, focus restored), every
   other key edits the query (`TextField`); `Ctrl+C` passes (quits, §2.1). Mouse: 11b. While open, the bottom row is
   the palette's keys `↑↓ move  ⏎ run  esc close` (spelled from `Keymap::shipped()`, a text field — R4).
9. **Box.** §1.16: ≤ 96 columns, centred over the focused pane's rect; frame width < 100 → full frame width. Height:
   rows to fit, capped at the pane's height minus 2; scrolls to keep the cursor in view. Title `keys here` (`Here`) /
   `find anything` (`Everything`) in the top border, `esc close` right. Group headers muted caps; key column 6 cells,
   accent; config id muted, right-aligned, dropped < 100 columns. No match: one row `nothing matches · esc close`.

## Scope
In: decisions 1–9; `?`, `⌃P`, `:` opening from every non-text-field context; the `NOT_YET_ANSWERED` rows for
`app.help`, `app.palette`, `palette.run` deleted (wherever 09d left `keymap_edge*`); the landing test that used `?` as a
no-op (`detail/landing/tests.rs:307-309`) re-pointed to an unbound key (`F20`, as 09d's sweeps), name kept.
Out (11b): the bar doorway `. more` → `? keys`, `. actions`, hint rows 163/538/539/553, mouse, the rebase-stop and
80×24 captures. Out (12): branches, tags, files, commits, recent. Out (19): `Practice tour` row.

## Signatures
```rust
// graft-core::matcher (new; graft-core/MAP.md row)
pub fn fuzzy_score(query: &str, text: &str) -> Option<u32>;          // decision 1; lower is better

// graft-ui::palette (new)
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum PaletteMode { Here(ContextId), Everything }
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum RowKind { Action, App, Branch, Tag, File, Commit, Recent }
#[derive(Clone, Debug, PartialEq)]
pub struct PaletteRow { pub label: SanitizedText, pub key: Option<KeyLabel>, pub config: Option<ActionId>, pub kind: RowKind }
#[derive(Clone, Debug, PartialEq)]
pub struct PaletteView { pub mode: PaletteMode, pub query: FieldView, pub groups: Vec<(SanitizedText, Vec<PaletteRow>)>,
    pub cursor: usize /* index over all rows, groups flattened */, pub top: usize, pub warnings: Vec<SanitizedText> }
pub struct Palette<'a> { pub view: &'a PaletteView, pub theme: &'a Theme }      // impl Widget
pub fn palette_area(over: Rect, frame: Rect) -> Rect;                          // decision 9
pub fn palette_row_at(area: Rect, view: &PaletteView, column: u16, row: u16) -> Option<usize>; // 11b's mouse
pub const NOTHING_MATCHES: &str = "nothing matches · esc close";

// graft (pub(crate)) — update::palette
pub(crate) struct PaletteState { pub mode: PaletteMode, pub query: TextField, pub cursor: usize, pub top: usize,
    pub opened_from: ContextId }
pub(crate) enum AppAction { Theme(ThemeKind) }                  // 19 adds `Tour`
pub(crate) enum PaletteTarget { Action(ActionId), App(AppAction), Note }  // `Note` = a YOUR CONFIG row; 12 adds its own
pub(crate) fn open(state: &mut AppState, mode: PaletteMode);
pub(crate) fn rows(state: &AppState, palette: &PaletteState) -> Vec<(SanitizedText, Vec<(PaletteRow, PaletteTarget)>)>;
pub(crate) fn palette_key(state: &mut AppState, commands: &mut Vec<Command>, key: KeyPress);
pub(crate) fn view(state: &AppState, palette: &PaletteState) -> PaletteView;
// AppState gains `pub palette: Option<PaletteState>`
// Command gains `Retheme(ThemeKind)`; Session gains private `chosen: Option<ThemeKind>`
```
12 extends `PaletteTarget`/`rows`; it must not need a signature change here — if the builder sees one coming, NOTES.

## Acceptance (each tick names its test, at tick time)
- [ ] Matcher: table test in graft-core (segment start beats inside; every word must hit; case-insensitive;
      `theme light` vs `Theme: light` = `Some`); `switch_list` tests green unedited.
- [ ] Widget at 80×24, 120×36, 200×60 (buffer tests): groups and order as §1.16; config id present ≥ 100 columns,
      absent below; `—` for an unbound row; box ≤ 96 wide and centred over `over` at 120 and 200, full width at 80;
      cursor row scrolled into view with 40 rows; `nothing matches · esc close` on an empty result.
- [ ] Walk tests counting keys (`inputs_of` rule: a key or a typed string = 1): J4 `find-key-by-name` = 2 (`?`
      `merge` → first HERE row `m … merge`); J5 `what-does-key` = 2 (`?` `x` → `x`'s row first, cursor on it); J6
      `run-unrecalled` = 3 (`⌃P` `stash` `⏎` → the stash prompt open); J7 `stopped-how-go-on` = 1 (rebase stopped,
      `?` → first group `IN PROGRESS`, first row `c continue`; then `⏎` continues — truth by git); J9 `light-theme` = 3
      (`⌃P` `theme light` `⏎` → `Command::Retheme(Light)` emitted).
- [ ] Theme path: `Command::Retheme(Light)` through the loop's handler → `session.retheme` called, theme kind light,
      strip cache flushed (test); a following `TerminalChange::Theme(dark hint)` leaves it light (test); 16's
      overrides for light applied (reuse 16's override fixture).
- [ ] Per-place spelling: `[keys.graph] merge = "j"` → `?` on the graph shows `j` for merge and **no** `j` beside
      `app.down` (it isn't reachable there); on the changes pane `j` beside move (test).
- [ ] Sweep: `?` opens a `Here(ctx)` palette on every non-text-field `ContextId` with a 09d fixture, plus the empty
      state; over a live confirm box `?` leaves state unchanged; in each text field `?` is typed (one test, cases
      printed).
- [ ] `⌃P` and `:` open `Everything` on the graph, a pane and the empty state (test). `esc` closes, focus restored,
      no command (test). `Ctrl+C` while open quits (test).
- [ ] 2 config problems → `?` lists `YOUR CONFIG` first with both words (test; the status words are 16's).
- [ ] A refusal through the palette reads as from the key: `⌃P` `merge` `⏎` with HEAD detached → the same status as
      `m` (test compares both).
- [ ] Rule 7: NOTES records the scout's `?`/`:`/`⌃P` lines (keymap_edge `NOT_YET_ANSWERED`, place/keys.rs:22-28)
      before the change; the `NOT_YET_ANSWERED` rows are gone (grep in NOTES).
- [ ] Captures `docs/shots/m8-help-graph-120x36.txt` (`?` on the graph) and `docs/shots/m8-palette-everything-120x36.txt`
      (`⌃P`, empty query), each beside §1.16's drawing in NOTES. **Colour unverified** (rule 5).
- [ ] MAP.md: graft-core `matcher`; graft-ui `palette`; graft `update::palette`, `AppState::palette`,
      `Command::Retheme`, `Session::chosen`, `context_of` (returns `Palette` while open).

## Touches
- `context_of` and every caller (`hint_context`/`footer_context` must show the palette's bar while open — check
  `view::rows`); `dispatch_key`'s callers (none new besides this); `switch_list` (`C`'s list — its tests).
- Theme: `Session::apply`, `retheme` (16), `turn.rs` theme arm, strip cache (`Flush::Theme`); `StripPaint::of` reads
  the new theme (16 decision 8 — no key change). Startup's theme choice unchanged.
- Sweeps: 09d's `SWALLOWED_BY_DESIGN` (decision 7), `NOT_YET_ANSWERED`, landing test; `every_key_the_bar_draws_is_answered`
  (no bar change here — stays green).
- Draw order: the palette draws after the focused screen and before `draw_modal_boxes` (graft MAP view, :22) — a box
  raised while the palette is open is impossible (palette is modal), state the check.
- Keymap: no rows added (`palette.run`, `app.help`, `app.palette` exist, table.rs:67-73, :489).
- Journal/undo: none — show the empty diff over `update/journal*` and `graft-ops/src/journal*`.
- 12 (extends `rows`/`PaletteTarget`), 18 (`?`/`⌃P` opening = "used once"), 19 (`AppAction::Tour`).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~66k: widget + tests 15k; matcher + switch 6k; state, open/close, filter, run, edge, view 20k; groups and spelling 8k;
theme command + `chosen` 6k; walks, sweep, config test 8k; 2 captures + MAP 3k.
Order: matcher → widget → state/open/filter/run → walks J4–J7 → theme → sweep → captures. Seam if over: commit after
J4–J7 green, report `blocked: theme rows, sweep, captures`.
