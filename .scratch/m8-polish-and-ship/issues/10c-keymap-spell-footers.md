# G-keymap 2c — the footers and boxes spell from the table; the bar keeps undo

Status: done but two acceptance items not met as written (R4 second half, Unbound) — deviations in NOTES for a ruling; five allow-listed literals need a follow-up
Build with: **Opus** — shape fixed by 10 and its rulings R1–R5; the rest is a mechanical sweep held by the spell
sweep (c), the grep test and the capture diff. Not Fable: no structural choice is left open.
Blocked by: 10 (partial — **continues on its branch `worktree-agent-a3081d1d2a2f421de`**; 10 lands with this)
Needs: from 10 — `HintKey::{Key, EveryChord, Word, Fixed}`, `Hint::{key_text, text}`, `hints_for(.., &Keymap)`,
`NOT_A_BINDING`, `Keymap::{shipped, live_labels, spell_keys, spell_every, key_verb, after}`,
`AppState::keymap: Arc<Keymap>`, the `PreviewBoxWidget`/`StashScreen` pattern (commit 7378eebd), the
`graph_screen::hints::spell_sweep` tests, `update::tests::keymap_edge::capture_merge_remapped_to_shift_m`. From
09/09b — `KeyWarning::Collision { key, context, kept, lost }`, `config::keymap_from`, `AppState::config_problems`.
Crate set: as 10 — graft-ui + graft (view call sites, the startup path that builds the keymap, `update/tests`).
Not graft status/refusal words, not graft-ops (10b). Not `keymap/table.rs` rows (09c).

## Read first
- `10-keymap-spell-from-the-table.md`: Decisions 1–8 and **Rulings after partial** (main); on the branch, its
  NOTES "Remaining for the follow-up", "Deviations", "Captures"
- `docs/design/02-keymap.md` §2.1 bullets "The hint bar is a ranked selection" and "Every binding is remappable"
- `docs/research/ux-principles.md` §2 rows 3, 4, 6, 8; §3.5
- `crates/graft-ui/MAP.md`: `## keymap`; `graph_screen::hints` (shrink ladder, `<> resize` insertion, words-only
  bars, `HintContext::RewriteEditor`, `Finish`); `finish_summary`; `commit_editor`; `picker`; `journal_screen`;
  `map_screen`; `blame`; `HintContext::History`; `frame_box`; `preview_box`; `stash_screen` (the done pattern)
- `crates/graft/MAP.md`: `update::edge::context_of`; `update::tests`; the empty state's draw path and startup

## Scope
In:
1. **R1** — `</> resize` appended last among the bar's own keys (after `TAIL`), not fourth; the doorway keeps its
   place. graft-ui `graph_screen::hints`.
2. **R2** — `spell_keys`/`spell_every` write a repeated `<word>+` prefix once: `alt+↑/↓`. Glyph modifiers unchanged.
3. **R4** — finish summary Subject/Body hints `Key(<Finish id>)`, not `Fixed`. A key with no Finish row: NOTES.
4. **Footers/boxes still literal** (10's NOTES list), each widget as `StashScreen`: `pub keys: &'a Keymap`, graft
   caller passes `&state.keymap`, parts via `key_verb`/`after`, two-space joins, an unbound part left out:
   - `commit_editor.rs:172,176,227` — text field: spelled from `Keymap::shipped()`; `Commit` a `Word`.
   - `picker.rs:257,259` — text field: `Keymap::shipped()`; `draw_picker` takes no keymap.
   - `journal_screen.rs:46,221` (`LEGEND` `u`, footer `J/esc close`, `b new branch here`, `/ find`, `n N` → `n/N`,
     `space command log`); `journal_screen/tests.rs:418`.
   - `finish_summary.rs:182`, `finish_summary/list.rs:99`, `finish_summary/frame.rs` (`preview_line`, `UNDO_LINE`).
   - `blame.rs:242,272`; `history.rs:347`; `conflict_resolver.rs:105,107` (ASCII variant's ` - ` kept).
   - `map_screen/render.rs:131` (`↑↓ select  ⏎ zoom to commits  M/Esc back`).
   - `graph_screen/mod.rs:~310` (`focus (F/Esc off)`); the commit-graph banner `B`/`X`
     (`graph.write_commit_graph`, `graph.decline_commit_graph`).
   - `frame_box.rs:307,317`; `rewrite_editor/tests.rs:389` (`<> moved` → check).
   - `empty_state.rs:99,101`: the empty state draws from the keymap built from config at startup (`keymap_from`).
     If it draws before that keymap exists, move the build ahead of it (graft startup); NOTES names the line.
     Never `Keymap::shipped()` here — `empty_state.*` is remappable.
5. `preview_box`'s `again` arrives as an action, not spelled text (graft caller changes with it).
6. **R5** — re-shoot `docs/shots/m8-keymap-remapped-120x36.txt` as the whole frame, status row included, the config
   read through the startup path's function (named in NOTES).
7. Tests: spell sweep (c); grep test over graft-ui; R1–R5 tests (Acceptance).
8. Re-shoot (`GRAFT_UPDATE_SHOTS=1` until stable, 2–3 passes); re-assemble the contact sheet of every milestone
   whose captures changed (`scripts/contact-sheet.sh`; m6/m7 `--captions-order`); MAP.md per widget.
Out: graft/graft-ops words (10b). Palette, `?`, doorway words (11). Ranking beyond R1 (15). New table rows (09c).
Docs (10's decision 8).

## Signatures
```rust
// graft-ui::commit_editor — text field: spelled from Keymap::shipped()
pub fn footer_subject() -> String;                 // replaces FOOTER_SUBJECT
pub fn footer_other() -> String;                   // replaces FOOTER_OTHER
// graft-ui::finish_summary
pub struct FinishScreen<'a> { /* view, theme, ascii, */ pub keys: &'a Keymap }
pub fn footer_hints(view: &FinishView, width: u16, keys: &Keymap) -> Vec<Hint>;
pub fn undo_line(keys: &Keymap) -> String;         // replaces UNDO_LINE
// graft-ui::journal_screen
pub struct JournalScreen<'a> { /* …, */ pub keys: &'a Keymap }
pub fn legend(keys: &Keymap) -> String;            // replaces LEGEND
// graft-ui::map_screen
pub struct MapScreen<'a> { /* …, */ pub keys: &'a Keymap }
// graft-ui::preview_box
pub again: Option<(ActionId, &'static str /* verb: "merge" | "rebase" */)>,   // was (SanitizedText, SanitizedText)
```
Blame, history, conflict resolver, graph focus line, empty state, frame_box: the same field (`pub keys: &'a
Keymap`) on their widget, or a trailing `keys: &Keymap` on a free draw fn; MAP.md records each name.

## Acceptance
- [x] **R1** (`graph_screen::hints::tests::resize_is_drawn_only_when_nothing_else_was_dropped`) — `resize_is_drawn_only_when_nothing_else_was_dropped` (or named at tick): every `every_context()` bar,
      every width 40..=240, both dividers: `</> resize` drawn ⇒ every other hint of the list drawn.
- [x] **R3** (`graph_screen::hints::tests::the_hints_resize_once_pushed_off_are_drawn_again`) — named tests: `u undo` drawn at 100 in the contexts of `m2-duplicate-commit`, `m2-evil-merge`;
      `/ search` at 120 in those of `m7/diff-cursor-commit-120x40`, `m7/pane-bar-file-vs-directory-120x40`; `d drop`
      in `RewriteEditor { narrow: true, .. }` at 100. Any context still dropping `u undo` at 100: NOTES, for 15.
- [x] **R2** (`keymap::tests::spelling::a_shared_modifier_word_is_written_once`) — `spell_keys` of the rewrite move pair is `alt+↑/↓`; `⌃O/⌃T` unchanged (test named).
- [~] **R4** — first half ticked (`graph_screen::hints::tests::a_fixed_key_is_drawn_only_where_keys_go_on_as_typed`); second half NOT built — deviation, NOTES — every `HintKey::Fixed(id)` in every list names a context whose `is_text_field()` (test named); the
      finish Subject/Body hints follow a remap of a Finish row in the spell sweep.
- [x] **R5** (`update::tests::keymap_edge::merge_on_shift_m_is_one_collision_that_loses_the_map`, `…::capture_merge_remapped_to_shift_m`) — test: `[keys.graph] merge = "M"` yields one `KeyWarning::Collision` keeping `graph.merge`, losing
      `graph.map`; capture `docs/shots/m8-keymap-remapped-120x36.txt` shows `M merge` on the bar, `Merge … (M)` in
      the `.` menu, no `M map`, and the status row `config: 1 problem · ? shows them`. **Colour unverified**
      (rule 5). Looks-right is the user's.
- [x] **Spell sweep (c)** (helpers `keymap::sweep`; `journal_screen::tests::the_journal_spells_every_key_from_his_keymap`, `blame::tests::blame_spells_its_back_key_from_his_keymap`, `history::tests::history_spells_its_stop_key_from_his_keymap`, `conflict_resolver::tests::the_too_small_line_spells_his_back_key`, `map_screen::tests::the_map_footer_spells_his_keys`, `finish_summary::tests::the_summary_spells_every_key_from_his_keymap`, `preview_box::tests::the_box_spells_every_key_from_his_keymap`, `stash_screen::tests::the_stash_list_spells_his_keys_and_its_prompt_the_shipped_ones`, `empty_state::tests::the_bar_spells_his_keys_and_the_open_prompt_the_shipped_ones`, `graph_screen::tests::the_filter_line_and_the_commit_graph_line_spell_his_keys`; text fields: `commit_editor::tests::the_footers_spell_the_shipped_keys`, `picker::tests::the_closing_words_spell_the_shipped_back_key`) — every footer/box in Scope 4–5 under 10's sweep keymap: the rendered line holds the
      remapped labels and no default label of a moved action as a whole token; text-field footers hold the shipped
      labels whatever the sweep keymap says. Test name at tick.
- [~] Unbound — deviation, NOTES: `m merge` in the box is a `preview.confirm` chord, not `graph.merge` (`update::tests::keymap_edge::the_preview_names_its_again_key_exactly_while_it_confirms`, `preview_box::tests::again_names_a_confirm_chord_only_while_it_confirms`) — `graph.merge = "none"`: the preview box drops its `m merge` part; nothing draws `—` (extend 10's
      `merge_unbound_is_drawn_nowhere_and_its_menu_row_names_no_key`).
- [x] **Grep test** (`graft-ui/tests/no_literal_keys.rs::no_user_facing_literal_spells_a_default_key`) over graft-ui user-facing string literals (10's Acceptance wording); allow-list with a reason per
      entry, in the test. Test name at tick.
- [x] Captures, default keymap (full `cargo nextest run`; check script in NOTES): every capture assert green. `git diff main -- docs/shots` holds only: decision 3,
      `</>`, `n/N`, `alt+↑/↓`, resize moved or dropped, the new shot. The five width knock-ons in 10's NOTES are gone
      (`u undo`, `/ search`, `drop` back; `m6/fix-tab-words-120x40` back to main's cut or NOTES says why). NOTES lists
      each changed file and its tokens, and the check used (10's NOTES script).
- [x] MAP.md (graft-ui `## keymap` m8/10c entry + hints/blame/history/journal/map/commit_editor/Filters lines; graft: no startup move, so no change) graft-ui (every widget now taking `&Keymap`, `footer_subject`/`footer_other`, `undo_line`, `legend`,
      `again`, R1 in the ladder entry, R2 in `## keymap`) and graft (startup keymap order, if moved).

## Touches
- The ladder (R1): every bar with a divider and its captures; `every_key_the_bar_draws_is_answered`; 15's captures
  (15 is after this) and 11's doorway (same hints files, after this).
- Every widget in Scope 4 and its graft view caller; `preview_box` callers in graft (`again`).
- Startup order (empty state); 18 and 19 touch the empty state after this.
- `keymap_edge` fixtures; contact sheets; MAP.md graft-ui, graft. Journal/undo: none (render only) — show the empty
  diff over `crates/graft/src/update/journal*` and `crates/graft-ops`.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~60k: R1/R2/R4/R5 + tests ~12k, footers ~22k, sweep (c) + grep test ~12k, re-shoot + sheets + MAP ~14k. Order: R1,
R2, R4, R5 first (they fix shipped regressions), then footers, sweep (c), grep test. Seam if over: commit after the
footers, report `blocked: grep test`.

## NOTES (builder, 2026-10-04)
**Compacted once mid-ticket** (context ran out after the footers). Nothing left undone from Scope; see the two
deviations and the five allow-listed literals below. Scout used: `scouts/10-keymap-spell.md` (from 10).

### Deviations
1. **R4 not built as written.** Finish Subject/Body are text fields: `AppState::text_field_open()`
   (`crates/graft/src/update/state.rs:873–887`) passes every key as typed when the finish summary's focus is not
   `Files` — so a remap does not act there, and spelling his remap would name a key that types a letter. Proof in
   tests already on main: `x_typed_into_every_field_is_an_x_with_merge_on_x` (its fields include "finish subject").
   Built instead: the invariant half — `Fixed` only where `is_text_field()` (test above). Answer to R4's question:
   yes, typed text passes in Subject/Body.
2. **Unbound item's premise.** The box's `m merge`/`r rebase` slot is not `graph.merge`/`graph.rebase`: those
   keys confirm through `preview.confirm`'s chords `[y, m, r]` (`crates/graft/src/update/preview/keys.rs:78–88`
   `confirm_key`). So `graph.merge = "none"` leaves the box's `m merge` correct. `preview_box::Again` now draws
   the slot exactly while its chord is a live `preview.confirm` chord; `preview.confirm = ["y"]` drops it, no `—`.
3. **R5 capture is three frames, not one**: the config notice holds the bar's row until the first key
   (`crates/graft/src/view/frame/bars.rs:336`, cleared in `update/keys/acknowledge.rs:33`), so `M merge` and
   `config: 1 problem` are never on one frame. Frames: startup (notice) → `.` menu (`Merge feature into main (M)`)
   → menu closed (`M merge`, no `M map`). Config read through `config::keymap_from` (called by
   `event_loop::startup::Arrival::first`), seeded by `AppState::seed_keys` as at startup. Colour unverified.

### Beyond the listed sites (found by the grep test)
Converted: blame ignore-revs line (`blame.ignore_revs`), resolver block headers and placeholders (`o/t/b`),
finish `⏎ hides`/`⏎ shows the change` (`finish.toggle_diff`), `MESSAGE · ⇥ edit` (`app.next_pane`), journal
row marks `← u`/`← U` (`RowMark::label(keys)`), name prompt `⇥ toggles` (text field → shipped).
**Allow-listed as KNOWN, follow-up needed** (no keymap reaches these widgets; out of 10c's list):
`detail_pane/header.rs` `x clears it`; `detail_pane/line_render.rs` `L load all`; `graph_screen/banner.rs`
(`space stages, a amends it, c continues · ⇥ banner`); `rewrite_editor/right.rs` `⏎ back to AFTER`;
`graph_screen/badges.rs` `x base`. Each entry carries its reason in the test.

### Other text changes (beyond decision 3)
- Map footer `↑↓ select` → `j/k select` (`app.down`/`app.up` first chords; the Map answers both,
  `crates/graft/src/update/map.rs:163–167`). `M/Esc` → `M/esc`.
- Commit-graph written line `Ctrl+R to use it` → `⌃R to use it` (`app.refresh` spelled).
- `focus (F/Esc off)` → `focus (F/esc off)`; journal `n N match` → `n/N match`.
- `journal_screen::JournalView::enter_words` is now the verb alone (the key is spelled in front).

### Ranking findings for 15
At 100 columns `u undo` still drops in these contexts whatever the divider (so ranking, not resize):
`BranchRow { current: false }` with all verbs; `Rewritable`/`Marked`/`PaneFocused` wrapping it;
`BranchRow`/`PlainRow` with take+amend.

### Captures
Re-shot with `GRAFT_UPDATE_SHOTS=1`, 3 passes (third: only `raster::write_shots`, which fails under the update
variable, and the multi-shot rewrite walk; both pass without it). 86 files differ from the merge-base. Check:
`shotcheck10c.py` — per hunk, old line normalised (`Esc→esc`, `↑↓→↑/↓`, `←→→←/→`, `<>→</>`, `n N→n/N`,
`tab→⇥`, `alt+↑↓→alt+↑/↓`, `Ctrl+R→⌃R`, map `↑/↓ select→j/k select`) and padding squashed must equal the new
line, or be R1: `</> resize` dropped/moved last with the freed width redrawing cut hints. 0 unexplained outside
contact sheets and the new shot. R1 gains: `/ search` back in m2-duplicate-commit, m2-evil-merge, m2-force-push,
m5-hints-detail-160x48, m6 fix-c-live-frames, m7 pane-bar, radar-not-checked, diff-pane-b-with-offer, m8
settle-following; `u undo` back in m2-explain-panel, m7 blame-then-commit, esc-back-to-{blame,history},
radar-not-checked-evil. Width knock-ons from 10: gone — `u undo` in m2-duplicate-commit/m2-evil-merge, `/ search`
in m7 diff-cursor-commit and pane-bar, `d drop` in rewrite-editor-100x30. **m6/fix-tab-words-120x40 keeps
`progr…`** (main: `progre…`): there nothing else drops, so `</> resize` stays (last) and its extra cell still
cuts the status lead. Contact sheets m3, m5, m6, m7 re-assembled; their remaining unmatched lines come from
captures this ticket did not change (sheets on main were already stale — 10 noted the m4 sheet's 354 lines).
New shot `docs/shots/m8-keymap-remapped-120x36.txt` (14 KB).

### Touches (how checked)
- Every widget now taking `&Keymap` and its graft caller: compiler; sweep (c) tests above.
- Preview `again` callers (graft `preview/keys.rs`, `boxes.rs`, `preview.rs`, amend_into tests, put_back_box):
  compiler + keymap_edge tests.
- The ladder (R1): `resize_is_drawn_only_when_nothing_else_was_dropped`, `every_key_the_bar_draws_is_answered`,
  graft `hint_bar` and `view::tests::the_resize_key_is_offered_only_where_there_is_a_divider_to_drag` (now asserts
  the divider flag the bar is handed; the 160-col plain-row bar never reaches resize after R1).
- Startup order: empty state draws with the keymap `event_loop/places.rs` already builds before the first draw;
  no move.
- Journal/undo: none — `git diff --stat $(git merge-base HEAD main) -- 'crates/graft/src/update/journal*'
  crates/graft-ops` is empty.
- Captures/sheets: above. MAP.md graft-ui updated; graft unchanged.
