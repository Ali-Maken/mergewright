# G-keymap 2c — the footers and boxes spell from the table; the bar keeps undo

Status: draft
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
- [ ] **R1** — `resize_is_drawn_only_when_nothing_else_was_dropped` (or named at tick): every `every_context()` bar,
      every width 40..=240, both dividers: `</> resize` drawn ⇒ every other hint of the list drawn.
- [ ] **R3** — named tests: `u undo` drawn at 100 in the contexts of `m2-duplicate-commit`, `m2-evil-merge`;
      `/ search` at 120 in those of `m7/diff-cursor-commit-120x40`, `m7/pane-bar-file-vs-directory-120x40`; `d drop`
      in `RewriteEditor { narrow: true, .. }` at 100. Any context still dropping `u undo` at 100: NOTES, for 15.
- [ ] **R2** — `spell_keys` of the rewrite move pair is `alt+↑/↓`; `⌃O/⌃T` unchanged (test named).
- [ ] **R4** — every `HintKey::Fixed(id)` in every list names a context whose `is_text_field()` (test named); the
      finish Subject/Body hints follow a remap of a Finish row in the spell sweep.
- [ ] **R5** — test: `[keys.graph] merge = "M"` yields one `KeyWarning::Collision` keeping `graph.merge`, losing
      `graph.map`; capture `docs/shots/m8-keymap-remapped-120x36.txt` shows `M merge` on the bar, `Merge … (M)` in
      the `.` menu, no `M map`, and the status row `config: 1 problem · ? shows them`. **Colour unverified**
      (rule 5). Looks-right is the user's.
- [ ] **Spell sweep (c)** — every footer/box in Scope 4–5 under 10's sweep keymap: the rendered line holds the
      remapped labels and no default label of a moved action as a whole token; text-field footers hold the shipped
      labels whatever the sweep keymap says. Test name at tick.
- [ ] Unbound — `graph.merge = "none"`: the preview box drops its `m merge` part; nothing draws `—` (extend 10's
      `merge_unbound_is_drawn_nowhere_and_its_menu_row_names_no_key`).
- [ ] **Grep test** over graft-ui user-facing string literals (10's Acceptance wording); allow-list with a reason per
      entry, in the test. Test name at tick.
- [ ] Captures, default keymap: every capture assert green. `git diff main -- docs/shots` holds only: decision 3,
      `</>`, `n/N`, `alt+↑/↓`, resize moved or dropped, the new shot. The five width knock-ons in 10's NOTES are gone
      (`u undo`, `/ search`, `drop` back; `m6/fix-tab-words-120x40` back to main's cut or NOTES says why). NOTES lists
      each changed file and its tokens, and the check used (10's NOTES script).
- [ ] MAP.md graft-ui (every widget now taking `&Keymap`, `footer_subject`/`footer_other`, `undo_line`, `legend`,
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
