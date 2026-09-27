# history-place-shown — history reaches the real frame, gets its own bar, and is captured

Status: draft
Remainder of: `21-history-place.md` (partial, merged 2ce85055). **21's scope, Signatures and Rulings (a)–(c) stand**;
this ticket only finishes them. Split: 21b (this) = frame hook, bars, captures, walks; `21c-history-whitespace-and-sizes`
= the two reader gaps.
Blocked by: none (21 merged). Parallel with 10b: see Overlaps.
Crates: graft, graft-ui.
Build with: **Opus** (planner recommends; conductor confirms). Brownfield across two crates; the hint catalogue's
exhaustive sweeps and coverage slots plus a capture producer make the knock-on list long. Nothing structural is left
(rulings made) — fails Fable test 1.
Scout: none new. 21's scout (`.scratch/m7-power-tools/scouts/21.md`) + 21's NOTES name every line this ticket starts
from. Builder confirms each named line still holds before editing (main moved since); a moved line → note it, carry on.
Needs (all landed in 21): `Trail::place_shown`, `Trail::top_history`, `HistoryState`, `HistoryFocus`, `draw_history`,
`view::frame::overlays::draw_history_place`, `place::esc_goes_back`, `update::place::f_key` (guard `blamable`).

## Files (ADR-0051)
Edit only these. Anything else → commit at the seam, report `blocked: needs <file>`, stop.
- **graft:**
  - `src/update/state.rs` — `graph_overlay` only: the `Place` arm asks `trail.place_shown()` (21 NOTES: `:623`).
  - Every other `trail.on_screen()` caller: grep first, list each in NOTES with its verdict (blame-only → keep; means
    "a place is shown" → `place_shown()`). One-line change per call site only. If a site is in a file 10b holds
    (Overlaps), stop and report.
  - `src/view/rows.rs` — `place_context` (History arm), `back_to_blame` → `back_to_place`.
  - `src/view/frame/overlays.rs` — only if `draw_history_place` needs the bar from `place_context`.
  - `src/update/tests.rs` — the sweeps over `every_context()` only (grep `every_context`); no other test there.
  - `src/update/place/tests/{history.rs,hints.rs,esc_back.rs}`.
  - `tests/it/history_place.rs` (capture tests), `tests/it/hint_bar.rs`, `tests/it/pane_bar_space.rs`,
    `tests/it/m7_walk/history.rs`, `tests/it/blame_place.rs` (re-shoot run only; edit only an assert on the bar words).
  - `MAP.md` (`update::place`: drop the "Gap (21, blocked)" sentence; `hint_context`).
- **graft-ui:**
  - `src/graph_screen/hints.rs` (`short_form`, `pane_focused_bar`, `hints_for` arms),
    `hints/context.rs`, `hints/catalogue.rs`, `hints/lists.rs`, `hints/tests.rs`, `src/graph_screen/tests.rs`
    (21 NOTES: `:2068`).
  - `src/blame/snapshots/*.snap` — only if the bar is in them (re-accept, name each).
  - `MAP.md` (`graph_screen::hints`: the two variants, the short form, slots).
- **Docs:** `docs/shots/m7/history-*.txt`, `docs/shots/m7/esc-back-to-history-from-the-graph-120x40.txt`, re-shot
  captures listed under Touches; `TESTING.md` rows for the four new captures.
- **Read only:** `update/keys.rs`, `update/place/{history.rs,history_keys.rs,history_answer.rs}` (21's; a bug found
  there → NOTES, not a fix), every `graft-git` file, `detail_pane/*`.

### Overlaps
- **10b** (`update/switch.rs`, `context_menu*`, `name_prompt*`, `operate*`, `clash*`, `update/preview.rs`, its walk and
  graft-ops tests): **no source file shared.** Watch at merge:
  - `update/tests.rs`: 10b's spec names no edit there. Conductor: confirm from 10b's diff before merging the second.
  - `tests/it/m7_walk/`: 10b adds walks; 21b edits `history.rs` only, no `mod` line. Disjoint unless 10b edits it.
  - `TESTING.md`: both add rows; separate sections, merge-trivial.
  - `docs/shots/m5-hints-detail-*.txt`: 21b may re-shoot (Touches); 10b re-shoots m5/m6 captures its words change.
    Same file from both → conductor re-shoots once after both merge.
- **21c:** after 21b (edits `hints/lists.rs`, `graft-ui/src/history.rs`, `place/history*.rs`).

## Decided (planner, 2026-09-27)
- **Frame hook:** `graph_overlay`'s `Place` arm is taken when `trail.place_shown()`. Nothing else in its precedence
  changes (`Finish`, `Resolver` still first).
- **History's bar** — the note's mock-up words (pass 1b "History", `:184`); modal, no doorway, no `<> resize`, like
  `Blame`:

  | Form | Bar |
  |---|---|
  | `rows: true, on_file: false` | `j/k older·newer  tab file  n/N next change  B blame here  ⏎ that commit  esc back` |
  | `rows: true, on_file: true` | `j/k scroll  tab list  n/N next change  B blame here  ⏎ that commit  esc back` |
  | `rows: false` (tracing, failed, zero rows) | `esc back` |

  - `rows`: the trace answered with ≥ 1 row. Zero rows (depth-1), a `TraceError`, and tracing all draw `esc back`
    alone (21 States; ADR-0037 "never a key that does nothing").
  - `on_file`: `HistoryFocus::File`. Below 120 the one pane shown is the focused one, so `tab file` / `tab list` names
    where `Tab` lands.
  - `v`, `L` stay unadvertised (as in the detail pane's bar); `?` lists them.
  - Floor check (48 cells): first three at full words = 42 (`List`) / 37 (`File`). Fits; no short form needed.
- **`esc back to history`:** `HintContext::BackToHistory(inner)` leads the inner bar exactly where
  `place::esc_goes_back` holds and the trail's top is a history place. Same predicate as `BackToBlame`; one function
  (`back_to_place`) picks the variant by the top's kind.
- **The pane's `F history`:** `PaneFocused { blamable: true, .. }` inserts `F history` right after `B blame`, and drops
  `TAIL`'s `F focus` in that form (there `F` is history, 21 NOTES `f_key`). `blamable: false` → `F focus` stays, no
  `F history`. No new field: `blamable` is `f_key`'s guard too; its doc comment says so.
- **`short_form`:** add `history of these lines` → `history` (a clean prefix: the user's prefixes-only rule holds).
- **Blame's `L`, back to the note's place** (third: `⏎ that commit  ^ before it  L history of these lines  / find …`),
  the rest in today's order. With the short form, the floor's three = `⏎ that commit  ^ before it  L history` = 37 ≤ 48.
  Gate: `the_floor_fits_at_forty_eight_cells_of_bar` green. If it fails anyway, leave `L` after `/ find`, say so in
  NOTES with the measured width; don't change the floor.

## Signatures (exact)
```rust
// graft-ui — graph_screen/hints/context.rs
pub enum HintContext { /* …existing… */
    /// The history place's modal bar (`history_place_list`). `rows`: the trace answered with ≥ 1 row.
    /// `on_file`: the file pane has the keys.
    History { rows: bool, on_file: bool },
    /// The inner bar led by `esc back to history` while a history place waits under the graph.
    BackToHistory(Box<HintContext>),
}
// graft-ui — graph_screen/hints/lists.rs
pub fn history_place_list(rows: bool, on_file: bool) -> Vec<Hint>;

// graft — view/rows.rs (renamed from back_to_blame; same parameters and return as today)
// wraps in BackToBlame or BackToHistory by the trail top's kind; unchanged when !esc_goes_back(state).
pub(crate) fn back_to_place(/* back_to_blame's parameters */) -> HintContext;
```
`every_context()` yields `History` × 3 reachable forms (`(true,false)`, `(true,true)`, `(false,false)`) and
`BackToHistory` over one inner form; `coverage_slot` takes the next free slots; `COVERAGE_SLOTS` grows by 4.
`(false,true)` is not reachable-distinct (same list as `(false,false)`); `catalogue` says why, as `pane_focused_form` does.

## Acceptance (name the verifying test at tick time; red first where marked)
- [ ] **In the frame:**
  - `a_history_place_is_drawn_in_the_frame` (`tests/it/history_place.rs`; **red first**: frame row 0 shows the graph
    header, not `HISTORY · `, with the hook reverted). Drawn frame: title row `HISTORY · <path>`, bar row = the
    `List` form's words.
  - `graph_overlay_is_place_while_history_is_shown` (`place/tests/history.rs`).
  - Every `on_screen()` site's verdict in NOTES, each with the grep line.
- [ ] **History's bar:**
  - `history_bar_words_in_order` (graft-ui `hints/tests.rs`): the table's two row forms, word for word.
  - `history_bar_with_no_rows_offers_esc_back_alone`.
  - `the_floor_fits_at_forty_eight_cells_of_bar`, `every_context_reaches_every_shape` green (named).
  - `every_key_the_bar_draws_is_answered` (graft `update/tests.rs`) green over the History forms (red first: a
    History form with `n/N` drawn and the `n` arm removed must fail it — say in NOTES that you saw it).
  - 20i's bar table (`place/tests/hints.rs`) gains rows: history with rows, list focus; file focus; tracing; failed;
    zero rows. Test `history_rows_in_the_place_bar_table`.
  - `a_depth_one_clone_shows_the_end_row_alone` and `a_trace_error_is_drawn_as_a_failure` (`tests/it/history_place.rs`)
    extended: the drawn bar reads `esc back` and nothing else.
  - `tests/it/hint_bar.rs`: `hint_bar_history_place`, `hint_bar_back_to_history` (one per reachable context, the file's
    rule).
- [ ] **`esc back to history`:** `the_bar_leads_esc_back_to_history_exactly_where_esc_goes_back`
  (`place/tests/esc_back.rs`, mirror of `:146`); `esc_on_the_graph_with_the_pane_open_returns_to_history` now ticks
  whole (Esc half from 21 + this bar half).
- [ ] **Pane bar:** `pane_bar_offers_f_history_on_a_file` (`place/tests/hints.rs`; **red first**: bar lacks
  `F history`); `pane_bar_keeps_f_focus_off_a_file`; `tests/it/pane_bar_space.rs` green.
- [ ] **Blame's bar:** `short_form_history_of_these_lines_is_history`; `blame_bar_words_in_order` with `L` third.
- [ ] **Walks** (`tests/it/m7_walk/history.rs`): `function_history` (`B L`) and `file_history` (`F`) each assert
  `route.within_count()` against `function-history` 2 / `file-history` 1, **and** that the drawn frame after the last
  key shows `HISTORY · ` (red first: the hook reverted).
- [ ] **Captures** (producer: `tests/it/history_place.rs` capture tests, `GRAFT_UPDATE_SHOTS=1`, then a run without,
  all `settled`). Colour unverified. **Right looks like:**
  - `docs/shots/m7/history-lines-187x50.txt`: 4 rows, `▸` on Lina's; file pane at line 18, `┃` on 18–29; line 19 `+`;
    the old line 22 `−` with no number; bar = `List` form.
  - `docs/shots/m7/history-file-140x40.txt`: after `F`; a `↳ was src/session.rs` row; list 40 wide; no `┃`.
  - `docs/shots/m7/history-100x30.txt`: one pane (the list); bar shows `tab file`.
  - `docs/shots/m7/esc-back-to-history-from-the-graph-120x40.txt`: after `⏎` from history; graph shown; bar leads
    `esc back to history`.
  - Fixture: the blame captures' repo (named in `tests/it/blame_place.rs`) if it has 4 commits on `refresh_token` and
    the rename; else build the story in `history_place.rs`'s own builder. Never edit a fixture another test pins
    (grep its users first).
- [ ] MAP.md (both), TESTING.md rows (one per new capture, its "right looks like" line).

## Touches (check each, report how)
- **`graph_overlay` readers:** `view::frame::draw_screen`, `update::banner::graph_screen_on_frame`,
  `view::rows::overlay_context`. With history shown the banner no longer claims the graph. Check: banner tests and
  `tests/it/hint_bar.rs` green; name them.
- **`on_screen()` callers** (keys chain `place_claim`, `mouse.rs`, `draw_place`): per-site verdict in NOTES.
- **Blame's bar and captures:** `L` moves. Re-shoot `docs/shots/m7/blame-{100x30,140x40,187x50,find-187x50,
  long-line-187x50,shallow-187x50}.txt`, `esc-back-to-blame-from-the-graph-120x40.txt` (producer
  `tests/it/blame_place.rs`); blame snapshots only if the bar is in them. NOTES lists each with its changed line.
- **Detail-pane bar:** `F history` on a file, `F focus` gone there. Re-shoot `docs/shots/m5-hints-detail-*.txt`,
  `m2-detail-*.txt` **only** where the capture's pane is focused on a file; find each producer by grep first; not
  found → `could not confirm: producer of <file>`, don't hand-edit.
- **Focus mode:** `F` with the graph focused unchanged. Check: `update/tests.rs:448` green;
  `git diff --stat docs/shots/16-criss-cross-focus* docs/shots/21-redetect-resize-and-focus.txt` empty.
- **`short_form` table:** one row added; every other context's bar unchanged. Check: `hints/tests.rs` green with no
  other expected string edited (say so, or name each edit and why).
- **Keymap doc:** no change (words are the note's). Journal/undo: none (read-only screen).

## Read first
- 21's spec: Rulings, Decided "Bars" and "States", NOTES. Scout 21 §"Blame / detail-bar pins".
- Note pass 1b "History" (`docs/design-notes/m7-power-tools.md:163–194`), P1 (`:1147–1164`), journey counts
  (`:1245–1269`).
- `crates/graft-ui/MAP.md` `graph_screen::hints` (the shrink ladder, `short_form`, `HintContext::Blame`/`BackToBlame`
  line 45), `history`. `crates/graft/MAP.md` `update::place` (`:579–599`), `hint_context` (`:778`), `graph_overlay`
  (`:170`). MAP before source.
- `docs/design/02-keymap.md` §2.17.

## Rules / Gates
Worktree only, own `CARGO_TARGET_DIR`. Never read `PROGRESS.md` or `KNOWN_GAPS.md`. `cargo fmt --check`,
`cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
~55k. One context. If it compacts: captures and re-shoots split off as `21b2`, after the code merges.
