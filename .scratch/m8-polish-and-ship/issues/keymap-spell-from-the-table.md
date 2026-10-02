# G-keymap 2 — every surface spells keys from the live table

Status: draft
Build with: _(blank — brownfield, ADR-0035.)_ Planner expects **Opus**: the contract is fixed by `keymap-table-and-
translate`; this is a sweep across hint tables and labels, held by a test.
Needs: `keymap-table-and-translate` (`Keymap::spell`, `ActionId`, `defaults()`), `look-chrome-header-gutter-bar`
(bar form), `look-cursor-row-and-dark-b` (same hints files).

## Read first
- `docs/adr/0055-keys-are-one-table-and-remaps-translate-at-the-edge.md` §1 and Consequences
- `docs/design/02-keymap.md` §2.1 bullets "The hint bar is a ranked selection" and "Every binding is remappable"
- `docs/design-notes/m8-polish-and-ship.md`: Interfaces `graft-ui::keymap`
- `crates/graft-ui/MAP.md`: `graph_screen::hints` (`Hint`, `hints_for`, `short_form`, `every_hint`), `context_menu`
  (labels carrying `(key)`), `confirm`, `preview_box` footer, `name_prompt`, `picker`, `stash_screen` footer
- `crates/graft/MAP.md`: `update::keys::bound_actions`, `update::tests` (`every_key_the_bar_draws_is_answered`,
  `every_bound_action_is_hinted_live_or_is_named_here`, `NEVER_HINTED`)

## Scope
In:
- `Hint` carries an `ActionId`; its drawn key comes from `Keymap::spell`. `hints_for` takes `&Keymap`.
- `.` menu labels' trailing `(key)`, confirm/preview/name-prompt/stash/picker/banner footers and any refusal or status
  words that name a key (`p pull first`, `c commit  S stash`) spell through the table. A string literal naming a key
  in a user-facing line is the defect this ticket removes; the scout lists them.
- `bound_actions` and `NEVER_HINTED` move from key strings to `ActionId`.
- **Spell sweep test** (ADR-0055 consequence, second half): remap every action to a free chord, draw every
  `every_context()` bar at 40/80/120/200 columns, every menu and box: no default chord of a remapped action is drawn;
  each drawn key is the remapped one.
Out:
- `?` and the palette (G-palette). The doorway change `. more` → `? keys` (G-palette). Which hints earn a slot (G-bar).

## Signatures
```rust
// graft-ui::graph_screen::hints
pub struct Hint { pub action: ActionId, pub verb: &'static str, pub state: HintState, pub rung: Rung,
    pub keeps_words: bool }
pub fn hints_for(context: &HintContext, divider: Divider, columns: u16, keys: &Keymap) -> Vec<Hint>;
impl Hint { pub fn text(&self, keys: &Keymap) -> String; }
```
`Hint::key` (the old string) is removed. Exact helper names beyond these are the builder's; MAP.md records them.

## Acceptance
- [ ] Spell sweep green (test name at tick); default keymap → every capture byte-identical to before (the existing
      capture asserts — none re-blessed; if one must change, NOTES says why).
- [ ] A grep test over graft-ui and graft user-facing string tables finds no hard-coded key in a hint, label or footer
      (allow-list for text-field help, documented in the test).
- [ ] `every_key_the_bar_draws_is_answered` and `every_bound_action_is_hinted_live_or_is_named_here` green, now over
      `ActionId`.
- [ ] One pty capture with `[keys.graph] merge = "M"`: `docs/shots/m8-keymap-remapped-120x36.txt` — bar shows `M merge`,
      `.` menu `Merge … (M)`. **Colour unverified** (rule 5).

## Touches
- Every hint table (`hints/lists.rs`, `lists/resolver.rs`), `catalogue.rs`, `context_menu` labels, `confirm`,
  `preview_box`, `name_prompt`, `picker`, `stash_screen`, resolver header keys, banner words, refusal words in graft
  `update` and graft-ops words that name keys (scout lists; graft-ops words take the label as a parameter, ops never
  imports ui).
- Tests: hint sweeps, `coverage_slot`, captures (unchanged under defaults). MAP.md graft-ui, graft.

## Scout questions
- Every user-facing string literal that names a key, by crate (file:line). Which are in graft-ops (and so need the
  label passed in)?
- Does any surface build key text by concatenation that a table lookup can't reach (e.g. `⌃O/⌃T all`)?

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~70k tokens. If the scout lists more than ~60 literal sites, the conductor splits graft-ops/graft words
into a Sonnet follow-up.
