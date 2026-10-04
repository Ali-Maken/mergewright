# G-keymap 2 — every hint, label and footer spells keys from the live table

Status: lands with 10c (remaining work done there; see 10c NOTES). Was: partial — blocked: footers (seam taken 2026-10-04). Was: ready (planner's spec-only pass 2026-10-04, after scout `scouts/10-keymap-spell.md`)
Build with: **Opus** — the contract is fixed by 09 and the shape by this spec; a wide sweep (~200 hint calls, ~35
footer sites) held by the spell sweep and the capture diff. Not Fable: fails "a wrong approach becomes structural"
(the shape is decided below).
Blocked by: 06, 07, 09, 09b
Needs: from 09 — `ActionId`, `Keymap::{spell, chords, in_context}`, `KeyLabel`, `defaults()`; from 09b —
`AppState::keymap`, `SharedKeymap` (`update/edge.rs`); `look-chrome-header-gutter-bar` (bar form),
`look-cursor-row-and-dark-b` (same hints files).
Crate set: graft-ui (hints, labels, footers, keymap helpers) + graft (view call sites, `update/context_menu/verbs.rs`,
`update/keys/{bound,table_check}.rs`, `update/edge.rs`, `update/tests`). Not graft's status/refusal words, not
graft-ops: those are **10b**.

## Read first
- `docs/adr/0055-keys-are-one-table-and-remaps-translate-at-the-edge.md` §1 and Consequences
- `docs/design/02-keymap.md` §2.1 bullets "The hint bar is a ranked selection" and "Every binding is remappable"
- `docs/research/ux-principles.md` row 4 (Consistency) — the reason for the spelling normalisation below
- `.scratch/m8-polish-and-ship/scouts/10-keymap-spell.md` (whole) — the site list
- `crates/graft-ui/MAP.md`: `## keymap`; `graph_screen::hints` (`Hint`, `hints_for`, `short_form`, `every_hint`,
  `stopped_row`, `stopped_row_before_keys`, `every_context`, coverage slots); `finish_summary` (`footer_spans`,
  `footer_hints`); `commit_editor` (`FOOTER_SUBJECT`, `FOOTER_OTHER`); `context_menu`, `confirm`, `preview_box`,
  `picker`, `stash_screen`, `journal_screen`, `map_screen`, `empty_state`
- `crates/graft/MAP.md`: `update::edge` (`SharedKeymap`), `update::keys::bound_actions`, `table_check`,
  `update::tests` (`every_key_the_bar_draws_is_answered`, `every_bound_action_is_hinted_live_or_is_named_here`,
  `NEVER_HINTED`), `update::context_menu::verbs`

## Decisions (spec-only pass; not the builder's)
1. **A hint carries a list of keys, not one action.** ~20 hints draw two keys (`j/k`, `n/N`, `⌃O/⌃T`, `⏎/space`).
   `HintKey::Key(id)` spells the action's first chord; `HintKey::EveryChord(id)` spells all its chords (one action
   with two defaults, e.g. `⏎/space` if the table holds them as one row — the builder reads `defaults()` per hint);
   `HintKey::Word(w)` is a word that is not a binding (below). Keys join with `/`.
2. **An unbound key is not drawn.** A `Key`/`EveryChord` whose action has no chord is left out of its hint; a hint
   with no key left is dropped from the bar (a hint naming a key that does nothing is a lie). `.` menu items keep
   their row (they act by `⏎`/click) and lose the `(key)` suffix. Never `—` on screen.
3. **One spelling, one joiner** (ux-principles row 4). The table's `KeyLabel` wins over today's literal: `Esc` →
   `esc`, `tab` → `⇥` where Tab is a table binding, `↑↓`/`←→` → `↑/↓`/`←/→`. These are the **only** capture changes
   allowed; NOTES lists each capture and the token that changed.
4. **Words that are not bindings** (09's exemptions `release`, `type`, `tab`): `HintKey::Word` only, each from one
   `NOT_A_BINDING: &[(&str, &str /* why */)]` list in graft-ui. `type` = typed text in a field (never remapped,
   ADR-0055); `release` = the end of a mouse drag; `tab` stays a word only where it is not a table binding. A word
   that turns out to be a binding spells from the table instead (and NOTES says so). `table_check.rs`'s exemption
   list is deleted: the words are no longer key strings.
5. **Fixed contexts spell from the table too** (`Picker`, `CommitEditor`: `is_text_field`). Remaps can't move them,
   but one path means the grep test has no exceptions to argue over.
6. **`Keymap` derives `Clone, Debug, PartialEq, Eq`.** graft keeps an `Arc` (AppState clones stay cheap) but holds
   `Arc<Keymap>` directly; `SharedKeymap` and its hand-written impls are deleted.
7. **`bound_actions` and `NEVER_HINTED` become `ActionId`.** `table_check` asserts each bound action is a row of
   `defaults()`.
8. Spelling happens at render: graft-ui widgets that print keys take `&Keymap` (not pre-spelled strings in their
   views). Docs (README, man, `02-keymap.md`) are **out**: `02-keymap.md` documents defaults, which stay true; the
   man page is generated from `defaults()` in 23.

## Scope
In:
- `Hint` shape (decision 1); every hint table (`hints/lists.rs`, `lists/resolver.rs`, `hints.rs` incl. `esc` :334,
  `BACK_TO_BLAME` :539, `BACK_TO_HISTORY` :543, `fitting.rs`, `catalogue.rs`) over `HintKey`. `hints_for`,
  `short_form`, `stopped_row`, `stopped_row_before_keys`, `footer_spans`, `footer_hints` take `&Keymap` where they
  spell or measure keys (width depends on the spelled label).
- `.` menu `(key)` suffixes: graft-ui `context_menu.rs:105-108,192,195,197,220`; graft
  `update/context_menu/verbs.rs:161,163,249,258` (Cherry-pick/Merge/Rebase).
- graft-ui footers and boxes (scout list): `frame_box.rs:307,317`; `preview_box.rs:42,46,85-89,99`;
  `stash_screen.rs:25,181,184,186`; `commit_editor.rs:172,176,227` (`FOOTER_*` consts become fns of `&Keymap`;
  `⌃⏎/Commit` keeps `Commit` — the button's name, a `Word`); `picker.rs:257,259`; `journal_screen.rs:46,221`;
  `finish_summary.rs:182`, `finish_summary/list.rs:99`; `blame.rs:242,272`; `history.rs:347`;
  `conflict_resolver.rs:105,107`; `empty_state.rs:99,101`; `map_screen/render.rs:131`; `graph_screen/mod.rs:310`.
- graft view call sites pass `&state.keymap`. `Keymap` derives; `SharedKeymap` deleted (decision 6).
- `bound_actions`, `NEVER_HINTED`, `table_check` over `ActionId` (decision 7).
- **Spell sweep test** (ADR-0055 consequence, second half), in graft-ui.
- **Grep test** over graft-ui only (10b extends it to graft and graft-ops).
Out:
- graft status/refusal words and graft-ops words (10b). `?` and the palette, doorway `. more` → `? keys` (11). Which
  hints earn a slot (15). Docs (decision 8). `operation.rs:920` ("press the key again") names no key — unchanged.

## Signatures
```rust
// graft-ui::graph_screen::hints
pub enum HintKey { Key(ActionId), EveryChord(ActionId), Word(&'static str) }
pub struct Hint { pub keys: &'static [HintKey], pub verb: &'static str, pub state: HintState, pub rung: Rung,
    pub keeps_words: bool }
impl Hint {
    pub fn key_text(&self, keys: &Keymap) -> Option<String>;   // "/"-joined; None when every key is unbound
    pub fn text(&self, keys: &Keymap) -> Option<String>;       // "<key_text> <verb>"
}
pub fn hints_for(context: &HintContext, divider: Divider, columns: u16, keys: &Keymap) -> Vec<Hint>; // drops None
pub const NOT_A_BINDING: &[(&str, &str)];                       // (word, why) — decision 4

// graft-ui::keymap
#[derive(Clone, Debug, PartialEq, Eq)] pub struct Keymap { /* unchanged */ }
impl Keymap {
    pub fn spell_keys(&self, actions: &[ActionId]) -> Option<String>;   // first chord each, unbound left out, "/"
    pub fn spell_every(&self, action: &ActionId) -> Option<String>;     // every chord, "/"; None when unbound
}

// graft::update::keys
pub(crate) fn bound_actions() -> &'static [ActionId];
pub(crate) const NEVER_HINTED: &[(ActionId, &str /* why */)];
// graft::AppState: `keymap: Arc<Keymap>` (SharedKeymap gone)
```
`Hint::key` (the old string) is removed. Builder names other helpers; MAP.md records them.

## Acceptance
- [~] **Spell sweep** green — (a),(d) `graph_screen::hints::spell_sweep::every_bar_spells_the_remapped_chords`; (b) `…::every_menu_suffix_spells_the_remapped_chord`; (c) footers **not built** (seam). (test name at tick): a keymap remapping every movable action to a distinctive free chord
      (09's sweep set); every `every_context()` bar at 40/80/120/200 columns, every `.` menu, every footer/box above:
      (a) each drawn hint's `key_text` equals the remapped spelling — asserted on `Hint`s, not frame text; (b) menu
      suffixes likewise; (c) footers: the rendered line holds the remapped labels and no default label of a moved
      action as a whole token; (d) no chord drawn twice in one bar.
- [x] Unbound test (`update::tests::keymap_edge::merge_unbound_is_drawn_nowhere_and_its_menu_row_names_no_key`; "no `—`" asserted as: each context fixture draws exactly as many `—` as under the default keymap, since ghost rows draw `—` for sha/age by design): `graph.merge = "none"` → the merge hint is absent at every width, the menu row reads `Merge …`
      with no suffix, and no `—` is drawn anywhere (test name at tick).
- [ ] **Grep test** — **not built** (seam; it would fail on the unconverted footers). over graft-ui user-facing string literals: no default chord label in a hint position (literal
      start, after the bar's two-space gap, after `· `, or as a trailing `(x)`); allow-list with a reason per entry,
      documented in the test. Test name at tick.
- [x] `every_key_the_bar_draws_is_answered` and `every_bound_action_is_hinted_live_or_is_named_here` green over
      `ActionId` (`update::tests::every_key_the_bar_draws_is_answered`, `…::every_bound_action_is_hinted_live_or_is_named_here`, `…::every_planned_hint_is_still_unbound`); `table_check` has no word exemptions (`update::keys::table_check::every_bound_action_is_a_row`, `…::no_action_is_listed_twice`).
- [~] Default keymap: every capture assert green (full `cargo nextest run`). Diff = the normalisations **plus width knock-ons** (NOTES, question 1), so not strictly "only". `git diff --stat main -- docs/shots` = the new shot + only the
      decision-3 normalisations; NOTES lists each changed file and its token(s), and states the check used to show the
      diff holds nothing else (e.g. a word-diff filtered for those tokens).
- [~] Capture (in-process draw, **not pty** — deviation 4) `update::tests::keymap_edge::capture_merge_remapped_to_shift_m`, with `[keys.graph] merge = "M"`: `docs/shots/m8-keymap-remapped-120x36.txt` — bar shows `M merge`,
      `.` menu `Merge … (M)`. If the remap displaces another action, its hint is absent; NOTES names it. **Colour
      unverified** (rule 5). Looks-right is the user's.
- [x] MAP.md graft-ui (for what is built; footer widgets other than `PreviewBoxWidget`/`StashScreen` still to come) (`HintKey`, `Hint`, `NOT_A_BINDING`, `spell_keys`, `spell_every`, every widget that now takes
      `&Keymap`) and graft (`AppState::keymap`, `bound_actions`, `NEVER_HINTED`; `SharedKeymap` removed).

## Touches
- Every hint table and every caller of `hints_for`/`Hint` (bar, `stopped_row*`, finish border, coverage slots,
  `every_hint`); `context_menu` + graft verbs; every widget above and its graft view caller.
- `AppState` derive chain (`keymap` field type), config `keymap_from` caller (16 shares `config/keys.rs` — 16 is after
  this), `update::tests::keymap_edge` fixtures.
- Captures (decision 3 only). MAP.md graft-ui, graft. Journal/undo: none (render only) — say how checked.
- 11 (palette) and 15 (bar) build on `HintKey`; 19's card uses `spell_keys`.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~90k: shape + helpers ~10k, hint tables ~35k (mechanical: key string → `ActionId` by context), footers/menus ~20k,
tests + capture ~25k. Seam if over: land shape, hint tables, bar, menus, `bound_actions` and the sweep over bars and
menus; commit; report `blocked: footers` — the conductor gives the graft-ui footer/box sites to an Opus follow-up.

## NOTES (builder, 2026-10-04)

**Compacted twice** in this session. Seam taken: shape, hint tables, bar, menus, `bound_actions` and the
sweep over bars and menus landed, **plus** the preview box and stash footers. Report: `blocked: footers`.
Scout used: `scouts/10-keymap-spell.md`.

### Remaining for the follow-up (exact next steps)
Pattern (as `preview_box.rs` / `stash_screen.rs`, commit 7378eebd): the widget gets `pub keys: &'a Keymap`; its
graft caller passes `&state.keymap`; footers use `keys.key_verb(&[ActionId(..)], verb)` / `keys.after(words, id,
verb)`; parts joined by two spaces, an unbound one left out. Text fields (`CommitEditor`, `Picker`) spell from
`Keymap::shipped()` (decision 5 + ADR-0055: rows fixed). Sites still literal:
- `commit_editor.rs:172,176` `FOOTER_SUBJECT`/`FOOTER_OTHER` → fns (`⌃⏎/Commit`: `Commit` is a `Word`); `:227`.
- `picker.rs:257,259` (`esc closes`); `draw_picker` needs keys.
- `journal_screen.rs:46,221` (`u takes it back`, legend `u can undo it`, footer `J/esc close`, `b new branch here`,
  `/ find`, `n N match` → `n/N`, `space command log`); `journal_screen/tests.rs:418`.
- `finish_summary.rs:182`, `finish_summary/list.rs:99`, `finish_summary/frame.rs` `preview_line`/`UNDO_LINE`.
- `blame.rs:242,272`; `history.rs:347`; `conflict_resolver.rs:105,107` (ASCII variant uses ` - `);
  `empty_state.rs:99,101` (find where it gets a keymap: no `AppState`); `map_screen/render.rs:131`
  (`↑↓ select  ⏎ zoom to commits  M/Esc back` → `app.down/up`, `map.seek`, `map.close`+`app.back`);
  `graph_screen/mod.rs:~310` (`focus (F/Esc off)`); commit-graph line `B`/`X` (`graph.write_commit_graph`,
  `graph.decline_commit_graph`); `frame_box.rs:307,317`; check `rewrite_editor/tests.rs:389` (`<> moved`).
- `preview_box`: the `again` part (`m merge`/`r rebase`) still arrives **spelled by graft** as `(SanitizedText,
  SanitizedText)` — move it to an `ActionId`.
- Then: spell sweep (c) over every footer; the grep test; re-shoot (`GRAFT_UPDATE_SHOTS=1` until stable —
  multi-capture walks stop at the first stale file, so it takes 2–3 passes); `scripts/contact-sheet.sh` m3–m7
  (m6/m7 `--captions-order`); MAP.md for each widget.
- Capture check used here: pair `-`/`+` lines per hunk, apply the allowed normalisations, squash padding runs of
  spaces/`─`, classify whole ` · ` clauses dropped; anything else is unexplained and read by eye.

### Deviations (for the reviewer)
1. **`HintKey::Fixed(ActionId)`** — fourth variant: a text field's key, spelled from `Keymap::shipped()` whatever
   his config says (decision 5 says "spell from the table"; a remap can't move them, ADR-0055, so spelling them
   from his map would draw a key that does not work there).
2. **Joiner normalisations beyond decision 3**: `<>` → `</>` (resize), `n N` → `n/N`, `alt+↑↓` → `alt+↑/alt+↓`
   (each key spelled whole by `KeyLabel`). Follow from "one joiner" but are not listed in decision 3.
3. **Menus are spelled at construction** (`ContextMenu::for_row(.., keys)`, `with_key`), not at render
   (decision 8). Harmless today (the keymap is fixed after startup), but `keymap_edge`'s sweep had to account for
   it: its baseline now presses with a keymap holding each sweep chord **ahead of** the default (`both()`), so the
   menu suffixes it builds match the remapped run's.
4. **The remapped capture is an in-process draw**, not a pty run: `capture_merge_remapped_to_shift_m` builds the
   keymap through `config::keymap_from` from a `[keys.graph] merge = "M"` config and draws bar + `.` menu at
   120x36 → `docs/shots/m8-keymap-remapped-120x36.txt`. **Displaced:** `graph.map` (its `M`) — no `M map` hint is
   drawn anywhere (asserted). **Colour unverified** (rule 5). Looks-right is the user's.
5. `NEVER_HINTED` no longer lists `esc dismiss`: `app.back` is drawn live.
6. Test-only `crate::spelled::Spelled` (`hint.key()`, default spelling) keeps old test literals readable.

### Captures (68 files; every changed line checked)
Check: per hunk, `-`/`+` lines paired; each old line with the normalisations applied and padding squashed must
equal the new line. 0 unexplained except the width cases below (classified by the script, each read by eye).
- `←/→` (preview box footer) — 29 files: m5-ui-preview-* (8), m6 fix-preview-verb*, set-aside-*, stash-apply/pop,
  m7 absorb/amend-into/new-branch/put-back*/rebase-*/refused-box/take, m8 fork-follows-preview, reword-preview.
- `</>` (resize) — 29 files: m2 ×4, m3-changes-pane ×2, m5-hints-* ×5, m5-lane-cap, m6 fix-c-*, fix-tab-words,
  stash-clash-take-esc-pane, m7 blame-*/diff-*/esc-back-*/pane-bar/radar-*/submodule-*, m8 settle-following.
- `alt+↑/alt+↓` — 6: m7 rebase-keep-drops-editor, rewrite-editor-{100x30,140x40,187x50},
  rewrite-editor-left-behind, m8 fork-follows-rewrite-editor. `↑/↓` — m7 picker-empty-query, picker-filtered.
  `n/N` — m7 blame-find. `⇥` — m3-stash-save (`tab scope`), m7 rewrite-editor-100x30 (`tab after`).
- **Width knock-ons (not in the allowed list — question 1):** `</>` is one cell wider, so on full bars the ladder
  drops a clause: `u undo` in m2-duplicate-commit, m2-evil-merge (100 cols); `/ search` in
  m7/diff-cursor-commit-120x40, m7/pane-bar-file-vs-directory-120x40; a status cut one cell earlier in
  m6/fix-tab-words-120x40 (`progre…` → `progr…`); m7/rewrite-editor-100x30 loses `drop` after `d`.
- Contact sheets m3–m7 re-assembled. They also pick up earlier tickets' unassembled capture changes (the m4 sheet:
  354 lines from m4-* captures this ticket did not touch).

### Questions
1. `</>` and `alt+↑/alt+↓` cost cells and drop `/ search` / `u undo` / `drop` at 100–120 cols. Keep the strict
   joiner, or allow `<>`/`alt+↑↓` as a compact spelling of a pair? (ux-principles row 4 vs. the bar's width.)
2. Finish Subject/Body keys spelled `Fixed` (text fields) — confirm.

### Touches (how checked)
- Every `hints_for`/`Hint` caller: compiler (signature change) + `every_key_the_bar_draws_is_answered`.
- `.` menu + graft verbs: `context_menu::tests`, `verbs::tests` (`every_label_the_app_can_build`), spell sweep (b).
- `AppState` derive chain: `SharedKeymap` deleted, `Arc<Keymap>`; `keymap_edge` sweep green.
- Captures: 68 re-shot, script-checked above; sheets re-assembled.
- Journal/undo: none — render only; diff against main over `crates/graft/src/update/journal*` and `crates/graft-ops`
  is empty.
- graft-ops / 10b words: untouched.

## Rulings after partial (planner, spec-only pass 2026-10-04)
The builder took the seam (compacted twice; NOTES and questions on branch `worktree-agent-a3081d1d2a2f421de`). The
remainder, with the fixes below, is **`10c-keymap-spell-footers.md`**, built on the same branch; 10 lands with it.
The NOTES and ticks on the branch stand; on merge keep both them and this section.

**R1 — the bar's width (Q1). Cause: rank, not spelling.** `</> resize` is "inserted fourth when `Divider::Shown` (so
the six-hint tier reaches it)" (graft-ui MAP, `graph_screen::hints`). The six-hint tier is gone ("No width tiers",
same entry), so the reason is gone, and a layout key outranks `u undo` and `/ search`: one cell of resize's **own**
new spelling (`<>` → `</>`) pushed undo off at 100 columns. Ruling: **`</> resize` is the last of the bar's own keys**
(after `TAIL`; the doorway keeps its place). It drops first; nothing else pays for its width.
Why: 02-keymap §2.1 "ranks by relevance at the cursor"; ux-principles §3.5 (Graft skips confirmations *because*
undo exists) + row 6 (recognition over recall) + row 3 ("undo exists") → undo must be on screen where he works;
row 8 (every glyph earns its cell) → a layout key earns its cell last. **Rejected as the fix:** a compact `<>`
alone — symptom; the next one-cell change drops undo again. The strict joiner stays: `</>`; `n/N` (same width as
`n N`, costs nothing).

**R2 — a shared modifier word is written once.** `spell_keys`/`spell_every`: a label that repeats the previous
label's `<word>+` prefix drops it: `alt+↑/alt+↓` → **`alt+↑/↓`** (7 cells; main's `alt+↑↓` was 6). Only `+`-joined
words; glyph modifiers keep theirs (`⌃O/⌃T` unchanged — the resolver's spelling since m6). Still one joiner (row 4);
the repeated `alt+` earns nothing (row 8). With decision 3's `tab` → `⇥` (−2), the rewrite editor's narrow bar is
one cell narrower than main's: `d drop` comes back at 100x30. Deviation 2 stands as amended here.

**R3 — undo visible is a test, not a hope.** Invariant: no bar draws `</> resize` while dropping any other hint its
list holds. Named cases: `u undo` in the contexts of `m2-duplicate-commit` and `m2-evil-merge` at 100 columns;
`/ search` in those of `m7/diff-cursor-commit-120x40` and `m7/pane-bar-file-vs-directory-120x40` at 120; `d drop` in
`RewriteEditor { narrow: true, .. }` at 100. A context that still drops `u undo` at 100 after R1 is a ranking
finding for 15: NOTES names context and width; not fixed here.

**R4 — `HintKey::Fixed` (Q2; deviation 1).** Fixed is right **iff** the action's context `is_text_field()`
(graft-ui MAP `ContextId::is_text_field`: Picker, Palette, CommitEditor, Credential). The finish summary is
`Finish`, not a text field, and `context_of` hands its keys to Finish (graft MAP `update::edge::context_of`), so his
remap is what acts there: its Subject/Body hints are `Key(<Finish-context id>)`, **not** `Fixed`. A key its bar
draws with no Finish-context row (e.g. `⌃E`, `⇥`): NOTES names it; rows are not added here (the table is 09c's).
Test: every `Fixed(id)` in every list names a text-field context.
**Question to the conductor (no line, rule 7):** does translate pass typed text in the finish summary's Subject and
Body? If not, a letter he remaps in `[keys.finish]` (or `confirm.yes` after 09c) acts while he types — ADR-0055
"typed text in a field is never remapped". 09c's to check.

**R5 — `M` displacing `graph.map` (Q3): not a defect.** `M` is Map's default (02-keymap §2.3 `M` | Toggle Map view;
§2.3a `M` | Open or close the Map view). A user binding onto it is a collision, "warned at startup, never fatal: the
status row reads `config: N problems · ? shows them`" (02-keymap §2.1 "Every binding is remappable"); the lost chord
is not spelled (`Keymap::live_labels`, graft-ui MAP `## keymap`). So `M map` absent is right. It would be a defect
only if the warning were missing — 10c proves it isn't: the capture becomes the **whole frame** (status row
included), config through the startup path, plus a test that the warning is a `Collision` keeping `graph.merge` and
losing `graph.map`. The example stays: a colliding remap shows him its cost. Deviation 4 (in-process draw, not pty)
accepted on those terms.

**R6 — blockers.** 10 lands only with 10c (same branch). 09c, 10b, 11, 15, 16, 19: `Blocked by: 10` → **10c**.
