# G-keymap 1d — sweeps that prove something; the last literal keys spell from the table

Status: done (builder, 2026-10-07; see NOTES). Was: draft (planner spec-only pass, 2026-10-04; split from `09c-keymap-review-fixes.md` — its D4, Majors 3–4 and
Minors 2, 8, 12 — plus 10c's five allow-listed literals; amended 2026-10-07 with the rulings on 09c's deviations and
questions, KNOWN_GAPS `M8/09c-deviations`)
Build with: **Opus** — the oracle and every literal's row are decided here; held by named tests, not by eye. Not Fable:
no structural choice is left open.
Blocked by: none (09c merged)
Needs: from 09c — `ContextId::borrows`, `Keymap::reachable`, `KeySwitches`, `Clash`, the golden list without
`finish.commit`/`rewrite.start`/`empty_state.confirm`/`preview.confirm`; from 09b — `outside_edge`, `edge::unanswered`;
from 10/10c — `HintKey::{Key, Fixed}`, `Keymap::{key_verb, after, spell_keys}`, the `StashScreen` widget pattern
(`pub keys: &'a Keymap`), `keymap::sweep` helpers, `graft-ui/tests/no_literal_keys.rs`.
Crate set: graft-ui (`detail_pane/header.rs`, `detail_pane/line_render.rs`, `graph_screen/banner.rs`,
`graph_screen/badges.rs`, `rewrite_editor/right.rs`, their widget structs, `tests/no_literal_keys.rs`; **D6:** the one
`ContextId::borrows()` entry for `EmptyState`; **D7:** the file holding `Keymap::live_labels` and its cache, plus keymap
tests) + graft (`update/tests/keymap_edge*` rewrite and split, `empty_state` tests, `view/status` test, the view call
sites of those four widgets; **D6:** the `empty_state` module's `outside_edge`; `update/tests/keymap_rules.rs` only
where a ruling below changes an asserted outcome). No other `keymap/*` rows or resolution.
**Files, for overlap with builder 16 (theme):** graft-ui `detail_pane/{header,line_render}.rs` (+ `detail_pane` mod),
`graph_screen/{banner,badges}.rs` (+ `graph_screen` mod), `rewrite_editor/right.rs` (+ `rewrite_editor` mod),
`keymap/*` (borrows entry, `live_labels`, tests), `tests/no_literal_keys.rs`, `MAP.md`; graft `empty_state*` (module +
tests), `update/tests/keymap_edge*`, `update/tests/keymap_rules.rs`, `view/status` tests, the `view/*` call sites of the
four widgets, `MAP.md`. **None in `theme*` or `config*`.** Likely contact: if 16 changes how a widget takes its theme,
the same widget structs and view call sites — rebase on 16, the compiler names each.

## Read first
- `docs/reviews/m8-09-keymap.md` Majors 3–4 (inside :58–148), Minors 2, 8, 12 (:158, :200)
- `09c-keymap-review-fixes.md` D1, D3 and **Rulings on 10c** (R4's rule); its NOTES "Deviations" 1–5 and "Other
  observations". `10c-keymap-spell-footers.md` NOTES "Beyond the listed sites"
- This ticket's **Rulings on 09c** (whole)
- `docs/design/02-keymap.md` §2.3b (:288–289), §2.3c (:296), §2.13 (:542–555); `x` row (:250); `L` row (:314);
  §2.6a (:347–360); rewrite `y`/`Tab` rows (:429–431)
- `docs/research/ux-principles.md` rows 4, 6 (:54, :56)
- `crates/graft-ui/MAP.md` `## keymap` (`live_labels`, `borrows`, `Clash`), `detail_pane`, `graph_screen::badges`,
  `GraphView::banner`, `BannerView`, `TabWord`, `rewrite_editor`, `preview_box`; `crates/graft/MAP.md` `empty_state`
  (:6), `update::edge`, `context_of`, `outside_edge`, `update::tests`, `AppState::text_field_open`, `AppState::rewrite`
- Source after the maps: `update/tests/keymap_edge.rs` (whole — you rewrite it), each literal's line (10c NOTES),
  `outside_edge` and `outside_key`'s init-box arm, `live_labels`

## Decisions (planner; not the builder's)

**D4 — The behavioural sweep's oracle is behaviour, not a copy of `resolve`** (Major 3; moved verbatim from 09c).
Delete `acting_default` (`keymap_edge.rs:160–171`). Expected = what the row's default chord does under the default
keymap. Which rows a place shadows comes from a hand-written `SHADOWED: &[(ContextId, &str /*id*/, &str /*chord*/,
&str /*by id*/, &str /*why: § or file:line*/)]`, never computed. graft-ui's `default_holder`
(`keymap/tests/translate.rs:11–17`) may stay for the pure sweep; 09c's pinned D1 cases are what catch an order bug
there.

**D5 — The five literals 10c allow-listed as KNOWN spell from the row that answers them** (10's decision 8: spelling
at render, widget takes `&Keymap`). An unbound key's part is left out (10's decision 2); never `—`.
| Literal (file) | Spells from | Why |
|---|---|---|
| `x clears it` (`detail_pane/header.rs`) | `Key(graph.compare)`; unbound → clause dropped | §2.3 `x` (:250) marks/clears the compare base |
| `L load all` (`detail_pane/line_render.rs`) | `Key(diff.load_all)`; unbound → clause dropped | §2.5 `L` (:314) |
| `space stages, a amends it, c continues · ⇥ banner` (`graph_screen/banner.rs`) | `space` → the Graph row `space` translates to at an `edit` stop; `a` → `graph.amend`; `c` → `graph.commit`; `⇥` → `app.next_pane`; each clause dropped if unbound | an `edit` stop gives the keyboard to the graph (§2.6a :356–358); `c` there is the banner's `c` (§2.3b :303) |
| `⏎ back to AFTER` (`rewrite_editor/right.rs`) | drawn only while the `r` field is open (`text_field_open`, graft MAP `AppState::rewrite`) → `Fixed(<the row the field answers>)`; drawn outside it → `Key(<row>)` | R4's rule (09c): inside a text field only shipped keys are spelled |
| `x base` (`graph_screen/badges.rs`) | `Key(graph.compare)`; unbound → `base` | the badge names how to clear the base |
- The rows above are the planner's reading of the design, **not checked against code** (rule 7): the builder pins
  each with `Keymap::shipped().translate(<live ctx>, <default chord>) == <id>`. A mismatch is a stop: NOTES names the
  handler's `file:line` and the row it found; report it, don't pick.
- `⏎ back to AFTER` with no row the field answers: a text field's key never moves, so the literal is what the shipped
  spell prints. It stays on the allow-list with reason `text field, no row: <file:line>` — not a `KNOWN` entry.
- No new table rows here (09c's table). A literal that needs one: stop, NOTES, report.

## Rulings on 09c (planner, 2026-10-07)

**Deviation 1 — init box keeps `c` as clone under `confirm.yes = "c"`: fix (D6).** The init box is a confirm box
(§2.13 :549 "behind a confirm (`y`)"); a confirm box swallows every key but its own (§2.3c :296), so clone is not live
while it is open. Today's outcome — `c` swallowed as clone's default, `y` unanswered — leaves the box with no confirm
key at all; ux-principles row 4 (one confirm key in every box, §2.3b :289 "one confirm key").
- **D6 — while the init box is open, `outside_edge` translates in `ContextId::Confirm`, not `EmptyState`.** Options:
  (a) a new `ContextId` for the box — a new `[keys.<ctx>]` section frozen at release (ADR-0055 §2) for one box;
  (b) reuse `Confirm`, whose own row is `confirm.yes`. **Take (b).** `EmptyState`'s `borrows()` drops `confirm.yes`
  (the box was its only confirm; its comment goes with it). No `ContextId` added, no row added.
- **Stop condition (rule 7, not checked against code):** under the shipped keymap, every key `outside_key`'s
  init-box arm answers today (`y`, `n`, `esc`, `←`/`→`, `⇥`, `⏎`, … — read the arm, list them in NOTES) must give the
  same `OutsideStep` and box state after D6 as before. Any that differs: stop, NOTES names the arm's `file:line` and
  the row `Confirm` resolved it to; report, don't pick.
- He sees, `[keys.confirm] yes = "c"`: zero problems (the `on the start screen c stays clone` warning is gone); start
  screen `c` opens Clone…; init box open: `c` inits, `y` swallowed (box stays), bar reads `c init` (spelled from
  `confirm.yes`, as today).

**Deviation 2 — merge preview opened from `.` draws `y confirm`, not `y merge`: accept, no fix.** The ruling's
"`y merge` stays" was the planner's error: the box's footer has always drawn `y confirm` beside the `again` slot
(`y confirm  m merge …`, graft-ui MAP `preview_box`; graft MAP `take_key` "the bar stays `y confirm`"), and §2.3b
(:288) gives a preview opened any other way "only `y`". `y confirm` in every preview is ux-principles row 4. 09c's
extended test already pins it. Conductor closes item (2) of the KNOWN_GAPS row.

**Deviation 3 — R4 (b) checked on the drawn frame, not `HintKey` values: accept.** The drawn bar and footers are what
he sees (rule 1); a `Key` that spells a moved row shows as `alt+…` under the sweep keymap. The oracle holds only if
that keymap moves every row a bar or footer can spell: add one guard assertion to
`keymap_rules::a_text_field_draws_no_key_his_remap_moves` that no row reachable in the 11 fields' contexts keeps a
shipped chord under the sweep keymap (NOTES lists any it must exempt, with why).

**Question 1 — J's legend and blame's find header spell his remapped keys while find is open: accept, no fix.** R4's
rule covers the **bar and footers** — the keys for what is typed now. Pane text that describes the place (J's
`… can undo it`, blame's `… shows them`) names keys that act the moment the field closes; spelling them from the
shipped keymap while the field is open would flip the legend's words each time find opens (row 4) and name keys that
are not his (row 6). Builder writes this scope into graft MAP `text_field_open`.

**Question 2 — `live_labels` ignores context: fix (D7).** `live_labels(A)` keeps every chord A's own context resolves
to A (graft-ui MAP `## keymap`), so after `[keys.graph] merge = "j"` a hint for `app.down` still spells `j` — on the
graph, where `j` merges. A key spelled where it does something else is a lie (10's decision 2: an omission, never a
wrong key; row 6 needs the shown key to be the one that acts).
- **D7 — a chord is not live for A if any place took it from A:** `live_labels(A)` drops every `k` for which the
  keymap's warnings hold a `Clash { key: k, lost: A, .. }` (09c's take-clauses, computed where the losing row lives —
  NOTES deviation 5). Options: (a) context-aware spelling (`spell_in(ctx, …)`, every hint caller passes its place);
  (b) D7. **Take (b) now:** one function, no caller changes, shipped keymap has no clashes so no default label or
  capture moves. **Trade-off, stated:** after `merge = "j"` the changes pane, diff and menus spell move `↓` alone
  though `j` still moves there — an omission, not a lie. (a) is the precise fix; it waits for ticket 11's `?`, which
  needs per-place spelling anyway (conductor: note on 11).
- He sees, `merge = "j"`: every bar and `?`-less hint spells move `↓`; graph bar `j merge` where it shows merge;
  `config: 1 problem`, F2 words unchanged (09c pinned).

## Scope
In: D4–D7, Majors 3–4, Minors 2, 8, 12, the R4 guard (deviation 3), MAP scope line (question 1). Out: resolution order,
rows, warning words (09c); `?`/palette and context-aware spelling (11); ranking (15).

## Signatures
```rust
// graft-ui — the 10c pattern: `pub keys: &'a Keymap` on the widget, or a trailing `keys: &Keymap` on a free draw
// fn, for the detail pane (header, line_render), graph_screen::badges, the graph banner, rewrite_editor::right.
// MAP.md records each name.
// graft-ui::keymap — no signature change:
//   ContextId::borrows(EmptyState) loses ActionId("confirm.yes")                       // D6
//   Keymap::live_labels(&self, &ActionId) -> &[String]  — drops every chord a Clash names as lost by that id  // D7
// graft::empty_state — no signature change: outside_edge(state, &Keymap, key) translates in ContextId::Confirm
//   while the init box (Prompt::ConfirmInit) is open, else EmptyState                  // D6
// graft::update::tests — sweep tables
const SHADOWED: &[(ContextId, &str, &str, &str, &str)];
const SWALLOWED_BY_DESIGN: &[(ContextId, &str /*id*/, &str /*why*/)];
const NO_EFFECT_IN_FIXTURE: &[(ContextId, &str /*id*/, &str /*why*/)];
```
No new `pub` items.

## Acceptance (each tick names its test, at tick time)
- [x] **Major 3.** Sweep over `reachable(ctx)` for every context with a fixture (all rows, not own + Global). Overlays
  given rows to move over; pane cursors mid-list. Every case either **acts** (default press ≠ F20) or is listed with
  a reason (`SWALLOWED_BY_DESIGN` — Preview/Confirm/Banner swallows, §2.3c; `NO_EFFECT_IN_FIXTURE`; `SHADOWED`). A
  listed entry that acts, or a shadow that doesn't hold, fails the test. Test prints acting/listed counts; asserts
  acting ≥ the count at landing (written in the test with the date). — `update::tests::keymap_edge::sweep::every_reachable_row_acts_or_is_listed_and_follows_its_remap` (837 cases: 396 act, 399 listed, 42 shadowed; `ACTING_AT_LANDING = 396`, 2026-10-07). Counts ride on the assert messages, not a print (NOTES).
- [x] **Major 4.** EmptyState swept like Major 3 through `outside_edge`, three fixtures: plain, init box open (closes
  Minor 12: `y`/`n`/`esc` under a remap — swept in `Confirm` after D6), prompt open (every key typed). F2
  sanitising pinned: the reviewer's probe `"\u001b[31mmerge"` → F2 draws `graph.␛[31mmerge is not an action`
  (Minor 2). — `empty_state::sweep_tests::every_row_the_start_screen_reaches_acts_or_is_listed_and_follows_its_remap` (97 cases: 13 act, 52 listed); `update::tests::keymap_edge::remaps::f2_draws_an_escape_in_his_action_name_as_a_symbol`.
- [x] **D6.** graft: shipped keymap, init box open — each key the arm answers (NOTES list) gives the same step before
  and after (`empty_state::tests::the_init_box_answers_the_same_keys_in_confirm`). `confirm.yes = "c"` through
  `load_from` + `outside_edge`: zero problems; init box `c` → `OutsideStep::Init`, `y` → box stays; start screen `c`
  → Clone prompt. Re-point 09c's `empty_state::tests::the_init_box_confirms_on_his_confirm_key` from `Y` to `c`, name
  kept. graft-ui: `ContextId::EmptyState.borrows()` holds no `confirm.yes`; `with_overrides(confirm.yes = "c")`
  yields no warning (`keymap::tests::rules::confirm_on_c_is_no_problem_on_the_start_screen`). Any 09c test asserting
  the removed warning is edited in place; NOTES names each. — `empty_state::tests::the_init_box_answers_the_same_keys_in_confirm`, `…::the_init_box_confirms_on_his_confirm_key` (start screen `c` → `Stay`: no Clone prompt exists yet, NOTES), `keymap::tests::rules::confirm_on_c_is_no_problem_on_the_start_screen`.
- [x] **D7.** graft-ui: `merge = "j"` → `live_labels(app.down)` holds `↓`, not `j`; shipped keymap → every
  `live_labels` equal to before (all ids, compared against `Keymap::shipped()` built the old way in the test)
  (`keymap::tests::spell::a_key_taken_anywhere_is_not_spelled_for_its_loser`,
  `…::the_shipped_labels_are_unchanged`). graft, through `update` + view, `merge = "j"`: the graph bar and the
  changes pane's bar draw no `j` beside move (`update::tests::keymap_rules::merge_on_j_spells_move_without_j`; NOTES
  says which bars draw move at all). 10c's sweep (c) green; if the sweep keymap yields clashes and a label moves:
  stop, NOTES, report. — `keymap::tests::spelling::a_key_taken_anywhere_is_not_spelled_for_its_loser`, `…::the_shipped_labels_are_unchanged` (module is `spelling`, not `spell`); `update::tests::keymap_rules::merge_on_j_spells_move_without_j` (pins the Map footer and finish files — the graph and changes bars draw no move, NOTES). 10c sweep (c) green in the full run.
- [x] **Deviation 3 guard** in `keymap_rules::a_text_field_draws_no_key_his_remap_moves` (test named, green). — exemptions `UNMOVED` in that file (NOTES); first run failed on `app.quit_now` (the guard can fail).
- [x] `keymap_edge.rs` split under 400 lines per file (Minor 8). 09c's `keymap_rules.rs` edited only per D6, D7 and
  the guard. — `keymap_edge.rs` 374; `keymap_edge/{fields 50, remaps 230, sweep 212, sweep_swallowed 248, sweep_tables 188, unbound 192}`; `keymap_rules.rs` also had `j_still_moves…`'s comment and loop widened to Map/Journal/Stash now they have rows (NOTES).
- [x] **D5 literals.** Test `each_former_literal_names_the_row_that_answers_it` (shipped translate per row of the
  table). Under 10's sweep keymap each of the five lines holds the remapped label and no default label of a moved
  action as a whole token (10c sweep (c) helpers); the `r` field's line holds the shipped label whatever the sweep
  says. `graph.compare = "none"` → no `x` in the header or the badge, no `—`. — `keymap::tests::literals::each_former_literal_names_the_row_that_answers_it`, `detail_pane::tests::the_compare_line_and_the_load_all_footer_spell_his_keys`, `graph_screen::banner::tests::the_edit_hint_spells_his_keys`, `rewrite_editor::tests::the_changes_header_spells_his_key_and_the_shipped_one_in_the_field`, `graph_screen::tests::the_base_badge_spells_his_compare_key`.
- [x] **Grep test.** `no_literal_keys.rs::no_user_facing_literal_spells_a_default_key` green with **no `KNOWN`
  entry left**; any remaining allow-list entry names its `file:line` and reason. — `no_literal_keys.rs::no_user_facing_literal_spells_a_default_key`, `KNOWN` empty (red first: "allow-list entries no longer in the source").
- [x] Captures: default config → no capture changes (the spelled labels equal the literals; D6/D7 move nothing under
  the shipped keymap). If a token changes (e.g. `space`'s label), it is a decision-3 change: re-shoot
  (`GRAFT_UPDATE_SHOTS=1`), NOTES lists file and token, re-assemble the affected contact sheets. Colour unverified
  (rule 5). — full `cargo nextest run`: every `graft_fixtures::capture` compared against the committed file with `GRAFT_UPDATE_SHOTS` unset, all equal; `git status` shows no `docs/shots/` change. Nothing re-shot. Colour unverified.
- [x] MAP.md graft-ui: each widget now taking `&Keymap`; `borrows` (EmptyState), `live_labels`' new rule. graft:
  `update::tests` sweep tables; `empty_state` (`outside_edge` in `Confirm` while the init box is open);
  `text_field_open` — R4 covers bar and footers, pane text spells live keys (question 1). — both MAP.md edited (docs, no test).

## Touches
- The four widgets' graft view callers (compiler); every capture drawing a compare base badge, the detail header's
  compare line, a truncated diff, an `edit` stop banner, or the rewrite editor's message block (default keymap: no
  change expected — the capture run shows it).
- Badge width: `x base` is measured in the graph row's layout — no column shift under the default keymap (capture
  run); a longer remapped label is cut like any badge (sweep test).
- 09b's sweeps and J8 tests (rewritten here; J8's words pinned by 09c stay green). `empty_state` tests.
- **D6:** `empty_state::bar_keys` (init box spells `confirm.yes` — unchanged, check); ticket 18's knock-on line
  ("`c` meets `confirm.yes` … warned") no longer holds — conductor amends 18; ticket 11's `?` over the init box lists
  `Confirm`'s rows like any confirm box; captures of the init box (default keymap: none expected).
- **D7:** every caller of `spell_keys`/`spell_every`/`key_verb`/`after` (all read `live_labels`; grep, NOTES count) —
  no change under the shipped keymap; 09c's warning tests (words unchanged); ticket 11 (`?` spelling; per-place
  spelling is its call — conductor notes it on 11).
- Journal/undo: none (render, key routing at the start screen, tests) — show the empty diff over
  `crates/graft/src/update/journal*` and `crates/graft-ops`. KNOWN_GAPS: 09b tick 1's EmptyState row closes with the
  Major 4 box; `M8/09c-deviations` closes with D6, D7 and the guard (conductor).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~72k: Major 3 sweep rewrite ~25k; EmptyState + F2 ~10k; split ~5k; D5 literals + tests ~12k; D6 ~6k; D7 ~6k; guard
+ captures + MAP ~8k. One context, near the line.
Order: literals, then D6 and D7 (small, independent), then sweeps. Seam if over: commit after Major 3, report
`blocked: Major 4`.

## NOTES (builder, 2026-10-07)

- **Compacted once, mid-ticket** (after D5–D7 and the split). Everything in the ticket is done; nothing remains.
- **Scouts:** used `09c-borrows.md` (sufficed for the swallow lines); plus one new scout classifying the 396 first-run
  failures with `file:line` (its table is `sweep_swallowed.rs`/`sweep_tables.rs`; I spot-checked 5 lines — map.rs:176,
  preview/keys.rs:69-70, keys.rs:405, search.rs:20, resolver.rs:383 — all held). The sweep itself re-checks every entry.
- **Red before green:** D5 grep test, D6 rules + empty_state, D7 spelling, the guard: red first. `merge_on_j…`, the F2
  pin and the two sweeps' first runs were red only on missing table entries / wrong probes (the code under test already
  behaved); the F2 pin went green first — a pin of existing sanitising.
- **Deviations:** (1) counts are in the sweep's assert messages, not printed — `update_cannot_reach_the_world` scans
  every file under `update/`, tests included, for print macros. (2) Two more per-chord tables beside the three named:
  `ONE_CHORD_IDLE` (a row with one chord acting and one idle, e.g. resolver `u` vs `⌃Z`) and `DEFECTS`. (3) No Clone
  prompt exists (ticket 18): start screen `c` is `Stay`, listed `NOT_YET_ANSWERED`. (4) `merge_on_j…` pins the Map footer
  and finish files: the graph bar and the changes bar draw no move hint at all; bars that draw `DOWN_UP`
  (`hints/lists.rs`): Map, finish files, resolver lists, history place, graph overflow tail. (5) `#[rustfmt::skip]` on
  the five table consts (one row per line).
- **D6 edits in place:** graft-ui `rules::one_confirm_line_moves_every_confirm` (warning now none); graft
  `empty_state::tests::the_init_box_confirms_on_his_confirm_key` (`Y` → `c`).
- **D6 knock-on (question):** `empty_state.cancel` (`n`) is now reachable only on the list, where `n` does nothing —
  the init box declines on raw `n` (empty_state.rs:112) in `Confirm`. A remap of `empty_state.cancel` moves nothing.
  Listed SWALLOWED; the row may want deleting or folding into `confirm.no`. Conductor/planner.
- **Init box** answers only `y`/`n`/`esc` (empty_state.rs:110-116), not `confirm.choose/left/right/cycle`; it draws
  `y init · esc keep it`, so listed as swallowed, not a defect.
- **DEFECTS found by the sweep (not fixed, out of scope), each with its line:**
  - Map and stash list, `/`: search.rs:20 opens the graph's search prompt behind the overlay — unseen, and it takes the
    next keys (`modal_owns_keys` omits both, state.rs:907-915).
  - Changes and Diff panes, `R`/`C`/`O` (and Diff `M`): gated on `graph_has_focus` (keys.rs:405, picker.rs:125,
    strip.rs:17, map.rs:144) though the keymap scope makes Graph rows reachable there (graft-ui keymap.rs:149-150) and
    `c a m r b t f p` do act from the panes.
- **Questions (one chord idle, `ONE_CHORD_IDLE`):** resolver and focused banner answer `u` but swallow `⌃Z`
  (resolver.rs:379/383, banner/focus_keys.rs:147/173); blame answers `g`/`G` but swallows `home`/`end` while
  `BLAME_BORROWS` claims all four (place/keys.rs:57-58,78); changes pane has no `home` arm (detail/keys.rs:307).
- **Follow-up gap:** `rewrite_editor/right.rs` still draws an `AFTER y` literal the grep test does not catch.
- **Not verified:** colour (rule 5); word-diff/heatmap toggles that change colour only are listed NO_EFFECT on the
  scout's reading — the text frame cannot show them.
- **Touches:** four widgets' graft callers (compiler); captures (full run, none changed); `spell_keys/spell_every/
  key_verb/after` — 61 non-test call sites, all read `live_labels`, shipped labels pinned unchanged by
  `the_shipped_labels_are_unchanged`; `empty_state::bar_keys` unchanged (its test green); `git diff --stat c8b005d9 --
  crates/graft/src/update/journal crates/graft/src/update/journal.rs crates/graft-ops` empty; clippy fixes in 16's
  `rewrite_editor/tests.rs` (`;`) and `pane_boundaries.rs` (`header_prefix` helper, too_many_lines) caused by D5's
  signature changes. Ticket 18's knock-on line and ticket 11's `?`-over-init-box: conductor.
