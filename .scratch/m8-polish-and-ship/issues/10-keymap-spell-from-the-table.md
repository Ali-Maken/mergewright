# G-keymap 2 — every hint, label and footer spells keys from the live table

Status: ready (replaced by the planner's spec-only pass 2026-10-04, after scout `scouts/10-keymap-spell.md`)
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
- [ ] **Spell sweep** green (test name at tick): a keymap remapping every movable action to a distinctive free chord
      (09's sweep set); every `every_context()` bar at 40/80/120/200 columns, every `.` menu, every footer/box above:
      (a) each drawn hint's `key_text` equals the remapped spelling — asserted on `Hint`s, not frame text; (b) menu
      suffixes likewise; (c) footers: the rendered line holds the remapped labels and no default label of a moved
      action as a whole token; (d) no chord drawn twice in one bar.
- [ ] Unbound test: `graph.merge = "none"` → the merge hint is absent at every width, the menu row reads `Merge …`
      with no suffix, and no `—` is drawn anywhere (test name at tick).
- [ ] **Grep test** over graft-ui user-facing string literals: no default chord label in a hint position (literal
      start, after the bar's two-space gap, after `· `, or as a trailing `(x)`); allow-list with a reason per entry,
      documented in the test. Test name at tick.
- [ ] `every_key_the_bar_draws_is_answered` and `every_bound_action_is_hinted_live_or_is_named_here` green over
      `ActionId`; `table_check` has no word exemptions.
- [ ] Default keymap: every capture assert green. `git diff --stat main -- docs/shots` = the new shot + only the
      decision-3 normalisations; NOTES lists each changed file and its token(s), and states the check used to show the
      diff holds nothing else (e.g. a word-diff filtered for those tokens).
- [ ] pty capture with `[keys.graph] merge = "M"`: `docs/shots/m8-keymap-remapped-120x36.txt` — bar shows `M merge`,
      `.` menu `Merge … (M)`. If the remap displaces another action, its hint is absent; NOTES names it. **Colour
      unverified** (rule 5). Looks-right is the user's.
- [ ] MAP.md graft-ui (`HintKey`, `Hint`, `NOT_A_BINDING`, `spell_keys`, `spell_every`, every widget that now takes
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
