# G-keymap 1e — the nine keys 09d's sweep found idle, the dead `empty_state.cancel` row, and the changes pane's bar over an overlay (11b row 553)

Status: draft (planner spec-only pass, 2026-10-07; rewritten the same day after the scout's stop — ruling on Defects 1–2
re-made against the code, stale lines fixed; from 09d NOTES "DEFECTS found by the sweep" and "D6 knock-on
(question)"; KNOWN_GAPS `M8/09d-empty-state-cancel-dead` and the 09d sweep-defects row. Second rewrite the same day:
11b merged partial — its row 553 folded in here (same `overlay_context`, same over-overlay bar, same swallow oracle);
11b's other three open items ruled below, one of them sent to the new ticket `doorway-on-overlays-places-and-marked-rows.md`)
Build with: **Sonnet** (provisional — brownfield, ADR-0035: conductor confirms; scout done). Every ruling is made
below, each fix is a gate or an arm at a line the scout pinned (553: the line 11b's builder pinned), and 09d's sweep is
the oracle that flips each row. Not Fable: nothing structural is left open.
Blocked by: none (11 and 11b merged). **Order:** before 17 (look polish) — 09e is unblocked now and fixes live defects;
17's item 6 may stop for a planner ruling and must not hold this. 17 shares `view/rows.rs` (`modal_blanked`) and the
capture set: run them serially, never concurrently. After 09e: the doorway ticket (shares `hints/lists.rs`, catalogue,
captures).
Needs: from 09d — the sweep (`update::tests::keymap_edge::sweep`), its tables `SWALLOWED_BY_DESIGN`
(`sweep_swallowed.rs:44`), `DEFECTS` (`sweep_tables.rs:178–188`), `ACTING_AT_LANDING = 396` (`sweep.rs:29`);
`empty_state::sweep_tests`; `empty_state::tests::the_init_box_answers_the_same_keys_in_confirm`. From m6/85 — the
S/J-from-the-pane precedent (`update::keys::tests::s_opens_the_stash_list_from_the_changes_pane`,
`…::j_opens_the_journal_from_the_changes_pane`; landing lines `update/stash.rs:175–176`, `update/journal/keys.rs:43`).
From 11b — `the_bar_behind_an_overlay_draws_no_key_the_overlay_swallows` (update/tests.rs:4105), 11b's `DOORWAY`
(the `? keys` hint, `graph_screen::hints`).
Scout: `.scratch/m8-polish-and-ship/scouts/09e-sweep-defects.md` (on main). Read it before source.

**Files.** Paths under `crates/graft/src/` unless stated.
- graft: the three focus gates — `update/keys.rs:409` (`R`), `update/picker.rs:125` (`C`), `update/strip.rs:17`
  (`O`); the search's overlay guard — `update/search.rs:20` (opener) and `:22` (`n`/`N` step); `empty_state.rs`
  init-box arm `:254–261` (raw `n` at `:256`) + its tests and `empty_state/sweep_tests.rs` (cancel row `:43`);
  `update/tests/keymap_edge/{sweep,sweep_tables,sweep_swallowed}.rs`; `update/keys.rs` tests;
  `update/search/tests.rs` (`slash_belongs_to_every_modal_that_is_up`, `:197`); `tests/it/hint_bar.rs:578–583`
  (expected bar string); `MAP.md`. **553:** `view/rows.rs` `overlay_context` — the final
  `Some(HintContext::PaneOverOverlay { … })` (≈ :696), only its changes-pane case; `update/tests.rs:4105` (the swallow
  test, extended); `tests/it/hint_bar.rs` (one test added). **Doc comments only** (no code): `update/state.rs:906–911`
  (`modal_owns_keys`' "Not the Map, journal or stash overlay … `/ search`, which works there"),
  `update/journal/find.rs:162–164` ("`/` stays the search that pane's bar offers"). **Not** `modal_owns_keys`' body,
  not `graph_has_focus`, not `update/map.rs` (`M` stays), no other `view/*` line (`rows.rs` `modal_blanked` and
  `hint_context` are 17's and untouched), not `update/edge.rs`.
- graft-ui: `graph_screen/hints/lists.rs:324–335` (`pane_over_overlay_list` loses `/ search`) and the changes pane's
  list (`over_overlay` filter, 553); `graph_screen/hints/context.rs:126–129` (doc: "`u undo` and `/ search`" →
  `u undo` only) and `HintContext::ChangesPane` (field added, 553); `graph_screen/hints/catalogue.rs` (`every_context`,
  `coverage_slot`, `COVERAGE_SLOTS` for the new shape; `:483` comment only: "`R` needs the graph's keyboard … never
  under a pane" → the pane bar doesn't offer `R`; ranking is 15 — behaviour unchanged); every `ChangesPane { unmerged }`
  constructor (grep; adds `over_overlay: false`); `keymap/table.rs:495–500` (row deleted);
  `tests/keymap_ids.txt:57`; `MAP.md`. No `keymap/resolve.rs`, no `borrows`. No `keymap/tests/*` names the row
  (scout grep).
- 10b / 10e: no file above is theirs by their specs' crate lines — conductor confirms.

## Read first
- The scout file, whole (one page).
- `09d-keymap-sweeps-and-literals.md` NOTES: "DEFECTS found by the sweep", "D6 knock-on (question)", "Init box"
- `11b-palette-doorway-and-hint-rows.md` NOTES bullet "553, stop (rule 7 trace)" — the repro and the site
- `docs/design/02-keymap.md` §2.1 hint-bar bullet (:38–59, "everything bound is a candidate"), §2.1 modal list
  (:61–113, the Map/Journal/Stash bullet :100–105, search prompt :98–99),
  §2.1 remap paragraph (:174–193, "a line acts where its section is live", "every binding is remappable"), §2.2 `/`
  (:210 "Find in the focused list"), §2.3 `R` `C` `M` `O` (:248–249, :266, :268), §2.3a (:279), §2.3c (:296), §2.4 `M`
  (:308), §2.9 (:470–479), §2.13 (:542–555)
- `.scratch/m6-conflicts/issues/85-usability-2-fixes.md` m2 (:48–49) and NOTES (:221) — S/J from the pane
- `docs/research/ux-principles.md` rows 1, 3, 4, 6 (:51–56)
- `crates/graft/MAP.md` `empty_state` (:6), `AppState::graph_has_focus` (:215), `view::rows` `overlay_context` (:930),
  m8/11b row (:1160), sweep tables (:937); `crates/graft-ui/MAP.md` `## keymap` (`ContextId::borrows`, `reachable`),
  `graph_screen::hints` (:25 — `ChangesPane`, `PaneOverOverlay`, `every_context`, `coverage_slot`)
- Source after the maps: each line in the Files list; `update/mod.rs:1106–1124` (search before `key_event`);
  `view/frame.rs:158–160` and `view/frame/overlays.rs:83–107, 262–349` (what draws under Map/Stash)

## Rulings (planner; not the builder's)

**Defects 1–2 — `/` under the Map and the stash list: today it *is* the graph's search, opened where it isn't drawn.
Defect; fix by making the graph's search stand down under the three overlays.** Checked in code (planner, 2026-10-07):
- What happens: `update/mod.rs:1121` runs `search::search_key` when `!modal_owns_keys()`; the Map, journal and stash
  are kept out of `modal_owns_keys` (state.rs:907–909, body :913–922), so `search.rs:20` sets
  `search_prompt = Some("")` with the overlay up — before any overlay arm (mod.rs:1122–1123).
- Where the prompt is drawn: only the graph screen's footer — `view/rows.rs:94` / `:265` (`GraphView::search`) →
  graft-ui `graph_screen/footer.rs:86` (` /text_`). Under Map/Stash the frame takes `draw_map`/`draw_stash`
  (`view/frame.rs:158,160`), which render `MapScreen`/`StashScreen` and, in the `Tab` layout, only the pane's hint row
  (`overlays.rs:310–349` → `layout.rs:257`). No `GraphView`, so **no prompt line, at any width**. Neither
  `map_screen.rs` nor `stash_screen.rs` draws a search line (grep). So he presses `/`, nothing appears, and every next
  letter goes into an unseen query (search.rs:18–19) until `Esc`; `⏎` seeks a graph he can't see.
- Why state.rs:908's "which works there" passed its test: `the_bar_behind_an_overlay_draws_no_key_the_overlay_swallows`
  (update/tests.rs:4105) counts a hint as answered when state changes (`answered`/`did_something`, tests.rs:4533–4547)
  — opening an invisible prompt changes state. It proves "something moved", not "he can see it".
- The same cause reaches two places 09d's sweep did not flag (no cell for them): **the pane focused over any of the
  three overlays** — the `PaneOverOverlay` bar offers `/ search` (lists.rs:332; the Journal too, since J's own find
  answers only with graph focus, journal/find.rs:165–170) and it opens the same unseen prompt; and **`n`/`N` with a
  kept graph search** — search.rs:22–25 steps it with no overlay check, moving the cursor of a graph that isn't shown.
- Options: (a) draw the prompt over the overlay — but the query searches graph rows the overlay hides, and its hits
  land on a hidden cursor: a visible prompt over invisible results. Against §2.2 :210 (`/` finds in the *focused*
  list; the focused list is the Map's branches or the stashes, not the graph) and ux row 1 (status visible).
  (b) `/` idle under the three overlays; the bar stops offering it. (c) build find in the Map and the stash list —
  new feature, no prior-art study (pipeline step 2); J's and blame's finds were designed (§2.8, §2.11).
  **Take (b).** One rule at the cause: **the graph's search answers only while the graph is what's shown** —
  `search_key`'s opener (search.rs:20) and step (:22) add `next.graph_overlay().is_none()`; the open-prompt branch
  (:18) is unchanged. J's find still answers first with graph focus (mod.rs:1109), so J at graph focus is unchanged.
  Every other overlay `graph_overlay()` names is already in `modal_owns_keys`. Reject widening `modal_owns_keys`: it
  also drives the banner tab and bottom-row rungs — knock-on far past one key. ux rows 1 (no unseen mode), 3 (no mode
  without a visible exit), 4 (`/` means find in the focused list), 6 (the bar offers only what acts).
- He sees: Map or stash list open, `/` → nothing changes; `j`/`k` still move the overlay's row. Detail tab over any of
  the three overlays: bar ` ⏎ open  j/k move  u undo` (was `…  / search`), `/` → nothing. A kept search's `n` under an
  overlay → nothing; close the overlay, `n` steps it as before.
- Sweep rows (Map `/`, Stash `/`) flip to `SWALLOWED_BY_DESIGN`, why `search.rs:20 graph search stands down under an
  overlay (§2.2 :210 find in the focused list; the prompt draws only in the graph footer)`.
- The doc comments at state.rs:906–911 and journal/find.rs:162–164 say the opposite today — rewrite them to this rule.

**Defects 3–8 — Changes and Diff, `R` `C` `O`: defect, fix (act as from the graph).** The detail pane is not modal
(§2.1's list :61–113 does not name it), so the graph's rows are live there (§2.1 :187 "a line acts where its section
is live"; graft-ui `scope()` puts Graph in both). `c a m r b t f p` act from the panes (09d NOTES), and since m6/85 m2
so do `S` and `J` — graft MAP :215 still says they read `graph_has_focus`, stale. `R`/`C`/`O` are the last
uppercase graph keys gated on `graph_has_focus` (keys.rs:409, picker.rs:125, strip.rs:17) for no reason the design
gives; pressing one in a pane does nothing, silently (ux rows 4, 6). Remove the `graph_has_focus` term from each
gate; keep every other term (`R`'s `graph_overlay().is_none()`; `C`'s plain-key, `Screen::Graph`, no picker, no
overlay, `!modal_owns_keys()`; `O`'s plain-key and `!map.shown`).
- **Landing — one rule, m6/85's:** a key that opens something taking the keyboard (`R` rewrite editor, `C` picker,
  as `S`/`J`) **moves focus to the graph, then opens exactly as from the graph**; `Esc` out of it lands on the graph
  focused, the detail pane still open (85 NOTES :221). `R` acts on the graph's cursor row (§2.3 :248). A key that takes
  no keyboard (`O` toggles the strip) acts where he is: focus stays in the pane.
- Count, "switch branch while reading a diff": before `esc C <name> ⏎` (4), after `C <name> ⏎` (3). Same −1 for `R`.
- Rows flip to acting. `ACTING_AT_LANDING` 396 → the new count (≥ 402), dated.

**Defect 9 — Diff `M`: intended, no behaviour change; list it.** §2.3a :279: `M` opens the Map "from the graph pane".
In the detail pane `M` is the mergetool's letter (§2.4 :308, §2.6a :375); making it Map in the diff of a conflicted
file would give one letter two meanings in one pane (ux row 4). Options: (a) Map from Diff; (b) mergetool from Diff
(a new Diff row — a spec change, not a defect fix); (c) idle, listed. **Take (c).** Trade-off: `M` in the diff pane
does nothing; the bar never offered it there.
- Row flips to `SWALLOWED_BY_DESIGN` with why `§2.3a :279 Map opens from the graph pane; M in the detail pane is the
  mergetool's (§2.4 :308)`.

**Row 553 (from 11b) — overlay up, changes pane focused: the bar draws the changes pane's keys that answer there, and
no other. Fix (a): `ChangesPane` minus what the overlay swallows.** Traced by 11b's builder (NOTES, rule 7):
`view/rows.rs` `overlay_context` returns `PaneOverOverlay` for a focused changes pane (≈ :696, reached before
`hint_context`'s `on_changes_pane()` branch), so the bar reads `⏎ open · j/k move · u undo · / search` while `space`,
`d`, `⌃A`, `e`, `⇥`, `.`, `?` all answer — live keys he is never shown (§2.1 :38–59 "everything bound is a
candidate"; ux row 6). Choosing `ChangesPane` draws `c commit`, which falls through to the graph and the overlay's
`shown_key` swallows.
- Options: (a) bar = the changes pane's list minus every key the overlay swallows (`c commit`; `/ search` by Defects
  1–2 above); (b) make `c` reach the commit editor from the pane under an overlay; (c) leave as is.
  **Take (a).** (b) rejected: the Map, journal and stash are modal (§2.1 :100–105) and the frame draws the overlay, not
  the graph screen, in their place (`view/frame.rs:158,160`) — a commit editor opened under one is the unseen-modal
  failure Defects 1–2 remove for `/`; and `c` is the graph's verb reached through the graph, which is exactly what an
  overlay takes away (09e's one rule: graph keys answer only while the graph is shown). (c) rejected: live keys
  never drawn (ux row 6). Trade-off: `c` in the changes pane commits everywhere except under an overlay — reached by
  closing the overlay first, as today (ux row 4 dented, not broken: the bar never offers `c` there).
- Mechanism: `HintContext::ChangesPane` gains `over_overlay: bool`; `overlay_context` returns
  `ChangesPane { unmerged, over_overlay: true }` where it returns `PaneOverOverlay` for a focused changes pane today
  (the commit's files/diff case keeps `PaneOverOverlay`). The list with `over_overlay` omits `c commit` and `/ search`
  and every graph-dispatched key the swallow test finds; it keeps 11b's doorway rule as `ChangesPane` has it. Every
  other constructor passes `false` (grep, NOTES lists them).
- **The oracle is the frame, not the state** (Defects 1–2's lesson: "something moved" ≠ "he can see it"). For each key
  the over-overlay changes bar (and `PaneOverOverlay`'s) draws, on a fixture where the key has something to act on:
  pressing it changes the drawn frame or emits a `Command` (`e` hands over to `$EDITOR`). A key that changes nothing →
  dropped from the `over_overlay` list (NOTES names it). A key that changes `AppState` but not the frame (opens a
  menu, prompt or box the frame doesn't draw under the overlay — check `.` first) → stands down under an overlay at its
  opener with `graph_overlay().is_none()`, as `/` does, and its hint drops; NOTES names the opener line. **Stop** if
  more than two keys need that, or if an opener is shared with the graph-shown path so a guard can't separate them.
- He sees (Stash open, Detail tab, changes pane focused, one unstaged file): bar ends `… u undo · ? keys` with
  `space stage` among the hints, no `c commit`, no `/ search`; `space` stages the file (the pane redraws); `c` does
  nothing.

**11b's other open items — ruled here, built elsewhere or not at all:**
- **Overlays and places draw no `? keys` (11b NOTES).** Fix, in the new ticket
  `doorway-on-overlays-places-and-marked-rows.md` (after 09e: shares `lists.rs`, catalogue, captures; no
  `overlay_context` code, so not folded). Why: 11b's decision 1 named them; ADR-0058 §4's bar-modal list (resolver,
  banner, boxes, prompts, preview — "there the keys are the point") does not; `?` answers there (`keys.rs` `open_key`
  runs first); DESIGN §1.0 (01-view-mockups :50–52) "the doorway is always last"; ux rows 4, 6, 10.
- **Marked rows have no `? keys` (11b NOTES).** Fix, partial, same new ticket: `? keys` drawn on a marked row as an
  ordinary candidate ranked last, dropped first — the one non-modal bar where the doorway yields, because `u undo` and
  `/ search` must stay (10c R1) and the row's own `.` is `explain`. Details there.
- **`. actions` first draws at 267 columns, not 200.** Accept, no code. Decision 1 ranked it last before the doorway
  (§2.1 :54–55, everything bound is a candidate — drawn when it fits, never promised a width); the 200 in 11b's tick
  was an assumption, already corrected there. `.` stays one key away through `?` (it lists `. actions` under HERE).
  Re-ranking is ticket 15's.

**`?` and the palette do not read the sweep tables** (corrected 2026-10-07). `SWALLOWED_BY_DESIGN`/`DEFECTS` are
test-only consts; their readers are `update/tests/keymap_edge/*`, `empty_state/sweep_tests.rs` (its own table) and
graft MAP.md:937 (scout grep; planner grep agrees). 11's lists come from the keymap's resolution —
`update/palette.rs:271–288` (`place_rows` over `Keymap::reachable`) and `:299–308` (`key_in`: `acting` +
`translate`). So listing a row as swallowed here does **not** hide it from `?`. **Check and report, don't fix:** at
the Diff place, does `?`/the palette list `graph.map` with `M`; at the Map and stash places, `app.find` with `/`? Each
"yes" goes to NOTES as a finding with the `palette.rs` line that lists it (rule 7) — a separate ticket, 11's file.

**Question — `empty_state.cancel` after D6: dead, delete the row; the init box stops answering raw `n`.** §2.13
(:546–555) has no cancel/`n` row; the init box is a confirm box (09d D6), where `Esc` cancels and **any other key is
swallowed** (§2.3c :296). After D6 the row is reachable only on the list, where `n` does nothing; in the box, `n`
declines through a raw arm (empty_state.rs:256, `KeyCode::Char('n') | KeyCode::Escape => Stay`) no row names — a key
his config cannot move (§2.1 :174 "every binding is remappable"). Options: (a) delete the row and the raw `n` arm;
(b) add `confirm.no = "n"` for every box — a second dismiss key in every confirm box, a §2.3c change, against ux row 3
(one obvious exit); (c) keep the row, map it to the box's `n` — a row for one undocumented key. **Take (a)**,
pre-release, no alias (as 09c removed `empty_state.confirm`). Edit `:256` to `KeyCode::Escape => Stay`; `n` then
takes `:257–259` (prompt re-set, box stays). He sees: init box `y init · esc keep it` (unchanged — `n` was never
drawn); `n` → box stays.
- Stop condition checked by the scout, **not met**: the shared confirm handler swallows raw `n`
  (`update/confirm.rs:192–196`, `_ => {}`), so the init box joining it is consistent; no non-test site reads
  `empty_state.cancel` (graft-ui `src/empty_state.rs:122` draws "cancel" from `app.back`, not this row).

**Out:** the `ONE_CHORD_IDLE` questions (resolver/banner `⌃Z`, blame `home`/`end`, changes `home`) — not ruled here;
the hint bar's ranking and the pane bar offering `R`/`C`/`O` (`hint_context` still reads `graph_has_focus`; ranking
is 15); `?`/palette content (11's file — report only, above); `M8/blame-find-drops-help-quit` (separate path, scout §6);
the doorway on overlays, places and marked rows (new ticket, above).

## Signatures
No new `pub` items. One field added.
```rust
// graft::update::search — guard added, signature unchanged:
pub(super) fn search_key(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool;
//   opener (:20) and step (:22) also require `next.graph_overlay().is_none()`
// graft-ui::graph_screen::hints::lists — `hint(FIND, "search")` removed:
fn pane_over_overlay_list(opens: bool) -> Vec<Hint>; // [⏎ open]? j/k move, u undo
// graft-ui::graph_screen::hints::context — 553:
pub enum HintContext { /* … */ ChangesPane { unmerged: bool, over_overlay: bool }, /* … */ }
//   over_overlay: true only from graft's overlay_context; list omits `c commit`, `/ search` and what the oracle drops
// graft::view::rows — signature unchanged; a focused changes pane under an overlay → ChangesPane { unmerged, over_overlay: true }
fn overlay_context(state: &AppState) -> Option<HintContext>;
// graft-ui::keymap::table — row deleted (golden list tests/keymap_ids.txt loses `empty_state.cancel`)
// Binding { action: ActionId("empty_state.cancel"), context: ContextId::EmptyState, default: &[n], .. }  // gone
// graft::update::tests::keymap_edge — DEFECTS ends empty and is deleted with its loop (sweep.rs:19,44,194);
//   nine rows moved: 6 → acting (none listed), 3 → SWALLOWED_BY_DESIGN (Map "/", Stash "/", Diff graph.map)
const ACTING_AT_LANDING: usize; // raised to the new count, date 09e's
```
(`overlay_context`'s parameter is as graft MAP :930 names it; if the code differs, keep the code's signature.)

## Acceptance (each tick names its test, at tick time)
- [ ] **Defects 1–2, graph focus.** Map open, `/` → `search_prompt` is `None`, `AppState` equal to an unbound key's;
  then `j` moves the Map row. Same for the stash list. Red first.
  (`update::search::tests::slash_under_the_map_opens_no_search`, `…_under_the_stash_list…`)
- [ ] **Defects 1–2, pane focus.** Detail pane focused over each of Map, journal, stash: `/` → `search_prompt` `None`.
  Red first. (`update::search::tests::slash_from_the_pane_over_an_overlay_opens_no_search`)
- [ ] **Kept search under an overlay.** A kept graph search, open the Map (then the stash list): `n` → history cursor
  unchanged; close it, `n` → steps. Red first — or, if opening the overlay already clears `search`, NOTES names that
  line and the test stays as the guard. (`update::search::tests::n_under_an_overlay_leaves_the_graph_where_it_was`)
- [ ] **Never a prompt that isn't drawn.** For Map, journal, stash × graph/pane focus × 80×24 and 120×40: press `/`,
  draw the frame; either `search_prompt` is `None` or the frame contains ` /_`. Fails today for Map/Stash.
  (`view::tests::a_search_prompt_is_never_open_where_it_is_not_drawn`)
- [ ] **Bar.** `the_bar_behind_an_overlay_draws_what_the_pane_still_answers` (tests/it/hint_bar.rs:578) expects
  ` ⏎ open  j/k move  u undo` plus 11b's doorway as that bar has it today (NOTES quotes the before/after string);
  `the_bar_behind_an_overlay_draws_no_key_the_overlay_swallows` green; graft-ui hint catalogue tests green.
  `slash_belongs_to_every_modal_that_is_up` unchanged and green (overlays are not modals; the new tests cover them).
- [ ] **553, red first.** 11b's repro, committed: Map, journal, stash × changes pane focused: the bar contains
  `space stage` and no `c commit`, no `/ search`. (`tests/it/hint_bar.rs::the_changes_pane_over_an_overlay_draws_its_live_keys`)
- [ ] **553, swallow test.** `the_bar_behind_an_overlay_draws_no_key_the_overlay_swallows` (update/tests.rs:4105)
  extended to the changes pane over each overlay; green. (that test)
- [ ] **553, frame oracle.** Each key drawn on the over-overlay changes bar and on `PaneOverOverlay`'s, over each of
  the three overlays, changes the frame or emits a `Command`; NOTES lists any key dropped or stood down, with its line.
  (`view::tests::every_key_on_a_bar_over_an_overlay_changes_what_he_sees`)
- [ ] **553, catalogue.** `every_context` yields `ChangesPane { over_overlay: true }` forms; `coverage_slot` exhaustive
  on the new field; `COVERAGE_SLOTS` raised (NOTES: old → new); `every_context_reaches_every_shape` green.
- [ ] **Defects 3–8.** From Changes and from Diff: `C` → picker open, graph focused; `Esc` → graph focused, detail pane
  open. `R` → the rewrite editor on the graph cursor's row, the same editor as `⇥`-to-graph then `R` (state compared);
  `Esc` → graph focused, pane open. `O` → strip toggled, focus still in the pane. Red first, one test per key.
  (`update::keys::tests::{c_opens_the_picker_from_the_panes, r_opens_the_rewrite_editor_from_the_panes,
  o_toggles_the_strip_from_the_panes}`)
- [ ] **Defect 9.** Diff `M` listed in `SWALLOWED_BY_DESIGN` with the cite above; the sweep fails if it acts (09d's
  rule). (the sweep test)
- [ ] **Sweep.** `DEFECTS` deleted; `every_reachable_row_acts_or_is_listed_and_follows_its_remap` green with
  `ACTING_AT_LANDING` at the new count (≥ 402, NOTES gives the number and which rows moved).
- [ ] **Question.** graft-ui: `empty_state.cancel` gone from the table and `keymap_ids.txt`;
  `with_overrides([keys.empty_state] cancel = "x")` → `UnknownAction` (`keymap::tests::rules::empty_state_cancel_is_no_action`).
  graft: init box open, `n` → box stays, no step; `y` → `Init`; `esc` → closed
  (`empty_state::tests::the_init_box_answers_the_same_keys_in_confirm` edited in place for `n`, NOTES says so);
  `empty_state::sweep_tests` green with the row gone (case count in NOTES; its `:43` comment citing `:256` goes too).
- [ ] **`?`/palette check** (report only): NOTES answers both questions in the `?` ruling, each with its line.
- [ ] **Captures.** Full `cargo nextest run` with `GRAFT_UPDATE_SHOTS` set once, then unset and green. The only
  allowed diffs: a pane-over-overlay bar losing `/ search`; a changes-pane-over-overlay bar becoming the changes
  pane's list (553). NOTES lists every changed file under `docs/shots/` and its one-line diff; any other diff is a
  stop. Add `docs/shots/m8/changes-over-stash-bar-80x24.txt` (Stash open, Detail tab, changes pane focused, one
  unstaged file) and, if no committed capture shows the commit pane's bar over an overlay,
  `docs/shots/m8/pane-over-map-bar-80x24.txt` (Map open, Detail tab, files focused) — the visual result is each bar's
  last row, judged by the user (rule 6). Colour unverified (rule 5).
- [ ] MAP.md — graft: `graph_has_focus` read by `M` (and `hint_context`, `offer_keys_live`) only; `S`/`J` (m6/85)
  and `R`/`C`/`O` (09e) answer from the panes; `empty_state` init box answers `y`/`esc` only; the graph's search
  answers only with no overlay up; `overlay_context` returns `ChangesPane { over_overlay: true }` for a focused changes
  pane (the m8/11b row at :1160 closed). graft-ui: the row's removal; `PaneOverOverlay` bar is `⏎`/`j/k`/`u`;
  `ChangesPane.over_overlay`. (doc, no test)

## Touches
- Callers of the three gates and the search: grep each function; the dispatcher tests (`update::keys::tests`),
  `update::search::tests`, m6/85's S/J tests green.
- Other screens drawing this state: the graph footer still draws the prompt whenever the graph is shown (the opener is
  unchanged there — existing search tests); the `PaneOverOverlay` bar changes (above); the changes pane's bar over an
  overlay changes (553); every other bar unchanged — `hint_context` untouched (only `overlay_context`'s changes-pane
  case), captures prove it. Overview strip drawn at the pane-focused sizes: existing captures cover it toggled from the
  graph; no new capture.
- `HintContext::ChangesPane` constructors and matches (graft and graft-ui; grep `ChangesPane {`): each passes or
  ignores `over_overlay`; `every_context`, `marked_form`, `pane_focused_form`, `coverage_slot` (exhaustive matches).
- Any key stood down under an overlay by the 553 oracle: its opener's tests and the sweep (a row may flip to
  `SWALLOWED_BY_DESIGN` — same cite form as Map `/`).
- J's own find: unchanged at graph focus (runs before, mod.rs:1109); its `find.rs:162–164` comment rewritten. Journal
  find tests green.
- Keymap: one row deleted (golden list); no chord moves. 02-keymap.md needs no edit (§2.2 :210 already says it; 553
  follows §2.1 :38–59 and :100–105 as written).
- Journal/undo: none — key routing and the bar only; `R`/`C` open places whose writes are already journaled. Show the
  empty `git diff --stat` over `crates/graft/src/update/journal*` (comment lines in `journal/find.rs` excepted — say
  so) and `crates/graft-ops`.
- 11 (merged): its `?`/palette reads the keymap, not the sweep tables — the check above reports any row it lists that
  doesn't act. 18 (Clone prompt) does not name `empty_state.cancel` (planner grep of `.scratch/m8-polish-and-ship/issues/`,
  2026-10-07).
- 17 (look polish) — `view/rows.rs` (`modal_blanked`) and captures: serial, 09e first. The doorway ticket reads
  `ChangesPane.over_overlay`: after 09e.
- KNOWN_GAPS: `M8/09d-empty-state-cancel-dead`, the 09d sweep-defects row and 11b's row 553 close (conductor).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~56k: three gates + landing tests ~12k; search guard, pane/`n` cases, frame invariant ~10k; bar + hint_bar test +
captures ~5k; row + arm + tests ~8k; sweep tables, count, MAP, `?` check ~7k; 553 (field, catalogue slots,
`overlay_context`, repro, frame oracle, two captures) ~14k. If the 553 oracle stands down a key, NOTES and stop
rather than run past ~70k.
