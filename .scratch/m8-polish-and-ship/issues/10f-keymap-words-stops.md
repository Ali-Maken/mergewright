# G-keymap 2f — stop, outcome and failure words spell keys from his keymap

Status: ready
Build with: **Sonnet** — every site listed, every rule decided (10b W1–W6), each held by a table row. Not Fable.
Blocked by: 10b, 10e (same callers' files; test scaffold).
Needs: from 10e — `update/tests/keymap_words.rs` helpers, `OpKeys`; from 10/10c — `Keymap::{spell_keys, key_verb,
after, translate}`, `AppState::keymap`.
Crate set: graft + graft-ui `tests/no_literal_keys/known_stops.rs` only.
**Files:** graft `update/banner/{focus_keys,pressed,resume}.rs`, `update/commit_editor/refusal.rs`, `update/mod.rs`
(one site), `update/operate/{outcome.rs,outcome/staging_not_kept.rs,failure.rs,diff_cursor.rs}`,
`update/finish/keys.rs`, `view/journal/summary.rs`, `update/tests/keymap_words/stops.rs`, `MAP.md` (own entries);
graft-ui `tests/no_literal_keys/known_stops.rs`. Plus any path 10b put in `known_stops.rs` (conductor amends).
Disjoint from 10g, 10h, 10i except MAP.md.

## Read first
- `10b-keymap-spell-in-words.md` **Ruling** → W1–W6
- `crates/graft/MAP.md` entries for each module in Files; `OperationRefusal`, `operation_refusal`, `tab_keys`
- `crates/graft-ui/MAP.md` `## keymap` (`key_verb`, `after`, `spell_keys`)

## Scope
In — each site spelled per W1–W6 (sites from `10b-grep-hits.txt` at 80dd116b; regenerate from `known_stops.rs`):
- `banner/focus_keys.rs`, `banner/pressed.rs` ×3, `view/journal/summary.rs`: `c continue  a abort`;
  `banner/pressed.rs` `u puts the merge back`
- `banner/resume.rs` `e edits {}  a abort`, `a abort`; `update/mod.rs` `e edits {}  a abort`
- `commit_editor/refusal.rs` `then c finish  a abort`, `then c continue  a abort` (keep `OperationRefusal`'s one-value
  rule: `editor_words()` and `status()` stay from one value)
- `operate/outcome.rs` `c continues`, `u undoes`; `outcome/staging_not_kept.rs` `u undoes it, staging included`
- `operate/failure.rs` `S p puts them back` (W5), `u restores the backup` ×2, `e edit  M mergetool`,
  `S stash {count} changed`
- `operate/diff_cursor.rs:180,297` (from 10b's original list; not grep hits)
- `finish/keys.rs` `⇥ to write one`
- every other `KNOWN: 10f` row in `known_stops.rs`
Out: prose rows; graft-ops words (10e); rewording (20 — its item 17 is in `failure.rs`).

## Signatures
No new public items. Word fns gaining `keys: &Keymap` are named in MAP.md.

## Acceptance (each tick names its test, at tick time)
- [ ] `update::tests::keymap_words::stops::each_stop_word_spells_the_remapped_key` — one row per site (count
      stated): sweep keymap → remapped label, no default label of a moved action as a whole token; shipped →
      byte-identical to the literal.
- [ ] `…::stops::each_word_names_the_row_that_answers_it` — W2 pin per site (ctx stated). A mismatch was a stop.
- [ ] `…::stops::an_unbound_key_drops_its_clause` — `banner.abort` (or the row W2 pins) `= "none"`: `c continue`
      alone; the failure line with stash unbound drops its `S …` clause; no `—`.
- [ ] `no_literal_keys::no_user_facing_literal_spells_a_default_key` green; `known_stops.rs` holds no `KNOWN` row.
- [ ] Captures: shipped keymap → `git diff --stat main -- docs/shots` empty, or only W6 normalisations listed in NOTES.
- [ ] MAP.md graft: changed fn signatures.

## Touches
- Callers of each changed word fn (compiler); tests asserting these strings (grep each literal).
- The banner and journal summary draw the same `c continue  a abort` state: both spelled by one helper, say which.
- Journal/undo: words only — `git diff main -- crates/graft/src/update/journal*` empty.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~50k: ~21 sites ~20k, table + pins ~20k, MAP/NOTES ~5k.
