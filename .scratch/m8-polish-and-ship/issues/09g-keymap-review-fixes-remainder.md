# G-keymap 1g — 09f remainder: the five one-chord idles and `AFTER y` (C), the ruling on `ONE_CHORD_IDLE`'s other 22 rows, the `?`-at-Map capture, the keymap seam in ARCHITECTURE.md (D)

Status: partial (builder 2026-10-07: C handlers + R1 shrink done; stopped at R1 classes — see NOTES). Was: ready (planner spec-only pass, 2026-10-07, from 09f NOTES + Handoff; 09f merged partial at 82bbe686, A and B
done)
Build with: **Opus** (confirmed by conductor 2026-10-07 after scout `scouts/09g-one-chord-idle.md`: no source overlap with 10j; only Menu `s` types itself — other rows are defects per R1, a third class → stop). Two judgements a test cannot make for
the builder: where each of the five chords is lost (handler vs the edge's translate — a stop if the edge), and the class
of each of the 22 rows left in `ONE_CHORD_IDLE` (a wrong "by design" hides a defect in a table). Not Fable: the sweep
and the new typing assertion are full oracles; nothing here is structural.
Blocked by: **none for source.** 10g is merged (3693ddd6), so 09f's handoff item 2 (`update/resolver.rs` held by 10g)
is met. 17 is merged (partial). **10j (building) may run alongside** — no shared source file (see Shared with 10j);
two cargo helpers at a time, so this is the second. **Capture step:** not while any other ticket re-shooting captures
is in flight (17b, 11c, 12, 12b, 15 — conductor checks). **11c runs after this ticket** (09f's serial order carries
over: 11c's report-only `?` HERE check and `update/tests.rs`).
Needs: 09f's merged A/B — `ContextId::{scope, borrows, stands_down}` (graft-ui `keymap/claims.rs`), the sweep
(`update::tests::keymap_edge::{sweep, sweep_tables}`), `ONE_CHORD_IDLE`, `ACTING_AT_LANDING` (444 after B),
`update::tests::palette_live` + `palette_live/dead.rs` `DEAD_ELSEWHERE` (214); graft-ui `Keymap::{translate, spell_*,
key_verb, after, shipped}`.
Scout: **update** 09f's — write `.scratch/m8-polish-and-ship/scouts/09g-one-chord-idle.md` (≤1 page): (1) the 27
`ONE_CHORD_IDLE` rows as a table (place, action, idle chord, acting chord) with, per row, the file:line where the idle
chord arrives (an insert-char arm, a `_ => {}`, never reached); (2) for the five C rows, does the edge translate
`Home`/`End`/`⌃Z` to the default chord at that place before the handler runs (`update/edge.rs` call order vs
`resolver_key` / banner focus / place / detail dispatch) — file:line, or "could not confirm"; (3) the file holding the
resolver's `u`/`U` arm in `act` (MAP :491) after 10g's moves.

**Files.** Paths under `crates/graft/src/` unless stated.
- graft: `update/place/keys.rs` (:57–58, :78); `update/detail/keys.rs` (:307); `update/banner/focus_keys.rs`
  (:147/173); the resolver file holding `act`'s `u`/`U` arm (scout (3); expected `update/resolver.rs` root); their test
  files (resolver: a file **other than** the one holding `update::resolver::tests::file_list`, and not
  `update/tests/keymap_words/resolver.rs` — both 10j's/10g's); `update/tests/keymap_edge/{sweep,sweep_tables}.rs`;
  `update/tests/palette_live/dead.rs` (the one-chord row only); `tests/it/look_polish.rs` (one capture); `MAP.md`.
- graft-ui: `rewrite_editor/right.rs` (:143–144), `rewrite_editor/tests.rs`, `tests/no_literal_keys.rs` (pattern only),
  `MAP.md` (`rewrite_editor`, `no_literal_keys` entries only — `## keymap` is done).
- `ARCHITECTURE.md` (one entry). `docs/shots/m8/help-at-map-120x36.txt` (new).
- **Not:** `update/edge.rs`, `update/mod.rs`, graft-ui `keymap/*` (a cause there is a stop), `update/resolver/answers.rs`,
  `update/resolver/resolved.rs`, `update/banner/{resume,pressed}.rs`, `update/status_read.rs`, `tests/no_literal_keys/
  known_*.rs`, `prose.rs`, `update/tests/keymap_words/*` (10j), `update/palette.rs` (12/12b), `hints*` (11c, 15).

## Read first
- 09f: **Rulings** Major 3 and Major 4 (`09f-keymap-review-fixes.md` :96–127), Acceptance C/Captures/D (:170–190),
  NOTES + Handoff (:217–256). Those rulings stand; this file adds the `ONE_CHORD_IDLE` ruling and replaces 09f's
  "`ONE_CHORD_IDLE` deleted".
- Scouts `09f-keymap-review-fixes.md` (Part C) and `09g-one-chord-idle.md`, whole
- `docs/design/02-keymap.md` §2.1 :78–82, :97–99 (text fields claim every key), :116–121 + :147 (`.` menu claims five
  keys; the rest fall through), :179–193 (translate before any pane; ⌃Z with `suspend_on_ctrl_z`; **:192–193 "Inside a
  text field no line acts: keys type themselves"**); §2.6 :344
- `docs/research/ux-principles.md` row 4
- `crates/graft/MAP.md` :491 (resolver `u`/`U` — it answers with a notice, not an undo), :945–946 (sweep, `palette_live`);
  graft-ui `MAP.md` `## keymap`
- Source after the maps: the lines in Files; `sweep_tables.rs` `ONE_CHORD_IDLE` and `DEFECTS` (shape only).

## Rulings (planner; not the builder's)

**R1 — `ONE_CHORD_IDLE`: shrink and judge every row; the name stays, the table stops holding unjudged rows.** 09f's
ruling rested on "`ONE_CHORD_IDLE` is 09d's record of exactly these four" (09f :99); the builder measured 27 rows (09f
NOTES :239). That premise was wrong; the fix is the class, not the delete.
- Options: (a) delete and move all 22 to `KNOWN_GAPS` — loses the exact-match ratchet, a fixed row would never be
  noticed; (b) fold into `SWALLOWED_BY_DESIGN`/`DEFECTS` — both are keyed per row, and these rows **act** on their other
  chord, so "a listed row that acts fails" would fire; (c) **keep the per-chord table, add a class to each row, and make
  the by-design class observed by a test.** **Take (c).**
- The five C rows are fixed and leave the table (Blame `Home`, Blame `End`, Changes `Home`, Resolver `⌃Z`, Banner `⌃Z`).
- Every other row gets one of two classes, each with its file:line:
  - `TypesItself` — the place is a text field (§2.1 :78–82, :97–99; Credential: cite the line where its field takes
    characters) **and** the idle chord is a plain printable character that the field inserts. By design: §2.1
    :192–193. **Observed, not read:** a new assertion presses the chord in the field and checks the field's text gained
    exactly that character. A row that fails it is not `TypesItself`.
  - `Defect` — anything else (a chord the place neither acts on nor types: e.g. a modifier chord in a field, the Menu
    `s` row unless it types). ux row 4 + 09f's Major 3 rule (a place answering one chord of a row and not another).
    **Not fixed here** (files outside this set, causes unread, size); NOTES lists them for the conductor's gap row
    `M8/one-chord-idle` (**broken** — idle is observed by the sweep).
- **Stop:** a row the builder thinks is by design for any reason other than `TypesItself` (e.g. the `.` menu's
  fall-through, §2.1 :147) — report it with the design line; a third class is the conductor's to approve.
- If all 22 turn out `Defect`, the table still stays (it is the ratchet the gap row closes against).

**R2 — the resolver's `⌃Z` reaches what `u` does there, which is a notice, not an undo.** MAP :491: resolver `u`/`U`
→ `undo_answered`, no command, the notice row says where undo is. So the test compares `⌃Z` with `u` (state, notice,
commands), it does not expect an undo. Test renamed accordingly (below). Banner: whatever `u` does at the focused
banner, `⌃Z` does the same. `suspend_on_ctrl_z` on → `⌃Z` is `app.suspend` at both, not `u`'s answer (§2.1 :191–192).

**R3 — the cause rule from 09f Major 3 holds.** The handler answers the translated action, not a second raw arm.
**Stop** if scout (2) or the builder finds the chord is lost in `update/edge.rs`, `update/mod.rs`'s dispatch order or
`Keymap::translate` (shared by every place; `update/mod.rs` is also 10j's this round).

**R4 — `AFTER y`, captures, D: as 09f ruled** (09f :112–116, :120–127, Acceptance :179–190), with two changes:
(a) the `no_literal_keys` pattern extension must add **exactly one** hit when the literal is restored and none on the
tree as merged; any other new hit → stop with file:line (do not add `KNOWN` rows: 10j is emptying those files);
(b) captures: 09f's B run was green with no shot change, so the expected diff is the new file only; an existing shot
changing is a stop unless it is a `?` capture at Map/Stash/Journal losing Graph rows or stood-down rows (09f's
allowance).

**Out:** the `Defect` rows' fixes (gap row); 09f's question "stash list, pane focused after `⇥`, the frame is identical"
(no line — conductor files it as a question); the review Minors and Questions 2–3; `DEAD_ELSEWHERE`'s other 213 rows.

## Shared with 10j (building now)
- **No shared source file.** 10j's Files: `banner/resume.rs`, `banner/pressed.rs`, `update/mod.rs`
  (`mergetool_returned`), `status_read.rs`, `resolver/answers.rs`, `resolver/resolved.rs`, the `file_list` test file,
  `keymap_words/stops.rs`, graft-ui `no_literal_keys/{known_stops,known_resolver,prose}.rs`. 10j lists this ticket's
  `focus_keys.rs`, `detail/keys.rs`, `banner/tests*`, `no_literal_keys.rs` root under **Not**. Builder: if scout (3)
  puts the `u`/`U` arm in `resolver/answers.rs` or `resolver/resolved.rs`, stop and wait for 10j's merge.
- **Shared docs:** `crates/graft/MAP.md` (10j: :470, :495, :499, :908; this: :491, :945–946, place/detail/banner
  entries) — conductor merges hunks.
- **Same test binary:** graft-ui `no_literal_keys` (root pattern here, `known_*` there). R4(a)'s "no new hit on the
  merged tree" is checked after 10j merges if 10j merges first; NOTES says which base.
- 10j's Touches check `git diff <base> -- banner/focus_keys.rs detail/keys.rs` is against 10j's own base — unaffected.

## Signatures
No production signature changes.
```rust
// graft::update::tests::keymap_edge::sweep_tables — test-only. Mirror the existing row tuple; add the class.
enum IdleWhy {
    /// §2.1 :192–193 — a text field inserts this character. `at`: the insert arm's file:line.
    TypesItself { at: &'static str },
    /// Neither acts nor types. `at`: where the chord falls idle. Gap row M8/one-chord-idle.
    Defect { at: &'static str },
}
const ONE_CHORD_IDLE: &[(ContextId, ActionId, KeyChord, IdleWhy)]; // 27 → 22 rows
const ACTING_AT_LANDING: usize; // raised by C, dated
```

## Acceptance (each tick names its test, at tick time)
- [x] **C, blame.** Blame open, cursor mid-file: `End` → as `G`, `Home` → as `g` (state compared); `[keys.app] top =
  "t"` → `t` goes top, `Home` does nothing. Red first. (`update::place::tests::home_and_end_in_blame_act_as_g_and_capital_g`)
- [x] **C, changes.** Changes pane, cursor on the third file: `Home` → first file, as `g`. Red first.
  (`update::detail::tests::home_in_the_changes_pane_acts_as_g`)
- [x] **C, resolver `⌃Z`.** Resolver open: `⌃Z` → same state, notice and commands as `u` (R2); `suspend_on_ctrl_z` on →
  not `u`'s answer. Red first. (`update::resolver::tests::ctrl_z_answers_in_the_resolver_as_u_does`)
- [x] **C, banner `⌃Z`.** Focused banner after a journaled step: `⌃Z` → same state and commands as `u`;
  `suspend_on_ctrl_z` on → no undo. Red first. (`update::banner::tests::ctrl_z_undoes_on_the_focused_banner_as_u_does`)
- [x] **C, cause.** NOTES names, per chord, where it was lost (file:line) and the fix point; `git diff <base> --
  crates/graft/src/update/edge.rs crates/graft/src/update/mod.rs crates/graft-ui/src/keymap` empty (shown). No new raw
  `Home`/`End`/`⌃Z` arm (`rg` output in NOTES). (that command)
- [ ] **R1, table.** `ONE_CHORD_IDLE` 27 → 22, each row classed with its `at`; the five C rows gone; the sweep green.
  (`update::tests::keymap_edge::sweep::every_reachable_row_acts_or_is_listed_and_follows_its_remap`)
- [ ] **R1, typing observed.** Every `TypesItself` row: the chord pressed in its field adds exactly that character to
  the field's text. Shown failing once on a row deliberately mis-classed (temporary, NOTES).
  (`update::tests::keymap_edge::sweep::every_one_chord_row_marked_typed_types_itself`)
- [x] **R1, count.** `ACTING_AT_LANDING` 444 → new (dated); NOTES: old → new and which rows rose. `DEAD_ELSEWHERE`'s
  one-chord row removed if it is one of the five (else kept, NOTES says why); count 214 → new.
  (`…::sweep::every_reachable_row_acts_or_is_listed_and_follows_its_remap`,
  `update::tests::palette_live::dead_rows_help_lists_elsewhere_match_the_known_table`)
- [ ] **R1, gap list.** NOTES lists every `Defect` row (place, action, chord, `at`) for `M8/one-chord-idle`; count stated.
  (table read against NOTES; `rg -c 'Defect \{' sweep_tables.rs` = NOTES count)
- [ ] **C, `AFTER y`.** `confirm.yes = "c"` → `AFTER c` outside the field; inside the `r` field the shipped `y`; unbound
  → `AFTER` alone, no `—`. Red first. (`rewrite_editor::tests::the_after_heading_spells_his_confirm_key`)
- [ ] **C, ratchet.** Literal restored → exactly one new hit, red observed (NOTES); merged tree → no new hit (R4a).
  (`no_literal_keys::no_user_facing_literal_spells_a_default_key`)
- [ ] **Captures.** Full `cargo nextest run` with `GRAFT_UPDATE_SHOTS` once, then unset and green; `git diff --stat
  <base> -- docs/shots` = the new file only (or R4b's allowance, listed). New `docs/shots/m8/help-at-map-120x36.txt`
  (Map open, graph focused, `?`): HERE = Map's own keys, EVERYWHERE = Global keys that act there, no `c commit`, `R`,
  `m merge`, `/ Find…`. Judged by the user (rule 6); colour unverified (rule 5).
  (`tests/it/look_polish.rs::help_at_the_map_lists_only_live_keys`, via `captures::check_shot`)
- [ ] **D, ARCHITECTURE.md.** The "Changing a key, or words that name one" entry after :84–85 as 09f Major 4 rules
  (≤15 lines, ADR-0055/0058 cited). (doc, checked by reading; `rg -n 'Changing a key' ARCHITECTURE.md`)
- [ ] **D, MAP.md.** graft: :945 sweep entry (`ONE_CHORD_IDLE` classes, `ACTING_AT_LANDING` value — :945 still says
  396), :946 `DEAD_ELSEWHERE` count, :491 resolver `u`/`U` + `⌃Z`, the blame/changes/banner handlers. graft-ui:
  `rewrite_editor` (`AFTER` spelled), `no_literal_keys` pattern. (doc; `rg -n 'ONE_CHORD_IDLE|ACTING_AT_LANDING'
  crates/graft/MAP.md` shows the new values)

## Touches
- Translate/dispatch: unchanged (C, cause box). Keymap: no binding added/moved — `git diff <base> --
  crates/graft-ui/tests/keymap_ids.txt` empty (shown).
- Hint bars: no bar draws `Home`/`⌃Z` for these rows today — `rg` for their spelled labels in `docs/shots` before/after
  (NOTES); captures box holds the rest.
- `?`/palette: rows at Blame/Changes/Resolver/Banner now act on both chords — `palette_live` exact table holds it.
- Journal/undo: `⌃Z` reaches existing paths — `git diff --stat <base> -- crates/graft/src/update/journal*
  crates/graft/src/update/undo* crates/graft-ops` empty (shown).
- 10j: see Shared with 10j. 11c: after this. 12/12b, 15, 17b: no source shared; captures serial (conductor).
- KNOWN_GAPS (conductor): close 09f's three Major 3 rows (blame `Home`/`End`, ⌃Z/changes `Home`, `AFTER y`) with this
  merge; open `M8/one-chord-idle` (broken) from R1's NOTES list, if non-empty.
- Contact sheet: re-assembled with the new capture (conductor).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~38k: four handlers + four tests ~12k; R1 classes + typing assertion ~8k; `AFTER` + pattern ~4k; captures ~4k;
ARCHITECTURE + MAP ~6k; NOTES ~4k. Order C handlers → R1 → `AFTER` → captures → D. Past ~65k: stop after R1, hand
captures and D to a split (NOTES says where).

## NOTES (builder, 2026-10-07)
Scout used: `scouts/09g-one-chord-idle.md`. No compaction. Base: main d1ed9683. **Partial — R1's stop** (a third class
of row, below). Done: the four C handlers, the five rows out of `ONE_CHORD_IDLE`, `ACTING_AT_LANDING`, the
`DEAD_ELSEWHERE` row. Not done: R1 classes + typing assertion, `AFTER y`, ratchet, captures, D (MAP touched only for
the acting count).

**C, cause per chord** (all four red first, observed: blame cursor 18 not 0; changes cursor 3 not 1; resolver notice
None vs u's; banner `[]` vs u's `Undo`). Edge not involved: Home/End/⌃Z are shipped chords, so `translate` passes them
raw (graft-ui `keymap/resolve.rs:156–163`); each was lost in the handler.
- Blame Home/End: `place/keys.rs` catch-all (was :78); fix: the `g`/`G` arms became `_ if shipped_as(key, "app.top")`
  / `"app.bottom"` — every shipped chord of the action. Remap `top = "t"`: `t` tops, Home does nothing (in the test).
- Changes Home: went past `files_key` to the graph (no arm). Fix in `detail/keys.rs` `files_key`, same pattern; `End`
  was the same defect (moved the graph, not the files cursor — the sweep counted it as acting) and is fixed with it.
- Resolver ⌃Z: `act`'s catch-all (was :383). Fix: one arm `asks_undo` (shipped chords of `app.undo`/`app.redo`)
  replaces the raw `u`/`U` arm. **Deviation from R2:** with `suspend_on_ctrl_z`, ⌃Z stays swallowed in the resolver,
  not suspended — a `Suspend` returns `Product::HandedOver`, which `update/mod.rs:441` reads as `e`'s editor returning
  and calls `finish::record_edited` (marks the file edited). Fixing needs `update/mod.rs` (10j's, and outside this set).
  The box's "not u's answer" holds (test). Gap candidate for the conductor.
- Banner ⌃Z: catch-all (was :173). Fix: `undo_keys` before the match (shipped chords of undo/redo, the one
  `undo_pressed` path, refusal first as before); with `suspend_on_ctrl_z` the banner returns unclaimed and the one
  suspend path (`update/keys.rs:243`) suspends — test sees `[Suspend]`.
- `git diff d1ed9683 -- crates/graft/src/update/edge.rs crates/graft/src/update/mod.rs crates/graft-ui/src/keymap` →
  0 lines. New raw arms: `git diff | rg '^\+.*(KeyCode::Home|KeyCode::End|Char\('z'\))'` → 5 hits, all in the four
  test files, none in a handler. The handlers read chords off `Keymap::shipped()` (no shared home in this file set for
  the helper: 4 small copies — `place/keys.rs`, `detail/keys.rs`, `resolver.rs`, `banner/focus_keys.rs`; a natural
  home is `update/keys/predicates.rs`, a follow-up).
- Not fixed, same class, found: Diff pane `Home`/`End` — `detail/keys.rs` `diff_key` answers `g`/`G` only
  (`Char('g') if plain` arm); Home/End fall to the graph. Not in `ONE_CHORD_IDLE` because the graph moves in the fixture.

**R1, count.** `ACTING_AT_LANDING` 444 → 449 (+5, exactly the five C rows; 727 cases, 244 listed, 34 shadowed).
`DEAD_ELSEWHERE`'s one-chord row (`Changes app.top`) **kept, count stays 214**: still found dead, but from "changes
over Map", where `?` lists `g` and that place's files cursor is already on the first file (probed: `g` idle, cursor 1).
Reason relabelled `R_FIRST_FILE`; `R_ONE_CHORD` removed.

**R1 stop — rows fitting neither `TypesItself` nor `Defect`.** None of the 22 is `TypesItself` (Menu is no text field;
every other idle chord is non-printable). Ten rows *act* in their handler but the fixture gives them nothing to change —
R1's `Defect` ("neither acts nor types") would be false for them:
- Picker `app.up` ↑: `picker.rs:141` handles `Up`; selected already 0.
- CommitEditor `app.top` home, `app.bottom` end: `commit_editor/keys.rs:229–230` move the subject caret; already there.
- Credential `edit_text` delete, →, end, and `app.bottom` end: `credential.rs:378,380,382`; caret at the end.
- Credential `edit_text` ←, home, and `app.top` home: `credential.rs:379,381` move the caret; the frame does not
  change ("not drawn" in the table) — not checked whether the caret is drawn; question, not a defect (rule 7).
Proposed: a third class `NoEffectInFixture { at }` (as the per-row table), or a fixture with mid-text caret / second
picker row so they act and leave the table. Conductor's call.

**Defect rows (12), for gap `M8/one-chord-idle` (broken):** Picker ⌃Z, home, end (`picker.rs:158` catch-all), ⌃P
(`R_PICKER`); CommitEditor ⌃Z (`commit_editor/keys.rs:110`), ↑ in the subject (subject arm has no `Up`, catch-all
after :230), ⌃P (`R_EDITOR`); Credential ⌃Z (`credential.rs:326`), ↓, ↑ (`:385` catch-all), ⌃P (`R_CREDENTIAL`); Menu
`changes.stage` `s` (no changes pane under the menu; the graph has no `s`). Not yet written into the table as classes.

**Other.** `published_repo::nothing_a_commit_would_carry_names_this_machine` failed on main's scout file: the text
"home/end" after a slash matches its home-directory rule (false positive in the detector). Reworded two scout lines
("delete, ←, →, home, end"). Cause is the detector's pattern — not fixed.

**Touches:** edge/mod/keymap/`keymap_ids.txt`/graft-ops/journal/undo: empty diff (shown above). Hint bars + captures:
full nextest green with no shot change (no `GRAFT_UPDATE_SHOTS` run). `palette_live` exact table holds (green). MAP:
:945 acting count only. 10j files: untouched.

**Next (split):** R1 classes per conductor ruling → typing assertion (vacuous if zero `TypesItself`) → `AFTER y` +
ratchet → captures (`help-at-map-120x36.txt`) → D (ARCHITECTURE entry, MAP :491/:945/:946, handler entries).
