# G-palette 1b — the bar's doorway is `? keys`; four hint rows; the palette takes the mouse

Status: draft (planner spec-only split, 2026-10-07, from scout `.scratch/m8-polish-and-ship/scouts/11-palette-help.md`;
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
- [ ] Doorway: every bar `every_context()` yields at 80/120/200 ends with `? keys`, never `. more` (sweep test); `.
      actions` drawn at 200 on the graph, dropped before any other candidate as width shrinks (test). `[keys.app]
      help = "none"` → doorway `⌃P keys`; both none → no doorway, no `—` (test).
- [ ] `every_key_the_bar_draws_is_answered` green and covers `? keys` and `. actions` (named in the test's cases).
- [ ] 163: 80×24 graph — bar ends `? keys`; `?` lists `space`, `1`, `H` under HERE (test).
- [ ] 538: Detail tab under 100 columns — no `1` in its bar; graph bar at 120 still has it (test).
- [ ] 539: cursor on HEAD's row (unpushed, not blocked) → bar has `a amend`; NOTES quotes `NEVER_HINTED` before/after
      (test `a_amend_is_hinted_on_heads_row`).
- [ ] 553: test reproducing it fails before, passes after; NOTES has the trace line (or the stop).
- [ ] Mouse: click selects, double-click runs (J6's row → stash prompt), wheel scrolls, outside click closes with no
      command, palette beats an open picker's hit area (tests in `update::tests` mouse).
- [ ] Captures: `docs/shots/m8-help-rebase-stop-120x36.txt` (rebase stopped, `?`: IN PROGRESS first),
      `docs/shots/m8-help-80x24.txt` (80×24 graph bar ending `? keys`, then `?` open). Every committed capture with
      `. more` re-shot (`GRAFT_UPDATE_SHOTS=1` and the pty scripts that own them); NOTES lists each file; contact
      sheets re-assembled. **Colour unverified** (rule 5).
- [ ] MAP.md graft-ui (`DOORWAY`, `. actions` candidate); graft (palette in mouse routing; 553's fix site).

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
