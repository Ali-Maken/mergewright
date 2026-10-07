# G-keymap 2i — preview, remote and rewrite-editor words spell keys from his keymap

Status: ready
Build with: **Sonnet** — every site listed, every rule decided (10b W1–W6), each held by a table row. Not Fable.
Blocked by: 10b, 10e (callers' files; test scaffold). **After 09d merges** (`view/rewrite_editor*` holds 09d's
`rewrite_editor::right` call site).
Needs: from 10e — `update/tests/keymap_words.rs` helpers; from 10/10c — `Keymap::{spell_keys, key_verb, after,
translate}`, `AppState::keymap`.
Crate set: graft + graft-ui `tests/no_literal_keys/known_preview.rs` only.
**Files:** graft `update/preview.rs`, `update/preview/{history,line_rows,take,answer}.rs`, `update/remote.rs`,
`update/remote/push_box.rs`, `update/rewrite_editor/{open,words}.rs`, `view/rewrite_editor.rs`,
`update/tests/keymap_words/preview_remote.rs`, `MAP.md` (own entries); graft-ui
`tests/no_literal_keys/known_preview.rs`. Plus any path 10b put in `known_preview.rs`. **Not** `preview/boxes.rs`
(10h). Disjoint from 10f, 10g, 10h except MAP.md.

## Read first
- `10b-keymap-spell-in-words.md` **Ruling** → W1–W6
- `crates/graft/MAP.md` entries for each module in Files; `take_key`
- `crates/graft-ui/MAP.md` `## keymap`; `preview_box` (the `again` slot 10c moved to `ActionId`)

## Scope
In — each site spelled per W1–W6 (from `10b-grep-hits.txt` and 10b's original list; regenerate from `known_preview.rs`):
- `preview.rs:130`; `preview/answer.rs:338`; `preview/history.rs` `a stash isn't a branch · ⏎ opens it`
- `preview/line_rows.rs:228`, `· R to keep them`, `P will need force after`
- `preview/take.rs:83`, `o takes a commit · this row isn't one`, `o is waiting for the refs`
- `remote.rs:285,349` (incl. `c commit  S stash`, `p pull first`), `S s stashes them first` (W5), `m merge it
  r rebase onto it`, `· esc cancels`; `remote/push_box.rs` `f to fetch first`
- `rewrite_editor/open.rs` `R opens it`, `r then R opens it` (W5); `rewrite_editor/words.rs:14`, `p keeps a commit`,
  `r there changes the message`; `view/rewrite_editor.rs` `p keeps it`, `r on "{subject}" changes it` (W4 if drawn
  while the `r` field is open — 09d D5's rule)
- every other `KNOWN: 10i` row
Out: prose rows (`1 commit`, `1 new commit`); graft-ops remote words (10e); 20's items 10, 15, 16 (rewording).

## Signatures
No new public items. Word fns gaining `keys: &Keymap` are named in MAP.md.

## Acceptance (each tick names its test, at tick time)
- [ ] `update::tests::keymap_words::preview_remote::each_word_spells_the_remapped_key` — one row per site (count
      stated): sweep keymap → remapped label, no default label of a moved action as a whole token; shipped →
      byte-identical.
- [ ] `…::preview_remote::each_word_names_the_row_that_answers_it` — W2 pin per site (ctx stated).
- [ ] `…::preview_remote::an_unbound_key_drops_its_clause` — `c commit  S stash` with stash `none` → `c commit`;
      `p pull first` with pull `none` → clause gone; `m merge it  r rebase onto it` with merge `none` →
      `r rebase onto it`; no `—`.
- [ ] `no_literal_keys::no_user_facing_literal_spells_a_default_key` green; `known_preview.rs` holds no `KNOWN` row.
- [ ] Captures: shipped keymap → `git diff --stat main -- docs/shots` empty, or only W6 normalisations in NOTES.
- [ ] MAP.md graft: changed fn signatures.

## Touches
- Callers (compiler); tests asserting these strings (grep each literal); preview captures m5–m8 (shipped: none
  expected).
- The diverged prompt (`m merge it  r rebase onto it`) and the preview box's `again` slot name the same rows: same
  spelling under the sweep keymap (one row in the table).
- Journal/undo: words only — `git diff main -- crates/graft/src/update/journal* crates/graft-ops` empty.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~50k: ~23 sites ~18k, table + pins ~20k, MAP/NOTES ~5k.
