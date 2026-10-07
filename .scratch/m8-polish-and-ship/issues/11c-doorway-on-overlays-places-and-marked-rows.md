# G-palette 1c — `? keys` on the overlays, the places and marked rows (11b's two open doorway items)

Status: draft (planner spec-only pass, 2026-10-07, from 11b NOTES "Doorway placement" and "Marked rows keep no
doorway"; rulings recorded in `09e-keymap-sweep-defects.md` "11b's other open items")
Build with: **Sonnet** (provisional — brownfield, ADR-0035: conductor confirms; 11b's scout
`scouts/11-palette-help.md` covers the doorway code, its "palette not built" half is stale). Mechanical: one hint
appended to existing lists with the rule 11b already built; the only judgement (does `?` draw over each place) is a
test with a stop. Not Fable.
Blocked by: **09e** (shares `hints/lists.rs`, `catalogue.rs`, `hint_bar` tests and captures; reads
`ChangesPane.over_overlay`). Serial with **17** (shared captures; source disjoint) — either order, never concurrent.
Needs: from 11b — the doorway hint (`graph_screen::hints::DOORWAY`, spelled from `app.help`, falling back to
`app.palette`, then nothing), `the_doorway_falls_back_to_the_palette_key_then_to_nothing`; from 11 — `?` opens the
palette over overlays and places (`update/keys.rs` `open_key` runs before them), `PaletteMode::Here(ContextId)`; from
09e — `HintContext::ChangesPane { unmerged, over_overlay }`, `pane_over_overlay_list(opens)`.
**Crate set:** graft-ui (`graph_screen/hints.rs`, `hints/lists.rs` — journal, stash, Map, pane-over-overlay, marked,
`blame_place_list`, `history_place_list`; `hints/catalogue.rs`; `hints/tests.rs`; MAP.md) + graft (`tests/it/hint_bar.rs`,
`update/tests.rs` bar tests, MAP.md) + `docs/design/02-keymap.md` (one sentence, below) + captures re-shot.

## Read first
- `09e-keymap-sweep-defects.md` "11b's other open items" (the rulings)
- `11b-palette-doorway-and-hint-rows.md` decision 1 and NOTES "Marked rows keep no doorway", "Doorway placement"
- `docs/design/02-keymap.md` §2.1 hint-bar bullet (:38–59), modal list (:61–113: overlays :100–105, places :83–91),
  §2.12 (:528–542, `?` opens on this place)
- `docs/design/01-view-mockups.md` §1.0 bar bullet (:50–52, "the doorway … is always last"), journal bar (:365)
- `docs/adr/0058-…md` §4 (:46–48 — which contexts are modal for the bar)
- `docs/research/ux-principles.md` rows 4, 6, 10 (:54, :56, :60)
- `crates/graft-ui/MAP.md` `graph_screen::hints` (:25 — ladder, floor, `Marked`, `Journal`/`Stash`/`Map`/
  `PaneOverOverlay` "draw no `. more`"), `HintContext::Blame`/`History`/`BackTo*` (:53); `crates/graft/MAP.md` m8/11b
  row (:1160)
- Source after the maps: 11b's doorway site in `hints.rs` (where `DOORWAY` is appended), the lists named in the crate set

## Decisions (planner; not the builder's)
1. **Overlays and places draw the doorway.** `Journal`, `Stash`, `Map`, `PaneOverOverlay`,
   `ChangesPane { over_overlay: true }` (09e), `Blame` (every form), `History` (every form), `BackToBlame`,
   `BackToHistory` end with `DOORWAY` — last, after `esc close`/`esc back`, never dropped, by 11b's fallback
   (`?` → `⌃P` → nothing). Why: these are modal for keys (§2.1) but not for the bar — ADR-0058 §4 names the bar-modal
   contexts (resolver, banner, boxes, prompts, preview, where "the keys are the point"); `?` answers in all of them and
   lists the place's keys (§2.12); DESIGN §1.0 "the doorway is always last"; ux 4 (`?` means the same on every list),
   6 (keys visible at the point of use), 10 (help for this pane). They keep their own ladder and two-space joins.
2. **Still no doorway** (`?` types or is swallowed there): resolver, banner, confirm/name/credential boxes, the
   picker, the palette, the rewrite editor and its field, the Ghost Preview, `Dragging`, and any text-field sub-state
   of a place (journal find, blame find, the stash save prompt). Unchanged.
3. **Marked rows: `? keys` as an ordinary candidate, ranked last, dropped first.** The one non-modal bar where the
   doorway yields: on a marked row it cost `/ search` at 92 columns and `u undo` at 83 (11b NOTES), and those two must
   stay (10c R1); the row's own `.` is `explain`. So the doorway is drawn whenever it fits after every other Marked
   hint, never pushing one off. Monotone: more width only adds it. `?` answers on a marked row whether drawn or not.
4. **Narrowest bars.** The doorway is in the floor for decision-1 contexts (never dropped). Where floor + doorway no
   longer fits the narrowest reachable bar (48 cells), the existing below-floor ladder applies (floor shrinks first,
   then the doorway alone) — no new rung. NOTES gives each affected context's measured floor.
5. **Doc.** 02-keymap §2.1 hint-bar bullet gains one sentence after "behind the doorway (…)": "*The doorway ends every
   bar but the modal ones ADR-0058 §4 names; on a marked row it is the last candidate and yields to `u undo` and
   `/ search`.*" (planner amendment, 2026-10-07).

## Scope
In: decisions 1–5; `every_key_the_bar_draws_is_answered` covering `?` in each new context; captures re-shot.
Out: what the palette lists (11's file — report only, below); bar ranking (15); the resolver and banner bars; blame
find's dropped help/quit (`M8/blame-find-drops-help-quit`).

## Signatures
No new `pub` items, no signature changes. The lists named in the crate set append `DOORWAY` (decision 1) or rank it
last as a droppable candidate (`Marked`, decision 3). MAP.md records which contexts end with it.

## Acceptance (each tick names its test, at tick time)
- [ ] Decision 1: each listed context at 80, 120 and 200 columns ends `? keys`; `help = "none"` → `⌃P keys`; both
  none → no doorway, no `—`. Red first. (`graft_ui::graph_screen::hints::tests::overlays_and_places_end_with_the_keys_doorway`)
- [ ] Decision 2: none of the listed modal contexts draws `? keys` or `⌃P keys`.
  (`…::tests::modal_bars_draw_no_doorway`)
- [ ] Decision 3: a marked row's bar has `? keys` last at 120; at 83 and 92 `u undo` and `/ search` are drawn and
  `? keys` is not; the bar is monotone in width. (`…::tests::a_marked_row_keeps_undo_and_search_before_the_doorway`)
- [ ] Decision 4: `the_floor_fits_at_forty_eight_cells_of_bar` green or amended with NOTES' measured floors; no bar
  draws a fragment. (that test)
- [ ] `?` is answered and **drawn** over each place: Map, journal, stash (graph and pane focus), blame, history,
  marked row — press `?`, draw the frame: it contains the palette's `keys here` title. A place where `?` changes state
  but the palette is not in the frame is a **stop** (NOTES, report — the unseen-modal class 09e fixed for `/`).
  (`graft::view::tests::the_keys_doorway_opens_a_palette_he_can_see_everywhere_it_is_drawn`)
- [ ] `every_key_the_bar_draws_is_answered` green with the new contexts. (`graft::update::tests::every_key_the_bar_draws_is_answered`)
- [ ] Report only: for Map, journal, stash, blame, history — does `?`'s `HERE` group list that place's keys (one named
  key each, e.g. Map `⏎ seek`), or the graph's? NOTES answers each with the `update/palette.rs` line; a wrong one is a
  finding for 11's file, not fixed here.
- [ ] Captures: `GRAFT_UPDATE_SHOTS=1` once, then unset and green. Allowed diffs: a listed bar gaining `? keys` (and,
  at narrow widths, losing words or hints by the ladder — each listed in NOTES). Frozen history (11b NOTES list:
  m3/m5 sheets, m6/m7 prototypes) untouched. New: `docs/shots/m8/doorway-journal-120x36.txt`,
  `docs/shots/m8/doorway-blame-80x24.txt`, `docs/shots/m8/doorway-marked-120x36.txt`. Bar row judged by the user
  (rule 6). **Colour unverified** (rule 5).
- [ ] 02-keymap §2.1 sentence (decision 5); MAP.md graft-ui (`graph_screen::hints` :25 — "Journal, Stash, Map and
  PaneOverOverlay draw no `. more`" replaced; `Blame`/`History`/`Picker` lines say which draw the doorway) and graft
  m8/11b row. (doc, checked by reading)

## Touches
- Every overlay and place bar and its tests (`hints/tests.rs`, `every_context`, `coverage_slot` — no new shapes);
  `tests/it/hint_bar.rs` expected strings for those bars; the floor test.
- 09e's `ChangesPane { over_overlay }` list and `pane_over_overlay_list` (gain the doorway; 09e's frame oracle stays
  green — `?` now on those bars must change the frame).
- Palette: `?` already answered in these places (11); nothing in `update/palette.rs` changes.
- Captures of journal, stash, Map, blame, history and marked rows; m8 contact sheet re-assembled.
- Keymap: no binding changes. Journal/undo: none.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~50k: lists + doorway placement 10k; marked candidate + monotone test 6k; tests and catalogue 10k; drawn-palette check
6k; re-shoots + three captures 15k; MAP and doc 3k. If re-shoots exceed ~30 files, captures go to a follow-up
(conductor).
