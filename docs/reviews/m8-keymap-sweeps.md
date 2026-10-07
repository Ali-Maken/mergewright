# Review — m8-polish-and-ship keymap group, round 1: 10 + 10c, 09c, 09d, 10b, 10e, 09e

Reviewer, one round, read-only at main `c18e7d72`. Each ticket reviewed as `git diff <M>^1 <M>`: `dcf7f031`
(10+10c), `236491db` (09c), `dea5e164` (09d), `5c23f776` (10b), `f77256bb` (10e), `22c0ff52` (09e). Not repeated:
`docs/reviews/m8-09-keymap.md` (its Majors 1–5 are this group's input and are closed by 09c/09d; checked below).

**Counts: 0 Critical · 4 Major · 9 Minor · 3 Questions.** Rules 5/6: no colour and no "looks right" judged here.

## What I ran and what it showed

| Check | Result |
|---|---|
| `cargo fmt`, `clippy` (plain and `--features heavy-tests`), `nextest`, `test --doc` | **not run** — conductor's instruction (two builders hold the CPU, user cap 2). Green gates are the builders' claims (09c 5558/5558, 10b 5597, 10e 5697/5697), not observed here |
| Heavy tier, release perf run, fuzz, benches | **not run** — same instruction. No fast-suite time, slowest tests, peak memory or `target/` size to report |
| Method | `git diff`, `grep`, reading. Every behavioural finding below carries the lines I read; nothing was executed |

## Pipeline

- Design note has Need (`docs/design-notes/m8-polish-and-ship.md:20`), Prior art (`:34`, keymap rows `:56–64`, remap
  collisions/unbind = take), Journeys (`:99`). Study exists: `docs/research/keymap-help-remap.md`.
- Journeys this group touches: J8 rebind (5+1) — a person's step, not walked (`:214`), unchanged. J21 `init-here` = 2
  (`i y`) — walked in `crates/graft/tests/it/outside_repo.rs:195` (`init_here_is_i_then_y`, against the fence);
  09d D6 / 09e left `y` as the confirm, so the count holds by reading. The "Init declined (`esc`/`n`)" Lands row
  (`:161`) is stale after 09e; already tracked (`KNOWN_GAPS.md` M8/j21-note-stale).
- 10b/10e add no screen or interaction (words spelled from the table, ADR-0055), so no new study is owed.

## Tick spot-check: 26 checked, 26 held, 0 false

All 87 test names cited in the seven tickets' ticks exist (grep, one per name). I read the assertions of 26:

- **10 (1):** `keymap_edge::unbound::merge_unbound_is_drawn_nowhere_and_its_menu_row_names_no_key` — held (bar,
  menu suffix, `—` count per fixture equal to shipped).
- **10c (4):** R1 `resize_is_drawn_only_when_nothing_else_was_dropped` — held, but weaker than its tick (Minor 4);
  R3 `the_hints_resize_once_pushed_off_are_drawn_again`; R2 `a_shared_modifier_word_is_written_once`; grep test
  `no_user_facing_literal_spells_a_default_key` (hit + stale checks) — held.
- **09c (9):** `j_still_moves_in_every_place_after_merge_on_j`, `undo_on_o_is_one_problem`, the four suspend pairs,
  `confirm_on_c_confirms_every_box_and_y_still_copies_on_the_graph`, `merge_on_capital_m_confirms_on_capital_m_again`,
  `a_line_on_the_repeat_key_is_one_problem` — held.
- **09d (3):** sweep floor (`sweep.rs:187–189`, `acting >= ACTING_AT_LANDING`), `f2_draws_an_escape_in_his_action_name_as_a_symbol`
  (asserts `␛` drawn and no raw ESC), grep test with no `KNOWN` row left in graft-ui's own
  `ALLOWED` — held.
- **10b (1):** stale check covers all six tables (`no_literal_keys.rs:124–131`) — held.
- **10e (3):** `every_op_key_is_a_row_and_spells_todays_letter` (W2 via `acting`), `each_ops_word_spells_the_remapped_key`
  (29 sites; old literals in the `operation.rs` diff match the test's templates for the 6 I compared),
  `an_unbound_key_drops_its_clause` — held.
- **09e (5):** `n_under_an_overlay_leaves_the_graph_where_it_was`, `r_opens_the_rewrite_editor_from_the_panes`,
  `o_toggles_the_strip_from_the_panes`, `empty_state_cancel_is_no_action`,
  `every_key_on_a_bar_over_an_overlay_changes_what_he_sees` (frame or command oracle) — held.

Earlier review (`m8-09-keymap.md`): Majors 1, 2, 5 closed by 09c (tests above); Major 3 by 09d's sweep over
`reachable` with hand-written `SHADOWED`; Major 4 by 09d (EmptyState sweep + F2 pin). Minors 2 and 12 closed by 09d; Minor 8 half (`keymap_edge.rs` split, `table.rs` still 509).

## Critical

None.

## Major

### 1. Files over the 400-line limit grew or crossed it in this group, with no split and no gap row (PROMPT 6.2)

- Crossed 400 here: `crates/graft-ui/src/keymap/resolve.rs` 307 → 514 (10: 460, 09c: 488, 09d: 504/514);
  `crates/graft-ui/src/preview_box.rs` 378 → 445; `crates/graft-ops/src/restore.rs` 390 → 404 (10e).
- Grew while already over: `graph_screen/hints.rs` 639 → 825 (10, 09e), `stash_screen.rs` 649 → 747,
  `context_menu.rs` 824 → 870, `hints/lists.rs` 522 → 562, `graft-ops/src/operation.rs` 1852 → 1912,
  `graft/src/update/undo.rs` 1903 → 1953, `jobs/runner.rs` 1366 → 1400 (10e; plus a new `#[expect(too_many_lines)]`
  on `dispatch`, 10e NOTES).
- `KNOWN_GAPS.md` M8/look-file-sizes (:1128) and M8/stop-files-over-400 name other files, none of these.
- **Why it matters:** `resolve.rs` is the one file a reader needs to understand the remap rules (resolution order,
  followers, switches, spelling, `join`); at 514 lines it mixes three jobs.
- **Fix:** split `resolve.rs` into resolution (`resolve`, `holder`, `best`, `rank`, `reachable_rows`), spelling
  (`relabel`, `live_labels`, `spell_*`, `key_verb(s)`, `after`, `join`, `word_prefix`) and `apply`/`parse_list`; open
  a gap row for the rest with the numbers above.

### 2. `?` at the Map, stash list and journal lists keys the overlay swallows — `/` among them since 09e (knock-on; by reading, not run)

- **What the code does.** `palette.rs:167` opens `?` at `context_of`, which is `Map`/`Stash`/`Journal` with the graph
  focused (`update/edge.rs:84–86`). `place_rows` (`palette.rs:276–292`) lists every row of `keymap.reachable(at)` whose
  chord `key_in` (`:303–310`) resolves there and is not `Unbound`. Those places' scope is `[Map|Stash|Journal, Graph,
  Global]` (`graft-ui/src/keymap.rs:151–153`), so `c`, `a`, `R`, `m`, … (Graph rows) resolve and are listed — the
  very rows 09d's sweep records as swallowed there (`update/tests/keymap_edge/sweep_swallowed.rs`, `R_MAP` rows).
- **09e's part.** 09e made `/` idle under the three overlays (`update/search.rs:20–28`, `graph_is_shown`) and moved
  Map/Stash `app.find` to `SWALLOWED_BY_DESIGN`, but `?`'s EVERYWHERE list still spells `/ Find…` there (`app.find` is
  Global, resolves at `Map`, `translate` gives `Pass`). 09e's spec asked for exactly this check; its NOTES say "likely,
  unverified" and no gap row was opened.
- **Why it matters:** `?` is the "what does this key do here" journey (J5) and the place §2.1 says shows live keys;
  listing a dozen dead keys at an overlay is the dead-hint failure ADR-0041 / ux-principles row 6 forbid, in the screen
  meant to answer it.
- **Fix (ticket 11's file):** `key_in` should drop a row the place swallows — either a `swallows(at)` list next to
  `borrows()` (read off the overlays' `shown_key`, like `borrows` cites its lines), or narrow the overlays' scope.
  Open a gap row now; a test: at each overlay, every `?` row's key changes the frame (09e's frame oracle, reused).

### 3. Defects and questions this group found are not in KNOWN_GAPS (rule 4)

Each was found by a builder, recorded only in ticket NOTES or a test-only table:

- **Blame `Home`/`End` do nothing** though `BLAME_BORROWS` claims `app.top`/`app.bottom` (`app.top` = `[g, Home]`,
  `keymap/table.rs:92–93`); the blame arm answers only `g`/`G` (`update/place/keys.rs:57–58`). Confirmed by reading.
  Listed in `ONE_CHORD_IDLE` (09d NOTES "Questions").
- **Resolver and focused banner swallow `⌃Z`** (an `app.undo` chord) while answering `u` — 09d NOTES cite
  `resolver.rs:379/383`, `banner/focus_keys.rs:147/173`; changes pane has no `Home` arm (`detail/keys.rs:307`).
  Not re-verified by me (cited as 09d's lines).
- **`AFTER y` literal** in the rewrite editor's right pane (`graft-ui/src/rewrite_editor/right.rs:143–144`) is not
  spelled from the table and the grep test does not see it (09d NOTES "Follow-up gap").
- **Fix:** one gap row each (the first is **broken**, the second **unverified**, the third **broken**), so the usability
  walk and 11/15 can find them.

### 4. ARCHITECTURE.md says nothing about the keymap seam this group made structural (PROMPT 6.1)

- `ARCHITECTURE.md:84–85` ("Changing a screen or a keybinding → `ui` for the widget, `app` for the key-to-`Command`
  mapping") is the only key entry. Absent: the one table (`graft-ui::keymap`), translate at the edge
  (`graft::update::edge::key`, `context_of`), the rule that every printed key is spelled from the live table, and
  10e's new cross-crate contract — graft-ops names keys by `OpKey` through `KeyNames` (`graft-ops/src/keys.rs`), graft
  answers in `update/op_keys.rs`, jobs carry `Arc<Keymap>`, and "no production impl holds default letters".
- **Why it matters:** the next builder adding a refusal in graft-ops or a footer in graft-ui has no pointer to the rule
  the grep ratchet will fail them on; the seam is enforced only by a graft-ui test reading other crates' sources.
- **Fix:** a "Changing a key, or words that name one" entry: table + golden ids, `edge` translate, `Keymap::spell_*` /
  `key_verb` at render, `HintKey`, `OpKey`/`KeyNames`/`OpKeys`, `Keymap::shipped()` for text fields, and the
  `no_literal_keys` ratchet.

## Minor

1. **R2's shared prefix makes a mixed pair ambiguous under a remap.** `join` (`keymap/resolve.rs:420–436`) writes a
   repeated `alt+` once (`alt+↑/↓`) but prints a following plain label whole, so `alt+↑` then a remapped `J` reads
   `alt+↑/J` — by R2's own convention, `alt+J`. Only when one key of a pair is remapped. Fix: share the prefix only
   when **every** label in the run has it; else spell each whole. Add the case to `a_shared_modifier_word_is_written_once`.
2. **Duplicated doc comment:** `resolve.rs:413` ("`labels` joined with `/`, or `None` when there are none.") sits on
   top of the real doc (`:414–419`). Delete it.
3. **`ACTING_AT_LANDING`'s comment credits 09e with +48** (`update/tests/keymap_edge/sweep.rs:26–30`: "444 after
   m8/09e … which made R/C/O act"). 09e moved 6 rows to acting (`sweep_tables.rs` diff); its NOTES say the rest came
   from merges in between (ticket 11's tables). Say so in the comment.
4. **R1's test checks one width, not 40..=240.** `resize_is_drawn_only_when_nothing_else_was_dropped`
   (`graph_screen/hints/tests.rs:406–411`) finds the first width that draws resize with `partition_point` — which
   assumes the predicate is monotone — and checks only there. The tick (10c Acceptance R1) says every width. Loop the
   widths, or assert monotonicity.
5. **`R` refused from a pane still takes the focus.** `update/keys.rs:421` moves focus to the graph before
   `open_rewrite`, which can refuse and return (`update/rewrite_editor/open.rs:33–36`). He presses `R` in the diff,
   reads a refusal, and his pane lost the keyboard. Set the focus only once the read is asked (after `:43`), or on a
   refusal put it back.
6. **A graft-ui test reads graft's and graft-ops' sources by path** (`graft-ui/tests/no_literal_keys.rs:140–150`,
   10b). `cargo test -p graft-ui` now fails on a graft edit — the coupling the crate table forbids for code, in a test.
   Move the ratchet to `crates/graft/tests/it/` (graft may see every crate) and leave graft-ui's own scan in graft-ui.
7. **Five gap rows still open that this group closed** (conductor): M8/10c-literal-keys-allowlisted
   (`KNOWN_GAPS.md:1137`, 09d emptied `KNOWN`), M8/10c-r4-unbound-open (`:1138`, 09c), M8/09b-emptystate-sweep-partial
   (`:1131`, 09d Major 4 + F2 pin), M8/09d-sweep-defects (`:1142`) and M8/09d-empty-state-cancel-dead (`:1143`, 09e).
   M8/09-review-minors (`:1132`): Minors 2 and 12 are closed by 09d; Minor 8 half (keymap_edge split; `table.rs` still 509 lines).
8. **D7 is not in the design doc.** 02-keymap §2.1 (`docs/design/02-keymap.md:180–193`) says every surface spells
   from the live table, not that a chord taken anywhere is not spelled for its loser anywhere (after `merge = "j"`,
   move spells `↓` alone even where `j` moves). One sentence.
9. **`Spelled::key` leaks a string per call** (`crates/graft/src/spelled.rs:20–26`, `Box::leak`, test-only). The
   sweeps call it in loops over contexts × widths; memory grows for the life of the test binary. Return `String`
   and compare with `.as_str()`.

## Questions (no line proves a defect)

1. **D7 drops a lost chord app-wide.** `relabel(&taken)` (`resolve.rs:88–99`, `:297–307`) removes `(chord, loser)`
   from every bar when any one place takes it. For `merge = "j"` the planner accepted the omission. Is the same
   wanted when a line takes a Global key in one modal place only (the loser keeps it everywhere else)? `?` is per-place
   and unaffected.
2. **10e's `ApplyError::Journal` staging clause** (`graft-ops/src/apply/set_aside/answer.rs:322–327`, now keyless
   `Display`) — already a gap row (M8/10e-open-points). Whether `u` reaches an entry whose journal write failed decides
   if dropping `u undoes it` is a regression or a fix.
3. **Changes-pane-over-overlay bar uses ` · `, the commit pane's uses two spaces** (`docs/shots/m8/changes-over-stash-bar-80x24.txt`
   last row vs `pane-over-map-bar-80x24.txt`). Both are the pre-existing styles of their lists; for the usability
   walk, not a defect.

## Asked about by the conductor

- **Against `02-keymap.md`:** 09c's §2.1 amendment is in (`:187–193`); `empty_state.cancel` deletion matches §2.13
  (no `n` row); §2.2 `/` "find in the focused list" matches 09e's stand-down. D7 missing (Minor 8).
- **Errors swallowed / unwrap / `let _ =`:** none added outside tests. Non-test `unwrap_or_default()` in
  `preview_box.rs:75`, `journal_screen.rs:248`, `map_screen/render.rs:131` turn an *unbound* key (`None`) into no
  text — W3 by design, not a failure.
- **Escapes:** F2 sanitising now pinned (09d). Words spelled from config pass through `KeyLabel`; no new raw path seen.
- **Git on the UI thread / data loss:** none — keys, words and bars only; jobs gain an `Arc<Keymap>` clone.
- **Crate rules:** graft-ops has no graft-ui edge (`crates/graft-ops/Cargo.toml:10–11`; no `graft_ui` in its src).
