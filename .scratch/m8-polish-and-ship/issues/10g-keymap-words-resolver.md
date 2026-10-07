# G-keymap 2g — conflict resolver words spell keys from his keymap

Status: partial (2026-10-07) — 2 of 19 KNOWN rows left, see NOTES
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
- [x] `update::tests::keymap_words::resolver::each_resolver_word_spells_the_remapped_key` — one row per site (count
      stated): sweep keymap → remapped label, no default label of a moved action as a whole token; shipped →
      byte-identical. 21 sites, held by that test (asserts the count).
- [x] `…::resolver::each_word_names_the_row_that_answers_it` — W2 pin per site (ctx stated). 12 pins, held by
      that test: Resolver n/⏎/d/e/o/t/c/esc; Graph `u` (W5, `esc first · u …`); Graph and Changes `esc`
      (`esc stops it`, resolver not open); Changes `u`.
- [x] `…::resolver::an_unbound_key_drops_its_clause` — the `t` row W2 pins `= "none"`: `o keeps {ours}` alone;
      `o or t removes it` → `o removes it`; no `—`. Held by that test (also: both unbound → empty; `⏎` unbound →
      `all blocks taken` alone).
- [x] A status word and the resolver footer name the same key the same way under the sweep keymap (one row: `⏎`
      marks done) — `…::resolver::a_notice_and_the_footer_spell_mark_done_alike`.
- [ ] `no_literal_keys::no_user_facing_literal_spells_a_default_key` green (yes, full nextest); **`known_resolver.rs`
      still holds 2 `KNOWN: 10g` rows** — not ticked, see NOTES.
- [x] Captures: shipped keymap → `git diff --stat main -- docs/shots` empty — checked by that command; shipped
      bytes also held by `each_resolver_word_spells_the_remapped_key` and the existing resolver tests unchanged.
- [x] MAP.md graft: changed fn signatures — `whole_file_keys` (+`keys: &Keymap`), `Identity::Done.restart`
      (`bool` → `Option<&str>`). Checked by reading the diff.

## Touches
- Callers (compiler); resolver tests asserting these strings (grep each literal); m6 resolver captures (shipped: none
  expected).
- Journal/undo: words only — `git diff main -- crates/graft/src/update/journal* crates/graft-ops` empty.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~45k: ~20 sites ~15k, table + pins ~20k, MAP/NOTES ~5k.

## NOTES (builder, 2026-10-07)

Scout: none — no scout file named; the ticket listed every site with its literal, and `known_resolver.rs` was the map.
Did not compact.

**Done (17 of 19 KNOWN rows + 4 the grep missed).** Spelled from `state.keymap` via `key_verb` / `key_verbs` /
`after`: `take.rs` `n next` + `all_taken`; `resolved.rs` `n next`; `answers/taken.rs` bulk and settled notices;
`answers.rs` `no conflict markers`; `done.rs`; `open.rs` ×2; `whole_file.rs` ×7 (`o or t` joins the bound labels
with ` or `); `view/resolver/whole_file.rs` ×2. Not on the KNOWN list (scanner missed: `\u{b7}` / leading `·` /
`format!` with `{c}`) but in 10g's paths, spelled too: `resolver.rs:399` `esc first · u undoes from the graph`
(W5: `app.back` + `app.undo`; every key unbound → no notice), `view/resolver.rs` done title ` · d starts it over`,
`view/resolver.rs` `finish_row` `c review and finish …` / `c continue …` (`resolver.finish`; unbound → no row).

**Left — 2 KNOWN rows, conductor must route:**
1. `answers.rs:open_failed` `e edit  M mergetool` — **W2 stop.** The resolver closes before the line shows; a
   context-gated spelling (Banner → `operation.*`, Changes → `changes.*`, else none) failed the existing
   `update::resolver::tests::file_list::an_unmerged_open_failure_still_closes`, i.e. in that flow `context_of` after
   the close is neither Banner nor Changes. Which context it is I did not confirm. In Graph `M` is `graph.map`
   (`graft-ui/src/keymap/table.rs:155`), so the shipped words may already name a key that does not hand over —
   filed as a question. Literal left, comment at the site.
2. `resolved.rs:finish_refused_notice` `{reason} · ⏎ marks done` — needs `keys: &Keymap`; its callers are
   `update/banner/pressed.rs:40` (10f's) and `update/status_read.rs:67` (10h's area). **blocked: needs those two
   files** — signature: `pub(crate) fn finish_refused_notice(reason: &str, keys: &Keymap) -> (String, bool)`, callers
   pass `&state.keymap`.

**Touches:** callers — compiler (`whole_file_keys`: view + `tests/it/resolver/whole_file.rs`; `resolver_title` /
`Identity::Done`: `view/resolver.rs`, `view/resolver/whole_file.rs`); resolver tests asserting these strings —
full nextest 5750/5750 green unchanged; captures — `git diff --stat main -- docs/shots` empty; journal/undo —
`git diff main -- crates/graft/src/update/journal* crates/graft-ops` empty; 10f — only shared file MAP.md (one
entry edited: `whole_file_keys`, plus the `Identity` clause in the `resolver` view entry); `keymap_words.rs` not
touched (mod line already there).
