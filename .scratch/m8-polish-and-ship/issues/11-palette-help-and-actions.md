# G-palette 1 — `?` and `⌃P` / `:` open one palette; actions, theme rows, your config

Status: **done except J7's truth-by-git** (builder 2026-10-07, compacted once; continuation 1 2026-10-07 finished
the empty state, the loop theme test, refusal parity and the confirm sweep — see NOTES "Continuation 1")
(planner spec-only split, 2026-10-07, from scout `.scratch/m8-polish-and-ship/scouts/11-palette-help.md`;
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
- [x] Matcher: table test in graft-core (segment start beats inside; every word must hit; case-insensitive;
      `theme light` vs `Theme: light` = `Some`); `switch_list` tests green unedited.
      — `graft_core::matcher::tests::{the_table, segment_start_ranks_below_inside}`; switch tests unedited, green in
      the workspace run.
- [x] Widget at 80×24, 120×36, 200×60 (buffer tests): groups and order as §1.16; config id present ≥ 100 columns,
      absent below; `—` for an unbound row; box ≤ 96 wide and centred over `over` at 120 and 200, full width at 80;
      cursor row scrolled into view with 40 rows; `nothing matches · esc close` on an empty result.
      — `graft_ui::palette::tests::{groups_and_rows_in_the_order_of_the_drawing,
      the_key_column_is_six_cells_and_the_cursor_marks_its_row, the_config_id_is_drawn_from_100_columns_and_dropped_below,
      the_box_is_at_most_96_wide_and_centred_from_100_columns_full_width_below, the_box_is_as_tall_as_its_lines,
      the_cursor_row_is_scrolled_into_view_with_40_rows, an_empty_result_draws_one_line_that_says_how_out,
      a_click_finds_the_row_it_landed_on}`.
- [~] Walk tests counting keys: J4 = 2 — `keymap_edge::palette::j4_help_then_merge_lands_on_the_merge_row_in_two_inputs`;
      J5 = 2 — `j5_help_then_a_key_puts_what_it_does_first`; J6 = 3 — `j6_palette_stash_enter_does_what_its_key_does`
      (asserts the same commands and frame as `S`, not "the stash prompt" by name); J9 = 3 —
      `j9_palette_theme_light_asks_the_loop_for_light`. **J7 partly**: `tests/it/palette_in_progress.rs`
      `j7_help_mid_rebase_puts_continue_first_and_enter_asks_what_c_asks` — real stopped rebase, `?` → `IN PROGRESS`
      first, `▸c Continue`; `⏎` emits exactly what `c` emits (`ContinueInGit { kind: Rebase }`). **"Truth by git" not
      observed**: `ContinueInGit` is a terminal handover the loop carries out (`jobs/runner.rs:1313` refuses it), so
      the app harness cannot run it. Not a routing problem: palette and key take the same path.
- [x] Theme path: chosen theme survives a later dark hint with light's overrides —
      `session::theme_tests::a_chosen_theme_outlasts_a_re_detected_background`; through the loop (`⌃P` `theme light`
      `⏎`, then a dark background wake: one `Flush::Theme`, still light) —
      `event_loop::tests::a_theme_chosen_in_the_palette_flushes_once_and_outlasts_a_dark_hint`.
- [x] Per-place spelling — `keymap_edge::palette::rows_are_spelled_with_what_the_key_does_in_that_place` (merge `j`,
      move `↓` on the graph; move `j` on Changes).
- [x] Sweep — `keymap_edge::palette::help_opens_on_every_place_and_is_typed_in_every_field` (every fixture context;
      confirm box: `?`, `⌃P`, `:` each `Seen`-equal to `F20`; every field: `?` typed — **except blame find**, see
      NOTES). Empty state: `empty_state::palette_tests::{help_opens_on_the_start_screen_and_esc_closes_it,
      not_over_the_init_box_and_typed_in_the_open_prompt}`; start-screen sweep
      `empty_state::sweep_tests::every_row_the_start_screen_reaches_acts_or_is_listed_and_follows_its_remap` (its
      `NOT_YET_ANSWERED` `app.help`/`app.palette` rows deleted; init box lists them swallowed, decision 7).
- [x] `⌃P`/`:` open `Everything` on graph and a pane — `ctrl_p_and_colon_open_everything_on_the_graph_and_a_pane`;
      `esc` closes, no command, same frame; `Ctrl+C` quits — `esc_closes_with_nothing_run_and_ctrl_c_still_quits`.
      Empty state — `empty_state::palette_tests::{ctrl_p_and_colon_open_everything_on_the_start_screen,
      help_opens_on_the_start_screen_and_esc_closes_it, ctrl_c_quits_with_the_palette_up,
      enter_runs_a_row_as_its_key_would, a_theme_row_asks_the_loop_for_that_theme,
      config_problems_head_the_list_on_the_start_screen,
      the_palette_draws_over_the_start_screen_with_its_keys_on_the_bar}`.
- [x] 2 config problems → `YOUR CONFIG` first, cursor past them — `keymap_edge::palette::config_problems_head_the_list`.
- [x] A refusal through the palette reads as from the key (HEAD detached) —
      `keymap_edge::palette::a_refusal_through_the_palette_reads_as_from_the_key` (`⌃P` `merge` `⏎` vs `m`: same
      commands, same status, `m needs a branch checked out`).
- [x] Rule 7 — NOTES below; `NOT_YET_ANSWERED` rows for `app.help`, `app.palette`, `palette.run` deleted from
      `keymap_edge/sweep.rs` (09d's `every_reachable_row_acts_or_is_listed_and_follows_its_remap` green).
- [x] Captures — `keymap_edge::palette::capture_help_and_palette` writes both; beside §1.16 in NOTES. Empty state:
      `docs/shots/m8-help-empty-state-120x36.txt` by `the_palette_draws_over_the_start_screen_with_its_keys_on_the_bar`.
      Colour unverified.
- [x] MAP.md — graft-core `matcher`; graft-ui `palette`, `Keymap::acting`; graft-ops `filter_switch_rows`; graft
      `update::palette` (incl. `AppState::palette`), `Command::Retheme`, `Session::choose`, `context_of`. (No
      `Session::chosen` getter: nothing reads it; the field is private.)

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

## NOTES (builder, 2026-10-07)

**Compacted once mid-ticket.** Scout used (`scouts/11-palette-help.md`): its lines held (NOT_YET_ANSWERED moved to
`keymap_edge/sweep.rs:22-25` by 09d; `place/keys.rs:22-28` as cited; `edge.rs` `context_of` doc). It missed the empty
state's separate loop (`event_loop/places.rs` `outside_loop`, no `AppState`), the turn.rs command path for a
loop-owned command, `FieldView`'s lifetime, and the `no_literal_keys` grep test. Partly sufficed.

**Test-first:** not red-first. The widget, matcher and state code were written before their tests in the first
context; the walk tests were written after the code and run green at first compile except where they found real
things (fixture probes, `Menu` closing on F20, blame find — below).

**Rule 7, before the change:** `crates/graft/src/update/tests/keymap_edge/sweep.rs:22-25` listed `app.help`,
`app.palette`, `palette.run` as `NOT_YET_ANSWERED` ("ticket 11"); `crates/graft/src/update/place/keys.rs:22-28`
`live_global` passed `?` up from a blame place to a chain where nothing answered it. After: the three rows are
deleted (grep `ticket 11` in `sweep.rs` → nothing); 09d's sweep counts 438 acting (was 396).

**Found, not fixed (09e's ground):** in blame's find prompt `?` (and by the same line `q`) is not typed:
`update/keys.rs:125` `place_claim` returns early for `place::live_global(key)` (`place/keys.rs:23-29`) without
checking `find_prompt`, so the key leaves the place's prompt; the palette then refuses (a text field is open) and the
`?` is dropped. Pinned as `NOT_TYPED_YET` in `keymap_edge/palette.rs`, which fails once it is fixed. Whether `q` quits
from that prompt: not checked.

**Deviations from the signatures (say so, not silently):**
- `PaletteView.query` is `SanitizedText` + `query_cursor`, not `FieldView` (it has a lifetime and no `PartialEq`).
- `PaletteView` gained `wide`; dropped `warnings` (the `YOUR CONFIG` rows carry them) and `top` (scroll is computed
  from the cursor at draw time, the picker's rule). `PaletteState` has no `top` either.
- `AppState::palette` is `Option<Box<PaletteState>>`: unboxed it grew `AppState` past `large_stack_arrays` in four
  existing tests. One test (`resolver/tests/take_all.rs:125`) still tripped by 8 bytes; it now loops over the two
  presses instead of an array of two states (test-only change, outside the listed files).
- `NOTHING_MATCHES` = `nothing matches`; ` · esc close` is spelled from `Keymap::shipped()` (the grep test forbids
  the literal).
- `Keymap::acting(ctx, chord) -> Option<ActionId>` added in graft-ui `keymap/resolve.rs` (not in the crate set's file
  list): the per-place spelling needs "what does this chord do here", which `translate` does not say.
- `graft_core::matcher` is behaviour in core, which normally holds none — the ticket's choice; flagged.
- No `Session::chosen()` getter (dead code); the private field does the work.
- `command.rs` `Retheme` forced `place/write.rs` `is_write` (exhaustive) to classify it: a read.

**Interpretations (questions for the planner):**
- IN PROGRESS rows are spelled in the `Banner` context; `⌃P`'s ACTIONS includes them.
- `⏎` on IN PROGRESS `s`/`a` opened from the Graph dispatches `s`/`a` as the graph would get them. With `c` that
  continues (J7 shows `ContinueInGit` either way); for `s`/`a` on the graph: not checked.
- The box's bottom border lands on the pane's last row (the graph's divider row in the captures):
  `palette_area` height is `over.height − 2` as specified.

**Captures** (colour unverified, rule 5): `docs/shots/m8-help-graph-120x36.txt`, `docs/shots/m8-palette-everything-120x36.txt`.
Beside §1.16 (`docs/design/01-view-mockups.md:471-498`): title, `esc close`, `›` row, `HERE` header, `▸` row, key
column, right-aligned config id match. Differ: the labels are the key table's (`Merge selected branch into current`),
not the drawing's (`merge the selected branch into yours`) — wording drift between table and mock-up; the drawing's
`▏` cursor is not a character in the capture; with the fixture's 30+ HERE rows, `EVERYWHERE` is below the fold.

**Touches:** `context_of` (09d `context_of_names_each_fixture` green); `text_field_open` callers (all `keymap_edge`
field tests green); key claim chain (`update::tests` green, 09d sweep green); `dispatch` / paste chain; `Command`
match sites (`jobs/runner.rs`, `place/write.rs` — compile-forced); `turn.rs` `apply` (no loop test — Remaining);
`Session::apply` theme arm (`theme_tests` green); `switch_list` (32 tests unedited, green); view draw order (palette
after `draw_box_bars`, before `draw_modal_boxes`; in the pixel cut); captures — only the two new ones; landing test
`detail/landing/tests.rs` `a_key_during_a_landing_drops_it` moved from `?` to `F20`, name kept, green. Journal/undo: `git diff --stat` over `crates/graft/src/update/journal*` and
`crates/graft-ops/src/journal*` is empty.

**Remaining** (for the conductor to split):
1. Empty state: `OutsideState.palette`, open on `?`/`⌃P`/`:` in `empty_state::list_key`, edge routes to it, `⏎` via
   `outside_key(default.to_press())`, theme rows → a new `OutsideStep::Retheme` handled in `event_loop/places.rs`
   (needs a mutable session there), draw over `EmptyState`; tests in `empty_state/tests.rs` + sweep.
2. Loop-level theme test (`event_loop/tests.rs`, `pixel_flushes`): `⌃P` `theme light` `⏎` then a dark `Theme` wake →
   one `Flush::Theme`, still light.
3. Refusal parity test (HEAD detached: `⌃P` `merge` `⏎` vs `m`, same status).
4. Confirm sweep: assert the whole `Seen` unchanged, not just "no palette".
5. J7 truth by git needs a pty or loop-level test (the handover is the loop's).

## NOTES — Continuation 1 (builder, 2026-10-07)

**Not compacted.** Scout: the first builder's scout file and its NOTES "Remaining" sufficed (the empty-state loop
it missed is now read: `event_loop/places.rs` `outside_loop`); no new scout — every file touched was read in full.

**Test-first:** the empty-state tests (`empty_state/palette_tests.rs`) were written before the code; red was a
compile failure (missing `palette_rows`, `render`, fields), not a failing run. The refusal-parity and confirm-sweep
tests were green at first run — they pin behaviour the first builder built. The loop theme test was **red on a real
run** (2 flushes, wanted 1) and found the defect below.

**Found and fixed (cause):** `event_loop/running/turn.rs` `terminal_changed` flushed the strip cache whenever
`Session::apply` returned `true`, and `apply` returns `true` for **any** new hint (session.rs `apply`, "The hint
changed, so something drawn from it did") — even when his palette choice (or a forced `ui.theme`) keeps the theme as
it was. Every strip was thrown away and re-rasterised for nothing. Now the flush keys on the theme itself changing
(`StripPaint::of` paints from `session.theme`); the frame is still owed. Cost to him: none visible; saves a full
re-raster on a background re-detection under a chosen theme. **Existing test edited:**
`a_new_cell_size_or_theme_flushes_the_strips` used an `Assumed` light hint, which `ThemeKind::select` reads as dark
(theme/mod.rs:213) — it changed no strip and passed only through the conflation. It now uses a light `Background`
read, and gains the inverse case (a hint that leaves the theme dark: 0 flushes).

**Empty state built** (`empty_state.rs`, `event_loop/places.rs`): `OutsideState` gains `palette` and
`config_problems` (his config problems, also shown here; still seeded on the first repository screen after).
`outside_edge` routes every key to the palette while it is up (a text field); after translation, `act` opens it on
`?`/`⌃P`/`:` when no prompt is up — the init box swallows (§2.3c), the open prompt types them. `⏎` runs the row's
default chord through `act`; a theme row returns `OutsideStep::Retheme(kind)`, which the loop answers with
`Session::choose` (no strips to flush on this screen). `render` draws the list, the palette over the screen bar the
bar row, and `↑↓ move  ⏎ run  esc close` on the bar row.

**Questions (not deviations):**
- A theme chosen on the start screen lasts while he is there; opening a repository settles a new `Session`
  (`run_repo`), so the choice does not carry over. "This run only" (§2.12) may mean it should. Not built; ask.
- The start screen's `?` lists, under EVERYWHERE, every `Global` row `reachable(EmptyState)` gives — `u`, `U`, `⇥`,
  `/`, `.`, `⌃R`, … — which the list swallows (`empty_state/sweep_tests.rs` `SWALLOWED_BY_DESIGN`, `LIST`); and
  under HERE `c Clone…`/`t Practice tour` (tickets 18/19) and `n No — don't init` (swallowed on the list). Decision 2
  lists by `reachable`, so this is to spec; the capture shows it. Whether help should hide rows that do nothing on
  this screen is the planner's call.

**Not done:** J7's truth by git (the `[~]` above) — `ContinueInGit` is a terminal handover the loop carries out
through `Outside`'s `Handover`, which the headless and test loops do not have (`Outside::default()`); needs the pty
walk. Blame find's `?`/`q` drop stays (gap row, 09e).

**Touches:** `outside_edge` callers — `places::outside_loop` only (grep `outside_edge`), start-screen sweep green
(`ACTING_AT_LANDING` 13 holds); `OutsideState` literals (2 test files, `places.rs`); `OutsideStep` matches —
`places.rs` only (grep `OutsideStep::`); `update::palette` reused unchanged (`listing`, `opened`, `opener`, `edit`,
`view_of`, `bar`, `default_chord`); `terminal_changed` — every `event_loop::tests` theme/flush test green (25 by
filter `theme|flushes`), pixel captures unchanged (workspace run, no shot diff but the new file); sweep tables'
`file:line` citations moved to the new lines (`empty_state.rs:302`, `:257`, `:256`); MAP.md (`empty_state`,
`places::drive`, `running` split). Journal/undo: no file under `update/journal*` or `graft-ops/src/journal*` in the
diff. Keymap: no rows added or moved.
