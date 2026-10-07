# G-keymap 1f — the keymap review's four Majors: `resolve.rs` split, `?` at the overlays lists only live keys, the four one-chord idles and `AFTER y`, the keymap seam in ARCHITECTURE.md

Status: **partial** (builder 2026-10-07: A and B done; C, captures, D handed off; see NOTES). Was: draft (planner spec-only pass, 2026-10-07, from `docs/reviews/m8-keymap-sweeps.md` Majors 1–4; review at
main `c18e7d72`)
Build with: **Opus** (confirmed by conductor 2026-10-07 after scout `scouts/09f-keymap-review-fixes.md`). Part B changes the
resolution contract (`scope()` stops over-including at the three overlays) and needs judgement on what the sweep
shows; the rest is mechanical. Not Fable: the sweep is a full oracle (acting count must not move in Part B), so
correctness is judged by tests, not by eye.
Blocked by: **10f** (conductor 2026-10-07: scout found both edit `update/banner/focus_keys.rs`); otherwise none for source (09d, 09e, 11 merged). **17 (building):** may run alongside — this ticket touches no
`view/rows.rs`, no `graph_screen/hints*`, no theme/graph_column/detail_pane file (scout confirms 17's diff does not
touch `update/detail/keys.rs` or `update/place/keys.rs`); the **capture step waits for 17 to merge** (rebase, then
re-shoot). **11c:** serial, **09f first** — shares `update/tests.rs` (one `mod` line), captures, and 11c's report-only
`?` HERE check reads what Part B changes. **12/12b:** never concurrent (palette captures; 12 reads `reachable`); no
`update/palette.rs` source edit here. **10g / 10i:** never concurrent if their file lists name `update/resolver.rs`,
`update/banner/focus_keys.rs`, graft-ui `preview_box.rs` or `rewrite_editor/right.rs` (conductor checks).
Needs: from 09d — the sweep (`update::tests::keymap_edge::sweep`), `SWALLOWED_BY_DESIGN` (`sweep_swallowed.rs`),
`ONE_CHORD_IDLE`, `ACTING_AT_LANDING` (444, 09e); from 09e — the frame oracle in
`update::tests::every_key_on_a_bar_over_an_overlay_changes_what_he_sees` (its press-and-compare helper, reused); from
11 — `update::palette::rows(state, palette)` and `context_of`; graft-ui `ContextId::{scope, borrows}`,
`Keymap::{reachable, acting, translate}`.
Scout: write `.scratch/m8-polish-and-ship/scouts/09f-keymap-review-fixes.md` (≤1 page): `resolve.rs`'s function inventory
with line counts per proposed child; `preview_box.rs` (inline tests? line count after moving them); `scope()` /
`borrows()` bodies and return types; `SWALLOWED_BY_DESIGN`'s key shape (by `ContextId` + `ActionId`?) and its rows at
`Map`/`Stash`/`Journal`, split Graph-context vs Global; which Graph rows **act** at those three (sweep); the four
one-chord handlers at the lines below and how a key reaches each (translated or raw); `right.rs:143–144`.

**Files.** Paths under `crates/graft/src/` unless stated.
- graft-ui: `keymap/resolve.rs` → split (Part A); `keymap.rs` (`scope()`, `borrows()`, new `stands_down()`);
  `keymap/tests/*` (scope/reachable/clash tests); `preview_box.rs` (+ child, Part A); `rewrite_editor/right.rs`
  (:143–144) + `rewrite_editor/tests.rs`; `tests/no_literal_keys.rs` (pattern only); `MAP.md`.
- graft: `update/tests/keymap_edge/{sweep,sweep_tables,sweep_swallowed}.rs`; new `update/tests/palette_live.rs`
  (+ its `mod` line in `update/tests.rs`); `update/resolver.rs` (:379/383); `update/banner/focus_keys.rs` (:147/173);
  `update/place/keys.rs` (:57–58, :78); `update/detail/keys.rs` (:307); their tests; `MAP.md`.
- `ARCHITECTURE.md` (one entry, Part D). `docs/shots/m8/` (one capture). **Not** `update/palette.rs`, not
  `update/edge.rs`, not `view/*`, not `graph_screen/hints*`, not graft-ops.

## Read first
- `docs/reviews/m8-keymap-sweeps.md` Majors 1–4 (:59–118), Minor 2 (:126), Question 1 (:155)
- The scout file, whole
- `09d-keymap-sweeps-and-literals.md` NOTES "Questions (one chord idle)" (:232–235) and its literal table rule (:56, :61)
- `09e-keymap-sweep-defects.md` rulings "Defects 1–2" (graph search stands down under an overlay) and NOTES `?`/palette
  check (:318)
- `docs/design/02-keymap.md` §2.1 remap paragraph (:174–193: :179–180 "translates a pressed chord to the action's
  default chord before any pane sees it"; :187 "a line acts where its section is live"), §2.1 modal list Map/Journal/
  Stash bullet (:100–105), §2.6 resolver `Home`/`End` as `g`/`G` (:344, the precedent), §2.12 (`?`)
- `docs/design-notes/m8-polish-and-ship.md:56` (`?` = **live** keys here); `docs/research/ux-principles.md` rows 4, 6
- `crates/graft-ui/MAP.md` `## keymap` (:393–411); `crates/graft/MAP.md` `update::palette` (:697), sweep tables (:937)
- `ARCHITECTURE.md:72–85`
- Source after the maps: the lines named in Files.

## Rulings (planner; not the builder's)

**Major 1 — files over 400. Fix `resolve.rs` and `preview_box.rs` now; the rest to a gap row.**
- **Part A (pure moves, first commit, public paths unchanged):** `keymap/resolve.rs` (514) → `resolve.rs` (resolution:
  `Keymap`, `Translated`, `resolve`, `holder`, `best`, `rank`, `reachable_rows`), `keymap/spell.rs` (`relabel`,
  `live_labels`, `spell_*`, `key_verb(s)`, `after`, `join`, `word_prefix`), `keymap/apply.rs` (`apply`, `parse_list`).
  `pub(crate) keymap::join` keeps its path (re-export). Minor 2 folded (the duplicated doc line at `:413` is in the
  moved span — delete it, nothing else changes). `preview_box.rs` (445): inline tests → `preview_box/tests.rs` (the
  m7/30h pattern); if still over 400, `Again` → `preview_box/again.rs`. Each file ends under 400.
- **Gap row (conductor opens `M8/keymap-file-sizes`, broken):** `graph_screen/hints.rs` 825, `hints/lists.rs` 562
  (held by 11c and 15 — a split now collides with both; split after 15); `stash_screen.rs` 747, `context_menu.rs` 870
  (graft-ui, outside this group's keymap work; after 15); `graft-ops/src/operation.rs` 1912, `restore.rs` 404,
  `graft/src/update/undo.rs` 1953, `jobs/runner.rs` 1400 + `#[expect(too_many_lines)]` on `dispatch` (held by the
  graft-ops/graft words tickets 10f–10i; `restore.rs` is 4 over — not worth widening this ticket's crate set to
  graft-ops). No overlap with `M8/stop-files-over-400` (pool, exec, apply, resume) or `M8/look-file-sizes`.

**Major 2 — `?` at the Map, stash list and journal lists keys the overlay swallows. Fix at the cause: the graph's
section is not live under an overlay, so those places stop reaching it.** Review :76–90 (`palette.rs:167, 276–310`,
`keymap.rs:151–153`). Cause: `scope()` for `Map`/`Stash`/`Journal` is `[self, Graph, Global]` — MAP's contract "may
over-include, never under-include" — so `reachable(at)` hands `?` every Graph row, and 09e's search stand-down
(Global `app.find`, and `n`/`N`) is invisible to the keymap.
- Options: (a) a hand list of swallowed rows per place next to `borrows()` — ~all Graph rows × 3, drifts; (b) filter
  in `update/palette.rs` by `graph_overlay()` — 12/12b's file, and fixes `?` only, leaving clash and D7 wrong;
  (c) **narrow the three scopes to `[self, Global]`; any Graph row the sweep shows acting there goes into that place's
  `borrows()` (commented with graft's claiming line, as `borrows` already is); Global rows the place answers with
  nothing go into a new `ContextId::stands_down()`, excluded from `reachable`.** **Take (c).** Why: §2.1 :187 (a line
  acts where its section is live) and :100–105 (the three are modal); the design note's `?` row (:56, *live* keys);
  ux rows 4, 6. It also fixes Question 1's D7 case for overlays: a Map line taking a Graph chord no longer drops it
  app-wide (nothing at Map reaches the Graph row). `translate` is unchanged for `stands_down` rows (09e's guard keeps
  the behaviour; this ticket changes what is listed, not what acts).
- Expected `stands_down` (scout confirms from the sweep): Map and Stash — `app.find`, `app.next_match`,
  `app.prev_match` (cite `update/search.rs:20–28` `graph_is_shown`, 09e). Journal: whatever its own find leaves
  (J's find answers `/` with graph focus, `journal/find.rs:165–170` — likely none). Cite each with graft's line.
- **Stops:** (1) more than five Graph rows act at any one overlay (then it is not modal, and (c) is wrong — report);
  (2) a `SWALLOWED_BY_DESIGN` row at the three is neither Graph-context nor Global; (3) the sweep's acting count moves
  in Part B (444 before and after, both measured and in NOTES) — a moved count means a live key was lost.
- **Out (ratchet, not fixed):** `?` from a pane focused over an overlay (`context_of` → `Changes`/`Diff`, which keep
  Graph) still lists graph rows the overlay swallows; Diff `graph.map` (`M`); the start-screen list's `n`
  (09e NOTES :317). The new test records every such row outside the three overlays in a test-only table
  `DEAD_ELSEWHERE` (09d's `DEFECTS` pattern: exact match, a new dead row fails, a fixed one fails until removed);
  the conductor opens `M8/help-lists-dead-keys` from it. Fixing those needs pane-over-overlay state in
  `update/palette.rs` (12/12b's) or per-place `stands_down` lists for every modal place — a separate ticket.
- He sees: Map open, `?` → HERE lists the Map's keys (`⏎ seek` …), EVERYWHERE lists Global keys that act there; no
  `c commit`, `R`, `m merge`, `/ Find…`. J5 (`?` = 1 step) unchanged.

**Major 3 — the four one-chord idles and `AFTER y`: fix now, not gap rows.** All five are small and in files no
other open ticket holds (Blocked-by checks the rest). Rule: §2.1 :179–180 — a pressed chord is translated to its
action's default **before any pane sees it**, so every chord of a row acts where the row acts; a place answering one
chord of a row and not another is a defect (ux row 4). `ONE_CHORD_IDLE` is 09d's record of exactly these four (09d
NOTES :232–234; the sweep executed them, so idle is observed, not read).
- Blame `Home`/`End` → top/bottom as `g`/`G` (`place/keys.rs:57–58`; `BLAME_BORROWS` claims `app.top`/`app.bottom`,
  `table.rs:92–93`). Changes pane `Home` → first file as `g` (`detail/keys.rs:307`; §2.6 :344 already pairs
  `Home`/`End` with `g`/`G` in the resolver). Resolver and focused banner `⌃Z` → undo as `u` (`resolver.rs:379/383`,
  `banner/focus_keys.rs:147/173`); with `suspend_on_ctrl_z` on, `⌃Z` is `app.suspend` there and does **not** undo
  (§2.1 :191–192).
- Fix at the cause: the scout names, for each, why the second chord misses (a raw-chord arm, a handler before the
  edge's translate, …); the fix makes the handler answer the translated action, not add a second raw arm (a raw `Home`
  arm would ignore his remap — the class 09e removed for `n`). **Stop** if the cause is in `update/edge.rs` or
  `Keymap::translate` (shared by every place: a wider defect than four rows).
- `ONE_CHORD_IDLE` ends empty and is deleted with its loop; `ACTING_AT_LANDING` raised (dated, NOTES gives old → new
  and says which rise is Part C's, not Part B's).
- **`AFTER y`** (`rewrite_editor/right.rs:143–144`; 09d NOTES :235): spell the key from the row that answers it there
  (expected `confirm.yes`, via Rewrite's borrows), by 09d's table rule (:56, :61) — drawn inside the `r` text field →
  `Keymap::shipped()`; outside → live. Unbound → `AFTER` alone, no `—`. Extend `no_literal_keys`' pattern so the
  restored literal fails it (NOTES says why it was missed). If `y` there is not a key (a column word), NOTES says so
  with the line, nothing changes, and the gap row closes as not-a-defect.
- **Gap rows:** the conductor opens the three rows the review asks for now (blame `Home`/`End` **broken**; ⌃Z/changes
  `Home` **unverified**; `AFTER y` **broken**) so the usability walk finds them; 09f's merge closes them.

**Major 4 — ARCHITECTURE.md: add the entry now.** After `:84–85`, a "**Changing a key, or words that name one**"
entry: the one table (`graft-ui::keymap`, `table.rs` + `tests/keymap_ids.txt` golden ids, never renamed); resolution
in a place (`scope()` → own rows, `borrows()`, the rest; `stands_down()`; the sweep as the oracle that keeps all three
honest); translate at the edge (`graft::update::edge::key`, `context_of`); every printed key spelled from the live table
at render (`Keymap::spell_*`, `key_verb`, `after`, `HintKey`; `Keymap::shipped()` inside text fields, ADR-0055); the
cross-crate words contract (graft-ops names keys by `OpKey` through `KeyNames`, `graft-ops/src/keys.rs`; graft answers
in `update/op_keys.rs`; jobs carry `Arc<Keymap>`; no production `KeyNames` impl holds default letters); and the
`no_literal_keys` ratchet that fails a literal. Pointers, ≤15 lines; the ADR numbers (0055, 0058) cited, not restated.

**Out:** the review's Minors (except 2, folded) and Questions 2–3 — conductor files them; Minor 6 (moving the grep
ratchet to graft) is not done here even though this ticket edits that test's pattern.

## Signatures
No call-site signature changes; one new `pub` fn.
```rust
// graft-ui::keymap — signature unchanged; Map, Stash, Journal now [self, Global] (Graph dropped)
impl ContextId { pub fn scope(self) -> /* unchanged */; }
// graft-ui::keymap — signature unchanged; gains any Graph row the sweep shows acting at Map/Stash/Journal, with graft's line
impl ContextId { pub fn borrows(self) -> /* unchanged */; }
// graft-ui::keymap — NEW: in-scope rows this place answers with nothing; each list commented with graft's line.
// Element type matches `borrows()` (if `borrows()` returns rows rather than ids, mirror it).
impl ContextId { pub fn stands_down(self) -> &'static [ActionId]; }
// graft-ui::keymap::resolve — signature unchanged; excludes `ctx.stands_down()`
impl Keymap { pub fn reachable(&self, ctx: ContextId) -> Vec<(&Binding, &[KeyChord])>; }
// Part A: keymap::{spell, apply} children; every pub/pub(crate) path re-exported where it was (`keymap::join`).
// graft::update::tests::keymap_edge — ONE_CHORD_IDLE deleted; SWALLOWED_BY_DESIGN loses the three overlays' Graph
// rows (unreachable) and their Global stand-downs (now production `stands_down()`)
const ACTING_AT_LANDING: usize; // raised by Part C, dated
// graft::update::tests::palette_live — test-only
const DEAD_ELSEWHERE: &[(ContextId, ActionId, &str /* why */)];
```

## Acceptance (each tick names its test, at tick time)
- [x] **A, pure moves.** `resolve.rs`, `spell.rs`, `apply.rs`, `preview_box.rs` (+ child) each under 400 lines
  (`wc -l` in NOTES); no test renamed or edited; full gates green on the move commit alone. (gates; NOTES `wc -l`) — ticked: full gates on the A commit, 5756/5756.
- [x] **B, keymap.** `reachable(Map|Stash|Journal)` holds no Graph-context row except that place's borrows, and no
  `stands_down()` row; `scope()` of every other place unchanged (compared to a pinned list). Red first.
  (`keymap::tests::rules::an_overlay_reaches_no_graph_row_it_does_not_borrow`) — ticked, red seen first.
- [x] **B, clash.** `[keys.map] seek = "c"` → no `Collision` with `graph.commit`, and `c commit` is still spelled on
  the graph bar (D7 no longer drops it). Any clash test changed by the narrowing is listed in NOTES (old → new).
  (`keymap::tests::clash::a_map_line_on_a_graph_chord_loses_nothing_on_the_graph`; plus `keymap::tests::clash::a_map_line_on_a_key_the_map_stands_down_loses_nothing`) — ticked; the first was **green before** the change (see NOTES), the second red first.
- [x] **B, `?` at the overlays.** Map, stash list, journal open, graph focused: open `?`, for every listed row with a
  key, press it (fresh fixture where it has something to act on) → the frame changes or a `Command` is emitted
  (09e's oracle). Red first (fails today on `c`, `R`, `/`). (`update::tests::palette_live::every_key_help_lists_at_an_overlay_changes_what_he_sees`) — ticked, red seen first.
- [x] **B, ratchet elsewhere.** The same check at every other place `?` opens (pane over each overlay, Diff, start
  screen list, …) equals `DEAD_ELSEWHERE` exactly; NOTES lists its rows for the conductor's gap row.
  (`update::tests::palette_live::dead_rows_help_lists_elsewhere_match_the_known_table`) — ticked; 214 rows, NOTES.
- [x] **B, sweep.** Every `stands_down()` row is idle at its place (new assertion); acting count 444 before and after
  Part B (both in NOTES); `every_reachable_row_acts_or_is_listed_and_follows_its_remap` green.
  (`update::tests::keymap_edge::sweep::every_row_a_place_stands_down_is_idle_there`, and that test) — ticked, 444 → 444.
- [ ] **C, blame.** Blame open, cursor mid-file: `End` → as `G`, `Home` → as `g` (state compared); `[keys.app] top =
  "t"` → `t` goes top, `Home` does nothing. Red first. (`update::place::tests::home_and_end_in_blame_act_as_g_and_capital_g`)
- [ ] **C, changes.** Changes pane, cursor on the third file: `Home` → first file, as `g`. Red first.
  (`update::detail::tests::home_in_the_changes_pane_acts_as_g`)
- [ ] **C, `⌃Z`.** Resolver open after a journaled step; focused banner likewise: `⌃Z` → the same undo `u` gives
  (state and commands compared); with `suspend_on_ctrl_z` on, `⌃Z` does not undo. Red first.
  (`update::resolver::tests::ctrl_z_undoes_in_the_resolver_as_u_does`, `update::banner::tests::ctrl_z_undoes_on_the_focused_banner_as_u_does`)
- [ ] **C, sweep.** `ONE_CHORD_IDLE` deleted; `ACTING_AT_LANDING` at the new count (NOTES: old → new, which rows).
  (`…::sweep::every_reachable_row_acts_or_is_listed_and_follows_its_remap`)
- [ ] **C, `AFTER y`.** `confirm.yes = "c"` → `AFTER c` outside the field; inside the `r` field the shipped `y`;
  unbound → `AFTER`, no `—`. Red first. (`rewrite_editor::tests::the_after_heading_spells_his_confirm_key`); the
  literal restored fails the grep test (`no_literal_keys::no_user_facing_literal_spells_a_default_key`, red observed,
  NOTES).
- [ ] **Captures.** After 17 merges: full `cargo nextest run` with `GRAFT_UPDATE_SHOTS` set once, then unset and
  green. Allowed diffs: a `?` capture at Map/Stash/Journal losing Graph rows and `/`/`n`/`N`; nothing else (any other
  diff is a stop). Add `docs/shots/m8/help-at-map-120x36.txt` (Map open, `?`) — HERE and EVERYWHERE are the visual
  result, judged by the user (rule 6). Colour unverified (rule 5).
- [ ] **D.** ARCHITECTURE.md entry as ruled; MAP.md — graft-ui `## keymap` (files line: `resolve`/`spell`/`apply`;
  `scope()` contract now "over-includes nowhere a place swallows: the three overlays are `[self, Global]`";
  `stands_down()`; `preview_box` child); graft — sweep tables (`ONE_CHORD_IDLE` gone, `stands_down` assertion),
  `palette_live`, the four handlers. (doc, checked by reading)

## Touches
- `scope()` / `reachable` / `translate` / `acting` callers: `update::palette` (`place_rows`, `key_in` — lists change at
  three places, code unchanged), the sweep, `with_overrides` clash detection, `live_labels` (D7 relabel), hint bars
  (`spell_*` read `live_labels` — a bar may now spell a chord D7 used to drop; captures prove none changed). Grep
  `scope(` and `reachable(` in graft and graft-ui; NOTES lists each site and how checked.
- Translate at the three overlays for Graph chords: was `Default`, now `Pass`/`Unbound` — every such row is already
  swallowed (sweep), so behaviour holds; the unchanged acting count (Part B) is the proof.
- 09e's search stand-down (`search.rs:20–28`): unchanged; `stands_down()` cites it.
- Keymap: no binding added, removed or moved; `keymap_ids.txt` unchanged (show the empty diff).
- Journal/undo: `⌃Z` reaches the existing undo path — no new journal write; show empty `git diff --stat` over
  `crates/graft/src/update/journal*`, `crates/graft/src/update/undo*` and `crates/graft-ops`.
- 11c: its report-only `?` HERE check (Map, journal, stash) now reads the fixed lists — run after 09f. 15 (bar ranking)
  and 11c hold `hints*` — untouched here. 12/12b: palette rows at overlays shrink; their palette captures re-shot after.
- Captures: `?` captures at overlays; the m8 contact sheet re-assembled (conductor).
- KNOWN_GAPS (conductor): open `M8/keymap-file-sizes` (Major 1 rest) and, from NOTES, `M8/help-lists-dead-keys`
  (`DEAD_ELSEWHERE`); open-then-close the three Major 3 rows with this merge.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~62k: A pure moves ~8k; B scope/borrows/`stands_down` + keymap tests ~12k, sweep tables ~6k, `palette_live` (two tests,
oracle reuse) ~10k; C four handlers + tests ~12k, `AFTER` + grep ~4k; captures ~4k; ARCHITECTURE + MAP ~6k. Commit
order A → B → C → D. Past ~70k: stop after B, hand C and D to a split (NOTES says where).

## NOTES (builder, 2026-10-07)
**Compacted once mid-ticket** (after B's gates). Scout used: `scouts/09f-keymap-review-fixes.md`.

Done: A (commit "A - split …"), B (commit "B - …"). Not done: C, captures, D — handed off below.

- `wc -l`: keymap.rs 274, resolve.rs 307, spell.rs 144, apply.rs 77, claims.rs 144 (new, B: the borrows/stands-down
  lists, moved out because keymap.rs reached 407), preview_box.rs 329, preview_box/footer.rs 129.
- Deviation A: moving only `Again` left preview_box.rs at 403 (its tests were already a child). The whole footer
  (part, join, footer_label, too_small, Again) went to `preview_box/footer.rs`. One import added to
  `preview_box/tests.rs` (`use crate::keymap::ActionId;`), no test edited.
- B design: `ContextId::stands_down()` is much bigger than the planner expected — every Global row each overlay
  swallows (map.rs:176; stash.rs:224,226; journal/keys.rs:111,113), not only search. `reachable` and clash `lives_in`
  leave them out; translate/resolve unchanged.
- Graph rows that act at the overlays (stop 1 at ≤5): Map borrows `graph.stash`, `graph.journal`; Stash borrows
  `graph.journal` (claim chain `update/keys.rs:309-313`); Journal none.
- Clash test `seek = "c"` was **green before** B: `lives_in` was already false for Graph rows at Map, so no collision
  existed. The red-first clash test is the new `seek = "q"` one (Map stands down `app.quit`). No existing clash test changed.
- Sweep: before B 837 cases, 444 act, 351 listed, 42 shadowed; after B 727 cases, 444 act, 249 listed, 34 shadowed.
  Removed 96 SWALLOWED rows (Map/Stash/Journal) and 8 SHADOWED rows. Measured with a temporary assert, since removed.
- `DEAD_ELSEWHERE` (gap row `M8/help-lists-dead-keys`), 214 rows in `update/tests/palette_live/dead.rs`, by reason:
  swallowed 87, no effect in the fixture 69, pane over an overlay 52, one-chord idle 1, Finish listing the banner's
  `operation.*` 5. `?` lists swallowed keys at Banner, Finish, Blame, History and others — the same class as Major 2.
- Contradiction: C says "`ONE_CHORD_IDLE` deleted". It has 27 rows; most are text fields (Picker, CommitEditor,
  Credential) and the Menu `s` row, which C does not fix. Only the five C rows can go (Changes Home, Resolver ⌃Z,
  Banner ⌃Z, Blame Home, Blame End). Ruling needed: shrink the table rather than delete it.
- Question (no line yet): with the stash list shown and the pane focused after `⇥`, the frame is identical — the
  focus shows only by colour. Is a text cue wanted?
- Touches: `reachable` callers grepped — `update/palette.rs:281` (place_rows), sweep; `scope()` — `resolve.rs:226,299`;
  `history`'s `reachable` is unrelated. Empty diffs: `tests/keymap_ids.txt`, `update/palette.rs`, `update/edge.rs`,
  graft-ops, update/journal. Captures: full nextest green with no shot change (no `GRAFT_UPDATE_SHOTS` run yet).

### Handoff (C, captures, D)
1. C, blame Home/End: `crates/graft/src/update/place/keys.rs`. Changes Home: `update/detail/keys.rs`. Banner ⌃Z:
   `update/focus_keys.rs`. `AFTER y`: `rewrite_editor/right.rs:143-144`.
2. **C resolver ⌃Z needs `crates/graft/src/update/resolver.rs` — held by 10g.** Do it after 10g merges.
3. Then remove the five rows from `ONE_CHORD_IDLE` (see the ruling above) and re-measure `ACTING_AT_LANDING`; the
   matching `DEAD_ELSEWHERE` row (one-chord idle) goes too.
4. Captures: add `docs/shots/m8/help-at-map-120x36.txt` via `crates/graft/tests/it/look_polish.rs`
   (`crate::captures::check_shot`); full run with `GRAFT_UPDATE_SHOTS` once.
5. D: ARCHITECTURE.md entry; graft MAP.md for the C handlers. graft-ui MAP.md is already updated for A and B.
