# G-keymap 2h — undo, journal, recover and status-row words spell keys from his keymap

Status: partial (6/7 boxes; captures box open)
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
- [x] `update::tests::keymap_words::undo_journal::each_word_spells_the_remapped_key` (22 rows; search shipped bytes: `update::tests::keymap_words::undo_journal::the_search_hint_normalises_esc_only`) — one row per site (count
      stated): sweep keymap → remapped label, no default label of a moved action as a whole token; shipped →
      byte-identical (or the W6 normalisation, asserted).
- [x] `…::undo_journal::each_word_names_the_row_that_answers_it` (16 pins) — W2 pin per site (ctx stated); the credential
      row pinned in its text-field context.
- [x] `…::undo_journal::an_unbound_key_drops_its_clause` + `an_unbound_back_drops_its_clause` + `an_unbound_undo_or_help_drops_its_clause` + `no_unbound_key_draws_a_dash` — redo unbound: no `U redoes …` clause; journal unbound:
      no `J journal` / `· J to inspect`; `app.back = "none"`: `esc dismiss` not drawn; no `—`.
- [x] Credential line under the sweep keymap equals the shipped line (W4) — `update::tests::keymap_words::undo_journal::the_credential_keys_are_the_shipped_ones_under_any_keymap`.
- [x] `no_literal_keys::no_user_facing_literal_spells_a_default_key` green (ROWS now empty — Esc has a Credential row, pinned); `known_undo.rs` holds only `text field,
      no row` rows with `file:line`.
- [ ] Captures: shipped keymap → only W6 normalisations (`Esc` → `esc` in search captures); each file and token in
      NOTES; affected contact sheets re-assembled. Colour unverified (rule 5).
- [x] MAP.md graft: changed fn signatures (checked by reading the diff; no test).

## Touches
- Callers (compiler); tests asserting these strings (grep each literal); every capture showing search's hint line.
- `view/status.rs` also draws other tickets' statuses — only the `esc dismiss` literal changes.
- Journal/undo: words only — `undo.rs`, `journal/*`, `recover.rs` logic untouched; show the diff holds only word
  lines (NOTES: the check used).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~55k: ~25 sites ~20k, table + pins ~20k, captures ~5k, MAP/NOTES ~5k.

## NOTES (builder 10h, 2026-10-07)

No compaction. Scout: none — 10f/10g/10j NOTES served as scout; sites read directly.
- **TDD:** red observed as compile failure for every new test (fns missing); plus one mutation red (`dismiss` returning the
  literal → `each_word_spells…` and `an_unbound…` FAIL), reverted, green. Assertion red per test not observed otherwise.
- **Captures box OPEN:** search hint `Esc` → `esc` (W6). Stale files, token `Esc close`/`Esc cancel`:
  `docs/shots/26-search-far.txt` (7 lines), `26-search-typing.txt` (2), `m2-18-git-git-search-enter-160x40-unicode.txt` (1).
  No generator in the repo (no test or script names them); hand-shot pty captures. Not hand-edited (would fake a capture).
  Needs a re-shoot by the conductor or an acceptance walk. Colour unverified (rule 5).
- **Also spelled (scanner missed):** `quit.rs` waits (used `spell` → `—` when unbound; now clauses drop); `undo.rs`
  `so u restores the whole tree`; `operate/failure.rs` dismiss stand-in.
- **W2 contexts:** see doc comment on `each_word_names_the_row_that_answers_it`. Search `n/N` answer translated keys
  (`update/mod.rs` `dispatch_key` → `search::search_key`).
- **Unbound `journal.branch` on a `●` row:** status left unchanged (W3: line is only a key clause).
- **Outside Files (compiler-forced / visibility):** `operate/outcome.rs`, `operate/failure.rs`, `preview/answer.rs`,
  `view/rows.rs`; `mod` lines in `journal.rs`, `preview.rs`, `place.rs`, `view/frame.rs`, `view/mod.rs` → `pub(crate)`;
  tests `recover/tests.rs`, `clash/tests.rs`, `undo/tests/set_aside.rs`, `view/tests/bottom_row/rungs.rs`, `update/tests.rs`
  (pass `Keymap::shipped()`).
- **Touches:** callers via compiler; string-asserting tests — full nextest 5781/5781 green, unchanged expectations;
  journal/undo logic: `git diff` of undo.rs/journal/recover.rs holds only word fns + their call lines (read by eye);
  keymap table untouched; captures as above.
