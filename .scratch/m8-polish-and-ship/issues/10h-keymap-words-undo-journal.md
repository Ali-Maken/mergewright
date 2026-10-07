# G-keymap 2h — undo, journal, recover and status-row words spell keys from his keymap

Status: ready
Build with: **Sonnet** — every site listed, every rule decided (10b W1–W6), each held by a table row. Not Fable.
Blocked by: 10b, 10e (callers' files; test scaffold). **After 09d merges** (`view/status*`, possible `view/frame*`
contact with 09d's widget call sites).
Needs: from 10e — `update/tests/keymap_words.rs` helpers; from 10/10c — `Keymap::{spell_keys, key_verb, after,
translate, shipped}`, `AppState::keymap`.
Crate set: graft + graft-ui `tests/no_literal_keys/known_undo.rs` only.
**Files:** graft `update/undo.rs`, `update/journal/{answers,keys}.rs`, `update/recover.rs`,
`update/amend_into/landing.rs`, `update/clash.rs`, `update/switch.rs`, `update/preview/boxes.rs`, `search.rs`,
`update/place/answer.rs`, `update/quit.rs`, `view/credential.rs`, `view/frame/bars.rs`, `view/status.rs`,
`update/tests/keymap_words/undo_journal.rs`, `MAP.md` (own entries); graft-ui `tests/no_literal_keys/known_undo.rs`.
Plus any path 10b put in `known_undo.rs`. Disjoint from 10f, 10g, 10i except MAP.md.

## Read first
- `10b-keymap-spell-in-words.md` **Ruling** → W1–W6
- `crates/graft/MAP.md` entries for each module in Files; `AppState::text_field_open`
- `crates/graft-ui/MAP.md` `## keymap`; `ContextId::is_text_field`

## Scope
In — each site spelled per W1–W6 (from `10b-grep-hits.txt` and 10b's original list; regenerate from `known_undo.rs`):
- `undo.rs:756`, `⇥ banner`, `U redoes {waiting}`, `J journal`, `· J to inspect`
- `journal/answers.rs` `b makes {name} at {}`; `journal/keys.rs` `⏎ puts it back`, `b makes a branch on a $ or ✗ row`
- `recover.rs` `u undoes`, `u undoes it`; `amend_into/landing.rs` `u undoes`; `switch.rs` `u goes back`;
  `clash.rs:297-298` and `u brings it back`; `preview/boxes.rs` `{name} is dropped · u brings it back`
- `search.rs` `n/N next/prev  Esc cancel|close` (W6: `Esc` → `esc` is a decision-3 change — re-shoot, NOTES)
- `place/answer.rs:181,184` `esc back`; `quit.rs:103`, `esc stays`
- `view/credential.rs` `⏎ send  ⌃U clear  esc cancel` — a text field: W4 (shipped; `⌃U` with no row stays listed
  `text field, no row: <file:line>`)
- `view/frame/bars.rs` `config: {count} {noun} · ? shows them`; `view/status.rs` `esc dismiss`
- every other `KNOWN: 10h` row
- dropped: `undo/consequence.rs:415` (test-only; :81 already spells — 10b NOTES)
Out: prose rows; graft-ops `UndoError` words (10e); 20's item 5 (`undo.rs:645-652` cut) — rewording.

## Signatures
No new public items. Word fns gaining `keys: &Keymap` are named in MAP.md.

## Acceptance (each tick names its test, at tick time)
- [ ] `update::tests::keymap_words::undo_journal::each_word_spells_the_remapped_key` — one row per site (count
      stated): sweep keymap → remapped label, no default label of a moved action as a whole token; shipped →
      byte-identical (or the W6 normalisation, asserted).
- [ ] `…::undo_journal::each_word_names_the_row_that_answers_it` — W2 pin per site (ctx stated); the credential
      row pinned in its text-field context.
- [ ] `…::undo_journal::an_unbound_key_drops_its_clause` — redo unbound: no `U redoes …` clause; journal unbound:
      no `J journal` / `· J to inspect`; `app.back = "none"`: `esc dismiss` not drawn; no `—`.
- [ ] Credential line under the sweep keymap equals the shipped line (W4) — test named.
- [ ] `no_literal_keys::no_user_facing_literal_spells_a_default_key` green; `known_undo.rs` holds only `text field,
      no row` rows with `file:line`.
- [ ] Captures: shipped keymap → only W6 normalisations (`Esc` → `esc` in search captures); each file and token in
      NOTES; affected contact sheets re-assembled. Colour unverified (rule 5).
- [ ] MAP.md graft: changed fn signatures.

## Touches
- Callers (compiler); tests asserting these strings (grep each literal); every capture showing search's hint line.
- `view/status.rs` also draws other tickets' statuses — only the `esc dismiss` literal changes.
- Journal/undo: words only — `undo.rs`, `journal/*`, `recover.rs` logic untouched; show the diff holds only word
  lines (NOTES: the check used).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~55k: ~25 sites ~20k, table + pins ~20k, captures ~5k, MAP/NOTES ~5k.
