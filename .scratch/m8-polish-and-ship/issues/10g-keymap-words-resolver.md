# G-keymap 2g — conflict resolver words spell keys from his keymap

Status: ready
Build with: **Sonnet** — every site listed, every rule decided (10b W1–W6), each held by a table row. Not Fable.
Blocked by: 10b, 10e (callers' files; test scaffold).
Needs: from 10e — `update/tests/keymap_words.rs` helpers; from 10/10c — `Keymap::{spell_keys, key_verb, after,
translate}`, `AppState::keymap`.
Crate set: graft + graft-ui `tests/no_literal_keys/known_resolver.rs` only.
**Files:** graft `update/resolver.rs`, `update/resolver/{answers.rs,answers/taken.rs,done.rs,open.rs,resolved.rs,
take.rs,whole_file.rs}`, `view/resolver/whole_file.rs`, `update/tests/keymap_words/resolver.rs`, `MAP.md` (own
entries); graft-ui `tests/no_literal_keys/known_resolver.rs`. Plus any path 10b put in `known_resolver.rs`.
Disjoint from 10f, 10h, 10i except MAP.md.

## Read first
- `10b-keymap-spell-in-words.md` **Ruling** → W1–W6
- `crates/graft/MAP.md` resolver entries (`update::resolver::*`, `view::resolver`)
- `crates/graft-ui/MAP.md` `## keymap`; `conflict_resolver` (the footer 10c spelled — same rows, one spelling)

## Scope
In — each site spelled per W1–W6 (from `10b-grep-hits.txt`; regenerate from `known_resolver.rs`):
- `resolver.rs:399` (10b's original list)
- `answers/taken.rs` `· ⏎ marks {} done`; `answers.rs` `⏎ marks it resolved`, `e edit  M mergetool`
- `done.rs` `d starts it over`; `view/resolver/whole_file.rs` `d starts it over`, `e opens the whole file`
- `open.rs` `esc closes it`, `esc stops it`
- `resolved.rs` `n next`, `⏎ marks done`; `take.rs` `n next`, `⏎ marks {} done`
- `whole_file.rs` ×7 `o keeps …  t …` / `o or t removes it` (W3: `o or t` keeps the bound one)
- every other `KNOWN: 10g` row
Out: prose rows (`a write is running`, `a take is still landing`); the resolver footer (10c, done); rewording.

## Signatures
No new public items. Word fns gaining `keys: &Keymap` are named in MAP.md.

## Acceptance (each tick names its test, at tick time)
- [ ] `update::tests::keymap_words::resolver::each_resolver_word_spells_the_remapped_key` — one row per site (count
      stated): sweep keymap → remapped label, no default label of a moved action as a whole token; shipped →
      byte-identical.
- [ ] `…::resolver::each_word_names_the_row_that_answers_it` — W2 pin per site (ctx stated).
- [ ] `…::resolver::an_unbound_key_drops_its_clause` — the `t` row W2 pins `= "none"`: `o keeps {ours}` alone;
      `o or t removes it` → `o removes it`; no `—`.
- [ ] A status word and the resolver footer name the same key the same way under the sweep keymap (one row: `⏎`
      marks done) — test named.
- [ ] `no_literal_keys::no_user_facing_literal_spells_a_default_key` green; `known_resolver.rs` holds no `KNOWN` row.
- [ ] Captures: shipped keymap → `git diff --stat main -- docs/shots` empty, or only W6 normalisations in NOTES.
- [ ] MAP.md graft: changed fn signatures.

## Touches
- Callers (compiler); resolver tests asserting these strings (grep each literal); m6 resolver captures (shipped: none
  expected).
- Journal/undo: words only — `git diff main -- crates/graft/src/update/journal* crates/graft-ops` empty.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~45k: ~20 sites ~15k, table + pins ~20k, MAP/NOTES ~5k.
