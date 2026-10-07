# G-palette 1b — the bar's doorway is `? keys`; four hint rows; the palette takes the mouse

Status: partial — 553 stopped for a ruling; Marked-row doorway asked (see NOTES). (was: draft, planner spec-only split, 2026-10-07, from scout `.scratch/m8-polish-and-ship/scouts/11-palette-help.md`;
part 1 is `11-palette-help-and-actions.md`)
Build with: **Sonnet** — mechanical: a hint constant and its rank, tests and captures re-shot, four rulings already
made, mouse wiring onto 11's pure hit-test. Row 553 is trace-first with a stop. Escalate per policy if it stalls.
Blocked by: **11** (palette exists: `?` must open something before the bar names it; `palette_area`,
`palette_row_at`, `AppState::palette`). Conductor: if any 10b-family ticket (being re-split now) touches
`graph_screen/hints*` or `empty_state*`, serialise with it.
Needs: from 11 — `palette_area(Rect, Rect) -> Rect`, `palette_row_at(Rect, &PaletteView, u16, u16) -> Option<usize>`,
`update::palette::{view, palette_key}`, `AppState::palette`; from 10/10c — `HintKey::{Key, Fixed}`, hints over
`ActionId`, `empty_state::bar_keys(view, keys)`.
**Crate set:** graft-ui (`graph_screen/hints.rs`, `hints/catalogue.rs`, `hints/lists.rs` for 538, `hints/tests.rs`,
`empty_state*` bar, MAP.md) + graft (`update/mouse.rs`, `view/rows.rs` for 553 if the trace lands there,
`update/tests.rs` `NEVER_HINTED`, empty-state bar tests, MAP.md) + every capture with `. more` (re-shoot).

## Read first
- `docs/design/02-keymap.md` §2.1 bullets "The hint bar is a ranked selection" (doorway last, never dropped;
  everything bound is a candidate) and the modal-pane rule; §2.12 (:526–541)
- `docs/design/01-view-mockups.md` §1.16 (:471–498) — mouse sentence only
- `docs/design-notes/m8-polish-and-ship.md` New UI bullet "Bar doorway becomes `? keys`" (:347)
- `11-palette-help-and-actions.md` decisions 8–9 (modal, box)
- `crates/graft-ui/MAP.md` `graph_screen::hints` (doorway, `every_context`, `coverage_slot`), `HintContext::ChangesPane`
  (:193), `empty_state`; `crates/graft/MAP.md` `view::rows::row_verbs` (:213), `view` (:22 `draw_status_or_hints`,
  the Detail tab's hint row), `update::picker` (mouse order)
- Source after the maps: `graph_screen/hints.rs:355` (`MORE`), `:575`; `hints/catalogue.rs:544`; the six tests at
  `hints/tests.rs` 258, 981, 1053, 1068, 1137, 1204; `update/tests.rs:5751` (`NEVER_HINTED`)

## Decisions (planner; not the builder's)
1. **Doorway.** `MORE = hint(MENU, "more")` becomes the doorway `hint(app.help, "keys")` — spelled from the table
   (`?` by default, his remap otherwise), drawn last, never dropped, in every non-modal bar (graph, panes, places,
   overlays, empty state). `.` stays bound and becomes an ordinary candidate `. actions`, ranked last before the
   doorway (§2.1: everything bound is a candidate), so it drops first at narrow widths. `app.help` unbound → the
   doorway is `⌃P keys` (`app.palette`); both unbound → no doorway (an omission, never `—`; 10's decision 2).
2. **Row 163.** At 80 columns `space`, `1`, `H` may drop from the graph's bar; they are reachable because `? keys`
   is drawn and `?` lists them under HERE. No ranking change for them.
3. **Row 538.** On the Detail tab (sub-100-column tab layout, graph hidden) the bar does not offer `1 first-parent`
   (its effect is off-screen; ADR-0041 carve-out). Elsewhere unchanged.
4. **Row 539 — `a amend` hinted where it acts.** Graft MAP :213 records m7-40 adding `amend` to `RowVerbs` (HEAD's row,
   `!amend_blocked`) with the words "`a amend` left `update::tests::NEVER_HINTED`" — read as *removed*, but not
   checked against code (rule 7). **Builder check, first step:** read `NEVER_HINTED` (`update/tests.rs:5751`), NOTES
   quotes the line. Present → remove it there; absent → NOTES says so. Either way the acceptance test below holds.
   Older unpushed rows are ticket 15's ruling 3 — out.
5. **Row 553 — overlay up, changes pane focused: the bar draws the pane's live keys.** Not traced (rule 7). **Trace
   first:** reproduce with a test, NOTES names the `file:line` where `hint_context`/`footer_context` picks the overlay's
   context over the focused pane's. Fix there if the fix is choosing the focused place's context (the modal-pane rule:
   the bar is the focused place's). If the cause is elsewhere or needs a ruling: stop, NOTES, report.
6. **Mouse** (§1.16, §2.12 modal): while the palette is open it is first in mouse routing (before picker, menu, panes).
   Click a row = move the cursor there; double-click = run it (same as `⏎`); wheel scrolls the list (cursor follows,
   clamped); click outside `palette_area` closes it (like `esc`, nothing run). Double-click = two clicks on the same
   row within the existing double-click rule the picker uses (reuse, don't add a timer).

## Scope
In: decisions 1–6; `every_key_the_bar_draws_is_answered` covering `? keys` and `. actions`; the doorway tests (the six
named above) re-pointed; every committed capture showing `. more` re-shot; the two palette captures 11 left.
Out: palette content, filter, theme (11); bar rulings at stops / search / stash rows / older unpushed rows (15).

## Signatures
No new `pub` items. Changed: `graph_screen::hints::MORE` → `DOORWAY` (pub(crate) const or fn — whichever the
`HintKey` pattern needs), `MENU`'s entry becomes an ordinary candidate with verb `actions`. MAP.md records both.

## Acceptance (each tick names its test, at tick time)
- [x] Doorway: every bar ends `? keys`, never `. more` at 80/120/200 —
      `graft_ui::graph_screen::hints::tests::every_bar_ends_with_the_keys_doorway_and_never_more`; `. actions` last
      before the doorway and dropped first — `…::actions_is_drawn_wide_and_dropped_first` (**not at 200**: on a
      current-branch row it is first drawn at 267 columns, every other candidate ranks ahead); `help = "none"` →
      `⌃P keys`, both none → nothing, no `—` — `…::the_doorway_falls_back_to_the_palette_key_then_to_nothing`.
      Exception: `Marked` rows (see NOTES).
- [x] `every_key_the_bar_draws_is_answered` green, covering `? keys` and `. actions` (drawn on every row context;
      `app.help`/`app.palette` added to `BOUND_ACTIONS`) — `graft::update::tests::every_key_the_bar_draws_is_answered`,
      `every_bound_action_is_hinted_live_or_is_named_here`.
- [x] 163: 80×24 graph bar ends `? keys`; `?` lists `space`, `1`, `H` under HERE —
      `graft::update::tests::keymap_edge::palette::at_80_the_bar_ends_with_keys_and_help_lists_what_it_dropped`.
- [x] 538: Detail tab under 100 columns — no `1`; graph bar at 120 keeps it —
      `…::keymap_edge::palette::the_detail_tab_offers_no_first_parent`.
- [x] 539: HEAD's row has `a amend` at 80 and 120 — `graft::it hint_bar::a_amend_is_hinted_on_heads_row`.
      `NEVER_HINTED` before (and after, unchanged): `update/tests.rs:5752-5774` lists only `app.top`, `app.bottom`,
      `app.page_up`, `app.page_down`, `app.refresh`, `app.redo` — **`a amend` absent**; graft MAP :213's "left" means
      removed, and it was.
- [ ] 553: **stopped for a ruling** (NOTES). Repro written and run red; not committed (no disabled tests).
- [x] Mouse: click selects — `…::palette::a_click_in_the_palette_moves_the_cursor_only`; double-click runs J6's row
      as `S` — `…::a_double_click_runs_the_row_as_its_key`; wheel — `…::the_wheel_moves_the_palette_cursor`; outside
      click closes, nothing run — `…::a_click_outside_the_palette_closes_it`; first in routing —
      `…::the_palette_takes_the_click_before_the_menu` (tested over the `.` menu, not the picker: the picker is a text
      field, `?` types into it, so the palette never stands over it — `update/palette.rs` `open_key`).
- [x] Captures: `docs/shots/m8-help-rebase-stop-120x36.txt` (`palette_in_progress::j7_…`),
      `docs/shots/m8-help-80x24.txt` (`at_80_the_bar_ends_…`); 71 committed captures re-shot by
      `GRAFT_UPDATE_SHOTS=1`, 6 insta snapshots, m6/m7 contact sheets re-assembled (`scripts/contact-sheet.sh
      --captions-order`). **Colour unverified** (rule 5).
- [x] MAP.md graft-ui and graft: `**m8/11b**` rows (checked by reading).

## Touches
- Every non-modal bar and its tests (`hints/tests.rs`, `every_context`, coverage slots); the empty-state bar
  (`bar_keys`) and its tests; 18 and 19 add `c clone…` / `t practice tour` to the same bar after this (serial).
- Captures: grep `docs/shots` for `. more` before and after — after must be zero outside `docs/shots/m[1-7]*` history
  the conductor chooses to freeze (NOTES lists any left with why).
- Context menu `.` itself unchanged (still opens the menu). `NEVER_HINTED` (decision 4). Mouse routing order
  (picker, menu, drag). Docs: `docs/design/02-keymap.md` already says `? keys` — no edit.
- Journal/undo: none.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~55k: doorway + rank + tests 14k; 163/538/539 8k; 553 trace + fix 8k; mouse 8k; re-shoots and two captures 12k;
MAP 3k. If the re-shoot list alone exceeds ~25 files, captures go to a follow-up (conductor).

## NOTES (builder, 2026-10-07)
- Scout `scouts/11-palette-help.md` predates 11's merge; its doorway lines (`hints.rs:355`, `:575`, catalogue `:544`,
  the six tests) held; its "palette not built" half is stale (11 built it). Enough for this ticket, with a re-read of
  `update/palette.rs`, `update/mouse.rs`, `view/rows.rs`.
- **553, stop (rule 7 trace).** Reproduced: overlay (Map/Journal/Stash) up, changes pane focused → bar is
  `PaneOverOverlay` (`⏎ open · j/k move · u undo · / search`), no `space stage`. Site:
  `crates/graft/src/view/rows.rs` `overlay_context`, the final `Some(HintContext::PaneOverOverlay { … })` (≈ line 696),
  reached from `hint_context` before its `on_changes_pane()` branch. Tried choosing `ChangesPane` there: `space`,
  `d`, `⌃A`, `e`, `⇥`, `.`, `?` all answered, but **`c commit` is not** — `c` falls through to the graph, which the
  overlay's `shown_key` catch-all swallows. Ruling needed: (a) bar = `ChangesPane` minus `c`; (b) make `c` reach the
  commit editor from a focused changes pane under an overlay; (c) leave as is. Reverted; repro test not committed.
- **Marked rows keep no doorway** (deviation from decision 1, asked): `? keys` on a marked row costs 9 cells and
  pushed `/ search` off at 92 columns and `u undo` at 83 in `m2` captures; the brief says undo/search must stay.
  `. explain` is still there. Ruling: doorway on marked rows (losing undo/search there) or not.
- `. actions` first fits at 267 columns on a current-branch row, not 200 (it ranks after every candidate, as ruled).
- Doorway placement: kept exactly where `. more` was (row and pane contexts) plus the empty state. The modal
  overlays/places (journal, stash, Map, blame, history, resolver) still draw none, though `?` opens the palette there
  (`keys.rs` `open_key` runs before them): decision 1 lists "overlays, places" but the code and keymap §2.1 call them
  modal. Question for the planner.
- `. more` left in `docs/shots` (frozen history, not re-shot): m3/m5 contact sheets and captions, m5-07 branch-row
  hints, m6/m6-19-after-esc-esc-*, m6/m6-25-menu-fixed-slots-*, m6 sheets 2/3/5 + captions, m6/prototype-journeys,
  m7-proto-*. No test checks them.
- Bar widths: every re-shot diff is the bar line only; a script over `git diff docs/shots` found 0 lines where
  `u undo` or `/ search` was lost.
- Touches: hint bar (every row/pane context), empty-state bar, palette mouse, mouse routing order, `BOUND_ACTIONS`,
  71 captures, 6 snapshots, 2 contact sheets, MAP.md ×2. No keymap/journal change. Literal-key grep test green.
- No compaction.
