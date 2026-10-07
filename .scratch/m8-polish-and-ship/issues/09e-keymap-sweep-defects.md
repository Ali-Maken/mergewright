# G-keymap 1e — the nine keys 09d's sweep found idle, and the dead `empty_state.cancel` row

Status: draft (planner spec-only pass, 2026-10-07; from 09d NOTES "DEFECTS found by the sweep" and "D6 knock-on
(question)"; KNOWN_GAPS `M8/09d-empty-state-cancel-dead` and the 09d sweep-defects row)
Build with: **Sonnet** (provisional — brownfield, ADR-0035: conductor confirms after the scout, if one runs). Every
ruling is made below, each fix is a gate or an arm at a line 09d already named, and 09d's sweep is the oracle that
flips each row. Not Fable: nothing structural is left open.
Blocked by: 11 (conductor 2026-10-07: 11 was already building when 09e was specced; 09e rebases onto it rather than stopping 11)
Needs: from 09d — the sweep (`update::tests::keymap_edge::sweep`), its tables `SWALLOWED_BY_DESIGN`,
`NO_EFFECT_IN_FIXTURE`, `SHADOWED`, `DEFECTS`, `ACTING_AT_LANDING = 396`; `empty_state::sweep_tests`;
`empty_state::tests::the_init_box_answers_the_same_keys_in_confirm`. From m6/85 — the S/J-from-the-pane precedent
(`update::keys::tests::s_opens_the_stash_list_from_the_changes_pane`, `…::j_opens_the_journal_from_the_changes_pane`).
Scout: conductor's call. 09d NOTES pins every line below; m6/85's scout covers the S/J precedent.

**Files (for scheduling against 11, 10b, 10e).**
- graft: the four gate sites 09d named — `update/keys.rs` (:405, `R`), `update/picker.rs` (:125, `C`),
  the overview `strip.rs` (:17, `O`) — and `update/search.rs` (:20, `/`); `empty_state.rs` (the init box arm,
  :110–116) + its tests and `sweep_tests`; `update/tests/keymap_edge/{sweep,sweep_tables,sweep_swallowed}.rs`;
  `update/keys.rs` tests; `MAP.md`. **Not** `update/state.rs` (`modal_owns_keys`, `graph_has_focus` unchanged),
  **not** `update/map.rs` (`M` stays as is), not `view/*`, not `update/edge.rs`.
- graft-ui: `keymap/table.rs` (one row deleted), `tests/keymap_ids.txt`, any `keymap/tests/*` naming the row,
  `MAP.md`. No `keymap/resolve.rs`, no `borrows`.
- **Overlap with 11:** yes — `update/tests/keymap_edge*` (both edit the sweep tables; 11 deletes rows and reads
  `SWALLOWED_BY_DESIGN`), `empty_state*` tests, both MAP.md. **09e first:** it is small, and it settles what 11's `?`
  must show — which Graph rows act from the panes, that Diff `M` and Map/Stash `/` are idle by design (11 reads
  that table), and that the start screen has no `cancel` row to list. 11 rebases on it.
- 10b / 10e: no file named above is theirs as far as their specs' crate lines go — conductor confirms.

## Read first
- `09d-keymap-sweeps-and-literals.md` NOTES: "DEFECTS found by the sweep", "D6 knock-on (question)", "Init box"
- `docs/design/02-keymap.md` §2.1 modal list (:61–113, the Map/Journal/Stash bullet :100–105), §2.1 remap paragraph
  (:174–193, "a line acts where its section is live", "every binding is remappable"), §2.2 `/` (:210), §2.3 `R` `C`
  `M` `O` (:248–249, :266, :268), §2.3a (:279), §2.3c (:296), §2.4 `M` (:308), §2.9 (:470–479), §2.13 (:542–555)
- `.scratch/m6-conflicts/issues/85-usability-2-fixes.md` m2 (:48–49) and NOTES (:221) — S/J from the pane
- `docs/research/ux-principles.md` rows 3, 4, 6 (:53–56)
- `crates/graft/MAP.md` `empty_state` (:6), `AppState::graph_has_focus` (:215); `crates/graft-ui/MAP.md` `## keymap`
  (`ContextId::borrows`, `reachable`)
- Source after the maps: each line in the Files list; the sweep tables' `DEFECTS` rows

## Rulings (planner; not the builder's)

**Defects 1–2 — Map `/`, Stash `/`: defect, fix (idle by design, never the graph's search).** §2.1 (:100–102): the
Map and Stash overlays are modal — each "claims the keyboard … before the graph sees it". Neither table has `/`
(§2.3a :277–282, §2.9 :472–479). Today `/` opens the graph's search prompt behind the overlay (search.rs:20): unseen,
and it takes his next keys — a hidden mode (ux row 6). Options: (a) the search opener refuses while the Map or Stash
overlay is shown; (b) each overlay's key arm swallows `/`; (c) build find in the Map and stash lists. **Take (a)** —
the cause is the opener not asking who holds the keyboard; one guard covers both and any later overlay. Reject (c): a
new feature with no prior-art study (pipeline step 2); the journal's and blame's finds were designed (§2.8, §2.11).
Reject widening `modal_owns_keys` (state.rs:907–915): it also drives the banner tab and bottom-row rungs — knock-on
far past one key. Guard reuses whatever predicate the overlays' own key arms use to say "I am up"; no new state.
- He sees: Map or stash open, `/` → nothing changes; `j`/`k` still move the overlay's row. Bar unchanged.
- Rows flip to `SWALLOWED_BY_DESIGN` with why `§2.1 :100 overlay claims the keyboard; no / in §2.3a/§2.9`.

**Defects 3–8 — Changes and Diff, `R` `C` `O`: defect, fix (act as from the graph).** The detail pane is not modal
(§2.1's list :61–113 does not name it), so the graph's rows are live there (§2.1 :187 "a line acts where its section
is live"; graft-ui `scope()` puts Graph in both). `c a m r b t f p` act from the panes (09d NOTES), and since m6/85 m2
so do `S` and `J` — graft MAP :215 still says they read `graph_has_focus`, stale. `R`/`C`/`O` are the last
uppercase graph keys gated on `graph_has_focus` (keys.rs:405, picker.rs:125, strip.rs:17) for no reason the design
gives; pressing one in a pane does nothing, silently (ux rows 4, 6). Remove the `graph_has_focus` term from each
gate; keep every other term (overlay up, operation state, `R`'s "no overlay").
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
does nothing; the bar never offered it there, and 11's `?` must not list it (it reads `SWALLOWED_BY_DESIGN`).
- Row flips to `SWALLOWED_BY_DESIGN` with why `§2.3a :279 Map opens from the graph pane; M in the detail pane is the
  mergetool's (§2.4 :308)`.

**Question — `empty_state.cancel` after D6: dead, delete the row; the init box stops answering raw `n`.** §2.13
(:546–555) has no cancel/`n` row; the init box is a confirm box (09d D6), where `Esc` cancels and **any other key is
swallowed** (§2.3c :296). After D6 the row is reachable only on the list, where `n` does nothing; in the box, `n`
declines through a raw arm (empty_state.rs:112) no row names — a key his config cannot move (§2.1 :174 "every
binding is remappable"). Options: (a) delete the row and the raw `n` arm; (b) add `confirm.no = "n"` for every box —
a second dismiss key in every confirm box, a §2.3c change, against ux row 3 (one obvious exit); (c) keep the row, map it
to the box's `n` — a row for one undocumented key. **Take (a)**, pre-release, no alias (as 09c removed
`empty_state.confirm`). He sees: init box `y init · esc keep it` (unchanged — `n` was never drawn); `n` → box stays.
- **Stop (rule 7, not checked against code):** if the shared confirm box's handler answers raw `n` today, the init
  box is consistent with it and this ruling is wrong — NOTES names that `file:line`; report, don't pick. Same if any
  non-test site reads `empty_state.cancel` to draw a line.

**Out:** the `ONE_CHORD_IDLE` questions (resolver/banner `⌃Z`, blame `home`/`end`, changes `home`) — not ruled here;
the hint bar (`hint_context` still reads `graph_has_focus`; ranking is 15); `?` content (11).

## Signatures
No new `pub` items. No signature changes.
```rust
// graft-ui::keymap::table — row deleted (golden list tests/keymap_ids.txt loses `empty_state.cancel`)
// Binding { action: ActionId("empty_state.cancel"), context: ContextId::EmptyState, default: &[n], .. }  // gone
// graft::update::tests::keymap_edge — DEFECTS ends empty and is deleted with its loop; nine rows moved:
//   6 → acting (none listed), 3 → SWALLOWED_BY_DESIGN (Map "/", Stash "/", Diff graph.map)
const ACTING_AT_LANDING: usize; // raised to the new count, date 09e's
```

## Acceptance (each tick names its test, at tick time)
- [ ] **Defects 1–2.** Map open, `/` → no search prompt open, `AppState` unchanged; then `j` moves the Map row. Same
  for the stash list. Red first. (`update::keys::tests::slash_in_the_map_opens_no_search`, `…_in_the_stash_list…`)
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
  `empty_state::sweep_tests` green with the row gone (case count in NOTES).
- [ ] Captures: full `cargo nextest run` with `GRAFT_UPDATE_SHOTS` unset — every committed capture equal (no bar or
  box draws a changed key under the default keymap). `git status docs/shots` clean. Colour unverified (rule 5).
- [ ] MAP.md — graft: `graph_has_focus` read by `M` (and `hint_context`, `offer_keys_live`) only; `S`/`J` (m6/85)
  and `R`/`C`/`O` (09e) answer from the panes; `empty_state` init box answers `y`/`esc` only; search refuses under
  the Map/Stash overlay. graft-ui: the row's removal. (doc, no test)

## Touches
- Callers of the three gates and the opener: grep each function; the dispatcher tests (`update::keys::tests`) and
  m6/85's S/J tests green.
- Other screens drawing this state: hint bar unchanged (`hint_context` untouched) — captures prove it. Overview strip
  drawn at the pane-focused sizes: existing captures cover it toggled from the graph; no new capture.
- Keymap: one row deleted (golden list); no chord moves. 02-keymap.md needs no edit (every ruling is its text).
- Journal/undo: none — key routing only; `R`/`C` open places whose writes are already journaled. Show the empty
  `git diff --stat` over `crates/graft/src/update/journal*` and `crates/graft-ops`.
- 11 (`?` reads `SWALLOWED_BY_DESIGN`; no `cancel` on the start screen) — conductor notes it on 11. 18 (Clone prompt)
  does not name `empty_state.cancel` (planner grep of `.scratch/m8-polish-and-ship/issues/`, 2026-10-07).
- KNOWN_GAPS: `M8/09d-empty-state-cancel-dead` and the 09d sweep-defects row close (conductor).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~35k: three gates + landing tests ~12k; search guard ~5k; row + arm + tests ~8k; sweep tables, count, MAP ~10k.
